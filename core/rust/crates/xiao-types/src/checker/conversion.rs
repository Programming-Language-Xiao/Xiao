//! 显式转换、常量目标校验与编译期常量求值。
//!
//! 本模块集中处理转换规则依赖的检查器状态；无状态的常量运算仍由
//! [`super::constant`] 提供，避免把表达式检查和声明检查耦合到同一文件。

use xiao_syntax::{BinaryOperator, Expression, LiteralKind, ScalarType};

use crate::conversion::{is_float, is_integer};
use crate::diagnostics::INVALID_CONVERSION_CODE;
use crate::numeric::{
    ConstantValue, NumericError, check_float_range, check_float_to_integer_range,
    check_integer_range, is_decimal_integer, parse_float_literal, parse_integer_literal,
};

use super::TypeChecker;
use super::constant::{convert_constant, decode_string, eval_const_binary, eval_const_unary};

impl<'source> TypeChecker<'source> {
    /// 识别未被反引号包裹的标量转换构造器。
    pub(super) fn scalar_callee(&self, callee: &Expression) -> Option<ScalarType> {
        let name = self.simple_callee_name(callee)?;
        ScalarType::from_name(&name)
    }

    /// 检查已知常量是否满足转换的内容约束；动态值留给运行时检查。
    pub(super) fn static_conversion_value_is_valid(
        &mut self,
        expression: &Expression,
        target: ScalarType,
    ) -> bool {
        let Some(constant) = self.eval_const(expression) else {
            return true;
        };
        if target == ScalarType::Bool {
            if let ConstantValue::String(value) = &constant {
                if !matches!(value.as_str(), "true" | "True" | "false" | "False") {
                    self.type_error(
                        INVALID_CONVERSION_CODE,
                        "x02.type.invalid_string_boolean",
                        expression.span(),
                        "只有 true/True/false/False 可以转换为 bool".to_string(),
                    );
                    return false;
                }
            }
        }
        true
    }

    /// 检查编译期已知数值是否超出显式转换的目标范围。
    pub(super) fn static_target_range_is_valid(
        &mut self,
        expression: &Expression,
        target: ScalarType,
    ) -> bool {
        let Some(constant) = self.eval_const(expression) else {
            return true;
        };
        let numeric_source = matches!(
            &constant,
            ConstantValue::Integer(_) | ConstantValue::BigInteger(_) | ConstantValue::Float(_)
        );
        if numeric_source && (is_integer(target) || is_float(target)) {
            if let Err(error) = self.check_explicit_constant_target(&constant, target) {
                self.numeric_error(expression.span(), BinaryOperator::Add, error);
                return false;
            }
        }
        true
    }

    /// 验证显式数值转换的常量边界；与隐式初始化不同，浮点到整数
    /// 在这里允许向零截断，只要截断后的结果仍在目标范围内。
    fn check_explicit_constant_target(
        &self,
        value: &ConstantValue,
        target: ScalarType,
    ) -> Result<(), NumericError> {
        match value {
            ConstantValue::Float(value) if is_integer(target) => {
                check_float_to_integer_range(*value, target)
            }
            _ => self.check_constant_target(value, target),
        }
    }

    /// 读取普通名称调用者文本；反引号名称不视为内建函数。
    pub(super) fn simple_callee_name(&self, callee: &Expression) -> Option<String> {
        let Expression::Name(name) = callee else {
            return None;
        };
        if name.backticked {
            return None;
        }
        Some(name.unquoted_text(self.source).to_owned())
    }

    /// 返回任意名称调用者的规范化环境键；内建函数仍由
    /// [`Self::simple_callee_name`] 单独限制为普通 ASCII 名称。
    pub(super) fn function_callee_key(&self, callee: &Expression) -> Option<String> {
        let Expression::Name(name) = callee else {
            return None;
        };
        Some(self.name_key(*name))
    }

    /// 纯递归求值一个已知编译期表达式，动态输入返回 `None`。
    pub(super) fn eval_const(&self, expression: &Expression) -> Option<ConstantValue> {
        match expression {
            Expression::Literal { kind, span } => match kind {
                LiteralKind::Integer => {
                    let text = self.source.slice(*span);
                    parse_integer_literal(text)
                        .map(ConstantValue::Integer)
                        .ok()
                        .or_else(|| {
                            is_decimal_integer(text)
                                .then(|| ConstantValue::BigInteger(text.to_owned()))
                        })
                }
                LiteralKind::Float => parse_float_literal(self.source.slice(*span))
                    .ok()
                    .map(ConstantValue::Float),
                LiteralKind::String => {
                    decode_string(self.source.slice(*span)).map(ConstantValue::String)
                }
                LiteralKind::Boolean => {
                    Some(ConstantValue::Boolean(self.source.slice(*span) == "true"))
                }
                LiteralKind::None => Some(ConstantValue::None),
            },
            Expression::Name(name) => self.constant_values.get(&self.name_key(*name)).cloned(),
            Expression::Group { expression, .. } => self.eval_const(expression),
            Expression::Unary {
                operator, operand, ..
            } => {
                let value = self.eval_const(operand)?;
                match eval_const_unary(*operator, value) {
                    Some(Ok(value)) => Some(value),
                    Some(Err(_)) | None => None,
                }
            }
            Expression::Binary {
                operator,
                left,
                right,
                ..
            } => {
                let left = self.eval_const(left)?;
                let right = self.eval_const(right)?;
                match eval_const_binary(*operator, left, right) {
                    Some(Ok(value)) => Some(value),
                    Some(Err(_)) | None => None,
                }
            }
            Expression::Cast {
                expression, target, ..
            } => {
                let value = self.eval_const(expression)?;
                convert_constant(value, *target)
            }
            Expression::Call {
                callee, arguments, ..
            } => {
                let target = self.scalar_callee(callee)?;
                let value = self.eval_const(&arguments.first()?.value)?;
                convert_constant(value, target)
            }
            _ => None,
        }
    }

    /// 验证常量值能否落入指定标量类型的表示范围。
    pub(super) fn check_constant_target(
        &self,
        value: &ConstantValue,
        target: ScalarType,
    ) -> Result<(), NumericError> {
        match value {
            ConstantValue::Integer(value) => {
                if is_integer(target) {
                    check_integer_range(*value, target)
                } else if is_float(target) {
                    check_float_range(*value as f64, target)
                } else {
                    Ok(())
                }
            }
            ConstantValue::BigInteger(_) => {
                if target == ScalarType::Lint {
                    Ok(())
                } else {
                    Err(NumericError::Overflow(target))
                }
            }
            ConstantValue::Float(value) => {
                if is_float(target) {
                    check_float_range(*value, target)
                } else if is_integer(target) {
                    Err(NumericError::InvalidLiteral(
                        "浮点值不能隐式转换为整数".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
            ConstantValue::Boolean(_) if target == ScalarType::Bool => Ok(()),
            ConstantValue::String(_) if target == ScalarType::Str => Ok(()),
            _ => Ok(()),
        }
    }
}
