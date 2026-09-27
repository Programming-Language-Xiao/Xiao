//! 不透明对象头、强/弱句柄和可插拔计数策略。
//!
//! 句柄是 Runtime 与后续 VM/LLVM 的唯一对象边界。对象头不暴露给语言层；
//! 本阶段使用单线程非原子计数，但通过策略接口保留未来原子实现的位置。

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::errors::{RuntimeError, RuntimeResult};

/// Runtime 堆对象的类型标签。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeTypeTag {
    /// 不可变字符串对象。
    String,
    /// 表实例或单例表对象。
    Table,
    /// 数组对象。
    Array,
    /// 元组对象。
    Tuple,
    /// 无序字典表对象。
    DictTable,
    /// 顺序稳定的字典列对象。
    DictColumn,
    /// 集合对象。
    Set,
    /// 为后续 Runtime 扩展保留的用户对象标签。
    Custom(u32),
}

impl RuntimeTypeTag {
    /// 返回稳定的调试名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::String => "str",
            Self::Table => "table",
            Self::Array => "array",
            Self::Tuple => "tuple",
            Self::DictTable => "dict_table",
            Self::DictColumn => "dict_column",
            Self::Set => "set",
            Self::Custom(_) => "custom",
        }
    }
}

/// 对象头中的布局摘要。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ObjectLayout {
    /// 类型标签。
    pub type_tag: RuntimeTypeTag,
    /// 载荷大小（字节）。
    pub payload_size: usize,
    /// 载荷对齐要求（字节）。
    pub payload_align: usize,
}

impl ObjectLayout {
    /// 从类型标签和 Rust 类型布局创建摘要。
    #[must_use]
    pub const fn for_type<T>(type_tag: RuntimeTypeTag) -> Self {
        Self {
            type_tag,
            payload_size: std::mem::size_of::<T>(),
            payload_align: std::mem::align_of::<T>(),
        }
    }
}

/// 引用计数的可插拔算术策略。
///
/// 当前对象头仍使用 `Cell<usize>` 保存计数；未来原子策略可以复用相同的
/// 溢出/下溢契约，并在独立对象头实现中提供原子存储。
pub trait RefCountStrategy: Sync {
    /// 增加一个计数并返回新值。
    fn increment(&self, current: usize) -> RuntimeResult<usize>;
    /// 减少一个计数并返回新值。
    fn decrement(&self, current: usize) -> RuntimeResult<usize>;
    /// 返回策略名称。
    fn name(&self) -> &'static str;
}

/// 首版单线程非原子引用计数策略。
#[derive(Clone, Copy, Debug, Default)]
pub struct NonAtomicRefCount;

impl RefCountStrategy for NonAtomicRefCount {
    /// 使用 checked 加法拒绝计数溢出。
    fn increment(&self, current: usize) -> RuntimeResult<usize> {
        current
            .checked_add(1)
            .ok_or_else(|| RuntimeError::refcount_invariant("强/弱引用计数溢出"))
    }

    /// 使用 checked 减法拒绝计数下溢。
    fn decrement(&self, current: usize) -> RuntimeResult<usize> {
        current
            .checked_sub(1)
            .ok_or_else(|| RuntimeError::refcount_invariant("强/弱引用计数下溢"))
    }

    /// 返回策略名称。
    fn name(&self) -> &'static str {
        "non-atomic"
    }
}

/// 首版对象头共享的非原子计数策略实例。
static NON_ATOMIC_REFCOUNT: NonAtomicRefCount = NonAtomicRefCount;

/// 当前可选择的引用计数策略身份。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CounterStrategyKind {
    /// 单线程非原子策略。
    NonAtomic,
}

impl CounterStrategyKind {
    /// 返回策略实现。
    #[must_use]
    pub fn implementation(self) -> &'static dyn RefCountStrategy {
        match self {
            Self::NonAtomic => &NON_ATOMIC_REFCOUNT,
        }
    }

    /// 返回稳定名称。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.implementation().name()
    }
}

/// Runtime 堆对象载荷的内部协议。
pub(crate) trait ObjectPayload: Any {
    /// 返回载荷类型标签。
    fn type_tag(&self) -> RuntimeTypeTag;
    /// 返回载荷布局摘要。
    fn layout(&self) -> ObjectLayout;
    /// 返回载荷额外拥有的堆存储字节数，不重复计算其他 Runtime 对象。
    fn owned_bytes(&self) -> usize {
        0
    }
    /// 在最后一个强引用释放前执行用户可观察的释放钩子。
    fn on_drop(&mut self) -> RuntimeResult<()>;
    /// 暴露只读 `Any` 视图供同 crate 类型安全下转型。
    fn as_any(&self) -> &dyn Any;
    /// 暴露可变 `Any` 视图供同 crate 类型安全下转型。
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[repr(C)]
/// Runtime 对象的私有不透明头，承载计数和销毁状态。
struct ObjectHeader {
    type_tag: RuntimeTypeTag,
    strong_count: Cell<usize>,
    weak_count: Cell<usize>,
    layout: ObjectLayout,
    strategy: &'static dyn RefCountStrategy,
    destroyed: Cell<bool>,
    accounted_bytes: Cell<u64>,
    measurement: Option<Arc<MemoryCounter>>,
    payload: ManuallyDrop<RefCell<Box<dyn ObjectPayload>>>,
}

/// 一次运行的对象存活字节计数；每个对象头只归属于创建它的运行。
#[derive(Default)]
struct MemoryCounter {
    current: AtomicU64,
    peak: AtomicU64,
}

impl MemoryCounter {
    /// 更新当前存活字节数及历史峰值。
    fn add(&self, bytes: u64) {
        let current = self.current.fetch_add(bytes, Ordering::Relaxed).saturating_add(bytes);
        self.peak.fetch_max(current, Ordering::Relaxed);
    }

    /// 对象释放或收缩时扣除对应字节数。
    fn subtract(&self, bytes: u64) {
        self.current.fetch_sub(bytes, Ordering::Relaxed);
    }
}

thread_local! {
    /// 当前线程的运行测量；对象头保留自身归属，释放时不依赖此线程状态。
    static ACTIVE_MEASUREMENT: RefCell<Option<Arc<MemoryCounter>>> = const { RefCell::new(None) };
}

/// 本次运行的 Runtime 对象内存测量作用域。
pub struct MemoryMeasurement {
    counter: Arc<MemoryCounter>,
    previous: Option<Arc<MemoryCounter>>,
}

impl MemoryMeasurement {
    /// 返回本次运行期间同时存活的 Runtime 对象峰值字节数。
    #[must_use]
    pub fn peak_live_bytes(&self) -> u64 {
        self.counter.peak.load(Ordering::Relaxed)
    }
}

impl Drop for MemoryMeasurement {
    /// 恢复外层测量作用域，嵌套运行互不覆盖。
    fn drop(&mut self) {
        ACTIVE_MEASUREMENT.with(|active| *active.borrow_mut() = self.previous.take());
    }
}

/// 为一次同步运行建立独立的 Runtime 对象内存测量。
#[must_use]
pub fn start_memory_measurement() -> MemoryMeasurement {
    let counter = Arc::new(MemoryCounter::default());
    let previous = ACTIVE_MEASUREMENT.with(|active| active.borrow_mut().replace(counter.clone()));
    MemoryMeasurement { counter, previous }
}

/// 强拥有的不透明 Runtime 句柄。
pub struct StrongHandle {
    ptr: NonNull<ObjectHeader>,
    // 明确把首版句柄标为单线程，避免自动实现 Send/Sync。
    _single_thread: PhantomData<Rc<()>>,
}

/// 不拥有目标对象生命周期的弱 Runtime 句柄。
pub struct WeakHandle {
    ptr: NonNull<ObjectHeader>,
    _single_thread: PhantomData<Rc<()>>,
}

impl Debug for StrongHandle {
    /// 输出不泄漏载荷内容的句柄摘要。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StrongHandle")
            .field("type_tag", &self.type_tag())
            .field("strong_count", &self.strong_count())
            .field("weak_count", &self.weak_count())
            .finish()
    }
}

impl Debug for WeakHandle {
    /// 输出不泄漏载荷内容的弱句柄摘要。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WeakHandle")
            .field("type_tag", &self.type_tag())
            .field("alive", &self.is_alive())
            .field("strong_count", &self.strong_count())
            .finish()
    }
}

impl StrongHandle {
    /// 返回对象类型标签。
    #[must_use]
    pub fn type_tag(&self) -> RuntimeTypeTag {
        // 句柄只要能构造出来就始终指向仍保留对象头的分配。
        unsafe { self.ptr.as_ref().type_tag }
    }

    /// 返回对象布局摘要。
    #[must_use]
    pub fn layout(&self) -> ObjectLayout {
        unsafe { self.ptr.as_ref().layout }
    }

    /// 返回强引用计数。
    #[must_use]
    pub fn strong_count(&self) -> usize {
        unsafe { self.ptr.as_ref().strong_count.get() }
    }

    /// 返回弱引用计数（包含对象头的隐式弱引用）。
    #[must_use]
    pub fn weak_count(&self) -> usize {
        unsafe { self.ptr.as_ref().weak_count.get() }
    }

    /// 返回当前计数策略名称。
    #[must_use]
    pub fn counter_strategy(&self) -> &'static str {
        unsafe { self.ptr.as_ref().strategy.name() }
    }

    /// 判断载荷是否已经被最后一个强引用销毁。
    #[must_use]
    pub fn is_alive(&self) -> bool {
        unsafe { !self.ptr.as_ref().destroyed.get() }
    }

    /// 判断两个强句柄是否指向同一个对象头。
    #[must_use]
    pub(crate) fn same_object(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }

    /// 创建一个不拥有对象生命周期的弱句柄。
    #[must_use]
    pub fn downgrade(&self) -> WeakHandle {
        self.try_downgrade()
            .expect("弱引用计数溢出违反 Runtime 不变量")
    }

    /// 创建弱句柄并显式返回计数错误。
    pub fn try_downgrade(&self) -> RuntimeResult<WeakHandle> {
        let header = unsafe { self.ptr.as_ref() };
        let count = header.strategy.increment(header.weak_count.get())?;
        header.weak_count.set(count);
        Ok(WeakHandle {
            ptr: self.ptr,
            _single_thread: PhantomData,
        })
    }

    /// 尝试克隆强句柄并显式处理计数溢出。
    pub fn try_clone(&self) -> RuntimeResult<Self> {
        let header = unsafe { self.ptr.as_ref() };
        if header.destroyed.get() || header.strong_count.get() == 0 {
            return Err(RuntimeError::use_after_release());
        }
        let count = header.strategy.increment(header.strong_count.get())?;
        header.strong_count.set(count);
        Ok(Self {
            ptr: self.ptr,
            _single_thread: PhantomData,
        })
    }

    /// 立即释放一个强句柄；若它是最后一个强句柄，返回 `drop` 错误。
    pub fn try_release(self) -> RuntimeResult<()> {
        let this = ManuallyDrop::new(self);
        unsafe { release_strong(this.ptr) }
    }

    /// 在载荷仍存活时只读访问一个内部对象类型。
    pub(crate) fn with_payload<T: Any, R>(
        &self,
        expected: RuntimeTypeTag,
        callback: impl FnOnce(&T) -> R,
    ) -> RuntimeResult<R> {
        with_payload(self.ptr, expected, callback)
    }

    /// 在载荷仍存活时可变访问一个内部对象类型。
    pub(crate) fn with_payload_mut<T: Any, R>(
        &self,
        expected: RuntimeTypeTag,
        callback: impl FnOnce(&mut T) -> R,
    ) -> RuntimeResult<R> {
        with_payload_mut(self.ptr, expected, callback)
    }

    /// 在内部可变对象通过间接可变性修改后刷新其计量大小。
    pub(crate) fn refresh_memory_usage(&self) {
        refresh_memory_usage(self.ptr);
    }
}

impl Clone for StrongHandle {
    /// 克隆强句柄并在计数溢出时显式 panic；可恢复调用方请使用 [`Self::try_clone`]。
    fn clone(&self) -> Self {
        self.try_clone().expect("强引用计数溢出违反 Runtime 不变量")
    }
}

impl Drop for StrongHandle {
    /// 在句柄离开 Rust 作用域时执行确定性释放。
    fn drop(&mut self) {
        // Drop trait 无法返回错误；显式 API `try_release` 会把最后一个
        // `drop` 错误交给调用方，隐式路径仍完成释放并保持对象不泄漏。
        let _ = unsafe { release_strong(self.ptr) };
    }
}

impl WeakHandle {
    /// 返回对象类型标签；对象头在弱句柄存活期间仍然存在。
    #[must_use]
    pub fn type_tag(&self) -> RuntimeTypeTag {
        unsafe { self.ptr.as_ref().type_tag }
    }

    /// 返回强引用计数。
    #[must_use]
    pub fn strong_count(&self) -> usize {
        unsafe { self.ptr.as_ref().strong_count.get() }
    }

    /// 判断目标载荷是否仍存活。
    #[must_use]
    pub fn is_alive(&self) -> bool {
        unsafe {
            let header = self.ptr.as_ref();
            !header.destroyed.get() && header.strong_count.get() > 0
        }
    }

    /// 尝试克隆弱句柄并显式处理计数溢出。
    pub fn try_clone(&self) -> RuntimeResult<Self> {
        let header = unsafe { self.ptr.as_ref() };
        let count = header.strategy.increment(header.weak_count.get())?;
        header.weak_count.set(count);
        Ok(Self {
            ptr: self.ptr,
            _single_thread: PhantomData,
        })
    }

    /// 尝试将弱句柄升级为强句柄。
    pub fn upgrade(&self) -> RuntimeResult<StrongHandle> {
        let header = unsafe { self.ptr.as_ref() };
        if header.destroyed.get() || header.strong_count.get() == 0 {
            return Err(RuntimeError::weak_upgrade());
        }
        let count = header.strategy.increment(header.strong_count.get())?;
        header.strong_count.set(count);
        Ok(StrongHandle {
            ptr: self.ptr,
            _single_thread: PhantomData,
        })
    }
}

impl Clone for WeakHandle {
    /// 克隆弱句柄并在计数溢出时显式 panic。
    fn clone(&self) -> Self {
        let header = unsafe { self.ptr.as_ref() };
        let count = header
            .strategy
            .increment(header.weak_count.get())
            .expect("弱引用计数溢出违反 Runtime 不变量");
        header.weak_count.set(count);
        Self {
            ptr: self.ptr,
            _single_thread: PhantomData,
        }
    }
}

impl Drop for WeakHandle {
    /// 释放弱句柄；最后一个弱句柄同时释放对象头分配。
    fn drop(&mut self) {
        let header = unsafe { self.ptr.as_ref() };
        let count = header
            .strategy
            .decrement(header.weak_count.get())
            .expect("弱引用计数下溢违反 Runtime 不变量");
        header.weak_count.set(count);
        if count == 0 {
            unsafe { free_header(self.ptr) };
        }
    }
}

/// 为一个内部载荷分配对象头和首个强句柄。
pub(crate) fn allocate_payload(payload: Box<dyn ObjectPayload>) -> RuntimeResult<StrongHandle> {
    let layout = payload.layout();
    let bytes = object_bytes(&*payload);
    let measurement = ACTIVE_MEASUREMENT.with(|active| active.borrow().clone());
    let header = Box::new(ObjectHeader {
        type_tag: payload.type_tag(),
        strong_count: Cell::new(1),
        // 一个隐式弱引用保证强引用归零后对象头仍可供 Weak 查询。
        weak_count: Cell::new(1),
        layout,
        strategy: CounterStrategyKind::NonAtomic.implementation(),
        destroyed: Cell::new(false),
        accounted_bytes: Cell::new(bytes),
        measurement: measurement.clone(),
        payload: ManuallyDrop::new(RefCell::new(payload)),
    });
    let ptr = NonNull::new(Box::into_raw(header)).ok_or_else(|| {
        RuntimeError::new(
            crate::errors::RuntimeErrorKind::Resource,
            crate::errors::ALLOCATION_CODE,
            "runtime.allocation",
            "无法分配 Runtime 对象",
        )
    })?;
    if let Some(counter) = measurement {
        counter.add(bytes);
    }
    Ok(StrongHandle {
        ptr,
        _single_thread: PhantomData,
    })
}

/// 在类型标签和载荷布局验证后只读访问对象载荷。
fn with_payload<T: Any, R>(
    ptr: NonNull<ObjectHeader>,
    expected: RuntimeTypeTag,
    callback: impl FnOnce(&T) -> R,
) -> RuntimeResult<R> {
    let header = unsafe { ptr.as_ref() };
    if header.destroyed.get() {
        return Err(RuntimeError::use_after_release());
    }
    if header.type_tag != expected {
        return Err(RuntimeError::type_mismatch(
            expected.as_str(),
            header.type_tag.as_str(),
        ));
    }
    let payload = (*header.payload).borrow();
    let typed = payload
        .as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| RuntimeError::type_mismatch(expected.as_str(), "载荷布局"))?;
    Ok(callback(typed))
}

/// 在类型标签和载荷布局验证后可变访问对象载荷。
fn with_payload_mut<T: Any, R>(
    ptr: NonNull<ObjectHeader>,
    expected: RuntimeTypeTag,
    callback: impl FnOnce(&mut T) -> R,
) -> RuntimeResult<R> {
    let header = unsafe { ptr.as_ref() };
    if header.destroyed.get() {
        return Err(RuntimeError::use_after_release());
    }
    if header.type_tag != expected {
        return Err(RuntimeError::type_mismatch(
            expected.as_str(),
            header.type_tag.as_str(),
        ));
    }
    let mut payload = (*header.payload).borrow_mut();
    let typed = payload
        .as_any_mut()
        .downcast_mut::<T>()
        .ok_or_else(|| RuntimeError::type_mismatch(expected.as_str(), "载荷布局"))?;
    let result = callback(typed);
    let bytes = object_bytes(&**payload);
    update_memory_usage(header, bytes);
    Ok(result)
}

/// 计算对象头、静态载荷与载荷额外拥有的堆存储。
fn object_bytes(payload: &dyn ObjectPayload) -> u64 {
    let bytes = std::mem::size_of::<ObjectHeader>()
        .saturating_add(payload.layout().payload_size)
        .saturating_add(payload.owned_bytes());
    u64::try_from(bytes).unwrap_or(u64::MAX)
}

/// 按对象归属的计量作用域刷新变长载荷大小。
fn update_memory_usage(header: &ObjectHeader, bytes: u64) {
    let previous = header.accounted_bytes.replace(bytes);
    if let Some(counter) = &header.measurement {
        if bytes > previous {
            counter.add(bytes - previous);
        } else if previous > bytes {
            counter.subtract(previous - bytes);
        }
    }
}

/// 支持表字段等内部可变载荷的显式刷新。
fn refresh_memory_usage(ptr: NonNull<ObjectHeader>) {
    let header = unsafe { ptr.as_ref() };
    if !header.destroyed.get() {
        let payload = (*header.payload).borrow();
        update_memory_usage(header, object_bytes(&**payload));
    }
}

/// 减少强计数，并在最后一个强引用释放时销毁载荷。
unsafe fn release_strong(ptr: NonNull<ObjectHeader>) -> RuntimeResult<()> {
    let header = unsafe { ptr.as_ref() };
    let current = header.strong_count.get();
    let next = header.strategy.decrement(current)?;
    header.strong_count.set(next);
    if next != 0 {
        return Ok(());
    }

    let drop_result = unsafe { destroy_payload(ptr) };
    let weak_next = header.strategy.decrement(header.weak_count.get())?;
    header.weak_count.set(weak_next);
    if weak_next == 0 {
        unsafe { free_header(ptr) };
    }
    drop_result.map_or(Ok(()), Err)
}

/// 执行一次载荷释放钩子并拆除载荷存储。
unsafe fn destroy_payload(ptr: NonNull<ObjectHeader>) -> Option<RuntimeError> {
    let header = unsafe { ptr.as_ref() };
    if header.destroyed.replace(true) {
        return Some(RuntimeError::refcount_invariant("对象载荷被重复释放"));
    }
    let cell = &*header.payload;
    let mut payload = cell.borrow_mut();
    let result = payload.on_drop().err();
    drop(payload);
    unsafe { ManuallyDrop::drop(&mut (*ptr.as_ptr()).payload) };
    if let Some(counter) = &header.measurement {
        counter.subtract(header.accounted_bytes.get());
    }
    result
}

/// 在强弱计数都归零后释放对象头。
unsafe fn free_header(ptr: NonNull<ObjectHeader>) {
    // payload 已经由 destroy_payload 显式取出；ManuallyDrop 防止二次析构。
    unsafe { drop(Box::from_raw(ptr.as_ptr())) };
}

#[cfg(test)]
/// 对象头计数、弱引用存活和载荷释放的回归测试。
mod tests {
    use super::{ObjectLayout, ObjectPayload, RuntimeTypeTag, allocate_payload, start_memory_measurement};
    use crate::containers::ArrayHandle;
    use crate::errors::RuntimeResult;
    use crate::value::RuntimeValue;
    use std::any::Any;

    /// 用于观察载荷释放次数的测试对象。
    struct Probe {
        dropped: std::rc::Rc<std::cell::Cell<u32>>,
    }

    /// 在释放钩子中临时分配 Runtime 对象的测试载荷。
    struct AllocatingDrop {
        bytes: Vec<u8>,
    }

    impl ObjectPayload for AllocatingDrop {
        /// 返回测试对象标签。
        fn type_tag(&self) -> RuntimeTypeTag {
            RuntimeTypeTag::Custom(2)
        }

        /// 返回测试载荷布局。
        fn layout(&self) -> ObjectLayout {
            ObjectLayout::for_type::<Self>(RuntimeTypeTag::Custom(2))
        }

        /// 将测试载荷的字节缓冲区计入对象内存。
        fn owned_bytes(&self) -> usize {
            self.bytes.capacity()
        }

        /// 在原对象仍存活时构造并释放临时数组。
        fn on_drop(&mut self) -> RuntimeResult<()> {
            let temporary = ArrayHandle::new(vec![RuntimeValue::Int(1); 100_000])?;
            drop(temporary);
            Ok(())
        }

        /// 暴露测试载荷的只读视图。
        fn as_any(&self) -> &dyn Any {
            self
        }

        /// 暴露测试载荷的可变视图。
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    impl ObjectPayload for Probe {
        /// 返回测试对象标签。
        fn type_tag(&self) -> RuntimeTypeTag {
            RuntimeTypeTag::Custom(1)
        }

        /// 返回测试对象布局。
        fn layout(&self) -> ObjectLayout {
            ObjectLayout::for_type::<Self>(RuntimeTypeTag::Custom(1))
        }

        /// 记录一次测试对象释放。
        fn on_drop(&mut self) -> RuntimeResult<()> {
            self.dropped.set(self.dropped.get() + 1);
            Ok(())
        }

        /// 暴露只读测试对象视图。
        fn as_any(&self) -> &dyn Any {
            self
        }

        /// 暴露可变测试对象视图。
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    /// 最后一个强引用释放载荷，但弱句柄仍可观察对象已失活。
    fn releases_payload_on_last_strong_and_keeps_weak_header() {
        let dropped = std::rc::Rc::new(std::cell::Cell::new(0));
        let handle = allocate_payload(Box::new(Probe {
            dropped: dropped.clone(),
        }))
        .expect("应分配对象");
        let weak = handle.downgrade();
        assert_eq!(handle.counter_strategy(), "non-atomic");
        assert!(weak.is_alive());
        handle.try_release().expect("释放应成功");
        assert_eq!(dropped.get(), 1);
        assert!(!weak.is_alive());
        assert!(weak.upgrade().is_err());
    }

    #[test]
    /// 峰值包含容器元素缓冲区，释放后新测量不会混入上一轮分配。
    fn peak_live_bytes_counts_container_contents_per_run() {
        let empty_sample = start_memory_measurement();
        let empty = ArrayHandle::new(Vec::new()).expect("空数组应分配");
        let empty_peak = empty_sample.peak_live_bytes();
        drop(empty);
        drop(empty_sample);

        let full_sample = start_memory_measurement();
        let full = ArrayHandle::new(vec![RuntimeValue::Int(1); 1_000_000])
            .expect("百万元素数组应分配");
        assert!(full_sample.peak_live_bytes() > empty_peak + 1_000_000);
        let before_growth = full_sample.peak_live_bytes();
        full.with_elements_mut(|elements| elements.reserve(1_000_000))
            .expect("数组扩容应成功");
        assert!(full_sample.peak_live_bytes() > before_growth);
        drop(full);
        assert_eq!(full_sample.counter.current.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[test]
    /// 释放钩子期间的原对象与新对象同时计入峰值。
    fn peak_live_bytes_includes_objects_allocated_during_drop() {
        let temporary_sample = start_memory_measurement();
        let temporary =
            ArrayHandle::new(vec![RuntimeValue::Int(1); 100_000]).expect("临时数组应分配");
        let temporary_bytes = temporary_sample.peak_live_bytes();
        drop(temporary);
        drop(temporary_sample);

        let sample = start_memory_measurement();
        let parent = allocate_payload(Box::new(AllocatingDrop {
            bytes: vec![0; 1_000_000],
        }))
        .expect("父对象应分配");
        let parent_bytes = sample.peak_live_bytes();
        parent.try_release().expect("释放钩子应成功");
        assert!(sample.peak_live_bytes() >= parent_bytes + temporary_bytes);
        assert_eq!(
            sample
                .counter
                .current
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }
}
