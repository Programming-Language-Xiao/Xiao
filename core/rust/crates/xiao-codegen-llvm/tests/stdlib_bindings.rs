//! 同名官方函数的选择导入、模块内调用和真实函数体移除回归。

use xiao_codegen_llvm::{CodegenError, CodegenOptions, NativeBuild, TargetDescription};
use xiao_ir::{
    IrCallArgument, IrEntryMode, IrExpression, IrExpressionKind, IrImportItem, IrModule, IrName,
    IrParameter, IrProgram, IrSpan, IrStatement, IrStatementKind, IrSymbol, IrType,
};

/// 所有定义故意共享源码区间，必须通过模块和节点身份区分。
fn span() -> IrSpan {
    IrSpan::new(0, 1)
}

/// 独立原始 IR 不依赖七库草稿或前端加载顺序。
fn name(text: &str) -> IrName {
    IrName {
        text: text.to_owned(),
        backticked: false,
        span: span(),
    }
}

/// 固定测试数值。
fn integer(value: i64) -> IrExpression {
    IrExpression {
        kind: IrExpressionKind::Literal {
            literal: "int".to_owned(),
            text: value.to_string(),
        },
        ty: IrType::Dynamic,
        span: span(),
    }
}

/// 逆序命名实参同时核对模块形参身份与既有绑定排列。
fn call(function: &str, parameters: &[&str]) -> IrExpression {
    IrExpression {
        kind: IrExpressionKind::Call {
            callee: Box::new(IrExpression {
                kind: IrExpressionKind::Name {
                    name: name(function),
                },
                ty: IrType::Dynamic,
                span: span(),
            }),
            arguments: parameters
                .iter()
                .rev()
                .enumerate()
                .map(|(index, parameter)| IrCallArgument {
                    kind: "keyword".to_owned(),
                    name: Some(name(parameter)),
                    value: integer(index as i64 + 1),
                    span: span(),
                })
                .collect(),
        },
        ty: IrType::Dynamic,
        span: span(),
    }
}

/// 最小纯函数定义。
fn function(function: &str, parameters: &[&str], result: IrExpression) -> IrStatement {
    IrStatement {
        kind: IrStatementKind::Function {
            name: name(function),
            parameters: parameters
                .iter()
                .map(|parameter| IrParameter {
                    name: name(parameter),
                    kind: "positional_or_keyword".to_owned(),
                    ty: IrType::Dynamic,
                    default: None,
                    span: span(),
                })
                .collect(),
            return_type: IrType::Dynamic,
            body: vec![IrStatement {
                kind: IrStatementKind::Return {
                    value: Some(result),
                },
                span: span(),
                leading_docs: Vec::new(),
            }],
        },
        span: span(),
        leading_docs: Vec::new(),
    }
}

/// 两模块各有同名 pick/relay，pick 的形参集合不同。
fn module(module: &str, parameters: &[&str]) -> IrModule {
    IrModule {
        name: module.to_owned(),
        kind: "file".to_owned(),
        path: module.to_owned(),
        has_concurrency: false,
        body: vec![
            function("pick", parameters, integer(42)),
            function("relay", &[], call("pick", parameters)),
        ],
        symbols: ["pick", "relay"]
            .iter()
            .map(|symbol| IrSymbol {
                name: (*symbol).to_owned(),
                kind: "function".to_owned(),
                origin: "local".to_owned(),
                span: span(),
            })
            .collect(),
    }
}

/// 导入与实际调用，不用重写函数名伪装成无冲突情形。
fn import(module: &str, selected: &str, binding: &str) -> IrStatement {
    IrStatement {
        kind: IrStatementKind::Import {
            description: String::new(),
            items: vec![IrImportItem {
                module: module.to_owned(),
                selected: Some(name(selected)),
                binding: name(binding),
                binding_module: module.to_owned(),
            }],
        },
        span: span(),
        leading_docs: Vec::new(),
    }
}

/// 无别名 pick 必须选择第二模块，两个 relay 必须各自调用本模块 pick。
fn program() -> IrProgram {
    let mut body = vec![
        import("stdlib.binding_first", "relay", "first_relay"),
        import("stdlib.binding_second", "relay", "second_relay"),
        import("stdlib.binding_second", "pick", "pick"),
    ];
    for expression in [
        call("first_relay", &[]),
        call("second_relay", &[]),
        call("pick", &["left", "right"]),
    ] {
        body.push(IrStatement {
            kind: IrStatementKind::Expression { value: expression },
            span: span(),
            leading_docs: Vec::new(),
        });
    }
    let mut program = IrProgram::new(IrEntryMode::Script, body, span());
    program.modules = vec![
        module("stdlib.binding_first", &["value"]),
        module("stdlib.binding_second", &["left", "right"]),
    ];
    program
}

#[test]
/// 模块加载顺序与相同源码区间均不得造成函数错绑、漏发射或符号重复。
fn imports_and_internal_calls_keep_all_real_module_definitions() {
    for reverse in [false, true] {
        let mut program = program();
        if reverse {
            program.modules.reverse();
        }
        for target in [
            TargetDescription::windows_x86_64(),
            TargetDescription::linux_x86_64(),
            TargetDescription::macos_aarch64(),
        ] {
            let lowered = NativeBuild::new()
                .lower(&program, &CodegenOptions::for_target(target))
                .unwrap();
            assert_eq!(lowered.text.matches("define void @xiao.fn.").count(), 4);
            let symbols: std::collections::BTreeSet<_> = lowered
                .text
                .lines()
                .filter(|line| line.starts_with("define void @xiao.fn."))
                .collect();
            assert_eq!(symbols.len(), 4);
        }
    }
}

#[test]
/// 移除被选中的真实函数体后明确拒绝，不借用另一模块同名函数。
fn missing_selected_body_cannot_fall_back_to_another_module() {
    let mut program = program();
    program.modules[1].body.remove(0);
    let Err(CodegenError::Unsupported { feature, span }) =
        NativeBuild::new().lower(&program, &Default::default())
    else {
        panic!("缺失函数体没有明确拒绝");
    };
    assert!(feature.contains("stdlib.binding_second.pick"), "{feature}");
    assert!(span.is_some());
}
