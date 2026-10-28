//! VM 的单线程句柄包装与跨线程传递信封。
//!
//! 普通值必须实际经过 XCON 编解码；显式共享句柄独立保留来源 Runtime 身份。
//! 线程只携带 `VmTransferValue`，不携带普通 StrongHandle、VM 帧或 Rc。

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

use crate::concurrency::{SharedHandle, SharedOwnedGuard};
use crate::scheduler::{
    RuntimeCancellation, RuntimeExecError, RuntimeHandle, TaskHandle, TaskPool, ThreadHandle,
};
use crate::serialization::{
    XCON_MAX_DEPTH, XCON_MAX_FRAME_BYTES, XCON_MAX_NODES, XconValue, decode_xcon, encode_xcon,
};
use crate::{FatalError, RuntimeResult, RuntimeValue, TupleHandle, XiaoError, XiaoErrorKind};

/// XCON 载荷和 VM 并发句柄的运行时回归。
#[cfg(test)]
#[path = "vm_concurrency_tests.rs"]
mod tests;

/// 仅有字节和值语义共享句柄的跨线程载荷。
#[derive(Clone, Debug)]
pub enum VmTransferValue {
    /// 已编码且受大小限制的普通值。
    Xcon(Vec<u8>),
    /// 含显式共享句柄的元组。
    Tuple(Vec<Self>),
    /// 只读共享对象可跨 Runtime。
    Shared(SharedHandle<Self>),
    /// 共享可变对象只可在同一 Runtime 内传递。
    SharedMut(SharedHandle<Self>),
}

/// 子 VM 的完整错误出口；普通错误保留身份、cause 和 suppressed。
#[derive(Clone, Debug)]
pub enum VmExecutionError {
    /// 可恢复的语言错误。
    Error(XiaoError),
    /// 并发取消 027；父 VM 不得把它路由到用户 catch。
    ConcurrencyCancelled(XiaoError),
    /// 子 VM 致命错误。
    Fatal(FatalError),
    /// 外部 VM 取消，独立于并发 027。
    Cancelled,
}

/// 子 VM 返回的可发送结果。
pub type VmExecutionResult = Result<VmTransferValue, VmExecutionError>;

/// detached 子 VM 的结构化错误记录。
#[derive(Clone, Debug)]
pub struct VmDetachedError {
    /// 执行单元所属实际 Runtime。
    pub runtime_id: u64,
    /// 027 或 028；原始失败保存在 cause 中。
    pub error: XiaoError,
}

/// 追踪本次 VM 已交给 detached 回收器的完成回调。
type VmDetachedCompletion = (u64, mpsc::Receiver<()>);

/// 各执行单元共享的 detached 回收完成队列。
type VmDetachedCompletionQueue = Arc<Mutex<Vec<VmDetachedCompletion>>>;

/// VM 执行单元共享的 detached 回收完成追踪器。
#[derive(Clone, Default)]
pub struct VmDetachedCompletionTracker {
    pending: VmDetachedCompletionQueue,
}

impl VmDetachedCompletionTracker {
    fn track(&self, runtime_id: u64, completion: mpsc::Receiver<()>) {
        self.pending
            .lock()
            .expect("VM detached 完成队列锁未中毒")
            .push((runtime_id, completion));
    }

    /// 短暂收取给定 Runtime 已完成的 detached 回收回调，不等待长时间积压。
    #[must_use]
    pub fn wait_runtime_ids(&self, runtime_ids: &[u64]) -> Vec<u64> {
        let selected = {
            let Ok(mut pending) = self.pending.lock() else {
                return Vec::new();
            };
            let mut selected = Vec::new();
            let mut remaining = Vec::new();
            for item in std::mem::take(&mut *pending) {
                if runtime_ids.contains(&item.0) {
                    selected.push(item);
                } else {
                    remaining.push(item);
                }
            }
            *pending = remaining;
            selected
        };
        let deadline = Instant::now() + Duration::from_millis(25);
        let mut completed = Vec::new();
        let mut still_pending = Vec::new();
        for (runtime_id, completion) in selected {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                still_pending.push((runtime_id, completion));
                continue;
            }
            match completion.recv_timeout(remaining) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => completed.push(runtime_id),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    still_pending.push((runtime_id, completion));
                }
            }
        }
        if let Ok(mut pending) = self.pending.lock() {
            pending.extend(still_pending);
        }
        completed
    }
}

/// 进程级 VM detached 错误通道最多保留的报告数。
pub const VM_DETACHED_ERROR_CAPACITY: usize = 4096;

/// 因 detached 错误通道达到容量而未保存的报告数量。
static VM_DETACHED_ERROR_DROPS: AtomicUsize = AtomicUsize::new(0);

/// 进程级保存尚未由宿主读取的 detached 执行失败。
fn detached_reports() -> &'static Mutex<Vec<VmDetachedError>> {
    /// 子 VM detached 错误记录。
    static REPORTS: OnceLock<Mutex<Vec<VmDetachedError>>> = OnceLock::new();
    REPORTS.get_or_init(Default::default)
}

/// 取走子 VM 的 detached 失败，保持进程主退出码不变。
#[must_use]
pub fn take_vm_detached_errors() -> Vec<VmDetachedError> {
    std::mem::take(&mut *detached_reports().lock().expect("VM detached 通道锁未中毒"))
}

/// 取出指定 Runtime 所属的 detached 失败，供 VM 关闭时并入本次运行结果。
#[must_use]
pub fn take_vm_detached_errors_for(runtime_ids: &[u64]) -> Vec<VmDetachedError> {
    if runtime_ids.is_empty() {
        return Vec::new();
    }
    let Ok(mut reports) = detached_reports().lock() else {
        return Vec::new();
    };
    let mut selected = Vec::new();
    let mut remaining = Vec::with_capacity(reports.len());
    for report in std::mem::take(&mut *reports) {
        if runtime_ids.contains(&report.runtime_id) {
            selected.push(report);
        } else {
            remaining.push(report);
        }
    }
    *reports = remaining;
    selected
}

/// 取出 detached 错误通道的报告丢弃计数并清零。
#[must_use]
pub fn take_vm_detached_error_drops() -> usize {
    VM_DETACHED_ERROR_DROPS.swap(0, Ordering::AcqRel)
}

/// 把 detached 子执行失败规范化为 027/028 并保留 Runtime 身份。
fn report_detached(runtime_id: u64, result: Result<VmExecutionResult, RuntimeExecError>) {
    let error = match result {
        Ok(Ok(_)) => return,
        Ok(Err(VmExecutionError::Error(error))) if error.code() == "X06-RUNTIME-027" => error,
        Ok(Err(VmExecutionError::ConcurrencyCancelled(error))) => error,
        Ok(Err(VmExecutionError::Error(error))) => {
            execution_error(RuntimeExecError::JoinFailed).with_cause(error)
        }
        Ok(Err(VmExecutionError::Fatal(fatal))) => execution_error(RuntimeExecError::JoinFailed)
            .with_context(
                "fatal_code",
                crate::DiagnosticParam::Text(fatal.code().to_owned()),
            ),
        Ok(Err(VmExecutionError::Cancelled)) => execution_error(RuntimeExecError::Cancelled),
        Err(error) => execution_error(error),
    };
    let mut reports = detached_reports().lock().expect("VM detached 通道锁未中毒");
    if reports.len() < VM_DETACHED_ERROR_CAPACITY {
        reports.push(VmDetachedError { runtime_id, error });
    } else {
        VM_DETACHED_ERROR_DROPS.fetch_add(1, Ordering::AcqRel);
    }
}

/// 将 Runtime 错误转换为冻结的结构化错误身份。
#[must_use]
pub fn execution_error(error: RuntimeExecError) -> XiaoError {
    XiaoError::new(
        XiaoErrorKind::Other,
        error.code(),
        "runtime.concurrency",
        error.to_string(),
    )
}

/// 普通值跨边界编码失败使用 029。
#[must_use]
pub fn transfer_error(message: impl Into<String>) -> XiaoError {
    XiaoError::new(
        XiaoErrorKind::Other,
        "X06-RUNTIME-029",
        "runtime.concurrency_serialization",
        message,
    )
}

impl VmTransferValue {
    /// 编码一个合法边界值，并在递归分配前检查深度与节点预算。
    pub fn encode(value: &RuntimeValue, runtime_id: u64) -> RuntimeResult<Self> {
        let mut nodes = 0;
        let value = encode_value(value, runtime_id, 0, &mut nodes)?;
        if value.byte_len() > XCON_MAX_FRAME_BYTES {
            return Err(transfer_error("边界帧超过 64 MiB"));
        }
        Ok(value)
    }

    /// 将 XCON 字节解码到接收执行单元自己的普通句柄。
    pub fn decode(&self, runtime_id: u64) -> RuntimeResult<RuntimeValue> {
        match self {
            Self::Xcon(bytes) => {
                from_xcon(decode_xcon(bytes).map_err(|error| transfer_error(error.to_string()))?)
            }
            Self::Tuple(values) => Ok(RuntimeValue::Tuple(TupleHandle::new(
                values
                    .iter()
                    .map(|value| value.decode(runtime_id))
                    .collect::<RuntimeResult<Vec<_>>>()?,
            )?)),
            Self::Shared(handle) => Ok(RuntimeValue::Concurrency(VmConcurrencyValue::Shared(
                handle.clone(),
            ))),
            Self::SharedMut(handle) if handle.runtime_id() == runtime_id => Ok(
                RuntimeValue::Concurrency(VmConcurrencyValue::SharedMut(handle.clone())),
            ),
            Self::SharedMut(_) => Err(execution_error(RuntimeExecError::RuntimeClosed)),
        }
    }

    /// 返回拥有的序列化字节数，共享句柄只计包装大小。
    #[must_use]
    pub fn byte_len(&self) -> usize {
        match self {
            Self::Xcon(bytes) => bytes.len(),
            Self::Tuple(values) => values.iter().fold(16_usize, |size, value| {
                size.saturating_add(value.byte_len())
            }),
            Self::Shared(_) | Self::SharedMut(_) => 16,
        }
    }
}

/// 递归复制或编码候选 A 值，并在递归时共用节点预算。
fn encode_value(
    value: &RuntimeValue,
    runtime_id: u64,
    depth: usize,
    nodes: &mut usize,
) -> RuntimeResult<VmTransferValue> {
    *nodes += 1;
    if depth > XCON_MAX_DEPTH || *nodes > XCON_MAX_NODES {
        return Err(transfer_error("边界值超过深度/节点限制"));
    }
    let scalar = match value {
        RuntimeValue::None => XconValue::None,
        RuntimeValue::Int(value) => XconValue::Int(*value),
        RuntimeValue::Sint(value) => XconValue::Sint(*value),
        RuntimeValue::Lint(value) => XconValue::Lint(value.clone()),
        RuntimeValue::Float(value) => XconValue::Float(*value),
        RuntimeValue::Sfloat(value) => XconValue::Sfloat(*value),
        RuntimeValue::Lfloat(value) => XconValue::Lfloat(value.clone()),
        RuntimeValue::Bool(value) => XconValue::Bool(*value),
        RuntimeValue::Str(value) => XconValue::Str(value.to_string()?),
        RuntimeValue::Tuple(tuple) => {
            let values = tuple.with_elements(|values| {
                values
                    .iter()
                    .map(|value| encode_value(value, runtime_id, depth + 1, nodes))
                    .collect::<RuntimeResult<Vec<_>>>()
            })??;
            if values
                .iter()
                .all(|value| matches!(value, VmTransferValue::Xcon(_)))
            {
                let scalars = values
                    .iter()
                    .map(|value| match value {
                        VmTransferValue::Xcon(bytes) => {
                            decode_xcon(bytes).map_err(|error| transfer_error(error.to_string()))
                        }
                        _ => unreachable!(),
                    })
                    .collect::<RuntimeResult<Vec<_>>>()?;
                XconValue::Tuple(scalars)
            } else {
                return Ok(VmTransferValue::Tuple(values));
            }
        }
        RuntimeValue::Concurrency(VmConcurrencyValue::Shared(handle)) => {
            return Ok(VmTransferValue::Shared(handle.clone()));
        }
        RuntimeValue::Concurrency(VmConcurrencyValue::SharedMut(handle))
            if handle.runtime_id() == runtime_id =>
        {
            return Ok(VmTransferValue::SharedMut(handle.clone()));
        }
        RuntimeValue::Concurrency(VmConcurrencyValue::SharedMut(_)) => {
            return Err(execution_error(RuntimeExecError::RuntimeClosed));
        }
        _ => {
            return Err(transfer_error(format!(
                "{} 不可作为边界值",
                value.type_name()
            )));
        }
    };
    Ok(VmTransferValue::Xcon(
        encode_xcon(&scalar).map_err(|error| transfer_error(error.to_string()))?,
    ))
}

/// 把已验证的 XCON 值还原为接收 VM 自己拥有的 RuntimeValue。
fn from_xcon(value: XconValue) -> RuntimeResult<RuntimeValue> {
    Ok(match value {
        XconValue::None => RuntimeValue::None,
        XconValue::Int(value) => RuntimeValue::Int(value),
        XconValue::Sint(value) => RuntimeValue::Sint(value),
        XconValue::Lint(value) => RuntimeValue::Lint(value),
        XconValue::Float(value) => RuntimeValue::Float(value),
        XconValue::Sfloat(value) => RuntimeValue::Sfloat(value),
        XconValue::Lfloat(value) => RuntimeValue::Lfloat(value),
        XconValue::Bool(value) => RuntimeValue::Bool(value),
        XconValue::Str(value) => RuntimeValue::new_string(value)?,
        XconValue::Tuple(values) => RuntimeValue::Tuple(TupleHandle::new(
            values
                .into_iter()
                .map(from_xcon)
                .collect::<RuntimeResult<Vec<_>>>()?,
        )?),
        XconValue::Array(_) | XconValue::Dict(_) | XconValue::Set(_) => {
            return Err(transfer_error("XCON 可变容器不属于候选 A"));
        }
    })
}

/// 可被 VM 别名引用的单次消费线程槽。
pub struct VmThreadValue {
    handle: RefCell<Option<ThreadHandle<VmExecutionResult>>>,
    cancellation: RuntimeCancellation,
    runtime: RuntimeHandle,
    detached: Option<VmDetachedCompletionTracker>,
}

impl Drop for VmThreadValue {
    /// 未 join 的线程在最后一个 VM 别名释放时转入 detached 回收。
    fn drop(&mut self) {
        if let Some(handle) = self.handle.get_mut().take() {
            let id = self.runtime.id();
            let (completion_tx, completion_rx) = mpsc::channel();
            handle.detach_with(move |result| {
                report_detached(id, result);
                let _ = completion_tx.send(());
            });
            if let Some(tracker) = &self.detached {
                tracker.track(id, completion_rx);
            }
        }
    }
}

/// 可被 VM 别名引用的单次消费任务槽。
pub struct VmTaskValue {
    handle: RefCell<Option<TaskHandle<VmExecutionResult>>>,
    cancellation: RuntimeCancellation,
    runtime: RuntimeHandle,
    detached: Option<VmDetachedCompletionTracker>,
}

impl Drop for VmTaskValue {
    /// 未 wait 的任务在最后一个 VM 别名释放时转入 detached 回收。
    fn drop(&mut self) {
        if let Some(handle) = self.handle.get_mut().take() {
            let runtime_id = self.runtime.id();
            let (completion_tx, completion_rx) = mpsc::channel();
            handle.detach_with(move |result| {
                report_detached(runtime_id, result);
                let _ = completion_tx.send(());
            });
            if let Some(tracker) = &self.detached {
                tracker.track(runtime_id, completion_rx);
            }
        }
    }
}

/// VM 中的不透明并发句柄。线程局部消费槽和锁守卫不实现 Send。
#[derive(Clone)]
pub enum VmConcurrencyValue {
    /// 可复制 Runtime 别名。
    Runtime(RuntimeHandle),
    /// 单次消费线程。
    Thread(Rc<VmThreadValue>),
    /// 单次消费任务。
    Task(Rc<VmTaskValue>),
    /// 可复制任务池。
    Pool(TaskPool),
    /// 只读共享句柄。
    Shared(SharedHandle<VmTransferValue>),
    /// 可变共享句柄。
    SharedMut(SharedHandle<VmTransferValue>),
    /// 作用域独占锁，最后一个别名释放时解锁。
    LockGuard(Rc<RefCell<SharedOwnedGuard<VmTransferValue>>>),
}

impl fmt::Debug for VmConcurrencyValue {
    /// 只显示句柄种类，不暴露线程、池或共享对象内部状态。
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.type_name())
    }
}

impl PartialEq for VmConcurrencyValue {
    /// 以底层句柄身份比较并发值，不调用用户值比较。
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Runtime(left), Self::Runtime(right)) => left.id() == right.id(),
            (Self::Thread(left), Self::Thread(right)) => Rc::ptr_eq(left, right),
            (Self::Task(left), Self::Task(right)) => Rc::ptr_eq(left, right),
            (Self::Pool(left), Self::Pool(right)) => left.same_identity(right),
            (Self::Shared(left), Self::Shared(right))
            | (Self::SharedMut(left), Self::SharedMut(right)) => left.same_object(right),
            (Self::LockGuard(left), Self::LockGuard(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }
}

impl Eq for VmConcurrencyValue {}

impl VmConcurrencyValue {
    /// 返回稳定的句柄类型名。
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Runtime(_) => "runtime",
            Self::Pool(_) => "pool",
            Self::Thread(_) => "thread",
            Self::Task(_) => "task",
            Self::Shared(_) => "shared",
            Self::SharedMut(_) => "shared_mut",
            Self::LockGuard(_) => "lock_guard",
        }
    }

    /// 包装可消费线程，别名共享消费状态。
    pub fn thread(handle: ThreadHandle<VmExecutionResult>, runtime: RuntimeHandle) -> Self {
        Self::thread_tracked(handle, runtime, None)
    }

    /// 包装线程并追踪 detached 回收完成。
    pub fn thread_tracked(
        handle: ThreadHandle<VmExecutionResult>,
        runtime: RuntimeHandle,
        detached: Option<VmDetachedCompletionTracker>,
    ) -> Self {
        let cancellation = handle.cancellation();
        Self::Thread(Rc::new(VmThreadValue {
            handle: RefCell::new(Some(handle)),
            cancellation,
            runtime,
            detached,
        }))
    }

    /// 包装可消费任务，别名共享消费状态。
    pub fn task(handle: TaskHandle<VmExecutionResult>, runtime: RuntimeHandle) -> Self {
        Self::task_tracked(handle, runtime, None)
    }

    /// 包装任务并追踪 detached 回收完成。
    pub fn task_tracked(
        handle: TaskHandle<VmExecutionResult>,
        runtime: RuntimeHandle,
        detached: Option<VmDetachedCompletionTracker>,
    ) -> Self {
        let cancellation = handle.cancellation();
        Self::Task(Rc::new(VmTaskValue {
            handle: RefCell::new(Some(handle)),
            cancellation,
            runtime,
            detached,
        }))
    }

    /// 返回可变共享句柄或守卫所属实际 Runtime。
    pub fn mutable_runtime_id(&self) -> Option<u64> {
        match self {
            Self::SharedMut(handle) => Some(handle.runtime_id()),
            Self::LockGuard(guard) => Some(guard.borrow().runtime_id()),
            _ => None,
        }
    }

    /// 借用取消，不消费线程或任务槽。
    pub fn cancel(&self) -> Result<(), RuntimeExecError> {
        match self {
            Self::Thread(value) if value.handle.borrow().is_some() => value.cancellation.cancel(),
            Self::Task(value) if value.handle.borrow().is_some() => value.cancellation.cancel(),
            _ => return Err(RuntimeExecError::JoinFailed),
        }
        Ok(())
    }

    /// 消费线程并以可中断方式等待完整 VM 结果。
    pub fn join_thread(
        &self,
        interrupted: impl Fn() -> bool,
    ) -> Result<VmExecutionResult, RuntimeExecError> {
        let Self::Thread(value) = self else {
            return Err(RuntimeExecError::JoinFailed);
        };
        let handle = value
            .handle
            .borrow_mut()
            .take()
            .ok_or(RuntimeExecError::JoinFailed)?;
        let result = handle.join_interruptible(interrupted)?;
        Ok(cancel_result(result, &value.cancellation))
    }

    /// 消费任务并以可中断方式等待完整 VM 结果。
    pub fn wait_task(
        &self,
        interrupted: impl Fn() -> bool,
    ) -> Result<VmExecutionResult, RuntimeExecError> {
        let Self::Task(value) = self else {
            return Err(RuntimeExecError::JoinFailed);
        };
        let handle = value
            .handle
            .borrow_mut()
            .take()
            .ok_or(RuntimeExecError::JoinFailed)?;
        let _runtime = &value.runtime;
        let was_interrupted = interrupted();
        let result = handle.wait_interruptible(&interrupted)?;
        if was_interrupted || interrupted() {
            return Err(RuntimeExecError::Cancelled);
        }
        Ok(cancel_result(result, &value.cancellation))
    }

    /// 分离线程并保留原错误作为 028 的 cause。
    pub fn detach_thread(&self) -> Result<(), RuntimeExecError> {
        let Self::Thread(value) = self else {
            return Err(RuntimeExecError::JoinFailed);
        };
        let handle = value
            .handle
            .borrow_mut()
            .take()
            .ok_or(RuntimeExecError::JoinFailed)?;
        let runtime_id = value.runtime.id();
        let (completion_tx, completion_rx) = mpsc::channel();
        handle.detach_with(move |result| {
            report_detached(runtime_id, result);
            let _ = completion_tx.send(());
        });
        if let Some(tracker) = &value.detached {
            tracker.track(runtime_id, completion_rx);
        }
        Ok(())
    }
}

/// 把执行令牌取消映射成 027，同时保留子 VM 的既有失败。
fn cancel_result(
    result: VmExecutionResult,
    cancellation: &RuntimeCancellation,
) -> VmExecutionResult {
    if !cancellation.is_cancelled() {
        return result;
    }
    match result {
        Err(VmExecutionError::Error(error)) if error.code() == "X06-RUNTIME-027" => {
            Err(VmExecutionError::ConcurrencyCancelled(error))
        }
        Err(VmExecutionError::ConcurrencyCancelled(error)) => {
            Err(VmExecutionError::ConcurrencyCancelled(error))
        }
        Err(VmExecutionError::Fatal(fatal)) => Err(VmExecutionError::Fatal(fatal)),
        Err(VmExecutionError::Cancelled) => Err(VmExecutionError::Cancelled),
        other => {
            let mut error = execution_error(RuntimeExecError::Cancelled);
            if let Err(
                VmExecutionError::Error(cause) | VmExecutionError::ConcurrencyCancelled(cause),
            ) = other
            {
                error.push_suppressed(cause);
            }
            Err(VmExecutionError::ConcurrencyCancelled(error))
        }
    }
}
