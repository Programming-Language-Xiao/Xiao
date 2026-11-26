//! B4-1 类型检查和 IR 形状；不执行 VM、LLVM 或原生 cmath。

use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_ir::{IrExpression, IrExpressionKind, IrStatement, IrStatementKind, IrType};

const FROZEN: [(&str, &[&str]); 15] = [
    ("sqrt", &["value"]),
    ("floor", &["value"]),
    ("ceil", &["value"]),
    ("round", &["value"]),
    ("trunc", &["value"]),
    ("abs", &["value"]),
    ("min", &["left", "right"]),
    ("max", &["left", "right"]),
    ("pow", &["base", "exponent"]),
    ("sin", &["value"]),
    ("cos", &["value"]),
    ("tan", &["value"]),
    ("exp", &["value"]),
    ("ln", &["value"]),
    ("log10", &["value"]),
];

fn assignment_value(statement: &IrStatement) -> Option<&IrExpression> {
    match &statement.kind {
        IrStatementKind::Assignment { value, .. } => Some(value),
        _ => None,
    }
}

fn returned_intrinsic(statement: &IrStatement) -> Option<(u32, &[xiao_ir::IrCallArgument])> {
    let IrStatementKind::Function { body, .. } = &statement.kind else {
        return None;
    };
    let [
        IrStatement {
            kind: IrStatementKind::Return { value: Some(value) },
            ..
        },
    ] = body.as_slice()
    else {
        return None;
    };
    match &value.kind {
        IrExpressionKind::IntrinsicCall { id, arguments } => Some((*id, arguments)),
        _ => None,
    }
}

#[test]
/// 15项冻结float调用保留intrinsic ID、实参类型/顺序、返回类型与源码区间。
fn cmath_intrinsics_typecheck_and_lower_to_frozen_ir() {
    let mut source = String::new();
    for (index, (name, parameters)) in FROZEN.iter().enumerate() {
        let positional = match parameters {
            [_] => "1.0".to_owned(),
            [_left, _right] => "1.0, 2.0".to_owned(),
            _ => unreachable!(),
        };
        let named = match parameters {
            [value] => format!("{value} = 1.0"),
            [left, right] => format!("{right} = 2.0, {left} = 1.0"),
            _ => unreachable!(),
        };
        source.push_str(&format!(
            "positional_{index} = cmath_{name}({positional})\nnamed_{index} = cmath_{name}({named})\n"
        ));
    }
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source.clone()))
        .expect("冻结cmath原语应通过类型检查");
    assert_eq!(artifact.ir.body.len(), FROZEN.len() * 2);
    for (call_index, statement) in artifact.ir.body.iter().enumerate() {
        let index = call_index / 2;
        let is_named = call_index % 2 == 1;
        let value = assignment_value(statement).expect("调用结果应写入独立变量");
        let IrExpressionKind::IntrinsicCall { id, arguments } = &value.kind else {
            panic!("第{call_index}项没有降低为IntrinsicCall：{value:?}");
        };
        let (name, parameters) = FROZEN[index];
        assert_eq!(*id, 34 + index as u32, "{name}");
        assert_eq!(
            value.ty,
            IrType::Scalar {
                name: "float".into()
            },
            "{name}"
        );
        assert_eq!(arguments.len(), parameters.len(), "{name}");
        assert!(value.span.end > value.span.start, "{name}缺调用span");
        let source_call = &source[value.span.start..value.span.end];
        assert!(
            source_call.starts_with(&format!("cmath_{name}(")),
            "{source_call}"
        );
        for (argument, expected_parameter) in arguments.iter().zip(parameters) {
            assert_eq!(
                argument.value.ty,
                IrType::Scalar {
                    name: "float".into()
                },
                "{name}"
            );
            assert!(
                argument.span.start >= value.span.start && argument.span.end <= value.span.end,
                "{name} 参数span"
            );
            assert!(
                argument.value.span.start >= argument.span.start
                    && argument.value.span.end <= argument.span.end,
                "{name} 值span"
            );
            assert!(
                source_call.contains(&format!("{expected_parameter} =")) == is_named,
                "{name}"
            );
        }
        if is_named && parameters.len() == 1 {
            assert_eq!(arguments[0].name.as_ref().unwrap().text, parameters[0]);
        } else if is_named {
            assert_eq!(arguments[0].name.as_ref().unwrap().text, parameters[1]);
            assert_eq!(arguments[1].name.as_ref().unwrap().text, parameters[0]);
        } else {
            assert!(arguments.iter().all(|argument| argument.name.is_none()));
        }
    }
    assert_eq!(xiao_runtime_abi::ABI_MINOR_VERSION, 20);
    assert_eq!(xiao_codegen_llvm::CODEGEN_VERSION, 25);
    println!("B4-1-CMATH count=15 positional=named=exact ids=34..48 return=float result=success");
}

#[test]
/// 包装模块中的原语转发也通过类型检查并在模块IR保留对应ID。
fn cmath_wrapper_module_carries_intrinsic_ir_without_enabling_import_execution() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "from std.cmath import sqrt\nprint(sqrt(4.0))\n",
        ))
        .expect("B4-1应检查包装函数体，但不执行它");
    let module = artifact
        .ir
        .modules
        .iter()
        .find(|module| module.name == "stdlib.cmath")
        .expect("标准库模块摘要保留规范身份");
    assert_eq!(module.body.len(), FROZEN.len());
    for (index, function) in module.body.iter().enumerate() {
        let (id, arguments) = returned_intrinsic(function).expect("包装只包含原语转发");
        assert_eq!(id, 34 + index as u32);
        assert_eq!(arguments.len(), FROZEN[index].1.len());
        let IrStatementKind::Function { return_type, .. } = &function.kind else {
            unreachable!();
        };
        assert_eq!(
            return_type,
            &IrType::Scalar {
                name: "float".into()
            }
        );
        for (argument, parameter) in arguments.iter().zip(FROZEN[index].1) {
            assert!(
                matches!(
                    &argument.value.kind,
                    IrExpressionKind::Name { name } if name.text == *parameter
                ),
                "{} 转发实参不是 {parameter}：{argument:?}",
                FROZEN[index].0
            );
        }
    }
    assert!(
        artifact.ir.body.iter().any(|statement| {
            matches!(&statement.kind, IrStatementKind::Import { items, .. }
            if items.iter().any(|item| item.module == "std.cmath"))
        }),
        "B4-1保留运行时导入边界，不把导入消成可执行绑定"
    );
    println!("B4-1-CMATH-WRAPPER module=stdlib.cmath ids=34..48 import=retained result=success");
}

#[test]
/// 静态非法参数、实参形态与参数绑定都用既有调用诊断并保留源码span。
fn cmath_static_call_shape_and_type_errors_are_rejected() {
    for source in [
        "value = cmath_sqrt()\n",
        "value = cmath_sqrt(1.0, 2.0)\n",
        "value = cmath_sqrt(1)\n",
        "value = cmath_min(1.0)\n",
        "value = cmath_min(1.0, 2.0, 3.0)\n",
        "value = cmath_min(first = 1.0, second = 2.0)\n",
        "value = cmath_min(left = 1.0, left = 2.0)\n",
        "value = cmath_min(1.0, left = 2.0)\n",
        "value = cmath_min(*[1.0, 2.0])\n",
        "value = cmath_min(**{})\n",
    ] {
        let error = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .expect_err(source);
        assert!(
            error.diagnostics().iter().any(|diagnostic| {
                diagnostic.is_error()
                    && diagnostic.code() == "X04-TYPE-002"
                    && diagnostic.span().is_some()
            }),
            "{source}: {error:?}"
        );
    }
    println!("B4-1-CMATH-NEGATIVE cases=10 error=X04-TYPE-002 span=present result=success");
}
