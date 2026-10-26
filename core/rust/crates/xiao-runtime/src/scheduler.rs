//! 22C 最小 Runtime/线程/任务池执行骨架。
//!
//! 该模块只提供 Runtime 层的生命周期和取消边界；Xiao 标准库函数体及 VM opcode
//! lowering 仍由后续 22C 子批接入。句柄不复用普通单线程 `StrongHandle`。

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub use crate::concurrency::LockCancellation as RuntimeCancellation;
use crate::concurrency::global_runtime_registry;
use crate::{
    CONCURRENCY_CANCELLED_CODE, CONCURRENCY_POOL_ARGUMENT_CODE, CONCURRENCY_POOL_CLOSED_CODE,
    CONCURRENCY_RUNTIME_CLOSED_CODE,
};

thread_local! {
    static CURRENT_CANCELLATION: RefCell<Option<RuntimeCancellation>> = const { RefCell::new(None) };
}

struct CancellationContextGuard {
    previous: Option<RuntimeCancellation>,
}

impl Drop for CancellationContextGuard {
    fn drop(&mut self) {
        CURRENT_CANCELLATION.with(|current| {
            *current.borrow_mut() = self.previous.take();
        });
    }
}

fn enter_cancellation_context(token: RuntimeCancellation) -> CancellationContextGuard {
    let previous = CURRENT_CANCELLATION.with(|current| current.borrow_mut().replace(token));
    CancellationContextGuard { previous }
}

fn current_cancellation_context() -> Option<RuntimeCancellation> {
    CURRENT_CANCELLATION.with(|current| current.borrow().clone())
}

/// 可复制的 Runtime 句柄。
#[derive(Clone, Debug)]
pub struct RuntimeHandle {
    id: u64,
    inner: Arc<RuntimeInner>,
}

#[derive(Debug)]
struct RuntimeInner {
    closed: AtomicBool,
    active: Mutex<usize>,
    drained: Condvar,
    close_gate: Mutex<()>,
    pools: Mutex<Vec<Weak<PoolInner>>>,
    active_threads: Mutex<HashMap<thread::ThreadId, usize>>,
}

struct ActivityLease {
    runtime: Arc<RuntimeInner>,
    running_thread: Option<thread::ThreadId>,
}

impl ActivityLease {
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
        let inner = Arc::new(RuntimeInner {
            closed: AtomicBool::new(false),
            active: Mutex::new(0),
            drained: Condvar::new(),
            close_gate: Mutex::new(()),
            pools: Mutex::new(Vec::new()),
            active_threads: Mutex::new(HashMap::new()),
        });
        Self { id, inner }
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

    /// 幂等关闭 Runtime；旧别名可观察相同关闭状态。
    pub fn close(&self) {
        let pools = {
            let Ok(_close_gate) = self.inner.close_gate.lock() else {
                return;
            };
            if !self.inner.closed.swap(true, Ordering::AcqRel) {
                global_runtime_registry().close(self.id);
            }
            self.inner
                .pools
                .lock()
                .map(|mut pools| {
                    let live = pools.iter().filter_map(Weak::upgrade).collect::<Vec<_>>();
                    pools.clear();
                    live
                })
                .unwrap_or_default()
        };
        for pool in pools {
            TaskPool::shutdown_inner(&pool);
        }
        // 当前执行单元不能等待自己的 ActivityLease；让它返回并由外层
        // close 调用或 lease Drop 完成最终 drain。
        let current_active = self.current_thread_active_count();
        if current_active != 0 {
            self.wait_for_other_active(current_active);
            return;
        }
        let Ok(mut active) = self.inner.active.lock() else {
            return;
        };
        while *active != 0 {
            let Ok(remaining) = self.inner.drained.wait(active) else {
                return;
            };
            active = remaining;
        }
    }

    fn current_thread_active_count(&self) -> usize {
        self.inner
            .active_threads
            .lock()
            .ok()
            .and_then(|threads| threads.get(&thread::current().id()).copied())
            .unwrap_or(0)
    }

    fn wait_for_other_active(&self, current_active: usize) {
        let Ok(mut running) = self.inner.active_threads.lock() else {
            return;
        };
        while running.values().copied().sum::<usize>() > current_active {
            drop(running);
            let Ok(active) = self.inner.active.lock() else {
                return;
            };
            let Ok(_) = self
                .inner
                .drained
                .wait_timeout(active, Duration::from_millis(10))
            else {
                return;
            };
            running = match self.inner.active_threads.lock() {
                Ok(running) => running,
                Err(_) => return,
            };
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
    pub fn detach(mut self) {
        if let Some(join) = self.join.take() {
            detached_reaper()
                .send(Box::new(move || {
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
    static ERRORS: OnceLock<Mutex<Vec<RuntimeExecError>>> = OnceLock::new();
    ERRORS.get_or_init(|| Mutex::new(Vec::new()))
}

fn detached_reaper() -> &'static mpsc::SyncSender<Box<dyn FnOnce() + Send>> {
    const DETACHED_JOIN_WORKERS: usize = 4;
    const DETACHED_QUEUE_CAPACITY: usize = 256;
    static REAPER: OnceLock<mpsc::SyncSender<Box<dyn FnOnce() + Send>>> = OnceLock::new();
    REAPER.get_or_init(|| {
        let (sender, receiver) =
            mpsc::sync_channel::<Box<dyn FnOnce() + Send>>(DETACHED_QUEUE_CAPACITY);
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

/// 取走进程级 detached 失败记录；记录采用 X06-RUNTIME-028。
#[must_use]
pub fn take_detached_errors() -> Vec<RuntimeExecError> {
    detached_errors()
        .lock()
        .map(|mut errors| std::mem::take(&mut *errors))
        .unwrap_or_default()
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
    let activity = runtime.begin_activity()?;
    let cancellation = RuntimeCancellation::default();
    let child_token = cancellation.clone();
    let join = thread::Builder::new()
        .name(format!("xiao-runtime-{}", runtime.id))
        .spawn(move || {
            let mut activity = activity;
            activity.mark_running();
            let _activity = activity;
            let _cancellation_context = enter_cancellation_context(child_token.clone());
            if child_token.is_cancelled() {
                return Err(RuntimeExecError::Cancelled);
            }
            let result = function(child_token.clone());
            if child_token.is_cancelled() {
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

/// 任务池 worker 消息。
struct PoolMessage {
    run: Box<dyn FnOnce() + Send + 'static>,
}

/// 任务池共享内部状态。
struct PoolInner {
    sender: Mutex<Option<SyncSender<PoolMessage>>>,
    workers: Mutex<Vec<JoinHandle<()>>>,
    closed: AtomicBool,
    shutdown_gate: Mutex<()>,
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
}

impl<T> TaskHandle<T> {
    /// 借用取消任务；取消不消费任务句柄。
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    /// 消费任务句柄并等待结果。
    pub fn wait(self) -> Result<T, RuntimeExecError> {
        self.receiver
            .recv()
            .unwrap_or(Err(RuntimeExecError::JoinFailed))
    }
}

impl TaskPool {
    /// 创建固定容量任务池；容量范围为 1–256。
    pub fn create(runtime: RuntimeHandle, capacity: usize) -> Result<Self, RuntimeExecError> {
        if !(1..=256).contains(&capacity) {
            return Err(RuntimeExecError::PoolArgument);
        }
        let (sender, receiver) = mpsc::sync_channel::<PoolMessage>(capacity);
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(capacity.min(4));
        for index in 0..capacity.min(4) {
            let receiver = Arc::clone(&receiver);
            workers.push(
                thread::Builder::new()
                    .name(format!("xiao-pool-{}-{}", runtime.id, index))
                    .spawn(move || {
                        loop {
                            let message = receiver
                                .lock()
                                .ok()
                                .and_then(|receiver| receiver.recv().ok());
                            let Some(message) = message else { break };
                            (message.run)();
                        }
                    })
                    .map_err(|_| RuntimeExecError::JoinFailed)?,
            );
        }
        let inner = Arc::new(PoolInner {
            sender: Mutex::new(Some(sender)),
            workers: Mutex::new(workers),
            closed: AtomicBool::new(false),
            shutdown_gate: Mutex::new(()),
        });
        if let Err(error) = runtime.register_pool(&inner) {
            Self::shutdown_inner(&inner);
            return Err(error);
        }
        Ok(Self { runtime, inner })
    }

    /// 提交一个显式接收取消令牌的任务。
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
        let activity = self.runtime.begin_activity()?;
        let child_token = cancellation.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let message = PoolMessage {
            run: Box::new(move || {
                let mut activity = activity;
                activity.mark_running();
                let _activity = activity;
                let _cancellation_context = enter_cancellation_context(child_token.clone());
                let result = if child_token.is_cancelled() {
                    Err(RuntimeExecError::Cancelled)
                } else {
                    match catch_unwind(AssertUnwindSafe(|| function(child_token.clone()))) {
                        Ok(_result) if child_token.is_cancelled() => {
                            Err(RuntimeExecError::Cancelled)
                        }
                        Ok(result) => Ok(result),
                        Err(_) => Err(RuntimeExecError::JoinFailed),
                    }
                };
                let _ = sender.send(result);
            }),
        };
        let sender = {
            let sender = self
                .inner
                .sender
                .lock()
                .map_err(|_| RuntimeExecError::PoolClosed)?;
            if self.inner.closed.load(Ordering::Acquire) {
                return Err(RuntimeExecError::PoolClosed);
            }
            sender
                .as_ref()
                .cloned()
                .ok_or(RuntimeExecError::PoolClosed)?
        };
        let mut message = Some(message);
        loop {
            if cancellation.is_cancelled() {
                return Err(RuntimeExecError::Cancelled);
            }
            if self.runtime.is_closed() {
                return Err(RuntimeExecError::RuntimeClosed);
            }
            if self.inner.closed.load(Ordering::Acquire) {
                return Err(RuntimeExecError::PoolClosed);
            }
            match sender.try_send(message.take().expect("提交消息只消费一次")) {
                Ok(()) => break,
                Err(mpsc::TrySendError::Full(message_back)) => {
                    message = Some(message_back);
                    thread::park_timeout(Duration::from_millis(1));
                }
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    return Err(if self.runtime.is_closed() {
                        RuntimeExecError::RuntimeClosed
                    } else {
                        RuntimeExecError::PoolClosed
                    });
                }
            }
        }
        Ok(TaskHandle {
            receiver,
            cancellation,
        })
    }

    /// 关闭任务池并等待 worker 退出。
    pub fn shutdown(&self) -> Result<(), RuntimeExecError> {
        if self.runtime.is_closed() {
            return Err(RuntimeExecError::RuntimeClosed);
        }
        Self::shutdown_inner(&self.inner);
        if self.runtime.is_closed() {
            Err(RuntimeExecError::RuntimeClosed)
        } else {
            Ok(())
        }
    }

    fn shutdown_inner(inner: &Arc<PoolInner>) {
        let should_join = {
            let Ok(_shutdown_gate) = inner.shutdown_gate.lock() else {
                return;
            };
            if inner.closed.swap(true, Ordering::AcqRel) {
                false
            } else {
                if let Ok(mut sender) = inner.sender.lock() {
                    sender.take();
                }
                true
            }
        };
        if !should_join {
            return;
        }
        if let Ok(mut workers) = inner.workers.lock() {
            for worker in workers.drain(..) {
                if worker.thread().id() == thread::current().id() {
                    // 当前 worker 不能 join 自己；丢弃 JoinHandle 让它在
                    // 返回任务后自然结束，其他关闭者仍可观察 active drain。
                    continue;
                }
                let _ = worker.join();
            }
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
