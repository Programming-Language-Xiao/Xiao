//! 07-D.6 的隔离研究原型。
//!
//! 这里的类型只用于验证边界、清理顺序和成本测量，不属于 Xiao Runtime，也不提供
//! 语言或公开标准库契约。原子共享部分使用 Rust 标准库的 `Arc`/`Weak`/`Mutex` 作为
//! 参考实现，便于把线程安全的计数、锁和析构顺序变成可重复的观察量。

use std::fmt;
use std::hint::black_box;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex, MutexGuard, TryLockError, Weak};
use std::thread;
use std::time::{Duration, Instant};

/// 自定义原子对象头/ABI 的隔离模型模块。
mod atomic_abi;

/// 可跨线程深拷贝的最小嵌套值模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NestedValue {
    /// 标量。
    Int(i64),
    /// 不可变文本。
    Text(String),
    /// 递归只读容器的原型表示。
    Array(Vec<NestedValue>),
}

impl NestedValue {
    /// 显式递归复制，避免把 `Clone` 的实现细节当成跨线程契约。
    pub fn deep_copy(&self) -> Self {
        match self {
            Self::Int(value) => Self::Int(*value),
            Self::Text(value) => Self::Text(value.clone()),
            Self::Array(items) => Self::Array(items.iter().map(Self::deep_copy).collect()),
        }
    }

    /// 修改副本中的第一个标量，用于证明发送方别名没有被修改。
    pub fn set_first_int(&mut self, value: i64) -> bool {
        match self {
            Self::Int(current) => {
                *current = value;
                true
            }
            Self::Text(_) => false,
            Self::Array(items) => items.iter_mut().any(|item| item.set_first_int(value)),
        }
    }
}

/// 线程入口与深拷贝隔离场景。
pub fn thread_entry_and_deep_copy_demo() -> Result<(), String> {
    let source = NestedValue::Array(vec![
        NestedValue::Int(7),
        NestedValue::Array(vec![NestedValue::Text("xiao".to_owned())]),
    ]);
    let expected_source = source.deep_copy();
    let outbound = source.deep_copy();
    let join = thread::spawn(move || {
        let mut local = outbound;
        if !local.set_first_int(99) {
            return Err("线程副本没有找到可修改标量".to_owned());
        }
        Ok(local)
    });
    let received = join.join().map_err(|_| "线程入口发生 panic".to_owned())??;
    if source != expected_source {
        return Err("线程修改泄漏到发送方".to_owned());
    }
    if received == source {
        return Err("线程副本没有发生独立修改".to_owned());
    }
    Ok(())
}

/// 复制事务中的一个已复制部分。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyPart {
    /// 用于观察清理顺序的稳定编号。
    pub id: usize,
    /// 该部分清理时是否额外产生 suppressed 错误。
    pub cleanup_fails: bool,
}

/// 深拷贝失败后的主错误和清理记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyFailureReport {
    /// 复制失败的主错误。
    pub primary_error: String,
    /// 已复制前缀的逆序清理编号。
    pub cleanup_order: Vec<usize>,
    /// 清理阶段产生的附加错误。
    pub suppressed_errors: Vec<String>,
}

/// 模拟复制到指定部分失败，并按逆序清理已复制前缀。
pub fn simulate_copy_failure(parts: &[CopyPart], fail_at: usize) -> CopyFailureReport {
    let mut copied_prefix = Vec::new();
    let mut found_failure = false;
    for part in parts {
        if part.id == fail_at {
            found_failure = true;
            break;
        }
        copied_prefix.push(part.clone());
    }

    let mut cleanup_order = Vec::new();
    let mut suppressed_errors = Vec::new();
    while let Some(part) = copied_prefix.pop() {
        cleanup_order.push(part.id);
        if part.cleanup_fails {
            suppressed_errors.push(format!("清理部分 {} 失败", part.id));
        }
    }

    CopyFailureReport {
        primary_error: if found_failure {
            format!("复制部分 {} 失败", fail_at)
        } else {
            format!("未找到失败位置 {}", fail_at)
        },
        cleanup_order,
        suppressed_errors,
    }
}

/// 深拷贝失败、前缀逆序清理和主错误保留场景。
pub fn deep_copy_failure_demo() -> Result<(), String> {
    let report = simulate_copy_failure(
        &[
            CopyPart {
                id: 1,
                cleanup_fails: false,
            },
            CopyPart {
                id: 2,
                cleanup_fails: true,
            },
            CopyPart {
                id: 3,
                cleanup_fails: false,
            },
            CopyPart {
                id: 4,
                cleanup_fails: false,
            },
        ],
        4,
    );
    if report.primary_error != "复制部分 4 失败" {
        return Err("主复制错误没有保留".to_owned());
    }
    if report.cleanup_order != vec![3, 2, 1] {
        return Err(format!("前缀清理顺序错误：{:?}", report.cleanup_order));
    }
    if report.suppressed_errors != vec!["清理部分 2 失败"] {
        return Err("清理错误没有作为 suppressed 记录".to_owned());
    }
    Ok(())
}

/// 拆分后的生命周期事实。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryFacts {
    /// 是否跨过任务挂起/恢复边界。
    pub task_boundary: bool,
    /// 是否跨过 OS 线程边界。
    pub thread_boundary: bool,
}

/// 同一线程内挂起/恢复只产生任务边界。
pub fn same_thread_suspend() -> BoundaryFacts {
    BoundaryFacts {
        task_boundary: true,
        thread_boundary: false,
    }
}

/// 显式把值迁移到另一个 OS 线程时产生两个事实。
pub fn explicit_thread_migration() -> BoundaryFacts {
    BoundaryFacts {
        task_boundary: true,
        thread_boundary: true,
    }
}

/// `Send`/`Sync` 候选 A 的最小递归值集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateValue {
    /// 可复制标量。
    Scalar,
    /// 不可变文本。
    ImmutableText,
    /// 递归只读容器。
    ReadOnlyContainer(Vec<CandidateValue>),
    /// 可变容器。
    MutableContainer,
    /// 普通单线程句柄。
    OrdinaryHandle,
    /// 显式共享句柄。
    SharedHandle,
    /// 无法证明能力的动态值。
    Dynamic,
    /// 含外部资源的值。
    Resource,
}

/// 候选 A 对值的递归能力推导。
pub fn candidate_a_accepts(value: &CandidateValue) -> bool {
    match value {
        CandidateValue::Scalar | CandidateValue::ImmutableText | CandidateValue::SharedHandle => {
            true
        }
        CandidateValue::ReadOnlyContainer(items) => items.iter().all(candidate_a_accepts),
        CandidateValue::MutableContainer
        | CandidateValue::OrdinaryHandle
        | CandidateValue::Dynamic
        | CandidateValue::Resource => false,
    }
}

/// 固定类型集合的接受/拒绝表行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRow {
    /// 表中显示名称。
    pub label: &'static str,
    /// 候选 A 是否接受该值。
    pub accepted: bool,
}

/// 生成候选 A 的最小接受/拒绝表。
pub fn candidate_a_table() -> Vec<CandidateRow> {
    let cases = [
        ("scalar", CandidateValue::Scalar),
        ("immutable_text", CandidateValue::ImmutableText),
        (
            "readonly_nested_container",
            CandidateValue::ReadOnlyContainer(vec![
                CandidateValue::Scalar,
                CandidateValue::ReadOnlyContainer(vec![CandidateValue::ImmutableText]),
            ]),
        ),
        ("mutable_container", CandidateValue::MutableContainer),
        ("ordinary_handle", CandidateValue::OrdinaryHandle),
        ("shared_handle", CandidateValue::SharedHandle),
        ("dynamic", CandidateValue::Dynamic),
        ("resource", CandidateValue::Resource),
    ];
    cases
        .into_iter()
        .map(|(label, value)| CandidateRow {
            label,
            accepted: candidate_a_accepts(&value),
        })
        .collect()
}

/// 候选 A 推导和 `CrossThread` 拆分场景。
pub fn boundary_and_candidate_demo() -> Result<(), String> {
    if same_thread_suspend()
        != (BoundaryFacts {
            task_boundary: true,
            thread_boundary: false,
        })
    {
        return Err("同线程挂起错误地产生了线程边界".to_owned());
    }
    if explicit_thread_migration()
        != (BoundaryFacts {
            task_boundary: true,
            thread_boundary: true,
        })
    {
        return Err("显式线程迁移没有产生两个边界事实".to_owned());
    }
    let table = candidate_a_table();
    let accepted = table
        .iter()
        .filter(|row| row.accepted)
        .map(|row| row.label)
        .collect::<Vec<_>>();
    let rejected = table
        .iter()
        .filter(|row| !row.accepted)
        .map(|row| row.label)
        .collect::<Vec<_>>();
    if accepted
        != [
            "scalar",
            "immutable_text",
            "readonly_nested_container",
            "shared_handle",
        ]
    {
        return Err(format!("候选 A 接受表错误：{accepted:?}"));
    }
    if rejected
        != [
            "mutable_container",
            "ordinary_handle",
            "dynamic",
            "resource",
        ]
    {
        return Err(format!("候选 A 拒绝表错误：{rejected:?}"));
    }
    Ok(())
}

/// 只读句柄的实验状态：载荷和析构计数指针。
struct ReadState<T> {
    payload: T,
    drops: Arc<AtomicUsize>,
}

impl<T> Drop for ReadState<T> {
    /// 记录实验状态的最终析构。
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// 只读原子共享句柄的隔离模型。
pub struct ReadHandle<T> {
    inner: Arc<ReadState<T>>,
}

impl<T> Clone for ReadHandle<T> {
    /// 复制实验句柄包装器。
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> ReadHandle<T> {
    /// 创建一个带析构计数的只读共享句柄。
    pub fn new(payload: T, drops: Arc<AtomicUsize>) -> Self {
        Self {
            inner: Arc::new(ReadState { payload, drops }),
        }
    }

    /// 读取只读载荷。
    pub fn get(&self) -> &T {
        &self.inner.payload
    }

    /// 降级为弱句柄。
    pub fn downgrade(&self) -> ReadWeak<T> {
        ReadWeak {
            inner: Arc::downgrade(&self.inner),
        }
    }

    /// 观察原子强计数。
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// 观察原子弱计数。
    pub fn weak_count(&self) -> usize {
        Arc::weak_count(&self.inner)
    }
}

/// 只读共享句柄的弱引用。
pub struct ReadWeak<T> {
    inner: Weak<ReadState<T>>,
}

impl<T> Clone for ReadWeak<T> {
    /// 复制实验弱句柄包装器。
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> ReadWeak<T> {
    /// 尝试原子升级；强引用归零后必定失败。
    pub fn upgrade(&self) -> Option<ReadHandle<T>> {
        self.inner.upgrade().map(|inner| ReadHandle { inner })
    }
}

/// 只读原子句柄、弱升级竞态和恰好一次析构场景。
pub fn atomic_read_demo() -> Result<(), String> {
    /// 并发升级线程数量。
    const WORKERS: usize = 8;
    let drops = Arc::new(AtomicUsize::new(0));
    let handle = ReadHandle::new(41_u64, Arc::clone(&drops));
    let weak = handle.downgrade();
    if handle.strong_count() != 1 || handle.weak_count() != 1 || *handle.get() != 41 {
        return Err("只读句柄的强/弱计数或载荷错误".to_owned());
    }

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
    drop(handle);
    for join in joins {
        join.join().map_err(|_| "弱升级线程 panic".to_owned())?;
    }
    if successes.load(Ordering::SeqCst) + failures.load(Ordering::SeqCst) != WORKERS {
        return Err("弱升级竞态没有收集完整结果".to_owned());
    }
    if weak.upgrade().is_some() {
        return Err("强引用归零后弱句柄仍然可以升级".to_owned());
    }
    drop(weak);
    if drops.load(Ordering::SeqCst) != 1 {
        return Err(format!(
            "只读载荷没有恰好一次析构：{}",
            drops.load(Ordering::SeqCst)
        ));
    }
    Ok(())
}

/// 共享可变句柄的实验状态：显式锁和析构计数器。
struct MutableState<T> {
    value: Mutex<T>,
    drops: Arc<AtomicUsize>,
}

impl<T> Drop for MutableState<T> {
    /// 记录共享可变状态的最终析构。
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// 共享可变句柄，只通过显式 `Mutex` 保护访问。
pub struct SharedMutableHandle<T> {
    inner: Arc<MutableState<T>>,
}

/// 串行化预期 panic 的临时 hook，避免污染场景输出。
static EXPECTED_PANIC_HOOK_LOCK: Mutex<()> = Mutex::new(());

/// 捕获并静默记录一个预期异常路径。
fn catches_expected_panic<F>(operation: F) -> bool
where
    F: FnOnce() + std::panic::UnwindSafe,
{
    let _hook_lock = EXPECTED_PANIC_HOOK_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(operation);
    std::panic::set_hook(previous_hook);
    result.is_err()
}

impl<T> Clone for SharedMutableHandle<T> {
    /// 复制显式共享可变句柄包装器。
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> SharedMutableHandle<T> {
    /// 创建显式共享可变句柄。
    pub fn new(value: T, drops: Arc<AtomicUsize>) -> Self {
        Self {
            inner: Arc::new(MutableState {
                value: Mutex::new(value),
                drops,
            }),
        }
    }

    /// 获取锁；处理中毒锁时保留载荷并继续记录清理顺序。
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.inner
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 非阻塞尝试获取锁，用于取消等待模型。
    pub fn try_lock(&self) -> Result<MutexGuard<'_, T>, TryLockError<MutexGuard<'_, T>>> {
        self.inner.value.try_lock()
    }
}

/// 共享可变句柄等待锁时的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockWaitResult {
    /// 获得锁后立即释放。
    Acquired,
    /// 取消标记先于获得锁到达。
    Cancelled,
    /// 超过观测窗口仍没有结果。
    TimedOut,
}

/// 模拟可取消的锁等待；锁守卫的 RAII 保证 panic/取消路径会释放锁。
pub fn wait_for_lock_or_cancel<T>(
    handle: &SharedMutableHandle<T>,
    cancelled: &AtomicBool,
    timeout: Duration,
) -> LockWaitResult {
    let deadline = Instant::now() + timeout;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return LockWaitResult::Cancelled;
        }
        match handle.try_lock() {
            Ok(guard) => {
                drop(guard);
                return LockWaitResult::Acquired;
            }
            Err(TryLockError::Poisoned(guard)) => {
                drop(guard.into_inner());
                return LockWaitResult::Acquired;
            }
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                thread::yield_now();
            }
            Err(TryLockError::WouldBlock) => return LockWaitResult::TimedOut,
        }
    }
}

/// 共享可变句柄的锁、取消、panic 解锁和析构场景。
pub fn atomic_mutable_demo() -> Result<(), String> {
    /// 并发写入线程数量。
    const WRITERS: usize = 4;
    /// 每个写入线程的增量次数。
    const INCREMENTS: usize = 200;
    let drops = Arc::new(AtomicUsize::new(0));
    let handle = SharedMutableHandle::new(0_usize, Arc::clone(&drops));
    let mut joins = Vec::with_capacity(WRITERS);
    for _ in 0..WRITERS {
        let worker_handle = handle.clone();
        joins.push(thread::spawn(move || {
            for _ in 0..INCREMENTS {
                *worker_handle.lock() += 1;
            }
        }));
    }
    for join in joins {
        join.join()
            .map_err(|_| "共享可变写入线程 panic".to_owned())?;
    }
    if *handle.lock() != WRITERS * INCREMENTS {
        return Err("显式锁保护的写入结果错误".to_owned());
    }

    let guard = handle.lock();
    let cancelled = Arc::new(AtomicBool::new(false));
    let waiter_ready = Arc::new(Barrier::new(2));
    let waiter_handle = handle.clone();
    let waiter_cancelled = Arc::clone(&cancelled);
    let waiter_ready_clone = Arc::clone(&waiter_ready);
    let waiter = thread::spawn(move || {
        waiter_ready_clone.wait();
        wait_for_lock_or_cancel(
            &waiter_handle,
            &waiter_cancelled,
            Duration::from_millis(200),
        )
    });
    waiter_ready.wait();
    cancelled.store(true, Ordering::Release);
    if waiter.join().map_err(|_| "取消等待线程 panic".to_owned())? != LockWaitResult::Cancelled
    {
        return Err("等待锁的取消没有生效".to_owned());
    }
    drop(guard);

    let panic_observed = catches_expected_panic(std::panic::AssertUnwindSafe(|| {
        let _guard = handle.lock();
        panic!("原型故意触发共享可变路径异常");
    }));
    if !panic_observed {
        return Err("共享可变异常路径没有触发".to_owned());
    }
    *handle.lock() += 1;
    let clone = handle.clone();
    drop(handle);
    if drops.load(Ordering::SeqCst) != 0 {
        return Err("仍有共享句柄时提前析构".to_owned());
    }
    drop(clone);
    if drops.load(Ordering::SeqCst) != 1 {
        return Err(format!(
            "共享可变载荷析构次数错误：{}",
            drops.load(Ordering::SeqCst)
        ));
    }
    Ok(())
}

/// 取消清理中的稳定步骤。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupStep {
    /// 标记取消。
    MarkCancelled,
    /// 停止接纳新任务。
    StopAccepting,
    /// 关闭并 drain 边界队列。
    Drain,
    /// 等待安全点。
    SafePoint,
    /// 执行 finally/drop。
    FinallyDrop,
    /// 释放边界值。
    ReleaseBoundary,
    /// 汇总错误。
    AggregateErrors,
}

/// 取消发生的位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelPosition {
    /// 任务尚未开始。
    NotStarted,
    /// 任务正在执行。
    Running,
    /// 任务正在等待锁或消息。
    Waiting,
    /// 任务已完成但仍待回收。
    CompletedAwaitingReclaim,
}

/// 取消/清理的可观察报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationReport {
    /// 取消发生的位置。
    pub position: CancelPosition,
    /// 清理事件顺序。
    pub steps: Vec<CleanupStep>,
    /// 主错误必须保持在首位。
    pub primary_error: String,
    /// 清理错误只能作为 suppressed 保留。
    pub suppressed_errors: Vec<String>,
}

/// 按 07-D §10.4 顺序生成取消报告。
pub fn simulate_cancellation(position: CancelPosition, cleanup_fails: bool) -> CancellationReport {
    let steps = vec![
        CleanupStep::MarkCancelled,
        CleanupStep::StopAccepting,
        CleanupStep::Drain,
        CleanupStep::SafePoint,
        CleanupStep::FinallyDrop,
        CleanupStep::ReleaseBoundary,
        CleanupStep::AggregateErrors,
    ];
    CancellationReport {
        position,
        steps,
        primary_error: "任务取消".to_owned(),
        suppressed_errors: if cleanup_fails {
            vec!["finally/drop 清理失败".to_owned()]
        } else {
            Vec::new()
        },
    }
}

/// 四个取消位置和主/suppressed 错误顺序场景。
pub fn cancellation_demo() -> Result<(), String> {
    let expected = vec![
        CleanupStep::MarkCancelled,
        CleanupStep::StopAccepting,
        CleanupStep::Drain,
        CleanupStep::SafePoint,
        CleanupStep::FinallyDrop,
        CleanupStep::ReleaseBoundary,
        CleanupStep::AggregateErrors,
    ];
    for position in [
        CancelPosition::NotStarted,
        CancelPosition::Running,
        CancelPosition::Waiting,
        CancelPosition::CompletedAwaitingReclaim,
    ] {
        let report = simulate_cancellation(position, true);
        if report.steps != expected
            || report.primary_error != "任务取消"
            || report.suppressed_errors != vec!["finally/drop 清理失败"]
        {
            return Err(format!("取消位置 {position:?} 的清理报告错误：{report:?}"));
        }
    }
    Ok(())
}

/// 可复现成本测量结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measurements {
    /// 目标操作系统。
    pub os: &'static str,
    /// 目标架构。
    pub arch: &'static str,
    /// 每项测量的重复次数。
    pub iterations: usize,
    /// 每次 spawn/join 的纳秒数。
    pub spawn_join_ns: u128,
    /// 每次深拷贝的纳秒数。
    pub deep_copy_ns: u128,
    /// 每次原子增减的纳秒数。
    pub atomic_op_ns: u128,
    /// 每次锁保护增量的纳秒数。
    pub lock_op_ns: u128,
    /// 只读句柄包装器的原型尺寸，不是共享对象头布局。
    pub read_handle_wrapper_bytes: usize,
    /// 共享可变句柄包装器的原型尺寸，不是共享对象头布局。
    pub mutable_handle_wrapper_bytes: usize,
    /// 只读状态的原型尺寸（载荷 + 计数指针），不是原子对象头尺寸。
    pub read_state_layout_bytes: usize,
}

impl fmt::Display for Measurements {
    /// 输出稳定的键值测量记录。
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(output, "environment.os={}", self.os)?;
        writeln!(output, "environment.arch={}", self.arch)?;
        writeln!(output, "iterations={}", self.iterations)?;
        writeln!(output, "spawn_join_ns={}", self.spawn_join_ns)?;
        writeln!(output, "deep_copy_ns={}", self.deep_copy_ns)?;
        writeln!(output, "atomic_op_ns={}", self.atomic_op_ns)?;
        writeln!(output, "lock_op_ns={}", self.lock_op_ns)?;
        writeln!(
            output,
            "read_handle_wrapper_bytes={}",
            self.read_handle_wrapper_bytes
        )?;
        writeln!(
            output,
            "mutable_handle_wrapper_bytes={}",
            self.mutable_handle_wrapper_bytes
        )?;
        write!(
            output,
            "read_state_layout_bytes={}",
            self.read_state_layout_bytes
        )
    }
}

/// 把总耗时换算成每个操作的整数纳秒均值。
fn average_ns(elapsed: Duration, operations: usize) -> u128 {
    elapsed.as_nanos() / operations.max(1) as u128
}

/// 测量线程入口、深拷贝、原子操作、锁和原型对象尺寸。
pub fn measure(iterations: usize) -> Measurements {
    let iterations = iterations.max(1);
    let source = NestedValue::Array(vec![
        NestedValue::Int(1),
        NestedValue::Array(vec![NestedValue::Text("measure".to_owned())]),
    ]);

    let spawn_inputs = (0..iterations)
        .map(|_| source.deep_copy())
        .collect::<Vec<_>>();
    let start = Instant::now();
    for value in spawn_inputs {
        let join = thread::spawn(move || black_box(value));
        black_box(join.join().expect("measurement thread panicked"));
    }
    let spawn_join_ns = average_ns(start.elapsed(), iterations);

    let start = Instant::now();
    for _ in 0..iterations {
        black_box(source.deep_copy());
    }
    let deep_copy_ns = average_ns(start.elapsed(), iterations);

    let atomic_operations = iterations * 100;
    let counter = AtomicUsize::new(0);
    let start = Instant::now();
    for _ in 0..atomic_operations {
        counter.fetch_add(1, Ordering::SeqCst);
    }
    let atomic_op_ns = average_ns(start.elapsed(), atomic_operations);

    let lock = Mutex::new(0_usize);
    let start = Instant::now();
    for _ in 0..atomic_operations {
        *lock.lock().expect("measurement lock poisoned") += 1;
    }
    let lock_op_ns = average_ns(start.elapsed(), atomic_operations);

    Measurements {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        iterations,
        spawn_join_ns,
        deep_copy_ns,
        atomic_op_ns,
        lock_op_ns,
        read_handle_wrapper_bytes: size_of::<ReadHandle<u64>>(),
        mutable_handle_wrapper_bytes: size_of::<SharedMutableHandle<u64>>(),
        read_state_layout_bytes: size_of::<ReadState<u64>>(),
    }
}

/// 运行全部 8 类场景；命令行程序和文档测试共用这一入口。
pub fn run_all_scenarios() -> Result<(), String> {
    thread_entry_and_deep_copy_demo()?;
    deep_copy_failure_demo()?;
    boundary_and_candidate_demo()?;
    atomic_read_demo()?;
    atomic_mutable_demo()?;
    cancellation_demo()?;
    atomic_abi::run_demo()?;
    Ok(())
}

/// 返回自定义原子对象头/句柄原型的布局记录。
pub fn custom_abi_layout_report() -> String {
    atomic_abi::layout_report()
}

/// 返回自定义句柄操作成本的原型测量记录。
pub fn custom_abi_cost_report(iterations: usize) -> String {
    format!("{:?}", atomic_abi::measure_costs(iterations))
}

/// 原型场景的 Rust 单元测试集合。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证线程副本不会修改发送方。
    fn thread_entry_isolated_by_deep_copy() {
        thread_entry_and_deep_copy_demo().unwrap();
    }

    #[test]
    /// 验证复制失败的逆序清理和主错误保留。
    fn failed_copy_cleans_prefix_in_reverse_and_preserves_primary_error() {
        deep_copy_failure_demo().unwrap();
    }

    #[test]
    /// 验证边界事实和候选 A 表。
    fn boundary_facts_and_candidate_a_table_are_observable() {
        boundary_and_candidate_demo().unwrap();
    }

    #[test]
    /// 验证只读句柄的升级竞态和一次析构。
    fn atomic_read_handle_has_one_drop_and_safe_upgrade_race() {
        atomic_read_demo().unwrap();
    }

    #[test]
    /// 验证显式锁、取消等待和共享可变异常路径。
    fn atomic_mutable_handle_uses_explicit_lock_and_cancellation() {
        atomic_mutable_demo().unwrap();
    }

    #[test]
    /// 验证四个取消位置共享同一清理顺序。
    fn cancellation_order_and_suppressed_error_are_stable() {
        cancellation_demo().unwrap();
    }

    #[test]
    /// 验证测量结果包含有效重复次数和状态尺寸。
    fn measurements_have_positive_repeat_count_and_sizes() {
        let result = measure(2);
        assert_eq!(result.iterations, 2);
        assert!(result.read_handle_wrapper_bytes > 0);
        assert!(result.mutable_handle_wrapper_bytes > 0);
        assert!(result.read_state_layout_bytes > 0);
    }

    #[test]
    /// 验证一次运行覆盖全部八类原型场景。
    fn all_eight_scenarios_pass_as_one_reproducible_run() {
        run_all_scenarios().unwrap();
    }
}
