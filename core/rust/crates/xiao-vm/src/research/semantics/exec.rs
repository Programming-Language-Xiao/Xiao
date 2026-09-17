//! 机型无关的三地址语义核。
//!
//! 本模块只认三地址指令与 [`Carrier`]，决定算什么、错怎么抛、清理怎么走、
//! 调用怎么进；它不认识栈、寄存器或帧槽，也不认识任何机型的细节。
//!
//! 释放顺序的唯一来源是冻结计划：退出点上执行 `(作用域, 退出边)` 计划，按
//! `order` 逐条释放。动态错误发生时按同一份计划展开当前作用域栈。

use xiao_bytecode::research::{
    BlockId, FuncId, TacArgument, TacConstant, TacFunction, TacInstr, TacOp, TacProgram, VReg,
};
use xiao_diagnostics::{FatalError, XiaoError};
use xiao_runtime::RuntimeValue;

use crate::research::carrier::Carrier;
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
    /// 返回，携带可选返回值寄存器。
    Return(Option<VReg>),
}

/// 三地址解释器。
pub struct Vm<'p, C: Carrier, S: VmEventSink> {
    program: &'p TacProgram,
    frames: Vec<Frame<C>>,
    sink: S,
    metrics: VmMetrics,
    options: VmOptions,
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
            Err(Fault::Error(error)) => RunResult::Error(error),
            Err(Fault::Fatal(fatal)) => RunResult::Fatal(fatal),
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
            self.sink.record(VmEvent::FatalRaised {
                code: fatal.code().to_owned(),
            });
            return Err(Fault::Fatal(fatal));
        }
        self.metrics.max_call_depth = self.metrics.max_call_depth.max(self.frames.len() + 1);
        let mut frame = Frame::new(target, function.name.clone(), C::empty(), return_to);
        bind_arguments(function, program, &mut frame, arguments);
        let depth = self.frames.len() + 1;
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
            self.metrics.max_stack_depth = self.metrics.max_stack_depth.max(frame.carrier.peak());
        }
        self.sink.record(VmEvent::FunctionReturned {
            function: function.name.clone(),
            depth,
        });
        outcome
    }

    /// 执行一个函数的基本块序列。
    fn run_blocks(&mut self, function: &TacFunction) -> Result<Option<RuntimeValue>, Fault> {
        let mut block = function.entry;
        loop {
            let Some(current) = function.blocks.get(block.get() as usize) else {
                return Ok(None);
            };
            let mut flow = Flow::Next;
            for instruction in &current.instructions {
                self.metrics.instructions = self.metrics.instructions.saturating_add(1);
                flow = self.step(function, instruction)?;
                if !matches!(flow, Flow::Next) {
                    break;
                }
            }
            match flow {
                Flow::Jump(next) => block = next,
                Flow::Return(register) => {
                    return match register {
                        Some(register) => self.read(register).map(Some),
                        None => Ok(None),
                    };
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
                let value = self.execute(*callee, &bound, instruction.dst)?;
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
            TacOp::Raise { .. } => {
                return Err(Fault::Error(XiaoError::invalid_value(
                    "raise 的错误对象模型尚未实现",
                )));
            }
            TacOp::Check { .. } => {
                return Err(Fault::Error(XiaoError::invalid_value(
                    "运行时检查指令尚未在解释器中实现",
                )));
            }
            TacOp::Release { value, .. } => {
                let _ = self.take(*value)?;
                self.metrics.releases = self.metrics.releases.saturating_add(1);
            }
            TacOp::Transfer { .. } => {}
            TacOp::RunReleasePlan { scope, exit } => {
                self.run_plan(function, *scope, exit);
            }
            TacOp::EnterScope(scope) => {
                if let Some(frame) = self.frames.last_mut() {
                    frame.scopes.push(*scope);
                }
                self.sink.record(VmEvent::ScopeEntered { scope: *scope });
            }
            TacOp::ExitScope { scope, exit } => {
                if let Some(frame) = self.frames.last_mut()
                    && let Some(index) = frame.scopes.iter().rposition(|item| item == scope)
                {
                    frame.scopes.truncate(index);
                }
                self.sink.record(VmEvent::ScopeExited {
                    scope: *scope,
                    exit: exit.clone(),
                });
            }
        }
        Ok(Flow::Next)
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
            let _ = self.take(register);
            self.metrics.releases = self.metrics.releases.saturating_add(1);
            self.sink.record(VmEvent::ValueReleased {
                value: action.value,
                kind: action.kind.as_name().to_owned(),
            });
        }
    }

    /// 展开一个错误：按冻结计划清理尚未退出的作用域。
    ///
    /// 致命故障不执行释放计划，只记录退出——继续运行释放钩子在致命故障下已经
    /// 不安全，这是刻意的不对称。
    fn unwind(&mut self, fault: &Fault) {
        let program = self.program;
        let exit = match fault {
            Fault::Error(_) => "error",
            Fault::Fatal(_) => "fatal",
        };
        for frame in self.frames.iter_mut().rev() {
            let Some(function) = program.functions.get(frame.function.get() as usize) else {
                continue;
            };
            let scopes = frame.scopes.clone();
            if matches!(fault, Fault::Error(_)) {
                for scope in scopes.iter().rev() {
                    let Some(plan) = program
                        .plans
                        .iter()
                        .find(|plan| plan.scope == *scope && plan.exit == exit)
                    else {
                        continue;
                    };
                    for action in &plan.actions {
                        if let Some(register) = function.value_registers.get(&action.value).copied()
                        {
                            let _ = frame.carrier.take(register);
                        }
                    }
                }
            }
            frame.scopes.clear();
        }
        match fault {
            Fault::Error(error) => {
                self.sink.record(VmEvent::ErrorRaised {
                    code: error.code().to_owned(),
                    message_id: error.message_id().to_owned(),
                });
            }
            Fault::Fatal(fatal) => {
                self.sink.record(VmEvent::FatalRaised {
                    code: fatal.code().to_owned(),
                });
            }
        }
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
