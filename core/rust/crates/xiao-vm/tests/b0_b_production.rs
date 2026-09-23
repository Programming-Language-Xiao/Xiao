//! B0-B 生产 VM 契约回归。

use xiao_bytecode::lower_program;
use xiao_diagnostics::{FATAL_CORRUPT_ARTIFACT_CODE, FATAL_RUNTIME_INVARIANT_CODE};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_ir::IrEntryMode;
use xiao_vm::{
    CancellationSource, CancellationToken, RunRequest, RunResult, VmEvent, VmOptions, run_request,
};

/// 使用真实前端编译并降低一份测试源码。
fn compile(source: &str) -> (xiao_ir::IrProgram, xiao_bytecode::TacProgram) {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap_or_else(|error| panic!("前端应成功: {:?}", error.diagnostics()));
    let ir = artifact.ir;
    let tac = lower_program(&ir);
    (ir, tac)
}

/// 脚本和工程入口都固定降低到函数零。
#[test]
fn script_and_project_share_entry_zero() {
    let (script_ir, script_tac) = compile("value = 1 + 2\n");
    let (project_ir, project_tac) = compile("[main]\nvalue = 1 + 2\n");
    assert!(matches!(script_ir.entry_mode, IrEntryMode::Script));
    assert!(matches!(project_ir.entry_mode, IrEntryMode::Project { .. }));
    assert_eq!(script_tac.functions[0].name, "");
    assert_eq!(project_tac.functions[0].name, "");

    let script = run_request(&RunRequest::new(&script_ir, &script_tac));
    let project = run_request(&RunRequest::new(&project_ir, &project_tac));
    assert!(script.result.is_success(), "脚本结果: {:?}", script.result);
    assert!(
        project.result.is_success(),
        "工程结果: {:?}",
        project.result
    );
    assert!(script.events.iter().any(|event| matches!(
        event,
        VmEvent::ModuleLoaded { module } if module == "main"
    )));
}

/// 生产入口必须在创建 VM 前拒绝损坏 TAC。
#[test]
fn production_rejects_unverified_tac_before_entering_vm() {
    let (ir, mut tac) = compile("value = 1\n");
    tac.unsupported.push("测试注入的未降低构造".to_owned());
    let outcome = run_request(&RunRequest::new(&ir, &tac));
    assert!(matches!(outcome.result, RunResult::Fatal(_)));
    assert_eq!(
        outcome.result.error_code(),
        Some(FATAL_CORRUPT_ARTIFACT_CODE)
    );
    assert_eq!(
        outcome.report.as_ref().map(|report| report.class),
        Some(xiao_diagnostics::ReportClass::Fatal)
    );
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, VmEvent::FunctionEntered { .. }))
    );
}

/// IR 结构损坏也必须在验证关卡返回 Fatal，而不是让后端解引用坏表。
#[test]
fn production_rejects_invalid_ir_before_vm() {
    let (mut ir, tac) = compile("value = 1\n");
    ir.span = xiao_ir::IrSpan::new(10, 1);
    let outcome = run_request(&RunRequest::new(&ir, &tac));
    assert!(matches!(outcome.result, RunResult::Fatal(_)));
    assert_eq!(
        outcome.result.error_code(),
        Some(FATAL_CORRUPT_ARTIFACT_CODE)
    );
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, VmEvent::FunctionEntered { .. }))
    );
}

/// 非法调用深度参数必须转成结构化 Fatal。
#[test]
fn production_rejects_invalid_options_with_structured_fatal() {
    let (ir, tac) = compile("value = 1\n");
    let request = RunRequest::new(&ir, &tac)
        .with_options(VmOptions {
            max_call_depth: 0,
            ..VmOptions::default()
        })
        .with_event_capacity(1);
    let outcome = run_request(&request);
    assert!(matches!(outcome.result, RunResult::Fatal(_)));
    assert_eq!(
        outcome.result.error_code(),
        Some(FATAL_RUNTIME_INVARIANT_CODE)
    );
    assert_eq!(
        outcome.report.as_ref().map(|report| report.class),
        Some(xiao_diagnostics::ReportClass::Fatal)
    );
    assert!(
        outcome
            .events
            .iter()
            .any(|event| matches!(event, VmEvent::FatalRaised { .. }))
    );
}

/// 未捕获错误报告应包含模块、源码名和 pc 反解跨度。
#[test]
fn production_fault_report_contains_source_span_and_identity() {
    let source =
        "def square(int value) -> int\n    return value * value\nresult = square(4000000000)\n";
    let (ir, tac) = compile(source);
    let request = RunRequest::new(&ir, &tac)
        .with_module_name("sample")
        .with_source_name("sample.xiao");
    let outcome = run_request(&request);
    assert!(matches!(outcome.result, RunResult::Error(_)));
    let report = outcome.report.expect("未捕获错误必须有报告");
    let frame = report.stack.first().expect("报告必须有调用栈");
    assert_eq!(frame.module(), "sample");
    assert_eq!(frame.source_file(), Some("sample.xiao"));
    assert!(frame.span().is_some(), "pc 映射必须回填源码跨度");
}

/// 有界事件接收器丢弃观测时不能改变执行结果和释放计数。
#[test]
fn bounded_events_do_not_change_result_or_release_sequence() {
    let (ir, tac) = compile("value = \"x\"\n");
    let complete = run_request(&RunRequest::new(&ir, &tac).with_event_capacity(256));
    let bounded = run_request(&RunRequest::new(&ir, &tac).with_event_capacity(1));
    assert!(complete.result.is_success());
    assert!(bounded.result.is_success());
    assert_eq!(complete.value, bounded.value);
    assert_eq!(complete.metrics, bounded.metrics);
    assert_eq!(complete.metrics.releases, bounded.metrics.releases);
    assert!(bounded.dropped_events > 0);
}

/// 取消源在生产 VM 内部应能终止主解释循环，且关闭开关保持旧行为。
#[test]
fn production_checkpoint_can_cancel_and_can_be_disabled() {
    let (ir, tac) = compile("value = 1\n");
    let token = CancellationToken::new();
    token.cancel();
    let enabled = VmOptions {
        checkpoint_interval: 1,
        ..VmOptions::default()
    };
    let cancelled = run_request(
        &RunRequest::new(&ir, &tac)
            .with_options(enabled)
            .with_cancellation(CancellationSource::new().with_token(token.clone())),
    );
    assert!(matches!(cancelled.result, RunResult::Cancelled));
    assert_eq!(cancelled.metrics.instructions, 1);

    let disabled = VmOptions {
        checkpoints_enabled: false,
        checkpoint_interval: 1,
        ..VmOptions::default()
    };
    let completed = run_request(
        &RunRequest::new(&ir, &tac)
            .with_options(disabled)
            .with_cancellation(CancellationSource::new().with_token(token)),
    );
    assert!(completed.result.is_success());
}
