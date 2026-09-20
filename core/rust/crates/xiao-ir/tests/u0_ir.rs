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
/// 表镜像保留字段类型、私有性和方法接口，函数 IR 另存带 self 的完整签名。
fn mirrors_table_interface_and_typed_receiver() {
    let ir = lower(
        "[[Counter]]\n    _value = 2\n    def read(self) -> int\n        return self._value\n",
    );
    let table = &ir.table_signatures[0];
    let signature = table.runtime_signature().expect("接口可还原");
    let field = signature.member("ascii:_value").unwrap();
    assert!(!field.is_public());
    assert_eq!(
        field.ty,
        xiao_types::Type::scalar(xiao_syntax::ScalarType::Int)
    );
    let method = signature.member("ascii:read").unwrap();
    assert!(method.is_method());
    assert!(
        matches!(&method.ty, xiao_types::Type::Function { parameters, .. } if parameters.is_empty())
    );
    let IrStatementKind::Table { body, .. } = &ir.body[0].kind else {
        panic!("表声明")
    };
    let IrStatementKind::Function { parameters, .. } = &body[1].kind else {
        panic!("方法")
    };
    assert_eq!(
        parameters[0].ty,
        IrType::Table {
            name: "Counter".to_owned(),
            kind: "instance".to_owned()
        }
    );
    assert_eq!(from_json(&to_json(&ir).unwrap()).unwrap(), ir);
    let mut invalid = table.clone();
    invalid.members.push(invalid.members[0].clone());
    assert!(invalid.runtime_signature().is_none());
    invalid = table.clone();
    invalid.members[0].ty = IrType::Scalar {
        name: "invalid".to_owned(),
    };
    assert!(invalid.runtime_signature().is_none());
    let mut invalid_ir = ir.clone();
    invalid_ir.table_signatures[0] = invalid;
    assert!(!IrValidator::new().validate(&invalid_ir).is_success());
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

#[test]
/// 字典键必须在类型层与 IR 层得到同一份规范化文本。
///
/// 两侧曾各写一份：类型层对字符串键去引号并处理转义，IR 却直接存含引号的原始
/// 切片，于是静态能通过的键在运行时查不到。这条用例把两层钉在一起。
fn normalizes_dict_keys_consistently() {
    let ir = lower("mapping = {\"a\" = 1, plain = 2}\n");
    let IrStatementKind::Assignment { value, .. } = &ir.body[0].kind else {
        panic!("expected assignment");
    };
    let IrExpressionKind::DictTable { entries } = &value.kind else {
        panic!("expected dict table literal");
    };
    let keys = entries
        .iter()
        .map(|entry| entry.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(keys, vec!["a", "plain"], "字符串键必须已去掉外围引号");
}

#[test]
/// 带转义的字符串键与转义后的文本一致。
fn normalizes_escaped_dict_keys() {
    let ir = lower("mapping = {\"a\tb\" = 1}\n");
    let IrStatementKind::Assignment { value, .. } = &ir.body[0].kind else {
        panic!("expected assignment");
    };
    let IrExpressionKind::DictTable { entries } = &value.kind else {
        panic!("expected dict table literal");
    };
    assert_eq!(entries[0].key, "a\tb", "转义必须与类型层同样处理");
}
