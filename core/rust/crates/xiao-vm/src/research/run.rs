//! 运行入口、结构化结果与运行指标。

use xiao_diagnostics::{FatalError, XiaoError};
use xiao_runtime::RuntimeValue;

use crate::research::carrier::Carrier;
use crate::research::machine::hybrid::HybridCarrier;
use crate::research::machine::register::RegisterCarrier;
use crate::research::machine::stack::StackCarrier;
use crate::research::semantics::{BoundArgument, Vm};
use crate::research::sink::{RecordingSink, VmEvent};

/// 一次运行的结构化结果。
///
/// 可恢复错误与致命故障分开表达：`Fatal` 不是「更严重的错误」，它不可被普通
/// `catch` 恢复，也不执行释放计划，因此不能与 `Error` 共用一个分支。
#[derive(Debug)]
pub enum RunResult {
    /// 正常结束。
    Success,
    /// 以可恢复错误结束。
    Error(XiaoError),
    /// 以不可恢复故障结束。
    Fatal(FatalError),
}

impl RunResult {
    /// 判断运行是否成功。
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// 返回可恢复错误码。
    #[must_use]
    pub fn error_code(&self) -> Option<&str> {
        match self {
            Self::Error(error) => Some(error.code()),
            Self::Fatal(error) => Some(error.code()),
            Self::Success => None,
        }
    }
}

/// 运行参数。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmOptions {
    /// 最大调用深度；超限产生不可恢复的栈溢出故障。
    pub max_call_depth: usize,
}

impl VmOptions {
    /// 创建默认参数。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_call_depth: 1024,
        }
    }
}

impl Default for VmOptions {
    /// 使用默认调用深度上限。
    fn default() -> Self {
        Self::new()
    }
}

/// 运行指标。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VmMetrics {
    /// 执行的指令条数。
    pub instructions: u64,
    /// 达到过的最大调用深度。
    pub max_call_depth: usize,
    /// 载体占用的历史峰值。
    pub max_stack_depth: usize,
    /// 按冻结计划释放的值的数量。
    pub releases: usize,
    /// 写入独立帧槽的次数。
    ///
    /// 它与 [`Self::stack_map_entries`] 量纲不同：这里数的是**落帧槽的次数**，
    /// 那里数的是**需要栈映射的程序点**。两者不可合并，否则三种机型在 09R3
    /// 的对比中不再可比。
    pub spill_count: u64,
    /// 需要建立栈映射的程序点数量（R1-F 的机型分界）。
    ///
    /// 栈式数跳转目标、混合式数调用点与帧尾、分类型寄存器式恒为 0；
    /// 详见 [`crate::research::carrier::CarrierMetrics::stack_map_entries`]。
    pub stack_map_entries: usize,
    /// 跨调用保存值的次数。
    pub call_save_count: u64,
}

/// 一次运行的完整结果。
#[derive(Debug)]
pub struct RunOutcome {
    /// 结构化结果。
    pub result: RunResult,
    /// 研究 VM 入口返回的值；脚本没有显式返回值时为 `None`。
    ///
    /// 这是为了让 09R 研究测试观察语义结果而暴露的调试字段，不是生产执行
    /// 接口的稳定承诺。可恢复错误或致命故障结束时该字段始终为 `None`。
    pub value: Option<RuntimeValue>,
    /// 运行指标。
    pub metrics: VmMetrics,
    /// 记录到的调试事件。
    pub events: Vec<VmEvent>,
}

/// 用栈式载体运行一份三地址产物并记录全部事件。
#[must_use]
pub fn run(program: &xiao_bytecode::research::TacProgram, options: VmOptions) -> RunOutcome {
    run_with::<StackCarrier>(program, options)
}

/// 使用指定静态载体运行一份三地址产物并记录全部事件。
#[must_use]
pub fn run_with<C: Carrier>(
    program: &xiao_bytecode::research::TacProgram,
    options: VmOptions,
) -> RunOutcome {
    let mut vm = Vm::<C, RecordingSink>::new(program, options, RecordingSink::new());
    let (result, value) = vm.run_with_value();
    let metrics = vm.metrics();
    RunOutcome {
        result,
        value,
        metrics,
        events: vm.into_sink().into_events(),
    }
}

/// 使用位置实参和指定载体运行一份三地址产物。
///
/// 该入口只供研究 VM 的动态边界向量注入入口形参；实参按入口函数的形参
/// 声明顺序绑定，不承诺生产 VM 的脚本调用 ABI。
#[must_use]
pub fn run_with_values<C: Carrier>(
    program: &xiao_bytecode::research::TacProgram,
    options: VmOptions,
    arguments: &[RuntimeValue],
) -> RunOutcome {
    let bound_arguments = arguments
        .iter()
        .cloned()
        .map(|value| BoundArgument {
            keyword: None,
            value,
        })
        .collect::<Vec<_>>();
    let mut vm = Vm::<C, RecordingSink>::new(program, options, RecordingSink::new());
    let (result, value) = vm.run_with_arguments(&bound_arguments);
    let metrics = vm.metrics();
    RunOutcome {
        result,
        value,
        metrics,
        events: vm.into_sink().into_events(),
    }
}

/// 使用指定种子的栈式载体运行一份三地址产物。
#[must_use]
pub fn run_with_seed(
    program: &xiao_bytecode::research::TacProgram,
    options: VmOptions,
    seed: u128,
) -> RunOutcome {
    let mut vm = Vm::<StackCarrier, RecordingSink>::new_with_seed(
        program,
        options,
        RecordingSink::new(),
        seed,
    );
    let (result, value) = vm.run_with_value();
    let metrics = vm.metrics();
    RunOutcome {
        result,
        value,
        metrics,
        events: vm.into_sink().into_events(),
    }
}

/// 使用指定种子的任意静态载体运行一份三地址产物。
#[must_use]
pub fn run_with_machine_seed<C: Carrier>(
    program: &xiao_bytecode::research::TacProgram,
    options: VmOptions,
    seed: u128,
) -> RunOutcome {
    let mut vm =
        Vm::<C, RecordingSink>::new_with_seed(program, options, RecordingSink::new(), seed);
    let (result, value) = vm.run_with_value();
    let metrics = vm.metrics();
    RunOutcome {
        result,
        value,
        metrics,
        events: vm.into_sink().into_events(),
    }
}

/// 用分类型寄存器载体运行一份三地址产物。
#[must_use]
pub fn run_register(
    program: &xiao_bytecode::research::TacProgram,
    options: VmOptions,
) -> RunOutcome {
    run_with::<RegisterCarrier>(program, options)
}

/// 用混合式窗口/求值栈载体运行一份三地址产物。
#[must_use]
pub fn run_hybrid(program: &xiao_bytecode::research::TacProgram, options: VmOptions) -> RunOutcome {
    run_with::<HybridCarrier>(program, options)
}
