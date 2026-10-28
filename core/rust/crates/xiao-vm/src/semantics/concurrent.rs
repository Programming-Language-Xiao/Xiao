//! 按 TAC 计划调用并发 Runtime；每个执行单元独立构造 VM 和普通值。

use super::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use xiao_bytecode::TacConcurrencyPlan;
use xiao_runtime::{
    RuntimeCancellation, RuntimeExecError, RuntimeHandle, SharedHandle, SharedTag, TaskPool,
    VmConcurrencyValue, VmExecutionError, VmExecutionResult, VmTransferValue, execution_error,
    transfer_error,
};

/// 子 VM 事件缓冲区的固定容量。
const CHILD_EVENT_CAPACITY: usize = 4096;

/// 暂存子执行单元产生的事件，并统计被容量限制丢弃的数量。
#[derive(Default)]
struct ChildEvents {
    pending: Vec<VmEvent>,
    detached: Vec<xiao_runtime::VmDetachedError>,
    dropped: usize,
}

/// 将子 VM 事件汇入父 VM 共享的有界缓冲区。
struct ChildSink(Arc<Mutex<ChildEvents>>);
impl VmEventSink for ChildSink {
    /// 缓冲事件；容量耗尽时只增加丢弃计数。
    fn record(&mut self, event: VmEvent) {
        let _ = record_child_event(&self.0, event);
    }

    fn record_detached_failure(
        &mut self,
        report: xiao_runtime::VmDetachedError,
    ) -> Result<(), xiao_runtime::VmDetachedError> {
        record_detached_child_error(&self.0, report)
    }
}

fn record_detached_child_error(
    events: &Arc<Mutex<ChildEvents>>,
    report: xiao_runtime::VmDetachedError,
) -> Result<(), xiao_runtime::VmDetachedError> {
    let mut events = events.lock().expect("子 VM 事件锁未中毒");
    if events.detached.len() >= CHILD_EVENT_CAPACITY {
        return Err(report);
    }
    events.detached.push(report);
    Ok(())
}

fn record_child_event(events: &Arc<Mutex<ChildEvents>>, event: VmEvent) -> bool {
    let mut events = events.lock().expect("子 VM 事件锁未中毒");
    if events.pending.len() >= CHILD_EVENT_CAPACITY {
        let replaceable = events
            .pending
            .iter()
            .position(|existing| !is_priority_child_event(existing));
        if let Some(index) = replaceable {
            events.pending.remove(index);
            events.dropped = events.dropped.saturating_add(1);
        } else {
            events.dropped = events.dropped.saturating_add(1);
            return false;
        }
    }
    events.pending.push(event);
    true
}

fn is_priority_child_event(event: &VmEvent) -> bool {
    matches!(
        event,
        VmEvent::DetachedFailure { .. } | VmEvent::BoundaryCopyReleased { .. }
    )
}

/// 尚未交给子 VM 的边界副本；排队任务被取消删除时仍按 IR 计划释放并记录事件。
struct DeferredBoundaryCopies {
    boundary: Arc<Mutex<Option<Vec<VmTransferValue>>>>,
    plan: TacConcurrencyPlan,
    events: Arc<Mutex<ChildEvents>>,
}

impl DeferredBoundaryCopies {
    fn take(&self) -> Option<Vec<VmTransferValue>> {
        self.boundary
            .lock()
            .ok()
            .and_then(|mut boundary| boundary.take())
    }
}

impl Drop for DeferredBoundaryCopies {
    fn drop(&mut self) {
        // 函数体运行前即发生提交失败或排队取消时，闭包负责回滚；正常启动时
        // `take()` 已把副本交给子 VM，这里不会重复消费。
        let copies = self
            .boundary
            .lock()
            .ok()
            .and_then(|mut boundary| boundary.take());
        let Some(copies) = copies else {
            return;
        };
        release_arguments(&self.plan, copies, |value| {
            let _ = record_child_event(&self.events, VmEvent::BoundaryCopyReleased { value });
        });
    }
}

/// 只在线程内部共享的执行上下文；可发送部分仅为事件队列和取消令牌。
#[derive(Default)]
pub(super) struct ConcurrentContext {
    runtime: RefCell<Option<RuntimeHandle>>,
    owned: RefCell<Vec<(RuntimeHandle, RuntimeHandle)>>,
    cancellation: RuntimeCancellation,
    cancellation_handled: Cell<bool>,
    children: RefCell<Vec<RuntimeCancellation>>,
    events: Arc<Mutex<ChildEvents>>,
    detached: xiao_runtime::VmDetachedCompletionTracker,
    /// 本次调用观测的实际目标 Runtime，不使用静态标签代替。
    observed_runtime: Cell<u64>,
}

impl ConcurrentContext {
    /// 返回当前 VM 绑定的 Runtime，缺省时延迟创建。
    pub(super) fn runtime(&self) -> RuntimeHandle {
        self.runtime
            .borrow_mut()
            .get_or_insert_with(RuntimeHandle::create)
            .clone()
    }
}

/// 将 Runtime 调度错误转换为 VM 可恢复错误。
pub(super) fn runtime_fault(error: RuntimeExecError) -> Fault {
    let fault = execution_error(error.clone());
    if matches!(error, RuntimeExecError::Cancelled) {
        Fault::ConcurrencyCancelled(fault)
    } else {
        Fault::Error(fault)
    }
}

/// 将无效并发计划或句柄形状标记为损坏产物。
fn artifact(message: &str) -> Fault {
    Fault::Fatal(FatalError::corrupt_artifact(message))
}

impl<C: Carrier, S: VmEventSink> Vm<'_, C, S> {
    /// 在 IR 标记的并发安全点检查执行单元取消。
    pub(super) fn concurrent_checkpoint(&mut self) -> Result<(), Fault> {
        if !self.concurrency.cancellation_handled.get()
            && self.concurrency.cancellation.is_cancelled()
        {
            self.concurrency.cancellation_handled.set(true);
            return Err(runtime_fault(RuntimeExecError::Cancelled));
        }
        Ok(())
    }

    /// 合并宿主取消、截止时间和 Runtime 取消令牌状态。
    fn concurrent_interrupted(&self) -> bool {
        self.cancellation
            .as_ref()
            .is_some_and(CancellationSource::is_cancelled)
            || !self.concurrency.cancellation_handled.get()
                && self.concurrency.cancellation.is_cancelled()
    }

    /// 保持外部 VM 取消与并发 027 的错误通道不同。
    fn interruption_fault(&self, error: RuntimeExecError) -> Fault {
        if self
            .cancellation
            .as_ref()
            .is_some_and(CancellationSource::is_cancelled)
        {
            Fault::Cancelled
        } else {
            if error == RuntimeExecError::Cancelled && self.concurrency.cancellation.is_cancelled()
            {
                self.concurrency.cancellation_handled.set(true);
            }
            runtime_fault(error)
        }
    }

    /// 将子 VM 暂存事件排入当前 VM 的事件接收器。
    fn flush_child_events(&mut self) {
        let (pending, detached, dropped) = {
            let mut events = self.concurrency.events.lock().expect("子 VM 事件锁未中毒");
            (
                std::mem::take(&mut events.pending),
                std::mem::take(&mut events.detached),
                std::mem::take(&mut events.dropped),
            )
        };
        self.sink.record_dropped_events(dropped);
        for event in pending {
            self.sink.record(event);
        }
        for detached in detached {
            if let Err(detached) = self.sink.record_detached_failure(detached) {
                let _ = xiao_runtime::restore_vm_detached_error(detached);
            }
        }
    }

    /// 执行入口结束清理、关闭本 VM 所属 Runtime 并刷新子事件。
    pub(super) fn finish_concurrency(&mut self, outcome: &Result<Option<RuntimeValue>, Fault>) {
        if outcome.is_err() {
            for token in self.concurrency.children.borrow().iter() {
                token.cancel();
            }
        }
        // 只收束本 VM 创建/提交的 Runtime；这不建立递归 owner 权限。
        let runtimes = self.concurrency.owned.borrow().clone();
        for (runtime, owner) in runtimes.iter().rev() {
            let _ = owner.in_context(|| runtime.try_close());
        }
        let mut runtime_ids = runtimes
            .iter()
            .map(|(runtime, _)| runtime.id())
            .collect::<Vec<_>>();
        if let Some(runtime) = self.concurrency.runtime.borrow().clone() {
            let _ = runtime.try_close();
            runtime_ids.push(runtime.id());
        }
        runtime_ids.extend(self.concurrency.detached.wait_runtime_ids(&runtime_ids));
        for detached in xiao_runtime::take_vm_detached_errors_for(&runtime_ids) {
            if let Err(detached) = self.sink.record_detached_failure(detached) {
                let _ = xiao_runtime::restore_vm_detached_error(detached);
            }
        }
        self.flush_child_events();
    }

    /// 将计划中的静态 Runtime 标签核对到实参携带的实际 Runtime。
    fn check_runtime_label(
        &self,
        plan: &TacConcurrencyPlan,
        actual: &RuntimeHandle,
    ) -> Result<(), Fault> {
        let Some(label) = plan.runtime_id else {
            return Ok(());
        };
        if actual.is_closed() {
            return Err(runtime_fault(RuntimeExecError::RuntimeClosed));
        }
        if label == u64::MAX {
            if self.concurrency.runtime().id() != actual.id() {
                return Err(runtime_fault(RuntimeExecError::RuntimeClosed));
            }
        } else {
            let function = self.frames.last().map_or(0, |frame| frame.function.get());
            let program = self.active_program.as_deref().unwrap_or(self.program);
            let value_id = u32::try_from(label - 1)
                .map_err(|_| artifact("静态 Runtime 标签不是有效值身份"))?;
            let register = program
                .functions
                .get(function as usize)
                .and_then(|function| function.value_registers.get(&value_id))
                .ok_or_else(|| artifact("静态 Runtime 标签未映射到当前帧值"))?;
            let actual_binding = match self.read(*register)? {
                RuntimeValue::Concurrency(VmConcurrencyValue::Runtime(runtime)) => runtime.id(),
                RuntimeValue::Concurrency(VmConcurrencyValue::Pool(pool)) => pool.runtime().id(),
                _ => return Err(artifact("静态 Runtime 标签引用了非 Runtime 值")),
            };
            if actual_binding != actual.id() {
                return Err(runtime_fault(RuntimeExecError::RuntimeClosed));
            }
        }
        for fact in &plan.facts {
            if let Some(source) = fact.source_runtime_id {
                if source == u64::MAX
                    && self.concurrency.runtime().id() != actual.id()
                    && plan
                        .argument_types
                        .iter()
                        .any(|ty| matches!(ty, xiao_ir::IrType::SharedMut { .. }))
                {
                    return Err(runtime_fault(RuntimeExecError::RuntimeClosed));
                }
            }
        }
        Ok(())
    }

    /// 原生绑定只消费计划中的 operation、边界动作和取消类别。
    pub(super) fn execute_concurrency(
        &mut self,
        plan: &TacConcurrencyPlan,
        args: &[BoundArgument],
    ) -> Result<RuntimeValue, Fault> {
        if args.len() != plan.argument_types.len() {
            return Err(artifact("并发实参数量与计划不符"));
        }
        for (argument, expected) in args.iter().zip(&plan.argument_types) {
            if !matches_type(&argument.value, expected) {
                return Err(artifact("并发实参类型与计划不符"));
            }
        }
        self.flush_child_events();
        if !plan.cancellation_checks.is_empty() && self.concurrent_interrupted() {
            return Err(self.interruption_fault(RuntimeExecError::Cancelled));
        }
        let current = self.concurrency.runtime();
        self.concurrency.observed_runtime.set(current.id());
        let result = match plan.operation.as_str() {
            "runtime_create" => {
                let runtime = RuntimeHandle::create();
                self.concurrency.observed_runtime.set(runtime.id());
                self.concurrency
                    .owned
                    .borrow_mut()
                    .push((runtime.clone(), current.clone()));
                RuntimeValue::Concurrency(VmConcurrencyValue::Runtime(runtime))
            }
            "current_runtime" => RuntimeValue::Concurrency(VmConcurrencyValue::Runtime(current)),
            "runtime_close" => {
                let runtime = runtime_arg(args, 0)?;
                self.concurrency.observed_runtime.set(runtime.id());
                runtime.try_close().map_err(runtime_fault)?;
                RuntimeValue::None
            }
            "pool_create" => {
                let RuntimeValue::Int(capacity) = args[1].value else {
                    return Err(artifact("pool 容量类型错误"));
                };
                let capacity = usize::try_from(capacity)
                    .map_err(|_| runtime_fault(RuntimeExecError::PoolArgument))?;
                let pool =
                    TaskPool::create(runtime_arg(args, 0)?, capacity).map_err(runtime_fault)?;
                RuntimeValue::Concurrency(VmConcurrencyValue::Pool(pool))
            }
            "shutdown" => {
                pool_arg(args, 0)?.shutdown().map_err(runtime_fault)?;
                RuntimeValue::None
            }
            "cancel" => {
                handle_arg(args, 0)?.cancel().map_err(runtime_fault)?;
                RuntimeValue::None
            }
            "detach" => {
                handle_arg(args, 0)?
                    .detach_thread()
                    .map_err(runtime_fault)?;
                RuntimeValue::None
            }
            "join" | "wait" => {
                let handle = handle_arg(args, 0)?;
                let result = if plan.operation == "join" {
                    handle.join_thread(|| self.concurrent_interrupted())
                } else {
                    handle.wait_task(|| self.concurrent_interrupted())
                };
                self.flush_child_events();
                match result.map_err(|error| self.interruption_fault(error))? {
                    Ok(value) => value.decode(current.id()).map_err(Fault::Error)?,
                    Err(VmExecutionError::Error(error)) => return Err(Fault::Error(error)),
                    Err(VmExecutionError::ConcurrencyCancelled(error)) => {
                        return Err(Fault::ConcurrencyCancelled(error));
                    }
                    Err(VmExecutionError::Fatal(fatal)) => return Err(Fault::Fatal(fatal)),
                    Err(VmExecutionError::Cancelled) => return Err(Fault::Cancelled),
                }
            }
            "spawn" | "submit" => {
                let pool = if plan.operation == "submit" {
                    Some(pool_arg(args, 0)?)
                } else {
                    None
                };
                let runtime = match &pool {
                    Some(pool) => pool.runtime(),
                    None => runtime_arg(args, 0)?,
                };
                self.concurrency.observed_runtime.set(runtime.id());
                self.check_runtime_label(plan, &runtime)?;
                let boundary = encode_arguments(plan, &args[2..], runtime.id(), |value| {
                    self.sink.record(VmEvent::BoundaryCopyReleased { value })
                })?;
                let boundary = Arc::new(Mutex::new(Some(boundary)));
                let (program, function, module, source_name) =
                    match self.entry_function(&args[1].value) {
                        Ok(entry) => entry,
                        Err(error) => {
                            if let Some(copies) = boundary.lock().expect("边界副本锁未中毒").take()
                            {
                                release_arguments(plan, copies, |value| {
                                    self.sink.record(VmEvent::BoundaryCopyReleased { value })
                                });
                            }
                            return Err(error);
                        }
                    };
                let loader = match self.snapshot_loader(&program, &module) {
                    Ok(loader) => loader,
                    Err(error) => {
                        if let Some(copies) = boundary.lock().expect("边界副本锁未中毒").take()
                        {
                            release_arguments(plan, copies, |value| {
                                self.sink.record(VmEvent::BoundaryCopyReleased { value })
                            });
                        }
                        return Err(error);
                    }
                };
                let options = self.options;
                let capabilities = self.capabilities;
                let external = self.cancellation.clone();
                let events = Arc::clone(&self.concurrency.events);
                let events_for_child = Arc::clone(&events);
                let return_runtime_id = current.id();
                let task_runtime = runtime.clone();
                let detached_tracker = self.concurrency.detached.clone();
                let boundary_cleanup = DeferredBoundaryCopies {
                    boundary: Arc::clone(&boundary),
                    plan: plan.clone(),
                    events,
                };
                let function_body = move |token: RuntimeCancellation| {
                    let boundary = boundary_cleanup.take().ok_or_else(|| {
                        VmExecutionError::Error(transfer_error("边界副本重复消费"))
                    })?;
                    execute_child(
                        program,
                        function,
                        module,
                        source_name,
                        loader,
                        boundary,
                        options,
                        capabilities,
                        external,
                        events_for_child,
                        task_runtime,
                        return_runtime_id,
                        token,
                        detached_tracker,
                    )
                };
                if let Some(pool) = pool {
                    let task = match pool.submit_cooperative(
                        RuntimeCancellation::default(),
                        || self.concurrent_interrupted(),
                        function_body,
                    ) {
                        Ok(task) => task,
                        Err(error) => {
                            if let Some(copies) = boundary.lock().expect("边界副本锁未中毒").take()
                            {
                                release_arguments(plan, copies, |value| {
                                    self.sink.record(VmEvent::BoundaryCopyReleased { value })
                                });
                            }
                            return Err(self.interruption_fault(error));
                        }
                    };
                    self.concurrency
                        .children
                        .borrow_mut()
                        .push(task.cancellation());
                    RuntimeValue::Concurrency(VmConcurrencyValue::task_tracked(
                        task,
                        runtime,
                        Some(self.concurrency.detached.clone()),
                    ))
                } else {
                    let thread =
                        match xiao_runtime::scheduler::spawn_cooperative(&runtime, function_body) {
                            Ok(thread) => thread,
                            Err(error) => {
                                if let Some(copies) =
                                    boundary.lock().expect("边界副本锁未中毒").take()
                                {
                                    release_arguments(plan, copies, |value| {
                                        self.sink.record(VmEvent::BoundaryCopyReleased { value })
                                    });
                                }
                                return Err(runtime_fault(error));
                            }
                        };
                    self.concurrency
                        .children
                        .borrow_mut()
                        .push(thread.cancellation());
                    RuntimeValue::Concurrency(VmConcurrencyValue::thread_tracked(
                        thread,
                        runtime,
                        Some(self.concurrency.detached.clone()),
                    ))
                }
            }
            "shared_read" | "shared_mut" => {
                if current.is_closed() {
                    return Err(runtime_fault(RuntimeExecError::RuntimeClosed));
                }
                let payload =
                    VmTransferValue::encode(&args[0].value, current.id()).map_err(Fault::Error)?;
                let tag = if plan.operation == "shared_read" {
                    SharedTag::Read
                } else {
                    SharedTag::Mut
                };
                let handle = SharedHandle::try_new(payload, current.id(), tag)
                    .map_err(|_| runtime_fault(RuntimeExecError::RuntimeClosed))?;
                RuntimeValue::Concurrency(if tag == SharedTag::Read {
                    VmConcurrencyValue::Shared(handle)
                } else {
                    VmConcurrencyValue::SharedMut(handle)
                })
            }
            "read" => {
                let VmConcurrencyValue::Shared(handle) = handle_arg(args, 0)? else {
                    return Err(artifact("read 需要 shared"));
                };
                handle
                    .get()
                    .map_err(|_| runtime_fault(RuntimeExecError::RuntimeClosed))?
                    .decode(current.id())
                    .map_err(Fault::Error)?
            }
            "lock" | "lock_read" | "lock_write" => {
                let handle = handle_arg(args, 0)?;
                if current.is_closed() || handle.mutable_runtime_id() != Some(current.id()) {
                    return Err(runtime_fault(RuntimeExecError::RuntimeClosed));
                }
                match plan.operation.as_str() {
                    "lock" => {
                        let VmConcurrencyValue::SharedMut(shared) = handle else {
                            return Err(artifact("lock 需要 shared_mut"));
                        };
                        let guard = shared
                            .lock_owned(&self.concurrency.cancellation, || {
                                self.cancellation
                                    .as_ref()
                                    .is_some_and(CancellationSource::is_cancelled)
                            })
                            .map_err(|_| self.interruption_fault(RuntimeExecError::Cancelled))?;
                        RuntimeValue::Concurrency(VmConcurrencyValue::LockGuard(Rc::new(
                            RefCell::new(guard),
                        )))
                    }
                    "lock_read" => {
                        let VmConcurrencyValue::LockGuard(guard) = handle else {
                            return Err(artifact("lock_read 需要守卫"));
                        };
                        guard.borrow().decode(current.id()).map_err(Fault::Error)?
                    }
                    _ => {
                        let VmConcurrencyValue::LockGuard(guard) = handle else {
                            return Err(artifact("lock_write 需要守卫"));
                        };
                        **guard.borrow_mut() =
                            VmTransferValue::encode(&args[1].value, current.id())
                                .map_err(Fault::Error)?;
                        RuntimeValue::None
                    }
                }
            }
            _ => return Err(artifact("未知并发计划操作")),
        };
        if !matches_type(&result, &plan.result_type) {
            return Err(artifact("并发结果类型与计划不符"));
        }
        let detached_queue_depth = xiao_runtime::detached_reaper_queue_depth();
        self.sink.record(VmEvent::ConcurrencyObserved {
            operation: plan.operation.clone(),
            runtime_id: self.concurrency.observed_runtime.get(),
            detached_queue_depth,
            detached_threshold_reached: detached_queue_depth
                >= xiao_runtime::scheduler::DETACHED_REAPER_WARNING_DEPTH,
        });
        Ok(result)
    }

    /// 从 VM 函数句柄解析子执行单元的 TAC 入口和来源信息。
    fn entry_function(
        &self,
        value: &RuntimeValue,
    ) -> Result<(Arc<TacProgram>, u32, String, String), Fault> {
        let RuntimeValue::ModuleFunction(module, index) = value else {
            return Err(artifact("并发入口不是函数"));
        };
        let (program, source_name) = if module.is_empty() || *module == self.current_module {
            (
                self.active_program.as_deref().unwrap_or(self.program),
                self.metadata.source_name.clone(),
            )
        } else {
            let instance = self
                .session
                .initialized_modules
                .get(module)
                .ok_or_else(|| artifact("函数所属模块尚未初始化"))?;
            (instance.program.as_ref(), instance.source_name.clone())
        };
        if *index as usize >= program.functions.len() || *index == 0 {
            return Err(artifact("并发入口函数编号无效"));
        }
        Ok((
            Arc::new(program.clone()),
            *index,
            module.clone(),
            source_name,
        ))
    }

    /// 为隔离子 VM 预加载只读模块快照，不共享父 VM 会话状态。
    fn snapshot_loader(&self, program: &TacProgram, module: &str) -> Result<FrozenLoader, Fault> {
        let mut loader = FrozenLoader::default();
        let mut pending = vec![(module.to_owned(), program.clone())];
        while let Some((owner, program)) = pending.pop() {
            for op in program
                .functions
                .iter()
                .flat_map(|f| &f.blocks)
                .flat_map(|b| &b.instructions)
                .map(|i| &i.op)
            {
                let TacOp::ImportModule { module, .. } = op else {
                    continue;
                };
                if native_module(module) {
                    continue;
                }
                let identity = if module == crate::STANDARD_LIBRARY_NAMESPACE
                    || module.starts_with(crate::STANDARD_LIBRARY_PREFIX)
                {
                    module.clone()
                } else if owner.starts_with("stdlib.") {
                    format!("stdlib.{module}")
                } else if let Some(package) = owner.strip_prefix("package:") {
                    format!(
                        "package:{}.{}",
                        package.split('.').next().unwrap_or(package),
                        module
                    )
                } else {
                    format!("project:{module}")
                };
                if loader.modules.contains_key(&identity) {
                    continue;
                }
                let compiled = self
                    .module_loader
                    .ok_or_else(|| "缺少模块加载器".to_owned())
                    .and_then(|loader| loader.load(&identity));
                if let Ok(Some(module)) = &compiled {
                    pending.push((identity.clone(), module.program.clone()));
                }
                loader.modules.insert(identity, compiled);
            }
        }
        Ok(loader)
    }
}

/// 编码参数严格遵守调用位置；失败时按计划逆序释放已构造副本。
fn encode_arguments(
    plan: &TacConcurrencyPlan,
    args: &[BoundArgument],
    target: u64,
    mut released: impl FnMut(u32),
) -> Result<Vec<VmTransferValue>, Fault> {
    let mut copies = Vec::with_capacity(args.len());
    let mut bytes = 0_usize;
    for argument in args {
        let copy = VmTransferValue::encode(&argument.value, target).and_then(|copy| {
            bytes = bytes.saturating_add(copy.byte_len());
            if bytes > xiao_runtime::XCON_MAX_FRAME_BYTES {
                Err(transfer_error("全部边界参数超过 64 MiB"))
            } else {
                Ok(copy)
            }
        });
        match copy {
            Ok(copy) => copies.push(copy),
            Err(error) => {
                if !args.is_empty() && plan.transfer.is_none() {
                    return Err(artifact("边界清理计划缺失"));
                }
                release_arguments(plan, copies, &mut released);
                return Err(Fault::Error(error));
            }
        }
    }
    Ok(copies)
}

/// 按 IR cleanup_order 逆序释放已经成功复制的边界参数。
fn release_arguments(
    plan: &TacConcurrencyPlan,
    copies: Vec<VmTransferValue>,
    mut released: impl FnMut(u32),
) {
    let Some(transfer) = &plan.transfer else {
        return;
    };
    let completed = copies.len();
    let mut copies = copies.into_iter().map(Some).collect::<Vec<_>>();
    for value in &transfer.cleanup_order {
        if let Some(index) = transfer
            .parameters
            .iter()
            .position(|candidate| candidate == value)
            .filter(|index| *index < completed)
        {
            drop(copies[index].take());
            released(*value);
        }
    }
}

/// 读取并发句柄实参，非句柄形状视为产物不一致。
fn handle_arg(args: &[BoundArgument], index: usize) -> Result<&VmConcurrencyValue, Fault> {
    match args.get(index).map(|arg| &arg.value) {
        Some(RuntimeValue::Concurrency(handle)) => Ok(handle),
        _ => Err(artifact("并发句柄实参无效")),
    }
}
/// 读取 Runtime 句柄实参并克隆共享别名。
fn runtime_arg(args: &[BoundArgument], index: usize) -> Result<RuntimeHandle, Fault> {
    match handle_arg(args, index)? {
        VmConcurrencyValue::Runtime(runtime) => Ok(runtime.clone()),
        _ => Err(artifact("Runtime 实参无效")),
    }
}
/// 读取任务池句柄实参并克隆共享别名。
fn pool_arg(args: &[BoundArgument], index: usize) -> Result<TaskPool, Fault> {
    match handle_arg(args, index)? {
        VmConcurrencyValue::Pool(pool) => Ok(pool.clone()),
        _ => Err(artifact("Pool 实参无效")),
    }
}

/// 检查运行时并发值是否符合 verifier 固定的 IR 类型。
fn matches_type(value: &RuntimeValue, expected: &xiao_ir::IrType) -> bool {
    use xiao_ir::IrType;
    match expected {
        IrType::Scalar { name } => value.type_name() == *name,
        IrType::None => matches!(value, RuntimeValue::None),
        IrType::Function { .. } => matches!(value, RuntimeValue::ModuleFunction(_, _)),
        IrType::Runtime => value.type_name() == "runtime",
        IrType::Pool => value.type_name() == "pool",
        IrType::Thread { .. } => value.type_name() == "thread",
        IrType::Task { .. } => value.type_name() == "task",
        IrType::Shared { .. } => value.type_name() == "shared",
        IrType::SharedMut { .. } => value.type_name() == "shared_mut",
        IrType::LockGuard { .. } => value.type_name() == "lock_guard",
        IrType::Tuple { elements } => match value {
            RuntimeValue::Tuple(tuple) => tuple
                .with_elements(|values| {
                    values.len() == elements.len()
                        && values.iter().zip(elements).all(|(v, t)| matches_type(v, t))
                })
                .unwrap_or(false),
            _ => false,
        },
        _ => false,
    }
}

/// 判断模块是否由 VM 直接绑定而无需模块载入器。
fn native_module(module: &str) -> bool {
    matches!(module, "std.concurrent" | "stdlib.concurrent")
}

/// 只提供编译阶段已验证模块的子 VM 载入器。
#[derive(Debug, Default)]
struct FrozenLoader {
    modules: BTreeMap<String, Result<Option<crate::ExecutableModule>, String>>,
}

impl ModuleLoader for FrozenLoader {
    /// 查询模块是否已进入冻结快照。
    fn contains(&self, identity: &str) -> bool {
        self.modules.contains_key(identity)
    }
    /// 禁止子 VM 在执行阶段重新编译模块。
    fn compile(&self, _: &str) -> Result<Option<crate::CompiledModule>, String> {
        Err("隔离 VM 只加载已验证 TAC".to_owned())
    }
    /// 从快照克隆可执行模块，不跨 VM 共享可变状态。
    fn load(&self, identity: &str) -> Result<Option<crate::ExecutableModule>, String> {
        match self
            .modules
            .get(identity)
            .ok_or("子 VM 模块不在只读快照中")?
        {
            Ok(Some(module)) => Ok(Some(crate::ExecutableModule {
                program: module.program.clone(),
                source_name: module.source_name.clone(),
                instance_tables: module.instance_tables.clone(),
            })),
            Ok(None) => Ok(None),
            Err(error) => Err(error.clone()),
        }
    }
    /// 冻结载入器不向子 VM 暴露宿主环境信息。
    fn environment(&self, _: &str) -> Option<&str> {
        None
    }
}

#[allow(clippy::too_many_arguments)]
/// 在独立 VM、模块快照和 Runtime 上执行一个任务入口。
fn execute_child(
    program: Arc<TacProgram>,
    function: u32,
    module: String,
    source_name: String,
    loader: FrozenLoader,
    arguments: Vec<VmTransferValue>,
    options: VmOptions,
    capabilities: u64,
    external: Option<CancellationSource>,
    events: Arc<Mutex<ChildEvents>>,
    runtime: RuntimeHandle,
    return_runtime_id: u64,
    cancellation: RuntimeCancellation,
    detached_tracker: xiao_runtime::VmDetachedCompletionTracker,
) -> VmExecutionResult {
    let mut vm = Vm::<crate::StackCarrier, ChildSink>::new_with_metadata(
        &program,
        options,
        ChildSink(Arc::clone(&events)),
        VmMetadata::new(module.clone(), source_name, None),
        VmSession::new(),
        capabilities,
    );
    vm.concurrency = Rc::new(ConcurrentContext {
        runtime: RefCell::new(Some(runtime.clone())),
        cancellation,
        events,
        detached: detached_tracker.clone(),
        ..Default::default()
    });
    vm.current_module = module;
    vm.cancellation = external;
    vm.module_loader = Some(&loader);
    let bound = arguments
        .iter()
        .map(|value| {
            value.decode(runtime.id()).map(|value| BoundArgument {
                keyword: None,
                value,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(VmExecutionError::Error)?;
    let outcome = vm.execute(FuncId::new(function), &bound, None);
    // 子执行单元自身的 Runtime 由父方管理；这里只关闭子 VM 显式创建的 Runtime。
    if outcome.is_err() {
        for token in vm.concurrency.children.borrow().iter() {
            token.cancel();
        }
    }
    for (owned, owner) in vm.concurrency.owned.borrow().iter().rev() {
        let _ = owner.in_context(|| owned.try_close());
    }
    let owned_runtime_ids = vm
        .concurrency
        .owned
        .borrow()
        .iter()
        .map(|(runtime, _)| runtime.id())
        .collect::<Vec<_>>();
    let mut report_runtime_ids = owned_runtime_ids.clone();
    report_runtime_ids.extend(detached_tracker.wait_runtime_ids(&owned_runtime_ids));
    for detached in xiao_runtime::take_vm_detached_errors_for(&report_runtime_ids) {
        if let Err(detached) = record_detached_child_error(&vm.concurrency.events, detached) {
            let _ = xiao_runtime::restore_vm_detached_error(detached);
        }
    }
    match outcome {
        Ok(value) => {
            VmTransferValue::encode(&value.unwrap_or(RuntimeValue::None), return_runtime_id)
                .map_err(VmExecutionError::Error)
        }
        Err(Fault::Error(error)) => Err(VmExecutionError::Error(error)),
        Err(Fault::ConcurrencyCancelled(error)) => {
            Err(VmExecutionError::ConcurrencyCancelled(error))
        }
        Err(Fault::Fatal(fatal)) => Err(VmExecutionError::Fatal(fatal)),
        Err(Fault::Cancelled) => Err(VmExecutionError::Cancelled),
    }
}

#[cfg(test)]
/// 验证 VM 的 Runtime owner 规则和跨 Runtime 拒绝。
mod tests {
    use super::*;
    use crate::BoundedSink;

    #[test]
    /// 验证 owner 只能操作直接子 Runtime，祖 Runtime 权限不传递。
    fn vm_close_reports_non_owner_and_direct_ownership_is_not_transitive() {
        let a = RuntimeHandle::create();
        let b = a.in_context(RuntimeHandle::create).unwrap();
        let c = b.in_context(RuntimeHandle::create).unwrap();
        let ir = xiao_ir::IrProgram::new(
            xiao_ir::IrEntryMode::Script,
            Vec::new(),
            xiao_ir::IrSpan::new(0, 0),
        );
        let program = xiao_bytecode::lower_program(&ir);
        let mut vm = Vm::<crate::StackCarrier, crate::RecordingSink>::new(
            &program,
            VmOptions::default(),
            crate::RecordingSink::new(),
        );
        vm.concurrency.runtime.replace(Some(a.clone()));
        let plan = TacConcurrencyPlan {
            operation: "runtime_close".to_owned(),
            span: xiao_ir::IrSpan::new(0, 1),
            argument_spans: vec![xiao_ir::IrSpan::new(0, 1)],
            argument_types: vec![xiao_ir::IrType::Runtime],
            result_type: xiao_ir::IrType::None,
            runtime_id: None,
            facts: Vec::new(),
            transfer: None,
            cancellation_checks: Vec::new(),
        };
        let arg = |runtime| {
            vec![BoundArgument {
                keyword: None,
                value: RuntimeValue::Concurrency(VmConcurrencyValue::Runtime(runtime)),
            }]
        };
        let error = a
            .in_context(|| vm.execute_concurrency(&plan, &arg(c.clone())))
            .unwrap()
            .unwrap_err();
        assert!(matches!(error, Fault::Error(error) if error.code() == "X06-RUNTIME-031"));
        a.in_context(|| c.close()).unwrap();
        assert!(!c.is_closed(), "旧 close 的 no-op 不得被 VM 当成关闭成功");
        a.in_context(|| vm.execute_concurrency(&plan, &arg(b.clone())))
            .unwrap()
            .unwrap();
        assert!(b.is_closed());
        assert!(!c.is_closed(), "父 Runtime 关闭不递归关闭孙 Runtime");
        c.try_close().unwrap();
        a.try_close().unwrap();
    }

    #[test]
    /// VM 执行结束后仍以创建它的 owner 上下文关闭嵌套 Runtime。
    fn vm_exit_closes_owned_runtime_after_restoring_owner_context() {
        let owner = RuntimeHandle::create();
        let child = owner.in_context(RuntimeHandle::create).unwrap();
        let ir = xiao_ir::IrProgram::new(
            xiao_ir::IrEntryMode::Script,
            Vec::new(),
            xiao_ir::IrSpan::new(0, 0),
        );
        let program = xiao_bytecode::lower_program(&ir);
        let mut vm = Vm::<crate::StackCarrier, crate::RecordingSink>::new(
            &program,
            VmOptions::default(),
            crate::RecordingSink::new(),
        );
        vm.concurrency
            .owned
            .borrow_mut()
            .push((child.clone(), owner.clone()));

        let outcome: Result<Option<RuntimeValue>, Fault> = Ok(None);
        vm.finish_concurrency(&outcome);

        assert!(
            child.is_closed(),
            "VM 退出应在 recorded owner 上下文中关闭子 Runtime"
        );
        owner.try_close().unwrap();
    }

    #[test]
    /// 排队任务被取消移除时仍按 TAC cleanup_order 记录边界副本释放。
    fn dropped_queued_boundary_copies_follow_cleanup_order() {
        let events = Arc::new(Mutex::new(ChildEvents::default()));
        let boundary = Arc::new(Mutex::new(Some(vec![
            VmTransferValue::Xcon(vec![1]),
            VmTransferValue::Xcon(vec![2]),
        ])));
        let deferred = DeferredBoundaryCopies {
            boundary: Arc::clone(&boundary),
            plan: TacConcurrencyPlan {
                operation: "submit".to_owned(),
                span: xiao_ir::IrSpan::new(0, 1),
                argument_spans: Vec::new(),
                argument_types: Vec::new(),
                result_type: xiao_ir::IrType::None,
                runtime_id: None,
                facts: Vec::new(),
                transfer: Some(xiao_bytecode::TacBoundaryTransfer {
                    copy_kind: "deep_copy".to_owned(),
                    parameters: vec![10, 20],
                    cleanup_order: vec![20, 10],
                }),
                cancellation_checks: Vec::new(),
            },
            events: Arc::clone(&events),
        };

        drop(boundary);
        drop(deferred);

        let released = events
            .lock()
            .expect("子 VM 事件锁未中毒")
            .pending
            .iter()
            .filter_map(|event| match event {
                VmEvent::BoundaryCopyReleased { value } => Some(*value),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(released, vec![20, 10]);
    }

    #[test]
    /// 子 VM 有界事件缓冲区的丢弃计数合入生产运行指标。
    fn child_event_drops_are_reported_by_parent_sink() {
        let ir = xiao_ir::IrProgram::new(
            xiao_ir::IrEntryMode::Script,
            Vec::new(),
            xiao_ir::IrSpan::new(0, 0),
        );
        let program = xiao_bytecode::lower_program(&ir);
        let mut vm = Vm::<crate::StackCarrier, BoundedSink>::new(
            &program,
            VmOptions::default(),
            BoundedSink::new(8),
        );
        vm.concurrency.events.lock().unwrap().dropped = 3;

        vm.flush_child_events();

        assert_eq!(vm.sink.dropped_events(), 3);
    }

    #[test]
    /// 普通事件缓冲区满载时仍独立保留 detached 报告和边界清理事件。
    fn detached_child_event_is_preserved_when_buffer_is_full() {
        let events = Arc::new(Mutex::new(ChildEvents::default()));
        for index in 0..CHILD_EVENT_CAPACITY {
            let _ = record_child_event(
                &events,
                VmEvent::BoundaryCopyReleased {
                    value: index as u32,
                },
            );
        }
        let error = xiao_runtime::execution_error(RuntimeExecError::JoinFailed);
        record_detached_child_error(
            &events,
            xiao_runtime::VmDetachedError {
                runtime_id: 7,
                error,
            },
        )
        .expect("分离的 detached 队列必须保留报告");
        let events = events.lock().unwrap();
        assert_eq!(events.pending.len(), CHILD_EVENT_CAPACITY);
        assert_eq!(events.dropped, 0);
        assert!(
            events
                .pending
                .iter()
                .all(|event| matches!(event, VmEvent::BoundaryCopyReleased { .. }))
        );
        assert_eq!(events.detached.len(), 1);
        assert_eq!(events.detached[0].runtime_id, 7);
    }

    #[test]
    /// 最终有界接收器满载优先保留边界清理，并将 detached 报告退回通道。
    fn bounded_sink_does_not_replace_boundary_cleanup_with_detached_failure() {
        let mut sink = BoundedSink::new(1);
        sink.record(VmEvent::BoundaryCopyReleased { value: 99 });
        let error = xiao_runtime::execution_error(RuntimeExecError::JoinFailed);
        let detached = xiao_runtime::VmDetachedError {
            runtime_id: 7,
            error,
        };
        let Err(returned) = sink.record_detached_failure(detached) else {
            panic!("满载的有界 sink 应退回 detached 报告");
        };
        assert_eq!(returned.runtime_id, 7);
        assert_eq!(returned.error.code(), "X06-RUNTIME-028");
        assert_eq!(
            sink.events(),
            &[VmEvent::BoundaryCopyReleased { value: 99 }]
        );
    }

    #[test]
    /// 子事件优先队列满时 detached 报告退回 Runtime 通道，宿主仍可读取。
    fn detached_child_event_overflow_returns_report_to_runtime_channel() {
        let events = Arc::new(Mutex::new(ChildEvents::default()));
        let runtime = RuntimeHandle::create();
        let runtime_id = runtime.id();
        for _ in 0..CHILD_EVENT_CAPACITY {
            record_detached_child_error(
                &events,
                xiao_runtime::VmDetachedError {
                    runtime_id,
                    error: xiao_runtime::execution_error(RuntimeExecError::JoinFailed),
                },
            )
            .expect("容量内报告应保留");
        }
        let overflow = xiao_runtime::VmDetachedError {
            runtime_id,
            error: xiao_runtime::execution_error(RuntimeExecError::Cancelled),
        };
        let Err(overflow) = record_detached_child_error(&events, overflow) else {
            panic!("超出子事件容量的报告应交还 Runtime 通道");
        };
        assert_eq!(events.lock().unwrap().dropped, 0);
        assert!(xiao_runtime::restore_vm_detached_error(overflow));
        let restored = xiao_runtime::take_vm_detached_errors_for(&[runtime_id]);
        assert!(
            restored
                .iter()
                .any(|report| { report.error.code() == RuntimeExecError::Cancelled.code() })
        );
        runtime.close();
    }
}
