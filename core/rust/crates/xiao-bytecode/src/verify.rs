//! 生产字节码验证器。
//!
//! 验证器是降低器与编码器/执行器之间的唯一结构关卡：它不修复输入，也不重新
//! 推断类型或生命周期，只确认冻结的 TAC 事实彼此一致。research 路径通过
//! 重导出层继续可用，但生产调用方应使用本模块的入口。

use std::collections::{BTreeSet, VecDeque};
use std::fmt::{Display, Formatter};

use xiao_ir::{IrProgram, IrType, ObservedRelease, reconcile_release_plans};
use xiao_lifetime::ExitKind;
use xiao_syntax::ScalarType;

use crate::cfg::jump_targets;
use crate::liveness::{analyze, instruction_use_def};
use crate::sig::{CallSig, ParamKind};
use crate::tac::{
    ArgKind, BlockId, CategoryMap, RegisterClass, TacFunction, TacInstr, TacOp, TacProgram, VReg,
};

/// TacProgram.unsupported 出现在生产路径时使用的稳定内部一致性编号。
pub const TAC_INTERNAL_CONSISTENCY_CODE: &str = "X09-BYTECODE-001";
/// TAC 结构、引用或 ABI 不一致时使用的稳定验证编号。
pub const TAC_STRUCTURE_CODE: &str = "X09-BYTECODE-002";

/// 一条生产验证错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TacVerificationError {
    /// 稳定错误编号。
    pub code: &'static str,
    /// 机器可读的产物路径。
    pub path: String,
    /// 面向开发者的简短原因。
    pub message: String,
}

impl TacVerificationError {
    /// 创建内部一致性错误。
    fn internal(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: TAC_INTERNAL_CONSISTENCY_CODE,
            path: path.into(),
            message: message.into(),
        }
    }

    /// 创建结构验证错误。
    fn structure(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: TAC_STRUCTURE_CODE,
            path: path.into(),
            message: message.into(),
        }
    }

    /// 判断是否为内部一致性错误。
    #[must_use]
    pub fn is_internal(&self) -> bool {
        self.code == TAC_INTERNAL_CONSISTENCY_CODE
    }
}

impl Display for TacVerificationError {
    /// 输出稳定编号、路径和开发者原因。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} at {}: {}",
            self.code, self.path, self.message
        )
    }
}

impl std::error::Error for TacVerificationError {}

/// 三地址自校验与释放序列对账结果。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TacVerification {
    /// 未降低的构造；非空表示降低产物不完整。
    pub unsupported: Vec<String>,
    /// 结构、引用或 ABI 错误说明（保留字符串视图供研究期调用方使用）。
    pub errors: Vec<String>,
    /// 需要生产入口拒绝的结构化内部错误。
    pub internal_errors: Vec<TacVerificationError>,
}

impl TacVerification {
    /// 判断验证是否通过。
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.unsupported.is_empty() && self.errors.is_empty() && self.internal_errors.is_empty()
    }

    /// 返回首条结构化内部错误。
    #[must_use]
    pub fn first_internal_error(&self) -> Option<&TacVerificationError> {
        self.internal_errors.first()
    }
}

/// 校验一份三地址产物的完整结构。
///
/// 检查指令边界和跨表引用、每个函数自己的寄存器类别与活跃区间、跳转目标、
/// 静态调用帧、finally 子程序配对，以及实际引用的释放计划。程序级
/// TacProgram::categories 只是历史兼容视图，验证器不会读取它。
#[must_use]
pub fn verify_program(program: &IrProgram, tac: &TacProgram) -> TacVerification {
    let mut result = TacVerification {
        unsupported: tac.unsupported.clone(),
        errors: Vec::new(),
        internal_errors: Vec::new(),
    };

    for (index, note) in tac.unsupported.iter().enumerate() {
        result.internal_errors.push(TacVerificationError::internal(
            format!("program.unsupported[{index}]"),
            format!("生产字节码含未降低构造：{note}"),
        ));
    }
    if tac.functions.is_empty() {
        push_error(&mut result, "program.functions", "函数表不能为空");
    }
    if tac.abi.ir_version != program.version {
        push_error(
            &mut result,
            "program.abi.ir_version",
            format!(
                "TAC 的 IR 版本 {} 与输入 IR 版本 {} 不一致",
                tac.abi.ir_version, program.version
            ),
        );
    }

    validate_signatures(tac, &mut result);
    validate_tables(tac, &mut result);
    validate_plans(program, tac, &mut result);
    for (function_index, function) in tac.functions.iter().enumerate() {
        validate_function(tac, function_index, function, &mut result);
    }

    // 释放序列的最终判据仍由 xiao-ir 的并列入口提供；这里传入的是 TAC
    // 实际发出的计划观测，不能只检查计划表是否存在。
    let observed = observed_plans(tac);
    for error in reconcile_release_plans(program, &observed).errors() {
        push_error(
            &mut result,
            error.path.clone(),
            format!("{}: {}", error.code, error.message),
        );
    }
    result
}

/// 生产执行器应调用的验证入口。
///
/// TacProgram.unsupported 不是用户可见的“暂不支持”诊断：它表示降低器与
/// 生产契约脱节，必须以内部一致性错误拒绝。结构错误也在进入编码器或 VM 前
/// 统一返回，避免把问题延迟成 EncodeError::Unsupported。
pub fn verify_production(
    program: &IrProgram,
    tac: &TacProgram,
) -> Result<(), TacVerificationError> {
    let result = verify_program(program, tac);
    if let Some(error) = result.first_internal_error() {
        return Err(error.clone());
    }
    if let Some(error) = result.errors.first() {
        return Err(TacVerificationError::structure("program", error.clone()));
    }
    Ok(())
}

/// verify_production 的语义别名，供驱动器代码按“执行前验证”命名调用。
pub fn verify_for_execution(
    program: &IrProgram,
    tac: &TacProgram,
) -> Result<(), TacVerificationError> {
    verify_production(program, tac)
}

/// 验证所有调用签名的平行数组、变参槽位和参数类别。
fn validate_signatures(tac: &TacProgram, result: &mut TacVerification) {
    for (index, signature) in tac.signatures.iter().enumerate() {
        let path = format!("signatures[{index}]");
        let lengths = [
            signature.parameter_names.len(),
            signature.parameter_kinds.len(),
            signature.parameter_types.len(),
            signature.has_defaults.len(),
        ];
        if lengths.iter().any(|length| *length != lengths[0]) {
            push_error(result, path, "调用签名的参数平行数组长度不一致");
            continue;
        }
        let mut var_args = None;
        let mut kw_args = None;
        for (parameter, kind) in signature.parameter_kinds.iter().enumerate() {
            match kind {
                ParamKind::VarArgs => {
                    if var_args.replace(parameter).is_some() {
                        push_error(
                            result,
                            format!("signatures[{index}].parameter_kinds"),
                            "同一调用签名不能有两个 *args 形参",
                        );
                    }
                }
                ParamKind::VarKeywords if kw_args.replace(parameter).is_some() => {
                    push_error(
                        result,
                        format!("signatures[{index}].parameter_kinds"),
                        "同一调用签名不能有两个 **kwargs 形参",
                    );
                }
                _ => {}
            }
        }
        for (field, declared, discovered) in [
            ("var_args_slot", signature.var_args_slot, var_args),
            ("kw_args_slot", signature.kw_args_slot, kw_args),
        ] {
            if declared != discovered {
                push_error(
                    result,
                    format!("signatures[{index}].{field}"),
                    format!("{field} 与参数类别不一致"),
                );
            }
            if let Some(slot) = declared {
                if slot >= lengths[0] {
                    push_error(
                        result,
                        format!("signatures[{index}].{field}"),
                        format!("{field} 超出参数数量 {}", lengths[0]),
                    );
                }
            }
        }
    }
}

/// 验证程序级表定义引用，避免调用帧间接跳入错误函数。
fn validate_tables(tac: &TacProgram, result: &mut TacVerification) {
    let mut names = BTreeSet::new();
    for (index, table) in tac.table_definitions.iter().enumerate() {
        if !names.insert(table.signature.name.clone()) {
            push_error(
                result,
                format!("table_definitions[{index}].signature.name"),
                "表签名名称重复",
            );
        }
        if table.signature.runtime_signature().is_none() {
            push_error(
                result,
                format!("table_definitions[{index}].signature"),
                "表签名不能转换为 Runtime 签名",
            );
        }
        check_function_reference(
            tac,
            table.fields,
            &format!("table_definitions[{index}].fields"),
            result,
        );
        for (name, function) in &table.methods {
            check_function_reference(
                tac,
                *function,
                &format!("table_definitions[{index}].methods[{name}]"),
                result,
            );
            if !table
                .signature
                .members
                .iter()
                .any(|member| member.method && member.name == *name)
            {
                push_error(
                    result,
                    format!("table_definitions[{index}].methods[{name}]"),
                    "表方法没有对应的静态成员签名",
                );
            }
        }
    }
}

/// 验证 TAC 释放计划自身，并与 IR 计划逐条对齐。
fn validate_plans(program: &IrProgram, tac: &TacProgram, result: &mut TacVerification) {
    let mut seen = BTreeSet::new();
    for (index, plan) in tac.plans.iter().enumerate() {
        let path = format!("plans[{index}]");
        if !seen.insert((plan.scope, plan.exit.clone())) {
            push_error(result, path.clone(), "释放计划键重复");
        }
        if ExitKind::from_name(&plan.exit).is_none() {
            push_error(result, format!("{path}.exit"), "退出边名称不是冻结拼写");
        }
        validate_action_orders(&path, &plan.actions, result);
        let Some(source) = program
            .ownership
            .release_plans
            .iter()
            .find(|item| item.scope == plan.scope && item.exit == plan.exit)
        else {
            push_error(result, path, "释放计划在输入 IR 中不存在");
            continue;
        };
        let actual = plan
            .actions
            .iter()
            .map(|action| (action.value, action.order, action.kind.as_name()))
            .collect::<Vec<_>>();
        let expected = source
            .actions
            .iter()
            .map(|action| (action.value, action.order, action.kind.as_str()))
            .collect::<Vec<_>>();
        if actual != expected {
            push_error(
                result,
                format!("plans[{index}].actions"),
                "TAC 释放动作与输入 IR 计划不一致",
            );
        }
        if plan.transferred != source.transferred {
            push_error(
                result,
                format!("plans[{index}].transferred"),
                "TAC 转移值集合与输入 IR 计划不一致",
            );
        }
    }
}

/// 检查释放动作的顺序编号连续且唯一。
fn validate_action_orders(
    path: &str,
    actions: &[crate::lower::TacReleaseAction],
    result: &mut TacVerification,
) {
    let mut orders = BTreeSet::new();
    for action in actions {
        if !orders.insert(action.order) {
            push_error(result, format!("{path}.actions"), "释放顺序编号重复");
        }
    }
    let expected = (0..actions.len()).collect::<Vec<_>>();
    let mut actual = actions
        .iter()
        .map(|action| action.order)
        .collect::<Vec<_>>();
    actual.sort_unstable();
    if actual != expected {
        push_error(
            result,
            format!("{path}.actions"),
            "释放顺序必须从零开始且连续",
        );
    }
}

/// 验证一个函数及其所有指令。
fn validate_function(
    tac: &TacProgram,
    function_index: usize,
    function: &TacFunction,
    result: &mut TacVerification,
) {
    let path = format!("functions[{function_index}]");
    if function.blocks.is_empty() {
        push_error(result, format!("{path}.blocks"), "函数至少需要一个基本块");
    }
    if function.entry.get() as usize >= function.blocks.len() {
        push_error(
            result,
            format!("{path}.entry"),
            format!("入口块 {} 越界", function.entry.get()),
        );
    }
    let mut block_ids = BTreeSet::new();
    for (block_index, block) in function.blocks.iter().enumerate() {
        if block.id.get() as usize != block_index || !block_ids.insert(block.id) {
            push_error(
                result,
                format!("{path}.blocks[{block_index}].id"),
                "基本块目录必须按 BlockId 严格递增",
            );
        }
        if !function.scopes.is_empty() && !function.scopes.contains(&block.scope) {
            push_error(
                result,
                format!("{path}.blocks[{block_index}].scope"),
                "基本块引用了未登记的作用域",
            );
        }
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            if instruction.span.start > instruction.span.end {
                push_error(
                    result,
                    format!("{path}.blocks[{block_index}].instructions[{instruction_index}].span"),
                    "指令源码区间起点不能大于终点",
                );
            }
            validate_instruction(
                tac,
                function_index,
                function,
                block.id,
                instruction_index,
                instruction,
                result,
            );
        }
    }
    validate_handlers(function_index, function, result);
    validate_function_signature(tac, function_index, function, result);
    validate_registers(tac, function_index, function, result);
    validate_subroutines(function_index, function, result);
}

/// 验证函数签名与形参寄存器的数量和类别。
fn validate_function_signature(
    tac: &TacProgram,
    function_index: usize,
    function: &TacFunction,
    result: &mut TacVerification,
) {
    let path = format!("functions[{function_index}]");
    let Some(signature_id) = function.signature else {
        if !function.parameters.is_empty() {
            push_error(
                result,
                format!("{path}.parameters"),
                "没有调用签名的函数不能声明形参",
            );
        }
        return;
    };
    let Some(signature) = tac.signatures.get(signature_id) else {
        push_error(
            result,
            format!("{path}.signature"),
            format!("签名 {} 越界", signature_id.get()),
        );
        return;
    };
    if signature.parameter_types.len() != function.parameters.len() {
        push_error(
            result,
            format!("{path}.parameters"),
            format!(
                "形参寄存器数量 {} 与签名数量 {} 不一致",
                function.parameters.len(),
                signature.parameter_types.len()
            ),
        );
    }
    for (index, (register, ty)) in function
        .parameters
        .iter()
        .zip(&signature.parameter_types)
        .enumerate()
    {
        expect_class(
            function,
            *register,
            class_of_type(ty),
            &format!("{path}.parameters[{index}]"),
            result,
        );
    }
}

/// 验证 handler 的保护区间、入口和 catch/finally 形态。
fn validate_handlers(function_index: usize, function: &TacFunction, result: &mut TacVerification) {
    let path = format!("functions[{function_index}]");
    for (handler_index, handler) in function.handlers.iter().enumerate() {
        let handler_path = format!("{path}.handlers[{handler_index}]");
        let start = handler.protected.0.get() as usize;
        let end = handler.protected.1.get() as usize;
        if start >= function.blocks.len() {
            push_error(
                result,
                format!("{handler_path}.protected.start"),
                "保护区起点越界",
            );
        }
        if end > function.blocks.len() {
            push_error(
                result,
                format!("{handler_path}.protected.end"),
                "保护区终点越界",
            );
        }
        if start > end {
            push_error(result, handler_path.clone(), "保护区间倒置");
        }
        if handler.handler.get() as usize >= function.blocks.len() {
            push_error(result, format!("{handler_path}.handler"), "处理器入口越界");
        }
        if !function.scopes.is_empty() && !function.scopes.contains(&handler.scope) {
            push_error(
                result,
                format!("{handler_path}.scope"),
                "处理器引用了未登记的作用域",
            );
        }
        if handler.exit != "finally" && ExitKind::from_name(&handler.exit).is_none() {
            push_error(
                result,
                format!("{handler_path}.exit"),
                "处理器退出边不是冻结拼写",
            );
        }
        if handler.exit == "finally" {
            if handler.catch_type.is_some() || handler.binding.is_some() {
                push_error(
                    result,
                    handler_path.clone(),
                    "finally 处理器不能携带 catch 类型或绑定寄存器",
                );
            }
        } else if handler.catch_type.is_none() {
            push_error(
                result,
                format!("{handler_path}.catch_type"),
                "catch 处理器缺少错误类型",
            );
        }
        if let Some(binding) = handler.binding {
            expect_class(
                function,
                binding,
                RegisterClass::Dynamic,
                &format!("{handler_path}.binding"),
                result,
            );
        }
    }
}

/// 验证每个函数自己的类别表与活跃区间，不读取程序级兼容类别视图。
fn validate_registers(
    tac: &TacProgram,
    function_index: usize,
    function: &TacFunction,
    result: &mut TacVerification,
) {
    let path = format!("functions[{function_index}]");
    let mut registers = BTreeSet::new();
    for register in function
        .parameters
        .iter()
        .chain(&function.locals)
        .chain(function.value_registers.values())
    {
        registers.insert(*register);
    }
    for block in &function.blocks {
        for instruction in &block.instructions {
            let (uses, defs) = instruction_use_def(function, instruction, &tac.plans);
            registers.extend(uses);
            registers.extend(defs);
        }
    }
    for register in &registers {
        if !category_is_registered(&function.categories, *register) {
            push_error(
                result,
                format!("{path}.categories[{}]", register.get()),
                "指令或活跃区间使用了未登记类别的寄存器",
            );
        }
    }
    let liveness = analyze(function, &tac.plans);
    for interval in liveness.intervals {
        if !category_is_registered(&function.categories, interval.register) {
            push_error(
                result,
                format!("{path}.categories[{}]", interval.register.get()),
                format!(
                    "活跃区间 {}..{} 没有对应的函数局部类别",
                    interval.start, interval.end
                ),
            );
        }
    }
}

/// 验证所有跳转目标、操作数引用和调用帧约束。
fn validate_instruction(
    tac: &TacProgram,
    function_index: usize,
    function: &TacFunction,
    block: BlockId,
    instruction_index: usize,
    instruction: &TacInstr,
    result: &mut TacVerification,
) {
    let path = format!(
        "functions[{function_index}].blocks[{}].instructions[{instruction_index}]",
        block.get()
    );
    let (uses, defs) = instruction_use_def(function, instruction, &tac.plans);
    for register in uses.into_iter().chain(defs) {
        check_register(function, register, &format!("{path}.register"), result);
    }
    for target in jump_targets(&instruction.op) {
        if target.get() as usize >= function.blocks.len() {
            push_error(
                result,
                format!("{path}.target"),
                format!("跳转目标块 {} 越界", target.get()),
            );
        }
    }
    match &instruction.op {
        TacOp::LoadConst(id) => {
            if let Some(constant) = tac.constants.get(*id) {
                if let Some(dst) = instruction.dst {
                    expect_class(
                        function,
                        dst,
                        constant.register_class(),
                        &format!("{path}.dst"),
                        result,
                    );
                }
            } else {
                push_error(
                    result,
                    format!("{path}.constant"),
                    format!("常量 {} 越界", id.get()),
                );
            }
        }
        TacOp::LoadNone => expect_dst_class(
            function,
            instruction.dst,
            RegisterClass::None,
            &format!("{path}.dst"),
            result,
        ),
        TacOp::LoadFunc(id) => {
            check_function_reference(tac, *id, &format!("{path}.function"), result);
        }
        TacOp::Cast { target, .. } => expect_dst_class(
            function,
            instruction.dst,
            class_of_scalar(*target),
            &format!("{path}.dst"),
            result,
        ),
        TacOp::Compare { .. } | TacOp::SetCompare { .. } => expect_dst_class(
            function,
            instruction.dst,
            RegisterClass::Bool,
            &format!("{path}.dst"),
            result,
        ),
        TacOp::SetOp { .. }
        | TacOp::NewArray { .. }
        | TacOp::NewTuple { .. }
        | TacOp::NewDictTable { .. }
        | TacOp::NewDictColumn { .. }
        | TacOp::NewSet { .. } => expect_dst_class(
            function,
            instruction.dst,
            RegisterClass::ObjHandle,
            &format!("{path}.dst"),
            result,
        ),
        TacOp::Len { .. } => expect_dst_class(
            function,
            instruction.dst,
            RegisterClass::Int,
            &format!("{path}.dst"),
            result,
        ),
        TacOp::LoadTable {
            table, arguments, ..
        } => {
            if *table as usize >= tac.table_definitions.len() {
                push_error(
                    result,
                    format!("{path}.table"),
                    format!("表定义 {} 越界", table),
                );
            }
            reject_expanded_arguments(arguments, &path, result);
            expect_dst_class(
                function,
                instruction.dst,
                RegisterClass::ObjHandle,
                &format!("{path}.dst"),
                result,
            );
        }
        TacOp::Call { .. } => validate_call(tac, function, instruction, &path, result),
        TacOp::CallDynamic { arguments, .. } => reject_expanded_arguments(arguments, &path, result),
        TacOp::CallSub { sub } => {
            if !function
                .handlers
                .iter()
                .any(|handler| handler.handler == *sub && handler.exit == "finally")
            {
                push_error(
                    result,
                    format!("{path}.sub"),
                    format!("CallSub 目标 {} 不是 finally 子程序入口", sub.get()),
                );
            }
        }
        TacOp::BranchIf { condition, .. } => {
            expect_class(
                function,
                *condition,
                RegisterClass::Bool,
                &format!("{path}.condition"),
                result,
            );
        }
        _ => {}
    }
    validate_instruction_references(tac, function, &instruction.op, &path, result);
}

/// 检查指令中除类别和跳转外的跨表引用与退出边。
fn validate_instruction_references(
    tac: &TacProgram,
    function: &TacFunction,
    op: &TacOp,
    path: &str,
    result: &mut TacVerification,
) {
    match op {
        TacOp::Call {
            callee, signature, ..
        } => {
            check_function_reference(tac, *callee, &format!("{path}.callee"), result);
            check_signature_reference(tac, *signature, &format!("{path}.signature"), result);
        }
        TacOp::Check {
            value,
            on_failure,
            expected,
            ..
        } => {
            check_register(function, *value, &format!("{path}.value"), result);
            check_block_reference(function, *on_failure, &format!("{path}.on_failure"), result);
            if let Some(expected) = expected {
                expect_class(
                    function,
                    *value,
                    class_of_type(expected),
                    &format!("{path}.value"),
                    result,
                );
            }
        }
        TacOp::RunReleasePlan { scope, exit } => {
            if ExitKind::from_name(exit).is_none() {
                push_error(result, format!("{path}.exit"), "释放计划退出边不是冻结拼写");
            }
            if !tac
                .plans
                .iter()
                .any(|plan| plan.scope == *scope && plan.exit == *exit)
            {
                push_error(
                    result,
                    format!("{path}.release_plan"),
                    format!("释放计划 ({scope}, {exit}) 不存在"),
                );
            }
        }
        TacOp::ExitScope { scope, exit } => {
            if !function.scopes.is_empty() && !function.scopes.contains(scope) {
                push_error(result, format!("{path}.scope"), "退出了未登记的作用域");
            }
            if ExitKind::from_name(exit).is_none() {
                push_error(result, format!("{path}.exit"), "作用域退出边不是冻结拼写");
            }
        }
        TacOp::EnterScope(scope) => {
            if !function.scopes.is_empty() && !function.scopes.contains(scope) {
                push_error(result, format!("{path}.scope"), "进入了未登记的作用域");
            }
        }
        TacOp::SelectorApply { plan, .. } => {
            if *plan as usize >= tac.selection_plans.len() {
                push_error(result, format!("{path}.plan"), "选择计划索引越界");
            }
        }
        TacOp::BroadcastAssign { plan, .. } => {
            if *plan as usize >= tac.broadcast_assignment_plans.len() {
                push_error(result, format!("{path}.plan"), "广播计划索引越界");
            }
        }
        TacOp::RandomSeed { plan, .. } if *plan as usize >= tac.random_seed_plans.len() => {
            push_error(result, format!("{path}.plan"), "随机种子计划索引越界");
        }
        _ => {}
    }
}

/// 校验一个静态调用的实参数量、类别、关键字和签名配对。
fn validate_call(
    tac: &TacProgram,
    caller: &TacFunction,
    instruction: &TacInstr,
    path: &str,
    result: &mut TacVerification,
) {
    let TacOp::Call {
        callee,
        signature: signature_id,
        arguments,
    } = &instruction.op
    else {
        return;
    };
    let Some(signature) = tac.signatures.get(*signature_id) else {
        return;
    };
    let Some(target) = tac.functions.get(callee.get() as usize) else {
        return;
    };
    if target.signature != Some(*signature_id) {
        push_error(
            result,
            format!("{path}.signature"),
            "Call 携带的签名不是被调函数的签名",
        );
    }
    if is_dynamic_signature(signature) {
        reject_expanded_arguments(arguments, path, result);
        return;
    }
    let count = signature.parameter_types.len();
    let mut assigned = vec![false; count];
    let mut next_positional = 0_usize;
    let mut saw_keyword = false;
    for (argument_index, argument) in arguments.iter().enumerate() {
        if matches!(argument.kind, ArgKind::VarArgs | ArgKind::KwArgs) {
            push_error(
                result,
                format!("{path}.arguments[{argument_index}].kind"),
                "生产调用帧不接受 *args/**kwargs 展开实参",
            );
            continue;
        }
        let slot = match argument.kind {
            ArgKind::Positional => {
                if saw_keyword {
                    push_error(
                        result,
                        format!("{path}.arguments[{argument_index}]"),
                        "关键字实参之后不能再出现位置实参",
                    );
                }
                let slot = next_positional;
                next_positional = next_positional.saturating_add(1);
                if slot >= count
                    || matches!(signature.parameter_kinds[slot], ParamKind::KeywordOnly)
                {
                    if signature.var_args_slot.is_none() {
                        push_error(
                            result,
                            format!("{path}.arguments[{argument_index}]"),
                            "位置实参数量超过调用签名",
                        );
                    }
                    None
                } else {
                    Some(slot)
                }
            }
            ArgKind::Keyword => {
                saw_keyword = true;
                let Some(name) = argument.name.as_deref() else {
                    push_error(
                        result,
                        format!("{path}.arguments[{argument_index}].name"),
                        "关键字实参缺少名称",
                    );
                    continue;
                };
                let Some(slot) = signature
                    .parameter_names
                    .iter()
                    .position(|parameter| parameter == name)
                else {
                    if signature.kw_args_slot.is_none() {
                        push_error(
                            result,
                            format!("{path}.arguments[{argument_index}].name"),
                            format!("关键字 {name} 不在调用签名中"),
                        );
                    }
                    continue;
                };
                if !signature.parameter_kinds[slot].accepts_keyword() {
                    push_error(
                        result,
                        format!("{path}.arguments[{argument_index}].name"),
                        format!("形参 {name} 不接受关键字实参"),
                    );
                }
                Some(slot)
            }
            ArgKind::VarArgs | ArgKind::KwArgs => None,
        };
        let Some(slot) = slot else {
            continue;
        };
        if assigned[slot] {
            push_error(
                result,
                format!("{path}.arguments[{argument_index}]"),
                "同一形参收到多个实参",
            );
            continue;
        }
        assigned[slot] = true;
        expect_class(
            caller,
            argument.value,
            class_of_type(&signature.parameter_types[slot]),
            &format!("{path}.arguments[{argument_index}].value"),
            result,
        );
    }
    for (slot, was_assigned) in assigned.iter().enumerate() {
        if *was_assigned
            || signature.has_defaults[slot]
            || matches!(
                signature.parameter_kinds[slot],
                ParamKind::VarArgs | ParamKind::VarKeywords
            )
        {
            continue;
        }
        push_error(
            result,
            format!("{path}.arguments"),
            format!("缺少必需形参 {}", signature.parameter_names[slot]),
        );
    }
    if let Some(dst) = instruction.dst {
        expect_class(
            caller,
            dst,
            class_of_type(&signature.return_type),
            &format!("{path}.dst"),
            result,
        );
    }
}

/// 拒绝明确标记的展开实参。
fn reject_expanded_arguments(
    arguments: &[crate::tac::TacArgument],
    path: &str,
    result: &mut TacVerification,
) {
    for (index, argument) in arguments.iter().enumerate() {
        if matches!(argument.kind, ArgKind::VarArgs | ArgKind::KwArgs) {
            push_error(
                result,
                format!("{path}.arguments[{index}].kind"),
                "生产调用帧不接受 *args/**kwargs 展开实参",
            );
        }
    }
}

/// 验证 CallSub/RetFromSub 的静态配对。
fn validate_subroutines(
    function_index: usize,
    function: &TacFunction,
    result: &mut TacVerification,
) {
    let finally_targets = function
        .handlers
        .iter()
        .filter(|handler| handler.exit == "finally")
        .map(|handler| handler.handler)
        .collect::<BTreeSet<_>>();
    let mut reachable_returns = BTreeSet::new();
    for target in &finally_targets {
        let mut queue = VecDeque::from([*target]);
        let mut seen = BTreeSet::new();
        let mut has_return = false;
        while let Some(block_id) = queue.pop_front() {
            if !seen.insert(block_id) {
                continue;
            }
            let Some(block) = function.block(block_id) else {
                continue;
            };
            for instruction in &block.instructions {
                if matches!(instruction.op, TacOp::RetFromSub) {
                    has_return = true;
                    reachable_returns.insert(block_id);
                }
                for successor in jump_targets(&instruction.op) {
                    // 子程序跳回入口之前的块会把控制权交还给调用点，不能继续
                    // 当作子程序正文遍历。
                    if successor >= *target {
                        queue.push_back(successor);
                    }
                }
            }
        }
        if !has_return {
            push_error(
                result,
                format!("functions[{function_index}].handlers"),
                format!("finally 子程序 {} 没有可达 RetFromSub", target.get()),
            );
        }
    }
    for (block_index, block) in function.blocks.iter().enumerate() {
        for instruction in &block.instructions {
            if let TacOp::CallSub { sub } = instruction.op
                && !finally_targets.contains(&sub)
            {
                push_error(
                    result,
                    format!("functions[{function_index}].blocks[{block_index}]"),
                    format!("CallSub {} 没有对应 finally handler", sub.get()),
                );
            }
            if matches!(instruction.op, TacOp::RetFromSub) && !reachable_returns.contains(&block.id)
            {
                push_error(
                    result,
                    format!("functions[{function_index}].blocks[{block_index}]"),
                    "RetFromSub 不在任何 finally 子程序的可达正文中",
                );
            }
        }
    }
}

/// 检查一个函数编号引用。
fn check_function_reference(
    tac: &TacProgram,
    id: crate::tac::FuncId,
    path: &str,
    result: &mut TacVerification,
) {
    if id.get() as usize >= tac.functions.len() {
        push_error(result, path, format!("函数 {} 越界", id.get()));
    }
}

/// 检查一个签名编号引用。
fn check_signature_reference(
    tac: &TacProgram,
    id: crate::tac::SigId,
    path: &str,
    result: &mut TacVerification,
) {
    if tac.signatures.get(id).is_none() {
        push_error(result, path, format!("签名 {} 越界", id.get()));
    }
}

/// 检查一个函数内基本块引用。
fn check_block_reference(
    function: &TacFunction,
    id: BlockId,
    path: &str,
    result: &mut TacVerification,
) {
    if function.block(id).is_none() {
        push_error(result, path, format!("基本块 {} 越界", id.get()));
    }
}

/// 检查寄存器是否有函数局部类别。
fn check_register(
    function: &TacFunction,
    register: VReg,
    path: &str,
    result: &mut TacVerification,
) {
    if !category_is_registered(&function.categories, register) {
        push_error(
            result,
            path,
            format!("寄存器 {} 没有函数局部类别", register.get()),
        );
    }
}

/// 检查一个寄存器的类别是否满足预期；Dynamic/Poly 是保守兼容类别。
fn expect_class(
    function: &TacFunction,
    register: VReg,
    expected: RegisterClass,
    path: &str,
    result: &mut TacVerification,
) {
    let Some(actual) = category_of(&function.categories, register) else {
        return;
    };
    if !classes_compatible(actual, expected) {
        push_error(
            result,
            path,
            format!(
                "寄存器 {} 的类别 {} 与期望 {} 不一致",
                register.get(),
                actual.as_name(),
                expected.as_name()
            ),
        );
    }
}

/// 对有结果指令执行类别检查；无结果指令不在这里强制补寄存器。
fn expect_dst_class(
    function: &TacFunction,
    dst: Option<VReg>,
    expected: RegisterClass,
    path: &str,
    result: &mut TacVerification,
) {
    if let Some(dst) = dst {
        expect_class(function, dst, expected, path, result);
    }
}

/// 判断类别表是否覆盖寄存器编号。
fn category_is_registered(categories: &CategoryMap, register: VReg) -> bool {
    (register.get() as usize) < categories.len()
}

/// 查询已登记类别。
fn category_of(categories: &CategoryMap, register: VReg) -> Option<RegisterClass> {
    category_is_registered(categories, register).then(|| categories.get(register))
}

/// 把 IR 类型映射成冻结的寄存器类别。
fn class_of_type(ty: &IrType) -> RegisterClass {
    match ty {
        IrType::Scalar { name } => match ScalarType::from_name(name) {
            Some(ScalarType::Int | ScalarType::Sint) => RegisterClass::Int,
            Some(ScalarType::Float | ScalarType::Sfloat) => RegisterClass::Float,
            Some(ScalarType::Bool) => RegisterClass::Bool,
            Some(ScalarType::Str | ScalarType::Lint | ScalarType::Lfloat) => {
                RegisterClass::ObjHandle
            }
            None => RegisterClass::Poly,
        },
        IrType::None => RegisterClass::None,
        IrType::Dynamic | IrType::Variable { .. } => RegisterClass::Dynamic,
        IrType::Function { .. } => RegisterClass::ObjHandle,
        _ => RegisterClass::ObjHandle,
    }
}

/// 把标量类型映射成寄存器类别。
fn class_of_scalar(scalar: ScalarType) -> RegisterClass {
    match scalar {
        ScalarType::Int | ScalarType::Sint => RegisterClass::Int,
        ScalarType::Float | ScalarType::Sfloat => RegisterClass::Float,
        ScalarType::Bool => RegisterClass::Bool,
        ScalarType::Str | ScalarType::Lint | ScalarType::Lfloat => RegisterClass::ObjHandle,
    }
}

/// 判断两个类别能否在冻结 ABI 边界相互传递。
fn classes_compatible(actual: RegisterClass, expected: RegisterClass) -> bool {
    actual == expected
        || matches!(actual, RegisterClass::Dynamic | RegisterClass::Poly)
        || matches!(expected, RegisterClass::Dynamic | RegisterClass::Poly)
}

/// 判断签名是否为全动态调用签名。
fn is_dynamic_signature(signature: &CallSig) -> bool {
    signature.parameter_names.is_empty()
        && signature.parameter_kinds.is_empty()
        && signature.parameter_types.is_empty()
        && signature.has_defaults.is_empty()
        && signature.var_args_slot.is_none()
        && signature.kw_args_slot.is_none()
        && signature.return_type == IrType::Dynamic
}

/// 把 TAC 键转成释放计划观测，供 xiao-ir 对账入口消费。
fn observed_plans(tac: &TacProgram) -> Vec<ObservedRelease> {
    let mut observed = Vec::new();
    for function in &tac.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let TacOp::RunReleasePlan { scope, exit } = &instruction.op else {
                    continue;
                };
                let Some(plan) = tac
                    .plans
                    .iter()
                    .find(|plan| plan.scope == *scope && plan.exit == *exit)
                else {
                    continue;
                };
                if observed
                    .iter()
                    .any(|item: &ObservedRelease| item.scope == *scope && item.exit == *exit)
                {
                    continue;
                }
                observed.push(ObservedRelease::new(
                    *scope,
                    exit.clone(),
                    plan.actions
                        .iter()
                        .map(|action| xiao_ir::IrReleaseAction {
                            value: action.value,
                            order: action.order,
                            kind: action.kind.as_name().to_owned(),
                        })
                        .collect(),
                ));
            }
        }
    }
    observed
}

/// 把普通字符串错误追加到验证结果。
fn push_error(result: &mut TacVerification, path: impl Into<String>, message: impl Into<String>) {
    result
        .errors
        .push(format!("{}: {}", path.into(), message.into()));
}

#[cfg(test)]
/// 生产验证器的结构和内部一致性回归。
mod tests {
    use super::*;
    use crate::{
        CallSigTable, ConstPool, TacAbi, TacBlock, TacConstant, TacFunction, TacInstr, TacOp,
        TacProgram,
    };
    use std::collections::BTreeMap;
    use xiao_ir::{IR_VERSION, IrEntryMode, IrSpan};

    /// 构造最小的合法脚本 TAC。
    fn empty_program() -> (IrProgram, TacProgram) {
        let ir = IrProgram::new(IrEntryMode::Script, Vec::new(), IrSpan::new(0, 0));
        let function = TacFunction {
            name: String::new(),
            signature: None,
            entry: BlockId::new(0),
            blocks: vec![TacBlock {
                id: BlockId::new(0),
                scope: 0,
                instructions: vec![TacInstr::new(
                    TacOp::Return { value: None },
                    IrSpan::new(0, 0),
                )],
            }],
            parameters: Vec::new(),
            locals: Vec::new(),
            categories: CategoryMap::new(),
            scopes: vec![0],
            handlers: Vec::new(),
            value_registers: BTreeMap::new(),
            span: IrSpan::new(0, 0),
        };
        let tac = TacProgram {
            version: crate::TAC_VERSION,
            abi: TacAbi {
                bytecode_abi_version: crate::TAC_BYTECODE_ABI_VERSION,
                runtime_abi_version: crate::TAC_RUNTIME_ABI_VERSION,
                ir_version: IR_VERSION,
                language_version: String::new(),
                target: String::new(),
            },
            constants: ConstPool::new(),
            signatures: CallSigTable::new(),
            functions: vec![function],
            categories: CategoryMap::new(),
            plans: Vec::new(),
            selection_plans: Vec::new(),
            broadcast_assignment_plans: Vec::new(),
            random_seed_plans: Vec::new(),
            table_definitions: Vec::new(),
            unsupported: Vec::new(),
        };
        (ir, tac)
    }

    #[test]
    /// 非空 `unsupported` 必须在生产入口报告内部一致性错误。
    fn production_entry_rejects_unsupported_as_internal_error() {
        let (ir, mut tac) = empty_program();
        tac.unsupported.push("测试缺口".to_owned());
        let error = verify_production(&ir, &tac).expect_err("生产入口必须拒绝");
        assert_eq!(error.code, TAC_INTERNAL_CONSISTENCY_CODE);
        assert!(error.is_internal());
    }

    #[test]
    /// 程序级兼容类别视图不能污染函数局部类别校验。
    fn program_level_categories_are_not_used_for_function_validation() {
        let (ir, mut tac) = empty_program();
        let register = VReg::new(0);
        tac.functions[0]
            .categories
            .insert(register, RegisterClass::Int);
        let constant = tac.constants.intern(TacConstant::Int(1));
        tac.functions[0].blocks[0].instructions.insert(
            0,
            TacInstr::with_dst(TacOp::LoadConst(constant), register, IrSpan::new(0, 0)),
        );
        tac.categories.insert(register, RegisterClass::ObjHandle);
        assert!(verify_program(&ir, &tac).errors.is_empty());
    }
}
