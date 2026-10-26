//! 22C 最小 Runtime/线程/任务池执行骨架。
//!
//! 该模块只提供 Runtime 层的生命周期和取消边界；Xiao 标准库函数体及 VM opcode
//! lowering 仍由后续 22C 子批接入。句柄不复用普通单线程 `StrongHandle`。

use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak, mpsc};
use std::thread::{self, JoinHandle};

pub use crate::concurrency::LockCancellation as RuntimeCancellation;
use crate::concurrency::global_runtime_registry;
use crate::{
    CONCURRENCY_CANCELLED_CODE, CONCURRENCY_POOL_ARGUMENT_CODE, CONCURRENCY_POOL_CLOSED_CODE,
    CONCURRENCY_RUNTIME_CLOSED_CODE,
};

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
}

struct ActivityLease {
    runtime: Arc<RuntimeInner>,
}

impl Drop for ActivityLease {
    fn drop(&mut self) {
        if let Ok(mut active) = self.runtime.active.lock() {
            *active = active.saturating_sub(1);
            if *active == 0 {
                self.runtime.drained.notify_all();
            }
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
        let Ok(_close_gate) = self.inner.close_gate.lock() else {
            return;
        };
        if !self.inner.closed.swap(true, Ordering::AcqRel) {
            global_runtime_registry().close(self.id);
        }
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
        for pool in pools {
            TaskPool::shutdown_inner(&pool);
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

fn detached_reaper() -> &'static mpsc::Sender<Box<dyn FnOnce() + Send>> {
    static REAPER: OnceLock<mpsc::Sender<Box<dyn FnOnce() + Send>>> = OnceLock::new();
    REAPER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<Box<dyn FnOnce() + Send>>();
        thread::Builder::new()
            .name("xiao-detached-reaper".to_owned())
            .spawn(move || {
                while let Ok(reap) = receiver.recv() {
                    reap();
                }
            })
            .expect("启动 detached 错误收集线程失败");
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
            let _activity = activity;
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
        let activity = self.runtime.begin_activity()?;
        let cancellation = RuntimeCancellation::default();
        let child_token = cancellation.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let message = PoolMessage {
            run: Box::new(move || {
                let _activity = activity;
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
        sender
            .send(message)
            .map_err(|_| RuntimeExecError::PoolClosed)?;
        Ok(TaskHandle {
            receiver,
            cancellation,
        })
    }

    /// 关闭任务池并等待 worker 退出。
    pub fn shutdown(&self) {
        Self::shutdown_inner(&self.inner);
    }

    fn shutdown_inner(inner: &Arc<PoolInner>) {
        let Ok(_shutdown_gate) = inner.shutdown_gate.lock() else {
            return;
        };
        if inner.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Ok(mut sender) = inner.sender.lock() {
            sender.take();
        }
        if let Ok(mut workers) = inner.workers.lock() {
            for worker in workers.drain(..) {
                let _ = worker.join();
            }
        }
    }
}

impl Drop for TaskPool {
    /// 最后一个池别名释放时执行关闭。
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            self.shutdown();
        }
    }
}

#[cfg(test)]
/// Runtime/线程/任务池基础回归。
mod tests {
    use super::*;

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
        pool.shutdown();
        assert!(matches!(
            pool.submit(|_| 1_u32),
            Err(error) if error.code() == CONCURRENCY_POOL_CLOSED_CODE
        ));
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
        pool.shutdown();
    }

    #[test]
    /// detached 线程 panic 进入进程级 028 通道。
    fn detached_thread_panic_is_reported_to_process_channel() {
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
        assert_eq!(errors[0].code(), "X06-RUNTIME-028");
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
