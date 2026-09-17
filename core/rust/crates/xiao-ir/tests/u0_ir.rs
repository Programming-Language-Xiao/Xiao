//! 08-U0 类型化 IR 降低、快照和验证规格。

use xiao_ir::{
    IrExpressionKind, IrStatementKind, IrType, IrValidator, SnapshotError, from_json,
    lower_program, to_json,
};
use xiao_lifetime::analyze as analyze_lifetime;
use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::check;

/// 解析、类型检查并降低一个无错误源码样例。
fn lower(source_text: &str) -> xiao_ir::IrProgram {
    let source = SourceFile::from_text(source_text);
    let parsed = Parser::new(&source).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "语法诊断: {:?}",
        parsed.diagnostics
    );
    let program = parsed.program.expect("程序");
    let typed = check(&source, &program);
    assert!(
        typed.diagnostics.is_empty(),
        "类型诊断: {:?}",
        typed.diagnostics
    );
    let lifetime = analyze_lifetime(&source, &program, &typed);
    assert!(
        lifetime.diagnostics.is_empty(),
        "生命周期诊断: {:?}",
        lifetime.diagnostics
    );
    lower_program(&source, &program, &typed, &lifetime, None)
}

#[test]
/// 递归降低应保留标量、函数、集合、选择器和错误控制流的结构。
fn lowers_complete_static_surface() {
    let source = "values = [1, 2, 3]\n".to_owned()
        + "items = {1, 2}\n"
        + "def f(int value) -> int\n    return value\n"
        + "try\n    result = f(values[0])\ncatch err as Error\n    result = 0\nfinally\n    done = true\n";
    let ir = lower(&source);
    assert!(IrValidator::new().validate(&ir).is_success());
    assert_eq!(ir.body.len(), 4);
    assert!(matches!(
        ir.body[0].kind,
        IrStatementKind::Assignment { .. }
    ));
    assert!(matches!(
        ir.body[1].kind,
        IrStatementKind::Assignment { .. }
    ));
    assert!(matches!(ir.body[2].kind, IrStatementKind::Function { .. }));
    let IrStatementKind::Try { catches, .. } = &ir.body[3].kind else {
        panic!("expected try");
    };
    assert_eq!(catches.len(), 1);
}

#[test]
/// 表达式类型应来自类型阶段，未知位置才保守使用 dynamic。
fn carries_type_result_without_reinference() {
    let ir = lower("value = 1 + 2\n");
    let IrStatementKind::Assignment { value, .. } = &ir.body[0].kind else {
        panic!("expected assignment");
    };
    assert_eq!(
        value.ty,
        IrType::Scalar {
            name: "int".to_owned()
        }
    );
    assert!(matches!(value.kind, IrExpressionKind::Binary { .. }));
}

#[test]
/// JSON 快照应可往返，并在版本不一致时明确拒绝。
fn snapshot_round_trip_and_version_guard() {
    let ir = lower("value = \"hello\"\n");
    let json = to_json(&ir).expect("snapshot");
    let restored = from_json(&json).expect("restore");
    assert_eq!(restored, ir);
    let bad = json.replace("\"version\":1", "\"version\":99");
    assert!(matches!(
        from_json(&bad),
        Err(SnapshotError::UnsupportedVersion(99))
    ));
}

#[test]
/// 验证器应拒绝不存在的控制流后继，而不是让后端接收坏 IR。
fn validator_rejects_broken_control_flow() {
    let mut ir = lower("value = 1\n");
    ir.control_flow.entry = Some(999);
    let result = IrValidator::new().validate(&ir);
    assert!(!result.is_success());
    assert_eq!(result.errors()[0].code, xiao_ir::IR_INVALID_CODE);
}
