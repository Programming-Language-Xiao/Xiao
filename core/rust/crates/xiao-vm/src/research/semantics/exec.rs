//! 机型无关的三地址语义核。
//!
//! 本模块只认三地址指令与 [`Carrier`]，决定算什么、错怎么抛、清理怎么走、
//! 调用怎么进；它不认识栈、寄存器或帧槽，也不认识任何机型的细节。
//!
//! 释放顺序的唯一来源是冻结计划：退出点上执行 `(作用域, 退出边)` 计划，按
//! `order` 逐条释放。动态错误发生时按同一份计划展开当前作用域栈。

use xiao_bytecode::research::{
    BlockId, FuncId, PcMap, TacArgument, TacConstant, TacFunction, TacHandler, TacInstr, TacOp,
    TacProgram, VReg, build_pc_map,
};
use xiao_diagnostics::{
    BackendLocation, FatalError, NUMERIC_OVERFLOW_CODE, StackFrame, TYPE_MISMATCH_CODE, XiaoError,
};
use xiao_runtime::{CatchRoute, RuntimeDriver, RuntimeValue};

use crate::research::carrier::{Carrier, CarrierContext, MapPoint};
use crate::research::frame::Frame;
use crate::research::ops;
use crate::research::run::{RunResult, VmMetrics, VmOptions};
use crate::research::sink::{VmEvent, VmEventSink};

/// 解释过程中的终止原因。
///
/// 可恢复错误与致命故障是两条通道：致命故障不执行释放计划，也不能被普通
/// 处理器捕获，因此不能与 [`XiaoError`] 共用一个变体。
#[derive(Debug)]
pub enum Fault {
    /// 可恢复的运行时错误。
    Error(XiaoError),
    /// 不可恢复的故障。
    Fatal(FatalError),
}

/// 一条已经求值的实参。
#[derive(Clone, Debug)]
pub struct BoundArgument {
    /// 关键字实参的名称；位置实参为空。
    pub keyword: Option<String>,
    /// 实参值。
    pub value: RuntimeValue,
}

/// 一条指令执行后的控制流去向。
enum Flow {
    /// 顺序执行下一条指令。
    Next,
    /// 跳转到指定块。
    Jump(BlockId),
    /// 运行时检查失败跳转；失败类别和原始块要保留到失败块构造错误。
    JumpWithFault {
        /// 失败处理块。
        target: BlockId,
        /// 触发检查的类别。
        kind: String,
    },
    /// 返回，携带可选返回值寄存器。
    Return(Option<VReg>),
    /// 从 `finally` 子程序返回。
    RetFromSub,
}

/// 当前帧异常路由的结果。
///
/// `finally` 可以覆盖挂起的错误或控制退出，因此异常路由除了跳进
/// `catch`，还必须能把 `return`/`break`/`continue` 交回普通块循环。
enum RouteDecision {
    /// 跳进一个匹配的 `catch` 或嵌套处理器。
    Jump(BlockId),
    /// `finally` 产生了新的非局部控制流。
    Flow(Flow),
}

/// 一次待路由故障的来源信息。
#[derive(Clone, Debug)]
struct PendingFault {
    /// 对应冻结的退出边名称。
    exit: String,
    /// 发生故障的原始基本块。
    origin: BlockId,
}

/// 三地址解释器。
pub struct Vm<'p, C: Carrier, S: VmEventSink> {
    program: &'p TacProgram,
    frames: Vec<Frame<C>>,
    sink: S,
    metrics: VmMetrics,
    options: VmOptions,
    /// 运行开始前建立的一次性只读 pc 映射；热路径只做查表。
    pc_map: Option<PcMap>,
}

impl<'p, C: Carrier, S: VmEventSink> Vm<'p, C, S> {
    /// 创建一个解释器。
    pub fn new(program: &'p TacProgram, options: VmOptions, sink: S) -> Self {
        Self {
            program,
            frames: Vec::new(),
            sink,
            metrics: VmMetrics::default(),
            options,
            pc_map: build_pc_map(program, xiao_bytecode::research::OperandWidth::Leb128).ok(),
        }
    }

    /// 返回当前运行指标。
    #[must_use]
    pub const fn metrics(&self) -> VmMetrics {
        self.metrics
    }

    /// 取出事件接收器。
    #[must_use]
    pub fn into_sink(self) -> S {
        self.sink
    }

    /// 从脚本入口开始执行。
    pub fn run(&mut self) -> RunResult {
        let program = self.program;
        self.sink.record(VmEvent::ModuleLoaded {
            module: program.abi.target.clone(),
        });
        match self.execute(FuncId::new(0), &[], None) {
            Ok(_) => RunResult::Success,
            Err(Fault::Error(error)) => {
                self.sink.record(VmEvent::ErrorRaised {
                    code: error.code().to_owned(),
                    message_id: error.message_id().to_owned(),
                });
                RunResult::Error(error)
            }
            Err(Fault::Fatal(fatal)) => {
                self.sink.record(VmEvent::FatalRaised {
                    code: fatal.code().to_owned(),
                });
                RunResult::Fatal(fatal)
            }
        }
    }

    /// 执行一个函数直到返回。
    fn execute(
        &mut self,
        target: FuncId,
        arguments: &[BoundArgument],
        return_to: Option<VReg>,
    ) -> Result<Option<RuntimeValue>, Fault> {
        let program = self.program;
        let Some(function) = program.functions.get(target.get() as usize) else {
            return Err(Fault::Error(XiaoError::invalid_value(format!(
                "函数索引 {} 不存在",
                target.get()
            ))));
        };
        if self.frames.len() >= self.options.max_call_depth {
            let fatal = FatalError::stack_overflow(format!(
                "调用深度超过上限 {}",
                self.options.max_call_depth
            ));
            return Err(Fault::Fatal(fatal));
        }
        self.metrics.max_call_depth = self.metrics.max_call_depth.max(self.frames.len() + 1);
        let depth = self.frames.len() + 1;
        let carrier = C::empty(CarrierContext {
            program,
            function_id: target,
            function,
            categories: &function.categories,
            call_depth: depth,
        });
        let mut frame = Frame::new(target, function.name.clone(), carrier, return_to);
        bind_arguments(function, program, &mut frame, arguments);
        self.frames.push(frame);
        self.sink.record(VmEvent::FunctionEntered {
            function: function.name.clone(),
            depth,
        });
        self.sink.record(VmEvent::StackFrame {
            function: function.name.clone(),
            depth,
            return_to,
        });

        let outcome = match self.run_blocks(function) {
            Ok(value) => Ok(value),
            Err(fault) => {
                self.unwind(&fault);
                Err(fault)
            }
        };

        if let Some(frame) = self.frames.pop() {
            let carrier_metrics = frame.carrier.metrics();
            self.metrics.max_stack_depth = self
                .metrics
                .max_stack_depth
                .max(carrier_metrics.peak_occupancy);
            self.metrics.spill_count = self
                .metrics
                .spill_count
                .saturating_add(carrier_metrics.spill_count);
            self.metrics.stack_map_entries = self
                .metrics
                .stack_map_entries
                .saturating_add(carrier_metrics.stack_map_entries);
            self.metrics.call_save_count = self
                .metrics
                .call_save_count
                .saturating_add(carrier_metrics.call_save_count);
        }
        self.sink.record(VmEvent::FunctionReturned {
            function: function.name.clone(),
            depth,
        });
        outcome
    }

    /// 通知当前帧的载体：控制流经过了一个可能需要栈映射的程序点。
    ///
    /// 语义核只报事实，**由载体决定自己是否计数**——R1-F 冻结的机型分界
    /// （栈式数跳转目标、混合式数调用点与帧尾、寄存器式一个都不数）因此写在
    /// 各自的载体实现里，而不是在这里分支判断机型。
    fn note_map_point(&mut self, point: MapPoint) {
        if let Some(frame) = self.frames.last_mut() {
            frame.carrier.map_point(point);
        }
    }

    /// 为在当前指令处产生的故障追加统一后端调用帧。
    ///
    /// 通过启动时建立的只读 pc 表追加统一后端调用帧。
    ///
    /// 映射缺失时保留空的后端位置，并记录结构化事件；绝不把源码偏移伪装
    /// 成物理 pc。每次故障离开一层调用帧时才会再次调用本方法，因此同一帧
    /// 的连续清理不会悄悄丢掉调用方信息。
    fn annotate_fault(&mut self, fault: Fault, block: BlockId, instruction: usize) -> Fault {
        let Some(frame) = self.frames.last() else {
            return fault;
        };
        let function = frame.function;
        let function_name = frame.name.clone();
        let pc = self
            .pc_map
            .as_ref()
            .and_then(|map| map.pc_at(function, block, instruction));
        if pc.is_none() {
            self.sink.record(VmEvent::BackendLocationMissing {
                function: function_name.clone(),
                block: block.get(),
                instruction,
            });
        }
        let backend = pc.map_or_else(BackendLocation::empty, |pc| {
            BackendLocation::empty().with_bytecode_offset(pc as u64)
        });
        let stack_frame =
            StackFrame::user(self.program.abi.target.clone(), function_name).with_backend(backend);
        match fault {
            Fault::Error(error) => Fault::Error(error.with_stack_frame(stack_frame)),
            Fault::Fatal(fatal) => Fault::Fatal(fatal.with_stack_frame(stack_frame)),
        }
    }

    /// 执行一个函数的基本块序列。
    fn run_blocks(&mut self, function: &TacFunction) -> Result<Option<RuntimeValue>, Fault> {
        let mut block = function.entry;
        let mut pending_fault: Option<PendingFault> = None;
        loop {
            let Some(current) = function.blocks.get(block.get() as usize) else {
                return Ok(None);
            };
            let mut flow = Flow::Next;
            for (instruction_index, instruction) in current.instructions.iter().enumerate() {
                self.metrics.instructions = self.metrics.instructions.saturating_add(1);
                flow = match self.step(function, instruction) {
                    Ok(flow) => flow,
                    Err(fault) => {
                        let fault = self.annotate_fault(fault, block, instruction_index);
                        let pending = pending_fault.take();
                        let exit = pending
                            .as_ref()
                            .map(|item| item.exit.as_str())
                            .unwrap_or_else(|| exit_for_instruction(&instruction.op));
                        let origin = pending.as_ref().map_or(block, |item| item.origin);
                        match self.route_fault(function, origin, exit, fault) {
                            Ok(RouteDecision::Jump(handler)) => Flow::Jump(handler),
                            Ok(RouteDecision::Flow(flow)) => flow,
                            Err(fault) => return Err(fault),
                        }
                    }
                };
                if !matches!(flow, Flow::Next) {
                    break;
                }
            }
            match flow {
                Flow::Jump(next) => {
                    self.note_map_point(MapPoint::JumpTarget);
                    self.prune_handler_contexts(function, next);
                    self.prune_scopes_for_target(function, next);
                    block = next;
                }
                Flow::JumpWithFault { target, kind } => {
                    pending_fault = Some(PendingFault {
                        exit: "dynamic_check_failure".to_owned(),
                        origin: block,
                    });
                    if let Some(frame) = self.frames.last_mut() {
                        frame.pending_check_kind = Some(kind);
                    }
                    self.note_map_point(MapPoint::JumpTarget);
                    self.prune_handler_contexts(function, target);
                    block = target;
                }
                Flow::Return(register) => {
                    self.note_map_point(MapPoint::FrameEnd);
                    return match register {
                        Some(register) => self.read(register).map(Some),
                        None => Ok(None),
                    };
                }
                Flow::RetFromSub => {
                    return Err(Fault::Error(XiaoError::invalid_value(
                        "finally 子程序在调用帧外返回",
                    )));
                }
                Flow::Next => return Ok(None),
            }
        }
    }

    /// 执行一条指令。
    fn step(&mut self, function: &TacFunction, instruction: &TacInstr) -> Result<Flow, Fault> {
        match &instruction.op {
            TacOp::LoadConst(id) => {
                let Some(constant) = self.program.constants.get(*id) else {
                    return Err(Fault::Error(XiaoError::invalid_value("常量索引不存在")));
                };
                let value = constant_value(constant)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::LoadNone => self.write_operand(instruction.dst, RuntimeValue::None),
            TacOp::LoadFunc(_) | TacOp::Box(_) | TacOp::Unbox(_) => {
                return Err(Fault::Error(XiaoError::invalid_value(
                    "该指令形态尚未在解释器中实现",
                )));
            }
            TacOp::Move(source) => {
                let value = self.take(*source)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::Copy(source) => {
                // 只读读取会克隆堆句柄（多持一次引用），源寄存器保持有效。
                let value = self.read(*source)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::Cast { value, target } => {
                let value = self
                    .read(*value)?
                    .convert_to(*target)
                    .map_err(Fault::Error)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::Arith { op, left, right } => {
                let left = self.read(*left)?;
                let right = self.read(*right)?;
                let value = ops::apply_arith(*op, &left, &right).map_err(Fault::Error)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::Compare { op, left, right } => {
                let left = self.read(*left)?;
                let right = self.read(*right)?;
                let value = ops::apply_compare(*op, &left, &right).map_err(Fault::Error)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::NewArray { elements } => {
                let values = self.collect(elements)?;
                let value = ops::new_array(values).map_err(Fault::Error)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::NewTuple { elements } => {
                let values = self.collect(elements)?;
                let value = ops::new_tuple(values).map_err(Fault::Error)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::NewDictTable { entries } => {
                let values = self.collect_entries(entries)?;
                let value = ops::new_dict_table(values).map_err(Fault::Error)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::NewDictColumn { entries } => {
                let values = self.collect_entries(entries)?;
                let value = ops::new_dict_column(values).map_err(Fault::Error)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::NewSet { elements } => {
                let values = self.collect(elements)?;
                let value = ops::new_set(values).map_err(Fault::Error)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::IndexGet { source, path } => {
                let source = self.read(*source)?;
                let value = ops::index_get(&source, path).map_err(Fault::Error)?;
                self.write_operand(instruction.dst, value);
            }
            TacOp::Jump(target) => return Ok(Flow::Jump(*target)),
            TacOp::BranchIf {
                condition,
                if_true,
                if_false,
            } => {
                let value = self.read(*condition)?;
                let Some(flag) = value.as_bool() else {
                    return Err(Fault::Error(XiaoError::type_mismatch(
                        "bool",
                        value.type_name(),
                    )));
                };
                return Ok(Flow::Jump(if flag { *if_true } else { *if_false }));
            }
            TacOp::Call {
                callee, arguments, ..
            } => {
                let bound = self.bind(arguments)?;
                self.note_map_point(MapPoint::CallSite);
                if let Some(frame) = self.frames.last_mut() {
                    frame.carrier.begin_call();
                }
                let value = self.execute(*callee, &bound, instruction.dst);
                if let Some(frame) = self.frames.last_mut() {
                    frame.carrier.end_call();
                }
                let value = value?;
                if let Some(register) = instruction.dst {
                    self.write(register, value.unwrap_or(RuntimeValue::None));
                }
            }
            TacOp::CallDynamic { .. } => {
                return Err(Fault::Error(XiaoError::invalid_value(
                    "动态派发调用尚未在解释器中实现",
                )));
            }
            TacOp::Return { value } => return Ok(Flow::Return(*value)),
            TacOp::Raise { value } => {
                let value = self.read(*value)?;
                let RuntimeValue::Error(error) = value else {
                    return Err(Fault::Error(XiaoError::type_mismatch(
                        "error",
                        value.type_name(),
                    )));
                };
                return Err(Fault::Error(*error));
            }
            TacOp::MakeError {
                type_name,
                code,
                message,
            } => {
                let pending_check = self
                    .frames
                    .last_mut()
                    .and_then(|frame| frame.pending_check_kind.take());
                let code = code
                    .map(|register| self.read(register))
                    .transpose()?
                    .as_ref()
                    .map(runtime_text)
                    .transpose()?;
                let message = message
                    .map(|register| self.read(register))
                    .transpose()?
                    .as_ref()
                    .map(runtime_text)
                    .transpose()?;
                let default_code = pending_check
                    .as_deref()
                    .and_then(runtime_check_code)
                    .or(code.as_deref());
                let error = XiaoError::from_type_name(type_name, default_code, message.as_deref())
                    .ok_or_else(|| {
                        Fault::Error(XiaoError::invalid_value(format!(
                            "未知或不可恢复的错误类型 {type_name}"
                        )))
                    })?;
                self.write_operand(instruction.dst, RuntimeValue::error(error));
            }
            TacOp::CallSub { sub } => {
                let finally_handler = function
                    .handlers
                    .iter()
                    .find(|handler| handler.handler == *sub && handler.exit == "finally");
                let already_completed = finally_handler.is_some_and(|handler| {
                    self.frames.last().is_some_and(|frame| {
                        frame.completed_finally.contains(&(handler.scope, *sub))
                    })
                });
                if already_completed {
                    // 异常路径进入 catch 前已经执行过本层 finally；catch 体的
                    // return/break/continue 仍会经过这里，但不得重复清理。
                    return Ok(Flow::Next);
                }
                if let Some(handler) = finally_handler {
                    // 正常路径调用 finally 也属于一次处理器进入；异常路径
                    // 由 `route_fault` 记录同一事件，保持观测口径一致。
                    self.sink.record(VmEvent::HandlerEntered {
                        scope: handler.scope,
                        handler: handler.handler.get(),
                    });
                }
                let pending_exit = self
                    .frames
                    .last()
                    .and_then(|frame| frame.pending_exits.last())
                    .cloned()
                    .unwrap_or_else(|| "normal".to_owned());
                let sub_flow = self.run_subroutine(function, *sub, &pending_exit)?;
                if let Some(frame) = self.frames.last_mut() {
                    if let Some(handler) = finally_handler {
                        let marker = (handler.scope, *sub);
                        if !frame.completed_finally.contains(&marker) {
                            frame.completed_finally.push(marker);
                        }
                    }
                }
                if !matches!(sub_flow, Flow::Next) {
                    return Ok(sub_flow);
                }
            }
            TacOp::RetFromSub => {
                return Ok(Flow::RetFromSub);
            }
            TacOp::Check {
                kind,
                value,
                on_failure,
            } => {
                let value = self.read(*value)?;
                if !check_value(kind, &value) {
                    return Ok(Flow::JumpWithFault {
                        target: *on_failure,
                        kind: kind.clone(),
                    });
                }
            }
            TacOp::Release { value, .. } => {
                // 临时值可能已被 `Move` 搬进绑定，此时寄存器是空的；空释放是
                // 空操作，既不算一次释放也不报错。
                if self.take_if_present(*value).is_some() {
                    self.metrics.releases = self.metrics.releases.saturating_add(1);
                }
            }
            TacOp::Transfer { .. } => {}
            TacOp::RunReleasePlan { scope, exit } => {
                self.run_plan(function, *scope, exit);
            }
            TacOp::EnterScope(scope) => {
                if let Some(frame) = self.frames.last_mut() {
                    // 同一作用域重新进入（例如循环中的 try）开启新的 finally
                    // 动态轮次，旧轮次的记账不能影响本次执行。
                    frame.completed_finally.retain(|(owner, _)| owner != scope);
                    if !frame.scopes.contains(scope) {
                        frame.scopes.push(*scope);
                    }
                }
                self.sink.record(VmEvent::ScopeEntered { scope: *scope });
            }
            TacOp::ExitScope { scope, exit } => {
                if let Some(frame) = self.frames.last_mut()
                    && let Some(index) = frame.scopes.iter().rposition(|item| item == scope)
                {
                    frame.scopes.truncate(index);
                    // `catch` 与其所属 `try` 是兄弟作用域。入口块的首条
                    // `EnterScope` 指令携带 catch 作用域编号，正常离开时据此
                    // 清掉动态上下文；catch 体抛错时不会走到这里。
                    frame.active_catches.retain(|(_, catch_block)| {
                        function.block(*catch_block).and_then(|block| {
                            block.instructions.iter().find_map(|instruction| {
                                if let TacOp::EnterScope(scope) = instruction.op {
                                    Some(scope)
                                } else {
                                    None
                                }
                            })
                        }) != Some(*scope)
                    });
                }
                self.sink.record(VmEvent::ScopeExited {
                    scope: *scope,
                    exit: exit.clone(),
                });
            }
        }
        Ok(Flow::Next)
    }

    /// 在当前帧执行一个 `finally` 子程序，直到 `RetFromSub`。
    ///
    /// 子程序中的非局部退出会覆盖挂起类别并交回调用点；普通 `RetFromSub`
    /// 返回 `Flow::Next`，由调用点继续原来的路径。由降低器生成的跨作用域
    /// 跳转目标总是位于子程序入口之前，入口之后的目标属于子程序本身（包括
    /// finally 内部声明的循环和嵌套处理器）。
    fn run_subroutine(
        &mut self,
        function: &TacFunction,
        sub: BlockId,
        pending_exit: &str,
    ) -> Result<Flow, Fault> {
        if let Some(frame) = self.frames.last_mut() {
            frame.pending_exits.push(pending_exit.to_owned());
            frame.active_subroutines.push(sub);
        }
        let result = (|| {
            let mut block = sub;
            let mut pending_fault: Option<PendingFault> = None;
            loop {
                let Some(current) = function.blocks.get(block.get() as usize) else {
                    return Ok(Flow::Next);
                };
                let mut flow = Flow::Next;
                for (instruction_index, instruction) in current.instructions.iter().enumerate() {
                    self.metrics.instructions = self.metrics.instructions.saturating_add(1);
                    flow = match self.step(function, instruction) {
                        Ok(flow) => flow,
                        Err(fault) => {
                            let fault = self.annotate_fault(fault, block, instruction_index);
                            let pending = pending_fault.take();
                            let exit = pending
                                .as_ref()
                                .map(|item| item.exit.as_str())
                                .unwrap_or_else(|| exit_for_instruction(&instruction.op));
                            let origin = pending.as_ref().map_or(block, |item| item.origin);
                            // 子程序只负责处理自己内部（在入口块之后建立）的
                            // 嵌套 try；外层处理器必须等回到 CallSub 的调用点
                            // 再路由，否则会跳出子程序却仍继续执行子块。
                            match self.route_fault_scoped(function, origin, exit, fault, Some(sub))
                            {
                                Ok(RouteDecision::Jump(handler)) => Flow::Jump(handler),
                                Ok(RouteDecision::Flow(flow)) => flow,
                                Err(fault) => return Err(fault),
                            }
                        }
                    };
                    if !matches!(flow, Flow::Next) {
                        break;
                    }
                }
                match flow {
                    Flow::Jump(next) => {
                        if next < sub {
                            return Ok(Flow::Jump(next));
                        }
                        // 只登记留在子程序内部的转移；`next < sub` 的那一支会
                        // 返回给外层 `run_blocks`，由它的 `Flow::Jump` 分支统一
                        // 计数，这里再记一次就是重复计数。
                        self.note_map_point(MapPoint::JumpTarget);
                        self.prune_handler_contexts(function, next);
                        self.prune_scopes_for_target(function, next);
                        block = next;
                    }
                    Flow::JumpWithFault { target, kind } => {
                        if target < sub {
                            return Ok(Flow::JumpWithFault { target, kind });
                        }
                        pending_fault = Some(PendingFault {
                            exit: "dynamic_check_failure".to_owned(),
                            origin: block,
                        });
                        if let Some(frame) = self.frames.last_mut() {
                            frame.pending_check_kind = Some(kind);
                        }
                        self.note_map_point(MapPoint::JumpTarget);
                        self.prune_handler_contexts(function, target);
                        block = target;
                    }
                    Flow::RetFromSub => return Ok(Flow::Next),
                    Flow::Return(register) => return Ok(Flow::Return(register)),
                    Flow::Next => return Ok(Flow::Next),
                }
            }
        })();
        if let Some(frame) = self.frames.last_mut() {
            if result.is_err() {
                frame.subroutine_faults.push(sub);
            }
            if let Ok(flow) = &result {
                if let Some(pending) = frame.pending_exits.last_mut() {
                    *pending = exit_name_for_flow(flow, pending);
                }
            }
            let _ = frame.pending_exits.pop();
            let _ = frame.active_subroutines.pop();
        }
        result
    }

    /// 在当前帧查找处理器、执行清理并决定继续跳转或向外传播。
    ///
    /// 块号区间只负责确认一个入口属于该处理器；真正的嵌套判定依赖当前帧
    /// 的作用域栈和 catch 上下文。这样即使多个嵌套处理器共享一个起点，
    /// 也不会把前置代码或已经离开的作用域误当成受保护体。
    fn route_fault(
        &mut self,
        function: &TacFunction,
        current_block: BlockId,
        fault_exit: &str,
        fault: Fault,
    ) -> Result<RouteDecision, Fault> {
        self.route_fault_scoped(function, current_block, fault_exit, fault, None)
    }

    /// 在指定的子程序边界内路由故障。
    ///
    /// `floor` 用来限制子程序内部只能跳入同一子程序之后建立的嵌套处理器；
    /// 外层处理器要等 `CallSub` 返回错误后在调用点处理。否则清理代码会跳到
    /// 外层 `catch`，随后又从错误的子程序上下文继续执行。
    fn route_fault_scoped(
        &mut self,
        function: &TacFunction,
        current_block: BlockId,
        fault_exit: &str,
        fault: Fault,
        floor: Option<BlockId>,
    ) -> Result<RouteDecision, Fault> {
        let Fault::Error(mut error) = fault else {
            // Fatal 是刻意的不对称通道：不查表、不清理、不进入普通 catch。
            return Err(fault);
        };
        let (
            origin_block,
            active_scopes,
            active_catches,
            active_subroutines,
            mut completed,
            failed_subroutines,
        ) = {
            let Some(frame) = self.frames.last_mut() else {
                return Err(Fault::Error(error));
            };
            (
                current_block,
                frame.scopes.clone(),
                frame.active_catches.clone(),
                frame.active_subroutines.clone(),
                frame.completed_finally.clone(),
                std::mem::take(&mut frame.subroutine_faults),
            )
        };
        let failed_scopes = failed_subroutines
            .iter()
            .filter_map(|sub| {
                function
                    .handlers
                    .iter()
                    .find(|handler| handler.handler == *sub && handler.exit == "finally")
                    .map(|handler| handler.scope)
            })
            .collect::<Vec<_>>();
        for sub in failed_subroutines {
            // 失败的 finally 已经离开 active_subroutines，仍要把它标成完成，
            // 防止同一个故障在调用点再次触发它。
            for handler in function
                .handlers
                .iter()
                .filter(|handler| handler.handler == sub && handler.exit == "finally")
            {
                completed.push((handler.scope, sub));
            }
        }
        let contains = |handler: &TacHandler| {
            origin_block >= handler.protected.0
                && origin_block < handler.protected.1
                && floor.is_none_or(|minimum| handler.protected.0 > minimum)
        };
        let scope_active = |scope: u32| active_scopes.contains(&scope);
        let catch_context_active =
            |scope: u32| active_catches.iter().any(|(owner, _)| *owner == scope);
        let scope_rank = |scope: u32| {
            active_scopes
                .iter()
                .position(|item| *item == scope)
                .unwrap_or(0)
        };
        let driver = RuntimeDriver::new();

        let mut catches = function
            .handlers
            .iter()
            .enumerate()
            .filter(|(_, handler)| {
                contains(handler)
                    && handler.catch_type.is_some()
                    && (scope_active(handler.scope) || catch_context_active(handler.scope))
                    && !active_subroutines.contains(&handler.handler)
                    && !failed_scopes.contains(&handler.scope)
            })
            .filter_map(|(index, handler)| {
                let name = handler.catch_type.as_deref()?;
                let route = driver.dispatch_catch(error.clone(), &[name]);
                matches!(route, CatchRoute::Matched { .. }).then_some((index, handler.clone()))
            })
            .collect::<Vec<_>>();
        catches.sort_by_key(|(index, handler)| {
            (
                std::cmp::Reverse(scope_rank(handler.scope)),
                handler
                    .protected
                    .1
                    .get()
                    .saturating_sub(handler.protected.0.get()),
                *index,
            )
        });
        let selected = catches.into_iter().next();
        let selected_depth = selected
            .as_ref()
            .map(|(_, handler)| scope_rank(handler.scope));

        let mut finalies = function
            .handlers
            .iter()
            .filter(|handler| {
                if handler.exit != "finally"
                    || !contains(handler)
                    || active_subroutines.contains(&handler.handler)
                    || completed.contains(&(handler.scope, handler.handler))
                {
                    return false;
                }
                let active = scope_active(handler.scope) || catch_context_active(handler.scope);
                if !active {
                    return false;
                }
                selected_depth.is_none_or(|depth| scope_rank(handler.scope) >= depth)
            })
            .cloned()
            .collect::<Vec<_>>();
        finalies.sort_by_key(|handler| {
            (
                std::cmp::Reverse(scope_rank(handler.scope)),
                handler.handler,
            )
        });
        finalies.dedup_by_key(|handler| handler.handler);

        // finally -> drop；清理错误只进入 suppressed，Fatal 则立即胜出。
        let mut override_flow: Option<Flow> = None;
        let mut pending_exit_name = fault_exit.to_owned();
        for handler in &finalies {
            if let Some(frame) = self.frames.last_mut() {
                let marker = (handler.scope, handler.handler);
                if !frame.completed_finally.contains(&marker) {
                    frame.completed_finally.push(marker);
                }
            }
            self.sink.record(VmEvent::HandlerEntered {
                scope: handler.scope,
                handler: handler.handler.get(),
            });
            match self.run_subroutine(function, handler.handler, &pending_exit_name) {
                Ok(flow) => {
                    if !matches!(flow, Flow::Next) {
                        pending_exit_name = exit_name_for_flow(&flow, &pending_exit_name);
                        override_flow = Some(flow);
                    }
                }
                Err(Fault::Error(cleanup)) => error.push_suppressed(cleanup),
                Err(Fault::Fatal(fatal)) => return Err(Fault::Fatal(fatal)),
            }
        }

        if let Some(flow) = override_flow {
            return Ok(RouteDecision::Flow(flow));
        }

        if let Some((_, handler)) = selected {
            // 处理器命中时，清理从当前最内层作用域一直到该 try 作用域，
            // 每层都使用冻结的 Catch 退出计划。
            let scopes = self.scopes_until(handler.scope, true);
            for scope in scopes {
                self.run_plan(function, scope, "catch");
            }
            self.truncate_scope(handler.scope);
            if let Some(binding) = handler.binding {
                self.write(binding, RuntimeValue::error(error.clone()));
            }
            if let Some(frame) = self.frames.last_mut() {
                frame.active_catches.push((handler.scope, handler.handler));
            }
            self.sink.record(VmEvent::HandlerMatched {
                scope: handler.scope,
                handler: handler.handler.get(),
                catch_type: handler.catch_type.clone(),
            });
            return Ok(RouteDecision::Jump(handler.handler));
        }

        if floor.is_some() {
            // 子程序内没有可匹配的嵌套处理器时，只把故障交回
            // `CallSub` 调用点。此时不能清理整帧，否则会提前抹掉外层
            // `catch` 的作用域；调用点的路由器会统一执行完整展开。
            return Err(Fault::Error(error));
        }

        // 没有匹配处理器时，当前帧所有仍在栈上的作用域都按
        // `UnmatchedError` 展开。检查失败保留其专用退出边，便于审计。
        let cleanup_exit = if fault_exit == "dynamic_check_failure" {
            fault_exit
        } else {
            "unmatched_error"
        };
        let scopes = self
            .frames
            .last()
            .map(|frame| {
                let mut seen = std::collections::HashSet::new();
                frame
                    .scopes
                    .iter()
                    .rev()
                    .copied()
                    .filter(|scope| seen.insert(*scope))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for scope in scopes {
            self.run_plan(function, scope, cleanup_exit);
            self.sink.record(VmEvent::HandlerUnmatched { scope });
        }
        if let Some(frame) = self.frames.last_mut() {
            frame.scopes.clear();
            frame.active_catches.clear();
            frame.completed_finally.clear();
        }
        Err(Fault::Error(error))
    }

    /// 从当前作用域栈移除指定作用域及其内层作用域。
    fn truncate_scope(&mut self, scope: u32) {
        if let Some(frame) = self.frames.last_mut()
            && let Some(index) = frame.scopes.iter().rposition(|item| *item == scope)
        {
            frame.scopes.truncate(index);
        }
    }

    /// 返回当前帧从最内层到指定作用域的展开序列。
    fn scopes_until(&self, target: u32, include_target: bool) -> Vec<u32> {
        let mut scopes = Vec::new();
        if let Some(frame) = self.frames.last() {
            for scope in frame.scopes.iter().rev().copied() {
                scopes.push(scope);
                if scope == target {
                    break;
                }
            }
        }
        if include_target && !scopes.contains(&target) {
            scopes.push(target);
        }
        scopes
    }

    /// 离开 catch 体后清除动态处理器上下文。
    fn prune_handler_contexts(&mut self, function: &TacFunction, target: BlockId) {
        let Some(frame) = self.frames.last() else {
            return;
        };
        let active = frame.active_catches.clone();
        let kept = active
            .iter()
            .filter(|(_, catch_block)| {
                // 跳入 catch 的第一块尚未执行 EnterScope，需保留上下文；
                // 后续块在 catch 作用域仍活动时也会保留。正常退出先执行
                // ExitScope，再跳到父作用域桥块，因而会在此处清掉。
                *catch_block == target
                    || function
                        .block(*catch_block)
                        .and_then(|block| {
                            block.instructions.iter().find_map(|instruction| {
                                if let TacOp::EnterScope(scope) = instruction.op {
                                    Some(scope)
                                } else {
                                    None
                                }
                            })
                        })
                        .is_some_and(|scope| frame.scopes.contains(&scope))
            })
            .copied()
            .collect::<Vec<_>>();
        if let Some(frame) = self.frames.last_mut() {
            frame.active_catches = kept;
            frame.completed_finally.retain(|(owner, sub)| {
                function.handlers.iter().any(|handler| {
                    handler.scope == *owner
                        && handler.handler == *sub
                        && handler.exit == "finally"
                        && target < handler.protected.1
                })
            });
        }
    }

    /// 根据跳转目标的静态作用域收缩运行时作用域栈。
    ///
    /// `break`/`continue` 和 `finally` 内覆盖性的控制退出没有机会经过普通
    /// `ExitScope` 指令；若保留旧作用域，后续错误会再次看到已经离开的处理器。
    /// 普通结构化跳转也可以安全调用此方法：目标块若尚未进入（例如 catch
    /// 入口），目标作用域不在栈中，函数不会做任何收缩。
    fn prune_scopes_for_target(&mut self, function: &TacFunction, target: BlockId) {
        let Some(target_scope) = function.block(target).map(|block| block.scope) else {
            return;
        };
        let Some(frame) = self.frames.last_mut() else {
            return;
        };
        if let Some(index) = frame
            .scopes
            .iter()
            .rposition(|scope| *scope == target_scope)
        {
            frame.scopes.truncate(index + 1);
        }
    }

    /// 执行一个冻结释放计划，按 `order` 逐条释放。
    fn run_plan(&mut self, function: &TacFunction, scope: u32, exit: &str) {
        let program = self.program;
        let Some(plan) = program
            .plans
            .iter()
            .find(|plan| plan.scope == scope && plan.exit == exit)
        else {
            return;
        };
        let actions = plan.actions.clone();
        for action in &actions {
            let Some(register) = function.value_registers.get(&action.value).copied() else {
                continue;
            };
            // 只在实际持有值时才算一次释放：计划里的某些值（例如函数名绑定）
            // 在本帧从未被物化，把空槽位记成释放会虚报释放次数。
            let Some(_released) = self.take_if_present(register) else {
                continue;
            };
            self.metrics.releases = self.metrics.releases.saturating_add(1);
            self.sink.record(VmEvent::ValueReleased {
                scope,
                exit: exit.to_owned(),
                value: action.value,
                kind: action.kind.as_name().to_owned(),
            });
        }
    }

    /// 展开一个已经离开本帧路由器的故障。
    ///
    /// `run_blocks` 会先尝试当前帧的处理器；只有未匹配的错误或致命故障才会
    /// 到这里。这里**只处理最内层帧**，不能把调用方的作用域一起清掉，否则
    /// 外层 `catch` 永远没有机会接住从被调函数传播出来的错误。普通错误仍按
    /// `UnmatchedError` 计划逐层释放；`Fatal` 则完全跳过释放计划。
    fn unwind(&mut self, fault: &Fault) {
        let Some(frame) = self.frames.last() else {
            return;
        };
        let scopes = frame.scopes.clone();
        let function = self
            .program
            .functions
            .get(frame.function.get() as usize)
            .cloned();
        if matches!(fault, Fault::Error(_)) {
            if let Some(function) = function.as_ref() {
                for scope in scopes.iter().rev().copied() {
                    self.run_plan(function, scope, "unmatched_error");
                }
            }
        }
        if let Some(frame) = self.frames.last_mut() {
            frame.scopes.clear();
            frame.active_catches.clear();
            frame.completed_finally.clear();
            frame.pending_check_kind = None;
        }
    }

    /// 读取一组容器元素寄存器。
    fn collect(&mut self, registers: &[VReg]) -> Result<Vec<RuntimeValue>, Fault> {
        let mut values = Vec::with_capacity(registers.len());
        for register in registers {
            values.push(self.read(*register)?);
        }
        Ok(values)
    }

    /// 读取一组字典条目寄存器。
    fn collect_entries(
        &mut self,
        entries: &[(String, VReg)],
    ) -> Result<Vec<(String, RuntimeValue)>, Fault> {
        let mut values = Vec::with_capacity(entries.len());
        for (key, register) in entries {
            values.push((key.clone(), self.read(*register)?));
        }
        Ok(values)
    }

    /// 求值一组实参。
    fn bind(&mut self, arguments: &[TacArgument]) -> Result<Vec<BoundArgument>, Fault> {
        let mut bound = Vec::with_capacity(arguments.len());
        for argument in arguments {
            bound.push(BoundArgument {
                keyword: argument.name.clone(),
                value: self.read(argument.value)?,
            });
        }
        Ok(bound)
    }

    /// 读取当前帧的一个寄存器。
    fn read(&self, register: VReg) -> Result<RuntimeValue, Fault> {
        let Some(frame) = self.frames.last() else {
            return Err(Fault::Error(XiaoError::invalid_value("没有活动调用帧")));
        };
        frame.carrier.read(register).map_err(Fault::Error)
    }

    /// 取出当前帧的一个寄存器。
    fn take(&mut self, register: VReg) -> Result<RuntimeValue, Fault> {
        let Some(frame) = self.frames.last_mut() else {
            return Err(Fault::Error(XiaoError::invalid_value("没有活动调用帧")));
        };
        frame
            .carrier
            .take(register)
            .ok_or_else(|| Fault::Error(XiaoError::invalid_handle("寄存器为空")))
    }

    /// 取出当前帧的一个寄存器；为空时返回 `None` 而不报错。
    fn take_if_present(&mut self, register: VReg) -> Option<RuntimeValue> {
        self.frames.last_mut()?.carrier.take(register)
    }

    /// 写入当前帧的一个寄存器。
    fn write(&mut self, register: VReg, value: RuntimeValue) {
        if let Some(frame) = self.frames.last_mut() {
            frame.carrier.write(register, value);
        }
    }

    /// 写入指令结果寄存器；无结果寄存器的指令被忽略。
    fn write_operand(&mut self, dst: Option<VReg>, value: RuntimeValue) {
        if let Some(register) = dst {
            self.write(register, value);
        }
    }
}

/// 把常量池条目转成运行时值。
fn constant_value(constant: &TacConstant) -> Result<RuntimeValue, Fault> {
    let value = match constant {
        TacConstant::Int(value) => RuntimeValue::Int(*value),
        TacConstant::Sint(value) => RuntimeValue::Sint(*value),
        TacConstant::Lint(text) => RuntimeValue::Lint(text.clone()),
        TacConstant::Float(value) => RuntimeValue::Float(*value),
        TacConstant::Sfloat(value) => RuntimeValue::Sfloat(*value),
        TacConstant::Lfloat(text) => RuntimeValue::Lfloat(text.clone()),
        TacConstant::Bool(value) => RuntimeValue::Bool(*value),
        TacConstant::Str(text) => RuntimeValue::new_string(text.clone()).map_err(Fault::Error)?,
    };
    Ok(value)
}

/// 从指令形态映射到冻结的退出边名称。
fn exit_for_instruction(op: &TacOp) -> &'static str {
    match op {
        TacOp::Raise { .. } => "raise",
        TacOp::MakeError { .. } => "construct_failure",
        _ => "error",
    }
}

/// 把子程序返回的控制流更新为新的挂起退出类别。
///
/// 普通 `Jump` 既可能是子程序内部的结构化跳转，也可能是 `break`/
/// `continue` 的跨作用域跳转；调用方已经用目标块判定边界，因此这里保留
/// 原类别。`return` 与动态检查失败则有稳定的一对一类别。
fn exit_name_for_flow(flow: &Flow, fallback: &str) -> String {
    match flow {
        Flow::Return(_) => "return".to_owned(),
        Flow::JumpWithFault { .. } => "dynamic_check_failure".to_owned(),
        _ => fallback.to_owned(),
    }
}

/// 返回运行时检查失败对应的稳定错误码。
fn runtime_check_code(kind: &str) -> Option<&'static str> {
    match kind {
        "boolean_condition" | "dynamic_conversion" | "string_boolean" => Some(TYPE_MISMATCH_CODE),
        "arithmetic" | "numeric_range" => Some(NUMERIC_OVERFLOW_CODE),
        _ => None,
    }
}

/// 从检查/错误构造参数读取稳定文本。
fn runtime_text(value: &RuntimeValue) -> Result<String, Fault> {
    match value {
        RuntimeValue::Str(handle) => handle.to_string().map_err(Fault::Error),
        RuntimeValue::Lint(text) | RuntimeValue::Lfloat(text) => Ok(text.clone()),
        _ => Err(Fault::Error(XiaoError::type_mismatch(
            "str",
            value.type_name(),
        ))),
    }
}

/// 判定本批次真正支持的四类 Runtime 检查。
fn check_value(kind: &str, value: &RuntimeValue) -> bool {
    match kind {
        "boolean_condition" => value.as_bool().is_some(),
        "dynamic_conversion" => matches!(value, RuntimeValue::Error(_)),
        "numeric_range" => match value {
            RuntimeValue::Int(_) | RuntimeValue::Sint(_) => true,
            RuntimeValue::Float(value) => value.is_finite(),
            RuntimeValue::Sfloat(value) => value.is_finite(),
            RuntimeValue::Lint(value) => value.parse::<i128>().is_ok(),
            RuntimeValue::Lfloat(value) => value.parse::<f64>().is_ok_and(f64::is_finite),
            _ => false,
        },
        "arithmetic" => matches!(
            value,
            RuntimeValue::Int(_)
                | RuntimeValue::Sint(_)
                | RuntimeValue::Float(_)
                | RuntimeValue::Sfloat(_)
                | RuntimeValue::Bool(_)
        ),
        _ => true,
    }
}

/// 按调用签名把实参绑定到形参寄存器。
///
/// 位置实参按声明顺序填充；关键字实参按形参名重排——这正是 R1 冻结的
/// 「关键字参数由被调用方按名重排」，调用方只写实际提供的槽位。
fn bind_arguments<C: Carrier>(
    function: &TacFunction,
    program: &TacProgram,
    frame: &mut Frame<C>,
    arguments: &[BoundArgument],
) {
    let signature = function.signature.and_then(|id| program.signatures.get(id));
    let mut next_positional = 0_usize;
    for argument in arguments {
        let slot = match &argument.keyword {
            None => {
                let slot = next_positional;
                next_positional = next_positional.saturating_add(1);
                slot
            }
            Some(name) => signature
                .and_then(|signature| {
                    signature
                        .parameter_names
                        .iter()
                        .position(|item| item == name)
                })
                .unwrap_or(usize::MAX),
        };
        if let Some(register) = function.parameters.get(slot) {
            frame.carrier.write(*register, argument.value.clone());
        }
    }
}
