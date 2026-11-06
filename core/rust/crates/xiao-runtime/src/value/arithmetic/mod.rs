//! B2-A精确算术与A6有界十进制加减；公开转换由独立模块承接。

mod common;
mod comparison;
mod decimal;
mod decimal_power;
mod exact_division;
mod float_lift;
mod integer;
mod modulus;
mod power;
mod quotient;
/// A6按冻结十进制对齐模型执行有界线性加减。
mod text_add;

use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use std::cmp::Ordering;

/// 内核运算身份，不分配intrinsic或C ABI编号。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
    FloorDivide,
    Remainder,
    Power,
}

impl Operation {
    /// 只用于现有结构化错误的算子参数。
    pub fn name(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::FloorDivide => "//",
            Self::Remainder => "%",
            Self::Power => "**",
        }
    }
}

/// 涉及高精度值的调用统一在这里检查类型；定宽旧路径仍由后端先提升。
#[inline(always)]
pub(super) fn binary(
    op: Operation,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> Option<RuntimeResult<RuntimeValue>> {
    if !is_extended(left) && !is_extended(right) {
        return None;
    }
    Some(extended_binary(op, left, right))
}

/// 高精度实际分派留在独立函数，定宽热路径只内联两个标签检查。
fn extended_binary(
    op: Operation,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    if !is_numeric(left) || !is_numeric(right) {
        return Err(RuntimeError::type_mismatch(
            "numeric",
            format!("{} / {}", left.type_name(), right.type_name()),
        ));
    }
    if let Some(result) = text_add::binary(op, left, right) {
        return result;
    }
    if is_integer(left) && is_integer(right) {
        return integer::binary(op, left, right);
    }
    decimal::binary(op, left, right)
}

/// 精确顺序比较；同族Eq/Hash沿用A1无分配规范视图。
#[inline(always)]
pub(super) fn compare(
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> Option<RuntimeResult<Ordering>> {
    if !is_extended(left) && !is_extended(right) {
        return None;
    }
    Some(extended_compare(left, right))
}

/// 高精度比较在冷分支恢复系数，不为定宽比较增加多层调用。
fn extended_compare(left: &RuntimeValue, right: &RuntimeValue) -> RuntimeResult<Ordering> {
    if is_integer(left) && is_integer(right) {
        return integer::compare(left, right);
    }
    if !is_numeric(left) || !is_numeric(right) {
        return Err(RuntimeError::type_mismatch(
            "numeric",
            format!("{} / {}", left.type_name(), right.type_name()),
        ));
    }
    decimal::compare(left, right)
}

/// 仅在至少一侧为已签高精度族时接管混合数值算子。
#[inline(always)]
fn is_extended(value: &RuntimeValue) -> bool {
    matches!(value, RuntimeValue::Lint(_) | RuntimeValue::Lfloat(_))
}

/// 一元负号先完整接纳输入，不能用零归一化掩盖坏语法或超限指数。
pub(super) fn negate(value: &RuntimeValue) -> RuntimeResult<RuntimeValue> {
    common::validate_input(value)?;
    let (number, family) = match value {
        RuntimeValue::Lint(_) => (common::Number::integer(value)?, common::Family::Integer),
        RuntimeValue::Lfloat(_) => (common::Number::decimal(value)?, common::Family::Decimal),
        _ => {
            return Err(RuntimeError::type_mismatch(
                "lint/lfloat",
                value.type_name(),
            ));
        }
    };
    common::finish(-number.coefficient, number.scale, family)
}

/// bool±lint保持奇偶规则和输入接纳，不窄化为i128。
pub(super) fn adjust_bool(value: bool, text: &str) -> RuntimeResult<bool> {
    integer::adjust_bool(value, text)
}

/// 本内核的整数输入类别。
fn is_integer(value: &RuntimeValue) -> bool {
    matches!(
        value,
        RuntimeValue::Int(_) | RuntimeValue::Sint(_) | RuntimeValue::Lint(_)
    )
}

/// 用于先于输入预算的动态类型检查。
fn is_numeric(value: &RuntimeValue) -> bool {
    matches!(
        value,
        RuntimeValue::Int(_)
            | RuntimeValue::Sint(_)
            | RuntimeValue::Lint(_)
            | RuntimeValue::Lfloat(_)
            | RuntimeValue::Float(_)
            | RuntimeValue::Sfloat(_)
    )
}

#[cfg(test)]
mod integer_tests;

#[cfg(test)]
mod decimal_tests;

#[cfg(test)]
mod resource_tests;
