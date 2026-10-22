//! 自定义原子对象头和句柄 ABI 的隔离模型。
//!
//! 这里故意不使用 `Arc`/`Weak` 保存对象和计数。原型只验证布局、计数、弱升级和
//! 回收顺序，不能被解释为 Xiao Runtime 的对象头或 ABI。

use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::{align_of, size_of, MaybeUninit};
use std::panic::AssertUnwindSafe;
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

use super::catches_expected_panic;

/// 原子句柄的实验 ABI 身份。
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HandleTag {
    /// 普通单线程句柄身份。
    Ordinary = 1,
    /// 显式共享句柄身份。
    Shared = 2,
}

/// 记录载荷和对象头各自恰好析构一次。
#[derive(Clone)]
pub(crate) struct DropCounters {
    /// 载荷析构次数。
    pub payload: Arc<AtomicUsize>,
    /// 对象头析构次数。
    pub header: Arc<AtomicUsize>,
}

impl DropCounters {
    /// 创建清零的析构计数器。
    pub(crate) fn new() -> Self {
        Self {
            payload: Arc::new(AtomicUsize::new(0)),
            header: Arc::new(AtomicUsize::new(0)),
        }
    }
}

/// 自定义对象头：原子强/弱计数、身份 tag 和实验析构探针。
#[repr(C)]
pub(crate) struct AtomicHeader {
    strong: AtomicUsize,
    weak: AtomicUsize,
    tag: AtomicU8,
}

impl AtomicHeader {
    /// 新建带一个隐式弱引用的对象头。
    fn new(tag: HandleTag) -> Self {
        Self {
            strong: AtomicUsize::new(1),
            weak: AtomicUsize::new(1),
            tag: AtomicU8::new(tag as u8),
        }
    }

    /// 读取对象身份 tag。
    fn tag(&self) -> HandleTag {
        match self.tag.load(Ordering::Acquire) {
            value if value == HandleTag::Ordinary as u8 => HandleTag::Ordinary,
            value if value == HandleTag::Shared as u8 => HandleTag::Shared,
            value => panic!("未知原子句柄 tag：{value}"),
        }
    }
}

/// 自定义对象头和初始化载荷的连续布局。
#[repr(C)]
struct AtomicObject<T> {
    header: AtomicHeader,
    payload_drops: Arc<AtomicUsize>,
    header_drops: Arc<AtomicUsize>,
    payload: UnsafeCell<MaybeUninit<T>>,
}

/// 不带测试探针的干净 ABI 对象布局，仅用于 `size_of`/`offset_of` 测量。
#[repr(C)]
struct CleanAtomicObject<T> {
    header: AtomicHeader,
    payload: UnsafeCell<MaybeUninit<T>>,
}

impl<T> Drop for AtomicObject<T> {
    /// 记录对象头只回收一次；载荷由最后强句柄显式析构。
    fn drop(&mut self) {
        self.header_drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// 用于验证高对齐载荷偏移的实验载荷。
#[allow(dead_code)]
#[repr(align(32))]
struct Align32([u8; 32]);

/// 显式共享句柄的自定义对象头包装器。
pub(crate) struct SharedHandle<T> {
    ptr: NonNull<AtomicObject<T>>,
    marker: PhantomData<T>,
}

// SAFETY: 共享句柄只在 T 本身可跨线程读写时跨线程；计数和回收由原子操作保护。
unsafe impl<T: Send + Sync> Send for SharedHandle<T> {}
// SAFETY: 载荷访问的安全条件由 T: Sync 和显式锁/原子载荷承担。
unsafe impl<T: Send + Sync> Sync for SharedHandle<T> {}

impl<T> SharedHandle<T> {
    /// 在自定义对象头中分配共享载荷。
    pub(crate) fn new(value: T, counters: &DropCounters) -> Self {
        let object = Box::new(AtomicObject {
            header: AtomicHeader::new(HandleTag::Shared),
            payload_drops: Arc::clone(&counters.payload),
            header_drops: Arc::clone(&counters.header),
            payload: UnsafeCell::new(MaybeUninit::new(value)),
        });
        Self {
            // Box::into_raw never returns null.
            ptr: unsafe { NonNull::new_unchecked(Box::into_raw(object)) },
            marker: PhantomData,
        }
    }

    /// 读取共享句柄的 ABI 身份。
    pub(crate) fn tag(&self) -> HandleTag {
        self.object().header.tag()
    }

    /// 读取强计数。
    pub(crate) fn strong_count(&self) -> usize {
        self.object().header.strong.load(Ordering::Acquire)
    }

    /// 读取包含隐式弱引用的弱计数。
    pub(crate) fn weak_count(&self) -> usize {
        self.object().header.weak.load(Ordering::Acquire)
    }

    /// 以 acquire 语义读取载荷引用。
    pub(crate) fn payload(&self) -> &T {
        // SAFETY: SharedHandle 持有强引用，载荷在强计数归零前保持初始化。
        unsafe { (&*self.object().payload.get()).assume_init_ref() }
    }

    /// 创建显式弱句柄。
    pub(crate) fn downgrade(&self) -> WeakHandle<T> {
        increment(&self.object().header.weak);
        WeakHandle {
            ptr: self.ptr,
            marker: PhantomData,
        }
    }

    /// 对象头布局和对齐的可观察结果。
    pub(crate) fn layout() -> AbiLayout {
        AbiLayout {
            header_bytes: size_of::<AtomicHeader>(),
            payload_offset: std::mem::offset_of!(AtomicObject<u64>, payload),
            header_align: align_of::<AtomicHeader>(),
            shared_handle_bytes: size_of::<SharedHandle<u64>>(),
            weak_handle_bytes: size_of::<WeakHandle<u64>>(),
            ordinary_handle_bytes: size_of::<OrdinaryHandle<u64>>(),
            object_u64_bytes: size_of::<AtomicObject<u64>>(),
            object_u64_align: align_of::<AtomicObject<u64>>(),
            object_align32_bytes: size_of::<AtomicObject<Align32>>(),
            object_align32_align: align_of::<AtomicObject<Align32>>(),
            align32_payload_offset: std::mem::offset_of!(AtomicObject<Align32>, payload),
            clean_object_u64_bytes: size_of::<CleanAtomicObject<u64>>(),
            clean_u64_payload_offset: std::mem::offset_of!(CleanAtomicObject<u64>, payload),
            clean_object_align32_bytes: size_of::<CleanAtomicObject<Align32>>(),
            clean_align32_payload_offset: std::mem::offset_of!(CleanAtomicObject<Align32>, payload),
        }
    }

    /// 访问仍由强计数保持存活的对象。
    fn object(&self) -> &AtomicObject<T> {
        // SAFETY: self.ptr originates from Box and is kept alive by a strong count.
        unsafe { self.ptr.as_ref() }
    }
}

/// 输出自定义对象头原型的稳定布局记录。
pub(crate) fn layout_report() -> String {
    format!("{:?}", SharedHandle::<u64>::layout())
}

/// 自定义句柄操作的隔离成本记录。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AbiCosts {
    /// 强句柄 clone/drop 平均纳秒数。
    pub clone_drop_ns: u128,
    /// 弱升级成功平均纳秒数。
    pub upgrade_success_ns: u128,
    /// 弱升级失败平均纳秒数。
    pub upgrade_failure_ns: u128,
    /// 自定义共享句柄锁保护增量平均纳秒数。
    pub lock_ns: u128,
    /// 取消标记检查平均纳秒数。
    pub cancel_check_ns: u128,
    /// 自定义载荷和对象头析构/分配平均纳秒数。
    pub drop_ns: u128,
    /// 不带析构探针的干净对象分配/析构平均纳秒数。
    pub clean_drop_ns: u128,
}

/// 测量自定义句柄的计数、升级、锁、取消检查和析构成本。
pub(crate) fn measure_costs(iterations: usize) -> AbiCosts {
    let iterations = iterations.max(1);
    let counters = DropCounters::new();
    let shared = SharedHandle::new(0_u64, &counters);
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let clone = shared.clone();
        std::hint::black_box(clone.payload());
        drop(clone);
    }
    let clone_drop_ns = super::average_ns(start.elapsed(), iterations);

    let weak = shared.downgrade();
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let upgraded = weak.upgrade().expect("强句柄存活时升级必须成功");
        std::hint::black_box(upgraded.payload());
        drop(upgraded);
    }
    let upgrade_success_ns = super::average_ns(start.elapsed(), iterations);
    drop(shared);

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        assert!(weak.upgrade().is_none());
    }
    let upgrade_failure_ns = super::average_ns(start.elapsed(), iterations);
    drop(weak);

    let lock_counters = DropCounters::new();
    let mutable = SharedHandle::new(Mutex::new(0_usize), &lock_counters);
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        *mutable
            .payload()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) += 1;
    }
    let lock_ns = super::average_ns(start.elapsed(), iterations);
    drop(mutable);

    let cancelled = AtomicU8::new(1);
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(cancelled.load(Ordering::Acquire));
    }
    let cancel_check_ns = super::average_ns(start.elapsed(), iterations);

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let drop_counters = DropCounters::new();
        let value = SharedHandle::new(1_u64, &drop_counters);
        drop(value);
    }
    let drop_ns = super::average_ns(start.elapsed(), iterations);

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let object = Box::new(CleanAtomicObject {
            header: AtomicHeader::new(HandleTag::Shared),
            payload: UnsafeCell::new(MaybeUninit::new(1_u64)),
        });
        std::hint::black_box(object);
    }
    let clean_drop_ns = super::average_ns(start.elapsed(), iterations);

    AbiCosts {
        clone_drop_ns,
        upgrade_success_ns,
        upgrade_failure_ns,
        lock_ns,
        cancel_check_ns,
        drop_ns,
        clean_drop_ns,
    }
}

impl<T> Clone for SharedHandle<T> {
    /// 原子增加强计数后复制共享句柄。
    fn clone(&self) -> Self {
        increment(&self.object().header.strong);
        Self {
            ptr: self.ptr,
            marker: PhantomData,
        }
    }
}

impl<T> Drop for SharedHandle<T> {
    /// 释放强计数；最后强引用先析构载荷，再释放隐式弱引用。
    fn drop(&mut self) {
        let object = self.object();
        let old = object.header.strong.fetch_sub(1, Ordering::AcqRel);
        assert!(old > 0, "共享句柄强计数下溢");
        if old == 1 {
            // SAFETY: old == 1 表示没有其他强句柄仍访问载荷。
            unsafe { ptr::drop_in_place((*object.payload.get()).as_mut_ptr()) };
            object.payload_drops.fetch_add(1, Ordering::SeqCst);
            release_weak(self.ptr);
        }
    }
}

/// 自定义对象头的弱句柄。
pub(crate) struct WeakHandle<T> {
    ptr: NonNull<AtomicObject<T>>,
    marker: PhantomData<T>,
}

// SAFETY: 弱句柄只操作原子计数，升级后的载荷安全由 T 的边界承担。
unsafe impl<T: Send + Sync> Send for WeakHandle<T> {}
// SAFETY: 弱句柄没有裸载荷访问，只能通过原子升级获得强句柄。
unsafe impl<T: Send + Sync> Sync for WeakHandle<T> {}

impl<T> Clone for WeakHandle<T> {
    /// 原子增加弱计数后复制弱句柄。
    fn clone(&self) -> Self {
        increment(&self.object().header.weak);
        Self {
            ptr: self.ptr,
            marker: PhantomData,
        }
    }
}

impl<T> WeakHandle<T> {
    /// 在强计数归零后拒绝升级，避免载荷复活。
    pub(crate) fn upgrade(&self) -> Option<SharedHandle<T>> {
        let header = &self.object().header;
        let mut strong = header.strong.load(Ordering::Acquire);
        loop {
            if strong == 0 {
                return None;
            }
            match header.strong.compare_exchange_weak(
                strong,
                strong.checked_add(1).expect("共享句柄强计数溢出"),
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Some(SharedHandle {
                        ptr: self.ptr,
                        marker: PhantomData,
                    });
                }
                Err(observed) => strong = observed,
            }
        }
    }

    /// 访问由弱计数保持对象头存活的对象。
    fn object(&self) -> &AtomicObject<T> {
        // SAFETY: 弱计数保持对象头存活，即使载荷已经析构。
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Drop for WeakHandle<T> {
    /// 释放弱计数；最后弱引用回收对象头。
    fn drop(&mut self) {
        release_weak(self.ptr);
    }
}

/// 普通句柄的独立 tag/类型模型，带 Rc 标记以保持单线程边界。
pub(crate) struct OrdinaryHandle<T> {
    value: T,
    tag: HandleTag,
    marker: PhantomData<Rc<()>>,
}

impl<T> OrdinaryHandle<T> {
    /// 创建普通句柄模型。
    pub(crate) fn new(value: T) -> Self {
        Self {
            value,
            tag: HandleTag::Ordinary,
            marker: PhantomData,
        }
    }

    /// 读取普通句柄身份；没有隐式转为共享句柄的方法。
    pub(crate) fn tag(&self) -> HandleTag {
        self.tag
    }

    /// 读取普通句柄载荷，避免测试字段被优化掉。
    pub(crate) fn payload(&self) -> &T {
        &self.value
    }
}

/// 自定义对象头和句柄的可观察尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AbiLayout {
    /// 对象头尺寸。
    pub header_bytes: usize,
    /// 载荷相对对象头的偏移。
    pub payload_offset: usize,
    /// 对象头对齐。
    pub header_align: usize,
    /// 共享句柄包装器尺寸。
    pub shared_handle_bytes: usize,
    /// 弱句柄包装器尺寸。
    pub weak_handle_bytes: usize,
    /// 普通句柄包装器尺寸。
    pub ordinary_handle_bytes: usize,
    /// `u64` 载荷对象尺寸。
    pub object_u64_bytes: usize,
    /// `u64` 载荷对象对齐。
    pub object_u64_align: usize,
    /// 32-byte 对齐载荷对象尺寸。
    pub object_align32_bytes: usize,
    /// 32-byte 对齐载荷对象对齐。
    pub object_align32_align: usize,
    /// 32-byte 对齐载荷的真实偏移。
    pub align32_payload_offset: usize,
    /// 不带探针的 `u64` 对象尺寸。
    pub clean_object_u64_bytes: usize,
    /// 不带探针的 `u64` 载荷偏移。
    pub clean_u64_payload_offset: usize,
    /// 不带探针的 32-byte 对齐对象尺寸。
    pub clean_object_align32_bytes: usize,
    /// 不带探针的 32-byte 对齐载荷偏移。
    pub clean_align32_payload_offset: usize,
}

/// 以 checked 算术增加原子计数。
fn increment(counter: &AtomicUsize) {
    counter
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .expect("原子句柄计数溢出");
}

/// 以 acquire/release 顺序释放弱计数并在归零时回收对象头。
fn release_weak<T>(ptr: NonNull<AtomicObject<T>>) {
    // SAFETY: 每个弱计数来源都保持对象头存活到这次递减。
    let object = unsafe { ptr.as_ref() };
    let old = object.header.weak.fetch_sub(1, Ordering::AcqRel);
    assert!(old > 0, "共享句柄弱计数下溢");
    if old == 1 {
        // SAFETY: strong == 0 且弱计数归零时，Box 是唯一剩余所有权。
        unsafe { drop(Box::from_raw(ptr.as_ptr())) };
    }
}

/// 自定义对象头原型的完整场景和验证。
pub(crate) fn run_demo() -> Result<(), String> {
    let counters = DropCounters::new();
    let shared = SharedHandle::new(AtomicUsize::new(0), &counters);
    let layout = SharedHandle::<u64>::layout();
    if layout.header_bytes == 0
        || layout.header_align == 0
        || layout.payload_offset < layout.header_bytes
        || layout.align32_payload_offset < layout.header_bytes
        || layout.object_align32_align != 32
        || !layout.align32_payload_offset.is_multiple_of(32)
        || layout.clean_object_u64_bytes != 32
        || layout.clean_u64_payload_offset != 24
        || layout.clean_object_align32_bytes != 64
        || layout.clean_align32_payload_offset != 32
        || layout.shared_handle_bytes == 0
        || layout.weak_handle_bytes == 0
        || layout.ordinary_handle_bytes == 0
    {
        return Err(format!("对象头布局无效：{layout:?}"));
    }
    if shared.tag() != HandleTag::Shared || shared.strong_count() != 1 || shared.weak_count() != 1 {
        return Err("共享句柄初始 tag/计数错误".to_owned());
    }

    // release -> acquire 发布路径：共享句柄在移动后读取原子载荷。
    let publisher = shared.clone();
    let writer = thread::spawn(move || {
        publisher.payload().store(7, Ordering::Release);
    });
    writer.join().map_err(|_| "发布线程 panic".to_owned())?;
    if shared.payload().load(Ordering::Acquire) != 7 {
        return Err("acquire/release 发布路径读取错误".to_owned());
    }

    let weak = shared.downgrade();
    /// 并发弱升级线程数量。
    const WORKERS: usize = 8;
    let barrier = Arc::new(Barrier::new(WORKERS + 1));
    let successes = Arc::new(AtomicUsize::new(0));
    let failures = Arc::new(AtomicUsize::new(0));
    let mut joins = Vec::with_capacity(WORKERS);
    for _ in 0..WORKERS {
        let worker_weak = weak.clone();
        let worker_barrier = Arc::clone(&barrier);
        let worker_successes = Arc::clone(&successes);
        let worker_failures = Arc::clone(&failures);
        joins.push(thread::spawn(move || {
            worker_barrier.wait();
            if worker_weak.upgrade().is_some() {
                worker_successes.fetch_add(1, Ordering::SeqCst);
            } else {
                worker_failures.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }
    barrier.wait();
    drop(shared);
    for join in joins {
        join.join()
            .map_err(|_| "自定义弱升级线程 panic".to_owned())?;
    }
    if successes.load(Ordering::SeqCst) + failures.load(Ordering::SeqCst) != WORKERS {
        return Err("自定义弱升级竞态结果不完整".to_owned());
    }
    if weak.upgrade().is_some() {
        return Err("自定义对象头强计数归零后仍可升级".to_owned());
    }
    drop(weak);
    if counters.payload.load(Ordering::SeqCst) != 1 || counters.header.load(Ordering::SeqCst) != 1 {
        return Err(format!(
            "自定义对象头析构次数错误：payload={}, header={}",
            counters.payload.load(Ordering::SeqCst),
            counters.header.load(Ordering::SeqCst)
        ));
    }

    let mutable_counters = DropCounters::new();
    let mutable = SharedHandle::new(Mutex::new(0_usize), &mutable_counters);
    let mut writers = Vec::new();
    for _ in 0..4 {
        let worker = mutable.clone();
        writers.push(thread::spawn(move || {
            for _ in 0..100 {
                *worker
                    .payload()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) += 1;
            }
        }));
    }
    for writer in writers {
        writer
            .join()
            .map_err(|_| "自定义共享写入线程 panic".to_owned())?;
    }
    if *mutable
        .payload()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        != 400
    {
        return Err("自定义共享句柄锁保护写入错误".to_owned());
    }

    let guard = mutable
        .payload()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let cancel = Arc::new(AtomicU8::new(0));
    let ready = Arc::new(Barrier::new(2));
    let waiter_handle = mutable.clone();
    let waiter_cancel = Arc::clone(&cancel);
    let waiter_ready = Arc::clone(&ready);
    let waiter = thread::spawn(move || {
        waiter_ready.wait();
        loop {
            if waiter_cancel.load(Ordering::Acquire) != 0 {
                return true;
            }
            if let Ok(waiter_guard) = waiter_handle.payload().try_lock() {
                drop(waiter_guard);
                return false;
            }
            thread::yield_now();
        }
    });
    ready.wait();
    cancel.store(1, Ordering::Release);
    if !waiter
        .join()
        .map_err(|_| "自定义取消等待线程 panic".to_owned())?
    {
        return Err("自定义共享句柄取消没有优先于锁获取".to_owned());
    }
    drop(guard);

    let panic_observed = catches_expected_panic(AssertUnwindSafe(|| {
        let _poison_guard = mutable
            .payload()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        panic!("原型故意触发自定义共享句柄异常");
    }));
    if !panic_observed {
        return Err("自定义共享句柄异常路径没有触发".to_owned());
    }
    *mutable
        .payload()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) += 1;
    let ordinary = OrdinaryHandle::new(3_u64);
    if ordinary.tag() != HandleTag::Ordinary
        || *ordinary.payload() != 3
        || mutable.tag() != HandleTag::Shared
    {
        return Err("普通/共享句柄 tag 身份混淆".to_owned());
    }
    drop(mutable);
    if mutable_counters.payload.load(Ordering::SeqCst) != 1
        || mutable_counters.header.load(Ordering::SeqCst) != 1
    {
        return Err("自定义共享可变句柄没有恰好一次析构".to_owned());
    }
    Ok(())
}

/// 自定义 ABI 原型的 Rust 单元测试。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证自定义对象头、计数、锁和析构场景。
    fn custom_atomic_abi_scenarios_pass() {
        run_demo().unwrap();
    }
}
