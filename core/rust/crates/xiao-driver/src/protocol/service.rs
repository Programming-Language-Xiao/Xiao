//! 协议请求分发和 stdin/stdout 服务。
//!
//! 这是协议职责图的最上层：只有本模块持有线程、共享输出锁和取消登记表；
//! 请求校验、运行和构建细节分别委托给下层子模块。

use std::collections::BTreeMap;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use serde_json::json;

use super::build;
use super::frame::{FrameError, decode_frame, read_frame, write_frame};
use super::mapping::{protocol_error_body, protocol_error_from_error};
use super::message::{CORE_VERSION, CoreVersions, PROTOCOL_VERSION, ProtocolResponse};
use super::request::{
    CORE_CRASH_CODE, OptimizationConfig, ProtocolError, ProtocolRequest, ProtocolTarget,
    SourceIdentity, ToolchainSpec,
};
use super::run::{protocol_error_response, run_request_response};
use super::test::test_request_response;
use super::validate::{validate_source, validate_target, validate_versions};
use crate::run::{CancellationToken, ExitCode};

/// 从流读取并解码一条请求。
pub fn read_request<R: Read>(reader: &mut R) -> Result<Option<ProtocolRequest>, FrameError> {
    let Some(payload) = read_frame(reader)? else {
        return Ok(None);
    };
    decode_frame(&payload).map(Some)
}

/// 直接处理一条请求；适合契约测试和不需要并发取消的调用方。
pub fn dispatch(request: ProtocolRequest) -> ProtocolResponse {
    match request {
        ProtocolRequest::Hello {
            request_id,
            protocol_version,
            core_version,
        } => hello_response(request_id, protocol_version, core_version),
        ProtocolRequest::Run {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            options,
        } => run_request_response(
            request_id,
            protocol_version,
            core_version,
            language_version,
            target,
            optimization,
            source,
            options,
            CancellationToken::new(),
        ),
        ProtocolRequest::Test {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            cases,
            options,
        } => test_request_response(
            request_id,
            protocol_version,
            core_version,
            language_version,
            target,
            optimization,
            cases,
            options,
            CancellationToken::new(),
        ),
        ProtocolRequest::Build {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            output,
            llvm_ir_output,
            toolchain,
            config_text,
        } => build_request_response(
            request_id,
            protocol_version,
            core_version,
            language_version,
            target,
            optimization,
            source,
            output,
            llvm_ir_output,
            toolchain,
            config_text,
            &CancellationToken::new(),
        ),
        ProtocolRequest::Cancel {
            request_id,
            protocol_version,
            core_version,
            target_request_id,
        } => cancel_response(
            request_id,
            protocol_version,
            core_version,
            target_request_id,
        ),
        ProtocolRequest::Shutdown {
            request_id,
            protocol_version,
            core_version,
        } => shutdown_response(request_id, protocol_version, core_version),
    }
}

/// 校验版本并构造 hello 响应。
fn hello_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
) -> ProtocolResponse {
    let versions = CoreVersions::current();
    match validate_versions(protocol_version, core_version) {
        Ok(()) => ProtocolResponse::Hello {
            request_id,
            accepted: true,
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            versions,
            capabilities: vec![
                "run".to_owned(),
                "test".to_owned(),
                "build".to_owned(),
                "cancel".to_owned(),
            ],
            error: None,
        },
        Err(error) => ProtocolResponse::Hello {
            request_id,
            accepted: false,
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            versions,
            capabilities: Vec::new(),
            error: Some(protocol_error_from_error(&error)),
        },
    }
}

#[allow(clippy::too_many_arguments)]
/// 校验构建请求并交给原生构建路径。
fn build_request_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    language_version: String,
    target: ProtocolTarget,
    optimization: OptimizationConfig,
    source: SourceIdentity,
    output: String,
    llvm_ir_output: Option<String>,
    toolchain: ToolchainSpec,
    config_text: Option<String>,
    cancellation: &CancellationToken,
) -> ProtocolResponse {
    if let Err(error) = validate_versions(protocol_version, core_version) {
        return protocol_error_response(Some(request_id), &error);
    }
    if let Err(error) = validate_source(&source).and_then(|_| validate_target(&target)) {
        return protocol_error_response(Some(request_id), &error);
    }
    build::build_response(
        request_id,
        language_version,
        target,
        optimization,
        source,
        output,
        llvm_ir_output,
        toolchain,
        config_text,
        cancellation,
    )
}

/// 校验取消请求并返回协议层取消结果。
fn cancel_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    target_request_id: String,
) -> ProtocolResponse {
    if let Err(error) = validate_versions(protocol_version, core_version) {
        return protocol_error_response(Some(request_id), &error);
    }
    ProtocolResponse::Cancelled {
        request_id,
        target_request_id,
        accepted: false,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

/// 校验关闭请求并返回服务关闭结果。
fn shutdown_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
) -> ProtocolResponse {
    if let Err(error) = validate_versions(protocol_version, core_version) {
        return protocol_error_response(Some(request_id), &error);
    }
    ProtocolResponse::Shutdown { request_id }
}

/// 服务线程共享的串行输出锁。
type SharedWriter<W> = Arc<Mutex<BufWriter<W>>>;
/// 请求 ID 到取消令牌的登记表。
type CancellationMap = Arc<Mutex<BTreeMap<String, CancellationToken>>>;

/// 在线程安全的输出锁上写入一条响应。
fn write_response<W: Write>(writer: &SharedWriter<W>, response: &ProtocolResponse) {
    if let Ok(mut writer) = writer.lock() {
        let _ = write_frame(&mut *writer, response);
    }
}

/// 在线程中执行运行/构建请求，并把 panic 转为稳定响应。
pub(super) fn worker_response(
    request: ProtocolRequest,
    token: CancellationToken,
) -> ProtocolResponse {
    match request {
        ProtocolRequest::Run {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            options,
        } => run_request_response(
            request_id,
            protocol_version,
            core_version,
            language_version,
            target,
            optimization,
            source,
            options,
            token,
        ),
        ProtocolRequest::Test {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            cases,
            options,
        } => test_request_response(
            request_id,
            protocol_version,
            core_version,
            language_version,
            target,
            optimization,
            cases,
            options,
            token,
        ),
        ProtocolRequest::Build {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            output,
            llvm_ir_output,
            toolchain,
            config_text,
        } => build_request_response(
            request_id,
            protocol_version,
            core_version,
            language_version,
            target,
            optimization,
            source,
            output,
            llvm_ir_output,
            toolchain,
            config_text,
            &token,
        ),
        other => dispatch(other),
    }
}

/// 在拥有的输入/输出流上运行可取消的协议服务。
pub fn serve<R, W>(reader: R, writer: W) -> Result<(), FrameError>
where
    R: Read,
    W: Write + Send + 'static,
{
    let mut reader = BufReader::new(reader);
    let writer = Arc::new(Mutex::new(BufWriter::new(writer)));
    let cancellations: CancellationMap = Arc::new(Mutex::new(BTreeMap::new()));
    let mut workers: Vec<JoinHandle<()>> = Vec::new();
    let mut first_frame = true;
    let mut negotiated = false;

    while let Some(request) = read_request(&mut reader)? {
        if first_frame {
            first_frame = false;
            if let Some(response) = reject_non_hello_first_frame(&request) {
                write_response(&writer, &response);
                break;
            }
        }
        match request {
            ProtocolRequest::Hello { .. } => {
                let response = dispatch(request);
                negotiated = matches!(&response, ProtocolResponse::Hello { accepted: true, .. });
                write_response(&writer, &response);
            }
            ProtocolRequest::Cancel {
                request_id,
                protocol_version,
                core_version,
                target_request_id,
            } => handle_cancel(
                &writer,
                &cancellations,
                negotiated,
                request_id,
                protocol_version,
                core_version,
                target_request_id,
            ),
            ProtocolRequest::Shutdown {
                request_id,
                protocol_version,
                core_version,
            } => {
                if handle_shutdown(
                    &writer,
                    negotiated,
                    request_id,
                    protocol_version,
                    core_version,
                ) {
                    break;
                }
            }
            request @ (ProtocolRequest::Run { .. }
            | ProtocolRequest::Test { .. }
            | ProtocolRequest::Build { .. }) => {
                if !negotiated {
                    let request_id = request_id_for(&request).expect("run/test/build request id");
                    let error = ProtocolError::version("必须先完成 hello 版本协商");
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                    continue;
                }
                spawn_worker(request, &writer, &cancellations, &mut workers);
            }
        }
    }
    for worker in workers {
        let _ = worker.join();
    }
    Ok(())
}

/// 拒绝未以 hello 开始的协议会话。
fn reject_non_hello_first_frame(request: &ProtocolRequest) -> Option<ProtocolResponse> {
    if matches!(request, ProtocolRequest::Hello { .. }) {
        return None;
    }
    let error = ProtocolError::version("首帧必须是 hello 版本协商");
    Some(protocol_error_response(request_id_for(request), &error))
}

/// 从可关联响应的请求中提取请求编号。
fn request_id_for(request: &ProtocolRequest) -> Option<String> {
    match request {
        ProtocolRequest::Run { request_id, .. }
        | ProtocolRequest::Test { request_id, .. }
        | ProtocolRequest::Build { request_id, .. }
        | ProtocolRequest::Cancel { request_id, .. }
        | ProtocolRequest::Shutdown { request_id, .. } => Some(request_id.clone()),
        ProtocolRequest::Hello { .. } => None,
    }
}

/// 处理已协商会话中的取消请求。
fn handle_cancel<W: Write>(
    writer: &SharedWriter<W>,
    cancellations: &CancellationMap,
    negotiated: bool,
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    target_request_id: String,
) {
    if !negotiated {
        let error = ProtocolError::version("必须先完成 hello 版本协商");
        write_response(writer, &protocol_error_response(Some(request_id), &error));
        return;
    }
    if let Err(error) = validate_versions(protocol_version, core_version) {
        write_response(writer, &protocol_error_response(Some(request_id), &error));
        return;
    }
    let accepted = cancellations
        .lock()
        .ok()
        .and_then(|map| map.get(&target_request_id).cloned())
        .map(|token| {
            token.cancel();
            true
        })
        .unwrap_or(false);
    write_response(
        writer,
        &ProtocolResponse::Cancelled {
            request_id,
            target_request_id,
            accepted,
            exit_code: ExitCode::ArtifactRejected.as_process_code(),
        },
    );
}

/// 处理已协商会话中的关闭请求，并报告是否结束服务循环。
fn handle_shutdown<W: Write>(
    writer: &SharedWriter<W>,
    negotiated: bool,
    request_id: String,
    protocol_version: u16,
    core_version: u32,
) -> bool {
    if !negotiated {
        let error = ProtocolError::version("必须先完成 hello 版本协商");
        write_response(writer, &protocol_error_response(Some(request_id), &error));
        return false;
    }
    if let Err(error) = validate_versions(protocol_version, core_version) {
        write_response(writer, &protocol_error_response(Some(request_id), &error));
        return false;
    }
    write_response(writer, &ProtocolResponse::Shutdown { request_id });
    true
}

/// 登记取消令牌并在线程中执行运行、测试或构建请求。
fn spawn_worker<W: Write + Send + 'static>(
    request: ProtocolRequest,
    writer: &SharedWriter<W>,
    cancellations: &CancellationMap,
    workers: &mut Vec<JoinHandle<()>>,
) {
    let request_id = request_id_for(&request).expect("run/test/build request id");
    let token = CancellationToken::new();
    if let Ok(mut map) = cancellations.lock() {
        map.insert(request_id.clone(), token.clone());
    }
    let writer_clone = Arc::clone(writer);
    let cancellations_clone = Arc::clone(cancellations);
    workers.push(thread::spawn(move || {
        let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            worker_response(request, token)
        }))
        .unwrap_or_else(|_| core_crash_response(Some(request_id.clone())));
        if let Ok(mut map) = cancellations_clone.lock() {
            map.remove(&request_id);
        }
        write_response(&writer_clone, &response);
    }));
}

/// 使用标准输入/输出启动 Rust 核心协议服务。
pub fn serve_stdio() -> Result<(), FrameError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve(stdin, stdout)
}

/// 将任意 panic 统一包装为核心崩溃响应。
#[must_use]
pub fn core_crash_response(request_id: Option<String>) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id,
        error: protocol_error_body(
            CORE_CRASH_CODE,
            "x11.protocol.core_crash",
            "Rust 核心处理请求时发生内部崩溃",
            Some("core".to_owned()),
            Some("收集核心日志并重启核心进程".to_owned()),
            BTreeMap::from([
                ("status".to_owned(), json!("panic")),
                (
                    "process_exit_code".to_owned(),
                    json!(ExitCode::Fatal.as_process_code()),
                ),
            ]),
        ),
        report: None,
        exit_code: ExitCode::Fatal.as_process_code(),
    }
}
