//! N0-B 动态表 ABI 降低回归测试。

use xiao_codegen_llvm::{
    CodegenOptions, EntryObservation, NativeBuild, TargetDescription, Toolchain, lower_program,
};
use xiao_ir::{
    IrArrayShape, IrDictEntry, IrEntryMode, IrExpression, IrExpressionKind, IrName, IrProgram,
    IrRuntimeCheck, IrSpan, IrStatement, IrStatementKind, IrTableMember, IrTableSignature, IrType,
};

/// 构造一段最小测试源码区间。
fn span() -> IrSpan {
    IrSpan::new(0, 1)
}

/// 构造未加引号的测试名称。
fn name(text: &str) -> IrName {
    IrName {
        text: text.to_owned(),
        backticked: false,
        span: span(),
    }
}

/// 构造一个固定标量类型的测试字面量。
fn literal(literal: &str, text: &str, ty: &str) -> IrExpression {
    IrExpression {
        kind: IrExpressionKind::Literal {
            literal: literal.to_owned(),
            text: text.to_owned(),
        },
        ty: IrType::Scalar {
            name: ty.to_owned(),
        },
        span: span(),
    }
}

/// 把一个动态表达式包装为最小脚本程序。
fn expression_program(value: IrExpression) -> IrProgram {
    IrProgram::new(
        IrEntryMode::Script,
        vec![IrStatement {
            kind: IrStatementKind::Expression { value },
            span: span(),
            leading_docs: Vec::new(),
        }],
        span(),
    )
}

/// 在设置了 `XIAO_LLVM_AS` 时用真实汇编器校验动态模块。
fn validate_with_llvm_as(text: &str) {
    let Some(llvm_as) = std::env::var_os("XIAO_LLVM_AS") else {
        return;
    };
    NativeBuild::new()
        .validate_llvm(
            text,
            &TargetDescription::host(),
            &Toolchain::new("unused").with_llvm_as(llvm_as),
        )
        .expect("llvm-as 应接受动态模块");
}

/// 构造包含一个字段和无参 `new` 的最小表程序。
fn table_program(kind: &str) -> IrProgram {
    let mut program = IrProgram::new(
        IrEntryMode::Script,
        vec![IrStatement {
            kind: IrStatementKind::Expression {
                value: IrExpression {
                    kind: IrExpressionKind::NewCall {
                        callee: Box::new(IrExpression {
                            kind: IrExpressionKind::Name {
                                name: name("Record"),
                            },
                            ty: IrType::Table {
                                name: "Record".to_owned(),
                                kind: "constructor".to_owned(),
                            },
                            span: span(),
                        }),
                        arguments: Vec::new(),
                    },
                    ty: IrType::Table {
                        name: "Record".to_owned(),
                        kind: "instance".to_owned(),
                    },
                    span: span(),
                },
            },
            span: span(),
            leading_docs: Vec::new(),
        }],
        span(),
    );
    program.table_signatures = vec![IrTableSignature {
        name: "Record".to_owned(),
        kind: kind.to_owned(),
        members: vec![IrTableMember {
            name: "count".to_owned(),
            method: false,
            public: true,
            ty: IrType::Scalar {
                name: "int".to_owned(),
            },
            span: span(),
        }],
        span: span(),
    }];
    program
}

#[test]
/// 表描述符必须携带字段名、字段类型、可见性和表形态，而不是全零占位。
fn emits_complete_table_descriptor() {
    let module = lower_program(&table_program("instance"), &CodegenOptions::default())
        .expect("动态表应降低");
    assert!(module.text.contains("%xiao.table.field = type"));
    assert!(module.text.contains("c\"count\\00\""));
    assert!(module.text.contains("i32 1"));
    assert!(module.text.contains("i8 1"));
    assert!(module.text.contains("insertvalue %xiao.table.descriptor"));
    assert!(
        !module
            .text
            .contains("store %xiao.table.descriptor zeroinitializer")
    );
}

#[test]
/// 空字段单例声明也必须构造表值，不能被当成无操作声明而留下空槽。
fn lowers_empty_singleton_declaration() {
    let mut program = IrProgram::new(
        IrEntryMode::Script,
        vec![IrStatement {
            kind: IrStatementKind::Table {
                name: name("Marker"),
                table_kind: "singleton".to_owned(),
                body: Vec::new(),
            },
            span: span(),
            leading_docs: Vec::new(),
        }],
        span(),
    );
    program.table_signatures = vec![IrTableSignature {
        name: "Marker".to_owned(),
        kind: "singleton".to_owned(),
        members: Vec::new(),
        span: span(),
    }];
    let module = lower_program(&program, &CodegenOptions::default()).expect("空字段单例也应降低");
    assert!(module.text.contains("@xiao_runtime_table_new"));
    validate_with_llvm_as(&module.text);
}

#[test]
/// 动态降低器不能静默丢弃 `new` 构造参数。
fn rejects_table_constructor_arguments_until_init_abi_exists() {
    let mut program = table_program("instance");
    let IrStatementKind::Expression { value } = &mut program.body[0].kind else {
        unreachable!();
    };
    let IrExpressionKind::NewCall { arguments, .. } = &mut value.kind else {
        unreachable!();
    };
    arguments.push(xiao_ir::IrCallArgument {
        kind: "positional".to_owned(),
        name: None,
        value: IrExpression {
            kind: IrExpressionKind::Literal {
                literal: "int".to_owned(),
                text: "1".to_owned(),
            },
            ty: IrType::Scalar {
                name: "int".to_owned(),
            },
            span: span(),
        },
        span: span(),
    });
    let error = lower_program(&program, &CodegenOptions::default()).expect_err("应拒绝丢参");
    assert!(error.to_string().contains("构造参数"));
}

#[test]
#[ignore = "需要 XIAO_LLVM_AS 与 XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
/// 动态表模块必须能被真实 LLVM 汇编器解析，避免只验证字符串片段。
fn optional_llvm_accepts_dynamic_table_module() {
    let llvm_as = std::env::var_os("XIAO_LLVM_AS")
        .expect("显式运行 --ignored 时 XIAO_LLVM_AS 必须已设置；准备方式见 10D §4");
    let triple = std::env::var("XIAO_TARGET_TRIPLE")
        .expect("显式运行 --ignored 时 XIAO_TARGET_TRIPLE 必须已设置；准备方式见 10D §4");
    let target = TargetDescription::new(
        triple,
        64,
        xiao_codegen_llvm::Endian::Little,
        xiao_codegen_llvm::ObjectFormat::Coff,
    )
    .expect("XIAO_TARGET_TRIPLE 必须是有效的 64 位 COFF 目标");
    let module = lower_program(&table_program("instance"), &CodegenOptions::default())
        .expect("动态表应降低");
    NativeBuild::new()
        .validate_llvm(
            &module.text,
            &target,
            &Toolchain::new("unused").with_llvm_as(llvm_as),
        )
        .expect("llvm-as 应接受动态表模块");
}

#[test]
/// Windows x64 的 Runtime 聚合值和字节视图必须使用与 Rust `extern "C"` 一致的间接 ABI。
fn emits_windows_indirect_runtime_abi_calls() {
    let module = lower_program(
        &expression_program(literal("str", "\"windows\"", "str")),
        &CodegenOptions::for_target(TargetDescription::windows_x86_64()),
    )
    .expect("Windows 动态字符串应降低");
    assert!(
        module
            .text
            .contains("declare void @xiao_runtime_value_str(ptr sret(%xiao.value) align 8, ptr)")
    );
    assert!(
        module
            .text
            .contains("call void @xiao_runtime_value_str(ptr sret(%xiao.value)")
    );
    assert!(
        module
            .text
            .contains("declare i32 @xiao_runtime_string_new(ptr, ptr)")
    );
}

#[test]
/// 动态数组的元素需逐项复制并释放临时值，且生成文本必须能被 LLVM 汇编器接受。
fn lowers_array_and_preserves_runtime_components() {
    let array = IrExpression {
        kind: IrExpressionKind::Array {
            elements: vec![
                literal("integer", "1", "int"),
                literal("str", "\"x\"", "str"),
            ],
        },
        ty: IrType::Array {
            shape: IrArrayShape::Unknown,
        },
        span: span(),
    };
    let module =
        xiao_codegen_llvm::lower_program(&expression_program(array), &CodegenOptions::default())
            .expect("动态数组应降低");
    assert!(
        module
            .runtime_components
            .iter()
            .any(|item| item == "containers")
    );
    validate_with_llvm_as(&module.text);
}

#[test]
/// 字符串转义必须复用类型层解码，避免原生和字节码看到不同的文本。
fn decodes_string_escapes_before_llvm_emission() {
    let program = expression_program(literal("str", "\"a\\n\\t\\\\b\"", "str"));
    let module = xiao_codegen_llvm::lower_program(&program, &CodegenOptions::default())
        .expect("字符串应降低");
    assert!(module.text.contains("a\\0A\\09\\5Cb"));
    assert!(
        !module
            .runtime_components
            .iter()
            .any(|item| item == "containers")
    );
    assert!(!module.runtime_components.iter().any(|item| item == "weak"));
    validate_with_llvm_as(&module.text);
}

#[test]
/// 字典构造使用已规范化键，并登记容器 ABI 组件。
fn lowers_dictionary_with_runtime_component() {
    let dictionary = IrExpression {
        kind: IrExpressionKind::DictTable {
            entries: vec![IrDictEntry {
                key: "key".to_owned(),
                value: literal("integer", "2", "int"),
                span: span(),
            }],
        },
        ty: IrType::DictTable {
            entries: Vec::new(),
        },
        span: span(),
    };
    let module = xiao_codegen_llvm::lower_program(
        &expression_program(dictionary),
        &CodegenOptions::default(),
    )
    .expect("字典应降低");
    assert!(
        module
            .runtime_components
            .iter()
            .any(|item| item == "containers")
    );
    validate_with_llvm_as(&module.text);
}

#[test]
/// 动态值程序的布尔分支应形成完整的 LLVM 基本块，而不是退回静态布局猜测。
fn lowers_dynamic_boolean_if() {
    let branch = IrStatement {
        kind: IrStatementKind::If {
            condition: literal("bool", "true", "bool"),
            body: vec![IrStatement {
                kind: IrStatementKind::Assignment {
                    target: name("value"),
                    value: literal("str", "\"then\"", "str"),
                },
                span: span(),
                leading_docs: Vec::new(),
            }],
            elif_branches: Vec::new(),
            else_body: Some(vec![IrStatement {
                kind: IrStatementKind::Assignment {
                    target: name("value"),
                    value: literal("str", "\"else\"", "str"),
                },
                span: span(),
                leading_docs: Vec::new(),
            }]),
        },
        span: span(),
        leading_docs: Vec::new(),
    };
    let module = xiao_codegen_llvm::lower_program(
        &IrProgram::new(IrEntryMode::Script, vec![branch], span()),
        &CodegenOptions::default(),
    )
    .expect("动态布尔分支应降低");
    assert!(module.text.contains("dynamic.if.then"));
    assert!(module.text.contains("dynamic.if.merge"));
    validate_with_llvm_as(&module.text);
}

#[test]
/// 动态降低器不能把跨类型 Cast 当作同布局透传，尤其不能跳过字符串布尔检查。
fn rejects_non_identity_dynamic_cast() {
    let cast = IrExpression {
        kind: IrExpressionKind::Cast {
            expression: Box::new(literal("str", "\"text\"", "str")),
            target: "bool".to_owned(),
        },
        ty: IrType::Scalar {
            name: "bool".to_owned(),
        },
        span: span(),
    };
    let error = lower_program(&expression_program(cast), &CodegenOptions::default())
        .expect_err("跨类型动态 Cast 必须拒绝");
    assert!(error.to_string().contains("动态 Cast"));
}

#[test]
/// 同类型 identity Cast 可以复用动态值所有权，不应被误判为跨类型转换。
fn accepts_identity_dynamic_cast() {
    let cast = IrExpression {
        kind: IrExpressionKind::Cast {
            expression: Box::new(literal("str", "\"text\"", "str")),
            target: "str".to_owned(),
        },
        ty: IrType::Scalar {
            name: "str".to_owned(),
        },
        span: span(),
    };
    let module = lower_program(&expression_program(cast), &CodegenOptions::default())
        .expect("identity Cast 应降低");
    assert!(module.text.contains("@xiao_runtime_value_str"));
}

#[test]
/// 前端登记但尚未由原生侧消费的 RuntimeCheck 必须结构化拒绝，不能静默丢语义。
fn rejects_unlowered_runtime_check() {
    let mut program = expression_program(literal("str", "\"text\"", "str"));
    program.runtime_checks.push(IrRuntimeCheck {
        kind: "string_boolean".to_owned(),
        span: span(),
        expected: None,
    });
    let error = lower_program(&program, &CodegenOptions::default())
        .expect_err("未消费的 RuntimeCheck 必须拒绝");
    assert!(error.to_string().contains("string_boolean"));
}

#[test]
/// 没有表签名的手工 IR 不能把表体初始化降成无效的空操作。
fn rejects_table_declaration_without_signature() {
    let program = IrProgram::new(
        IrEntryMode::Script,
        vec![IrStatement {
            kind: IrStatementKind::Table {
                name: name("Record"),
                table_kind: "instance".to_owned(),
                body: vec![IrStatement {
                    kind: IrStatementKind::Expression {
                        value: literal("str", "\"initializer\"", "str"),
                    },
                    span: span(),
                    leading_docs: Vec::new(),
                }],
            },
            span: span(),
            leading_docs: Vec::new(),
        }],
        span(),
    );
    let error =
        lower_program(&program, &CodegenOptions::default()).expect_err("表声明体必须结构化拒绝");
    assert!(error.to_string().contains("没有登记签名"));
}

#[test]
/// 动态入口显式要求退出码观察时，生成器必须产生有返回值的入口，而不是静默忽略选项。
fn dynamic_entry_observation_is_explicit() {
    let program = IrProgram::new(
        IrEntryMode::Script,
        vec![
            IrStatement {
                kind: IrStatementKind::Assignment {
                    target: name("text"),
                    value: literal("str", "\"text\"", "str"),
                },
                span: span(),
                leading_docs: Vec::new(),
            },
            IrStatement {
                kind: IrStatementKind::Assignment {
                    target: name("code"),
                    value: literal("integer", "7", "int"),
                },
                span: span(),
                leading_docs: Vec::new(),
            },
        ],
        span(),
    );
    let options = CodegenOptions::default().with_entry_observation(EntryObservation::ExitCode);
    let module = lower_program(&program, &options).expect("动态入口应支持显式观察");
    assert!(module.text.contains("define i64 @xiao_entry"));
    assert!(module.text.contains("call i64 @xiao_entry"));
    assert!(module.text.contains("extractvalue %xiao.value"));
    assert!(module.text.contains("store i64 %t"));
}
