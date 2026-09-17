//! 09R2 统一三地址降低规格。

use xiao_bytecode::research::{
    ArithOp, PathStep, RegisterClass, SigId, TacConstant, TacOp, TacProgram, lower_program,
    verify_program,
};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_ir::IrProgram;

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
    let classes = tac
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| instruction.dst)
        .map(|register| tac.categories.get(register))
        .collect::<Vec<_>>();
    assert!(classes.contains(&RegisterClass::ObjHandle));
    assert!(classes.contains(&RegisterClass::Int));
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
/// 本批次尚未降低的构造必须被显式记录，而不是静默跳过。
fn records_unsupported_constructs() {
    let (ir, tac) = lower("try\n    value = 1\ncatch err as Error\n    value = 2\n");
    let verification = verify_program(&ir, &tac);
    assert!(!verification.is_success(), "未降低的 try 不应被当成成功");
    assert!(!verification.unsupported.is_empty());
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
