//! 22C 并发 Runtime 的共享句柄和 Runtime 身份基础。
//!
//! 本模块不实现 Xiao 标准库函数，也不改变现有单线程 `StrongHandle`/`WeakHandle`。
//! 共享句柄拥有独立的 32B 对象头：对象头只保存强/弱计数、tag 和锁状态，Runtime ID
//! 保存在句柄包装中，符合 22A 的 ABI 约束。

use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

/// 22A 冻结的共享句柄 tag。
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SharedTag {
    /// 普通共享对象包装。
    Ordinary = 1,
    /// 只读共享句柄。
    Read = 2,
    /// 共享可变句柄。
    Mut = 3,
}

impl SharedTag {
    /// 从 ABI 原始值解析 tag。
    #[must_use]
    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::Ordinary),
            2 => Some(Self::Read),
            3 => Some(Self::Mut),
            _ => None,
        }
    }

    /// 返回稳定 ABI 数值。
    #[must_use]
    pub const fn raw(self) -> u64 {
        self as u64
    }
}

/// 共享对象头中的锁状态位。
pub const SHARED_LOCKED: u32 = 1;
/// 等待者提示位；等待实现不把等待计数写入头部。
pub const SHARED_WAITERS: u32 = 1 << 31;

/// 22A 冻结的 32B 共享对象头。
#[repr(C)]
pub struct SharedHeader {
    /// 强引用计数。
    pub strong: AtomicU64,
    /// 弱引用计数，包含对象头隐式弱引用。
    pub weak: AtomicU64,
    /// `SharedTag` 的稳定数值。
    pub tag: AtomicU64,
    /// 低 4B 锁状态；最高位表示存在等待者。
    pub lock_state: AtomicU32,
    /// 为 ABI 对齐保留的 4B。
    pub reserved: u32,
}

/// 编译期锁定共享头尺寸。
const _: () = assert!(std::mem::size_of::<SharedHeader>() == 32);
/// 编译期锁定共享头对齐。
const _: () = assert!(std::mem::align_of::<SharedHeader>() == 8);

impl SharedHeader {
    /// 创建初始强/弱计数和 tag。
    fn new(tag: SharedTag) -> Self {
        Self {
            strong: AtomicU64::new(1),
            weak: AtomicU64::new(1),
            tag: AtomicU64::new(tag.raw()),
            lock_state: AtomicU32::new(0),
            reserved: 0,
        }
    }
}

#[repr(C)]
/// 共享头与载荷的连续分配。
struct SharedAllocation<T> {
    header: SharedHeader,
    payload: T,
}

/// 共享句柄构造失败。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedHandleError {
    /// Runtime ID 0 是冻结的无效值。
    InvalidRuntimeId,
    /// 共享头 tag 不是 `shared_read`/`shared_mut` 可接受的值。
    InvalidTag,
    /// 共享对象已经被销毁。
    ObjectDestroyed,
    /// 进程内 Runtime ID 已耗尽。
    RuntimeIdExhausted,
}

impl fmt::Display for SharedHandleError {
    /// 输出开发者可读的构造错误。
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRuntimeId => formatter.write_str("Runtime ID 0 无效"),
            Self::InvalidTag => formatter.write_str("共享句柄 tag 无效"),
            Self::ObjectDestroyed => formatter.write_str("共享对象已经销毁"),
            Self::RuntimeIdExhausted => formatter.write_str("Runtime ID 已耗尽"),
        }
    }
}

impl std::error::Error for SharedHandleError {}

/// 共享锁获取失败。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedLockError {
    /// 等待锁时收到取消请求。
    Cancelled,
    /// 共享句柄不是可变共享句柄。
    InvalidTag,
}

impl fmt::Display for SharedLockError {
    /// 输出锁获取失败原因。
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("共享锁等待被取消"),
            Self::InvalidTag => formatter.write_str("共享句柄 tag 不允许可变锁"),
        }
    }
}

impl std::error::Error for SharedLockError {}

/// 带 Runtime ID 的强共享句柄包装。
pub struct SharedHandle<T> {
    ptr: NonNull<SharedAllocation<T>>,
    runtime_id: u64,
    _marker: PhantomData<T>,
}

// 共享句柄只允许由 Send + Sync 的载荷组成；对象头的计数和锁状态是原子的。
unsafe impl<T: Send + Sync> Send for SharedHandle<T> {}
unsafe impl<T: Send + Sync> Sync for SharedHandle<T> {}

impl<T: Send + Sync> SharedHandle<T> {
    /// 创建一个带来源 Runtime ID 的共享句柄。
    pub fn try_new(value: T, runtime_id: u64, tag: SharedTag) -> Result<Self, SharedHandleError> {
        if runtime_id == 0 {
            return Err(SharedHandleError::InvalidRuntimeId);
        }
        if !matches!(tag, SharedTag::Read | SharedTag::Mut) {
            return Err(SharedHandleError::InvalidTag);
        }
        let allocation = Box::new(SharedAllocation {
            header: SharedHeader::new(tag),
            payload: value,
        });
        let ptr = NonNull::from(Box::leak(allocation));
        Ok(Self {
            ptr,
            runtime_id,
            _marker: PhantomData,
        })
    }

    /// 创建共享句柄；无效 Runtime ID/tag 作为开发者不变量 panic。
    #[must_use]
    pub fn new(value: T, runtime_id: u64, tag: SharedTag) -> Self {
        Self::try_new(value, runtime_id, tag).expect("共享句柄构造违反 Runtime ABI")
    }

    /// 返回来源 Runtime ID。
    #[must_use]
    pub const fn runtime_id(&self) -> u64 {
        self.runtime_id
    }

    /// 返回共享句柄 tag。
    #[must_use]
    pub fn tag(&self) -> SharedTag {
        SharedTag::from_raw(unsafe { self.ptr.as_ref().header.tag.load(Ordering::Acquire) })
            .expect("共享头 tag 应由构造器保证")
    }

    /// 返回当前强引用数。
    #[must_use]
    pub fn strong_count(&self) -> u64 {
        unsafe { self.ptr.as_ref().header.strong.load(Ordering::Acquire) }
    }

    /// 返回当前弱引用数。
    #[must_use]
    pub fn weak_count(&self) -> u64 {
        unsafe { self.ptr.as_ref().header.weak.load(Ordering::Acquire) }
    }

    /// 读取只读共享载荷；共享可变句柄必须先持锁。
    pub fn get(&self) -> Result<&T, SharedHandleError> {
        if self.tag() != SharedTag::Read {
            return Err(SharedHandleError::InvalidTag);
        }
        Ok(unsafe { &self.ptr.as_ref().payload })
    }

    /// 创建弱共享句柄。
    #[must_use]
    pub fn downgrade(&self) -> SharedWeakHandle<T> {
        let header = unsafe { &self.ptr.as_ref().header };
        increment_checked(&header.weak, "共享弱引用计数溢出");
        SharedWeakHandle {
            ptr: self.ptr,
            runtime_id: self.runtime_id,
            _marker: PhantomData,
        }
    }

    /// 以共享可变 tag 获取独占锁。
    pub fn lock(&self) -> Result<SharedGuard<'_, T>, SharedHandleError> {
        if self.tag() != SharedTag::Mut {
            return Err(SharedHandleError::InvalidTag);
        }
        let waiters = acquire_lock(unsafe { &self.ptr.as_ref().header.lock_state }, None)
            .map_err(|_| SharedHandleError::ObjectDestroyed)?;
        Ok(SharedGuard {
            handle: self,
            waiters,
        })
    }

    /// 以共享可变 tag 获取可取消的独占锁。
    pub fn lock_with_cancel(
        &self,
        cancelled: &LockCancellation,
    ) -> Result<SharedGuard<'_, T>, SharedLockError> {
        if self.tag() != SharedTag::Mut {
            return Err(SharedLockError::InvalidTag);
        }
        let waiters = acquire_lock(
            unsafe { &self.ptr.as_ref().header.lock_state },
            Some(cancelled),
        )?;
        Ok(SharedGuard {
            handle: self,
            waiters,
        })
    }
}

impl<T> SharedHandle<T> {
    /// 释放一个强引用，并在最后一个强引用时销毁载荷。
    fn release_strong(&mut self) {
        let allocation = unsafe { self.ptr.as_ref() };
        if allocation.header.strong.fetch_sub(1, Ordering::AcqRel) == 1 {
            std::sync::atomic::fence(Ordering::Acquire);
            unsafe {
                std::ptr::drop_in_place(std::ptr::addr_of_mut!((*self.ptr.as_ptr()).payload));
            }
            release_weak(self.ptr);
        }
    }
}

impl<T> Clone for SharedHandle<T> {
    /// 原子增加强引用计数。
    fn clone(&self) -> Self {
        let header = unsafe { &self.ptr.as_ref().header };
        increment_strong(&header.strong);
        Self {
            ptr: self.ptr,
            runtime_id: self.runtime_id,
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for SharedHandle<T> {
    /// 归还强引用。
    fn drop(&mut self) {
        self.release_strong();
    }
}

impl<T> fmt::Debug for SharedHandle<T> {
    /// 输出不泄漏载荷的共享句柄摘要。
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header = unsafe { &self.ptr.as_ref().header };
        formatter
            .debug_struct("SharedHandle")
            .field("runtime_id", &self.runtime_id)
            .field(
                "tag",
                &SharedTag::from_raw(header.tag.load(Ordering::Acquire)),
            )
            .field("strong_count", &header.strong.load(Ordering::Acquire))
            .field("weak_count", &header.weak.load(Ordering::Acquire))
            .finish()
    }
}

/// 共享对象的弱句柄。
pub struct SharedWeakHandle<T> {
    ptr: NonNull<SharedAllocation<T>>,
    runtime_id: u64,
    _marker: PhantomData<T>,
}

unsafe impl<T: Send + Sync> Send for SharedWeakHandle<T> {}
unsafe impl<T: Send + Sync> Sync for SharedWeakHandle<T> {}

impl<T: Send + Sync> SharedWeakHandle<T> {
    /// 返回来源 Runtime ID。
    #[must_use]
    pub const fn runtime_id(&self) -> u64 {
        self.runtime_id
    }

    /// 尝试把弱句柄升级为强句柄。
    pub fn upgrade(&self) -> Option<SharedHandle<T>> {
        let header = unsafe { &self.ptr.as_ref().header };
        let mut current = header.strong.load(Ordering::Acquire);
        loop {
            if current == 0 || current == u64::MAX {
                return None;
            }
            match header.strong.compare_exchange_weak(
                current,
                current + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Some(SharedHandle {
                        ptr: self.ptr,
                        runtime_id: self.runtime_id,
                        _marker: PhantomData,
                    });
                }
                Err(observed) => current = observed,
            }
        }
    }
}

impl<T> Clone for SharedWeakHandle<T> {
    /// 原子增加弱引用计数。
    fn clone(&self) -> Self {
        let header = unsafe { &self.ptr.as_ref().header };
        increment_checked(&header.weak, "共享弱引用计数溢出");
        Self {
            ptr: self.ptr,
            runtime_id: self.runtime_id,
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for SharedWeakHandle<T> {
    /// 归还弱引用。
    fn drop(&mut self) {
        release_weak(self.ptr);
    }
}

/// 归还弱引用并在头部最后一个弱引用释放时回收分配。
fn release_weak<T>(ptr: NonNull<SharedAllocation<T>>) {
    let header = unsafe { &ptr.as_ref().header };
    if header.weak.fetch_sub(1, Ordering::AcqRel) == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        let layout = std::alloc::Layout::new::<SharedAllocation<T>>();
        unsafe { std::alloc::dealloc(ptr.as_ptr().cast::<u8>(), layout) };
    }
}

/// 共享可变句柄的独占访问守卫。
#[derive(Debug)]
pub struct SharedGuard<'a, T: Send + Sync> {
    handle: &'a SharedHandle<T>,
    waiters: Arc<LockWaiters>,
}

impl<T: Send + Sync> std::ops::Deref for SharedGuard<'_, T> {
    type Target = T;

    /// 返回共享守卫的只读载荷。
    fn deref(&self) -> &Self::Target {
        unsafe { &self.handle.ptr.as_ref().payload }
    }
}

impl<T: Send + Sync> std::ops::DerefMut for SharedGuard<'_, T> {
    /// 返回共享守卫的可变载荷。
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.handle.ptr.as_ptr()).payload }
    }
}

impl<T: Send + Sync> Drop for SharedGuard<'_, T> {
    /// 释放锁状态并唤醒阻塞的等待者。
    fn drop(&mut self) {
        let header = unsafe { &self.handle.ptr.as_ref().header };
        let _gate = self.waiters.gate.lock().expect("共享锁等待器锁未中毒");
        header.lock_state.store(0, Ordering::Release);
        self.waiters.ready.notify_all();
    }
}

/// 可唤醒共享锁等待者的取消令牌。
#[derive(Clone, Debug, Default)]
pub struct LockCancellation {
    inner: Arc<LockCancellationInner>,
}

#[derive(Debug, Default)]
struct LockCancellationInner {
    cancelled: AtomicBool,
    waiters: Mutex<Vec<Weak<LockWaiters>>>,
}

impl LockCancellation {
    /// 设置取消标志并唤醒所有已登记的锁等待者。
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Release);
        let waiters = self
            .inner
            .waiters
            .lock()
            .map(|mut waiters| {
                let live = waiters.iter().filter_map(Weak::upgrade).collect::<Vec<_>>();
                waiters.retain(|waiter| waiter.strong_count() > 0);
                live
            })
            .unwrap_or_default();
        for waiter in waiters {
            let _gate = waiter.gate.lock().expect("共享锁等待器锁未中毒");
            waiter.ready.notify_all();
        }
    }

    /// 查询取消状态。
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Debug, Default)]
struct LockWaiters {
    gate: Mutex<()>,
    ready: Condvar,
}

fn lock_waiters(address: usize) -> Arc<LockWaiters> {
    static WAITERS: OnceLock<Mutex<std::collections::BTreeMap<usize, Weak<LockWaiters>>>> =
        OnceLock::new();
    let waiters = WAITERS.get_or_init(Default::default);
    let mut waiters = waiters.lock().expect("共享锁等待器注册表锁未中毒");
    waiters.retain(|_, waiter| waiter.strong_count() > 0);
    if let Some(waiter) = waiters.get(&address).and_then(Weak::upgrade) {
        return waiter;
    }
    let waiter = Arc::new(LockWaiters::default());
    waiters.insert(address, Arc::downgrade(&waiter));
    waiter
}

/// 使用状态字 CAS 获取锁，通过条件变量等待并支持取消唤醒。
fn acquire_lock(
    state: &AtomicU32,
    cancelled: Option<&LockCancellation>,
) -> Result<Arc<LockWaiters>, SharedLockError> {
    let waiters = lock_waiters(state as *const AtomicU32 as usize);
    loop {
        if cancelled.is_some_and(LockCancellation::is_cancelled) {
            return Err(SharedLockError::Cancelled);
        }
        if let Some(cancelled) = cancelled {
            let mut registered = cancelled
                .inner
                .waiters
                .lock()
                .expect("共享锁取消等待器锁未中毒");
            if !registered
                .iter()
                .any(|waiter| waiter.as_ptr() == Arc::as_ptr(&waiters))
            {
                registered.push(Arc::downgrade(&waiters));
            }
            if cancelled.is_cancelled() {
                return Err(SharedLockError::Cancelled);
            }
        }
        let gate = waiters.gate.lock().expect("共享锁等待器锁未中毒");
        if cancelled.is_some_and(LockCancellation::is_cancelled) {
            return Err(SharedLockError::Cancelled);
        }
        if state
            .compare_exchange(0, SHARED_LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return Ok(Arc::clone(&waiters));
        }
        state.fetch_or(SHARED_WAITERS, Ordering::Release);
        drop(waiters.ready.wait(gate).expect("共享锁等待器锁未中毒"));
    }
}

fn increment_checked(counter: &AtomicU64, overflow_message: &str) {
    let mut current = counter.load(Ordering::Acquire);
    loop {
        let next = current.checked_add(1).expect(overflow_message);
        match counter.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn increment_strong(counter: &AtomicU64) {
    let mut current = counter.load(Ordering::Acquire);
    loop {
        assert_ne!(current, 0, "不能克隆已经销毁的共享句柄");
        let next = current.checked_add(1).expect("共享强引用计数溢出");
        match counter.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

/// 静态 Runtime 标签到实际 Runtime ID 的注册表。
#[derive(Debug)]
pub struct RuntimeRegistry {
    next_id: AtomicU64,
    state: Mutex<RuntimeRegistryState>,
}

#[derive(Debug, Default)]
struct RuntimeRegistryState {
    mappings: std::collections::BTreeMap<u64, u64>,
    closed: std::collections::BTreeSet<u64>,
}

impl RuntimeRegistry {
    /// 创建从实际 ID 1 开始的注册表。
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            state: Mutex::new(RuntimeRegistryState::default()),
        }
    }

    /// 为静态标签注册实际 Runtime ID；重复注册保持原 ID。
    pub fn register(&self, static_label: u64) -> Result<u64, SharedHandleError> {
        if static_label == 0 {
            return Err(SharedHandleError::InvalidRuntimeId);
        }
        let mut state = self.state.lock().expect("Runtime 注册表锁未中毒");
        if let Some(id) = state.mappings.get(&static_label).copied() {
            return Ok(id);
        }
        let id = self
            .next_id
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| SharedHandleError::RuntimeIdExhausted)?;
        state.mappings.insert(static_label, id);
        Ok(id)
    }

    /// 查询静态标签对应的实际 ID，且拒绝已关闭 Runtime。
    #[must_use]
    pub fn resolve(&self, static_label: u64) -> Option<u64> {
        let state = self.state.lock().ok()?;
        let id = state.mappings.get(&static_label).copied()?;
        (!state.closed.contains(&id)).then_some(id)
    }

    /// 关闭实际 Runtime；重复关闭保持幂等。
    pub fn close(&self, actual_id: u64) {
        if let Ok(mut state) = self.state.lock() {
            state.closed.insert(actual_id);
            state
                .mappings
                .retain(|_, mapped_id| *mapped_id != actual_id);
        }
    }
}

impl Default for RuntimeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 进程内共享的默认 Runtime 注册表。
#[must_use]
pub fn global_runtime_registry() -> &'static RuntimeRegistry {
    /// 进程级 Runtime 注册表单例。
    static REGISTRY: OnceLock<RuntimeRegistry> = OnceLock::new();
    REGISTRY.get_or_init(RuntimeRegistry::new)
}

#[cfg(test)]
/// 共享头、锁和 Runtime ID 映射回归。
mod tests {
    use super::*;
    use std::mem::{align_of, offset_of, size_of};

    #[test]
    /// 共享头布局保持 32B/8B 契约。
    fn shared_header_layout_is_frozen() {
        assert_eq!(size_of::<SharedHeader>(), 32);
        assert_eq!(align_of::<SharedHeader>(), 8);
        assert_eq!(offset_of!(SharedHeader, strong), 0);
        assert_eq!(offset_of!(SharedHeader, weak), 8);
        assert_eq!(offset_of!(SharedHeader, tag), 16);
        assert_eq!(offset_of!(SharedHeader, lock_state), 24);
    }

    #[test]
    /// 强弱引用与弱升级保持一次性回收。
    fn shared_counts_weak_upgrade_and_exact_drop() {
        let shared = SharedHandle::new(String::from("x"), 7, SharedTag::Mut);
        assert_eq!(shared.strong_count(), 1);
        let weak = shared.downgrade();
        let clone = shared.clone();
        assert_eq!(shared.strong_count(), 2);
        drop(clone);
        assert!(weak.upgrade().is_some());
        assert_eq!(shared.tag(), SharedTag::Mut);
    }

    #[test]
    /// 共享可变锁和取消状态可观察。
    fn shared_lock_supports_mutation_and_cancellation() {
        let shared = SharedHandle::new(1_u64, 9, SharedTag::Mut);
        {
            let mut guard = shared.lock().expect("共享可变句柄应可加锁");
            *guard += 1;
        }
        assert_eq!(shared.get(), Err(SharedHandleError::InvalidTag));
        assert_eq!(*shared.lock().expect("可重新获取锁"), 2);
        let cancelled = LockCancellation::default();
        cancelled.cancel();
        assert_eq!(
            shared.lock_with_cancel(&cancelled).unwrap_err(),
            SharedLockError::Cancelled
        );
    }

    #[test]
    /// 共享只读句柄可以读取，且普通 tag 不被接受为可变共享头。
    fn shared_read_access_is_tag_restricted() {
        let shared = SharedHandle::new(String::from("read"), 9, SharedTag::Read);
        assert_eq!(shared.get().map(String::as_str), Ok("read"));
        assert!(matches!(shared.lock(), Err(SharedHandleError::InvalidTag)));
    }

    fn wait_for_waiter(shared: &SharedHandle<u64>) {
        let state = unsafe { &shared.ptr.as_ref().header.lock_state };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while state.load(Ordering::Acquire) & SHARED_WAITERS == 0 {
            assert!(std::time::Instant::now() < deadline, "等待者未进入锁等待");
            std::thread::yield_now();
        }
    }

    #[test]
    /// 锁竞争进入条件变量等待，解锁唤醒后只有一个守卫修改载荷。
    fn shared_lock_blocks_and_wakes_waiting_thread() {
        let shared = SharedHandle::new(0_u64, 9, SharedTag::Mut);
        let guard = shared.lock().expect("主线程持锁");
        let other = shared.clone();
        let waiter = std::thread::spawn(move || {
            let mut guard = other.lock().expect("等待后获取锁");
            *guard += 1;
        });
        wait_for_waiter(&shared);
        drop(guard);
        waiter.join().expect("等待线程完成");
        assert_eq!(*shared.lock().expect("检查结果"), 1);
    }

    #[test]
    /// 取消会唤醒已阻塞的持锁等待者。
    fn lock_cancellation_wakes_waiting_thread() {
        let shared = SharedHandle::new(0_u64, 9, SharedTag::Mut);
        let guard = shared.lock().expect("主线程持锁");
        let other = shared.clone();
        let cancellation = LockCancellation::default();
        let child_cancellation = cancellation.clone();
        let waiter =
            std::thread::spawn(move || other.lock_with_cancel(&child_cancellation).map(drop));
        wait_for_waiter(&shared);
        cancellation.cancel();
        assert!(matches!(
            waiter.join().expect("等待线程完成"),
            Err(SharedLockError::Cancelled)
        ));
        drop(guard);
    }

    #[test]
    /// 静态标签映射到实际 Runtime ID，关闭后失效。
    fn runtime_registry_maps_and_closes_static_labels() {
        let registry = RuntimeRegistry::new();
        let id = registry.register(99).expect("静态标签应注册");
        assert_eq!(registry.register(99), Ok(id));
        assert_eq!(registry.resolve(99), Some(id));
        registry.close(id);
        assert_eq!(registry.resolve(99), None);
        let replacement = registry.register(99).expect("同一静态标签可进入新 Runtime");
        assert_ne!(replacement, id);
        assert_eq!(registry.resolve(99), Some(replacement));
    }
}
