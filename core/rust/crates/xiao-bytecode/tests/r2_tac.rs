//! 09R2 统一三地址降低规格。

use xiao_bytecode::research::{
    ArithOp, OperandWidth, PathStep, RegisterClass, SetCompareOp, SetOpKind, SigId, TacConstant,
    TacOp, TacProgram, VReg, encode, jump_targets, lower_program, verify_program,
};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_ir::{
    IrExpression, IrExpressionKind, IrProgram, IrRuntimeCheck, IrSpan, IrStatementKind, IrType,
};

/// 从 Xiao 源码编译出已验证的 IR。
fn compile(source_text: &str) -> IrProgram {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source_text))
        .unwrap_or_else(|error| panic!("前端应成功: {:?}", error.diagnostics()));
    artifact.ir
}

/// 降低并验证一份源码，返回 IR 与三地址产物。
fn lower(source_text: &str) -> (IrProgram, TacProgram) {
    let ir = compile(source_text);
    let tac = lower_program(&ir);
    (ir, tac)
}

#[test]
/// 标量赋值与算术应降低为常量加载、算术和搬运指令。
fn lowers_scalar_arithmetic() {
    let (ir, tac) = lower("value = 1 + 2\n");
    let ops = tac.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .map(|instruction| &instruction.op)
        .collect::<Vec<_>>();
    assert!(
        ops.iter().any(|op| matches!(op, TacOp::LoadConst(_))),
        "应加载常量"
    );
    assert!(
        ops.iter().any(|op| matches!(op, TacOp::Arith { .. })),
        "应有算术指令"
    );
    let verification = verify_program(&ir, &tac);
    assert!(
        verification.is_success(),
        "验证错误: {:?}",
        verification.errors
    );
}

#[test]
/// 函数定义各自成为独立函数，脚本入口保持索引 0。
fn lowers_functions_separately() {
    let (ir, tac) =
        lower("def add(int left, int right) -> int\n    return left + right\nresult = add(1, 2)\n");
    assert_eq!(tac.functions.len(), 2);
    assert_eq!(tac.functions[0].name, "");
    assert_eq!(tac.functions[1].name, "add");
    assert_eq!(tac.functions[1].parameters.len(), 2);
    let verification = verify_program(&ir, &tac);
    assert!(
        verification.is_success(),
        "验证错误: {:?}",
        verification.errors
    );
}

#[test]
/// 调用点携带合成的签名，补齐 IR 缺失的形参类别信息。
fn synthesizes_call_signatures() {
    let (_, tac) =
        lower("def add(int left, int right) -> int\n    return left + right\nresult = add(1, 2)\n");
    assert!(!tac.signatures.is_empty());
    let signature = tac.signatures.get(SigId::new(0)).expect("签名应存在");
    assert_eq!(signature.parameter_types.len(), 2);
}

#[test]
/// 释放计划在退出点上被引用，且引用的计划都真实存在。
fn references_existing_release_plans() {
    let (ir, tac) = lower("value = \"x\"\n");
    let referenced = tac
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter(|instruction| matches!(instruction.op, TacOp::RunReleasePlan { .. }))
        .count();
    assert!(referenced > 0, "堆值程序应引用释放计划");
    let verification = verify_program(&ir, &tac);
    assert!(
        verification.is_success(),
        "验证错误: {:?}",
        verification.errors
    );
}

#[test]
/// 对象句柄与整数应落在不同寄存器类别里。
fn assigns_register_classes() {
    let (_, tac) = lower("text = \"x\"\nnumber = 1\n");
    let function = &tac.functions[0];
    let classes = function
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| instruction.dst)
        .map(|register| function.categories.get(register))
        .collect::<Vec<_>>();
    assert!(classes.contains(&RegisterClass::ObjHandle));
    assert!(classes.contains(&RegisterClass::Int));
}

#[test]
/// 每个函数的局部寄存器编号可以重叠，但类别不得跨函数合并污染。
fn keeps_register_classes_local_to_each_function() {
    let (_, tac) = lower(
        "def number() -> int\n    value = 1\n    return value\ndef text() -> str\n    value = \"x\"\n    return value\nleft = number()\nright = text()\n",
    );
    let number = tac
        .functions
        .iter()
        .find(|function| function.name == "number")
        .expect("number 函数应存在");
    let text = tac
        .functions
        .iter()
        .find(|function| function.name == "text")
        .expect("text 函数应存在");
    assert_eq!(number.categories.get(VReg::new(0)), RegisterClass::Int);
    assert_eq!(text.categories.get(VReg::new(0)), RegisterClass::ObjHandle);
    assert_eq!(number.categories.get(VReg::new(0)), RegisterClass::Int);
}

#[test]
/// `if` 与 `while` 应产生条件分支，且跳转目标全部存在。
fn rebuilds_control_flow_blocks() {
    let (ir, tac) = lower(
        "value = 0\nwhile value != 3\n    value = value + 1\nif value == 3\n    done = true\n",
    );
    let branches = tac.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter(|instruction| matches!(instruction.op, TacOp::BranchIf { .. }))
        .count();
    assert!(branches >= 2, "循环与条件各应产生一次条件分支");
    let verification = verify_program(&ir, &tac);
    assert!(
        verification.is_success(),
        "验证错误: {:?}",
        verification.errors
    );
}

#[test]
/// `try` 现在必须生成处理器表与 finally 子程序，而不是静默跳过。
fn lowers_exception_handlers_and_finally_subroutine() {
    let (ir, tac) =
        lower("try\n    value = 1\ncatch err as Error\n    value = 2\nfinally\n    done = true\n");
    let verification = verify_program(&ir, &tac);
    assert!(
        verification.is_success(),
        "验证错误: {:?}",
        verification.errors
    );
    assert!(
        tac.functions[0]
            .handlers
            .iter()
            .any(|handler| { handler.catch_type.as_deref() == Some("Error") })
    );
    assert!(
        tac.functions[0]
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .any(|instruction| matches!(instruction.op, TacOp::CallSub { .. }))
    );
}

#[test]
/// 尚未支持的 RuntimeCheck 必须留在产物的 `unsupported`，不能被静默吞掉。
fn records_unsupported_runtime_checks() {
    let (_, tac) = lower("str raw = input(\"value\")\nbool parsed = raw as bool\n");
    assert!(
        tac.unsupported
            .iter()
            .any(|note| note.contains("string_boolean")),
        "string_boolean 检查应明确登记为未支持: {:?}",
        tac.unsupported
    );
}

#[test]
/// 容器字面量应降低为容器构造指令，且不再记为未支持。
fn lowers_container_literals() {
    let (ir, tac) = lower("values = [1, 2]\npair = (1, 2)\ntags = {1, 2}\n");
    let ops = tac.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .map(|instruction| &instruction.op)
        .collect::<Vec<_>>();
    assert!(ops.iter().any(|op| matches!(op, TacOp::NewArray { .. })));
    assert!(ops.iter().any(|op| matches!(op, TacOp::NewTuple { .. })));
    assert!(ops.iter().any(|op| matches!(op, TacOp::NewSet { .. })));
    let verification = verify_program(&ir, &tac);
    assert!(
        verification.is_success(),
        "验证错误: {:?}",
        verification.errors
    );
}

#[test]
/// 高级选择应携带类型阶段计划并降低为独立选择指令。
fn lowers_advanced_selector_plan() {
    let (ir, tac) = lower("values = [1, 2, 3, 4]\nselected = values[0, 2]\n");
    assert_eq!(ir.selection_plans.len(), 1);
    assert!(tac.selection_plans.len() == 1);
    assert!(
        tac.functions[0]
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .any(|instruction| matches!(instruction.op, TacOp::SelectorApply { .. }))
    );
    assert!(
        tac.unsupported.is_empty(),
        "不应有未支持项: {:?}",
        tac.unsupported
    );
}

#[test]
/// 精确索引应降低为带路径的读取指令，负索引保留有符号语义。
fn lowers_exact_index_with_signed_step() {
    let (ir, tac) = lower("values = [1, 2]\nfirst = values[0]\nlast = values[-1]\n");
    let paths = tac.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match &instruction.op {
            TacOp::IndexGet { path, .. } => Some(path.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 2, "两次精确索引各产生一条读取指令");
    assert_eq!(paths[0], vec![PathStep::Index(0)]);
    assert_eq!(paths[1], vec![PathStep::Index(-1)], "负索引保留符号");
    let verification = verify_program(&ir, &tac);
    assert!(
        verification.is_success(),
        "验证错误: {:?}",
        verification.errors
    );
}

#[test]
/// 字面量产生的临时堆值必须在消费后被显式释放。
///
/// `Release` 指令只由临时值释放路径发出（释放计划走 `RunReleasePlan`），
/// 因此它的出现本身就证明泄漏修复生效。
fn releases_temporary_heap_values() {
    let (ir, tac) = lower("def sink(str value) -> str\n    return value\nresult = sink(\"x\")\n");
    let entry = &tac.functions[0];
    let releases = entry
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter(|instruction| matches!(instruction.op, TacOp::Release { .. }))
        .count();
    assert!(releases > 0, "传参的字面量临时值应被释放");
    let verification = verify_program(&ir, &tac);
    assert!(
        verification.is_success(),
        "验证错误: {:?}",
        verification.errors
    );
}

#[test]
/// 字符串字面量必须按与类型层同一套规则进入常量池。
///
/// 只剥引号而不解转义会让 `"a\tb"` 变成反斜杠加 t，而类型层看到的是制表符；
/// 这是与字典键同一类的跨层不一致。
fn decodes_string_literals_like_the_type_layer() {
    let (_, tac) = lower("text = \"a\\tb\"\n");
    let texts = tac
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match &instruction.op {
            TacOp::LoadConst(id) => match tac.constants.get(*id) {
                Some(TacConstant::Str(text)) => Some(text.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    // 源码里的 `\\t` 是转义；常量池必须存制表符，而不是反斜杠加 t。
    assert_eq!(texts, vec!["a\tb".to_owned()], "转义必须与类型层同样解码");
}

#[test]
/// 内容本身以引号开头结尾的字符串不得被误剥。
///
/// `unquote` 按「首尾是引号就切掉」判断，对已经解码的字面量会吃掉真实内容。
fn keeps_quote_characters_inside_string_literals() {
    let (_, tac) = lower("text = '\"x\"'\n");
    let texts = tac
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match &instruction.op {
            TacOp::LoadConst(id) => match tac.constants.get(*id) {
                Some(TacConstant::Str(text)) => Some(text.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, vec!["\"x\"".to_owned()], "内容里的引号必须保留");
}

#[test]
/// 转义表必须与类型层单点共用，不能各自维护一张。
///
/// 词法层接受 `\\ \\' \\" n r t 0 b f v a` 十一类转义；类型层、常量折叠和 IR
/// 降低只要有一处漏解或多解，同一个字面量就会在不同层得到不同文本。
fn shares_the_escape_table_with_the_type_layer() {
    // `\0` 与 `\v` 曾经只在类型层被拒绝、在 IR 层被原样保留。
    let (_, tac) = lower("text = \"a\\0b\\vc\"\n");
    let texts = tac
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match &instruction.op {
            TacOp::LoadConst(id) => match tac.constants.get(*id) {
                Some(TacConstant::Str(text)) => Some(text.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, vec!["a\0b\u{b}c".to_owned()]);
    // 转义表的唯一来源。
    assert_eq!(xiao_types::decode_escape('v'), Some('\u{b}'));
    assert_eq!(xiao_types::decode_escape('q'), None);
    assert_eq!(xiao_types::decode_escape('\\'), Some('\\'));
}

#[test]
/// 一元 `not` 必须与 `false` 比较，不能与自身比较。
///
/// 与自身比较恒为真，是静默算错——程序照跑，结果全反。
fn unary_not_compares_against_false() {
    let (_, tac) = lower("flag = true\nvalue = not flag\n");
    let compares = tac.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match &instruction.op {
            TacOp::Compare { left, right, .. } => Some((*left, *right)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        compares.iter().all(|(left, right)| left != right),
        "比较的两个操作数不得相同：{compares:?}"
    );
}

#[test]
/// 一元负号不得退化成 `as int`，否则浮点会被静默截断。
fn unary_minus_keeps_float_width() {
    let (_, tac) = lower("value = -1.5\n");
    let ops = tac
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .map(|instruction| &instruction.op)
        .collect::<Vec<_>>();
    assert!(
        ops.iter().any(|op| matches!(
            op,
            TacOp::Arith {
                op: ArithOp::Subtract,
                ..
            }
        )),
        "一元负号应降低为 `0 - x`"
    );
    assert!(
        !ops.iter().any(|op| matches!(op, TacOp::Cast { .. })),
        "不得把浮点负号静默转成整数"
    );
    let zeros = tac
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match &instruction.op {
            TacOp::LoadConst(id) => tac.constants.get(*id),
            _ => None,
        })
        .filter(|constant| matches!(constant, TacConstant::Float(value) if *value == 0.0))
        .count();
    assert_eq!(zeros, 1, "零常量必须与操作数同宽度（浮点）");
}

#[test]
/// 跨宽度数值运算必须由后端插入显式转换。
///
/// 静态提升规则允许 `int + sint`，运行时不隐式提升；缺了这一步就会静态
/// 通过、运行时以「宽度不一致」拒绝。
fn promotes_cross_width_operands() {
    let (_, tac) = lower("value = 1 + sint(2)\n");
    let casts = tac
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter(|instruction| matches!(instruction.op, TacOp::Cast { .. }))
        .count();
    assert!(casts > 0, "跨宽度运算应插入显式转换");
}

#[test]
/// 反引号名称与普通名称是**不同**的绑定，不得混为一谈。
///
/// 同时接受 `ascii:` 与 `backtick:` 两个前缀会把 `foo` 与 `` `foo` `` 当成
/// 同一个绑定，引用到另一个的值上。
fn separates_backticked_from_plain_bindings() {
    let (ir, tac) = lower("foo = \"a\"\n`foo` = \"b\"\n");
    let targets = tac.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter(|instruction| matches!(instruction.op, TacOp::Move(_) | TacOp::Copy(_)))
        .filter_map(|instruction| instruction.dst)
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 2, "两个赋值各写一个绑定");
    assert_ne!(targets[0], targets[1], "两个绑定必须落在不同寄存器");
    let verification = verify_program(&ir, &tac);
    assert!(
        verification.is_success(),
        "验证错误: {:?}",
        verification.errors
    );
}

#[test]
/// 集合代数和比较的动态边界必须分别给左右操作数发检查。
fn lowers_set_checks_for_both_operands() {
    let (_, tac) =
        lower("left = set()\nright = {1}\nunion = left + right\nequal = left == right\n");
    let checks = tac.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match &instruction.op {
            TacOp::Check { kind, value, .. } => Some((kind.as_str(), *value)),
            _ => None,
        })
        .collect::<Vec<_>>();
    for kind in ["set_operation", "set_comparison"] {
        let values = checks
            .iter()
            .filter(|(check_kind, _)| *check_kind == kind)
            .map(|(_, value)| *value)
            .collect::<Vec<_>>();
        assert_eq!(values.len(), 2, "{kind} 必须检查左右两个操作数");
        assert_ne!(values[0], values[1], "{kind} 的左右检查不能绑定同一寄存器");
    }
    assert!(
        tac.unsupported.is_empty(),
        "不应留下未支持项: {:?}",
        tac.unsupported
    );
}

#[test]
/// 集合运算嵌套在选择器源中时，检查必须在二元表达式处完整消费。
fn nested_set_operation_does_not_bind_check_to_selector_source() {
    let mut ir = compile("values = [1, 2, 3]\nselected = values[0, 1]\n");
    let binary_span = IrSpan::new(10_000, 10_010);
    let mut injected = false;
    for statement in &mut ir.body {
        let IrStatementKind::Assignment { target, value } = &mut statement.kind else {
            continue;
        };
        if target.text != "selected" {
            continue;
        }
        let IrExpressionKind::Selector { source, .. } = &mut value.kind else {
            continue;
        };
        // 当前静态规则拒绝直接索引集合；这里把一个已验证选择器的源替换成
        // 动态集合二元表达式，只为隔离测试 lowering 的检查消费边界。
        let mut left = (**source).clone();
        left.ty = IrType::Set {
            members: Vec::new(),
            allows_dynamic: true,
            empty: false,
            unknown: true,
        };
        let mut right = left.clone();
        right.ty = IrType::Dynamic;
        **source = IrExpression {
            kind: IrExpressionKind::Binary {
                operator: "+".to_owned(),
                left: Box::new(left),
                right: Box::new(right),
            },
            ty: IrType::Dynamic,
            span: binary_span,
        };
        injected = true;
        break;
    }
    assert!(injected, "应找到选择器源表达式");
    ir.runtime_checks.push(IrRuntimeCheck {
        kind: "set_operation".to_owned(),
        span: binary_span,
    });
    let tac = lower_program(&ir);
    let selector_source = tac.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .find_map(|instruction| match instruction.op {
            TacOp::SelectorApply { source, .. } => Some(source),
            _ => None,
        })
        .expect("应有高级选择指令");
    let operation_checks = tac.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match &instruction.op {
            TacOp::Check { kind, value, .. } if kind == "set_operation" => Some(*value),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        operation_checks.len(),
        2,
        "集合运算检查必须完整消费: {operation_checks:?}"
    );
    assert!(
        operation_checks
            .iter()
            .all(|value| *value != selector_source),
        "集合检查不得被选择器源寄存器接管: {operation_checks:?}, source={selector_source:?}"
    );
}

#[test]
/// 集合指令没有显式控制流边；这一前提由 CFG 公共入口固定下来。
fn set_instructions_have_no_jump_targets() {
    let left = VReg::new(1);
    let right = VReg::new(2);
    assert!(
        jump_targets(&TacOp::SetOp {
            op: SetOpKind::Union,
            left,
            right,
        })
        .is_empty()
    );
    assert!(
        jump_targets(&TacOp::SetCompare {
            op: SetCompareOp::Equal,
            left,
            right,
        })
        .is_empty()
    );
}

#[test]
/// 含集合运算的降低产物必须能通过研究编码器验证，而不是只覆盖拒绝路径。
fn encodes_program_using_set_operation() {
    let (_, tac) = lower("value = {1} + {2}\n");
    assert!(
        tac.unsupported.is_empty(),
        "不应留下未支持项: {:?}",
        tac.unsupported
    );
    assert!(encode(&tac, OperandWidth::Leb128).is_ok());
    assert!(encode(&tac, OperandWidth::FixedU16).is_ok());
}

#[test]
/// 复合集合赋值的检查跨度与类型层登记点一致，不能残留为 unsupported。
fn consumes_runtime_checks_for_set_compound_assignment() {
    let (_, tac) = lower("set<int> target = set()\nsource = set()\ntarget += source\n");
    assert!(
        tac.functions[0]
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .any(|instruction| matches!(instruction.op, TacOp::SetOp { .. }))
    );
    assert!(
        tac.unsupported.is_empty(),
        "复合集合赋值的检查不应遗留: {:?}",
        tac.unsupported
    );
}
