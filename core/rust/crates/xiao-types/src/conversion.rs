//! 标量转换矩阵与赋值兼容判断。
//!
//! `as` 和构造式转换必须调用同一组函数；本模块不关心表达式来源，因此
//! 运行时和编译期入口可以共享规则而不会各自维护一份矩阵。

use std::fmt::{self, Display, Formatter};

use xiao_syntax::ScalarType;

use crate::types::Type;

/// 转换的语义类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConversionKind {
    /// 源和目标完全相同。
    Identity,
    /// 不损失表示范围的数值加宽。
    NumericWidening,
    /// 可能丢失范围或精度的显式数值转换。
    NumericNarrowing,
    /// `str` 到 `bool`，只接受四个冻结拼写。
    StrToBool,
    /// `bool` 到小写 `str`。
    BoolToStr,
    /// 源为动态值，需要运行时检查。
    RuntimeChecked,
}

/// 一次经过分类的标量转换。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Conversion {
    /// 转换前的静态类型；动态源以 `Dynamic` 表示。
    pub source: Type,
    /// 转换目标标量。
    pub target: ScalarType,
    /// 转换类别。
    pub kind: ConversionKind,
    /// 是否必须在运行时验证值的范围或内容。
    pub requires_runtime_check: bool,
}

/// 转换不被当前冻结矩阵接受的原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionError {
    /// 源类型。
    pub source: Type,
    /// 目标类型。
    pub target: ScalarType,
}

impl Display for ConversionError {
    /// 生成稳定的开发者说明。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot convert {} to {}",
            self.source,
            self.target.as_str()
        )
    }
}

impl std::error::Error for ConversionError {}

/// 判断一个显式 `as`/构造式转换是否合法，并返回运行时检查标记。
pub fn classify_conversion(
    source: &Type,
    target: ScalarType,
) -> Result<Conversion, ConversionError> {
    if source.is_dynamic() {
        return Ok(Conversion {
            source: source.clone(),
            target,
            kind: ConversionKind::RuntimeChecked,
            requires_runtime_check: true,
        });
    }
    let Type::Scalar(source_scalar) = source else {
        return Err(ConversionError {
            source: source.clone(),
            target,
        });
    };
    let kind = if *source_scalar == target {
        ConversionKind::Identity
    } else if *source_scalar == ScalarType::Str && target == ScalarType::Bool {
        ConversionKind::StrToBool
    } else if *source_scalar == ScalarType::Bool && target == ScalarType::Str {
        ConversionKind::BoolToStr
    } else if is_numeric(*source_scalar) && is_numeric(target) {
        if numeric_widens(*source_scalar, target) {
            ConversionKind::NumericWidening
        } else {
            ConversionKind::NumericNarrowing
        }
    } else {
        return Err(ConversionError {
            source: source.clone(),
            target,
        });
    };
    Ok(Conversion {
        source: source.clone(),
        target,
        kind,
        requires_runtime_check: matches!(
            kind,
            ConversionKind::NumericNarrowing | ConversionKind::StrToBool
        ),
    })
}

/// 判断源类型能否隐式赋给目标类型槽。
#[must_use]
pub fn can_assign(source: &Type, target: &Type) -> bool {
    if source == target || source.is_dynamic() || target.is_dynamic() {
        return true;
    }
    if source.is_container() && target.is_container() {
        return container_can_assign(source, target);
    }
    let (Type::Scalar(source), Type::Scalar(target)) = (source, target) else {
        return false;
    };
    if *source == *target {
        return true;
    }
    is_numeric(*source) && is_numeric(*target) && numeric_widens(*source, *target)
}

/// 检查两个容器类型的结构化赋值兼容性。
fn container_can_assign(source: &Type, target: &Type) -> bool {
    match (source, target) {
        (Type::Array(source), Type::Array(target)) => match (source, target) {
            (_, crate::containers::ArrayType::Unknown) => true,
            (crate::containers::ArrayType::Unknown, _) => true,
            (
                crate::containers::ArrayType::Homogeneous {
                    element: source,
                    length: source_length,
                },
                crate::containers::ArrayType::Homogeneous {
                    element: target,
                    length: target_length,
                },
            ) => {
                target_length.is_none_or(|length| source_length == &Some(length))
                    && can_assign(source, target)
            }
            (
                crate::containers::ArrayType::Heterogeneous { elements: source },
                crate::containers::ArrayType::Heterogeneous { elements: target },
            ) => {
                source.len() == target.len()
                    && source
                        .iter()
                        .zip(target)
                        .all(|(source, target)| can_assign(source, target))
            }
            (
                crate::containers::ArrayType::Heterogeneous { elements },
                crate::containers::ArrayType::Homogeneous { element, length },
            ) => {
                length.is_none_or(|length| length == elements.len())
                    && elements.iter().all(|item| can_assign(item, element))
            }
            (
                crate::containers::ArrayType::Homogeneous { element, length },
                crate::containers::ArrayType::Heterogeneous { elements },
            ) => {
                length.is_none_or(|length| length == elements.len())
                    && elements.iter().all(|item| can_assign(element, item))
            }
        },
        (Type::Tuple(source), Type::Tuple(target)) => {
            source.len() == target.len()
                && source
                    .iter()
                    .zip(target)
                    .all(|(source, target)| can_assign(source, target))
        }
        (Type::DictTable(source), Type::DictTable(target)) => {
            source.entries.len() == target.entries.len()
                && source.entries.iter().all(|source_entry| {
                    target
                        .value_type(&source_entry.key)
                        .is_some_and(|target| can_assign(&source_entry.value, target))
                })
        }
        (Type::DictColumn(source), Type::DictColumn(target)) => {
            source.entries.len() == target.entries.len()
                && source
                    .entries
                    .iter()
                    .zip(&target.entries)
                    .all(|(source, target)| {
                        source.key == target.key && can_assign(&source.value, &target.value)
                    })
        }
        _ => false,
    }
}

/// 计算两个数值标量的安全提升结果。
#[must_use]
pub fn promote_numeric_scalars(left: ScalarType, right: ScalarType) -> Option<ScalarType> {
    if !is_numeric(left) || !is_numeric(right) {
        return None;
    }
    if is_float(left) || is_float(right) {
        let rank = numeric_rank(left).max(numeric_rank(right));
        Some(float_for_rank(rank))
    } else {
        Some(integer_for_rank(
            numeric_rank(left).max(numeric_rank(right)),
        ))
    }
}

/// 返回整数/浮点族的宽度等级（32、64、无限分别为 0、1、2）。
#[must_use]
pub const fn numeric_rank(scalar: ScalarType) -> u8 {
    match scalar {
        ScalarType::Sint | ScalarType::Sfloat => 0,
        ScalarType::Int | ScalarType::Float => 1,
        ScalarType::Lint | ScalarType::Lfloat => 2,
        ScalarType::Str | ScalarType::Bool => 0,
    }
}

/// 判断标量是否为数值类型。
#[must_use]
pub const fn is_numeric(scalar: ScalarType) -> bool {
    matches!(
        scalar,
        ScalarType::Sint
            | ScalarType::Int
            | ScalarType::Lint
            | ScalarType::Sfloat
            | ScalarType::Float
            | ScalarType::Lfloat
    )
}

/// 判断标量是否为整数族。
#[must_use]
pub const fn is_integer(scalar: ScalarType) -> bool {
    matches!(
        scalar,
        ScalarType::Sint | ScalarType::Int | ScalarType::Lint
    )
}

/// 判断标量是否为浮点族。
#[must_use]
pub const fn is_float(scalar: ScalarType) -> bool {
    matches!(
        scalar,
        ScalarType::Sfloat | ScalarType::Float | ScalarType::Lfloat
    )
}

/// 判断从一个标量到另一个标量是否属于安全加宽。
fn numeric_widens(source: ScalarType, target: ScalarType) -> bool {
    if is_integer(source) && is_integer(target) {
        return numeric_rank(source) <= numeric_rank(target);
    }
    if is_float(source) && is_float(target) {
        return numeric_rank(source) <= numeric_rank(target);
    }
    is_integer(source) && is_float(target) && numeric_rank(source) <= numeric_rank(target)
}

/// 将宽度等级映射回整数族标量。
const fn integer_for_rank(rank: u8) -> ScalarType {
    match rank {
        0 => ScalarType::Sint,
        1 => ScalarType::Int,
        _ => ScalarType::Lint,
    }
}

/// 将宽度等级映射回浮点族标量。
const fn float_for_rank(rank: u8) -> ScalarType {
    match rank {
        0 => ScalarType::Sfloat,
        1 => ScalarType::Float,
        _ => ScalarType::Lfloat,
    }
}

#[cfg(test)]
/// 覆盖转换矩阵和数值提升的单元测试。
mod tests {
    use super::{ConversionKind, can_assign, classify_conversion, promote_numeric_scalars};
    use crate::types::Type;
    use xiao_syntax::ScalarType;

    #[test]
    /// 验证数值提升、显式窄化和字符串布尔转换共享同一矩阵。
    fn classifies_scalar_conversions() {
        assert_eq!(
            promote_numeric_scalars(ScalarType::Sint, ScalarType::Float),
            Some(ScalarType::Float)
        );
        assert!(can_assign(
            &Type::scalar(ScalarType::Sint),
            &Type::scalar(ScalarType::Int)
        ));
        let conversion = classify_conversion(&Type::scalar(ScalarType::Str), ScalarType::Bool)
            .expect("str 到 bool 应可显式转换");
        assert_eq!(conversion.kind, ConversionKind::StrToBool);
        assert!(conversion.requires_runtime_check);
    }
}
