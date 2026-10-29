//! 22C 最小 Runtime/线程/任务池执行骨架。
//!
//! 该模块提供 Runtime 层的生命周期、调度和取消边界；VM 通过显式并发绑定消费，
//! LLVM 则经旁置 `abi_concurrency` trampoline 适配。Xiao 标准库函数体仍属于 22E。
//! 并发句柄不复用普通单线程 `StrongHandle`。

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(crate) mod runtime_helpers;
/// 并发 Runtime 的内部辅助入口，供 VM/原生 ABI 复用。
pub use runtime_helpers::{
    cancellation_safe_point, current_cancellation_token, detached_reaper_peak_depth,
    detached_reaper_queue_depth, record_detached_error, take_detached_errors,
};
use runtime_helpers::{current_runtime_handle, register_runtime};

/// Runtime、线程和任务池共用的取消令牌。
pub use crate::concurrency::LockCancellation as RuntimeCancellation;
use crate::concurrency::{CancelHookRegistration, global_runtime_registry};
use crate::{
    CONCURRENCY_CANCELLED_CODE, CONCURRENCY_POOL_ARGUMENT_CODE, CONCURRENCY_POOL_CLOSED_CODE,
    CONCURRENCY_RUNTIME_CLOSED_CODE,
};

thread_local! {
    static CURRENT_CANCELLATION: RefCell<Option<RuntimeCancellation>> = const { RefCell::new(None) };
    static CURRENT_RUNTIME_ID: RefCell<Option<u64>> = const { RefCell::new(None) };
}

/// 离开执行单元时恢复线程之前的取消上下文。
struct CancellationContextGuard {
    previous: Option<RuntimeCancellation>,
}

impl Drop for CancellationContextGuard {
    /// 恢复上一个线程局部取消令牌。
    fn drop(&mut self) {
        CURRENT_CANCELLATION.with(|current| {
            *current.borrow_mut() = self.previous.take();
        });
    }
}

/// 安装当前执行单元的取消令牌并返回恢复守卫。
fn enter_cancellation_context(token: RuntimeCancellation) -> CancellationContextGuard {
    let previous = CURRENT_CANCELLATION.with(|current| current.borrow_mut().replace(token));
    CancellationContextGuard { previous }
}

/// 克隆当前线程继承的取消令牌。
fn current_cancellation_context() -> Option<RuntimeCancellation> {
    CURRENT_CANCELLATION.with(|current| current.borrow().clone())
}

/// 离开执行单元时恢复线程之前的 Runtime 身份。
struct RuntimeContextGuard {
    previous: Option<u64>,
}

impl Drop for RuntimeContextGuard {
    /// 恢复上一个线程局部 Runtime ID。
    fn drop(&mut self) {
        CURRENT_RUNTIME_ID.with(|current| {
            *current.borrow_mut() = self.previous.take();
        });
    }
}

/// 安装当前执行单元的 Runtime ID 并返回恢复守卫。
fn enter_runtime_context(runtime_id: u64) -> RuntimeContextGuard {
    let previous = CURRENT_RUNTIME_ID.with(|current| current.borrow_mut().replace(runtime_id));
    RuntimeContextGuard { previous }
}

/// 读取当前线程绑定的 Runtime ID。
fn current_runtime_context() -> Option<u64> {
    CURRENT_RUNTIME_ID.with(|current| *current.borrow())
}

/// 可复制的 Runtime 句柄。
#[derive(Clone, Debug)]
pub struct RuntimeHandle {
    id: u64,
    inner: Arc<RuntimeInner>,
    owner_runtime_id: Option<u64>,
}

#[derive(Debug)]
/// Runtime 的关闭、活动计数、池登记和 worker 状态。
struct RuntimeInner {
    owner_runtime_id: Option<u64>,
    closed: AtomicBool,
    active: Mutex<usize>,
    drained: Condvar,
    close_gate: Mutex<RuntimeCloseState>,
    pools: Mutex<Vec<Weak<PoolInner>>>,
    active_threads: Mutex<HashMap<thread::ThreadId, usize>>,
    closing_threads: Mutex<HashMap<thread::ThreadId, usize>>,
}

#[derive(Debug, Default)]
/// 串行协调一个 Runtime 的唯一关闭 owner 和后续观察者。
struct RuntimeCloseState {
    draining: bool,
    finished: bool,
}

/// 当前调用在线程关闭协议中的职责。
enum CloseRole {
    Owner(Vec<Arc<PoolInner>>),
    ActiveFollower,
    PassiveFollower,
    Finished,
}

/// 跟踪从提交到完成的一项 Runtime 活动。
struct ActivityLease {
    runtime: Arc<RuntimeInner>,
    running_thread: Option<thread::ThreadId>,
}

/// 标记正在执行 close 的活动，防止 closer 互相等待。
struct ClosingActivityGuard {
    runtime: Arc<RuntimeInner>,
    thread_id: thread::ThreadId,
    count: usize,
}

impl Drop for ClosingActivityGuard {
    /// 关闭调用退出时移除线程的 closing 计数并唤醒 drain owner。
    fn drop(&mut self) {
        if let Ok(mut closing) = self.runtime.closing_threads.lock()
            && let Some(count) = closing.get_mut(&self.thread_id)
        {
            *count = count.saturating_sub(self.count);
            if *count == 0 {
                closing.remove(&self.thread_id);
            }
            self.runtime.drained.notify_all();
        }
    }
}

impl ActivityLease {
    /// 将已提交活动登记到当前运行线程。
    fn mark_running(&mut self) {
        let thread_id = thread::current().id();
        if let Ok(mut threads) = self.runtime.active_threads.lock() {
            *threads.entry(thread_id).or_insert(0) += 1;
            self.running_thread = Some(thread_id);
            self.runtime.drained.notify_all();
        }
    }
}

impl Drop for ActivityLease {
    /// 活动结束时同步减少线程计数和 Runtime 总活动数。
    fn drop(&mut self) {
        if let Some(thread_id) = self.running_thread.take()
            && let Ok(mut threads) = self.runtime.active_threads.lock()
        {
            if let Some(count) = threads.get_mut(&thread_id) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    threads.remove(&thread_id);
                }
            }
        }
        if let Ok(mut active) = self.runtime.active.lock() {
            *active = active.saturating_sub(1);
            self.runtime.drained.notify_all();
        }
    }
}

impl RuntimeHandle {
    /// 创建一个新的进程内 Runtime。
    #[must_use]
    pub fn create() -> Self {
        /// 进程内静态 Runtime 标签序列。
        static LABEL: AtomicU64 = AtomicU64::new(1);
        let label = LABEL
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |label| {
                label.checked_add(1)
            })
            .expect("Runtime 静态标签耗尽");
        let id = global_runtime_registry()
            .register(label)
            .expect("Runtime 静态标签应有效");
        let owner_runtime_id = current_runtime_context();
        let inner = Arc::new(RuntimeInner {
            owner_runtime_id,
            closed: AtomicBool::new(false),
            active: Mutex::new(0),
            drained: Condvar::new(),
            close_gate: Mutex::new(RuntimeCloseState::default()),
            pools: Mutex::new(Vec::new()),
            active_threads: Mutex::new(HashMap::new()),
            closing_threads: Mutex::new(HashMap::new()),
        });
        register_runtime(id, &inner);
        Self {
            id,
            inner,
            owner_runtime_id,
        }
    }

    /// 返回当前线程上下文 Runtime；没有显式上下文时 lazily 创建根 Runtime。
    #[must_use]
    pub fn current() -> Self {
        current_runtime_handle()
    }

    /// 返回实际 Runtime ID。
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// 判断 Runtime 是否已关闭。
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    /// 在受检的 Runtime 上下文内执行宿主代码；离开时恢复原线程上下文。
    pub fn in_context<T>(&self, body: impl FnOnce() -> T) -> Result<T, RuntimeExecError> {
        if !self.context_allows_current() {
            return Err(RuntimeExecError::RuntimeClosed);
        }
        let _context = enter_runtime_context(self.id);
        Ok(body())
    }

    /// 幂等关闭 Runtime；旧别名可观察相同关闭状态。
    ///
    /// 活动 Runtime 上下文只能关闭自身 Runtime，或关闭由该上下文创建的
    /// 子 Runtime；跨 Runtime 调用被拒绝且保持原有无返回值兼容。
    /// 这个兼容入口在拒绝时是 no-op；需要观察 `X06-RUNTIME-031` 的 Rust/VM
    /// 调用方必须使用 [`Self::try_close`]。
    pub fn close(&self) {
        if self.context_allows_current() {
            self.close_inner();
        }
    }

    /// 关闭 Runtime 并显式返回跨 Runtime 上下文拒绝。
    pub fn try_close(&self) -> Result<(), RuntimeExecError> {
        if !self.context_allows_current() {
            return Err(RuntimeExecError::RuntimeClosed);
        }
        self.close_inner();
        Ok(())
    }

    /// 判断当前 Runtime 上下文是否是自身或直接 owner。
    fn context_allows_current(&self) -> bool {
        current_runtime_context().is_none_or(|current_runtime_id| {
            current_runtime_id == self.id || self.owner_runtime_id == Some(current_runtime_id)
        })
    }

    /// 选择唯一 drain owner，并等待尚未进入关闭的 Runtime 活动。
    fn close_inner(&self) {
        let closing_guard = self.enter_current_thread_closing();
        let current_active = closing_guard.as_ref().map_or(0, |guard| guard.count);
        let role = {
            let Ok(mut close_state) = self.inner.close_gate.lock() else {
                return;
            };
            if !self.inner.closed.swap(true, Ordering::AcqRel) {
                close_state.draining = true;
                close_state.finished = false;
                global_runtime_registry().close(self.id);
                let pools = self
                    .inner
                    .pools
                    .lock()
                    .map(|mut pools| {
                        let live = pools.iter().filter_map(Weak::upgrade).collect::<Vec<_>>();
                        pools.clear();
                        live
                    })
                    .unwrap_or_default();
                CloseRole::Owner(pools)
            } else if close_state.draining {
                // 活动执行单元不能等待另一个 close；它先返回并让自己的
                // closing guard 释放后续 activity 等待。
                if current_active != 0 {
                    CloseRole::ActiveFollower
                } else {
                    CloseRole::PassiveFollower
                }
            } else {
                CloseRole::Finished
            }
        };

        match role {
            CloseRole::Owner(pools) => {
                for pool in pools {
                    TaskPool::shutdown_inner(&pool);
                }
                self.wait_for_non_closing_active();
                if let Ok(mut close_state) = self.inner.close_gate.lock() {
                    close_state.draining = false;
                    close_state.finished = true;
                    self.inner.drained.notify_all();
                }
            }
            CloseRole::ActiveFollower | CloseRole::Finished => {}
            CloseRole::PassiveFollower => {
                let Ok(mut close_state) = self.inner.close_gate.lock() else {
                    return;
                };
                while close_state.draining && !close_state.finished {
                    let Ok(remaining) = self
                        .inner
                        .drained
                        .wait_timeout(close_state, Duration::from_millis(10))
                    else {
                        return;
                    };
                    close_state = remaining.0;
                }
            }
        }
    }

    /// 将当前线程在此 Runtime 的全部活动标记为正在关闭。
    fn enter_current_thread_closing(&self) -> Option<ClosingActivityGuard> {
        let thread_id = thread::current().id();
        let Ok(threads) = self.inner.active_threads.lock() else {
            return None;
        };
        let current_active = threads.get(&thread_id).copied().unwrap_or(0);
        if current_active == 0 {
            return None;
        }
        let Ok(mut closing) = self.inner.closing_threads.lock() else {
            return None;
        };
        closing
            .entry(thread_id)
            .and_modify(|count| *count = count.saturating_add(current_active))
            .or_insert(current_active);
        Some(ClosingActivityGuard {
            runtime: Arc::clone(&self.inner),
            thread_id,
            count: current_active,
        })
    }

    /// 汇总当前已声明关闭的活动数。
    fn closing_active_count(&self) -> usize {
        self.inner
            .closing_threads
            .lock()
            .map(|threads| threads.values().copied().sum())
            .unwrap_or(0)
    }

    /// 等待 Runtime 活动全部退出或进入可由 owner 统一收束的关闭路径。
    fn wait_for_non_closing_active(&self) {
        let Ok(mut active) = self.inner.active.lock() else {
            return;
        };
        loop {
            if *active <= self.closing_active_count() {
                return;
            }
            let Ok(remaining) = self
                .inner
                .drained
                .wait_timeout(active, Duration::from_millis(10))
            else {
                return;
            };
            active = remaining.0;
        }
    }

    /// 检查 Runtime 是否仍接收新执行单元。
    fn begin_activity(&self) -> Result<ActivityLease, RuntimeExecError> {
        let mut active = self
            .inner
            .active
            .lock()
            .map_err(|_| RuntimeExecError::JoinFailed)?;
        if self.inner.closed.load(Ordering::Acquire) {
            return Err(RuntimeExecError::RuntimeClosed);
        }
        *active = active.checked_add(1).ok_or(RuntimeExecError::JoinFailed)?;
        Ok(ActivityLease {
            runtime: Arc::clone(&self.inner),
            running_thread: None,
        })
    }

    /// 登记池的弱引用，供 Runtime 关闭时收束仍存活的池。
    fn register_pool(&self, pool: &Arc<PoolInner>) -> Result<(), RuntimeExecError> {
        let mut pools = self
            .inner
            .pools
            .lock()
            .map_err(|_| RuntimeExecError::JoinFailed)?;
        if self.inner.closed.load(Ordering::Acquire) {
            return Err(RuntimeExecError::RuntimeClosed);
        }
        pools.push(Arc::downgrade(pool));
        Ok(())
    }
}

/// 22C Runtime 执行错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeExecError {
    /// Runtime 已关闭。
    RuntimeClosed,
    /// 任务池自身已关闭。
    PoolClosed,
    /// 任务池容量或参数错误。
    PoolArgument,
    /// 线程/任务被取消。
    Cancelled,
    /// 执行单元 panic 或失去结果。
    JoinFailed,
}

impl RuntimeExecError {
    /// 返回冻结错误码。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RuntimeClosed => CONCURRENCY_RUNTIME_CLOSED_CODE,
            Self::PoolClosed => CONCURRENCY_POOL_CLOSED_CODE,
            Self::PoolArgument => CONCURRENCY_POOL_ARGUMENT_CODE,
            Self::Cancelled => CONCURRENCY_CANCELLED_CODE,
            Self::JoinFailed => "X06-RUNTIME-028",
        }
    }
}

impl fmt::Display for RuntimeExecError {
    /// 输出稳定错误码。
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for RuntimeExecError {}

/// 一个可取消、可 join 的线程句柄。
pub struct ThreadHandle<T> {
    join: Option<JoinHandle<Result<T, RuntimeExecError>>>,
    cancellation: RuntimeCancellation,
}

impl<T: 'static> ThreadHandle<T> {
    /// 返回共享取消令牌，等待消耗句柄后仍可取消执行单元。
    #[must_use]
    pub fn cancellation(&self) -> RuntimeCancellation {
        self.cancellation.clone()
    }

    /// 等待期间轮询调用者中断；中断取消子线程并将回收交给 detached 通道。
    pub fn join_interruptible(
        mut self,
        interrupted: impl Fn() -> bool,
    ) -> Result<T, RuntimeExecError> {
        while !self.join.as_ref().is_none_or(JoinHandle::is_finished) {
            if interrupted() {
                self.cancel();
                self.detach();
                return Err(RuntimeExecError::Cancelled);
            }
            thread::park_timeout(Duration::from_millis(2));
        }
        if interrupted() {
            self.cancel();
            self.detach();
            return Err(RuntimeExecError::Cancelled);
        }
        self.join
            .take()
            .ok_or(RuntimeExecError::JoinFailed)?
            .join()
            .map_err(|_| RuntimeExecError::JoinFailed)?
    }

    /// 消费线程，并由调用方将结构化执行结果汇入自己的 detached 通道。
    pub fn detach_with(
        mut self,
        report: impl FnOnce(Result<T, RuntimeExecError>) + Send + 'static,
    ) {
        if let Some(join) = self.join.take() {
            enqueue_detached_reap(Box::new(move || {
                report(join.join().unwrap_or(Err(RuntimeExecError::JoinFailed)));
            }))
            .expect("detached 错误收集线程不可用");
        }
    }
    /// 借用取消句柄；取消不消费线程句柄。
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    /// 查询取消状态。
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    /// 消费线程句柄并等待结果。
    pub fn join(mut self) -> Result<T, RuntimeExecError> {
        self.join
            .take()
            .ok_or(RuntimeExecError::JoinFailed)?
            .join()
            .map_err(|_| RuntimeExecError::JoinFailed)?
    }

    /// 消费线程句柄并将失败送入进程级 detached 错误通道。
    ///
    /// 回收由固定 worker 池消费无界队列，发送不会因回收 worker 暂时繁忙而阻塞；
    /// 未开始执行的回收闭包数量可通过 [`detached_reaper_queue_depth`] 观测。
    pub fn detach(mut self) {
        if let Some(join) = self.join.take() {
            enqueue_detached_reap(Box::new(move || {
                let error = match join.join() {
                    Ok(Ok(_)) => None,
                    Ok(Err(error)) => Some(error),
                    Err(_) => Some(RuntimeExecError::JoinFailed),
                };
                if let Some(error) = error {
                    detached_errors()
                        .lock()
                        .expect("detached 错误通道锁未中毒")
                        .push(error);
                }
            }))
            .expect("detached 错误收集线程不可用");
        }
    }
}

fn detached_errors() -> &'static Mutex<Vec<RuntimeExecError>> {
    /// Rust Runtime API detached 执行失败记录。
    static ERRORS: OnceLock<Mutex<Vec<RuntimeExecError>>> = OnceLock::new();
    ERRORS.get_or_init(|| Mutex::new(Vec::new()))
}

/// 返回未开始回收的 detached 闭包计数器。
fn detached_queue_depth_counter() -> &'static AtomicUsize {
    /// 当前 detached 回收队列深度。
    static DEPTH: AtomicUsize = AtomicUsize::new(0);
    &DEPTH
}

/// detached 队列积压的观测阈值；22E 前必须据此选择资源限制策略。
pub const DETACHED_REAPER_WARNING_DEPTH: usize = 256;
/// 进程启动以来的 detached 回收队列峰值。
static DETACHED_REAPER_PEAK: AtomicUsize = AtomicUsize::new(0);

/// 非阻塞地把一次 join 回收工作放入固定 worker 队列。
fn enqueue_detached_reap(
    reap: Box<dyn FnOnce() + Send>,
) -> Result<(), mpsc::SendError<Box<dyn FnOnce() + Send>>> {
    let queued = detached_queue_depth_counter().fetch_add(1, Ordering::AcqRel) + 1;
    DETACHED_REAPER_PEAK.fetch_max(queued, Ordering::AcqRel);
    let depth = detached_queue_depth_counter();
    match detached_reaper().send(Box::new(move || {
        depth.fetch_sub(1, Ordering::AcqRel);
        reap();
    })) {
        Ok(()) => Ok(()),
        Err(error) => {
            detached_queue_depth_counter().fetch_sub(1, Ordering::AcqRel);
            Err(error)
        }
    }
}

/// 初始化并返回共享的 detached 回收 worker 池。
fn detached_reaper() -> &'static mpsc::Sender<Box<dyn FnOnce() + Send>> {
    /// 固定的 OS join worker 数量。
    const DETACHED_JOIN_WORKERS: usize = 4;
    /// 向 detached 回收池发送一次性 join 闭包。
    static REAPER: OnceLock<mpsc::Sender<Box<dyn FnOnce() + Send>>> = OnceLock::new();
    REAPER.get_or_init(|| {
        // 固定 worker 消费无界消息队列，detach 本身不因回收队列暂时繁忙而阻塞。
        // 每个 detached 任务仍只占一个排队闭包，不再为每个任务创建 OS 线程。
        let (sender, receiver) = mpsc::channel::<Box<dyn FnOnce() + Send>>();
        let receiver = Arc::new(Mutex::new(receiver));
        for index in 0..DETACHED_JOIN_WORKERS {
            let receiver = Arc::clone(&receiver);
            thread::Builder::new()
                .name(format!("xiao-detached-join-{index}"))
                .spawn(move || {
                    loop {
                        let reap = receiver
                            .lock()
                            .ok()
                            .and_then(|receiver| receiver.recv().ok());
                        let Some(reap) = reap else { break };
                        reap();
                    }
                })
                .expect("启动 detached join worker 失败");
        }
        sender
    })
}

/// 在 Runtime 中启动可取消线程；闭包显式接收取消令牌。
pub fn spawn<T, F>(
    runtime: &RuntimeHandle,
    function: F,
) -> Result<ThreadHandle<T>, RuntimeExecError>
where
    T: Send + 'static,
    F: FnOnce(RuntimeCancellation) -> T + Send + 'static,
{
    spawn_policy(runtime, function, true)
}

/// 启动由解释器处理取消/清理结果的执行单元，保留其结构化错误。
pub fn spawn_cooperative<T, F>(
    runtime: &RuntimeHandle,
    function: F,
) -> Result<ThreadHandle<T>, RuntimeExecError>
where
    T: Send + 'static,
    F: FnOnce(RuntimeCancellation) -> T + Send + 'static,
{
    spawn_policy(runtime, function, false)
}

/// 在普通 Runtime API 与 VM 协作式取消间共用线程启动逻辑。
fn spawn_policy<T, F>(
    runtime: &RuntimeHandle,
    function: F,
    check_result: bool,
) -> Result<ThreadHandle<T>, RuntimeExecError>
where
    T: Send + 'static,
    F: FnOnce(RuntimeCancellation) -> T + Send + 'static,
{
    if !runtime.context_allows_current() {
        return Err(RuntimeExecError::RuntimeClosed);
    }
    let activity = runtime.begin_activity()?;
    let cancellation = RuntimeCancellation::default();
    let child_token = cancellation.clone();
    let runtime_id = runtime.id;
    let join = thread::Builder::new()
        .name(format!("xiao-runtime-{}", runtime.id))
        .spawn(move || {
            let mut activity = activity;
            activity.mark_running();
            let _activity = activity;
            let _runtime_context = enter_runtime_context(runtime_id);
            let _cancellation_context = enter_cancellation_context(child_token.clone());
            if child_token.is_cancelled() {
                return Err(RuntimeExecError::Cancelled);
            }
            let result = function(child_token.clone());
            if check_result && child_token.is_cancelled() {
                Err(RuntimeExecError::Cancelled)
            } else {
                Ok(result)
            }
        })
        .map_err(|_| RuntimeExecError::JoinFailed)?;
    Ok(ThreadHandle {
        join: Some(join),
        cancellation,
    })
}

/// 一个尚未开始的任务池消息。
struct PoolMessage {
    id: u64,
    cancel: Box<dyn FnOnce() + Send + 'static>,
    run: Box<dyn FnOnce() + Send + 'static>,
}

/// 任务池有界等待队列；取消可在 worker 领取前真实移除任务。
struct PoolQueue {
    state: Mutex<PoolQueueState>,
    changed: Condvar,
    capacity: usize,
}

struct PoolQueueState {
    messages: VecDeque<PoolMessage>,
    closed: bool,
}

impl PoolQueue {
    fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(PoolQueueState {
                messages: VecDeque::new(),
                closed: false,
            }),
            changed: Condvar::new(),
            capacity,
        }
    }

    /// 领取队首任务；关闭后仍先排空已经接纳的消息。
    fn take(&self, wait: bool) -> Option<PoolMessage> {
        let mut state = self.state.lock().ok()?;
        loop {
            if let Some(message) = state.messages.pop_front() {
                self.changed.notify_all();
                return Some(message);
            }
            if state.closed || !wait {
                return None;
            }
            state = self.changed.wait(state).ok()?;
        }
    }

    /// 取消尚未开始的任务并释放其队列容量。
    fn remove(&self, id: u64) {
        let removed = self.state.lock().ok().and_then(|mut state| {
            let index = state.messages.iter().position(|message| message.id == id)?;
            let message = state.messages.remove(index);
            self.changed.notify_all();
            message
        });
        if let Some(message) = removed {
            let PoolMessage { cancel, run, .. } = message;
            // 丢弃任务闭包可能运行用户捕获值的析构逻辑，因此放在队列锁外。
            drop(run);
            // 任务副本、activity lease 和边界清理事件必须先完成，等待者再收到 027。
            cancel();
        }
    }

    /// 停止接纳新消息并唤醒所有等待者。
    fn close(&self, closed: &AtomicBool) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if closed.swap(true, Ordering::AcqRel) {
            return false;
        }
        state.closed = true;
        self.changed.notify_all();
        true
    }
}

/// 任务池共享内部状态。
struct PoolInner {
    runtime: Weak<RuntimeInner>,
    queue: Arc<PoolQueue>,
    workers: Mutex<Vec<JoinHandle<()>>>,
    worker_ids: Mutex<HashSet<thread::ThreadId>>,
    closed: AtomicBool,
    shutdown_gate: Mutex<()>,
    pending_submissions: Mutex<usize>,
    submissions_drained: Condvar,
    drain_started: AtomicBool,
    drain_worker_count: usize,
    next_message_id: AtomicU64,
}

/// 在提交等待期间计入池关闭需要排空的提交者。
struct SubmissionLease {
    inner: Arc<PoolInner>,
}

impl Drop for SubmissionLease {
    /// 提交成功或失败后减少挂起提交计数。
    fn drop(&mut self) {
        if let Ok(mut pending) = self.inner.pending_submissions.lock() {
            *pending = pending.saturating_sub(1);
            self.inner.submissions_drained.notify_all();
        }
    }
}

impl PoolInner {
    /// 登记一次提交尝试并在返回时自动释放登记。
    fn begin_submission(inner: &Arc<Self>) -> SubmissionLease {
        if let Ok(mut pending) = inner.pending_submissions.lock() {
            *pending = pending.saturating_add(1);
        }
        SubmissionLease {
            inner: Arc::clone(inner),
        }
    }
}

/// 固定容量 Runtime 任务池。
#[derive(Clone)]
pub struct TaskPool {
    runtime: RuntimeHandle,
    inner: Arc<PoolInner>,
}

/// 一个可等待的任务结果句柄。
pub struct TaskHandle<T> {
    receiver: mpsc::Receiver<Result<T, RuntimeExecError>>,
    cancellation: RuntimeCancellation,
    cancel_hook: Option<Arc<dyn Fn() + Send + Sync>>,
    cancel_registration: Option<CancelHookRegistration>,
}

impl<T> TaskHandle<T> {
    /// 读取任务取消令牌。
    #[must_use]
    pub fn cancellation(&self) -> RuntimeCancellation {
        self.cancellation.clone()
    }

    /// 可中断等待；中断或任务令牌在返回前已取消时返回 027。
    ///
    /// 无论取消来源如何，等待都先等任务闭包完成，以保留运行中 VM 的 finally/drop 结果。
    pub fn wait_interruptible(self, interrupted: impl Fn() -> bool) -> Result<T, RuntimeExecError> {
        self.wait_interruptible_inner(interrupted, true)
    }

    /// VM 内部等待完整任务结果，以便在并发取消时保留 finally/drop 的 suppressed 错误。
    #[doc(hidden)]
    pub fn wait_interruptible_preserving_result(
        self,
        interrupted: impl Fn() -> bool,
    ) -> Result<T, RuntimeExecError> {
        self.wait_interruptible_inner(interrupted, false)
    }

    fn wait_interruptible_inner(
        mut self,
        interrupted: impl Fn() -> bool,
        cancellation_wins: bool,
    ) -> Result<T, RuntimeExecError> {
        loop {
            if interrupted() && !self.cancellation.is_cancelled() {
                self.cancel();
            }
            match self.receiver.recv_timeout(Duration::from_millis(2)) {
                Ok(value) => {
                    let externally_interrupted = interrupted();
                    if externally_interrupted && !self.cancellation.is_cancelled() {
                        self.cancel();
                    }
                    let cancelled = self.cancellation.is_cancelled();
                    self.clear_cancel_hook();
                    return if cancellation_wins && (cancelled || externally_interrupted) {
                        Err(RuntimeExecError::Cancelled)
                    } else {
                        value
                    };
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let externally_interrupted = interrupted();
                    if externally_interrupted && !self.cancellation.is_cancelled() {
                        self.cancel();
                    }
                    let cancelled = self.cancellation.is_cancelled();
                    self.clear_cancel_hook();
                    return Err(
                        if cancellation_wins && (cancelled || externally_interrupted) {
                            RuntimeExecError::Cancelled
                        } else {
                            RuntimeExecError::JoinFailed
                        },
                    );
                }
            }
        }
    }

    fn clear_cancel_hook(&mut self) {
        self.cancel_registration.take();
        self.cancel_hook.take();
    }

    /// 将未等待的任务结果交给 detached 回收通道，不阻塞当前线程。
    pub(crate) fn detach_with(
        self,
        report: impl FnOnce(Result<T, RuntimeExecError>) + Send + 'static,
    ) where
        T: Send + 'static,
    {
        let TaskHandle {
            receiver,
            cancel_hook,
            cancel_registration,
            ..
        } = self;
        enqueue_detached_reap(Box::new(move || {
            let _cancel_hook = cancel_hook;
            let _cancel_registration = cancel_registration;
            report(receiver.recv().unwrap_or(Err(RuntimeExecError::JoinFailed)));
        }))
        .expect("detached 错误收集线程不可用");
    }

    /// 借用取消任务；取消不消费任务句柄。
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    /// 消费任务句柄并等待结果；任务取消先于结果消费时返回 027，即使任务体已完成。
    pub fn wait(mut self) -> Result<T, RuntimeExecError> {
        loop {
            match self.receiver.recv_timeout(Duration::from_millis(2)) {
                Ok(value) => {
                    let cancelled = self.cancellation.is_cancelled();
                    self.clear_cancel_hook();
                    return if cancelled {
                        Err(RuntimeExecError::Cancelled)
                    } else {
                        value
                    };
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let cancelled = self.cancellation.is_cancelled();
                    self.clear_cancel_hook();
                    return Err(if cancelled {
                        RuntimeExecError::Cancelled
                    } else {
                        RuntimeExecError::JoinFailed
                    });
                }
            }
        }
    }
}

impl TaskPool {
    /// 返回池绑定的 Runtime 别名，供隔离 VM 建立正确的当前上下文。
    #[must_use]
    pub fn runtime(&self) -> RuntimeHandle {
        self.runtime.clone()
    }
    /// 判断两个池句柄是否引用同一底层池。
    #[must_use]
    pub(crate) fn same_identity(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// 创建固定容量任务池；容量范围为 1–256。
    pub fn create(runtime: RuntimeHandle, capacity: usize) -> Result<Self, RuntimeExecError> {
        if !(1..=256).contains(&capacity) {
            return Err(RuntimeExecError::PoolArgument);
        }
        if !runtime.context_allows_current() {
            return Err(RuntimeExecError::RuntimeClosed);
        }
        let queue = Arc::new(PoolQueue::new(capacity));
        let mut workers: Vec<JoinHandle<()>> = Vec::with_capacity(capacity.min(4));
        let mut worker_ids = HashSet::with_capacity(capacity.min(4));
        let startup_closed = AtomicBool::new(false);
        for index in 0..capacity.min(4) {
            let worker_queue = Arc::clone(&queue);
            let worker = thread::Builder::new()
                .name(format!("xiao-pool-{}-{}", runtime.id, index))
                .spawn(move || {
                    while let Some(message) = worker_queue.take(true) {
                        (message.run)();
                    }
                });
            let worker = match worker {
                Ok(worker) => worker,
                Err(_) => {
                    queue.close(&startup_closed);
                    for worker in workers {
                        let _ = worker.join();
                    }
                    return Err(RuntimeExecError::JoinFailed);
                }
            };
            worker_ids.insert(worker.thread().id());
            workers.push(worker);
        }
        let inner = Arc::new(PoolInner {
            runtime: Arc::downgrade(&runtime.inner),
            queue,
            workers: Mutex::new(workers),
            worker_ids: Mutex::new(worker_ids),
            closed: AtomicBool::new(false),
            shutdown_gate: Mutex::new(()),
            pending_submissions: Mutex::new(0),
            submissions_drained: Condvar::new(),
            drain_started: AtomicBool::new(false),
            drain_worker_count: capacity.min(4).saturating_add(1),
            next_message_id: AtomicU64::new(1),
        });
        if let Err(error) = runtime.register_pool(&inner) {
            Self::shutdown_inner(&inner);
            return Err(error);
        }
        Ok(Self { runtime, inner })
    }

    /// 提交一个显式接收取消令牌的任务。
    ///
    /// 当前 Runtime worker 会自动继承其执行上下文的取消令牌。入口线程或
    /// 其他没有上下文的调用方应使用 [`Self::submit_with_cancel`] 取得提交前
    /// 的取消入口；普通 `submit` 在这种上下文下仍会在队列满时背压，但
    /// Runtime/pool 关闭会让等待停止并返回对应关闭错误。
    pub fn submit<T, F>(&self, function: F) -> Result<TaskHandle<T>, RuntimeExecError>
    where
        T: Send + 'static,
        F: FnOnce(RuntimeCancellation) -> T + Send + 'static,
    {
        self.submit_with_cancel(current_cancellation_context().unwrap_or_default(), function)
    }

    /// 提交一个带提交期取消令牌的任务。
    ///
    /// 队列已满时，提交方以可取消背压等待；取消令牌在任务入队前触发
    /// 会返回 027，任务不会进入池中。
    pub fn submit_with_cancel<T, F>(
        &self,
        cancellation: RuntimeCancellation,
        function: F,
    ) -> Result<TaskHandle<T>, RuntimeExecError>
    where
        T: Send + 'static,
        F: FnOnce(RuntimeCancellation) -> T + Send + 'static,
    {
        self.submit_interruptible(cancellation, || false, function)
    }

    /// 在提交期同时检查宿主中断来源；已接纳任务仍用自身取消令牌。
    pub fn submit_interruptible<T, F>(
        &self,
        cancellation: RuntimeCancellation,
        interrupted: impl Fn() -> bool,
        function: F,
    ) -> Result<TaskHandle<T>, RuntimeExecError>
    where
        T: Send + 'static,
        F: FnOnce(RuntimeCancellation) -> T + Send + 'static,
    {
        self.submit_policy(cancellation, interrupted, function, true)
    }

    /// 解释器提交入口，保留取消清理产生的主错误及 suppressed。
    pub fn submit_cooperative<T, F>(
        &self,
        cancellation: RuntimeCancellation,
        interrupted: impl Fn() -> bool,
        function: F,
    ) -> Result<TaskHandle<T>, RuntimeExecError>
    where
        T: Send + 'static,
        F: FnOnce(RuntimeCancellation) -> T + Send + 'static,
    {
        self.submit_policy(cancellation, interrupted, function, false)
    }

    /// 共享实现可取消背压、普通执行和 VM 保错执行的提交路径。
    fn submit_policy<T, F>(
        &self,
        cancellation: RuntimeCancellation,
        interrupted: impl Fn() -> bool,
        function: F,
        check_result: bool,
    ) -> Result<TaskHandle<T>, RuntimeExecError>
    where
        T: Send + 'static,
        F: FnOnce(RuntimeCancellation) -> T + Send + 'static,
    {
        if !self.runtime.context_allows_current() {
            return Err(RuntimeExecError::RuntimeClosed);
        }
        let activity = self.runtime.begin_activity()?;
        let _submission = PoolInner::begin_submission(&self.inner);
        let child_token = cancellation.clone();
        let runtime_id = self.runtime.id;
        let (result_sender, receiver) = mpsc::sync_channel(1);
        let cancel_sender = result_sender.clone();
        let message_id = self.inner.next_message_id.fetch_add(1, Ordering::Relaxed);
        let hook_inner = Arc::downgrade(&self.inner);
        let cancel_hook: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            if let Some(inner) = hook_inner.upgrade() {
                inner.queue.remove(message_id);
            }
        });
        let cancel_registration = cancellation.register_cancel_hook(&cancel_hook);
        let message = PoolMessage {
            id: message_id,
            cancel: Box::new(move || {
                let _ = cancel_sender.send(Err(RuntimeExecError::Cancelled));
            }),
            run: Box::new(move || {
                let mut activity = activity;
                activity.mark_running();
                let _activity = activity;
                let _runtime_context = enter_runtime_context(runtime_id);
                let _cancellation_context = enter_cancellation_context(child_token.clone());
                let result = if child_token.is_cancelled() {
                    Err(RuntimeExecError::Cancelled)
                } else {
                    match catch_unwind(AssertUnwindSafe(|| function(child_token.clone()))) {
                        Ok(_result) if check_result && child_token.is_cancelled() => {
                            Err(RuntimeExecError::Cancelled)
                        }
                        Ok(result) => Ok(result),
                        Err(_) => Err(RuntimeExecError::JoinFailed),
                    }
                };
                let _ = result_sender.send(result);
            }),
        };
        self.enqueue_message(message, &cancellation, &interrupted)?;
        Ok(TaskHandle {
            receiver,
            cancellation,
            cancel_hook: Some(cancel_hook),
            cancel_registration,
        })
    }

    fn enqueue_message(
        &self,
        message: PoolMessage,
        cancellation: &RuntimeCancellation,
        interrupted: &impl Fn() -> bool,
    ) -> Result<(), RuntimeExecError> {
        let mut state = self
            .inner
            .queue
            .state
            .lock()
            .map_err(|_| RuntimeExecError::PoolClosed)?;
        loop {
            if interrupted() || cancellation.is_cancelled() {
                return Err(RuntimeExecError::Cancelled);
            }
            if self.runtime.is_closed() {
                return Err(RuntimeExecError::RuntimeClosed);
            }
            if self.inner.closed.load(Ordering::Acquire) {
                return Err(RuntimeExecError::PoolClosed);
            }
            if state.messages.len() < self.inner.queue.capacity {
                let message_id = message.id;
                state.messages.push_back(message);
                self.inner.queue.changed.notify_one();
                // cancellation can race the queue insertion after its callback
                // already scanned the queue. Recheck after insertion so that
                // this ordering cannot strand a cancelled message.
                if interrupted() || cancellation.is_cancelled() {
                    drop(state);
                    self.inner.queue.remove(message_id);
                    return Err(RuntimeExecError::Cancelled);
                }
                return Ok(());
            }
            let (next, _) = self
                .inner
                .queue
                .changed
                .wait_timeout(state, Duration::from_millis(2))
                .map_err(|_| RuntimeExecError::PoolClosed)?;
            state = next;
        }
    }

    /// 关闭任务池并等待 worker 退出。
    pub fn shutdown(&self) -> Result<(), RuntimeExecError> {
        if self.runtime.is_closed() {
            return Err(RuntimeExecError::RuntimeClosed);
        }
        if !self.runtime.context_allows_current() {
            return Err(RuntimeExecError::RuntimeClosed);
        }
        let _closing_guard = self.runtime.enter_current_thread_closing();
        Self::shutdown_inner(&self.inner);
        if self.runtime.is_closed() {
            Err(RuntimeExecError::RuntimeClosed)
        } else {
            Ok(())
        }
    }

    /// 停止接纳任务、排空已接纳工作并回收可等待的 worker。
    fn shutdown_inner(inner: &Arc<PoolInner>) {
        let should_join = {
            let Ok(_shutdown_gate) = inner.shutdown_gate.lock() else {
                return;
            };
            inner.queue.close(&inner.closed)
        };
        if let Ok(mut pending) = inner.pending_submissions.lock() {
            while *pending != 0 {
                let Ok(remaining) = inner.submissions_drained.wait(pending) else {
                    return;
                };
                pending = remaining;
            }
        }
        let current_worker = inner
            .worker_ids
            .lock()
            .map(|worker_ids| worker_ids.contains(&thread::current().id()))
            .unwrap_or(false);
        let drain = current_worker.then(|| Self::start_drain_helpers(inner));

        if should_join {
            if let Ok(mut workers) = inner.workers.lock() {
                for worker in workers.drain(..) {
                    let worker_id = worker.thread().id();
                    if worker_id == thread::current().id()
                        || Self::worker_is_closing(inner, worker_id)
                    {
                        // 当前 worker 不能 join 自己；丢弃 JoinHandle 让它在
                        // 返回任务后自然结束；已经进入另一条 shutdown 路径的
                        // worker 也不能被交叉池 join，避免形成关闭环。
                        continue;
                    }
                    let _ = worker.join();
                }
            }
        }
        if let Some(drains) = drain.flatten() {
            for drain in drains {
                let _ = drain.join();
            }
        }
    }

    /// 判断 worker 是否已由 Runtime 的另一条关闭路径接管。
    fn worker_is_closing(inner: &Arc<PoolInner>, worker_id: thread::ThreadId) -> bool {
        inner
            .runtime
            .upgrade()
            .and_then(|runtime| {
                runtime
                    .closing_threads
                    .lock()
                    .ok()
                    .map(|threads| threads.contains_key(&worker_id))
            })
            .unwrap_or(false)
    }

    /// 为关闭路径启动一次受控排队任务接管。
    fn start_drain_helpers(inner: &Arc<PoolInner>) -> Option<Vec<JoinHandle<()>>> {
        if inner
            .drain_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return None;
        }
        let mut drains = Vec::with_capacity(inner.drain_worker_count);
        for index in 0..inner.drain_worker_count {
            let queue = Arc::clone(&inner.queue);
            match thread::Builder::new()
                .name(format!("xiao-pool-shutdown-drain-{index}"))
                .spawn(move || Self::drain_pending_messages(&queue))
            {
                Ok(drain) => drains.push(drain),
                Err(_) => {
                    // 极端情况下线程创建失败，当前 worker 直接接管剩余消息，
                    // 避免把已接纳 activity 永久留在队列中。
                    Self::drain_pending_messages(&inner.queue);
                    break;
                }
            }
        }
        Some(drains)
    }

    /// 运行队列中已经接纳、但尚未被 worker 领取的任务。
    fn drain_pending_messages(queue: &Arc<PoolQueue>) {
        while let Some(message) = queue.take(false) {
            (message.run)();
        }
    }
}

impl Drop for TaskPool {
    /// 最后一个池别名释放时执行关闭。
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            let _ = self.shutdown();
        }
    }
}

#[cfg(test)]
/// Runtime/线程/任务池基础回归。
mod tests {
    use super::*;

    /// 串行化使用进程级 detached 错误通道的单元测试。
    static DETACHED_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    /// Runtime 关闭、取消和 join 边界。
    fn runtime_spawn_cancel_and_join_are_wired() {
        let runtime = RuntimeHandle::create();
        let handle = spawn(&runtime, |token| token.is_cancelled()).expect("spawn");
        handle.cancel();
        assert_eq!(handle.join(), Err(RuntimeExecError::Cancelled));
        runtime.close();
        assert!(matches!(
            spawn(&runtime, |_| 1_u32),
            Err(error) if error.code() == CONCURRENCY_RUNTIME_CLOSED_CODE
        ));
    }

    #[test]
    /// 任务池容量、提交、等待和关闭边界。
    fn task_pool_validates_capacity_and_waits_tasks() {
        let runtime = RuntimeHandle::create();
        assert!(matches!(
            TaskPool::create(runtime.clone(), 0),
            Err(error) if error.code() == CONCURRENCY_POOL_ARGUMENT_CODE
        ));
        let pool = TaskPool::create(runtime, 1).expect("pool");
        let task = pool.submit(|_| 42_u32).expect("submit");
        assert_eq!(task.wait().expect("wait"), 42);
        pool.shutdown().expect("池应成功关闭");
        assert!(matches!(
            pool.submit(|_| 1_u32),
            Err(error) if error.code() == CONCURRENCY_POOL_CLOSED_CODE
        ));
    }

    #[test]
    /// Runtime 关闭后，公开 shutdown 按冻结语义返回 031。
    fn pool_shutdown_after_runtime_close_returns_runtime_closed() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
        runtime.close();
        assert_eq!(pool.shutdown(), Err(RuntimeExecError::RuntimeClosed));
    }

    #[test]
    /// 池 worker 内重入 Runtime close 不等待自身，也不 join 自身。
    fn runtime_close_is_reentrant_from_pool_worker() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
        let task_runtime = runtime.clone();
        let task = pool
            .submit(move |_| {
                task_runtime.close();
                9_u32
            })
            .expect("submit");
        assert_eq!(task.wait(), Ok(9));
        assert!(runtime.is_closed());
        assert!(matches!(
            pool.submit(|_| 1_u32),
            Err(RuntimeExecError::RuntimeClosed)
        ));
    }

    #[test]
    /// 两个 worker 同时关闭同一池时只有一个 closer join，不能互相等待。
    fn two_pool_workers_can_close_same_pool_concurrently() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime, 2).expect("pool");
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let first_pool = pool.clone();
        let first_barrier = Arc::clone(&barrier);
        let first = pool
            .submit(move |_| {
                first_barrier.wait();
                first_pool.shutdown()
            })
            .expect("first close task");
        let second_pool = pool.clone();
        let second_barrier = Arc::clone(&barrier);
        let second = pool
            .submit(move |_| {
                second_barrier.wait();
                second_pool.shutdown()
            })
            .expect("second close task");
        assert_eq!(first.wait(), Ok(Ok(())));
        assert_eq!(second.wait(), Ok(Ok(())));
    }

    #[test]
    /// 两个活动执行单元同时 close 时只有一个 Runtime drain owner，不能互相等待。
    fn two_active_runtime_closers_do_not_deadlock() {
        let runtime = RuntimeHandle::create();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let (done_tx, done_rx) = mpsc::channel();

        let first_runtime = runtime.clone();
        let first_barrier = Arc::clone(&barrier);
        let first_done = done_tx.clone();
        let first = spawn(&runtime, move |_| {
            first_barrier.wait();
            first_runtime.close();
            first_done.send(()).unwrap();
        })
        .expect("first closer");

        let second_runtime = runtime.clone();
        let second_barrier = Arc::clone(&barrier);
        let second_done = done_tx;
        let second = spawn(&runtime, move |_| {
            second_barrier.wait();
            second_runtime.close();
            second_done.send(()).unwrap();
        })
        .expect("second closer");

        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("第一个活动 closer 必须返回");
        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("第二个活动 closer 必须返回");
        assert_eq!(first.join(), Ok(()));
        assert_eq!(second.join(), Ok(()));
        assert!(runtime.is_closed());
    }

    #[test]
    /// pool.shutdown 与 Runtime close 并发时，pool join 不得与 Runtime drain 互等。
    fn pool_shutdown_and_runtime_close_do_not_deadlock() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 2).expect("pool");
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let (done_tx, done_rx) = mpsc::channel();

        let first_pool = pool.clone();
        let first_barrier = Arc::clone(&barrier);
        let first_done = done_tx.clone();
        let first = pool
            .submit(move |_| {
                first_barrier.wait();
                let result = first_pool.shutdown();
                first_done.send(()).unwrap();
                result
            })
            .expect("pool closer");

        let second_runtime = runtime.clone();
        let second_barrier = Arc::clone(&barrier);
        let second_done = done_tx;
        let second = pool
            .submit(move |_| {
                second_barrier.wait();
                second_runtime.close();
                second_done.send(()).unwrap();
            })
            .expect("runtime closer");

        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("pool/runtime closer 必须返回");
        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("pool/runtime closer 必须都返回");
        assert!(matches!(
            first.wait(),
            Ok(Ok(())) | Ok(Err(RuntimeExecError::RuntimeClosed))
        ));
        assert_eq!(second.wait(), Ok(()));
        assert!(runtime.is_closed());
    }

    #[test]
    /// 两个 Rust pool worker 互相 shutdown 时，closing worker 不得被对方 join。
    fn cross_pool_shutdowns_do_not_deadlock() {
        let runtime = RuntimeHandle::create();
        let pool_a = TaskPool::create(runtime.clone(), 1).expect("pool A");
        let pool_b = TaskPool::create(runtime.clone(), 1).expect("pool B");
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let (done_tx, done_rx) = mpsc::channel();

        let pool_b_for_a = pool_b.clone();
        let barrier_a = Arc::clone(&barrier);
        let done_a = done_tx.clone();
        let task_a = pool_a
            .submit(move |_| {
                barrier_a.wait();
                let result = pool_b_for_a.shutdown();
                done_a.send(()).unwrap();
                result
            })
            .expect("pool A worker");

        let pool_a_for_b = pool_a.clone();
        let barrier_b = Arc::clone(&barrier);
        let done_b = done_tx;
        let task_b = pool_b
            .submit(move |_| {
                barrier_b.wait();
                let result = pool_a_for_b.shutdown();
                done_b.send(()).unwrap();
                result
            })
            .expect("pool B worker");

        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("第一个跨池 shutdown 必须返回");
        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("第二个跨池 shutdown 必须返回");
        assert_eq!(task_a.wait(), Ok(Ok(())));
        assert_eq!(task_b.wait(), Ok(Ok(())));
        runtime.close();
    }

    #[test]
    /// Rust API 不允许 Runtime worker 关闭另一个 Runtime 的 pool。
    fn cross_runtime_pool_shutdown_is_rejected() {
        let runtime_a = RuntimeHandle::create();
        let runtime_b = RuntimeHandle::create();
        let pool_a = TaskPool::create(runtime_a.clone(), 1).expect("pool A");
        let pool_b = TaskPool::create(runtime_b.clone(), 1).expect("pool B");
        let target_pool = pool_b.clone();
        let task = pool_a
            .submit(move |_| target_pool.shutdown())
            .expect("跨 Runtime 负例任务");

        assert_eq!(
            task.wait(),
            Ok(Err(RuntimeExecError::RuntimeClosed)),
            "跨 Runtime pool shutdown 必须返回 031"
        );
        assert!(!pool_b.inner.closed.load(Ordering::Acquire));
        pool_a.shutdown().expect("pool A shutdown");
        pool_b.shutdown().expect("pool B shutdown");
        runtime_a.close();
        runtime_b.close();
    }

    #[test]
    /// Runtime worker 不能跨 Runtime close 或 spawn；两者都返回 031。
    fn cross_runtime_runtime_close_and_spawn_are_rejected() {
        let runtime_a = RuntimeHandle::create();
        let runtime_b = RuntimeHandle::create();
        let pool_a = TaskPool::create(runtime_a.clone(), 1).expect("pool A");
        let target_runtime = runtime_b.clone();
        let task = pool_a
            .submit(move |_| {
                let close_rejected = target_runtime.try_close().is_err();
                let spawn_rejected = spawn(&target_runtime, |_| 1_u32).is_err();
                (close_rejected, spawn_rejected)
            })
            .expect("跨 Runtime Runtime 负例任务");

        assert_eq!(task.wait(), Ok((true, true)));
        assert!(!runtime_b.is_closed());
        pool_a.shutdown().expect("pool A shutdown");
        runtime_a.close();
        runtime_b.close();
    }

    #[test]
    /// Runtime 任务创建的子 Runtime 可在 owner 上下文中创建 pool 并执行任务。
    fn task_owned_runtime_can_create_pool_and_spawn() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 1).expect("parent pool");
        let task = pool
            .submit(move |_| {
                let child = RuntimeHandle::create();
                let child_pool = TaskPool::create(child.clone(), 1).expect("child pool");
                let child_task = child_pool.submit(|_| 42_u32).expect("child task");
                let child_thread = spawn(&child, |_| 7_u32).expect("child thread");
                let values = (child_task.wait().expect("child wait"), child_thread.join());
                child.try_close().expect("child close");
                (values, child.is_closed())
            })
            .expect("owner task");

        assert_eq!(task.wait(), Ok(((42, Ok(7)), true)));
        pool.shutdown().expect("parent pool shutdown");
        runtime.close();
    }

    #[test]
    /// 容量大于四时，唯一 close owner 仍能并行 drain 多个后续排队任务。
    fn large_pool_close_drains_many_queued_tasks() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 8).expect("pool");
        let mut release_txs = Vec::new();
        let mut blocked_handles = Vec::new();
        for index in 0..3 {
            let (started_tx, started_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            release_txs.push(release_tx);
            blocked_handles.push(
                pool.submit(move |_| {
                    started_tx.send(index).unwrap();
                    release_rx.recv().unwrap();
                    index
                })
                .expect("阻塞 worker"),
            );
            assert_eq!(started_rx.recv_timeout(Duration::from_secs(2)), Ok(index));
        }

        let (closer_started_tx, closer_started_rx) = mpsc::channel();
        let (close_go_tx, close_go_rx) = mpsc::channel();
        let (close_returned_tx, close_returned_rx) = mpsc::channel();
        let closer_runtime = runtime.clone();
        let closer = pool
            .submit(move |_| {
                closer_started_tx.send(()).unwrap();
                close_go_rx.recv().unwrap();
                closer_runtime.close();
                close_returned_tx.send(()).unwrap();
                99_u32
            })
            .expect("close owner");
        closer_started_rx.recv().expect("close owner 已开始");

        let (queued_started_tx, queued_started_rx) = mpsc::channel();
        let mut queued = Vec::new();
        for index in 0..5_u32 {
            let queued_started_tx = queued_started_tx.clone();
            queued.push(
                pool.submit(move |_| {
                    queued_started_tx.send(index).unwrap();
                    index
                })
                .expect("排队任务"),
            );
        }
        drop(queued_started_tx);
        close_go_tx.send(()).unwrap();
        for release_tx in release_txs {
            release_tx.send(()).unwrap();
        }
        let mut started = Vec::new();
        for _ in 0..5 {
            started.push(
                queued_started_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("排队任务必须被 drain 槽执行"),
            );
        }
        started.sort_unstable();
        assert_eq!(started, vec![0, 1, 2, 3, 4]);
        close_returned_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("大容量 pool close 必须返回");
        for (index, task) in queued.into_iter().enumerate() {
            assert_eq!(task.wait(), Ok(index as u32));
        }
        for (index, task) in blocked_handles.into_iter().enumerate() {
            assert_eq!(task.wait(), Ok(index));
        }
        assert_eq!(closer.wait(), Ok(99));
    }

    #[test]
    /// 同一 Runtime 的独立 pool shutdown 与 Runtime close 并发时保持可收敛。
    fn runtime_close_and_independent_pool_shutdown_converge() {
        let runtime = RuntimeHandle::create();
        let pool_a = TaskPool::create(runtime.clone(), 1).expect("pool A");
        let pool_b = TaskPool::create(runtime.clone(), 1).expect("pool B");
        let (blocked_started_tx, blocked_started_rx) = mpsc::channel();
        let (blocked_release_tx, blocked_release_rx) = mpsc::channel();
        let blocked = pool_a
            .submit(move |_| {
                blocked_started_tx.send(()).unwrap();
                blocked_release_rx.recv().unwrap();
                1_u32
            })
            .expect("pool A task");
        blocked_started_rx.recv().expect("pool A task 已开始");

        let (shutdown_go_tx, shutdown_go_rx) = mpsc::channel();
        let (shutdown_done_tx, shutdown_done_rx) = mpsc::channel();
        let pool_b_for_task = pool_b.clone();
        let independent = pool_b
            .submit(move |_| {
                shutdown_go_rx.recv().unwrap();
                let result = pool_b_for_task.shutdown();
                shutdown_done_tx.send(result.clone()).unwrap();
                result
            })
            .expect("pool B task");

        let close_runtime = runtime.clone();
        let closer = thread::spawn(move || close_runtime.close());
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !runtime.is_closed() {
            assert!(std::time::Instant::now() < deadline, "Runtime close 未开始");
            thread::yield_now();
        }
        shutdown_go_tx.send(()).unwrap();
        blocked_release_tx.send(()).unwrap();

        let shutdown_result = shutdown_done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("独立 pool shutdown 必须返回");
        assert!(matches!(
            shutdown_result,
            Ok(()) | Err(RuntimeExecError::RuntimeClosed)
        ));
        assert_eq!(blocked.wait(), Ok(1));
        assert!(matches!(
            independent.wait(),
            Ok(Ok(())) | Ok(Err(RuntimeExecError::RuntimeClosed))
        ));
        closer.join().expect("Runtime close 完成");
    }

    #[test]
    /// 外部 pool shutdown 已先接管 worker 后，运行中的 worker close 仍会 drain 排队任务。
    fn external_pool_shutdown_and_worker_runtime_close_drain_queue() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
        let (running_started_tx, running_started_rx) = mpsc::channel();
        let (close_go_tx, close_go_rx) = mpsc::channel();
        let (close_returned_tx, close_returned_rx) = mpsc::channel();
        let worker_runtime = runtime.clone();
        let running = pool
            .submit(move |_| {
                running_started_tx.send(()).unwrap();
                close_go_rx.recv().unwrap();
                worker_runtime.close();
                close_returned_tx.send(()).unwrap();
                1_u32
            })
            .expect("运行任务");
        running_started_rx.recv().expect("运行任务已开始");

        let (queued_started_tx, queued_started_rx) = mpsc::channel();
        let queued = pool
            .submit(move |_| {
                queued_started_tx.send(()).unwrap();
                2_u32
            })
            .expect("排队任务");

        let shutdown_pool = pool.clone();
        let shutdown_done = thread::spawn(move || shutdown_pool.shutdown());
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !pool.inner.closed.load(Ordering::Acquire) {
            assert!(
                std::time::Instant::now() < deadline,
                "外部 shutdown 未停止池接纳"
            );
            thread::yield_now();
        }
        close_go_tx.send(()).unwrap();
        queued_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("worker close 必须移交排队任务");
        close_returned_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("worker close 必须返回");
        assert_eq!(running.wait(), Ok(1));
        assert_eq!(queued.wait(), Ok(2));
        assert_eq!(
            shutdown_done.join().expect("外部 shutdown 完成"),
            Err(RuntimeExecError::RuntimeClosed)
        );
    }

    #[test]
    /// 满队列提交可以由外部令牌取消，并返回 027。
    fn full_queue_submission_has_cancellable_backpressure() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime, 1).expect("pool");
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let running = pool
            .submit(move |_| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                1_u32
            })
            .expect("running task");
        started_rx.recv().expect("worker 已开始");
        let queued = pool.submit(|_| 2_u32).expect("队列任务");

        let cancellation = RuntimeCancellation::default();
        let submit_cancellation = cancellation.clone();
        let submit_pool = pool.clone();
        let submitter =
            thread::spawn(move || submit_pool.submit_with_cancel(submit_cancellation, |_| 2_u32));
        thread::sleep(Duration::from_millis(10));
        cancellation.cancel();
        assert!(matches!(
            submitter.join().expect("提交线程完成"),
            Err(RuntimeExecError::Cancelled)
        ));
        release_tx.send(()).unwrap();
        assert_eq!(running.wait(), Ok(1));
        assert_eq!(queued.wait(), Ok(2));
    }

    #[test]
    /// 普通 submit 从当前 worker 继承取消上下文，满队列不会永久阻塞。
    fn default_submit_inherits_worker_cancellation_context() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime, 2).expect("pool");
        let (block_started_tx, block_started_rx) = mpsc::channel();
        let (block_release_tx, block_release_rx) = mpsc::channel();
        let blocker = pool
            .submit(move |_| {
                block_started_tx.send(()).unwrap();
                block_release_rx.recv().unwrap();
                1_u32
            })
            .expect("阻塞任务");
        block_started_rx.recv().expect("阻塞任务已开始");

        let (parent_started_tx, parent_started_rx) = mpsc::channel();
        let (parent_proceed_tx, parent_proceed_rx) = mpsc::channel();
        let parent_pool = pool.clone();
        let parent = pool
            .submit(move |_| {
                parent_started_tx.send(()).unwrap();
                parent_proceed_rx.recv().unwrap();
                let nested = parent_pool.submit(|_| 3_u32);
                assert!(matches!(nested, Err(RuntimeExecError::Cancelled)));
                2_u32
            })
            .expect("父任务");
        parent_started_rx.recv().expect("父任务已开始");

        let queued_a = pool.submit(|_| 4_u32).expect("填充队列 A");
        let queued_b = pool.submit(|_| 5_u32).expect("填充队列 B");
        parent.cancel();
        parent_proceed_tx.send(()).unwrap();
        assert_eq!(parent.wait(), Err(RuntimeExecError::Cancelled));
        block_release_tx.send(()).unwrap();
        assert_eq!(blocker.wait(), Ok(1));
        assert_eq!(queued_a.wait(), Ok(4));
        assert_eq!(queued_b.wait(), Ok(5));
    }

    #[test]
    /// 无上下文入口线程的普通 submit 可由 Runtime 关闭终止满队列背压。
    fn default_submit_from_external_thread_stops_on_runtime_close() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
        let (running_started_tx, running_started_rx) = mpsc::channel();
        let (running_release_tx, running_release_rx) = mpsc::channel();
        let running = pool
            .submit(move |_| {
                running_started_tx.send(()).unwrap();
                running_release_rx.recv().unwrap();
                1_u32
            })
            .expect("运行任务");
        running_started_rx.recv().expect("运行任务已开始");
        let queued = pool.submit(|_| 2_u32).expect("队列任务");

        let submit_pool = pool.clone();
        let submitter = thread::spawn(move || submit_pool.submit(|_| 3_u32));
        thread::sleep(Duration::from_millis(20));
        let close_runtime = runtime.clone();
        let closer = thread::spawn(move || close_runtime.close());
        for _ in 0..100 {
            if runtime.is_closed() {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(runtime.is_closed(), "关闭线程应先停止 Runtime 接纳");
        running_release_tx.send(()).unwrap();

        assert!(matches!(
            submitter.join().expect("外部提交线程完成"),
            Err(RuntimeExecError::RuntimeClosed)
        ));
        closer.join().expect("Runtime close 完成");
        assert_eq!(running.wait(), Ok(1));
        assert_eq!(queued.wait(), Ok(2));
    }

    #[test]
    /// 排队任务取消后立即返回，并释放逻辑队列槽供后续提交使用。
    fn queued_task_cancel_releases_slot_before_worker_is_free() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let blocker = pool
            .submit(move |_| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                1_u32
            })
            .expect("阻塞任务");
        started_rx.recv().expect("阻塞任务已开始");
        let queued = pool.submit(|_| 2_u32).expect("排队任务");
        queued.cancel();
        let started = std::time::Instant::now();
        assert_eq!(
            queued.wait_interruptible(|| false),
            Err(RuntimeExecError::Cancelled)
        );
        assert!(started.elapsed() < Duration::from_millis(100));
        assert_eq!(
            *runtime
                .inner
                .active
                .lock()
                .expect("Runtime 活动计数锁未中毒"),
            1,
            "取消排队任务应释放其 Runtime activity lease"
        );
        let replacement = pool.submit(|_| 3_u32).expect("取消后应能重新提交");
        assert_eq!(
            pool.inner
                .queue
                .state
                .lock()
                .expect("任务池队列锁未中毒")
                .messages
                .len(),
            1,
            "已取消任务必须从队列移除，只留下替代任务"
        );
        release_tx.send(()).unwrap();
        assert_eq!(blocker.wait(), Ok(1));
        assert_eq!(replacement.wait(), Ok(3));
        pool.shutdown().expect("pool shutdown");
    }

    #[test]
    /// 公开 wait API 在令牌取消后即使任务体已返回仍返回 027。
    fn completed_task_cancellation_is_observed_by_public_wait_apis() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 1).expect("pool");

        let waited = pool.submit(|_| 7_u32).expect("submit");
        thread::sleep(Duration::from_millis(10));
        waited.cancel();
        assert_eq!(waited.wait(), Err(RuntimeExecError::Cancelled));

        let interrupted = pool.submit(|_| 8_u32).expect("submit");
        thread::sleep(Duration::from_millis(10));
        interrupted.cancel();
        assert_eq!(
            interrupted.wait_interruptible(|| false),
            Err(RuntimeExecError::Cancelled)
        );
        pool.shutdown().expect("shutdown");
        runtime.close();
    }

    #[test]
    /// 结果到达时才变为 true 的宿主中断仍覆盖已完成任务结果。
    fn wait_interruptible_checks_host_interrupt_after_receiving_result() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
        let (release_tx, release_rx) = mpsc::channel();
        let task = pool
            .submit(move |_| {
                release_rx.recv().expect("host poll releases task");
                7_u32
            })
            .expect("task");
        let release_tx = Mutex::new(Some(release_tx));
        let polls = AtomicUsize::new(0);
        assert_eq!(
            task.wait_interruptible(|| {
                if polls.fetch_add(1, Ordering::AcqRel) == 0 {
                    release_tx
                        .lock()
                        .expect("release sender lock")
                        .take()
                        .expect("release exactly once")
                        .send(())
                        .expect("release task");
                    false
                } else {
                    true
                }
            }),
            Err(RuntimeExecError::Cancelled)
        );
        pool.shutdown().expect("shutdown");
        runtime.close();
    }

    #[test]
    /// 接收器断开时仍检查最后一次宿主中断，不能把 027 漏成 028。
    fn wait_interruptible_checks_host_interrupt_on_disconnect() {
        let (sender, receiver) = mpsc::sync_channel::<Result<u32, RuntimeExecError>>(1);
        drop(sender);
        let cancellation = RuntimeCancellation::default();
        let task = TaskHandle {
            receiver,
            cancellation,
            cancel_hook: None,
            cancel_registration: None,
        };
        let polls = AtomicUsize::new(0);
        assert_eq!(
            task.wait_interruptible(|| polls.fetch_add(1, Ordering::AcqRel) != 0),
            Err(RuntimeExecError::Cancelled)
        );
    }

    #[test]
    /// 任务 panic 转成 028，worker 保持存活并继续接收工作。
    fn task_panic_is_reported_without_killing_pool_worker() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime, 1).expect("pool");
        let failed = pool
            .submit(|_| panic!("task panic regression"))
            .expect("submit");
        assert_eq!(failed.wait().unwrap_err().code(), "X06-RUNTIME-028");
        assert_eq!(pool.submit(|_| 42_u32).unwrap().wait(), Ok(42));
        pool.shutdown().expect("池应成功关闭");
    }

    #[test]
    /// detached 线程 panic 进入进程级 028 通道。
    fn detached_thread_panic_is_reported_to_process_channel() {
        let _test_lock = DETACHED_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("detached 测试锁未中毒");
        let _ = take_detached_errors();
        let runtime = RuntimeHandle::create();
        spawn(&runtime, |_| panic!("detached panic regression"))
            .expect("spawn")
            .detach();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let errors = loop {
            let errors = take_detached_errors();
            if !errors.is_empty() {
                break errors;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "detached 错误未进入通道"
            );
            thread::yield_now();
        };
        assert!(errors.iter().any(|error| error.code() == "X06-RUNTIME-028"));
        runtime.close();
    }

    #[test]
    /// detached 取消进入 027，panic 进入 028；长任务不会阻塞后续错误收集。
    fn detached_errors_are_classified_and_reaped_concurrently() {
        let _test_lock = DETACHED_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("detached 测试锁未中毒");
        let _ = take_detached_errors();
        let runtime = RuntimeHandle::create();
        let (release_tx, release_rx) = mpsc::channel();
        spawn(&runtime, move |_| {
            release_rx.recv().unwrap();
        })
        .expect("长 detached 任务")
        .detach();
        let cancelled = spawn(&runtime, |token| {
            while !token.is_cancelled() {
                thread::yield_now();
            }
            1_u32
        })
        .expect("可取消任务");
        cancelled.cancel();
        cancelled.detach();
        spawn(&runtime, |_| panic!("detached classification panic"))
            .expect("panic 任务")
            .detach();

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut errors = Vec::new();
        let errors = loop {
            errors.extend(take_detached_errors());
            if errors
                .iter()
                .any(|error| error.code() == CONCURRENCY_CANCELLED_CODE)
                && errors.iter().any(|error| error.code() == "X06-RUNTIME-028")
            {
                break errors;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "detached 错误未及时收集"
            );
            thread::yield_now();
        };
        assert!(
            errors
                .iter()
                .any(|error| error.code() == CONCURRENCY_CANCELLED_CODE)
        );
        assert!(errors.iter().any(|error| error.code() == "X06-RUNTIME-028"));
        release_tx.send(()).unwrap();
        runtime.close();
    }

    #[test]
    /// Runtime 关闭拒绝新任务，并等待已经接纳的线程退出。
    fn runtime_close_stops_admission_and_drains_threads() {
        let runtime = RuntimeHandle::create();
        let worker_runtime = runtime.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker = spawn(&runtime, move |_| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        })
        .expect("spawn");
        started_rx.recv().expect("线程开始");

        let (closed_tx, closed_rx) = mpsc::channel();
        let closer = thread::spawn(move || {
            worker_runtime.close();
            closed_tx.send(()).unwrap();
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !runtime.is_closed() {
            assert!(
                std::time::Instant::now() < deadline,
                "Runtime 未进入关闭状态"
            );
            thread::yield_now();
        }
        assert!(matches!(
            spawn(&runtime, |_| ()),
            Err(RuntimeExecError::RuntimeClosed)
        ));
        assert!(closed_rx.try_recv().is_err(), "关闭必须等待已接纳线程");
        release_tx.send(()).unwrap();
        assert_eq!(worker.join(), Ok(()));
        closed_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        closer.join().unwrap();
    }

    #[test]
    /// 活动 Runtime 线程调用 close 时仍等待其他 activity，只豁免自身 lease。
    fn active_thread_close_drains_other_threads_before_returning() {
        let runtime = RuntimeHandle::create();
        let (blocked_started_tx, blocked_started_rx) = mpsc::channel();
        let (blocked_release_tx, blocked_release_rx) = mpsc::channel();
        let blocked = spawn(&runtime, move |_| {
            blocked_started_tx.send(()).unwrap();
            blocked_release_rx.recv().unwrap();
        })
        .expect("blocked worker");
        blocked_started_rx.recv().expect("blocked worker started");

        let (close_started_tx, close_started_rx) = mpsc::channel();
        let (close_returned_tx, close_returned_rx) = mpsc::channel();
        let close_runtime = runtime.clone();
        let closer = spawn(&runtime, move |_| {
            close_started_tx.send(()).unwrap();
            close_runtime.close();
            close_returned_tx.send(()).unwrap();
        })
        .expect("active closer");
        close_started_rx.recv().expect("active closer started");
        assert!(close_returned_rx.try_recv().is_err(), "不能提前返回");
        blocked_release_tx.send(()).unwrap();
        assert_eq!(blocked.join(), Ok(()));
        assert_eq!(closer.join(), Ok(()));
        assert!(runtime.is_closed());
    }

    #[test]
    /// 活动 follower 可快速返回，但唯一 owner 仍等待未进入关闭路径的 activity。
    fn active_close_follower_leaves_owner_to_drain_other_activity() {
        let runtime = RuntimeHandle::create();
        let (blocked_started_tx, blocked_started_rx) = mpsc::channel();
        let (blocked_release_tx, blocked_release_rx) = mpsc::channel();
        let blocked = spawn(&runtime, move |_| {
            blocked_started_tx.send(()).unwrap();
            blocked_release_rx.recv().unwrap();
        })
        .expect("blocked worker");
        blocked_started_rx.recv().expect("blocked worker started");

        let (follower_started_tx, follower_started_rx) = mpsc::channel();
        let (follower_go_tx, follower_go_rx) = mpsc::channel();
        let (follower_returned_tx, follower_returned_rx) = mpsc::channel();
        let (follower_release_tx, follower_release_rx) = mpsc::channel();
        let follower_runtime = runtime.clone();
        let follower = spawn(&runtime, move |_| {
            follower_started_tx.send(()).unwrap();
            follower_go_rx.recv().unwrap();
            follower_runtime.close();
            follower_returned_tx.send(()).unwrap();
            follower_release_rx.recv().unwrap();
        })
        .expect("active follower");
        follower_started_rx.recv().expect("active follower started");

        let owner_runtime = runtime.clone();
        let owner_done = thread::spawn(move || owner_runtime.close());
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !runtime.is_closed() {
            assert!(
                std::time::Instant::now() < deadline,
                "Runtime owner 未开始 drain"
            );
            thread::yield_now();
        }
        follower_go_tx.send(()).unwrap();
        follower_returned_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("活动 follower 应在 owner drain 期间返回");
        assert!(!owner_done.is_finished(), "owner 不得绕过普通 activity");

        follower_release_tx.send(()).unwrap();
        blocked_release_tx.send(()).unwrap();
        owner_done.join().expect("Runtime owner 完成");
        assert_eq!(blocked.join(), Ok(()));
        assert_eq!(follower.join(), Ok(()));
    }

    #[test]
    /// 活动 pool worker close 时也要等待尚未开始但已接纳的排队 activity。
    fn active_close_waits_for_queued_pool_activity() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 2).expect("pool");
        let (blocked_started_tx, blocked_started_rx) = mpsc::channel();
        let (blocked_release_tx, blocked_release_rx) = mpsc::channel();
        let blocked = pool
            .submit(move |_| {
                blocked_started_tx.send(()).unwrap();
                blocked_release_rx.recv().unwrap();
                1_u32
            })
            .expect("阻塞任务");
        blocked_started_rx.recv().expect("阻塞任务已开始");

        let (closer_started_tx, closer_started_rx) = mpsc::channel();
        let (close_go_tx, close_go_rx) = mpsc::channel();
        let (close_returned_tx, close_returned_rx) = mpsc::channel();
        let closer_runtime = runtime.clone();
        let closer = pool
            .submit(move |_| {
                closer_started_tx.send(()).unwrap();
                close_go_rx.recv().unwrap();
                closer_runtime.close();
                close_returned_tx.send(()).unwrap();
                2_u32
            })
            .expect("Runtime closer");
        closer_started_rx.recv().expect("Runtime closer 已开始");

        let (queued_started_tx, queued_started_rx) = mpsc::channel();
        let (queued_release_tx, queued_release_rx) = mpsc::channel();
        let queued = pool
            .submit(move |_| {
                queued_started_tx.send(()).unwrap();
                queued_release_rx.recv().unwrap();
                3_u32
            })
            .expect("排队任务");
        close_go_tx.send(()).unwrap();
        assert!(
            close_returned_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "close 不得绕过排队 activity"
        );

        blocked_release_tx.send(()).unwrap();
        queued_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("释放 worker 后排队任务必须运行");
        assert!(
            close_returned_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "排队任务仍在运行时 close 不得返回"
        );
        queued_release_tx.send(()).unwrap();

        close_returned_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("排队 activity 完成后 close 必须返回");
        assert_eq!(blocked.wait(), Ok(1));
        assert_eq!(queued.wait(), Ok(3));
        assert_eq!(closer.wait(), Ok(2));
        assert!(runtime.is_closed());
    }

    #[test]
    /// 单 worker 也必须在 close 期间移交排队任务，避免 active 总数等待自锁。
    fn active_close_drains_queued_activity_with_single_worker() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
        let (closer_started_tx, closer_started_rx) = mpsc::channel();
        let (close_go_tx, close_go_rx) = mpsc::channel();
        let (close_returned_tx, close_returned_rx) = mpsc::channel();
        let closer_runtime = runtime.clone();
        let closer = pool
            .submit(move |_| {
                closer_started_tx.send(()).unwrap();
                close_go_rx.recv().unwrap();
                closer_runtime.close();
                close_returned_tx.send(()).unwrap();
                1_u32
            })
            .expect("Runtime closer");
        closer_started_rx.recv().expect("Runtime closer 已开始");

        let (queued_started_tx, queued_started_rx) = mpsc::channel();
        let (queued_release_tx, queued_release_rx) = mpsc::channel();
        let queued = pool
            .submit(move |_| {
                queued_started_tx.send(()).unwrap();
                queued_release_rx.recv().unwrap();
                2_u32
            })
            .expect("排队任务");
        close_go_tx.send(()).unwrap();
        queued_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("single worker close 必须移交排队任务");
        assert!(close_returned_rx.try_recv().is_err());
        queued_release_tx.send(()).unwrap();
        close_returned_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("排队任务完成后 close 必须返回");
        assert_eq!(queued.wait(), Ok(2));
        assert_eq!(closer.wait(), Ok(1));
    }

    #[test]
    /// 当前 worker 的 drain helper 执行排队 close 时也不能重新形成关闭环。
    fn shutdown_drain_helper_handles_queued_runtime_close() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 2).expect("pool");
        let (blocked_started_tx, blocked_started_rx) = mpsc::channel();
        let (blocked_release_tx, blocked_release_rx) = mpsc::channel();
        let blocked = pool
            .submit(move |_| {
                blocked_started_tx.send(()).unwrap();
                blocked_release_rx.recv().unwrap();
                0_u32
            })
            .expect("阻塞 worker");
        blocked_started_rx.recv().expect("阻塞 worker 已开始");

        let (closer_started_tx, closer_started_rx) = mpsc::channel();
        let (close_go_tx, close_go_rx) = mpsc::channel();
        let (close_returned_tx, close_returned_rx) = mpsc::channel();
        let closer_runtime = runtime.clone();
        let closer = pool
            .submit(move |_| {
                closer_started_tx.send(()).unwrap();
                close_go_rx.recv().unwrap();
                closer_runtime.close();
                close_returned_tx.send(()).unwrap();
                1_u32
            })
            .expect("Runtime closer");
        closer_started_rx.recv().expect("Runtime closer 已开始");

        let (queued_started_tx, queued_started_rx) = mpsc::channel();
        let (followup_result_tx, followup_result_rx) = mpsc::channel();
        let queued_runtime = runtime.clone();
        let queued = pool
            .submit(move |_| {
                queued_runtime.close();
                queued_started_tx.send(()).unwrap();
                followup_result_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("排队 close 任务必须取得后续任务结果");
                2_u32
            })
            .expect("排队 close 任务");
        let followup = pool
            .submit(move |_| {
                followup_result_tx.send(9_u32).unwrap();
                3_u32
            })
            .expect("排队后续任务");
        close_go_tx.send(()).unwrap();
        queued_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("drain helper 必须执行排队 close 任务");
        assert_eq!(followup.wait(), Ok(3));
        assert_eq!(queued.wait(), Ok(2));
        blocked_release_tx.send(()).unwrap();
        close_returned_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("主 worker close 必须返回");
        assert_eq!(blocked.wait(), Ok(0));
        assert_eq!(closer.wait(), Ok(1));
        assert!(runtime.is_closed());
    }

    #[test]
    /// detached 回收池忙碌时，扩展队列策略仍让 detach 发送立即返回。
    fn detached_reaper_queue_does_not_block_when_workers_are_busy() {
        let _test_lock = DETACHED_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("detached 测试锁未中毒");
        let (started_tx, started_rx) = mpsc::channel();
        let mut release_txs = Vec::new();
        for _ in 0..4 {
            let started_tx = started_tx.clone();
            let (release_tx, release_rx) = mpsc::channel();
            release_txs.push(release_tx);
            enqueue_detached_reap(Box::new(move || {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            }))
            .expect("发送 detached 回收任务");
        }
        drop(started_tx);
        for _ in 0..4 {
            started_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("四个 detached worker 必须占用回收槽");
        }

        let baseline_depth = detached_reaper_queue_depth();
        let start = std::time::Instant::now();
        for _ in 0..8 {
            for _ in 0..512 {
                enqueue_detached_reap(Box::new(|| {})).expect("无界回收队列必须可继续接收");
            }
        }
        assert!(
            detached_reaper_queue_depth() >= baseline_depth + 4096,
            "持续积压必须可观测"
        );
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "回收队列满时发送不应阻塞调用方"
        );
        for release_tx in release_txs {
            release_tx.send(()).unwrap();
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while detached_reaper_queue_depth() > baseline_depth {
            assert!(
                std::time::Instant::now() < deadline,
                "detached 回收队列积压未能清空"
            );
            thread::yield_now();
        }
    }

    #[test]
    /// Runtime 关闭会关闭所属任务池并 drain 已提交任务。
    fn runtime_close_shuts_down_and_drains_owned_pools() {
        let runtime = RuntimeHandle::create();
        let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let task = pool
            .submit(move |_| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                7
            })
            .expect("submit");
        started_rx.recv().expect("任务开始");
        let worker_runtime = runtime.clone();
        let closer = thread::spawn(move || worker_runtime.close());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !runtime.is_closed() {
            assert!(std::time::Instant::now() < deadline, "Runtime 未关闭");
            thread::yield_now();
        }
        release_tx.send(()).unwrap();
        assert_eq!(task.wait(), Ok(7));
        closer.join().unwrap();
        assert!(matches!(
            pool.submit(|_| 8),
            Err(RuntimeExecError::RuntimeClosed)
        ));
    }
}
