//! B4-3 FIX.1：只把Float pow自身的numeric_range留在静态路径。
use xiao_codegen_llvm::{CodegenOptions, NativeBuild};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_ir::{
    IrExpression, IrExpressionKind, IrRuntimeCheck, IrStatement, IrStatementKind, IrType,
};

fn floating(ty: &IrType) -> bool {
    matches!(ty, IrType::Scalar { name } if name == "float" || name == "sfloat")
}

fn pow_spans(expression: &IrExpression, spans: &mut Vec<xiao_ir::IrSpan>) {
    match &expression.kind {
        IrExpressionKind::Binary {
            operator,
            left,
            right,
        } => {
            if operator == "**"
                && floating(&expression.ty)
                && floating(&left.ty)
                && floating(&right.ty)
            {
                spans.push(expression.span);
            }
            pow_spans(left, spans);
            pow_spans(right, spans);
        }
        IrExpressionKind::Group { expression } | IrExpressionKind::Cast { expression, .. } => {
            pow_spans(expression, spans)
        }
        IrExpressionKind::Unary { operand, .. } => pow_spans(operand, spans),
        IrExpressionKind::Call { callee, arguments }
        | IrExpressionKind::NewCall { callee, arguments } => {
            pow_spans(callee, spans);
            for argument in arguments {
                pow_spans(&argument.value, spans);
            }
        }
        IrExpressionKind::IntrinsicCall { arguments, .. }
        | IrExpressionKind::DictBuiltin { arguments, .. } => {
            for argument in arguments {
                pow_spans(&argument.value, spans);
            }
        }
        _ => {}
    }
}

fn pow_spans_in(statements: &[IrStatement], spans: &mut Vec<xiao_ir::IrSpan>) {
    for statement in statements {
        match &statement.kind {
            IrStatementKind::Function { body, .. } => pow_spans_in(body, spans),
            IrStatementKind::Return { value: Some(value) }
            | IrStatementKind::Expression { value }
            | IrStatementKind::Assignment { value, .. } => pow_spans(value, spans),
            IrStatementKind::If {
                body,
                elif_branches,
                else_body,
                ..
            } => {
                pow_spans_in(body, spans);
                for branch in elif_branches {
                    pow_spans_in(&branch.body, spans);
                }
                if let Some(body) = else_body {
                    pow_spans_in(body, spans);
                }
            }
            _ => {}
        }
    }
}

fn compile(source: &str) -> xiao_driver::FrontendArtifact {
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap()
}

#[test]
/// 整数递归里的numeric_range仍进入动态路由并登记weak传递依赖。
fn integer_numeric_range_only_keeps_dynamic_weak_runtime() {
    let source = "def recurse(int depth) -> int\n    if depth == 0\n        return 0\n    return recurse(depth - 1)\nresult = recurse(2)\n";
    let artifact = compile(source);
    let checks: Vec<_> = artifact
        .ir
        .runtime_checks
        .iter()
        .filter(|check| check.kind == "numeric_range")
        .collect();
    assert!(!checks.is_empty());
    assert!(
        checks
            .iter()
            .any(|check| &source[check.span.start..check.span.end] == "depth - 1")
    );
    for check in &checks {
        println!(
            "FIX1-RANGE class=integer kind={} span={}..{} expression={:?}",
            check.kind,
            check.span.start,
            check.span.end,
            &source[check.span.start..check.span.end]
        );
    }
    let module = NativeBuild::new()
        .lower(&artifact.ir, &CodegenOptions::default())
        .unwrap();
    assert!(
        module
            .runtime_components
            .iter()
            .any(|component| component == "weak")
    );
    assert!(module.text.contains("define void @xiao.fn."));
}

#[test]
/// Float pow范围检查精确匹配pow表达式；静态路径继续共享cmath_pow且不发射llvm.pow.f64。
fn float_pow_numeric_range_keeps_static_shared_kernel() {
    let source = "def power(float base, float exponent) -> float\n    return base ** exponent\nresult = power(2.0, 3.0)\n";
    let artifact = compile(source);
    let mut spans = Vec::new();
    pow_spans_in(&artifact.ir.body, &mut spans);
    assert_eq!(spans.len(), 1);
    let float_pow_span = spans[0];
    let checks: Vec<&IrRuntimeCheck> = artifact
        .ir
        .runtime_checks
        .iter()
        .filter(|check| check.kind == "numeric_range")
        .collect();
    assert!(checks.iter().any(|check| check.span == float_pow_span));
    assert!(
        checks.iter().all(|check| check.span == float_pow_span),
        "额外检查不能搭便车：{checks:?}"
    );
    let module = NativeBuild::new()
        .lower(&artifact.ir, &CodegenOptions::default())
        .unwrap();
    assert!(module.text.contains("@xiao_runtime_cmath_pow"));
    assert!(!module.text.contains("llvm.pow.f64"));
    assert!(!module.text.contains("define void @xiao.fn."));
    assert!(
        !module
            .runtime_components
            .iter()
            .any(|component| component == "weak")
    );
    println!(
        "FIX1-ROUTE class=float-pow check_span={}..{} lowering=static kernel=cmath_pow llvm_pow_f64=false result=passed",
        float_pow_span.start, float_pow_span.end
    );

    let mixed = "def power(float base, float exponent, int depth) -> float\n    remaining = depth - 1\n    return base ** exponent\nresult = power(2.0, 3.0, 4)\n";
    let artifact = compile(mixed);
    let mut float_spans = Vec::new();
    pow_spans_in(&artifact.ir.body, &mut float_spans);
    assert_eq!(float_spans.len(), 1);
    assert!(artifact.ir.runtime_checks.iter().any(|check| {
        check.kind == "numeric_range"
            && check.span != float_spans[0]
            && &mixed[check.span.start..check.span.end] == "depth - 1"
    }));
    let module = NativeBuild::new()
        .lower(&artifact.ir, &CodegenOptions::default())
        .unwrap();
    assert!(
        module
            .runtime_components
            .iter()
            .any(|component| component == "weak")
    );
    assert!(module.text.contains("define void @xiao.fn."));
    println!("FIX1-ROUTE class=mixed integer_range=dynamic float_pow=shared_runtime result=passed");
}

#[test]
/// FIX.1不改变sfloat现有的静态f32幂指令路径。
fn sfloat_pow_keeps_existing_static_f32_intrinsic() {
    let source = "def power(sfloat base, sfloat exponent) -> sfloat\n    return base ** exponent\nresult = power(sfloat(2.0), sfloat(3.0))\n";
    let artifact = compile(source);
    let mut spans = Vec::new();
    pow_spans_in(&artifact.ir.body, &mut spans);
    assert_eq!(spans.len(), 1);
    assert!(
        artifact
            .ir
            .runtime_checks
            .iter()
            .any(|check| check.kind == "numeric_range" && check.span == spans[0])
    );
    let module = NativeBuild::new()
        .lower(&artifact.ir, &CodegenOptions::default())
        .unwrap();
    assert!(module.text.contains("@llvm.pow.f32"));
    assert!(!module.text.contains("@xiao_runtime_cmath_pow"));
    assert!(!module.text.contains("define void @xiao.fn."));
}
