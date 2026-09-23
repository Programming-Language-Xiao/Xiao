//! 项目测试请求的顺序执行与结构化结果聚合。

use super::mapping::{exit_name, protocol_error_body};
use super::message::{ProtocolErrorBody, ProtocolResponse, ProtocolTestCaseResult};
use super::request::{
    CANCELLED_ERROR_CODE, OptimizationConfig, ProtocolTarget, RunOptions, SourceIdentity,
};
use super::run::{protocol_error_response, run_request_response};
use crate::run::{CancellationToken, ExitCode};

/// 顺序执行项目测试用例并聚合首个失败退出码。
#[allow(clippy::too_many_arguments)]
pub(super) fn test_request_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    language_version: String,
    target: ProtocolTarget,
    optimization: OptimizationConfig,
    cases: Vec<SourceIdentity>,
    options: RunOptions,
    cancellation: CancellationToken,
) -> ProtocolResponse {
    if cases.is_empty() {
        return protocol_error_response(
            Some(request_id),
            &super::request::ProtocolError::request("cases", "项目测试请求至少需要一个测试用例"),
        );
    }

    let total = cases.len();
    let mut tests = Vec::with_capacity(total);
    let mut overall_exit_code = ExitCode::Success.as_process_code();
    for source in cases {
        let path = source.path.clone().unwrap_or_else(|| source.module.clone());
        let module = source.module.clone();
        let response = run_request_response(
            request_id.clone(),
            protocol_version,
            core_version,
            language_version.clone(),
            target.clone(),
            optimization.clone(),
            source,
            options.clone(),
            cancellation.clone(),
        );
        let result = test_case_result(path, module, response);
        if overall_exit_code == ExitCode::Success.as_process_code() && result.exit_code != 0 {
            overall_exit_code = result.exit_code;
        }
        tests.push(result);
    }
    let passed = tests.iter().filter(|test| test.exit_code == 0).count();
    let failed = tests.len() - passed;
    ProtocolResponse::TestResult {
        request_id,
        operation: "test".to_owned(),
        exit_code: overall_exit_code,
        exit_name: process_exit_name(overall_exit_code).to_owned(),
        total,
        passed,
        failed,
        tests,
    }
}

/// 把单用例的普通结果或协议错误转换为统一测试结果。
fn test_case_result(
    path: String,
    module: String,
    response: ProtocolResponse,
) -> ProtocolTestCaseResult {
    match response {
        ProtocolResponse::Result {
            exit_code,
            exit_name,
            diagnostics,
            report,
            events,
            metrics,
            value,
            ..
        } => ProtocolTestCaseResult {
            path,
            module,
            exit_code,
            exit_name,
            diagnostics,
            report,
            events,
            metrics,
            value,
            error: None,
        },
        ProtocolResponse::Error {
            error,
            report,
            exit_code,
            ..
        } => ProtocolTestCaseResult {
            path,
            module,
            exit_code,
            exit_name: process_exit_name(exit_code).to_owned(),
            diagnostics: Vec::new(),
            report,
            events: Vec::new(),
            metrics: None,
            value: None,
            error: Some(error),
        },
        ProtocolResponse::Cancelled { exit_code, .. } => ProtocolTestCaseResult {
            path,
            module,
            exit_code,
            exit_name: process_exit_name(exit_code).to_owned(),
            diagnostics: Vec::new(),
            report: None,
            events: Vec::new(),
            metrics: None,
            value: None,
            error: Some(cancelled_error()),
        },
        ProtocolResponse::Hello { .. }
        | ProtocolResponse::Shutdown { .. }
        | ProtocolResponse::TestResult { .. } => {
            let exit_code = ExitCode::Fatal.as_process_code();
            ProtocolTestCaseResult {
                path,
                module,
                exit_code,
                exit_name: process_exit_name(exit_code).to_owned(),
                diagnostics: Vec::new(),
                report: None,
                events: Vec::new(),
                metrics: None,
                value: None,
                error: Some(protocol_error_body(
                    "X11-PROTOCOL-002",
                    "x11.protocol.test_response",
                    "测试用例收到无效的协议响应类型",
                    Some("protocol".to_owned()),
                    Some("检查核心版本和协议实现".to_owned()),
                    Default::default(),
                )),
            }
        }
    }
}

/// 构造测试取消的机器错误体。
fn cancelled_error() -> ProtocolErrorBody {
    protocol_error_body(
        CANCELLED_ERROR_CODE,
        "x11.protocol.cancelled",
        "请求已取消",
        Some("control".to_owned()),
        Some("重新提交请求或继续等待其他请求".to_owned()),
        Default::default(),
    )
}

/// 将 B0-D 进程码转换为稳定名称。
fn process_exit_name(code: u8) -> &'static str {
    match code {
        0 => exit_name(ExitCode::Success),
        1 => exit_name(ExitCode::SourceRejected),
        2 => exit_name(ExitCode::ArtifactRejected),
        3 => exit_name(ExitCode::RuntimeError),
        _ => exit_name(ExitCode::Fatal),
    }
}
