//! 既有TAC常量恢复；在任何值进入VM运算前建立共享数值接纳不变量。

use super::exec::Fault;
use xiao_bytecode::TacConstant;
use xiao_runtime::RuntimeValue;

/// 把常量池条目转成运行时值。
pub(super) fn constant_value(constant: &TacConstant) -> Result<RuntimeValue, Fault> {
    let value = match constant {
        TacConstant::Int(value) => RuntimeValue::Int(*value),
        TacConstant::Sint(value) => RuntimeValue::Sint(*value),
        TacConstant::Lint(text) => RuntimeValue::new_lint(text).map_err(Fault::Error)?,
        TacConstant::Float(value) => RuntimeValue::Float(*value),
        TacConstant::Sfloat(value) => RuntimeValue::Sfloat(*value),
        TacConstant::Lfloat(text) => RuntimeValue::new_lfloat(text).map_err(Fault::Error)?,
        TacConstant::Bool(value) => RuntimeValue::Bool(*value),
        TacConstant::Str(text) => RuntimeValue::new_string(text.clone()).map_err(Fault::Error)?,
    };
    Ok(value)
}

#[cfg(test)]
#[path = "numeric_identity_tests.rs"]
mod numeric_identity_tests;
