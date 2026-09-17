//! 08-U0 统一前端流水线规格。

use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_ir::IrStatementKind;

#[test]
/// 无错误源码应得到经过验证的 IR，且保留源码入口模式。
fn compiles_source_to_verified_ir() {
    let request = FrontendRequest::from_text("value = 1\n");
    let artifact = FrontendCompiler::new().compile(&request).expect("frontend");
    assert_eq!(artifact.ir.version, xiao_ir::IR_VERSION);
    assert!(matches!(
        artifact.ir.entry_mode,
        xiao_ir::IrEntryMode::Script
    ));
    assert!(matches!(
        artifact.ir.body[0].kind,
        IrStatementKind::Assignment { .. }
    ));
}

#[test]
/// 语法和类型错误应累积后返回失败，不产生可消费 IR。
fn accumulates_diagnostics_and_stops_lowering() {
    let request = FrontendRequest::from_text("if 1\n    value = 1\n");
    let error = FrontendCompiler::new()
        .compile(&request)
        .expect_err("must fail");
    assert!(error.has_errors());
    assert!(!error.diagnostics().is_empty());
}

#[test]
/// 目标和语言版本应只作为 IR 元数据传递，不改变前端语义。
fn carries_context_metadata() {
    let mut context = xiao_driver::FrontendContext::host();
    context.target = "windows-x86_64".to_owned();
    context.language_version = "0.1-test".to_owned();
    let request = FrontendRequest::from_text("value = 1\n").with_context(context);
    let artifact = xiao_driver::compile(&request).expect("frontend");
    assert_eq!(artifact.ir.target, "windows-x86_64");
    assert_eq!(artifact.ir.language_version, "0.1-test");
}
