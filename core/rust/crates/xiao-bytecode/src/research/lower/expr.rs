//! 表达式的三地址展开。
//!
//! 每个表达式降低为一条或数条指令并把结果放进一个新的虚拟寄存器。这里只做
//! 1:1 语义展开：类型来自 `IrExpression.ty`，本模块不重新推断。

use xiao_ir::{
    IrCallArgument, IrDictEntry, IrExpression, IrExpressionKind, IrPathSegmentKind, IrSelector,
    IrSelectorItem, IrSpan, IrType,
};
use xiao_syntax::ScalarType;

use crate::research::lower::Lowerer;
use crate::research::tac::{
    ArithOp, CompareOp, PathStep, RegisterClass, TacArgument, TacConstant, TacInstr, TacOp, VReg,
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
        IrExpressionKind::Array { elements } => {
            lower_elements(lowerer, elements, expression, ContainerKind::Array)
        }
        IrExpressionKind::Tuple { elements } => {
            lower_elements(lowerer, elements, expression, ContainerKind::Tuple)
        }
        IrExpressionKind::Set { elements } => {
            lower_elements(lowerer, elements, expression, ContainerKind::Set)
        }
        IrExpressionKind::DictTable { entries } => {
            lower_entries(lowerer, entries, expression, ContainerKind::DictTable)
        }
        IrExpressionKind::DictColumn { entries } => {
            lower_entries(lowerer, entries, expression, ContainerKind::DictColumn)
        }
        IrExpressionKind::Selector {
            source, selector, ..
        } => lower_selector(lowerer, source, selector, expression),
        _ => lower_unsupported(lowerer, "expression", expression.span),
    }
}

/// 本批次支持的容器构造形态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContainerKind {
    /// 数组。
    Array,
    /// 元组。
    Tuple,
    /// 集合。
    Set,
    /// 无序字典表。
    DictTable,
    /// 字典列。
    DictColumn,
}

/// 构造一个容器并登记为待释放的临时值。
fn build_container(lowerer: &mut Lowerer<'_>, op: TacOp, span: IrSpan, ty: &IrType) -> VReg {
    let class = Lowerer::class_of_type(ty);
    let register = lowerer.new_register(class, span);
    lowerer.emit(TacInstr::with_dst(op, register, span));
    lowerer.note_temporary(register);
    register
}

/// 降低数组、元组和集合字面量。
fn lower_elements(
    lowerer: &mut Lowerer<'_>,
    elements: &[IrExpression],
    expression: &IrExpression,
    kind: ContainerKind,
) -> VReg {
    let elements = elements
        .iter()
        .map(|element| lowerer.lower_expression(element))
        .collect::<Vec<_>>();
    let op = match kind {
        ContainerKind::Array => TacOp::NewArray { elements },
        ContainerKind::Tuple => TacOp::NewTuple { elements },
        ContainerKind::Set => TacOp::NewSet { elements },
        ContainerKind::DictTable | ContainerKind::DictColumn => {
            return lower_unsupported(lowerer, "container", expression.span);
        }
    };
    build_container(lowerer, op, expression.span, &expression.ty)
}

/// 降低字典表与字典列字面量。
fn lower_entries(
    lowerer: &mut Lowerer<'_>,
    entries: &[IrDictEntry],
    expression: &IrExpression,
    kind: ContainerKind,
) -> VReg {
    let entries = entries
        .iter()
        .map(|entry| (entry.key.clone(), lowerer.lower_expression(&entry.value)))
        .collect::<Vec<_>>();
    let op = match kind {
        ContainerKind::DictTable => TacOp::NewDictTable { entries },
        ContainerKind::DictColumn => TacOp::NewDictColumn { entries },
        _ => return lower_unsupported(lowerer, "container", expression.span),
    };
    build_container(lowerer, op, expression.span, &expression.ty)
}

/// 降低精确索引选择器。
///
/// 只接受「单个 `Exact` 项且路径只有一段」的形态；多选、区间、全选和随机选择
/// 属于后续批次，混合段路径（既含索引又含键）需要嵌套派发，同样留待后续。
fn lower_selector(
    lowerer: &mut Lowerer<'_>,
    source: &IrExpression,
    selector: &IrSelector,
    expression: &IrExpression,
) -> VReg {
    let [IrSelectorItem::Exact { path, .. }] = selector.items.as_slice() else {
        return lower_unsupported(lowerer, "selector", expression.span);
    };
    let [segment] = path.segments.as_slice() else {
        return lower_unsupported(lowerer, "selector path", expression.span);
    };
    let step = match &segment.kind {
        IrPathSegmentKind::Index { text, negative } => {
            let digits = text.strip_prefix('-').unwrap_or(text);
            match digits.parse::<i128>() {
                Ok(magnitude) => PathStep::Index(if *negative { -magnitude } else { magnitude }),
                Err(_) => return lower_unsupported(lowerer, "selector index", expression.span),
            }
        }
        IrPathSegmentKind::Name { name } => PathStep::Key(name.text.clone()),
    };
    let source = lowerer.lower_expression(source);
    let class = Lowerer::class_of_type(&expression.ty);
    let register = lowerer.new_register(class, expression.span);
    lowerer.emit(TacInstr::with_dst(
        TacOp::IndexGet {
            source,
            path: vec![step],
        },
        register,
        expression.span,
    ));
    register
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
        // 必须与类型层共用同一份解码。只剥引号而不解转义，会让带转义的字符串
        // 在常量池里保留反斜杠，与类型层看到的文本不一致。
        "str" => TacConstant::Str(xiao_types::decode_string_literal(text)),
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
///
/// 三种形态各自有准确的语义，**不得**退化成「不是 `not` 就当 `as int`」：
/// 那会把 `-1.5` 静默截断成 `-1`，把未知运算符变成一次无声的类型转换。
fn lower_unary(
    lowerer: &mut Lowerer<'_>,
    operator: &str,
    operand: &IrExpression,
    expression: &IrExpression,
) -> VReg {
    let operand_register = lowerer.lower_expression(operand);
    match operator {
        // `+x` 是恒等；标量没有所有权，直接复用同一个寄存器。
        "+" => operand_register,
        "not" => {
            // 取反必须与 `false` 比较。与自身比较恒为真，是静默算错。
            let falsy = lowerer.emit_constant(TacConstant::Bool(false), expression.span);
            let register = lowerer.new_register(RegisterClass::Bool, expression.span);
            lowerer.emit(TacInstr::with_dst(
                TacOp::Compare {
                    op: CompareOp::Equal,
                    left: operand_register,
                    right: falsy,
                },
                register,
                expression.span,
            ));
            register
        }
        "-" => {
            // 一元负号等价于 `0 - x`；零常量必须与操作数同宽度，否则运行时会
            // 以「宽度不一致」拒绝——那正是不做隐式提升的代价。
            let Some(scalar) = scalar_of(&operand.ty) else {
                return lower_unsupported(lowerer, "unary minus", expression.span);
            };
            let zero = match scalar {
                ScalarType::Sint => TacConstant::Sint(0),
                ScalarType::Int => TacConstant::Int(0),
                ScalarType::Sfloat => TacConstant::Sfloat(0.0),
                ScalarType::Float => TacConstant::Float(0.0),
                _ => return lower_unsupported(lowerer, "unary minus", expression.span),
            };
            let zero_register = lowerer.emit_constant(zero, expression.span);
            let class = Lowerer::class_of_type(&expression.ty);
            let register = lowerer.new_register(class, expression.span);
            lowerer.emit(TacInstr::with_dst(
                TacOp::Arith {
                    op: ArithOp::Subtract,
                    left: zero_register,
                    right: operand_register,
                },
                register,
                expression.span,
            ));
            register
        }
        _ => lower_unsupported(lowerer, "unary operator", expression.span),
    }
}

/// 降低二元运算。
fn lower_binary(
    lowerer: &mut Lowerer<'_>,
    operator: &str,
    left: &IrExpression,
    right: &IrExpression,
    expression: &IrExpression,
) -> VReg {
    let left_register = lowerer.lower_expression(left);
    let right_register = lowerer.lower_expression(right);
    if let Some(op) = compare_op(operator) {
        let register = lowerer.new_register(RegisterClass::Bool, expression.span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::Compare {
                op,
                left: left_register,
                right: right_register,
            },
            register,
            expression.span,
        ));
        return register;
    }
    let Some(op) = arith_op(operator) else {
        return lower_unsupported(lowerer, "operator", expression.span);
    };
    // 运行时不做隐式宽度提升，后端的这一步之前是缺失的：静态允许 `int + sint`
    // 并提升为 `int`，运行时却会以「宽度不一致」拒绝。宽度由类型层的提升规则
    // 决定，这里不自行推断。
    let (left_register, right_register) = promote_operands(
        lowerer,
        op,
        left,
        right,
        left_register,
        right_register,
        expression.span,
    );
    let class = Lowerer::class_of_type(&expression.ty);
    let register = lowerer.new_register(class, expression.span);
    lowerer.emit(TacInstr::with_dst(
        TacOp::Arith {
            op,
            left: left_register,
            right: right_register,
        },
        register,
        expression.span,
    ));
    register
}

/// 把二元数值运算的两个操作数提升到同一宽度，必要时插入显式转换。
///
/// 非标量操作数（动态值等）原样返回，交给运行时检查。`lint`/`lfloat` 的算术
/// 尚无运行时实现，同样原样返回让它明确报错，而不是先转成一个假宽度。
fn promote_operands(
    lowerer: &mut Lowerer<'_>,
    op: ArithOp,
    left: &IrExpression,
    right: &IrExpression,
    left_register: VReg,
    right_register: VReg,
    span: IrSpan,
) -> (VReg, VReg) {
    let (Some(left_scalar), Some(right_scalar)) = (scalar_of(&left.ty), scalar_of(&right.ty))
    else {
        return (left_register, right_register);
    };
    let Some(promoted) = xiao_types::promote_numeric_scalars(left_scalar, right_scalar) else {
        return (left_register, right_register);
    };
    let rank = xiao_types::numeric_rank(promoted);
    let target = match op {
        // `/` 的静态结果类型是浮点，两侧都要先转成浮点。
        ArithOp::Divide => xiao_types::float_for_rank(rank),
        // `//` 与 `%` 只接受整数。
        ArithOp::FloorDivide | ArithOp::Remainder => xiao_types::integer_for_rank(rank),
        _ => promoted,
    };
    if matches!(target, ScalarType::Lint | ScalarType::Lfloat) {
        return (left_register, right_register);
    }
    (
        cast_if_needed(lowerer, left_register, left_scalar, target, span),
        cast_if_needed(lowerer, right_register, right_scalar, target, span),
    )
}

/// 源宽度与目标不同时插入一次显式转换。
fn cast_if_needed(
    lowerer: &mut Lowerer<'_>,
    register: VReg,
    source: ScalarType,
    target: ScalarType,
    span: IrSpan,
) -> VReg {
    if source == target {
        return register;
    }
    let class = class_of_scalar(target);
    let converted = lowerer.new_register(class, span);
    lowerer.emit(TacInstr::with_dst(
        TacOp::Cast {
            value: register,
            target,
        },
        converted,
        span,
    ));
    converted
}

/// 按标量返回它占用的寄存器类别。
fn class_of_scalar(scalar: ScalarType) -> RegisterClass {
    match scalar {
        ScalarType::Int | ScalarType::Sint => RegisterClass::Int,
        ScalarType::Float | ScalarType::Sfloat => RegisterClass::Float,
        ScalarType::Bool => RegisterClass::Bool,
        ScalarType::Str | ScalarType::Lint | ScalarType::Lfloat => RegisterClass::ObjHandle,
    }
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
            // `*`/`**` 展开尚未降低。只按 `name.is_some()` 区分关键字与位置会
            // 把展开实参静默当成普通实参，语义丢失且不报错。
            if argument.kind != "positional" && argument.kind != "keyword" {
                lowerer.record_unsupported(format!(
                    "展开实参尚未降低：{}（{}..{}）",
                    argument.kind, argument.span.start, argument.span.end
                ));
            }
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
