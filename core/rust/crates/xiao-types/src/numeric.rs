//! 数值族提升、字面量范围和编译期算术辅助。
//!
//! 这里不执行用户程序；函数只在输入已经是静态常量时进行纯计算，动态
//! 表达式则返回“需要运行时检查”的标记，供后端统一降低。

use std::fmt::{self, Display, Formatter};

use xiao_syntax::{BinaryOperator, ScalarType};

use crate::conversion::{is_float, is_integer, is_numeric, numeric_rank, promote_numeric_scalars};
use crate::types::Type;

/// 可用于类型检查的有限常量值。
#[derive(Clone, Debug, PartialEq)]
pub enum ConstantValue {
    /// 任意精度边界以内可直接表示的整数。
    Integer(i128),
    /// 超出宿主 `i128` 但仍为合法十进制的 `lint` 字面量。
    BigInteger(String),
    /// 有限双精度浮点值；`NaN` 与无穷大不被接受。
    Float(f64),
    /// 布尔值。
    Boolean(bool),
    /// UTF-8 字符串。
    String(String),
    /// `none` 空值。
    None,
}

/// 数值检查失败的结构化原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NumericError {
    /// 文本不是合法的十进制字面量。
    InvalidLiteral(String),
    /// 结果超出目标固定宽度。
    Overflow(ScalarType),
    /// 除法、整除或取模的除数为零。
    DivisionByZero,
    /// 浮点结果为 NaN 或无穷大。
    NonFinite,
    /// 运算符不接受给定的标量组合。
    InvalidOperands {
        /// 运算符。
        operator: BinaryOperator,
        /// 左类型。
        left: Type,
        /// 右类型。
        right: Type,
    },
}

impl Display for NumericError {
    /// 生成适合开发者日志的说明。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLiteral(text) => write!(formatter, "invalid numeric literal: {text}"),
            Self::Overflow(scalar) => write!(formatter, "numeric overflow for {}", scalar.as_str()),
            Self::DivisionByZero => formatter.write_str("division by zero"),
            Self::NonFinite => formatter.write_str("non-finite floating-point result"),
            Self::InvalidOperands {
                operator,
                left,
                right,
            } => write!(
                formatter,
                "operator {} does not accept {left} and {right}",
                operator.as_str()
            ),
        }
    }
}

impl std::error::Error for NumericError {}

/// 一次二元运算的静态结果描述。
#[derive(Clone, Debug, PartialEq)]
pub struct NumericOperation {
    /// 结果类型。
    pub result: Type,
    /// 输入是常量时的求值结果。
    pub constant: Option<ConstantValue>,
    /// 输入含动态值时是否必须插入运行时检查。
    pub requires_runtime_check: bool,
}

/// 只根据静态类型分析一次二元运算，不读取或执行用户值。
pub fn analyze_binary(
    operator: BinaryOperator,
    left: &Type,
    right: &Type,
) -> Result<NumericOperation, NumericError> {
    if left.is_dynamic() || right.is_dynamic() {
        return Ok(NumericOperation {
            result: Type::Dynamic,
            constant: None,
            requires_runtime_check: true,
        });
    }
    let (Type::Scalar(left), Type::Scalar(right)) = (left, right) else {
        return Err(NumericError::InvalidOperands {
            operator,
            left: left.clone(),
            right: right.clone(),
        });
    };
    let result = binary_scalar_type(operator, *left, *right)?;
    Ok(NumericOperation {
        result: Type::scalar(result),
        constant: None,
        requires_runtime_check: false,
    })
}

/// 解析非负十进制整数文本。
pub fn parse_integer_literal(text: &str) -> Result<i128, NumericError> {
    text.parse::<i128>()
        .map_err(|_| NumericError::InvalidLiteral(text.to_owned()))
}

/// 判断文本是否为合法的非负十进制整数，即使它超出 `i128` 也返回真。
#[must_use]
pub fn is_decimal_integer(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// 解析有限十进制浮点文本。
pub fn parse_float_literal(text: &str) -> Result<f64, NumericError> {
    let value = text
        .parse::<f64>()
        .map_err(|_| NumericError::InvalidLiteral(text.to_owned()))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(NumericError::NonFinite)
    }
}

/// 验证整数常量是否能放入目标标量。
pub fn check_integer_range(value: i128, target: ScalarType) -> Result<(), NumericError> {
    match target {
        ScalarType::Sint if !(i32::MIN as i128..=i32::MAX as i128).contains(&value) => {
            Err(NumericError::Overflow(target))
        }
        ScalarType::Int if !(i64::MIN as i128..=i64::MAX as i128).contains(&value) => {
            Err(NumericError::Overflow(target))
        }
        ScalarType::Lint => Ok(()),
        _ => Ok(()),
    }
}

/// 验证浮点常量是否能放入目标标量。
pub fn check_float_range(value: f64, target: ScalarType) -> Result<(), NumericError> {
    if !value.is_finite() {
        return Err(NumericError::NonFinite);
    }
    if target == ScalarType::Sfloat && (value as f32).is_infinite() {
        return Err(NumericError::Overflow(target));
    }
    Ok(())
}

/// 验证显式浮点到整数转换在截断后仍落入目标整数范围。
///
/// Xiao 的显式数值转换采用向零截断；因此 `1.9 as int` 是合法的，
/// 而不是把小数部分误判为类型错误。这个辅助函数只检查有限值和
/// 目标范围，实际截断由转换层完成。`lint` 没有固定上限，只要输入
/// 是有限浮点值即可继续交给高精度运行库。
pub fn check_float_to_integer_range(value: f64, target: ScalarType) -> Result<(), NumericError> {
    if !value.is_finite() {
        return Err(NumericError::NonFinite);
    }
    let truncated = value.trunc();
    match target {
        ScalarType::Sint => {
            if truncated < f64::from(i32::MIN) || truncated > f64::from(i32::MAX) {
                Err(NumericError::Overflow(target))
            } else {
                Ok(())
            }
        }
        // `i64::MAX as f64` 会舍入到 2^63，不能直接用 `<=`；上界
        // 用严格小于 2^63 才能排除刚好不可表示的值。
        ScalarType::Int => {
            if !(-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&truncated) {
                Err(NumericError::Overflow(target))
            } else {
                Ok(())
            }
        }
        ScalarType::Lint => Ok(()),
        _ => Err(NumericError::InvalidLiteral(
            "浮点值不能转换为非整数类型".to_string(),
        )),
    }
}

/// 根据运算符和两个标量计算结果类型。
pub fn binary_scalar_type(
    operator: BinaryOperator,
    left: ScalarType,
    right: ScalarType,
) -> Result<ScalarType, NumericError> {
    if matches!(operator, BinaryOperator::Add | BinaryOperator::Subtract)
        && left == ScalarType::Bool
        && is_integer(right)
    {
        return Ok(ScalarType::Bool);
    }
    if operator == BinaryOperator::Add && left == ScalarType::Str && right == ScalarType::Str {
        return Ok(ScalarType::Str);
    }
    if matches!(operator, BinaryOperator::Is | BinaryOperator::IsNot) {
        return Ok(ScalarType::Bool);
    }
    if matches!(operator, BinaryOperator::Equal | BinaryOperator::NotEqual) {
        if left == right || (is_numeric(left) && is_numeric(right)) {
            return Ok(ScalarType::Bool);
        }
        return Err(NumericError::InvalidOperands {
            operator,
            left: Type::scalar(left),
            right: Type::scalar(right),
        });
    }
    if matches!(
        operator,
        BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
    ) {
        if (is_numeric(left) && is_numeric(right))
            || (left == ScalarType::Str && right == ScalarType::Str)
        {
            return Ok(ScalarType::Bool);
        }
        return Err(NumericError::InvalidOperands {
            operator,
            left: Type::scalar(left),
            right: Type::scalar(right),
        });
    }
    if matches!(operator, BinaryOperator::In | BinaryOperator::NotIn) {
        return Err(NumericError::InvalidOperands {
            operator,
            left: Type::scalar(left),
            right: Type::scalar(right),
        });
    }
    if matches!(
        operator,
        BinaryOperator::Intersect | BinaryOperator::SymmetricDifference
    ) {
        return Err(NumericError::InvalidOperands {
            operator,
            left: Type::scalar(left),
            right: Type::scalar(right),
        });
    }
    if matches!(operator, BinaryOperator::And | BinaryOperator::Or)
        && left == ScalarType::Bool
        && right == ScalarType::Bool
    {
        return Ok(ScalarType::Bool);
    }
    if !is_integer(left) && !is_float(left) || !is_integer(right) && !is_float(right) {
        return Err(NumericError::InvalidOperands {
            operator,
            left: Type::scalar(left),
            right: Type::scalar(right),
        });
    }
    if operator == BinaryOperator::FloorDivide || operator == BinaryOperator::Remainder {
        if is_integer(left) && is_integer(right) {
            return Ok(integer_for_rank(
                numeric_rank(left).max(numeric_rank(right)),
            ));
        }
        return Err(NumericError::InvalidOperands {
            operator,
            left: Type::scalar(left),
            right: Type::scalar(right),
        });
    }
    let promoted =
        promote_numeric_scalars(left, right).ok_or_else(|| NumericError::InvalidOperands {
            operator,
            left: Type::scalar(left),
            right: Type::scalar(right),
        })?;
    if operator == BinaryOperator::Divide {
        return Ok(float_for_rank(numeric_rank(promoted)));
    }
    Ok(promoted)
}

/// 应用布尔加减的冻结奇偶规则。
#[must_use]
pub const fn boolean_integer_adjust(value: bool, amount: i128) -> bool {
    if amount.unsigned_abs() % 2 == 0 {
        value
    } else {
        !value
    }
}

/// 将宽度等级映射回整数族标量。
fn integer_for_rank(rank: u8) -> ScalarType {
    match rank {
        0 => ScalarType::Sint,
        1 => ScalarType::Int,
        _ => ScalarType::Lint,
    }
}

/// 将宽度等级映射回浮点族标量。
fn float_for_rank(rank: u8) -> ScalarType {
    match rank {
        0 => ScalarType::Sfloat,
        1 => ScalarType::Float,
        _ => ScalarType::Lfloat,
    }
}

#[cfg(test)]
/// 覆盖数值提升和布尔算术的单元测试。
mod tests {
    use super::{
        binary_scalar_type, boolean_integer_adjust, check_float_to_integer_range,
        check_integer_range,
    };
    use xiao_syntax::{BinaryOperator, ScalarType};

    #[test]
    /// 验证布尔奇偶加减、整数提升和固定宽度溢出。
    fn checks_boolean_arithmetic_and_ranges() {
        assert!(!boolean_integer_adjust(true, 1));
        assert!(boolean_integer_adjust(true, -2));
        assert_eq!(
            binary_scalar_type(BinaryOperator::Divide, ScalarType::Int, ScalarType::Sint),
            Ok(ScalarType::Float)
        );
        assert!(check_integer_range(i64::MAX as i128 + 1, ScalarType::Int).is_err());
        assert!(check_float_to_integer_range(1.9, ScalarType::Int).is_ok());
        assert!(
            check_float_to_integer_range(9_223_372_036_854_775_808.0, ScalarType::Int).is_err()
        );
    }
}
