//! 表达式的三地址展开。
//!
//! 每个表达式降低为一条或数条指令并把结果放进一个新的虚拟寄存器。这里只做
//! 1:1 语义展开：类型来自 `IrExpression.ty`，本模块不重新推断。

use xiao_ir::{
    IrCallArgument, IrDictEntry, IrExpression, IrExpressionKind, IrSelector, IrSelectorItem,
    IrSpan, IrType,
};
use xiao_syntax::ScalarType;

use crate::lower::Lowerer;
use crate::tac::{
    ArithOp, CompareOp, PathStep, RegisterClass, SetCompareOp, SetOpKind, TacArgument, TacConstant,
    TacInstr, TacOp, VReg,
};

/// 降低一个表达式并返回结果寄存器。
pub(super) fn lower(lowerer: &mut Lowerer<'_>, expression: &IrExpression) -> VReg {
    let register = match &expression.kind {
        IrExpressionKind::Literal { literal, text } => {
            lower_literal(lowerer, literal, text, expression)
        }
        IrExpressionKind::Name { name } => {
            lower_name(lowerer, &name.text, name.backticked, name.span)
        }
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
        IrExpressionKind::IntrinsicCall { id, arguments } => {
            lower_intrinsic_call(lowerer, *id, arguments, expression)
        }
        IrExpressionKind::DictBuiltin {
            operation,
            arguments,
        } => lower_dict_builtin(lowerer, operation, arguments, expression),
        IrExpressionKind::NewCall { callee, arguments } => {
            lowerer.lower_new(callee, arguments, expression.span)
        }
        IrExpressionKind::Member { object, member } => {
            // 模块命名空间仍是动态载体；补全静态函数签名后，继续使用既有
            // MemberGet/CallValue 路径，不能把模块函数误判成绑定表方法。
            if matches!(expression.ty, IrType::Function { .. })
                && !matches!(object.ty, IrType::Dynamic)
            {
                lower_known_unsupported(lowerer, "表方法值的动态调用", expression.span)
            } else {
                let object = lowerer.lower_expression(object);
                let register =
                    lowerer.new_register(Lowerer::class_of_type(&expression.ty), expression.span);
                lowerer.emit(TacInstr::with_dst(
                    TacOp::MemberGet {
                        object,
                        member: super::name_key(&member.text, member.backticked),
                    },
                    register,
                    expression.span,
                ));
                register
            }
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
            source,
            selector,
            step,
            selection_plan,
        } => lower_selector(
            lowerer,
            source,
            selector,
            step.as_deref(),
            *selection_plan,
            expression,
        ),
    };
    lowerer.emit_runtime_checks(expression.span, register);
    register
}

/// 降低标准库字典查询的编译器私有动作。
/// 将标准库字典私有入口降低为字典删除或枚举 TAC 指令。
fn lower_dict_builtin(
    lowerer: &mut Lowerer<'_>,
    operation: &str,
    arguments: &[IrCallArgument],
    expression: &IrExpression,
) -> VReg {
    let Some(source) = arguments
        .first()
        .map(|argument| lowerer.lower_expression(&argument.value))
    else {
        return lower_unsupported(lowerer, "dict builtin arguments", expression.span);
    };
    let class = Lowerer::class_of_type(&expression.ty);
    match operation {
        "remove" => {
            let Some(index) = arguments
                .get(1)
                .map(|argument| lowerer.lower_expression(&argument.value))
            else {
                return lower_unsupported(lowerer, "dict remove arguments", expression.span);
            };
            lowerer.emit(TacInstr::new(
                TacOp::DictRemoveDynamic { source, index },
                expression.span,
            ));
            let result = lowerer.new_register(RegisterClass::None, expression.span);
            lowerer.emit(TacInstr::with_dst(TacOp::LoadNone, result, expression.span));
            result
        }
        "keys" => {
            let result = lowerer.new_register(class, expression.span);
            lowerer.emit(TacInstr::with_dst(
                TacOp::DictKeys { source },
                result,
                expression.span,
            ));
            result
        }
        "values" => {
            let result = lowerer.new_register(class, expression.span);
            lowerer.emit(TacInstr::with_dst(
                TacOp::DictValues { source },
                result,
                expression.span,
            ));
            result
        }
        _ => lower_unsupported(lowerer, "unknown dict builtin", expression.span),
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

/// 降低选择器。单值计划继续使用精确 `IndexGet`，其余形态使用独立的
/// `SelectorApply` 操作，计划和动态操作数均由前端显式提供。
fn lower_selector(
    lowerer: &mut Lowerer<'_>,
    source: &IrExpression,
    selector: &IrSelector,
    step: Option<&IrExpression>,
    selection_plan: Option<u32>,
    expression: &IrExpression,
) -> VReg {
    let Some(plan_id) = selection_plan else {
        return lower_unsupported(lowerer, "selector plan", expression.span);
    };
    let Some(plan) = lowerer.selection_plan(plan_id).cloned() else {
        return lower_unsupported(lowerer, "selector plan reference", expression.span);
    };
    let source_register = lowerer.lower_expression(source);
    if selector.items.len() == 1
        && let xiao_ir::IrSelectorItem::ExactExpression {
            expression: index, ..
        } = &selector.items[0]
    {
        let index_register = lowerer.lower_expression(index);
        let class = Lowerer::class_of_type(&expression.ty);
        let register = lowerer.new_register(class, expression.span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::IndexGetDynamic {
                source: source_register,
                index: index_register,
            },
            register,
            expression.span,
        ));
        return register;
    }
    if selector.items.len() == 1
        && let xiao_ir::IrSelectorItem::Exact { path, .. } = &selector.items[0]
    {
        let Some(path) = lower_ir_path(path) else {
            return lower_unsupported(lowerer, "selector path", expression.span);
        };
        if matches!(
            source.ty,
            IrType::Array {
                shape: xiao_ir::IrArrayShape::Unknown
                    | xiao_ir::IrArrayShape::Homogeneous { length: None, .. }
            }
        ) && let [PathStep::Index(raw)] = path.as_slice()
            && let Ok(raw) = i64::try_from(*raw)
        {
            let index = lowerer.emit_constant(TacConstant::Int(raw), expression.span);
            lowerer.emit_runtime_checks_in(expression.span, index);
            let class = Lowerer::class_of_type(&expression.ty);
            let register = lowerer.new_register(class, expression.span);
            lowerer.emit(TacInstr::with_dst(
                TacOp::IndexGetDynamic {
                    source: source_register,
                    index,
                },
                register,
                expression.span,
            ));
            return register;
        }
        lowerer.emit_runtime_checks_in(expression.span, source_register);
        let class = Lowerer::class_of_type(&expression.ty);
        let register = lowerer.new_register(class, expression.span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::IndexGet {
                source: source_register,
                path,
            },
            register,
            expression.span,
        ));
        return register;
    }
    if plan.is_single_value() {
        let Some(path) = plan.selected_paths.first() else {
            return lower_unsupported(lowerer, "empty exact selector path", expression.span);
        };
        let Some(path) = lower_selection_path(path) else {
            return lower_unsupported(lowerer, "selector path", expression.span);
        };
        let class = Lowerer::class_of_type(&expression.ty);
        let register = lowerer.new_register(class, expression.span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::IndexGet {
                source: source_register,
                path,
            },
            register,
            expression.span,
        ));
        return register;
    }
    let dynamic_step = plan.step.as_ref().is_some_and(|item| item.dynamic);
    let step_register = dynamic_step
        .then(|| step.map(|item| lowerer.lower_expression(item)))
        .flatten();
    let mut random_counts = Vec::with_capacity(plan.items.len());
    for (item, ir_item) in plan.items.iter().zip(&selector.items) {
        if let xiao_ir::IrSelectionItemPlan::Random {
            dynamic_count: true,
            ..
        } = item
        {
            let IrSelectorItem::Random { count, .. } = ir_item else {
                return lower_unsupported(lowerer, "random selector plan", expression.span);
            };
            random_counts.push(Some(lowerer.lower_expression(count)));
        } else {
            random_counts.push(None);
        }
    }
    // 路径边界检查挂在选择项/端点跨度上；子表达式已经消费步长和
    // 随机数量检查，这里再收拢选择器剩余范围内的检查，避免静默丢失。
    lowerer.emit_runtime_checks_in(expression.span, source_register);
    let class = Lowerer::class_of_type(&expression.ty);
    let register = lowerer.new_register(class, expression.span);
    lowerer.emit(TacInstr::with_dst(
        TacOp::SelectorApply {
            source: source_register,
            plan: plan_id,
            step: step_register,
            random_counts,
        },
        register,
        expression.span,
    ));
    register
}

/// 把 IR 规范化路径转成精确索引路径。
fn lower_selection_path(path: &xiao_ir::IrSelectionPath) -> Option<Vec<PathStep>> {
    path.iter()
        .map(|segment| match segment {
            xiao_ir::IrSelectionPathSegment::Index { raw, .. } => Some(PathStep::Index(*raw)),
            xiao_ir::IrSelectionPathSegment::Key(key) => Some(PathStep::Key(key.clone())),
        })
        .collect()
}

/// 将 IR 的精确路径镜像为字节码路径；未解析的数字索引仍保留原始值，
/// 由 Runtime 统一完成长度和边界检查。
fn lower_ir_path(path: &xiao_ir::IrPath) -> Option<Vec<PathStep>> {
    path.segments
        .iter()
        .map(|segment| match &segment.kind {
            xiao_ir::IrPathSegmentKind::Index { text, negative } => {
                let magnitude = text.trim_start_matches('-').parse::<i128>().ok()?;
                Some(PathStep::Index(if *negative {
                    -magnitude
                } else {
                    magnitude
                }))
            }
            xiao_ir::IrPathSegmentKind::Name { name } => Some(PathStep::Key(name.text.clone())),
        })
        .collect()
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
            Some(ScalarType::Sint) => TacConstant::Sint(parse_literal_or_record(
                lowerer,
                text,
                expression.span,
                "sint",
            )),
            Some(ScalarType::Lint) => TacConstant::Lint(text.to_owned()),
            _ => TacConstant::Int(parse_literal_or_record(
                lowerer,
                text,
                expression.span,
                "int",
            )),
        },
        "float" => match scalar_of(&expression.ty) {
            Some(ScalarType::Sfloat) => TacConstant::Sfloat(parse_literal_or_record(
                lowerer,
                text,
                expression.span,
                "sfloat",
            )),
            Some(ScalarType::Lfloat) => {
                TacConstant::Lfloat(xiao_types::normalize_decimal_zero(text))
            }
            _ => TacConstant::Float(parse_literal_or_record(
                lowerer,
                text,
                expression.span,
                "float",
            )),
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

/// 解析定宽字面量；失败时显式登记内部缺口，禁止静默变成零值。
fn parse_literal_or_record<T>(
    lowerer: &mut Lowerer<'_>,
    text: &str,
    span: IrSpan,
    scalar: &str,
) -> T
where
    T: std::str::FromStr + Default,
{
    match text.parse() {
        Ok(value) => value,
        Err(_) => {
            lowerer.record_unsupported(format!(
                "{scalar} 字面量无法解析（{text:?}，{}..{}）",
                span.start, span.end
            ));
            T::default()
        }
    }
}

/// 降低名称引用：读取同名局部槽，或加载函数引用。
fn lower_name(lowerer: &mut Lowerer<'_>, name: &str, backticked: bool, span: IrSpan) -> VReg {
    if let Some(register) = lowerer.import_register(name, backticked) {
        return register;
    }
    if let Some(register) = lowerer
        .frame
        .parameter_names
        .get(&super::name_key(name, backticked))
    {
        return *register;
    }
    if let Some(register) = lowerer.global_import_reference(name, backticked, span) {
        return register;
    }
    if let Some(value) = lowerer.value_of_name(name, backticked) {
        if !lowerer.frame.value_regs.contains_key(&value) {
            if let Some(register) = lowerer.singleton_reference(name, span) {
                return register;
            }
            if let Some(register) = lowerer.global_constant(name, backticked) {
                return register;
            }
        }
        return lowerer.register_of(value);
    }
    if let Some(register) = lowerer.singleton_reference(name, span) {
        return register;
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
    if !backticked
        && lowerer
            .program
            .package_roots
            .iter()
            .any(|root| root == name)
    {
        let register = lowerer.new_register(RegisterClass::ObjHandle, span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::PackageRoot(name.to_owned()),
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
            if matches!(scalar, ScalarType::Lint | ScalarType::Lfloat) {
                let register =
                    lowerer.new_register(Lowerer::class_of_type(&expression.ty), expression.span);
                lowerer.emit(TacInstr::with_dst(
                    TacOp::Negate(operand_register),
                    register,
                    expression.span,
                ));
                return register;
            }
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
    if matches!(operator, "and" | "or") {
        return lower_logical(lowerer, operator, left, right, expression);
    }
    let left_register = lowerer.lower_expression(left);
    let right_register = lowerer.lower_expression(right);
    if let Some(op) = set_op(operator)
        && is_set_operation_candidate(operator, &left.ty, &right.ty)
    {
        lowerer.emit_runtime_checks_for(expression.span, &[left_register, right_register]);
        let register = lowerer.new_register(RegisterClass::ObjHandle, expression.span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::SetOp {
                op,
                left: left_register,
                right: right_register,
            },
            register,
            expression.span,
        ));
        return register;
    }
    if let Some(op) = set_compare(operator)
        && is_set_comparison_candidate(operator, &left.ty, &right.ty)
    {
        lowerer.emit_runtime_checks_for(expression.span, &[left_register, right_register]);
        let register = lowerer.new_register(RegisterClass::Bool, expression.span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::SetCompare {
                op,
                left: left_register,
                right: right_register,
            },
            register,
            expression.span,
        ));
        return register;
    }
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

/// 降低短路逻辑运算。
///
/// TAC 没有 phi 指令；结果寄存器由两个分支各自以 `Copy` 写入，并在合流块
/// 被后续指令读取。右侧表达式只在需要它的块中降低，因此它的指令和临时值
/// 都不会出现在短路边上。条件值沿用语句条件的动态转换规则，保证静态 bool
/// 和运行时动态值在 VM 中共享同一条分支语义。
fn lower_logical(
    lowerer: &mut Lowerer<'_>,
    operator: &str,
    left: &IrExpression,
    right: &IrExpression,
    expression: &IrExpression,
) -> VReg {
    let left_value = lowerer.lower_expression(left);
    let left_condition = lower_condition_register(lowerer, left, left_value, expression.span);
    let result = lowerer.new_register(Lowerer::class_of_type(&expression.ty), expression.span);

    let right_block = lowerer.new_block(expression.span);
    let short_block = lowerer.new_block(expression.span);
    let merge_block = lowerer.new_block(expression.span);
    let (if_true, if_false) = if operator == "and" {
        (right_block, short_block)
    } else {
        (short_block, right_block)
    };
    lowerer.emit(TacInstr::new(
        TacOp::BranchIf {
            condition: left_condition,
            if_true,
            if_false,
        },
        expression.span,
    ));
    // 左侧的临时值在分支前统一归还；右侧尚未降低，因而不可能被提前释放。
    lowerer.flush_temporaries(expression.span);

    lowerer.switch_to(short_block);
    let short_value = lowerer.emit_constant(TacConstant::Bool(operator == "or"), expression.span);
    lowerer.emit(TacInstr::with_dst(
        TacOp::Copy(short_value),
        result,
        expression.span,
    ));
    if !lowerer.current_block_terminated() {
        lowerer.emit(TacInstr::new(TacOp::Jump(merge_block), expression.span));
    }

    lowerer.switch_to(right_block);
    let right_value = lowerer.lower_expression(right);
    let right_condition = lower_condition_register(lowerer, right, right_value, expression.span);
    lowerer.emit(TacInstr::with_dst(
        TacOp::Copy(right_condition),
        result,
        expression.span,
    ));
    lowerer.flush_temporaries(expression.span);
    if !lowerer.current_block_terminated() {
        lowerer.emit(TacInstr::new(TacOp::Jump(merge_block), expression.span));
    }

    lowerer.switch_to(merge_block);
    result
}

/// 把动态/未决类型的条件值转成 bool；静态 bool 直接复用原寄存器。
fn lower_condition_register(
    lowerer: &mut Lowerer<'_>,
    condition: &IrExpression,
    value: VReg,
    span: IrSpan,
) -> VReg {
    if matches!(condition.ty, IrType::Dynamic | IrType::Variable { .. }) {
        let result = lowerer.new_register(RegisterClass::Bool, span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::Cast {
                value,
                target: ScalarType::Bool,
            },
            result,
            span,
        ));
        result
    } else {
        value
    }
}

/// 判断 IR 类型是否是集合形状或运行时才确定的动态边界。
fn is_set_shape(ty: &IrType) -> bool {
    matches!(
        ty,
        IrType::Set { .. } | IrType::Dynamic | IrType::Variable { .. }
    )
}

/// 判断二元运算是否应使用集合代数指令。
fn is_set_operation_candidate(operator: &str, left: &IrType, right: &IrType) -> bool {
    match operator {
        "&" | "^" => is_set_shape(left) || is_set_shape(right),
        "+" | "-" => matches!(left, IrType::Set { .. }) || matches!(right, IrType::Set { .. }),
        _ => false,
    }
}

/// 判断二元比较是否应使用集合关系指令。
fn is_set_comparison_candidate(operator: &str, left: &IrType, right: &IrType) -> bool {
    match operator {
        "in" | "not in" => is_set_shape(right),
        "==" | "!=" | "<" | "<=" | ">" | ">=" => {
            matches!(left, IrType::Set { .. }) || matches!(right, IrType::Set { .. })
        }
        _ => false,
    }
}

/// 映射集合代数运算符。
fn set_op(operator: &str) -> Option<SetOpKind> {
    Some(match operator {
        "+" => SetOpKind::Union,
        "&" => SetOpKind::Intersection,
        "-" => SetOpKind::Difference,
        "^" => SetOpKind::SymmetricDifference,
        _ => return None,
    })
}

/// 映射集合关系运算符；成员判断方向由 `SetCompareOp::Member` 固定。
fn set_compare(operator: &str) -> Option<SetCompareOp> {
    Some(match operator {
        "==" => SetCompareOp::Equal,
        "!=" => SetCompareOp::NotEqual,
        "<" => SetCompareOp::ProperSubset,
        "<=" => SetCompareOp::Subset,
        ">" => SetCompareOp::ProperSuperset,
        ">=" => SetCompareOp::Superset,
        "in" => SetCompareOp::Member,
        "not in" => SetCompareOp::NotMember,
        _ => return None,
    })
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
    if let IrExpressionKind::Member { object, member } = &callee.kind {
        if let Some(register) = lowerer.lower_method_call(object, member, arguments, expression) {
            return register;
        }
    }
    if is_random_seed_call(callee) {
        let Some(argument) = arguments.first() else {
            return lower_unsupported(lowerer, "random.seed 参数", expression.span);
        };
        let Some(plan) = lowerer.random_seed_plan_id(expression.span) else {
            return lower_unsupported(lowerer, "random.seed 计划", expression.span);
        };
        let value = lowerer.lower_expression(&argument.value);
        lowerer.emit(TacInstr::new(
            TacOp::RandomSeed { value, plan },
            expression.span,
        ));
        let register = lowerer.new_register(RegisterClass::None, expression.span);
        lowerer.emit(TacInstr::with_dst(
            TacOp::LoadNone,
            register,
            expression.span,
        ));
        return register;
    }
    let mut arguments = arguments
        .iter()
        .map(|argument| {
            let value = lowerer.lower_expression(&argument.value);
            // `*`/`**` 展开尚未降低。只按 `name.is_some()` 区分关键字与位置会
            // 把展开实参静默当成普通实参，语义丢失且不报错。
            if argument.kind != "positional" && argument.kind != "keyword" {
                lowerer.record_known_unsupported(format!(
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
    let explicit_arguments = arguments.clone();
    lowerer.append_function_defaults(callee, &explicit_arguments, &mut arguments);
    let class = Lowerer::class_of_type(&expression.ty);
    let register = lowerer.new_register(class, expression.span);
    let op = match lowerer.function_index_of_expression(callee) {
        Some(target) => {
            let signature = lowerer
                .signature_of_function(target)
                .unwrap_or_else(|| lowerer.signatures.intern(crate::sig::CallSig::dynamic()));
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

/// 将契约表登记的 intrinsic 降低为现有 Call 编码。
///
/// FuncId(0) 在生产 TAC 中是脚本入口，不能作为普通被调函数；这里把它
/// 保留为 intrinsic 标记，SigId 直接携带固定宽度的 IntrinsicId。这样
/// 不增加 09R3 opcode；稳定 u32 ID 拆入既有调用签名的两个 u16 槽位，Leb128
/// 和定宽目标都复用原有编码字段。
fn lower_intrinsic_call(
    lowerer: &mut Lowerer<'_>,
    id: u32,
    arguments: &[IrCallArgument],
    expression: &IrExpression,
) -> VReg {
    if xiao_intrinsics::IntrinsicId::new(id)
        .and_then(xiao_intrinsics::active_by_id)
        .is_none()
    {
        lowerer.record_unsupported(format!("未知 intrinsic ID {id}"));
        return lower_unsupported(lowerer, "intrinsic", expression.span);
    }
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
    let signature = lowerer
        .signatures
        .intern(crate::sig::CallSig::intrinsic(id));
    lowerer.emit(TacInstr::with_dst(
        TacOp::Call {
            callee: crate::tac::FuncId::new(0),
            signature,
            arguments,
        },
        register,
        expression.span,
    ));
    register
}

/// 判断 IR 调用是否为内建 `random.seed`。
fn is_random_seed_call(callee: &IrExpression) -> bool {
    let IrExpressionKind::Member { object, member } = &callee.kind else {
        return false;
    };
    matches!(&object.kind, IrExpressionKind::Name { name } if !name.backticked && name.text == "random")
        && !member.backticked
        && member.text == "seed"
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
    // `str -> bool` 的 RuntimeCheck 必须在 Cast 前消费：Cast 本身复用 Runtime
    // 转换矩阵，非法拼写若先到达它会得到泛化的 invalid_value，而不是冻结的
    // 类型错误码。其它转换仍由表达式末尾的统一检查入口处理。
    if target == ScalarType::Bool
        && matches!(
            &inner.ty,
            IrType::Scalar { name } if ScalarType::from_name(name) == Some(ScalarType::Str)
        )
    {
        lowerer.emit_runtime_checks(expression.span, source);
    }
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

/// 降低一个可达但已登记的未实现构造。
fn lower_known_unsupported(lowerer: &mut Lowerer<'_>, what: &str, span: IrSpan) -> VReg {
    lowerer.record_known_unsupported(format!("{what} 尚未降低（{}..{}）", span.start, span.end));
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

/// 集合候选分派的边界回归测试。
#[cfg(test)]
mod tests {
    use super::{is_set_comparison_candidate, is_set_operation_candidate};
    use xiao_ir::IrType;

    #[test]
    /// 未决类型变量与 `Dynamic` 一样属于运行时集合形状边界。
    fn unresolved_type_variables_keep_set_only_operators_and_membership_lowerable() {
        let variable = IrType::Variable { id: 1 };
        let scalar = IrType::Scalar {
            name: "int".to_owned(),
        };

        assert!(is_set_operation_candidate("&", &variable, &variable));
        assert!(is_set_operation_candidate("^", &variable, &variable));
        assert!(is_set_comparison_candidate("in", &scalar, &variable));
        assert!(is_set_comparison_candidate("not in", &scalar, &variable));

        // `+`、`-` 和普通比较仍有标量语义；没有静态集合锚点时不能抢先改走集合指令。
        assert!(!is_set_operation_candidate("+", &variable, &variable));
        assert!(!is_set_comparison_candidate("==", &variable, &variable));
    }
}
