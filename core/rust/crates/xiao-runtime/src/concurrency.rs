//! 22C 并发 Runtime 的共享句柄和 Runtime 身份基础。
//!
//! 本模块不实现 Xiao 标准库函数，也不改变现有单线程 `StrongHandle`/`WeakHandle`。
//! 共享句柄拥有独立的 32B 对象头：对象头只保存强/弱计数、tag 和锁状态，Runtime ID
//! 保存在句柄包装中，符合 22A 的 ABI 约束。

use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;

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

const _: () = assert!(std::mem::size_of::<SharedHeader>() == 32);
const _: () = assert!(std::mem::align_of::<SharedHeader>() == 8);

impl SharedHeader {
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
}

impl fmt::Display for SharedHandleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRuntimeId => formatter.write_str("Runtime ID 0 无效"),
            Self::InvalidTag => formatter.write_str("共享句柄 tag 无效"),
            Self::ObjectDestroyed => formatter.write_str("共享对象已经销毁"),
        }
    }
}

impl std::error::Error for SharedHandleError {}

/// 共享锁被取消。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SharedLockCancelled;

impl fmt::Display for SharedLockCancelled {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("共享锁等待被取消")
    }
}

impl std::error::Error for SharedLockCancelled {}

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

    /// 返回只读载荷。
    #[must_use]
    pub fn get(&self) -> &T {
        unsafe { &self.ptr.as_ref().payload }
    }

    /// 创建弱共享句柄。
    #[must_use]
    pub fn downgrade(&self) -> SharedWeakHandle<T> {
        let header = unsafe { &self.ptr.as_ref().header };
        let previous = header.weak.fetch_add(1, Ordering::AcqRel);
        assert_ne!(previous, u64::MAX, "共享弱引用计数溢出");
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
        acquire_lock(unsafe { &self.ptr.as_ref().header.lock_state }, None)
            .map_err(|_| SharedHandleError::ObjectDestroyed)?;
        Ok(SharedGuard { handle: self })
    }

    /// 以共享可变 tag 获取可取消的独占锁。
    pub fn lock_with_cancel(
        &self,
        cancelled: &AtomicBool,
    ) -> Result<SharedGuard<'_, T>, SharedLockCancelled> {
        acquire_lock(
            unsafe { &self.ptr.as_ref().header.lock_state },
            Some(cancelled),
        )?;
        Ok(SharedGuard { handle: self })
    }
}

impl<T> SharedHandle<T> {
    fn release_strong(&mut self) {
        let allocation = unsafe { self.ptr.as_ref() };
        if allocation.header.strong.fetch_sub(1, Ordering::AcqRel) == 1 {
            unsafe {
                std::ptr::drop_in_place(std::ptr::addr_of_mut!((*self.ptr.as_ptr()).payload));
            }
            release_weak(self.ptr);
        }
    }
}

impl<T> Clone for SharedHandle<T> {
    fn clone(&self) -> Self {
        let header = unsafe { &self.ptr.as_ref().header };
        let previous = header.strong.fetch_add(1, Ordering::AcqRel);
        assert_ne!(previous, 0, "不能克隆已经销毁的共享句柄");
        Self {
            ptr: self.ptr,
            runtime_id: self.runtime_id,
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for SharedHandle<T> {
    fn drop(&mut self) {
        self.release_strong();
    }
}

impl<T> fmt::Debug for SharedHandle<T> {
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
            if current == 0 {
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
    fn clone(&self) -> Self {
        let header = unsafe { &self.ptr.as_ref().header };
        let previous = header.weak.fetch_add(1, Ordering::AcqRel);
        assert_ne!(previous, u64::MAX, "共享弱引用计数溢出");
        Self {
            ptr: self.ptr,
            runtime_id: self.runtime_id,
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for SharedWeakHandle<T> {
    fn drop(&mut self) {
        release_weak(self.ptr);
    }
}

fn release_weak<T>(ptr: NonNull<SharedAllocation<T>>) {
    let header = unsafe { &ptr.as_ref().header };
    if header.weak.fetch_sub(1, Ordering::AcqRel) == 1 {
        let layout = std::alloc::Layout::new::<SharedAllocation<T>>();
        unsafe { std::alloc::dealloc(ptr.as_ptr().cast::<u8>(), layout) };
    }
}

/// 共享可变句柄的独占访问守卫。
#[derive(Debug)]
pub struct SharedGuard<'a, T: Send + Sync> {
    handle: &'a SharedHandle<T>,
}

impl<T: Send + Sync> std::ops::Deref for SharedGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.handle.get()
    }
}

impl<T: Send + Sync> std::ops::DerefMut for SharedGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.handle.ptr.as_ptr()).payload }
    }
}

impl<T: Send + Sync> Drop for SharedGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            self.handle
                .ptr
                .as_ref()
                .header
                .lock_state
                .store(0, Ordering::Release)
        };
    }
}

fn acquire_lock(
    state: &AtomicU32,
    cancelled: Option<&AtomicBool>,
) -> Result<(), SharedLockCancelled> {
    loop {
        if cancelled.is_some_and(|token| token.load(Ordering::Acquire)) {
            return Err(SharedLockCancelled);
        }
        if state
            .compare_exchange(0, SHARED_LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return Ok(());
        }
        state.fetch_or(SHARED_WAITERS, Ordering::Release);
        thread::yield_now();
    }
}

/// 静态 Runtime 标签到实际 Runtime ID 的注册表。
#[derive(Debug, Default)]
pub struct RuntimeRegistry {
    next_id: AtomicU64,
    mappings: Mutex<std::collections::BTreeMap<u64, u64>>,
    closed: Mutex<std::collections::BTreeSet<u64>>,
}

impl RuntimeRegistry {
    /// 创建从实际 ID 1 开始的注册表。
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            mappings: Mutex::new(std::collections::BTreeMap::new()),
            closed: Mutex::new(std::collections::BTreeSet::new()),
        }
    }

    /// 为静态标签注册实际 Runtime ID；重复注册保持原 ID。
    pub fn register(&self, static_label: u64) -> Result<u64, SharedHandleError> {
        if static_label == 0 {
            return Err(SharedHandleError::InvalidRuntimeId);
        }
        let mut mappings = self.mappings.lock().expect("Runtime 注册表锁未中毒");
        if let Some(id) = mappings.get(&static_label).copied() {
            return Ok(id);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        mappings.insert(static_label, id);
        Ok(id)
    }

    /// 查询静态标签对应的实际 ID，且拒绝已关闭 Runtime。
    #[must_use]
    pub fn resolve(&self, static_label: u64) -> Option<u64> {
        let id = self
            .mappings
            .lock()
            .ok()
            .and_then(|mappings| mappings.get(&static_label).copied())?;
        self.closed
            .lock()
            .ok()
            .filter(|closed| !closed.contains(&id))
            .map(|_| id)
    }

    /// 关闭实际 Runtime；重复关闭保持幂等。
    pub fn close(&self, actual_id: u64) {
        if let Ok(mut closed) = self.closed.lock() {
            closed.insert(actual_id);
        }
    }
}

/// 进程内共享的默认 Runtime 注册表。
#[must_use]
pub fn global_runtime_registry() -> &'static RuntimeRegistry {
    static REGISTRY: OnceLock<RuntimeRegistry> = OnceLock::new();
    REGISTRY.get_or_init(RuntimeRegistry::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, offset_of, size_of};

    #[test]
    fn shared_header_layout_is_frozen() {
        assert_eq!(size_of::<SharedHeader>(), 32);
        assert_eq!(align_of::<SharedHeader>(), 8);
        assert_eq!(offset_of!(SharedHeader, strong), 0);
        assert_eq!(offset_of!(SharedHeader, weak), 8);
        assert_eq!(offset_of!(SharedHeader, tag), 16);
        assert_eq!(offset_of!(SharedHeader, lock_state), 24);
    }

    #[test]
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
    fn shared_lock_supports_mutation_and_cancellation() {
        let shared = SharedHandle::new(1_u64, 9, SharedTag::Mut);
        {
            let mut guard = shared.lock().expect("共享可变句柄应可加锁");
            *guard += 1;
        }
        assert_eq!(*shared.get(), 2);
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            shared.lock_with_cancel(&cancelled).unwrap_err(),
            SharedLockCancelled
        );
    }

    #[test]
    fn runtime_registry_maps_and_closes_static_labels() {
        let registry = RuntimeRegistry::new();
        let id = registry.register(99).expect("静态标签应注册");
        assert_eq!(registry.register(99), Ok(id));
        assert_eq!(registry.resolve(99), Some(id));
        registry.close(id);
        assert_eq!(registry.resolve(99), None);
    }
}
