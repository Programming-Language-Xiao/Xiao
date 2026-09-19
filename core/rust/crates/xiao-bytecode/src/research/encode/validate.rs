//! 编码输入的版本、签名和跨表引用校验。

use super::codec::check_index_width;
use super::tags::release_tag;
use super::{
    BlockId, CallSig, ConstId, EncodeError, FuncId, OperandWidth, SigId, TAC_BYTECODE_ABI_VERSION,
    TAC_RUNTIME_ABI_VERSION, TAC_VERSION, TacFunction, TacInstr, TacOp, TacProgram, VReg,
};
use xiao_ir::IR_VERSION;
use xiao_lifetime::ExitKind;

/// 编码前的整体校验，也是编码器唯一的入口关卡。
///
/// 顺序是有意的：先卡版本字段（TAC、bytecode ABI、runtime ABI、IR，全部必须
/// 等于当前实现），再拒绝非空 `unsupported`——宁可整体失败，也不要产出一份
/// 「能解码但少算了一部分」的编码；最后才是签名自洽性与跨表引用。版本错误先
/// 于引用错误报出，因为版本不符时后面的编号根本没有可比对的基准。
///
/// `width` 必须由调用方传入真实的操作数宽度：定宽 `u16` 下部分索引可能放不下，
/// 传错宽度会让那条检查静默失效（见 [`check_index_width`]）。
pub(super) fn validate_input(program: &TacProgram, width: OperandWidth) -> Result<(), EncodeError> {
    if program.version != TAC_VERSION {
        return Err(EncodeError::VersionMismatch {
            field: "tac_version".to_owned(),
            expected: TAC_VERSION as u64,
            actual: program.version as u64,
        });
    }
    check_version(
        "bytecode_abi_version",
        program.abi.bytecode_abi_version,
        TAC_BYTECODE_ABI_VERSION,
    )?;
    check_version(
        "runtime_abi_version",
        program.abi.runtime_abi_version,
        TAC_RUNTIME_ABI_VERSION,
    )?;
    check_version("ir_version", program.abi.ir_version, IR_VERSION)?;
    if let Some(note) = program.unsupported.first() {
        return Err(EncodeError::Unsupported(note.clone()));
    }
    for signature in program.signatures.iter() {
        validate_signature(signature)?;
    }
    validate_references(program, width)
}

/// 比对单个版本字段，不一致时报出字段名与两侧取值。
///
/// 抽出来是为了让编码入口与解码入口共用同一条判定；四处版本字段各写一遍
/// `if` 很容易漏掉其中一处。
pub(super) fn check_version(field: &str, actual: u32, expected: u32) -> Result<(), EncodeError> {
    if actual != expected {
        return Err(EncodeError::VersionMismatch {
            field: field.to_owned(),
            expected: expected as u64,
            actual: actual as u64,
        });
    }
    Ok(())
}

/// 检查一条签名自洽：四条平行数组等长，且两个变参槽位指向真实存在的形参。
///
/// 槽位越界不会在当前编码里报错（它只是个编号），但运行期会按槽位去被调方帧
/// 取寄存器，取到的是别人的值——所以必须在编码期挡住。
pub(super) fn validate_signature(signature: &CallSig) -> Result<(), EncodeError> {
    let lengths = [
        signature.parameter_names.len(),
        signature.parameter_kinds.len(),
        signature.parameter_types.len(),
        signature.has_defaults.len(),
    ];
    if lengths.iter().any(|length| *length != lengths[0]) {
        return Err(EncodeError::InvalidFormat(
            "调用签名的参数平行数组长度不一致".to_owned(),
        ));
    }
    for (field, slot) in [
        ("var_args_slot", signature.var_args_slot),
        ("kw_args_slot", signature.kw_args_slot),
    ] {
        if let Some(slot) = slot {
            if slot >= lengths[0] {
                return Err(EncodeError::InvalidReference {
                    kind: field.to_owned(),
                    index: slot as u64,
                    limit: lengths[0],
                });
            }
        }
    }
    Ok(())
}

/// 检查所有「编码时会直接下标」的跨表引用。
///
/// 覆盖的范围比看起来大，因为它同时守住了两条结构性不变量：
///
/// - 块目录必须严格按 `BlockId` 递增排列（`blocks[i].id == i`）。编码端按位置
///   写块，handler 的物理 pc 又靠 `block_pcs[块号]` 直接下标取，一旦目录错位，
///   异常路由会静默指到别的块。
/// - handler 的保护区间必须非倒置，且终点允许等于块数（排他上界可以越过最后
///   一块，落到函数尾部）。
///
/// 这里也顺带过一遍释放动作的类别与所有退出边名称，因为这两者在编码端被当作
/// 「一定能映射到标签」使用。
fn validate_references(program: &TacProgram, width: OperandWidth) -> Result<(), EncodeError> {
    for plan in &program.plans {
        check_exit_name(&plan.exit)?;
        for action in &plan.actions {
            let _ = release_tag(action.kind)?;
        }
    }
    for (function_index, function) in program.functions.iter().enumerate() {
        if let Some(signature) = function.signature {
            check_sig(program, signature)?;
        }
        if function.entry.get() as usize >= function.blocks.len() {
            return Err(EncodeError::InvalidReference {
                kind: format!("function[{function_index}].entry"),
                index: function.entry.get() as u64,
                limit: function.blocks.len(),
            });
        }
        for (block_index, block) in function.blocks.iter().enumerate() {
            if block.id.get() as usize != block_index {
                return Err(EncodeError::InvalidFormat(format!(
                    "函数 {function_index} 的块目录不是按 BlockId 排列：位置 {block_index} 是 {}",
                    block.id.get()
                )));
            }
            for instruction in &block.instructions {
                validate_op(program, function, instruction, function_index)?;
            }
        }
        for handler in &function.handlers {
            for (name, id) in [
                ("protected.start", handler.protected.0),
                ("handler", handler.handler),
            ] {
                check_block(
                    function,
                    id,
                    format!("function[{function_index}].handler.{name}"),
                )?;
            }
            if handler.protected.1.get() as usize > function.blocks.len() {
                return Err(EncodeError::InvalidReference {
                    kind: format!("function[{function_index}].handler.protected.end"),
                    index: handler.protected.1.get() as u64,
                    limit: function.blocks.len() + 1,
                });
            }
            if handler.protected.0.get() > handler.protected.1.get() {
                return Err(EncodeError::InvalidFormat(format!(
                    "函数 {function_index} 的 handler 保护区间倒置"
                )));
            }
            check_exit_name(&handler.exit)?;
            if let Some(binding) = handler.binding {
                check_index_width(binding.get() as u64, "handler.binding", width)?;
            }
        }
    }
    Ok(())
}

/// 检查一条指令引用到的表项都在范围内。
///
/// 校验分三类：常量池（`LoadConst`）、函数表（`LoadFunc`、`Call`）、签名表
/// （`Call`），以及本函数的块目录（跳转、分支、`on_failure`、`Check`）。
/// `RunReleasePlan` 另有一条更强的约束：`(scope, exit)` 必须**成对**出现在
/// 释放计划表里，只对上作用域是没用的，解释器按这一对取计划。
///
/// [`VReg`] 编号**没有可校验的上界**：TAC 没有独立的寄存器表，编号空间与
/// [`CategoryMap`] 的稠密长度不是同一回事（未登记的编号只是默认为 `Poly`，
/// 合法）。因此 `check_vreg` 是一个空闭包——它在每个用到寄存器的分支上保留了
/// 调用位置，等将来有了真正的寄存器表再往里填检查即可。寄存器号唯一的硬约束
/// 是定宽模式下的 `u16` 上界，那由 `Writer::index` 在写出时兜住。
fn validate_op(
    program: &TacProgram,
    function: &TacFunction,
    instruction: &TacInstr,
    function_index: usize,
) -> Result<(), EncodeError> {
    let check_vreg = |register: VReg| {
        let _ = function;
        let _ = register;
        Ok::<(), EncodeError>(())
    };
    let check_block_ref =
        |block: BlockId| check_block(function, block, "instruction.block".to_owned());
    match &instruction.op {
        TacOp::LoadConst(id) => check_const(program, *id),
        TacOp::LoadFunc(id) => check_func(program, *id),
        TacOp::Move(value)
        | TacOp::Copy(value)
        | TacOp::Box(value)
        | TacOp::Unbox(value)
        | TacOp::Raise { value }
        | TacOp::Transfer { value } => check_vreg(*value),
        TacOp::Cast { value, .. } => check_vreg(*value),
        TacOp::Arith { left, right, .. }
        | TacOp::Compare { left, right, .. }
        | TacOp::SetOp { left, right, .. }
        | TacOp::SetCompare { left, right, .. } => {
            check_vreg(*left)?;
            check_vreg(*right)
        }
        TacOp::NewArray { elements }
        | TacOp::NewTuple { elements }
        | TacOp::NewSet { elements } => {
            for value in elements {
                check_vreg(*value)?;
            }
            Ok(())
        }
        TacOp::NewDictTable { entries } | TacOp::NewDictColumn { entries } => {
            for (_, value) in entries {
                check_vreg(*value)?;
            }
            Ok(())
        }
        TacOp::IndexGet { source, .. } => check_vreg(*source),
        TacOp::SelectorApply {
            source,
            plan,
            step,
            random_counts,
        } => {
            if *plan as usize >= program.selection_plans.len() {
                return Err(EncodeError::InvalidReference {
                    kind: format!("function[{function_index}].selection_plan"),
                    index: *plan as u64,
                    limit: program.selection_plans.len(),
                });
            }
            check_vreg(*source)?;
            step.map_or(Ok(()), check_vreg)?;
            for value in random_counts.iter().flatten() {
                check_vreg(*value)?;
            }
            Ok(())
        }
        TacOp::BroadcastAssign { root, value, plan } => {
            if *plan as usize >= program.broadcast_assignment_plans.len() {
                return Err(EncodeError::InvalidReference {
                    kind: format!("function[{function_index}].broadcast_plan"),
                    index: *plan as u64,
                    limit: program.broadcast_assignment_plans.len(),
                });
            }
            check_vreg(*root)?;
            check_vreg(*value)
        }
        TacOp::RandomSeed { value, plan } => {
            if *plan as usize >= program.random_seed_plans.len() {
                return Err(EncodeError::InvalidReference {
                    kind: format!("function[{function_index}].random_seed_plan"),
                    index: *plan as u64,
                    limit: program.random_seed_plans.len(),
                });
            }
            check_vreg(*value)
        }
        TacOp::Jump(target) | TacOp::CallSub { sub: target } => check_block_ref(*target),
        TacOp::BranchIf {
            condition,
            if_true,
            if_false,
        } => {
            check_vreg(*condition)?;
            check_block_ref(*if_true)?;
            check_block_ref(*if_false)
        }
        TacOp::Call {
            callee,
            signature,
            arguments,
        } => {
            check_func(program, *callee)?;
            check_sig(program, *signature)?;
            for argument in arguments {
                check_vreg(argument.value)?;
            }
            Ok(())
        }
        TacOp::CallDynamic { callee, arguments } => {
            check_vreg(*callee)?;
            for argument in arguments {
                check_vreg(argument.value)?;
            }
            Ok(())
        }
        TacOp::Return { value } => value.map_or(Ok(()), check_vreg),
        TacOp::MakeError { code, message, .. } => {
            code.map_or(Ok(()), check_vreg)?;
            message.map_or(Ok(()), check_vreg)
        }
        TacOp::RetFromSub | TacOp::LoadNone => Ok(()),
        TacOp::Check {
            value, on_failure, ..
        } => {
            check_vreg(*value)?;
            check_block_ref(*on_failure)
        }
        TacOp::Release { value, kind } => {
            let _ = release_tag(*kind)?;
            check_vreg(*value)
        }
        TacOp::RunReleasePlan { scope, exit } => {
            check_exit_name(exit)?;
            if !program
                .plans
                .iter()
                .any(|plan| plan.scope == *scope && plan.exit == *exit)
            {
                return Err(EncodeError::InvalidReference {
                    kind: format!("function[{function_index}].release_plan"),
                    index: *scope as u64,
                    limit: program.plans.len(),
                });
            }
            Ok(())
        }
        TacOp::EnterScope(_) => Ok(()),
        TacOp::ExitScope { exit, .. } => check_exit_name(exit),
    }
}

/// 检查退出边名称是 `ExitKind` 的稳定拼写。
///
/// 退出边名由 `xiao-lifetime` 冻结，编码按名字写字符串而不是编码成编号，所以
/// 校验只能走 `ExitKind::from_name`。诊断里的 `value` 固定填 0：出错的是「名字
/// 不认识」，没有一个可报的数字输入，字段名才是有效信息。
fn check_exit_name(name: &str) -> Result<(), EncodeError> {
    if ExitKind::from_name(name).is_none() {
        return Err(EncodeError::InvalidEnum {
            field: "ExitKind".to_owned(),
            value: 0,
        });
    }
    Ok(())
}

/// 检查 `ConstId` 落在常量池内。
fn check_const(program: &TacProgram, id: ConstId) -> Result<(), EncodeError> {
    if id.get() as usize >= program.constants.len() {
        return Err(EncodeError::InvalidReference {
            kind: "ConstId".to_owned(),
            index: id.get() as u64,
            limit: program.constants.len(),
        });
    }
    Ok(())
}

/// 检查 `FuncId` 落在函数表内。
///
/// 索引 0 是脚本入口，它可以被 `LoadFunc`/`Call` 正常引用（递归入口是合法的），
/// 所以这里只卡上界，不排除任何编号。
fn check_func(program: &TacProgram, id: FuncId) -> Result<(), EncodeError> {
    if id.get() as usize >= program.functions.len() {
        return Err(EncodeError::InvalidReference {
            kind: "FuncId".to_owned(),
            index: id.get() as u64,
            limit: program.functions.len(),
        });
    }
    Ok(())
}

/// 检查 `SigId` 落在签名表内。
///
/// 签名的变长平行数组不走这里，而由 [`validate_signature`] 单独负责：这里只
/// 回答「这个编号有没有指向一条存在的签名」。
fn check_sig(program: &TacProgram, id: SigId) -> Result<(), EncodeError> {
    if id.get() as usize >= program.signatures.len() {
        return Err(EncodeError::InvalidReference {
            kind: "SigId".to_owned(),
            index: id.get() as u64,
            limit: program.signatures.len(),
        });
    }
    Ok(())
}

/// 检查块号落在**本函数**的块目录内。
///
/// 块号是函数内编号空间，跨函数比较没有意义，所以基准取 `function` 而不是
/// `program`。`kind` 由调用方给出（跳转目标、`on_failure`、handler 区间……），
/// 直接进诊断，用来区分「哪一处引用越界」——同类错误在一条指令里可能出现多次，
/// 没有这个上下文就只能靠数编号。
fn check_block(function: &TacFunction, id: BlockId, kind: String) -> Result<(), EncodeError> {
    if id.get() as usize >= function.blocks.len() {
        return Err(EncodeError::InvalidReference {
            kind,
            index: id.get() as u64,
            limit: function.blocks.len(),
        });
    }
    Ok(())
}
