//! 三类产物共享的并发 feature 推导；输入只有已验证 IR 类型与操作，不读取源码名称。

use crate::{IrProgram, IrType};
use xiao_runtime_abi::*;

/// 并发值表示本身所需的能力；当前 Runtime 身份不代表显式创建多 Runtime。
#[must_use]
pub fn concurrency_type_features(ty: &IrType) -> u64 {
    match ty {
        IrType::Thread { value } => REQUIRED_FEATURE_THREADS | concurrency_type_features(value),
        IrType::Task { value } => REQUIRED_FEATURE_TASK_POOL | concurrency_type_features(value),
        IrType::Pool => REQUIRED_FEATURE_TASK_POOL,
        IrType::Shared { value } | IrType::SharedMut { value } | IrType::LockGuard { value } => {
            REQUIRED_FEATURE_SHARED_HANDLES | concurrency_type_features(value)
        }
        IrType::Function {
            parameters,
            return_type,
        } => parameters
            .iter()
            .fold(concurrency_type_features(return_type), |all, ty| {
                all | concurrency_type_features(ty)
            }),
        IrType::Tuple { elements }
        | IrType::Set {
            members: elements, ..
        } => elements
            .iter()
            .fold(0, |all, ty| all | concurrency_type_features(ty)),
        IrType::Array { shape } => match shape {
            crate::IrArrayShape::Homogeneous { element, .. } => concurrency_type_features(element),
            crate::IrArrayShape::Heterogeneous { elements } => elements
                .iter()
                .fold(0, |all, ty| all | concurrency_type_features(ty)),
            crate::IrArrayShape::Unknown => 0,
        },
        IrType::DictTable { entries } | IrType::DictColumn { entries } => {
            entries.iter().fold(0, |all, entry| {
                all | concurrency_type_features(&entry.value)
            })
        }
        _ => 0,
    }
}

/// 从同源 IR 的操作和所有权类型取最小并集；未知操作返回错误，不能猜测全置位。
pub fn required_concurrency_features(program: &IrProgram) -> Result<u64, String> {
    let mut features = 0;
    for call in &program.concurrency_calls {
        features |= concurrency_operation_features(&call.operation)
            .ok_or_else(|| format!("未知并发操作 {}", call.operation))?;
        features |= concurrency_type_features(&call.result_type);
        for ty in &call.argument_types {
            features |= concurrency_type_features(ty);
        }
    }
    for value in &program.ownership.values {
        if let Some(ty) = &value.ty {
            features |= concurrency_type_features(ty);
        }
    }
    Ok(features)
}
