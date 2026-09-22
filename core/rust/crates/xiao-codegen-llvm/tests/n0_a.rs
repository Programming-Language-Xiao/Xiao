//! N0-A 静态标量降低规格。

use xiao_codegen_llvm::{
    CodegenError, CodegenOptions, EntryObservation, NativeBuild, TargetDescription, Toolchain,
    lower_program,
};
use xiao_ir::{
    IrElifBranch, IrEntryMode, IrExpression, IrExpressionKind, IrName, IrParameter, IrProgram,
    IrSpan, IrStatement, IrStatementKind, IrType,
};

/// 返回测试用的最小有效源码区间。
fn span() -> IrSpan {
    IrSpan::new(0, 1)
}

/// 构造未加反引号的测试名称。
fn name(text: &str) -> IrName {
    IrName {
        text: text.to_owned(),
        backticked: false,
        span: span(),
    }
}

/// 构造一个固定宽度标量类型。
fn scalar(text: &str) -> IrType {
    IrType::Scalar {
        name: text.to_owned(),
    }
}

/// 读取环境依赖原生测试使用的固定目标三元组。
fn configured_native_target() -> TargetDescription {
    let triple = std::env::var("XIAO_TARGET_TRIPLE")
        .expect("显式运行 --ignored 时 XIAO_TARGET_TRIPLE 必须已设置；准备方式见 10D §4");
    TargetDescription::new(
        triple,
        64,
        xiao_codegen_llvm::Endian::Little,
        xiao_codegen_llvm::ObjectFormat::Coff,
    )
    .expect("XIAO_TARGET_TRIPLE 必须是有效的 64 位 COFF 目标")
}

/// 构造带类型和文本的测试字面量。
fn literal(ty: &str, text: &str) -> IrExpression {
    IrExpression {
        kind: IrExpressionKind::Literal {
            literal: ty.to_owned(),
            text: text.to_owned(),
        },
        ty: scalar(ty),
        span: span(),
    }
}

/// 用脚本入口包装测试语句。
fn simple_program(body: Vec<IrStatement>) -> IrProgram {
    IrProgram::new(IrEntryMode::Script, body, span())
}

/// 构造一个简单名称赋值语句。
fn assignment(target: &str, value: IrExpression) -> IrStatement {
    IrStatement {
        kind: IrStatementKind::Assignment {
            target: name(target),
            value,
        },
        span: span(),
        leading_docs: Vec::new(),
    }
}

/// 构造用于原生编译 smoke test 的加法程序。
fn compile_scalar_program() -> IrProgram {
    let add = IrExpression {
        kind: IrExpressionKind::Binary {
            operator: "+".to_owned(),
            left: Box::new(literal("int", "1")),
            right: Box::new(literal("int", "2")),
        },
        ty: scalar("int"),
        span: span(),
    };
    simple_program(vec![assignment("value", add)])
}

#[test]
/// 同一前端 IR 应生成固定宽度标量、函数和入口。
fn lowers_scalar_calls_and_control_flow() {
    let total_name = || IrExpression {
        kind: IrExpressionKind::Name {
            name: name("total"),
        },
        ty: scalar("int"),
        span: span(),
    };
    let limit_name = || IrExpression {
        kind: IrExpressionKind::Name {
            name: name("limit"),
        },
        ty: scalar("int"),
        span: span(),
    };
    let condition = IrExpression {
        kind: IrExpressionKind::Binary {
            operator: "!=".to_owned(),
            left: Box::new(total_name()),
            right: Box::new(limit_name()),
        },
        ty: scalar("bool"),
        span: span(),
    };
    let increment = IrExpression {
        kind: IrExpressionKind::Binary {
            operator: "+".to_owned(),
            left: Box::new(total_name()),
            right: Box::new(literal("int", "1")),
        },
        ty: scalar("int"),
        span: span(),
    };
    let function = IrStatement {
        kind: IrStatementKind::Function {
            name: name("count"),
            parameters: vec![IrParameter {
                name: name("limit"),
                kind: "positional".to_owned(),
                ty: scalar("int"),
                default: None,
                span: span(),
            }],
            return_type: scalar("int"),
            body: vec![
                IrStatement {
                    kind: IrStatementKind::Declaration {
                        target: name("total"),
                        declared_type: scalar("int"),
                        constraint_path: None,
                        value: Some(literal("int", "0")),
                    },
                    span: span(),
                    leading_docs: Vec::new(),
                },
                IrStatement {
                    kind: IrStatementKind::While {
                        condition,
                        body: vec![assignment("total", increment)],
                    },
                    span: span(),
                    leading_docs: Vec::new(),
                },
                IrStatement {
                    kind: IrStatementKind::Return {
                        value: Some(total_name()),
                    },
                    span: span(),
                    leading_docs: Vec::new(),
                },
            ],
        },
        span: span(),
        leading_docs: Vec::new(),
    };
    let call = IrExpression {
        kind: IrExpressionKind::Call {
            callee: Box::new(IrExpression {
                kind: IrExpressionKind::Name {
                    name: name("count"),
                },
                ty: IrType::Function {
                    parameters: vec![scalar("int")],
                    return_type: Box::new(scalar("int")),
                },
                span: span(),
            }),
            arguments: vec![xiao_ir::IrCallArgument {
                kind: "positional".to_owned(),
                name: None,
                value: literal("int", "3"),
                span: span(),
            }],
        },
        ty: scalar("int"),
        span: span(),
    };
    let program = simple_program(vec![function, assignment("result", call)]);
    let module = lower_program(
        &program,
        &CodegenOptions::for_target(TargetDescription::linux_x86_64()),
    )
    .expect("降低");
    assert!(module.text.contains("define i64 @xiao_fn_0_count"));
    assert!(module.text.contains("define void @xiao_entry"));
    assert!(module.text.contains("define i32 @main"));
    assert!(module.text.contains("llvm.sadd.with.overflow.i64"));
    assert!(!module.uses_runtime);
    if let Some(clang) = std::env::var_os("XIAO_CLANG") {
        let toolchain = Toolchain::new(clang);
        NativeBuild::new()
            .validate_llvm(&module.text, &TargetDescription::linux_x86_64(), &toolchain)
            .expect("LLVM 应接受控制流模块");
    }
}

#[test]
/// 溢出路径必须显式进入 trap，而不能生成裸回绕算术。
fn emits_checked_integer_arithmetic() {
    let program = compile_scalar_program();
    let module = lower_program(&program, &CodegenOptions::default()).expect("降低");
    assert!(module.text.contains("llvm.sadd.with.overflow.i64"));
    assert!(module.text.contains("call void @llvm.trap()"));
}

#[test]
/// 32/64 位浮点和 bool 标量使用独立 LLVM 类型，并检查非有限结果。
fn lowers_float_and_bool_scalars() {
    let float_add = IrExpression {
        kind: IrExpressionKind::Binary {
            operator: "+".to_owned(),
            left: Box::new(literal("float", "1.5")),
            right: Box::new(literal("float", "2.5")),
        },
        ty: scalar("float"),
        span: span(),
    };
    let program = simple_program(vec![
        assignment("value", float_add),
        assignment("flag", literal("bool", "true")),
    ]);
    let module = lower_program(&program, &CodegenOptions::default()).expect("降低");
    assert!(module.text.contains("fadd double"));
    assert!(module.text.contains("fcmp ord double"));
    assert!(module.text.contains("store i1 true"));
    if let Some(clang) = std::env::var_os("XIAO_CLANG") {
        NativeBuild::new()
            .validate_llvm(
                &module.text,
                &TargetDescription::host(),
                &Toolchain::new(clang),
            )
            .expect("LLVM 应接受浮点模块");
    }
}

#[test]
/// 布尔相等比较属于共享标量语义，直接映射为 `icmp` 而不引入 Runtime。
fn lowers_boolean_equality() {
    let equality = IrExpression {
        kind: IrExpressionKind::Binary {
            operator: "==".to_owned(),
            left: Box::new(literal("bool", "true")),
            right: Box::new(literal("bool", "false")),
        },
        ty: scalar("bool"),
        span: span(),
    };
    let module = lower_program(
        &simple_program(vec![assignment("value", equality)]),
        &CodegenOptions::default(),
    )
    .expect("布尔相等比较应降低");
    assert!(module.text.contains("icmp eq i1 true, false"));
    assert!(!module.uses_runtime);
    if let Some(clang) = std::env::var_os("XIAO_CLANG") {
        NativeBuild::new()
            .validate_llvm(
                &module.text,
                &TargetDescription::host(),
                &Toolchain::new(clang),
            )
            .expect("LLVM 应接受布尔相等比较");
    }
}

#[test]
/// 显式窄宽度声明决定局部槽类型，并对初始化值执行受检窄化。
fn respects_declared_scalar_width() {
    let declaration = IrStatement {
        kind: IrStatementKind::Declaration {
            target: name("small"),
            declared_type: scalar("sint"),
            constraint_path: None,
            value: Some(literal("int", "7")),
        },
        span: span(),
        leading_docs: Vec::new(),
    };
    let module = lower_program(
        &simple_program(vec![declaration, assignment("small", literal("int", "8"))]),
        &CodegenOptions::default(),
    )
    .expect("显式 sint 声明应降低");
    assert!(module.text.contains("alloca i32"));
    assert!(module.text.contains("trunc i64 7 to i32"));
    if let Some(clang) = std::env::var_os("XIAO_CLANG") {
        NativeBuild::new()
            .validate_llvm(
                &module.text,
                &TargetDescription::host(),
                &Toolchain::new(clang),
            )
            .expect("LLVM 应接受显式窄化");
    }
}

#[test]
/// 浮点幂必须使用 LLVM intrinsic 的二参数声明，并能通过真实汇编器验证。
fn lowers_float_power_with_valid_intrinsic_signature() {
    let power = IrExpression {
        kind: IrExpressionKind::Binary {
            operator: "**".to_owned(),
            left: Box::new(literal("float", "2.0")),
            right: Box::new(literal("float", "3.0")),
        },
        ty: scalar("float"),
        span: span(),
    };
    let program = simple_program(vec![assignment("value", power)]);
    let module = lower_program(&program, &CodegenOptions::default()).expect("降低");
    assert!(
        module
            .text
            .contains("declare double @llvm.pow.f64(double, double)")
    );
    assert!(!module.text.contains("double double"));
    if let Some(llvm_as) = std::env::var_os("XIAO_LLVM_AS") {
        NativeBuild::new()
            .validate_llvm(
                &module.text,
                &TargetDescription::host(),
                &Toolchain::new("unused").with_llvm_as(llvm_as),
            )
            .expect("llvm-as 应接受浮点幂模块");
    }
}

#[test]
/// N0-B 把字符串值切到稳定 ABI，同时保留可解释的 Runtime 组件清单。
fn lowers_runtime_values_with_abi() {
    let string_program = simple_program(vec![assignment(
        "value",
        IrExpression {
            kind: IrExpressionKind::Literal {
                literal: "str".to_owned(),
                text: "\"heap\"".to_owned(),
            },
            ty: scalar("str"),
            span: span(),
        },
    )]);
    let module = lower_program(&string_program, &CodegenOptions::default()).expect("动态值应降低");
    assert!(module.uses_runtime);
    assert!(module.text.contains("@xiao_runtime_string_new"));
    assert!(module.text.contains("@xiao_runtime_value_str"));
    assert!(
        module
            .runtime_components
            .iter()
            .any(|component| component == "value")
    );
}

#[test]
/// 关键字和展开实参不能绕过 N0-A 的静态调用签名。
fn rejects_expanded_arguments() {
    let function = IrStatement {
        kind: IrStatementKind::Function {
            name: name("identity"),
            parameters: vec![IrParameter {
                name: name("value"),
                kind: "positional".to_owned(),
                ty: scalar("int"),
                default: None,
                span: span(),
            }],
            return_type: scalar("int"),
            body: vec![IrStatement {
                kind: IrStatementKind::Return {
                    value: Some(literal("int", "1")),
                },
                span: span(),
                leading_docs: Vec::new(),
            }],
        },
        span: span(),
        leading_docs: Vec::new(),
    };
    let call = IrExpression {
        kind: IrExpressionKind::Call {
            callee: Box::new(IrExpression {
                kind: IrExpressionKind::Name {
                    name: name("identity"),
                },
                ty: IrType::Function {
                    parameters: vec![scalar("int")],
                    return_type: Box::new(scalar("int")),
                },
                span: span(),
            }),
            arguments: vec![xiao_ir::IrCallArgument {
                kind: "keyword".to_owned(),
                name: Some(name("value")),
                value: literal("int", "1"),
                span: span(),
            }],
        },
        ty: scalar("int"),
        span: span(),
    };
    let program = simple_program(vec![function, assignment("result", call)]);
    let error = lower_program(&program, &CodegenOptions::default()).expect_err("应拒绝");
    assert!(matches!(error, CodegenError::Unsupported { .. }));
}

#[test]
/// 逻辑和身份运算必须与当前生产字节码接受边界一致，不能被 LLVM eager 指令偷偷放行。
fn rejects_unshared_logical_and_identity_operators() {
    for operator in ["and", "or", "is", "is not"] {
        let expression = IrExpression {
            kind: IrExpressionKind::Binary {
                operator: operator.to_owned(),
                left: Box::new(literal("bool", "true")),
                right: Box::new(literal("bool", "false")),
            },
            ty: scalar("bool"),
            span: span(),
        };
        let error = lower_program(
            &simple_program(vec![assignment("value", expression)]),
            &CodegenOptions::default(),
        )
        .expect_err("未共享的运算符必须拒绝");
        assert!(
            matches!(error, CodegenError::Unsupported { .. }),
            "{operator}"
        );
    }
}

#[test]
/// 整数除法按类型阶段规则提升到对应浮点宽度，而不是被误报为整数 `/`。
fn lowers_integer_division_to_float() {
    let expression = IrExpression {
        kind: IrExpressionKind::Binary {
            operator: "/".to_owned(),
            left: Box::new(literal("int", "5")),
            right: Box::new(literal("int", "2")),
        },
        ty: scalar("float"),
        span: span(),
    };
    let module = lower_program(
        &simple_program(vec![assignment("value", expression)]),
        &CodegenOptions::default(),
    )
    .expect("整数除法应降低");
    assert!(module.text.contains("fdiv double"));
    if let Some(clang) = std::env::var_os("XIAO_CLANG") {
        NativeBuild::new()
            .validate_llvm(
                &module.text,
                &TargetDescription::host(),
                &Toolchain::new(clang),
            )
            .expect("LLVM 应接受整数提升后的除法");
    }
}

#[test]
/// `elif` 的最终假路径必须有终结跳转，避免生成 llvm-as 无法接受的空基本块。
fn emits_terminated_elif_false_path() {
    let condition = IrExpression {
        kind: IrExpressionKind::Name { name: name("flag") },
        ty: scalar("bool"),
        span: span(),
    };
    let function = IrStatement {
        kind: IrStatementKind::Function {
            name: name("choose"),
            parameters: vec![IrParameter {
                name: name("flag"),
                kind: "positional".to_owned(),
                ty: scalar("bool"),
                default: None,
                span: span(),
            }],
            return_type: scalar("int"),
            body: vec![IrStatement {
                kind: IrStatementKind::If {
                    condition,
                    body: vec![IrStatement {
                        kind: IrStatementKind::Return {
                            value: Some(literal("int", "1")),
                        },
                        span: span(),
                        leading_docs: Vec::new(),
                    }],
                    elif_branches: vec![IrElifBranch {
                        condition: literal("bool", "false"),
                        body: vec![IrStatement {
                            kind: IrStatementKind::Return {
                                value: Some(literal("int", "2")),
                            },
                            span: span(),
                            leading_docs: Vec::new(),
                        }],
                        span: span(),
                    }],
                    else_body: None,
                },
                span: span(),
                leading_docs: Vec::new(),
            }],
        },
        span: span(),
        leading_docs: Vec::new(),
    };
    let module = lower_program(&simple_program(vec![function]), &CodegenOptions::default())
        .expect("elif 应降低");
    if let Some(clang) = std::env::var_os("XIAO_CLANG") {
        NativeBuild::new()
            .validate_llvm(
                &module.text,
                &TargetDescription::host(),
                &Toolchain::new(clang),
            )
            .expect("LLVM 应接受所有 elif 基本块");
    }
}

#[test]
/// 外部工具路径由调用方注入；缺失工具会返回稳定的结构化错误。
fn missing_toolchain_is_structured() {
    let program = compile_scalar_program();
    let output = std::env::temp_dir().join("xiao-n0-a-missing.exe");
    let request = xiao_codegen_llvm::BuildRequest::new(
        program,
        TargetDescription::new(
            "x86_64-w64-windows-gnu",
            64,
            xiao_codegen_llvm::Endian::Little,
            xiao_codegen_llvm::ObjectFormat::Coff,
        )
        .expect("目标"),
        Toolchain::new("xiao-clang-that-does-not-exist"),
        output,
    );
    let error = NativeBuild::new()
        .build(&request)
        .expect_err("应报告工具缺失");
    assert!(matches!(error, CodegenError::ToolchainUnavailable { .. }));
}

#[test]
/// 入口观察策略属于测试驱动器选项，不改变默认零 Runtime 模块形态。
fn entry_observation_is_explicit() {
    let program = simple_program(vec![assignment("value", literal("bool", "true"))]);
    let options = CodegenOptions::default().with_entry_observation(EntryObservation::ExitCode);
    let module = lower_program(&program, &options).expect("降低");
    assert!(module.text.contains("define i32 @main"));
    assert!(!module.uses_runtime);
}

#[test]
#[ignore = "需要 XIAO_CLANG 与 XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
/// 入口观察值必须来自实际执行的分支，而不是降低顺序中的最后一个赋值。
fn optional_entry_observation_tracks_runtime_branch() {
    let clang = std::env::var_os("XIAO_CLANG")
        .expect("显式运行 --ignored 时 XIAO_CLANG 必须已设置；准备方式见 10D §4");
    let branch = IrStatement {
        kind: IrStatementKind::If {
            condition: literal("bool", "true"),
            body: vec![assignment("value", literal("int", "7"))],
            elif_branches: Vec::new(),
            else_body: Some(vec![assignment("value", literal("int", "2"))]),
        },
        span: span(),
        leading_docs: Vec::new(),
    };
    let target = configured_native_target();
    let options = CodegenOptions::for_target(target.clone())
        .with_entry_observation(EntryObservation::ExitCode);
    let request = xiao_codegen_llvm::BuildRequest::new(
        simple_program(vec![branch]),
        target,
        Toolchain::new(clang),
        std::env::temp_dir().join(format!("xiao-n0-a-observe-{}.exe", std::process::id())),
    )
    .with_options(options);
    let artifact = NativeBuild::new().build(&request).expect("构建");
    let run = xiao_codegen_llvm::NativeRun::new(&artifact)
        .run()
        .expect("启动");
    assert_eq!(run.status, Some(7), "stderr: {}", run.stderr);
    let _ = std::fs::remove_file(artifact.executable);
}

#[test]
#[ignore = "需要 XIAO_CLANG（可选 XIAO_LLVM_AS）与 XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
/// 在调用方显式提供 LLVM 工具链时，生成文本必须能验证、链接并启动。
fn optional_real_llvm_round_trip() {
    let clang = std::env::var_os("XIAO_CLANG")
        .expect("显式运行 --ignored 时 XIAO_CLANG 必须已设置；准备方式见 10D §4");
    let llvm_as = std::env::var_os("XIAO_LLVM_AS");
    let root = std::env::temp_dir().join(format!("xiao-n0-a-{}", std::process::id()));
    let output = root.join(if cfg!(windows) {
        "scalar.exe"
    } else {
        "scalar"
    });
    let ir_path = root.join("scalar.ll");
    let mut toolchain = Toolchain::new(clang);
    if let Some(llvm_as) = llvm_as {
        toolchain = toolchain.with_llvm_as(llvm_as);
    }
    let request = xiao_codegen_llvm::BuildRequest::new(
        compile_scalar_program(),
        configured_native_target(),
        toolchain,
        &output,
    )
    .with_llvm_ir_output(&ir_path);
    let artifact = NativeBuild::new().build(&request).expect("LLVM 构建");
    assert!(artifact.executable.exists());
    let run = xiao_codegen_llvm::NativeRun::new(&artifact)
        .run()
        .expect("启动");
    assert_eq!(run.status, Some(0), "stderr: {}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[ignore = "需要 XIAO_CLANG；准备方式见 10D §4"]
/// LLVM 语法损坏时，验证器必须拒绝文本而不是继续链接。
fn optional_corrupt_llvm_is_rejected() {
    let clang = std::env::var_os("XIAO_CLANG")
        .expect("显式运行 --ignored 时 XIAO_CLANG 必须已设置；准备方式见 10D §4");
    let toolchain = Toolchain::new(clang);
    let error = NativeBuild::new()
        .validate_llvm(
            "define i32 @broken( {",
            &TargetDescription::host(),
            &toolchain,
        )
        .expect_err("损坏 IR 应拒绝");
    assert!(matches!(error, CodegenError::ToolchainFailed { .. }));
}
