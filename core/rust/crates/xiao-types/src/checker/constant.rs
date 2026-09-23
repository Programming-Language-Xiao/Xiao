//! 常量求值与检查器共享的无状态辅助。
//!
//! 本模块不持有检查器状态；需要环境的递归求值仍由 `TypeChecker` 的实现块负责。

use xiao_source::SourceFile;
use xiao_syntax::{AssignmentOperator, BinaryOperator, Expression, ScalarType, UnaryOperator};

use crate::conversion::{is_float, is_integer};
use crate::numeric::{
    ConstantValue, NumericError, boolean_integer_adjust, check_float_range,
    check_float_to_integer_range, check_integer_range,
};

/// 判断调用者是否为未加反引号的 `random.seed`。
pub(super) fn is_random_seed_callee(callee: &Expression, source: &SourceFile) -> bool {
    let Expression::Member { object, member, .. } = callee else {
        return false;
    };
    let Expression::Name(module) = object.as_ref() else {
        return false;
    };
    !module.backticked
        && !member.backticked
        && module.unquoted_text(source) == "random"
        && member.unquoted_text(source) == "seed"
}
/// 将复合赋值运算符映射为对应的二元运算符。
pub(super) fn assignment_binary_operator(operator: AssignmentOperator) -> Option<BinaryOperator> {
    Some(match operator {
        AssignmentOperator::AddAssign => BinaryOperator::Add,
        AssignmentOperator::SubtractAssign => BinaryOperator::Subtract,
        AssignmentOperator::IntersectAssign => BinaryOperator::Intersect,
        AssignmentOperator::SymmetricDifferenceAssign => BinaryOperator::SymmetricDifference,
        AssignmentOperator::MultiplyAssign => BinaryOperator::Multiply,
        AssignmentOperator::DivideAssign => BinaryOperator::Divide,
        AssignmentOperator::FloorDivideAssign => BinaryOperator::FloorDivide,
        AssignmentOperator::RemainderAssign => BinaryOperator::Remainder,
        AssignmentOperator::PowerAssign => BinaryOperator::Power,
        AssignmentOperator::Assign => return None,
    })
}

/// 判断一个已经通过静态操作数检查的二元运算是否仍可能在运行时
/// 触发固定宽度溢出、非有限结果或除零。
pub(super) fn binary_requires_runtime_check(
    operator: BinaryOperator,
    left: ScalarType,
    right: ScalarType,
    result: ScalarType,
) -> bool {
    if !matches!(
        operator,
        BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::FloorDivide
            | BinaryOperator::Remainder
            | BinaryOperator::Power
    ) {
        return false;
    }
    // 布尔奇偶加减和字符串拼接没有数值范围检查；除法族无论
    // 宽度如何都必须检查零除数。
    if left == ScalarType::Bool && is_integer(right) {
        return false;
    }
    if left == ScalarType::Str && right == ScalarType::Str {
        return false;
    }
    matches!(
        operator,
        BinaryOperator::Divide | BinaryOperator::FloorDivide | BinaryOperator::Remainder
    ) || matches!(
        result,
        ScalarType::Sint | ScalarType::Int | ScalarType::Sfloat | ScalarType::Float
    )
}

/// 解码字符串字面量为常量，引号不配对或转义非法时返回 `None`。
///
/// 转义表来自 [`container_checker::decode_escape`]，与字典键和 IR 降低共用
/// 同一份定义——两侧各写一张表会让常量折叠与运行时值不一致。
pub(super) fn decode_string(text: &str) -> Option<String> {
    let mut characters = text.chars();
    let quote = characters.next()?;
    if quote != '\'' && quote != '"' || characters.next_back()? != quote {
        return None;
    }
    let inner = &text[quote.len_utf8()..text.len() - quote.len_utf8()];
    let mut result = String::new();
    let mut escaped = false;
    for character in inner.chars() {
        if escaped {
            result.push(super::container_checker::decode_escape(character)?);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            result.push(character);
        }
    }
    (!escaped).then_some(result)
}

/// 将已知常量转换为目标标量，失败时返回 `None`。
pub(super) fn convert_constant(value: ConstantValue, target: ScalarType) -> Option<ConstantValue> {
    match (value, target) {
        (ConstantValue::Boolean(value), ScalarType::Str) => Some(ConstantValue::String(
            if value { "true" } else { "false" }.to_owned(),
        )),
        (ConstantValue::String(value), ScalarType::Bool) => match value.as_str() {
            "true" | "True" => Some(ConstantValue::Boolean(true)),
            "false" | "False" => Some(ConstantValue::Boolean(false)),
            _ => None,
        },
        (ConstantValue::Integer(value), target) if is_integer(target) => {
            check_integer_range(value, target).ok()?;
            Some(ConstantValue::Integer(value))
        }
        (ConstantValue::BigInteger(value), ScalarType::Lint) => {
            Some(ConstantValue::BigInteger(value))
        }
        (ConstantValue::Integer(value), target) if is_float(target) => {
            let value = value as f64;
            check_float_range(value, target).ok()?;
            Some(ConstantValue::Float(value))
        }
        (ConstantValue::Float(value), target) if is_integer(target) => {
            check_float_to_integer_range(value, target).ok()?;
            // `trunc` 已在范围检查中完成语义判定；`as i128` 在这里
            // 只作为暂存形式，最终的 lint 高精度表示仍由 Runtime 接管。
            Some(ConstantValue::Integer(value.trunc() as i128))
        }
        (ConstantValue::Float(value), target) if is_float(target) => {
            check_float_range(value, target).ok()?;
            Some(ConstantValue::Float(value))
        }
        (value, target) => {
            let matching = matches!(
                (&value, target),
                (ConstantValue::Boolean(_), ScalarType::Bool)
                    | (ConstantValue::String(_), ScalarType::Str)
            );
            matching.then_some(value)
        }
    }
}

/// 纯求值一个常量二元运算并报告溢出/除零。
pub(super) fn eval_const_binary(
    operator: BinaryOperator,
    left: ConstantValue,
    right: ConstantValue,
) -> Option<Result<ConstantValue, NumericError>> {
    use std::cmp::Ordering;

    match operator {
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Multiply
        | BinaryOperator::Divide
        | BinaryOperator::FloorDivide
        | BinaryOperator::Remainder
        | BinaryOperator::Power => eval_numeric_binary(operator, left, right),
        BinaryOperator::Equal | BinaryOperator::NotEqual => {
            let equal = constants_equal(&left, &right)?;
            Some(Ok(ConstantValue::Boolean(
                if operator == BinaryOperator::Equal {
                    equal
                } else {
                    !equal
                },
            )))
        }
        BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => {
            let ordering = constants_ordering(&left, &right)?;
            let result = match operator {
                BinaryOperator::Less => ordering == Ordering::Less,
                BinaryOperator::LessEqual => ordering != Ordering::Greater,
                BinaryOperator::Greater => ordering == Ordering::Greater,
                BinaryOperator::GreaterEqual => ordering != Ordering::Less,
                _ => unreachable!("已限定为比较运算"),
            };
            Some(Ok(ConstantValue::Boolean(result)))
        }
        BinaryOperator::Is | BinaryOperator::IsNot => {
            let equal = constants_equal(&left, &right)?;
            Some(Ok(ConstantValue::Boolean(
                if operator == BinaryOperator::Is {
                    equal
                } else {
                    !equal
                },
            )))
        }
        BinaryOperator::And | BinaryOperator::Or => {
            let (ConstantValue::Boolean(left), ConstantValue::Boolean(right)) = (&left, &right)
            else {
                return None;
            };
            Some(Ok(ConstantValue::Boolean(
                if operator == BinaryOperator::And {
                    *left && *right
                } else {
                    *left || *right
                },
            )))
        }
        BinaryOperator::In | BinaryOperator::NotIn => None,
        BinaryOperator::Intersect | BinaryOperator::SymmetricDifference => None,
    }
}

/// 求值数值和字符串加法等算术运算；不支持的常量组合返回 `None`。
pub(super) fn eval_numeric_binary(
    operator: BinaryOperator,
    left: ConstantValue,
    right: ConstantValue,
) -> Option<Result<ConstantValue, NumericError>> {
    if let (
        BinaryOperator::Add | BinaryOperator::Subtract,
        ConstantValue::Boolean(value),
        ConstantValue::Integer(amount),
    ) = (operator, &left, &right)
    {
        return Some(Ok(ConstantValue::Boolean(boolean_integer_adjust(
            *value, *amount,
        ))));
    }
    if operator == BinaryOperator::Add {
        if let (ConstantValue::String(left), ConstantValue::String(right)) = (&left, &right) {
            return Some(Ok(ConstantValue::String(format!("{left}{right}"))));
        }
    }

    if let (ConstantValue::Integer(left), ConstantValue::Integer(right)) = (&left, &right) {
        return Some(match operator {
            BinaryOperator::Add => left
                .checked_add(*right)
                .map(ConstantValue::Integer)
                .ok_or(NumericError::Overflow(ScalarType::Lint)),
            BinaryOperator::Subtract => left
                .checked_sub(*right)
                .map(ConstantValue::Integer)
                .ok_or(NumericError::Overflow(ScalarType::Lint)),
            BinaryOperator::Multiply => left
                .checked_mul(*right)
                .map(ConstantValue::Integer)
                .ok_or(NumericError::Overflow(ScalarType::Lint)),
            BinaryOperator::FloorDivide => {
                if *right == 0 {
                    Err(NumericError::DivisionByZero)
                } else {
                    left.checked_div(*right)
                        .map(ConstantValue::Integer)
                        .ok_or(NumericError::Overflow(ScalarType::Lint))
                }
            }
            BinaryOperator::Remainder => {
                if *right == 0 {
                    Err(NumericError::DivisionByZero)
                } else {
                    left.checked_rem(*right)
                        .map(ConstantValue::Integer)
                        .ok_or(NumericError::Overflow(ScalarType::Lint))
                }
            }
            BinaryOperator::Power if *right >= 0 && *right <= u32::MAX as i128 => left
                .checked_pow(*right as u32)
                .map(ConstantValue::Integer)
                .ok_or(NumericError::Overflow(ScalarType::Lint)),
            BinaryOperator::Power => return None,
            BinaryOperator::Divide => {
                if *right == 0 {
                    Err(NumericError::DivisionByZero)
                } else {
                    finite_float(*left as f64 / *right as f64)
                }
            }
            _ => return None,
        });
    }

    let left_float = constant_as_f64(&left)?;
    let right_float = constant_as_f64(&right)?;
    let result = match operator {
        BinaryOperator::Add => left_float + right_float,
        BinaryOperator::Subtract => left_float - right_float,
        BinaryOperator::Multiply => left_float * right_float,
        BinaryOperator::Divide => {
            if right_float == 0.0 {
                return Some(Err(NumericError::DivisionByZero));
            }
            left_float / right_float
        }
        BinaryOperator::Power => left_float.powf(right_float),
        // 这两个运算在类型检查阶段只接受整数；混合浮点常量不可达。
        BinaryOperator::FloorDivide | BinaryOperator::Remainder => return None,
        _ => return None,
    };
    Some(finite_float(result))
}

/// 纯求值一个常量一元运算，并保留溢出错误。
pub(super) fn eval_const_unary(
    operator: UnaryOperator,
    value: ConstantValue,
) -> Option<Result<ConstantValue, NumericError>> {
    Some(match (operator, value) {
        (UnaryOperator::Plus, value @ ConstantValue::Integer(_))
        | (UnaryOperator::Plus, value @ ConstantValue::Float(_)) => Ok(value),
        (UnaryOperator::Minus, ConstantValue::Integer(value)) => value
            .checked_neg()
            .map(ConstantValue::Integer)
            .ok_or(NumericError::Overflow(ScalarType::Lint)),
        (UnaryOperator::Minus, ConstantValue::Float(value)) => finite_float(-value),
        (UnaryOperator::Not, ConstantValue::Boolean(value)) => Ok(ConstantValue::Boolean(!value)),
        _ => return None,
    })
}

/// 将有限的整数/浮点常量转换为双精度暂存值。
pub(super) fn constant_as_f64(value: &ConstantValue) -> Option<f64> {
    let result = match value {
        ConstantValue::Integer(value) => *value as f64,
        ConstantValue::Float(value) => *value,
        _ => return None,
    };
    result.is_finite().then_some(result)
}

/// 判断两个常量是否相等；数值族按数值而不是枚举变体比较。
pub(super) fn constants_equal(left: &ConstantValue, right: &ConstantValue) -> Option<bool> {
    if let (Some(left), Some(right)) = (constant_as_f64(left), constant_as_f64(right)) {
        return Some(left == right);
    }
    match (left, right) {
        (ConstantValue::BigInteger(left), ConstantValue::BigInteger(right)) => {
            Some(normalize_decimal(left) == normalize_decimal(right))
        }
        (ConstantValue::BigInteger(left), ConstantValue::Integer(right))
        | (ConstantValue::Integer(right), ConstantValue::BigInteger(left)) => {
            let normalized = normalize_decimal(left);
            Some(normalized.parse::<i128>().ok() == Some(*right))
        }
        (ConstantValue::Integer(_), ConstantValue::Float(_))
        | (ConstantValue::Float(_), ConstantValue::Integer(_)) => Some(false),
        (ConstantValue::Boolean(left), ConstantValue::Boolean(right)) => Some(left == right),
        (ConstantValue::String(left), ConstantValue::String(right)) => Some(left == right),
        (ConstantValue::None, ConstantValue::None) => Some(true),
        _ => None,
    }
}

/// 求两个可比较常量的顺序。
pub(super) fn constants_ordering(
    left: &ConstantValue,
    right: &ConstantValue,
) -> Option<std::cmp::Ordering> {
    if let (Some(left), Some(right)) = (constant_as_f64(left), constant_as_f64(right)) {
        return left.partial_cmp(&right);
    }
    match (left, right) {
        (ConstantValue::String(left), ConstantValue::String(right)) => Some(left.cmp(right)),
        (ConstantValue::Boolean(left), ConstantValue::Boolean(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

/// 去除十进制大整数的无意义前导零，便于常量相等比较。
pub(super) fn normalize_decimal(value: &str) -> &str {
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let trimmed = unsigned.trim_start_matches('0');
    if trimmed.is_empty() {
        "0"
    } else if value.starts_with('-') {
        // 当前字面量扫描只产生无符号大整数；该分支为后续负大整数
        // 常量保留语义位置，暂不分配新的字符串。
        value
    } else {
        trimmed
    }
}

/// 确保浮点结果有限。
pub(super) fn finite_float(value: f64) -> Result<ConstantValue, NumericError> {
    value
        .is_finite()
        .then_some(ConstantValue::Float(value))
        .ok_or(NumericError::NonFinite)
}
