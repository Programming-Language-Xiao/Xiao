//! 09-B0-C 前端到生产 VM 驱动器公共契约回归。

use std::time::Duration;

use xiao_driver::{
    CancellationToken, DRIVER_CANCELLED_CODE, DRIVER_TIMEOUT_CODE, DriverOutcome, DriverPhase,
    DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver, run,
};

/// 构造一份公共驱动器测试请求。
fn request(source: &str) -> DriverRequest {
    DriverRequest::new(FrontendRequest::from_text(source))
}

#[test]
/// 脚本与 `[main]` 工程入口都应通过公共驱动器执行。
fn runs_script_and_project_entries_through_the_public_driver() {
    let script = run(&request("value = 1 + 2\n"));
    assert!(script.is_success());
    let script_execution = script.as_executed().expect("脚本应进入 VM");
    assert!(!script_execution.events().is_empty());

    let project_request = request("[main]\nvalue = 1 + 2\n");
    let artifact = FrontendCompiler::new()
        .compile(&project_request.frontend)
        .expect("[main] 前端应成功");
    assert!(matches!(
        artifact.ir.entry_mode,
        xiao_ir::IrEntryMode::Project { .. }
    ));
    let project = FrontendVmDriver::new().run_artifact(&artifact, &project_request);
    assert!(project.is_success());
    assert!(project.as_executed().is_some());
}

#[test]
/// 前端阶段失败应作为第一段结构化结果返回。
fn frontend_failure_remains_a_structured_first_stage_result() {
    let outcome = run(&request("if 1\n    value = 1\n"));
    let DriverOutcome::Frontend(error) = outcome else {
        panic!("前端错误不应进入降低或 VM 阶段");
    };
    assert!(error.has_errors());
    assert!(!error.diagnostics().is_empty());
}

#[test]
/// 已进入 VM 的运行时错误应保留生产报告，而不是变成执行前拒绝。
fn runtime_failure_is_executed_and_keeps_the_vm_report() {
    let source =
        "def square(int value) -> int\n    return value * value\nresult = square(4000000000)\n";
    let outcome = run(&request(source));
    let DriverOutcome::Executed(execution) = outcome else {
        panic!("运行时错误已经进入 VM，不能被标成执行前拒绝");
    };
    assert!(!execution.outcome.result.is_success());
    assert_eq!(
        execution.outcome.result.error_code(),
        Some("X06-RUNTIME-009")
    );
    assert!(execution.report().is_some());
}

#[test]
/// 损坏 IR 应在创建 VM 前被生产验证器拒绝。
fn invalid_ir_is_rejected_before_the_vm_starts() {
    let request = request("value = 1\n");
    let mut artifact = FrontendCompiler::new()
        .compile(&request.frontend)
        .expect("前端应成功");
    artifact.ir.span.start = artifact.ir.span.end.saturating_add(1);

    let outcome = FrontendVmDriver::new().run_artifact(&artifact, &request);
    let DriverOutcome::Rejected(error) = outcome else {
        panic!("损坏 IR 必须在 VM 前被拒绝");
    };
    assert_eq!(error.phase(), DriverPhase::Verification);
    assert_eq!(error.code(), "X09-BYTECODE-002");
    assert!(error.report().is_some());
}

#[test]
/// 非法 VM 请求字段应作为结构化请求阶段拒绝返回。
fn invalid_vm_request_is_exposed_as_a_request_rejection() {
    let outcome = run(&request("value = 1\n").with_event_capacity(0));
    let DriverOutcome::Rejected(error) = outcome else {
        panic!("非法 VM 请求字段必须在执行前被拒绝");
    };
    assert_eq!(error.phase(), DriverPhase::Request);
    assert_eq!(error.code(), "X09-VM-002");
    assert_eq!(error.path(), Some("event_capacity"));
    assert!(error.report().is_some());
}

#[test]
/// 取消和超时编号应在公共驱动器边界保持稳定。
fn cancellation_and_timeout_are_stable_driver_boundary_results() {
    let token = CancellationToken::new();
    token.cancel();
    let cancelled = run(&request("value = 1\n").with_cancellation(token));
    assert_eq!(cancelled.code(), Some(DRIVER_CANCELLED_CODE));
    assert!(matches!(cancelled, DriverOutcome::Rejected(_)));

    let timed_out = run(&request("value = 1\n").with_timeout(Duration::ZERO));
    assert_eq!(timed_out.code(), Some(DRIVER_TIMEOUT_CODE));
    assert!(matches!(timed_out, DriverOutcome::Rejected(_)));
}
