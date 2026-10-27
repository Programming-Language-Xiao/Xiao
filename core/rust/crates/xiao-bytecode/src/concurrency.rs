//! 并发计划的格式不变量；源码运行、内存解码和产物读取共享此关卡。

use crate::{TacConcurrencyPlan, TacOp, TacProgram};
use xiao_ir::IrType;

/// 一项并发操作要求的 `.xiaoc` 功能位。
pub fn required_features(operation: &str) -> u64 {
    use xiao_runtime_abi::*;
    match operation {
        "spawn" | "join" | "detach" => {
            XIAOC_REQUIRED_FEATURE_THREADS
                | XIAOC_REQUIRED_FEATURE_SERIALIZATION_V1
                | XIAOC_REQUIRED_FEATURE_CANCEL_WAIT
        }
        "pool_create" | "submit" | "wait" | "shutdown" => {
            XIAOC_REQUIRED_FEATURE_TASK_POOL
                | XIAOC_REQUIRED_FEATURE_SERIALIZATION_V1
                | XIAOC_REQUIRED_FEATURE_CANCEL_WAIT
        }
        "shared_read" | "shared_mut" | "read" | "lock_read" | "lock_write" => {
            XIAOC_REQUIRED_FEATURE_SHARED_HANDLES
        }
        "lock" => XIAOC_REQUIRED_FEATURE_SHARED_HANDLES | XIAOC_REQUIRED_FEATURE_CANCEL_WAIT,
        "cancel" => XIAOC_REQUIRED_FEATURE_CANCEL_WAIT,
        "runtime_create" | "runtime_close" | "current_runtime" => {
            XIAOC_REQUIRED_FEATURE_MULTI_RUNTIME
        }
        _ => 0,
    }
}

/// 校验计划、调用引用、参数形状以及 22B 复制/取消事实。
pub fn validate(program: &TacProgram) -> Result<(), String> {
    for plan in &program.concurrency_plans {
        validate_plan(plan)?;
    }
    for function in &program.functions {
        for instruction in function.blocks.iter().flat_map(|block| &block.instructions) {
            if let TacOp::CallConcurrency { plan, arguments } = &instruction.op {
                let entry = plan
                    .checked_sub(1)
                    .and_then(|id| program.concurrency_plans.get(id as usize))
                    .ok_or("并发计划编号越界")?;
                if arguments.len() != entry.argument_types.len() || instruction.span != entry.span {
                    return Err("并发指令与计划的参数数量/区间不一致".to_owned());
                }
                if arguments
                    .iter()
                    .any(|arg| arg.kind != crate::tac::ArgKind::Positional || arg.name.is_some())
                {
                    return Err("并发调用只接受已验证的位置实参".to_owned());
                }
                for (argument, ty) in arguments.iter().zip(&entry.argument_types) {
                    let expected = crate::verify::class_of_type(ty);
                    let actual = function.categories.get(argument.value);
                    if actual != expected
                        && !matches!(
                            actual,
                            crate::tac::RegisterClass::Poly | crate::tac::RegisterClass::Dynamic
                        )
                    {
                        return Err("并发实参寄存器类别不符".to_owned());
                    }
                }
            }
        }
    }
    Ok(())
}

/// 校验并发计划的签名、边界类型、Runtime 身份和取消事实。
fn validate_plan(plan: &TacConcurrencyPlan) -> Result<(), String> {
    let args = &plan.argument_types;
    let output = &plan.result_type;
    let one = || args.len() == 1;
    let shape = match plan.operation.as_str() {
        "runtime_create" | "current_runtime" => args.is_empty() && *output == IrType::Runtime,
        "runtime_close" => args == &[IrType::Runtime] && *output == IrType::None,
        "pool_create" => {
            args.len() == 2
                && args[0] == IrType::Runtime
                && matches!(&args[1], IrType::Scalar {name} if name == "int")
                && *output == IrType::Pool
        }
        "shutdown" => args == &[IrType::Pool] && *output == IrType::None,
        "cancel" => {
            one()
                && matches!(args[0], IrType::Thread { .. } | IrType::Task { .. })
                && *output == IrType::None
        }
        "join" => one() && matches!(&args[0], IrType::Thread { value } if **value == *output),
        "detach" => one() && matches!(&args[0], IrType::Thread { .. }) && *output == IrType::None,
        "wait" => one() && matches!(&args[0], IrType::Task { value } if **value == *output),
        "shared_read" => one() && matches!(output, IrType::Shared { value } if **value == args[0]),
        "shared_mut" => {
            one() && matches!(output, IrType::SharedMut { value } if **value == args[0])
        }
        "read" => one() && matches!(&args[0], IrType::Shared {value} if **value == *output),
        "lock" => {
            one()
                && matches!((&args[0], output), (IrType::SharedMut {value: a}, IrType::LockGuard {value: b}) if a == b)
        }
        "lock_read" => one() && matches!(&args[0], IrType::LockGuard {value} if **value == *output),
        "lock_write" => {
            args.len() == 2
                && matches!(&args[0], IrType::LockGuard {value} if **value == args[1])
                && *output == IrType::None
        }
        "spawn" | "submit" => {
            if args.len() < 2 || args.len() > 66 {
                false
            } else {
                let receiver = if plan.operation == "spawn" {
                    IrType::Runtime
                } else {
                    IrType::Pool
                };
                args[0] == receiver
                    && match (&args[1], output) {
                        (
                            IrType::Function {
                                parameters,
                                return_type,
                            },
                            IrType::Thread { value },
                        ) if plan.operation == "spawn" => {
                            parameters.as_slice() == &args[2..] && return_type == value
                        }
                        (
                            IrType::Function {
                                parameters,
                                return_type,
                            },
                            IrType::Task { value },
                        ) if plan.operation == "submit" => {
                            parameters.as_slice() == &args[2..] && return_type == value
                        }
                        _ => false,
                    }
            }
        }
        _ => false,
    };
    if !shape || args.len() != plan.argument_spans.len() {
        return Err(format!("{} 并发计划签名无效", plan.operation));
    }
    let boundary_types = if matches!(plan.operation.as_str(), "spawn" | "submit") {
        &args[2..]
    } else if matches!(plan.operation.as_str(), "shared_read" | "shared_mut") {
        args.as_slice()
    } else {
        &[]
    };
    for ty in boundary_types {
        let ty = xiao_ir::restore_type(ty).ok_or("边界类型不能还原")?;
        if !(xiao_types::is_send(&ty)
            || matches!(plan.operation.as_str(), "spawn" | "submit")
                && xiao_types::can_share_mutably(&ty))
        {
            return Err("并发计划携带不可发送的边界类型".to_owned());
        }
    }
    if plan.span.start > plan.span.end
        || plan
            .argument_spans
            .iter()
            .any(|s| s.start > s.end || s.start < plan.span.start || s.end > plan.span.end)
    {
        return Err("并发参数区间越界".to_owned());
    }
    if plan.runtime_id == Some(0) {
        return Err("静态 Runtime 标签不能是 0".to_owned());
    }
    let required_check = match plan.operation.as_str() {
        "spawn" | "submit" => Some("safe_point"),
        "join" | "wait" => Some("wait"),
        "lock" => Some("lock_wait"),
        _ => None,
    };
    if plan
        .cancellation_checks
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        != required_check.into_iter().collect::<Vec<_>>()
    {
        return Err("并发取消检查与操作不匹配".to_owned());
    }
    if let Some(transfer) = &plan.transfer {
        if !matches!(plan.operation.as_str(), "spawn" | "submit")
            || !matches!(transfer.copy_kind.as_str(), "deep_copy" | "serialization")
            || plan.runtime_id.is_none()
        {
            return Err("边界动作不完整或操作不匹配".to_owned());
        }
        if transfer.cleanup_order
            != transfer
                .parameters
                .iter()
                .rev()
                .copied()
                .collect::<Vec<_>>()
        {
            return Err("边界清理必须为参数逆序".to_owned());
        }
        if transfer.parameters.len() != args.len() - 2 {
            return Err("边界参数与复制计划数量不一致".to_owned());
        }
        let facts = plan.facts.iter().map(|f| f.value).collect::<Vec<_>>();
        if facts
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != facts.len()
        {
            return Err("边界值身份重复".to_owned());
        }
        if facts != transfer.parameters
            || plan.facts.iter().any(|f| {
                Some(f.runtime_id) != plan.runtime_id
                    || f.runtime_id == 0
                    || !f.task_boundary
                    || !f.thread_boundary
                    || f.span != plan.span
                    || f.source_runtime_id == Some(0)
            })
        {
            return Err("边界事实与动作不一致".to_owned());
        }
    } else if !plan.facts.is_empty()
        || matches!(plan.operation.as_str(), "spawn" | "submit") && args.len() > 2
    {
        return Err("边界值缺少复制动作或事实".to_owned());
    }
    Ok(())
}
