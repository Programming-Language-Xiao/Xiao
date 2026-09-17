//! 09R2 统一三地址降低规格。

use xiao_bytecode::research::{
    RegisterClass, SigId, TacOp, TacProgram, lower_program, verify_program,
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
