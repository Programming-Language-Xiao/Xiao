//! 22C 最小 Runtime/线程/任务池执行骨架。
//!
//! 该模块只提供 Runtime 层的生命周期和取消边界；Xiao 标准库函数体及 VM opcode
//! lowering 仍由后续 22C 子批接入。句柄不复用普通单线程 `StrongHandle`。

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

use crate::concurrency::global_runtime_registry;
use crate::{
    CONCURRENCY_CANCELLED_CODE, CONCURRENCY_POOL_ARGUMENT_CODE, CONCURRENCY_POOL_CLOSED_CODE,
    CONCURRENCY_RUNTIME_CLOSED_CODE,
};

/// 可复制的 Runtime 句柄。
#[derive(Clone, Debug)]
pub struct RuntimeHandle {
    id: u64,
    closed: Arc<AtomicBool>,
}

impl RuntimeHandle {
    /// 创建一个新的进程内 Runtime。
    #[must_use]
    pub fn create() -> Self {
        static LABEL: AtomicU64 = AtomicU64::new(1);
        let label = LABEL.fetch_add(1, Ordering::Relaxed).max(1);
        let id = global_runtime_registry()
            .register(label)
            .expect("Runtime 静态标签应有效");
        Self {
            id,
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 返回实际 Runtime ID。
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// 判断 Runtime 是否已关闭。
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// 幂等关闭 Runtime；旧别名可观察相同关闭状态。
    pub fn close(&self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            global_runtime_registry().close(self.id);
        }
    }

    fn ensure_open(&self) -> Result<(), RuntimeExecError> {
        (!self.is_closed())
            .then_some(())
            .ok_or(RuntimeExecError::RuntimeClosed)
    }
}

/// 线程/任务取消令牌。
#[derive(Clone, Debug, Default)]
pub struct RuntimeCancellation {
    cancelled: Arc<AtomicBool>,
}

impl RuntimeCancellation {
    /// 设置取消标志。
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// 查询是否已取消。
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for RuntimeExecError {}

/// 一个可取消、可 join 的线程句柄。
pub struct ThreadHandle<T> {
    join: Option<JoinHandle<T>>,
    cancellation: RuntimeCancellation,
}

impl<T> ThreadHandle<T> {
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
            .map_err(|_| RuntimeExecError::JoinFailed)
    }
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
    runtime.ensure_open()?;
    let cancellation = RuntimeCancellation::default();
    let child_token = cancellation.clone();
    let join = thread::Builder::new()
        .name(format!("xiao-runtime-{}", runtime.id))
        .spawn(move || function(child_token))
        .map_err(|_| RuntimeExecError::JoinFailed)?;
    Ok(ThreadHandle {
        join: Some(join),
        cancellation,
    })
}

struct PoolMessage {
    run: Box<dyn FnOnce() + Send + 'static>,
}

struct PoolInner {
    sender: Mutex<Option<SyncSender<PoolMessage>>>,
    workers: Mutex<Vec<JoinHandle<()>>>,
    closed: AtomicBool,
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
        runtime.ensure_open()?;
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
        Ok(Self {
            runtime,
            inner: Arc::new(PoolInner {
                sender: Mutex::new(Some(sender)),
                workers: Mutex::new(workers),
                closed: AtomicBool::new(false),
            }),
        })
    }

    /// 提交一个显式接收取消令牌的任务。
    pub fn submit<T, F>(&self, function: F) -> Result<TaskHandle<T>, RuntimeExecError>
    where
        T: Send + 'static,
        F: FnOnce(RuntimeCancellation) -> T + Send + 'static,
    {
        self.runtime.ensure_open()?;
        if self.inner.closed.load(Ordering::Acquire) {
            return Err(RuntimeExecError::PoolClosed);
        }
        let cancellation = RuntimeCancellation::default();
        let child_token = cancellation.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let message = PoolMessage {
            run: Box::new(move || {
                let result = if child_token.is_cancelled() {
                    Err(RuntimeExecError::Cancelled)
                } else {
                    Ok(function(child_token.clone()))
                };
                let _ = sender.send(result);
            }),
        };
        self.inner
            .sender
            .lock()
            .ok()
            .and_then(|sender| sender.as_ref().cloned())
            .ok_or(RuntimeExecError::PoolClosed)?
            .send(message)
            .map_err(|_| RuntimeExecError::PoolClosed)?;
        Ok(TaskHandle {
            receiver,
            cancellation,
        })
    }

    /// 关闭任务池并等待 worker 退出。
    pub fn shutdown(&self) {
        if !self.inner.closed.swap(true, Ordering::AcqRel) {
            let _ = self.inner.sender.lock().map(|mut sender| sender.take());
            if let Ok(mut workers) = self.inner.workers.lock() {
                for worker in workers.drain(..) {
                    let _ = worker.join();
                }
            }
        }
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            self.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_spawn_cancel_and_join_are_wired() {
        let runtime = RuntimeHandle::create();
        let handle = spawn(&runtime, |token| token.is_cancelled()).expect("spawn");
        handle.cancel();
        assert!(handle.join().expect("join"));
        runtime.close();
        assert!(matches!(
            spawn(&runtime, |_| 1_u32),
            Err(error) if error.code() == CONCURRENCY_RUNTIME_CLOSED_CODE
        ));
    }

    #[test]
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
}
