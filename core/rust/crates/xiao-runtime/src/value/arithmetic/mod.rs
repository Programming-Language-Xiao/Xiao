//! B2-A2/A3精确算术内核；公开转换和完整语言接线仍后置。

mod common;
mod exact_division;
mod integer;
mod power;

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

/// 仅涉及lint的整数族调用在A2接管；其他已实现数值路径仍走既有内核。
pub(super) fn binary(
    op: Operation,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> Option<RuntimeResult<RuntimeValue>> {
    if !matches!(left, RuntimeValue::Lint(_)) && !matches!(right, RuntimeValue::Lint(_)) {
        return None;
    }
    if is_integer(left) && is_integer(right) {
        return Some(integer::binary(op, left, right));
    }
    if !is_numeric(left) || !is_numeric(right) {
        return Some(Err(RuntimeError::type_mismatch(
            "numeric",
            format!("{} / {}", left.type_name(), right.type_name()),
        )));
    }
    None
}

/// A2整数精确顺序；同族Eq/Hash沿用A1无分配规范视图。
pub(super) fn compare(
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> Option<RuntimeResult<Ordering>> {
    if !matches!(left, RuntimeValue::Lint(_)) && !matches!(right, RuntimeValue::Lint(_)) {
        return None;
    }
    if is_integer(left) && is_integer(right) {
        return Some(integer::compare(left, right));
    }
    if !is_numeric(left) || !is_numeric(right) {
        return Some(Err(RuntimeError::type_mismatch(
            "numeric",
            format!("{} / {}", left.type_name(), right.type_name()),
        )));
    }
    None
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
