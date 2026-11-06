//! 转换私有的临时十进制模型；只承接精确数值，不保存共享缓存。

use super::budget;
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::{RuntimeValue, numeric};
use num_bigint::BigInt;

/// 一次显式转换中的精确系数和显示尺度。
pub(super) struct Decimal {
    /// 带符号整数显示系数，输入接纳已完成。
    pub coefficient: BigInt,
    /// 数值为coefficient乘10的负scale次方。
    pub scale: i32,
    /// 去前导零、保留显示尾零的系数位数，零为一位。
    pub digits: usize,
}

impl Decimal {
    /// 精确承接六种数值标签，有限二进制值通过A4独立拆位路径恢复。
    pub fn from_value(value: &RuntimeValue) -> RuntimeResult<Self> {
        let (coefficient, scale) = match value {
            RuntimeValue::Lint(text) => (numeric::parse_integer(text)?, 0),
            RuntimeValue::Lfloat(text) => {
                let decimal = numeric::parse_decimal(text)?;
                let (coefficient, scale) = decimal.into_bigint_and_scale();
                (coefficient, scale as i32)
            }
            RuntimeValue::Int(value) => (BigInt::from(*value), 0),
            RuntimeValue::Sint(value) => (BigInt::from(*value), 0),
            RuntimeValue::Float(value) => return super::ieee::from_f64(*value),
            RuntimeValue::Sfloat(value) => return super::ieee::from_f32(*value),
            _ => return Err(RuntimeError::type_mismatch("numeric", value.type_name())),
        };
        let digits = budget::digits(&coefficient);
        Ok(Self {
            coefficient,
            scale,
            digits,
        })
    }

    /// 新数值到lfloat保留这里明确的scale；整数来源固定为0。
    pub fn into_lfloat(self) -> RuntimeResult<RuntimeValue> {
        budget::into_lfloat(self.coefficient, self.scale)
    }
}
