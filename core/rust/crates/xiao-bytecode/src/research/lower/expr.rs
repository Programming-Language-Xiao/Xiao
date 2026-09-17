//! 表达式的三地址展开。
//!
//! 每个表达式降低为一条或数条指令并把结果放进一个新的虚拟寄存器。这里只做
//! 1:1 语义展开：类型来自 `IrExpression.ty`，本模块不重新推断。

use xiao_ir::{IrCallArgument, IrExpression, IrExpressionKind, IrSpan, IrType};
use xiao_syntax::ScalarType;

use crate::research::lower::Lowerer;
use crate::research::tac::{
    ArithOp, CompareOp, RegisterClass, TacArgument, TacConstant, TacInstr, TacOp, VReg,
};

/// 降低一个表达式并返回结果寄存器。
pub(super) fn lower(lowerer: &mut Lowerer<'_>, expression: &IrExpression) -> VReg {
    match &expression.kind {
        IrExpressionKind::Literal { literal, text } => {
            lower_literal(lowerer, literal, text, expression)
        }
        IrExpressionKind::Name { name } => lower_name(lowerer, &name.text, name.span),
        IrExpressionKind::Group { expression: inner } => lowerer.lower_expression(inner),
        IrExpressionKind::Unary { operator, operand } => {
            lower_unary(lowerer, operator, operand, expression)
        }
        IrExpressionKind::Binary {
            operator,
            left,
            right,
        } => lower_binary(lowerer, operator, left, right, expression),
        IrExpressionKind::Call { callee, arguments } => {
            lower_call(lowerer, callee, arguments, expression)
        }
        IrExpressionKind::Cast {
            expression: inner,
            target,
        } => lower_cast(lowerer, inner, target, expression),
        _ => lower_unsupported(lowerer, "expression", expression.span),
    }
}

/// 降低字面量。
fn lower_literal(
    lowerer: &mut Lowerer<'_>,
    literal: &str,
    text: &str,
    expression: &IrExpression,
) -> VReg {
    let constant = match literal {
        "integer" => match scalar_of(&expression.ty) {
            Some(ScalarType::Sint) => TacConstant::Sint(text.parse().unwrap_or_default()),
            _ => TacConstant::Int(text.parse().unwrap_or_default()),
        },
        "float" => match scalar_of(&expression.ty) {
            Some(ScalarType::Sfloat) => TacConstant::Sfloat(text.parse().unwrap_or_default()),
            _ => TacConstant::Float(text.parse().unwrap_or_default()),
        },
        "bool" => TacConstant::Bool(text == "true"),
        "str" => TacConstant::Str(unquote(text).to_owned()),
        _ => {
            let register = lowerer.new_register(RegisterClass::None, expression.span);
            lowerer.emit(TacInstr::with_dst(
                TacOp::LoadNone,
                register,
                expression.span,
            ));
            return register;
        }
    };
    lowerer.emit_constant(constant, expression.span)
}

/// 降低名称引用：读取同名局部槽，或加载函数引用。
fn lower_name(lowerer: &mut Lowerer<'_>, name: &str, span: IrSpan) -> VReg {
    if let Some(value) = lowerer.value_of_name(name) {
        return lowerer.register_of(value);
    }
    if let Some(function) = lowerer.function_index(name) {
        let register = lowerer.new_register(RegisterClass::ObjHandle, span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::LoadFunc(function),
            register,
            span,
        ));
        return register;
    }
    lowerer.new_register(RegisterClass::Poly, span)
}

/// 降低一元运算。
fn lower_unary(
    lowerer: &mut Lowerer<'_>,
    operator: &str,
    operand: &IrExpression,
    expression: &IrExpression,
) -> VReg {
    let operand = lowerer.lower_expression(operand);
    let register = lowerer.new_register(RegisterClass::Bool, expression.span);
    if operator == "not" {
        lowerer.emit(TacInstr {
            op: TacOp::Compare {
                op: CompareOp::Equal,
                left: operand,
                right: operand,
            },
            dst: Some(register),
            span: expression.span,
        });
    } else {
        lowerer.emit(TacInstr::with_dst(
            TacOp::Cast {
                value: operand,
                target: ScalarType::Int,
            },
            register,
            expression.span,
        ));
    }
    register
}

/// 降低二元运算。
fn lower_binary(
    lowerer: &mut Lowerer<'_>,
    operator: &str,
    left: &IrExpression,
    right: &IrExpression,
    expression: &IrExpression,
) -> VReg {
    let left = lowerer.lower_expression(left);
    let right = lowerer.lower_expression(right);
    if let Some(op) = compare_op(operator) {
        let register = lowerer.new_register(RegisterClass::Bool, expression.span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::Compare { op, left, right },
            register,
            expression.span,
        ));
        return register;
    }
    let Some(op) = arith_op(operator) else {
        return lower_unsupported(lowerer, "operator", expression.span);
    };
    let class = Lowerer::class_of_type(&expression.ty);
    let register = lowerer.new_register(class, expression.span);
    lowerer.emit(TacInstr::with_dst(
        TacOp::Arith { op, left, right },
        register,
        expression.span,
    ));
    register
}

/// 降低调用。
fn lower_call(
    lowerer: &mut Lowerer<'_>,
    callee: &IrExpression,
    arguments: &[IrCallArgument],
    expression: &IrExpression,
) -> VReg {
    let arguments = arguments
        .iter()
        .map(|argument| {
            let value = lowerer.lower_expression(&argument.value);
            match argument.name.as_ref() {
                Some(name) => TacArgument::keyword(name.text.clone(), value),
                None => TacArgument::positional(value),
            }
        })
        .collect::<Vec<_>>();
    let class = Lowerer::class_of_type(&expression.ty);
    let register = lowerer.new_register(class, expression.span);
    let op = match lowerer.function_index_of_expression(callee) {
        Some(target) => {
            let signature = lowerer.signature_of_function(target).unwrap_or_else(|| {
                lowerer
                    .signatures
                    .intern(crate::research::sig::CallSig::dynamic())
            });
            TacOp::Call {
                callee: target,
                signature,
                arguments,
            }
        }
        None => {
            let callee = lowerer.lower_expression(callee);
            TacOp::CallDynamic { callee, arguments }
        }
    };
    lowerer.emit(TacInstr::with_dst(op, register, expression.span));
    register
}

/// 降低显式转换。
fn lower_cast(
    lowerer: &mut Lowerer<'_>,
    inner: &IrExpression,
    target: &str,
    expression: &IrExpression,
) -> VReg {
    let source = lowerer.lower_expression(inner);
    let Some(target) = ScalarType::from_name(target) else {
        return lower_unsupported(lowerer, "cast", expression.span);
    };
    let class = Lowerer::class_of_type(&expression.ty);
    let register = lowerer.new_register(class, expression.span);
    lowerer.emit(TacInstr::with_dst(
        TacOp::Cast {
            value: source,
            target,
        },
        register,
        expression.span,
    ));
    register
}

/// 记录一个本批次尚未降低的表达式形态，并返回占位寄存器。
fn lower_unsupported(lowerer: &mut Lowerer<'_>, what: &str, span: IrSpan) -> VReg {
    lowerer.record_unsupported(format!("{what} 尚未降低（{}..{}）", span.start, span.end));
    lowerer.new_register(RegisterClass::Poly, span)
}

/// 返回表达式静态类型对应的标量。
fn scalar_of(ty: &IrType) -> Option<ScalarType> {
    match ty {
        IrType::Scalar { name } => ScalarType::from_name(name),
        _ => None,
    }
}

/// 去掉字符串字面量的引号。
fn unquote(text: &str) -> &str {
    text.strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            text.strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(text)
}

/// 映射比较运算符。
fn compare_op(operator: &str) -> Option<CompareOp> {
    Some(match operator {
        "<" => CompareOp::Less,
        "<=" => CompareOp::LessEqual,
        ">" => CompareOp::Greater,
        ">=" => CompareOp::GreaterEqual,
        "==" => CompareOp::Equal,
        "!=" => CompareOp::NotEqual,
        _ => return None,
    })
}

/// 映射算术运算符。
fn arith_op(operator: &str) -> Option<ArithOp> {
    Some(match operator {
        "+" => ArithOp::Add,
        "-" => ArithOp::Subtract,
        "*" => ArithOp::Multiply,
        "/" => ArithOp::Divide,
        "//" => ArithOp::FloorDivide,
        "%" => ArithOp::Remainder,
        "**" => ArithOp::Power,
        _ => return None,
    })
}
