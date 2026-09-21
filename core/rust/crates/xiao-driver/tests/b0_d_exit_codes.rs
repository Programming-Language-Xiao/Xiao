//! 09-B0-D 驱动器退出码契约回归。

use std::time::Duration;

use xiao_driver::{
    CancellationToken, DriverOutcome, DriverPhase, DriverRequest, ExitCode, FrontendCompiler,
    FrontendRequest, FrontendVmDriver, run,
};
use xiao_vm::{RunResult, VmOptions};

/// 构造使用内存来源名的驱动请求。
fn request(source: &str) -> DriverRequest {
    DriverRequest::new(FrontendRequest::from_text(source))
}

/// 同时断言结构化退出语义和冻结的进程码。
fn assert_exit_code(outcome: &DriverOutcome, expected: ExitCode) {
    assert_eq!(outcome.exit_code(), expected);
    assert_eq!(
        outcome.exit_code().as_process_code(),
        expected.as_process_code()
    );
}

#[test]
/// 普通脚本正常结束时返回成功码零。
fn normal_script_is_success_zero() {
    let outcome = run(&request("value = 1 + 2\n"));
    assert_exit_code(&outcome, ExitCode::Success);
    assert!(outcome.is_success());
}

#[test]
/// `[main]` 工程入口正常结束时也返回成功码零。
fn project_main_is_success_zero() {
    let outcome = run(&request("[main]\nvalue = 1 + 2\n"));
    assert_exit_code(&outcome, ExitCode::Success);
    assert!(outcome.is_success());
}

#[test]
/// 前端检查失败映射到源码拒绝码一。
fn frontend_failure_is_source_rejected_one() {
    let outcome = run(&request("if 1\n    value = 1\n"));
    assert!(matches!(outcome, DriverOutcome::Frontend(_)));
    assert_exit_code(&outcome, ExitCode::SourceRejected);
}

#[test]
/// 损坏 IR 在 VM 前被拒绝并映射到产物拒绝码二。
fn corrupted_ir_is_artifact_rejected_two() {
    let request = request("value = 1\n");
    let mut artifact = FrontendCompiler::new()
        .compile(&request.frontend)
        .expect("前端应成功");
    artifact.ir.span.start = artifact.ir.span.end.saturating_add(1);

    let outcome = FrontendVmDriver::new().run_artifact(&artifact, &request);
    let DriverOutcome::Rejected(error) = &outcome else {
        panic!("损坏 IR 必须在 VM 前被拒绝");
    };
    assert_eq!(error.phase(), DriverPhase::Verification);
    assert_exit_code(&outcome, ExitCode::ArtifactRejected);
}

#[test]
/// 已取消的请求映射到产物拒绝码二。
fn cancellation_is_artifact_rejected_two() {
    let token = CancellationToken::new();
    token.cancel();
    let outcome = run(&request("value = 1\n").with_cancellation(token));
    assert_exit_code(&outcome, ExitCode::ArtifactRejected);
}

#[test]
/// 零期限超时映射到产物拒绝码二。
fn timeout_is_artifact_rejected_two() {
    let outcome = run(&request("value = 1\n").with_timeout(Duration::ZERO));
    assert_exit_code(&outcome, ExitCode::ArtifactRejected);
}

#[test]
/// 未捕获的可恢复错误映射到运行时错误码三。
fn uncaught_runtime_error_is_runtime_error_three() {
    let source =
        "def square(int value) -> int\n    return value * value\nresult = square(4000000000)\n";
    let outcome = run(&request(source));
    let DriverOutcome::Executed(execution) = &outcome else {
        panic!("运行时错误已经进入 VM");
    };
    assert!(matches!(execution.outcome.result, RunResult::Error(_)));
    assert_exit_code(&outcome, ExitCode::RuntimeError);
}

#[test]
/// VM 内部 Fatal 终局映射到码四。
fn fatal_is_four() {
    let source = "def down(int value) -> int\n    return down(value)\nresult = down(1)\n";
    let request = request(source).with_options(VmOptions { max_call_depth: 6 });
    let outcome = run(&request);
    let DriverOutcome::Executed(execution) = &outcome else {
        panic!("递归栈溢出应在 VM 内形成 Fatal");
    };
    assert!(matches!(execution.outcome.result, RunResult::Fatal(_)));
    assert_exit_code(&outcome, ExitCode::Fatal);
}

#[test]
/// 被 `catch` 消费的错误仍然是成功码零。
fn caught_error_is_success_zero() {
    let source = "try\n    raise ArithmeticError(code = \"caught\")\ncatch err as ArithmeticError\n    handled = true\n";
    let outcome = run(&request(source));
    let DriverOutcome::Executed(execution) = &outcome else {
        panic!("被 catch 消费的错误应完成一次正常 VM 执行");
    };
    assert!(matches!(execution.outcome.result, RunResult::Success));
    assert_exit_code(&outcome, ExitCode::Success);
}

#[test]
/// 改变报告展示上下文不应改变退出码。
fn exit_code_is_independent_of_diagnostic_display_context() {
    let source =
        "def square(int value) -> int\n    return value * value\nresult = square(4000000000)\n";
    let first = run(&request(source).with_source_name("english.xiao"));
    let second = run(&request(source).with_source_name("localized.xiao"));

    let first_report = first.report().expect("第一次运行应有报告");
    let second_report = second.report().expect("第二次运行应有报告");
    assert_ne!(
        first_report
            .stack
            .first()
            .and_then(|frame| frame.source_file()),
        second_report
            .stack
            .first()
            .and_then(|frame| frame.source_file())
    );
    assert_eq!(first.exit_code(), ExitCode::RuntimeError);
    assert_eq!(second.exit_code(), ExitCode::RuntimeError);
    assert_eq!(first.exit_code().as_process_code(), 3);
    assert_eq!(second.exit_code().as_process_code(), 3);
}

#[test]
/// 五个冻结进程码保持连续且逐项稳定。
fn frozen_process_codes_are_contiguous_and_stable() {
    assert_eq!(ExitCode::Success.as_process_code(), 0);
    assert_eq!(ExitCode::SourceRejected.as_process_code(), 1);
    assert_eq!(ExitCode::ArtifactRejected.as_process_code(), 2);
    assert_eq!(ExitCode::RuntimeError.as_process_code(), 3);
    assert_eq!(ExitCode::Fatal.as_process_code(), 4);
}
