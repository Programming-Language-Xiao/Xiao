//! 08-U0 类型化 IR 降低、快照和验证规格。

use xiao_ir::{
    IrBoundaryTransfer, IrCancellationCheck, IrCrossThreadFact, IrExpressionKind, IrOwnership,
    IrProgram, IrScope, IrSpan, IrStatementKind, IrType, IrValidator, IrValue, SnapshotError,
    from_json, lower_program, lower_type, restore_type, to_json,
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
/// 资源检查缺失或伪造 expected 时必须在进入后端之前拒绝。
fn resource_checks_require_valid_expected_kinds() {
    let mut ir = lower("def take(process value) -> none\n    return\n");
    assert_eq!(ir.runtime_checks.len(), 1);
    assert!(IrValidator::new().validate(&ir).is_success());
    for kind in ["resource_type", "resource_array", "resource_dictionary"] {
        for expected in [
            None,
            Some(IrType::Dynamic),
            Some(IrType::Resource {
                kind: "unknown".to_owned(),
            }),
        ] {
            ir.runtime_checks[0].kind = kind.to_owned();
            ir.runtime_checks[0].expected = expected;
            let result = IrValidator::new().validate(&ir);
            assert!(!result.is_success(), "{ir:?}");
            assert!(
                result
                    .errors()
                    .iter()
                    .any(|error| error.path == "runtime_checks[0].expected")
            );
        }
    }
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

#[test]
/// 并发类型必须能在 IR 与类型层之间递归往返。
fn concurrency_types_lower_and_restore() {
    let scalar = xiao_types::Type::scalar(xiao_syntax::ScalarType::Int);
    let types = vec![
        xiao_types::Type::Thread(Box::new(scalar.clone())),
        xiao_types::Type::Task(Box::new(scalar.clone())),
        xiao_types::Type::Runtime,
        xiao_types::Type::Pool,
        xiao_types::Type::Shared(Box::new(scalar.clone())),
        xiao_types::Type::SharedMut(Box::new(scalar.clone())),
        xiao_types::Type::LockGuard(Box::new(scalar.clone())),
        xiao_types::Type::OrdinaryHandle,
    ];
    for ty in types {
        let lowered = lower_type(&ty);
        assert_eq!(restore_type(&lowered), Some(ty));
        let encoded = serde_json::to_string(&lowered).expect("并发类型 JSON");
        let decoded: IrType = serde_json::from_str(&encoded).expect("并发类型 JSON 解码");
        assert_eq!(decoded, lowered);
    }
}

fn boundary_program(ty: IrType) -> IrProgram {
    let span = IrSpan::new(0, 1);
    let value = IrValue {
        id: 1,
        name: Some("value".to_owned()),
        scope: 0,
        span,
        ty: Some(ty),
        storage: "stack".to_owned(),
        declaration_order: 0,
        parameter: false,
        constant: false,
        temporary: false,
        escapes: vec!["cross_thread".to_owned()],
    };
    let mut program = IrProgram::new(xiao_ir::IrEntryMode::Script, Vec::new(), span);
    program.ownership = IrOwnership {
        scopes: vec![IrScope {
            id: 0,
            parent: None,
            kind: "program".to_owned(),
            span,
            depth: 0,
            values: vec![1],
        }],
        values: vec![value],
        cross_thread_facts: vec![IrCrossThreadFact::new(1, 7, true, true, span)],
        boundary_transfers: vec![IrBoundaryTransfer::deep_copy(vec![1], 7, span)],
        cancellation_checks: vec![IrCancellationCheck::safe_point(span)],
        ..IrOwnership::default()
    };
    program
}

#[test]
/// D-5 的边界事实、复制清理顺序和取消检查应通过统一 verifier。
fn verifies_boundary_facts_and_cleanup_order() {
    let program = boundary_program(IrType::Scalar {
        name: "int".to_owned(),
    });
    assert!(IrValidator::new().validate(&program).is_success());

    let mut invalid = program.clone();
    invalid.ownership.boundary_transfers[0].runtime_id = 8;
    assert!(!IrValidator::new().validate(&invalid).is_success());

    let mut invalid = program.clone();
    invalid.ownership.boundary_transfers[0].cleanup_order = vec![];
    assert!(!IrValidator::new().validate(&invalid).is_success());
}

#[test]
/// 普通句柄、可变共享句柄和锁守卫不能借边界事实逃逸。
fn rejects_non_send_concurrency_handles() {
    for ty in [
        IrType::OrdinaryHandle,
        IrType::SharedMut {
            value: Box::new(IrType::Scalar {
                name: "int".to_owned(),
            }),
        },
        IrType::LockGuard {
            value: Box::new(IrType::Scalar {
                name: "int".to_owned(),
            }),
        },
    ] {
        let program = boundary_program(ty);
        assert!(
            !IrValidator::new().validate(&program).is_success(),
            "不应放行 {program:?}"
        );
    }
}

#[test]
/// 缺失 D-5 新字段的旧快照必须按空容器读取。
fn old_snapshot_without_concurrency_fields_is_compatible() {
    let program = boundary_program(IrType::Scalar {
        name: "int".to_owned(),
    });
    let mut value: serde_json::Value =
        serde_json::from_str(&to_json(&program).expect("快照编码")).expect("JSON");
    let ownership = value
        .get_mut("ownership")
        .and_then(serde_json::Value::as_object_mut)
        .expect("ownership");
    ownership.remove("cross_thread_facts");
    ownership.remove("boundary_transfers");
    ownership.remove("cancellation_checks");
    let restored =
        from_json(&serde_json::to_string(&value).expect("旧快照编码")).expect("旧快照读取");
    assert!(restored.ownership.cross_thread_facts.is_empty());
    assert!(restored.ownership.boundary_transfers.is_empty());
    assert!(restored.ownership.cancellation_checks.is_empty());
}
