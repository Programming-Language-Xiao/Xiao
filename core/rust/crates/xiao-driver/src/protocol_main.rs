//! X0-A Rust 核心的标准输入/输出进程入口。

use std::io::Write;

use xiao_driver::{
    ExitCode, FrameError, ProtocolResponse, core_crash_response, serve_stdio, write_frame,
};

/// 启动标准输入/输出协议服务，并把边界错误映射为进程码。
fn main() {
    let result = std::panic::catch_unwind(serve_stdio);
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            let response = protocol_error_response(&error);
            let stdout = std::io::stdout();
            let mut writer = stdout.lock();
            let _ = write_frame(&mut writer, &response);
            std::process::exit(2);
        }
        Err(_) => {
            let response = core_crash_response(None);
            let stdout = std::io::stdout();
            let mut writer = stdout.lock();
            let _ = write_frame(&mut writer, &response);
            let _ = writer.flush();
            std::process::exit(ExitCode::Fatal.as_process_code().into());
        }
    }
}

/// 将帧错误包装为没有请求 ID 的机器响应。
fn protocol_error_response(error: &FrameError) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id: None,
        error: xiao_driver::ProtocolErrorBody {
            code: error.code().to_owned(),
            message_id: "x11.protocol.frame".to_owned(),
            message: error.to_string(),
            phase: Some("framing".to_owned()),
            next_step: Some("检查长度字段、UTF-8 和 JSON 后重试".to_owned()),
            details: std::collections::BTreeMap::new(),
        },
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}
