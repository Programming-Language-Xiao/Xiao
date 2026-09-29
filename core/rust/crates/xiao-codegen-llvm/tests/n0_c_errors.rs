//! N0-C 原生错误路径与源码映射回归测试。

use xiao_codegen_llvm::{CodegenOptions, NativeBuild, TargetDescription, Toolchain, lower_program};
use xiao_ir::{
    IrCallArgument, IrCatchClause, IrEntryMode, IrExpression, IrExpressionKind, IrName,
    IrOwnership, IrProgram, IrReleaseAction, IrReleasePlan, IrScope, IrSpan, IrStatement,
    IrStatementKind, IrType, IrValue,
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

fn expression_statement(value: IrExpression) -> IrStatement {
    IrStatement {
        kind: IrStatementKind::Expression { value },
        span: span(),
        leading_docs: Vec::new(),
    }
}

fn catch_clause(error_type: &str, body: Vec<IrStatement>) -> IrCatchClause {
    IrCatchClause {
        binding: name("error"),
        error_type: name(error_type),
        body,
        span: span(),
    }
}

fn try_statement(
    body: Vec<IrStatement>,
    catches: Vec<IrCatchClause>,
    finally_body: Option<Vec<IrStatement>>,
) -> IrStatement {
    IrStatement {
        kind: IrStatementKind::Try {
            body,
            catches,
            finally_body,
        },
        span: span(),
        leading_docs: Vec::new(),
    }
}

fn program_from_statements(body: Vec<IrStatement>) -> IrProgram {
    IrProgram::new(IrEntryMode::Script, body, span())
}

fn no_finally_catch_program() -> IrProgram {
    program_from_statements(vec![try_statement(
        vec![IrStatement {
            kind: IrStatementKind::Raise {
                value: error_constructor(),
            },
            span: span(),
            leading_docs: Vec::new(),
        }],
        vec![catch_clause(
            "ArithmeticError",
            vec![expression_statement(string_literal("handled"))],
        )],
        None,
    )])
}

fn multiple_catch_program() -> IrProgram {
    program_from_statements(vec![try_statement(
        vec![IrStatement {
            kind: IrStatementKind::Raise {
                value: error_constructor(),
            },
            span: span(),
            leading_docs: Vec::new(),
        }],
        vec![
            catch_clause(
                "TypeError",
                vec![expression_statement(string_literal("type"))],
            ),
            catch_clause(
                "ArithmeticError",
                vec![expression_statement(string_literal("arithmetic"))],
            ),
        ],
        Some(vec![expression_statement(string_literal("cleanup"))]),
    )])
}

fn nested_try_program() -> IrProgram {
    let inner = try_statement(
        vec![IrStatement {
            kind: IrStatementKind::Raise {
                value: error_constructor(),
            },
            span: span(),
            leading_docs: Vec::new(),
        }],
        vec![catch_clause(
            "ArithmeticError",
            vec![expression_statement(string_literal("inner"))],
        )],
        Some(vec![expression_statement(string_literal("inner-cleanup"))]),
    );
    program_from_statements(vec![try_statement(
        vec![inner],
        vec![catch_clause(
            "ArithmeticError",
            vec![expression_statement(string_literal("outer"))],
        )],
        Some(vec![expression_statement(string_literal("outer-cleanup"))]),
    )])
}

fn catch_raise_program() -> IrProgram {
    program_from_statements(vec![try_statement(
        vec![IrStatement {
            kind: IrStatementKind::Raise {
                value: error_constructor(),
            },
            span: span(),
            leading_docs: Vec::new(),
        }],
        vec![catch_clause(
            "ArithmeticError",
            vec![IrStatement {
                kind: IrStatementKind::Raise {
                    value: error_constructor(),
                },
                span: span(),
                leading_docs: Vec::new(),
            }],
        )],
        Some(vec![expression_statement(string_literal("cleanup"))]),
    )])
}

fn finally_control_program(statement: IrStatementKind) -> IrProgram {
    let try_body = vec![expression_statement(string_literal("body"))];
    let finally_body = vec![IrStatement {
        kind: statement,
        span: span(),
        leading_docs: Vec::new(),
    }];
    program_from_statements(vec![IrStatement {
        kind: IrStatementKind::While {
            condition: dynamic_bool_literal(true),
            body: vec![try_statement(try_body, Vec::new(), Some(finally_body))],
        },
        span: span(),
        leading_docs: Vec::new(),
    }])
}

fn finally_owned_control_program(statement: IrStatementKind, exit: &str) -> IrProgram {
    let mut program = finally_control_program(statement);
    let IrStatementKind::While { body, .. } = &mut program.body[0].kind else {
        unreachable!();
    };
    let IrStatementKind::Try {
        finally_body: Some(finally_body),
        ..
    } = &mut body[0].kind
    else {
        unreachable!();
    };
    finally_body.insert(
        0,
        IrStatement {
            kind: IrStatementKind::Assignment {
                target: name("cleanup"),
                value: string_literal("owned cleanup value"),
            },
            span: span(),
            leading_docs: Vec::new(),
        },
    );
    program.ownership = IrOwnership {
        scopes: vec![
            IrScope {
                id: 0,
                parent: None,
                kind: "program".to_owned(),
                span: span(),
                depth: 0,
                values: Vec::new(),
            },
            IrScope {
                id: 1,
                parent: Some(0),
                kind: "try".to_owned(),
                span: span(),
                depth: 1,
                values: Vec::new(),
            },
            IrScope {
                id: 2,
                parent: Some(0),
                kind: "finally".to_owned(),
                span: span(),
                depth: 1,
                values: vec![1],
            },
        ],
        values: vec![IrValue {
            id: 1,
            name: Some("cleanup".to_owned()),
            scope: 2,
            span: span(),
            ty: Some(IrType::Scalar {
                name: "str".to_owned(),
            }),
            storage: "heap_strong".to_owned(),
            declaration_order: 0,
            parameter: false,
            constant: false,
            temporary: false,
            escapes: Vec::new(),
        }],
        release_plans: vec![
            IrReleasePlan {
                scope: 0,
                exit: "normal".to_owned(),
                actions: Vec::new(),
                transferred: Vec::new(),
            },
            IrReleasePlan {
                scope: 0,
                exit: "return".to_owned(),
                actions: Vec::new(),
                transferred: Vec::new(),
            },
            IrReleasePlan {
                scope: 2,
                exit: exit.to_owned(),
                actions: vec![IrReleaseAction {
                    value: 1,
                    order: 0,
                    kind: "strong".to_owned(),
                }],
                transferred: Vec::new(),
            },
        ],
        ..IrOwnership::default()
    };
    program
}

fn loop_try_finally_program() -> IrProgram {
    program_from_statements(vec![IrStatement {
        kind: IrStatementKind::While {
            condition: dynamic_bool_literal(true),
            body: vec![try_statement(
                vec![expression_statement(string_literal("body"))],
                Vec::new(),
                Some(vec![expression_statement(string_literal("cleanup"))]),
            )],
        },
        span: span(),
        leading_docs: Vec::new(),
    }])
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
#[ignore = "需要 XIAO_LLVM_AS；准备方式见 10D §4"]
/// 在提供 `llvm-as` 时，错误派发的所有基本块必须通过真正的 LLVM 解析器。
fn optional_llvm_accepts_error_path_module() {
    let llvm_as = std::env::var_os("XIAO_LLVM_AS")
        .expect("显式运行 --ignored 时 XIAO_LLVM_AS 必须已设置；准备方式见 10D §4");
    let programs = [
        ("try-catch-finally", error_program()),
        ("try-catch", no_finally_catch_program()),
        ("multiple-catch", multiple_catch_program()),
        ("nested-try", nested_try_program()),
        ("catch-raise", catch_raise_program()),
        (
            "finally-owned-return",
            finally_owned_control_program(IrStatementKind::Return { value: None }, "return"),
        ),
        (
            "finally-owned-break",
            finally_owned_control_program(IrStatementKind::Break, "break"),
        ),
        (
            "finally-owned-continue",
            finally_owned_control_program(IrStatementKind::Continue, "continue"),
        ),
        (
            "finally-return",
            finally_control_program(IrStatementKind::Return { value: None }),
        ),
        (
            "finally-break",
            finally_control_program(IrStatementKind::Break),
        ),
        (
            "finally-continue",
            finally_control_program(IrStatementKind::Continue),
        ),
        (
            "finally-raise",
            finally_control_program(IrStatementKind::Raise {
                value: error_constructor(),
            }),
        ),
        (
            "try-return-finally",
            dynamic_control_program(IrStatementKind::Return { value: None }),
        ),
        (
            "try-break-finally",
            dynamic_control_program(IrStatementKind::Break),
        ),
        (
            "try-continue-finally",
            dynamic_control_program(IrStatementKind::Continue),
        ),
        ("loop-try-finally", loop_try_finally_program()),
    ];
    for (name, program) in programs {
        let module = lower_program(&program, &CodegenOptions::default())
            .unwrap_or_else(|error| panic!("{name} 应降低: {error}"));
        NativeBuild::new()
            .validate_llvm(
                &module.text,
                &TargetDescription::host(),
                &Toolchain::new("unused").with_llvm_as(llvm_as.clone()),
            )
            .unwrap_or_else(|error| panic!("llvm-as 应接受 {name}：{error}"));
    }
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
            "return" => "ret void",
            "break" => "br label %dynamic.while.end",
            "continue" => "br label %dynamic.while.cond",
            _ => unreachable!(),
        };
        assert!(module.text.contains(exit_marker));
    }

    let cases = [
        (
            IrStatementKind::Return { value: None },
            "return",
            "ret void",
        ),
        (
            IrStatementKind::Break,
            "break",
            "br label %dynamic.while.end",
        ),
        (
            IrStatementKind::Continue,
            "continue",
            "br label %dynamic.while.cond",
        ),
    ];
    for (statement, exit, target) in cases {
        let program = finally_owned_control_program(statement, exit);
        let module = lower_program(&program, &CodegenOptions::default())
            .unwrap_or_else(|error| panic!("带 finally 作用域释放计划的 {exit} 应降低: {error}"));
        let finally_release = module
            .text
            .find("call void @xiao_runtime_value_release(ptr %slot0)")
            .unwrap_or_else(|| panic!("{exit} 前应释放 finally 作用域拥有值"));
        let exit_target = module
            .text
            .get(finally_release..)
            .and_then(|text| text.find(target).map(|offset| finally_release + offset))
            .unwrap_or_else(|| panic!("缺少 {exit} 的实际退出目标 {target}"));
        assert!(
            finally_release < exit_target,
            "finally 作用域必须先释放再执行 {exit}"
        );
    }

    let loop_module = lower_program(&loop_try_finally_program(), &CodegenOptions::default())
        .expect("循环中的 try/finally 应降低");
    let stack_save = loop_module
        .text
        .find("call ptr @llvm.stacksave()")
        .expect("finally 入口应保存动态栈位置");
    let stack_restore = loop_module
        .text
        .find("call void @llvm.stackrestore(ptr ")
        .expect("finally 出口应恢复动态栈位置");
    assert!(stack_save < stack_restore);
}
