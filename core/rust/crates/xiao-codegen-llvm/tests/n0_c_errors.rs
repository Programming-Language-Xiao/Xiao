//! N0-C 原生错误路径与源码映射回归测试。

use xiao_codegen_llvm::{CodegenOptions, NativeBuild, TargetDescription, Toolchain, lower_program};
use xiao_ir::{
    IrCallArgument, IrCatchClause, IrEntryMode, IrExpression, IrExpressionKind, IrName, IrProgram,
    IrSpan, IrStatement, IrStatementKind, IrType,
};

fn span() -> IrSpan {
    IrSpan::new(10, 22)
}

fn name(text: &str) -> IrName {
    IrName {
        text: text.to_owned(),
        backticked: false,
        span: span(),
    }
}

fn string_literal(text: &str) -> IrExpression {
    IrExpression {
        kind: IrExpressionKind::Literal {
            literal: "str".to_owned(),
            text: format!("\"{text}\""),
        },
        ty: IrType::Scalar {
            name: "str".to_owned(),
        },
        span: span(),
    }
}

fn error_constructor() -> IrExpression {
    IrExpression {
        kind: IrExpressionKind::NewCall {
            callee: Box::new(IrExpression {
                kind: IrExpressionKind::Name {
                    name: name("ArithmeticError"),
                },
                ty: IrType::Dynamic,
                span: span(),
            }),
            arguments: vec![IrCallArgument {
                kind: "keyword".to_owned(),
                name: Some(name("code")),
                value: string_literal("N0-C"),
                span: span(),
            }],
        },
        ty: IrType::Dynamic,
        span: span(),
    }
}

fn error_program() -> IrProgram {
    IrProgram::new(
        IrEntryMode::Script,
        vec![IrStatement {
            kind: IrStatementKind::Try {
                body: vec![IrStatement {
                    kind: IrStatementKind::Raise {
                        value: error_constructor(),
                    },
                    span: span(),
                    leading_docs: Vec::new(),
                }],
                catches: vec![IrCatchClause {
                    binding: name("error"),
                    error_type: name("ArithmeticError"),
                    body: vec![IrStatement {
                        kind: IrStatementKind::Expression {
                            value: IrExpression {
                                kind: IrExpressionKind::Name {
                                    name: name("error"),
                                },
                                ty: IrType::Dynamic,
                                span: span(),
                            },
                        },
                        span: span(),
                        leading_docs: Vec::new(),
                    }],
                    span: span(),
                }],
                finally_body: Some(vec![IrStatement {
                    kind: IrStatementKind::Expression {
                        value: string_literal("cleanup"),
                    },
                    span: span(),
                    leading_docs: Vec::new(),
                }]),
            },
            span: span(),
            leading_docs: Vec::new(),
        }],
        span(),
    )
}

fn dynamic_bool_literal(value: bool) -> IrExpression {
    IrExpression {
        kind: IrExpressionKind::Literal {
            literal: "bool".to_owned(),
            text: value.to_string(),
        },
        ty: IrType::Scalar {
            name: "bool".to_owned(),
        },
        span: span(),
    }
}

fn dynamic_control_program(statement: IrStatementKind) -> IrProgram {
    IrProgram::new(
        IrEntryMode::Script,
        vec![IrStatement {
            kind: IrStatementKind::While {
                condition: dynamic_bool_literal(true),
                body: vec![IrStatement {
                    kind: IrStatementKind::Try {
                        body: vec![IrStatement {
                            kind: statement,
                            span: span(),
                            leading_docs: Vec::new(),
                        }],
                        catches: Vec::new(),
                        finally_body: Some(vec![IrStatement {
                            kind: IrStatementKind::Expression {
                                value: string_literal("cleanup"),
                            },
                            span: span(),
                            leading_docs: Vec::new(),
                        }]),
                    },
                    span: span(),
                    leading_docs: Vec::new(),
                }],
            },
            span: span(),
            leading_docs: Vec::new(),
        }],
        span(),
    )
}

#[test]
/// `raise`、错误构造和处理器必须全部落到 Runtime ABI，而不是被拒绝或 trap。
fn lowers_recoverable_error_path() {
    let module =
        lower_program(&error_program(), &CodegenOptions::default()).expect("错误路径应降低");
    for marker in [
        "@xiao_runtime_error_new",
        "@xiao_runtime_error_raise_value",
        "@xiao_runtime_error_matches",
        "@xiao_runtime_error_take",
        "dynamic.try.dispatch",
        "dynamic.try.catch0.body",
        "dynamic.try.normal.finally",
    ] {
        assert!(
            module.text.contains(marker),
            "缺少原生错误路径标记 {marker}"
        );
    }
    assert!(module.text.contains("xiao.source-map try 10..22"));
    assert!(module.text.contains("xiao.source-map raise 10..22"));
}

#[test]
/// 在提供 `llvm-as` 时，错误派发的所有基本块必须通过真正的 LLVM 解析器。
fn optional_llvm_accepts_error_path_module() {
    let Some(llvm_as) = std::env::var_os("XIAO_LLVM_AS") else {
        return;
    };
    let module =
        lower_program(&error_program(), &CodegenOptions::default()).expect("错误路径应降低");
    NativeBuild::new()
        .validate_llvm(
            &module.text,
            &TargetDescription::host(),
            &Toolchain::new("unused").with_llvm_as(llvm_as),
        )
        .expect("llvm-as 应接受原生错误路径");
}

#[test]
/// Fatal 错误类型不能被原生后端伪装成普通可捕获错误。
fn rejects_fatal_catch_type() {
    let mut program = error_program();
    if let IrStatementKind::Try { catches, .. } = &mut program.body[0].kind {
        catches[0].error_type = name("FatalError");
    }
    let error =
        lower_program(&program, &CodegenOptions::default()).expect_err("FatalError catch 必须拒绝");
    assert!(error.to_string().contains("原生 catch 类型 FatalError"));
}

#[test]
fn lowers_nonlocal_control_exits_through_cleanup_chain() {
    let cases = [
        (IrStatementKind::Return { value: None }, "return"),
        (IrStatementKind::Break, "break"),
        (IrStatementKind::Continue, "continue"),
    ];
    for (statement, exit) in cases {
        let module = lower_program(
            &dynamic_control_program(statement),
            &CodegenOptions::default(),
        )
        .unwrap_or_else(|error| panic!("{exit} 应降低: {error}"));
        assert!(module.text.contains("dynamic.try.normal.finally"));
        assert!(module.text.contains("xiao_runtime_value_release"));
        let exit_marker = match exit {
            "return" => "dynamic.return",
            "break" => "dynamic.break",
            "continue" => "dynamic.continue",
            _ => unreachable!(),
        };
        assert!(module.text.contains(exit_marker));
    }
}
