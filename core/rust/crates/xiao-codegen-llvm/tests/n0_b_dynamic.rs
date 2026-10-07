//! N0-B 动态表 ABI 降低回归测试。

use std::collections::BTreeSet;

use xiao_codegen_llvm::{
    CodegenOptions, EntryObservation, NativeBuild, TargetDescription, Toolchain, lower_program,
};
use xiao_ir::{
    IrArrayShape, IrDictEntry, IrEntryMode, IrExpression, IrExpressionKind, IrName, IrParameter,
    IrProgram, IrRuntimeCheck, IrSpan, IrStatement, IrStatementKind, IrTableMember,
    IrTableSignature, IrType,
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

/// 检查每个 LLVM 函数体内的分支标签引用都有同函数体内的定义。
fn assert_function_labels_are_defined(text: &str) {
    let mut in_function = false;
    let mut labels = BTreeSet::new();
    let mut references = Vec::new();
    for line in text.lines() {
        if line.starts_with("define ") {
            in_function = true;
            labels.clear();
            references.clear();
        }
        if !in_function {
            continue;
        }
        let trimmed = line.trim();
        if let Some(label) = trimmed.strip_suffix(':') {
            labels.insert(label.to_owned());
        }
        let mut rest = trimmed;
        while let Some(index) = rest.find("label %") {
            rest = &rest[index + 7..];
            let label = rest
                .split(|character: char| character == ',' || character.is_whitespace())
                .next()
                .expect("标签引用不能为空");
            references.push(label.to_owned());
            rest = &rest[label.len()..];
        }
        if trimmed == "}" {
            for reference in &references {
                assert!(
                    labels.contains(reference),
                    "LLVM 函数体引用了未定义标签 %{reference}; labels={labels:?}"
                );
            }
            in_function = false;
        }
    }
}

#[test]
/// 函数体动态条件的错误路由必须引用本函数自己的终点标签。
fn function_body_dynamic_condition_has_closed_labels() {
    let parameter = IrParameter {
        name: name("value"),
        kind: "positional".to_owned(),
        ty: IrType::Dynamic,
        default: None,
        span: span(),
    };
    let condition = IrExpression {
        kind: IrExpressionKind::Name {
            name: name("value"),
        },
        ty: IrType::Scalar {
            name: "bool".to_owned(),
        },
        span: span(),
    };
    let mut program = IrProgram::new(
        IrEntryMode::Script,
        vec![IrStatement {
            kind: IrStatementKind::Function {
                name: name("check"),
                parameters: vec![parameter],
                return_type: IrType::Scalar {
                    name: "int".to_owned(),
                },
                body: vec![IrStatement {
                    kind: IrStatementKind::If {
                        condition,
                        body: Vec::new(),
                        elif_branches: Vec::new(),
                        else_body: None,
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
    program.runtime_checks.push(IrRuntimeCheck {
        kind: "boolean_condition".to_owned(),
        span: span(),
        expected: None,
    });
    let module =
        lower_program(&program, &CodegenOptions::default()).expect("函数体动态条件应生成 LLVM");
    assert_function_labels_are_defined(&module.text);
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

/// 默认用结构检查校验每个函数体的标签；有 `XIAO_LLVM_AS` 时再追加真实汇编器校验。
fn validate_with_llvm_as(text: &str) {
    assert_function_labels_are_defined(text);
    let Some(llvm_as) = std::env::var_os("XIAO_LLVM_AS") else {
        eprintln!("N0-B LLVM 校验：未设置 XIAO_LLVM_AS，已完成不依赖外部工具的函数标签结构校验");
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
    let configured_triple = std::env::var("XIAO_TARGET_TRIPLE")
        .expect("显式运行 --ignored 时 XIAO_TARGET_TRIPLE 必须已设置；准备方式见 10D §4");
    let target = TargetDescription::host();
    assert_eq!(
        configured_triple, target.triple,
        "XIAO_TARGET_TRIPLE 必须与当前 Rust 编译目标一致"
    );
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
    assert!(module.text.contains(
        "declare void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) align 8, ptr)"
    ));
    assert!(
        module
            .text
            .contains("call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value)")
    );
    assert!(
        module
            .text
            .contains("declare i32 @xiao_runtime_string_new(ptr, ptr)")
    );
}

#[test]
/// Unix 动态值必须按 C ABI 的两个字段返回，避免把 union payload 拆成错误的第三个寄存器。
fn emits_sysv_runtime_value_layout() {
    let module = lower_program(
        &expression_program(literal("str", "\"linux\"", "str")),
        &CodegenOptions::for_target(TargetDescription::linux_x86_64()),
    )
    .expect("Linux 动态字符串应降低");
    assert!(module.text.contains("%xiao.value = type { i32, i64 }"));
    assert!(
        module
            .text
            .contains("declare %xiao.value @xiao_runtime_value_str_owned(ptr)")
    );
    assert!(!module.text.contains("i32, i32, i64"));
}

/// 构造一个动态表成员读取程序（`Record().count`）。
///
/// 复用 `table_program` 的表签名与构造表达式，只把顶层语句换成成员读取——
/// 这是唯一会走到 `emit_table_get` 的形状。
fn member_read_program() -> IrProgram {
    let mut program = table_program("instance");
    let constructor = program.body.pop().expect("table_program 应当产出一条语句");
    let IrStatementKind::Expression { value: object } = constructor.kind else {
        panic!("table_program 的首条语句应当是表达式");
    };
    program.body.push(IrStatement {
        kind: IrStatementKind::Expression {
            value: IrExpression {
                kind: IrExpressionKind::Member {
                    object: Box::new(object),
                    member: name("count"),
                },
                ty: IrType::Scalar {
                    name: "int".to_owned(),
                },
                span: span(),
            },
        },
        span: span(),
        leading_docs: Vec::new(),
    });
    program
}

#[test]
/// A1 字段读取传入完整值指针；标量观测仍须从两字段 value 的索引 1 读取 payload。
/// 继续钉住历史上的索引 2 越界回归，不要求把弱视图错误地提取为强句柄。
fn dynamic_table_member_uses_value_pointer_and_valid_payload_index() {
    let options = CodegenOptions::default().with_entry_observation(EntryObservation::ExitCode);
    let module = lower_program(&member_read_program(), &options).expect("动态表成员读取应降低");
    assert!(
        module
            .text
            .contains("call i32 @xiao_runtime_table_get_value(ptr")
    );
    assert!(!module.text.contains("call i32 @xiao_runtime_table_get("));
    let indexed: Vec<&str> = module
        .text
        .lines()
        .filter(|line| line.contains("extractvalue %xiao.value"))
        .collect();
    assert!(
        !indexed.is_empty(),
        "标量观测应当产生 payload extractvalue；实际生成：\n{}",
        module.text
    );
    for line in indexed {
        assert!(
            line.trim_end().ends_with(", 1"),
            "extractvalue 必须取索引 1（%xiao.value 是两字段结构 {{ i32, i64 }}），实际：{line}"
        );
    }
    validate_with_llvm_as(&module.text);
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
/// `print` intrinsic 必须在动态 LLVM 文本中声明并调用稳定 Runtime ABI。
fn lowers_print_intrinsic_to_runtime_abi() {
    let print = IrExpression {
        kind: IrExpressionKind::IntrinsicCall {
            id: 19,
            arguments: vec![xiao_ir::IrCallArgument {
                kind: "positional".to_owned(),
                name: None,
                value: literal("str", "\"hello\"", "str"),
                span: span(),
            }],
        },
        ty: IrType::None,
        span: span(),
    };
    let module = lower_program(&expression_program(print), &CodegenOptions::default())
        .expect("print 应降低到动态 Runtime ABI");
    assert!(
        module
            .text
            .contains("declare i32 @xiao_runtime_print_values(ptr, i64)")
    );
    assert!(
        module
            .text
            .contains("call i32 @xiao_runtime_print_values(ptr")
    );
    validate_with_llvm_as(&module.text);
}

#[test]
/// `input` intrinsic 使用返回值 ABI 和挂起错误槽，不退化为未实现的名称调用。
fn lowers_input_intrinsic_to_runtime_abi() {
    let input = IrExpression {
        kind: IrExpressionKind::IntrinsicCall {
            id: 20,
            arguments: vec![xiao_ir::IrCallArgument {
                kind: "positional".to_owned(),
                name: None,
                value: literal("str", "\"prompt: \"", "str"),
                span: span(),
            }],
        },
        ty: IrType::Scalar {
            name: "str".to_owned(),
        },
        span: span(),
    };
    let module = lower_program(&expression_program(input), &CodegenOptions::default())
        .expect("input 应降低到动态 Runtime ABI");
    assert!(module.text.contains("xiao_runtime_input"));
    assert!(module.text.contains("xiao_runtime_error_class"));
    validate_with_llvm_as(&module.text);
}

/// 构造带一个 `code` 字符串参数的错误构造 intrinsic 调用。
fn error_constructor(id: u32) -> IrExpression {
    IrExpression {
        kind: IrExpressionKind::IntrinsicCall {
            id,
            arguments: vec![xiao_ir::IrCallArgument {
                kind: "positional".to_owned(),
                name: Some(name("code")),
                value: literal("str", "\"CODE\"", "str"),
                span: span(),
            }],
        },
        ty: IrType::Dynamic,
        span: span(),
    }
}

#[test]
/// 错误构造器经契约表降成 `IntrinsicCall` 后，仍必须走 `error_new_values` ABI，
/// 不能落入“动态 intrinsic 未支持”；此前这条路径只被环境门控的用例覆盖。
fn lowers_error_constructor_intrinsics_to_runtime_abi() {
    for (id, type_name) in [(10, "Error"), (12, "ArithmeticError"), (17, "TypeError")] {
        let module = lower_program(
            &expression_program(error_constructor(id)),
            &CodegenOptions::default(),
        )
        .unwrap_or_else(|error| panic!("{type_name} 应降低到动态 Runtime ABI：{error:?}"));
        assert!(
            module.text.contains("xiao_runtime_error_new_values"),
            "{type_name} 缺少错误构造调用"
        );
        assert!(
            module.text.contains(type_name),
            "{type_name} 的类型名应进入发射文本"
        );
        validate_with_llvm_as(&module.text);
    }
}

#[test]
/// `FatalError` 不得经由值 ABI 混入普通错误通道，契约表迁移前后都应如此。
fn fatal_error_intrinsic_is_still_rejected() {
    let error = lower_program(
        &expression_program(error_constructor(18)),
        &CodegenOptions::default(),
    )
    .expect_err("FatalError 不应构造为可恢复错误值");
    assert!(
        format!("{error:?}").contains("FatalError"),
        "拒绝原因应指明 FatalError：{error:?}"
    );
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
/// 动态跨类型 Cast 必须经 Runtime 唯一转换入口，不能把值按同布局透传。
fn lowers_non_identity_dynamic_cast() {
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
    let module = lower_program(&expression_program(cast), &CodegenOptions::default())
        .expect("跨类型 Cast 应降到 Runtime");
    assert!(module.text.contains("@xiao_runtime_value_cast"));
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
/// 前端登记的 RuntimeCheck 必须降到 Runtime ABI，不能再由动态入口整体拒绝。
fn lowers_runtime_check_to_abi() {
    let mut program = expression_program(literal("str", "\"text\"", "str"));
    program.runtime_checks.push(IrRuntimeCheck {
        kind: "string_boolean".to_owned(),
        span: span(),
        expected: None,
    });
    let module =
        lower_program(&program, &CodegenOptions::default()).expect("RuntimeCheck 应降到 ABI");
    assert!(module.text.contains("@xiao_runtime_dynamic_check"));
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

#[test]
/// 普通动态产物不得生成调试会话生命周期调用。
fn ordinary_dynamic_artifact_does_not_activate_diagnostics() {
    let module = lower_program(
        &expression_program(literal("str", "\"ordinary\"", "str")),
        &CodegenOptions::default(),
    )
    .expect("应生成普通动态 IR");
    assert!(!module.text.contains("xiao_native_debug_start"));
    assert!(!module.text.contains("xiao_runtime_diagnostic_prepare"));
    assert!(!module.text.contains("xiao_runtime_diagnostic_ready"));
    assert!(!module.text.contains("xiao_runtime_diagnostic_finish"));
}

#[test]
/// 调试动态产物必须包含完整的诊断会话生命周期调用。
fn debug_dynamic_artifact_activates_diagnostic_session() {
    let options = CodegenOptions::default().with_debug_startup("xiao-diagnostics");
    let module = lower_program(
        &expression_program(literal("str", "\"debug\"", "str")),
        &options,
    )
    .expect("应生成调试动态 IR");
    assert!(module.text.contains("xiao_runtime_diagnostic_prepare"));
    assert!(module.text.contains("xiao_runtime_diagnostic_ready"));
    assert!(module.text.contains("xiao_runtime_diagnostic_finish"));
    assert!(module.text.contains("xiao_native_debug_start"));
    assert!(
        module
            .text
            .contains("xiao.debug.fail:\n  call void @xiao_runtime_diagnostic_finish()")
    );
    assert!(
        module
            .text
            .contains("xiao.debug.ready.fail:\n  call void @xiao_runtime_diagnostic_finish()")
    );
}
