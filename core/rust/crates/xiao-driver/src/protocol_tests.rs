//! 构建协议的帧边界、兼容性、原生输出与旁置文件回归测试。

use super::*;
use std::io::Cursor;
use std::sync::Arc;

/// 测试用的线程安全输出缓冲区。
#[derive(Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuffer {
    /// 追加响应帧字节。
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("buffer lock").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    /// 测试缓冲区无需额外刷新动作。
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// 构造一条正确版本的 hello 请求。
fn hello() -> ProtocolRequest {
    ProtocolRequest::Hello {
        request_id: "hello-1".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
    }
}

#[test]
/// 长度字段固定为 8 字节大端且只计算 JSON 负载。
fn frame_uses_eight_byte_big_endian_payload_length() {
    let frame = encode_frame(&hello()).expect("frame");
    assert_eq!(
        &frame[..FRAME_LENGTH_BYTES],
        &[0, 0, 0, 0, 0, 0, 0, frame.len() as u8 - 8]
    );
    let decoded: ProtocolRequest = decode_frame(&frame[FRAME_LENGTH_BYTES..]).expect("decode");
    assert_eq!(decoded, hello());
}

#[test]
/// 干净 EOF 可结束服务，部分长度必须拒绝。
fn read_frame_accepts_clean_eof_and_rejects_truncation() {
    assert!(
        read_frame(&mut Cursor::new(Vec::<u8>::new()))
            .expect("eof")
            .is_none()
    );
    let error = read_frame(&mut Cursor::new(vec![1, 2])).expect_err("truncated");
    assert!(matches!(error, FrameError::TruncatedLength { read: 2 }));
}

#[test]
/// 超长帧在分配前被拒绝。
fn read_frame_rejects_oversized_payload_before_allocating() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&((MAX_FRAME_BYTES as u64) + 1).to_be_bytes());
    let error = read_frame(&mut Cursor::new(bytes)).expect_err("oversized");
    assert!(matches!(error, FrameError::LengthTooLarge { .. }));
}

#[test]
/// 版本失配返回稳定机器错误码。
fn version_mismatch_is_machine_readable() {
    let response = dispatch(ProtocolRequest::Hello {
        request_id: "bad".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION + 1,
    });
    let ProtocolResponse::Hello {
        accepted, error, ..
    } = response
    else {
        panic!("expected hello response");
    };
    assert!(!accepted);
    assert_eq!(error.expect("error").code, VERSION_MISMATCH_CODE);
}

#[test]
/// hello 能明确声明项目测试操作，避免客户端猜测能力。
fn hello_advertises_test_capability() {
    let ProtocolResponse::Hello { capabilities, .. } = dispatch(hello()) else {
        panic!("hello must produce hello response");
    };
    assert!(capabilities.iter().any(|capability| capability == "test"));
}

#[test]
/// 空测试集合在协议边界被拒绝，而不是产生虚假的成功结果。
fn test_request_rejects_empty_case_list() {
    let response = dispatch(ProtocolRequest::Test {
        request_id: "test-empty".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: OptimizationConfig::default(),
        cases: Vec::new(),
        options: RunOptions::default(),
    });
    let ProtocolResponse::Error { error, .. } = response else {
        panic!("empty test cases must produce an error");
    };
    assert_eq!(error.code, REQUEST_ERROR_CODE);
    assert_eq!(
        error.details["field"],
        serde_json::Value::String("cases".to_owned())
    );
}

#[test]
/// 测试响应保留请求顺序，并以首个非零用例码作为整体退出码。
fn test_request_preserves_case_order_and_aggregates_results() {
    let response = dispatch(ProtocolRequest::Test {
        request_id: "test-order".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: OptimizationConfig::default(),
        cases: vec![
            SourceIdentity {
                module: "tests/z-last".to_owned(),
                path: Some("tests/z-last.xiao".to_owned()),
                text: "value = 1\n".to_owned(),
            },
            SourceIdentity {
                module: "tests/a-first".to_owned(),
                path: Some("tests/a-first.xiao".to_owned()),
                text: "value = 1\n".to_owned(),
            },
        ],
        options: RunOptions::default(),
    });
    let ProtocolResponse::TestResult {
        exit_code,
        total,
        passed,
        failed,
        tests,
        ..
    } = response
    else {
        panic!("test request must produce test_result");
    };
    assert_eq!(exit_code, 0);
    assert_eq!((total, passed, failed), (2, 2, 0));
    assert_eq!(
        tests
            .iter()
            .map(|test| test.path.as_str())
            .collect::<Vec<_>>(),
        ["tests/z-last.xiao", "tests/a-first.xiao"]
    );
}

#[test]
/// 首帧不是 hello 时会拒绝会话，不让请求绕过版本协商。
fn service_requires_hello_as_first_frame() {
    let request = ProtocolRequest::Run {
        request_id: "run-before-hello".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: OptimizationConfig::default(),
        source: SourceIdentity {
            module: "main".to_owned(),
            path: None,
            text: "value = 1\n".to_owned(),
        },
        options: RunOptions::default(),
    };
    let mut input = encode_frame(&request).expect("run frame");
    input.extend_from_slice(&encode_frame(&hello()).expect("hello frame"));
    let output = SharedBuffer::default();
    let output_view = Arc::clone(&output.0);
    serve(Cursor::new(input), output).expect("service");

    let mut cursor = Cursor::new(output_view.lock().expect("buffer lock").clone());
    let payload = read_frame(&mut cursor)
        .expect("error frame")
        .expect("one response");
    let ProtocolResponse::Error { error, .. } = decode_frame(&payload).expect("response") else {
        panic!("首帧违规应返回 error");
    };
    assert_eq!(error.code, VERSION_MISMATCH_CODE);
    assert!(read_frame(&mut cursor).expect("end of session").is_none());
}

#[test]
/// 帧损坏和语义请求错误使用不同的稳定机器码。
fn frame_and_request_errors_keep_distinct_codes() {
    let frame_error = decode_frame::<ProtocolRequest>(b"not-json").expect_err("bad json");
    assert_eq!(frame_error.code(), FRAME_ERROR_CODE);

    let response = dispatch(ProtocolRequest::Run {
        request_id: "bad-source".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: OptimizationConfig::default(),
        source: SourceIdentity {
            module: "  ".to_owned(),
            path: None,
            text: "value = 1\n".to_owned(),
        },
        options: RunOptions::default(),
    });
    let ProtocolResponse::Error { error, .. } = response else {
        panic!("无效源码身份应返回 error");
    };
    assert_eq!(error.code, REQUEST_ERROR_CODE);
}

#[test]
/// 取消令牌映射到冻结的产物拒绝进程码。
fn real_source_run_maps_cancel_to_artifact_rejected() {
    let token = CancellationToken::new();
    token.cancel();
    let request = ProtocolRequest::Run {
        request_id: "run-1".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: OptimizationConfig::default(),
        source: SourceIdentity {
            module: "main".to_owned(),
            path: None,
            text: "value = 1\n".to_owned(),
        },
        options: RunOptions::default(),
    };
    let response = worker_response(request, token);
    let (ProtocolResponse::Result { exit_code, .. } | ProtocolResponse::Error { exit_code, .. }) =
        response
    else {
        panic!("expected run response");
    };
    assert_eq!(exit_code, ExitCode::ArtifactRejected.as_process_code());
}

#[test]
/// 真实 Xiao 源码经前端和 VM 后返回结构化结果。
fn real_source_run_returns_structured_result_without_text_parsing() {
    let response = dispatch(ProtocolRequest::Run {
        request_id: "run-success".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: OptimizationConfig::default(),
        source: SourceIdentity {
            module: "main".to_owned(),
            path: Some("main.xiao".to_owned()),
            text: "value = 1 + 2\n".to_owned(),
        },
        options: RunOptions::default(),
    });
    let ProtocolResponse::Result {
        exit_code,
        exit_name,
        metrics,
        ..
    } = response
    else {
        panic!("真实源码应产生结构化结果");
    };
    assert_eq!(exit_code, ExitCode::Success.as_process_code());
    assert_eq!(exit_name, "success");
    assert!(metrics.is_some());
}

#[test]
/// 服务入口先协商版本，再处理真实源码并确认关闭。
fn service_round_trips_hello_run_and_shutdown_frames() {
    let run = ProtocolRequest::Run {
        request_id: "service-run".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: OptimizationConfig::default(),
        source: SourceIdentity {
            module: "main".to_owned(),
            path: None,
            text: "value = 1\n".to_owned(),
        },
        options: RunOptions::default(),
    };
    let shutdown = ProtocolRequest::Shutdown {
        request_id: "service-shutdown".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
    };
    let mut input = encode_frame(&hello()).expect("hello frame");
    input.extend_from_slice(&encode_frame(&run).expect("run frame"));
    input.extend_from_slice(&encode_frame(&shutdown).expect("shutdown frame"));
    let output = SharedBuffer::default();
    let output_view = Arc::clone(&output.0);
    serve(Cursor::new(input), output).expect("service");

    let mut cursor = Cursor::new(output_view.lock().expect("buffer lock").clone());
    let mut responses = Vec::new();
    while let Some(payload) = read_frame(&mut cursor).expect("response frame") {
        responses.push(decode_frame::<ProtocolResponse>(&payload).expect("response"));
    }
    assert!(
        responses
            .iter()
            .any(|response| matches!(response, ProtocolResponse::Hello { accepted: true, .. }))
    );
    assert!(responses.iter().any(|response| matches!(response, ProtocolResponse::Result { request_id, exit_code: 0, .. } if request_id == "service-run")));
    assert!(responses.iter().any(|response| matches!(response, ProtocolResponse::Shutdown { request_id } if request_id == "service-shutdown")));
}

#[test]
/// Build 请求的新字段必须在协议 JSON 中保持可逆，旧字段顺序不参与契约。
fn build_request_round_trips_toolchain_and_config_fields() {
    let request = ProtocolRequest::Build {
        request_id: "build-1".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: OptimizationConfig {
            level: 0,
            debug: true,
            diagnostics: None,
        },
        source: SourceIdentity {
            module: "main".to_owned(),
            path: Some("main.xiao".to_owned()),
            text: "value = 1\n".to_owned(),
        },
        output: "build/main.exe".to_owned(),
        llvm_ir_output: Some("build/main.ll".to_owned()),
        toolchain: ToolchainSpec {
            clang: "clang".to_owned(),
            llvm_as: None,
            llc: None,
            runtime_library: None,
            native_static_libraries: Vec::new(),
            rustc: Some("rustc".to_owned()),
            diagnostics_path: Some("xiao-diagnostics.exe".to_owned()),
            versions: ToolchainVersionsSpec {
                clang: "clang version 22".to_owned(),
                llvm_as: None,
                llc: None,
                rustc: Some("rustc 1.90".to_owned()),
            },
        },
        config_text: Some("[Runtime]\ncall_stack_depth = 64\n".to_owned()),
    };
    let encoded = serde_json::to_vec(&request).expect("编码 build 请求");
    let decoded: ProtocolRequest = serde_json::from_slice(&encoded).expect("解码 build 请求");
    assert_eq!(decoded, request);

    let mut legacy = serde_json::to_value(&request).expect("编码旧 build 形状");
    let object = legacy.as_object_mut().expect("build 请求对象");
    object.remove("config_text");
    let toolchain = object
        .get_mut("toolchain")
        .and_then(Value::as_object_mut)
        .expect("toolchain 对象");
    toolchain.remove("rustc");
    toolchain.remove("diagnostics_path");
    toolchain
        .get_mut("versions")
        .and_then(Value::as_object_mut)
        .expect("versions 对象")
        .remove("rustc");
    let legacy: ProtocolRequest = serde_json::from_value(legacy).expect("旧 build 形状应兼容");
    let ProtocolRequest::Build {
        toolchain,
        config_text,
        ..
    } = legacy
    else {
        panic!("应解码为 build 请求");
    };
    assert_eq!(toolchain.rustc, None);
    assert_eq!(toolchain.diagnostics_path, None);
    assert_eq!(toolchain.versions.rustc, None);
    assert_eq!(config_text, None);
}

#[test]
/// config.xiao 只固化静态树，并明确允许普通配置覆盖。
fn runtime_config_freeze_is_deterministic_and_validated() {
    let frozen = freeze_runtime_config(
        "build/main.exe",
        Some("[Runtime]\ncall_stack_depth = 64\n[debug]\nterminal_level = \"info\"\n"),
    )
    .expect("合法配置应通过")
    .expect("应产生固化配置");
    assert_eq!(frozen.value["format_version"], json!(1));
    assert_eq!(frozen.value["cli_overrides"], json!(true));
    assert_eq!(
        frozen.value["tables"]["runtime"]["call_stack_depth"],
        json!(64)
    );
    assert!(freeze_runtime_config("build/main.exe", Some("if true { value = 1 }")).is_err());
}

#[test]
/// 同一路径重复构建可以替换固化配置，不保留上一次的字段。
fn runtime_config_can_be_replaced() {
    let root = std::env::temp_dir().join(format!("xiao-runtime-config-{}", std::process::id()));
    let executable = root.join("app.exe");
    let _ = fs::remove_dir_all(&root);
    let first = FrozenRuntimeConfig {
        value: json!({"format_version": 1, "cli_overrides": true, "tables": {"runtime": {"call_stack_depth": 64}}}),
    };
    let second = FrozenRuntimeConfig {
        value: json!({"format_version": 1, "cli_overrides": true, "tables": {"runtime": {"call_stack_depth": 128}}}),
    };
    write_runtime_config(&executable, &first).expect("首次写入");
    let summary = write_runtime_config(&executable, &second).expect("重复写入");
    let value: Value =
        serde_json::from_slice(&fs::read(&summary.path).expect("读取配置")).expect("解析配置");
    assert_eq!(value["tables"]["runtime"]["call_stack_depth"], json!(128));
    let _ = fs::remove_dir_all(root);
}

#[test]
/// 调试组件目标始终落在原生可执行文件的同一目录。
fn diagnostics_component_is_staged_next_to_executable() {
    let path = diagnostics_component_path("out/bin/app.exe", "C:/xiao/xiao-diagnostics.exe");
    let normalized = path.replace('\\', "/");
    assert!(
        normalized.ends_with("out/bin/xiao-diagnostics.exe"),
        "unexpected staged path: {path}"
    );
}

#[test]
/// 诊断组件已在产物目录时不会复制自身，跨目录时使用完整提交。
fn diagnostics_component_staging_distinguishes_existing_source() {
    let root = std::env::temp_dir().join(format!("xiao-diagnostics-stage-{}", std::process::id()));
    let source = root.join("source").join("xiao-diagnostics.exe");
    let destination = root.join("output").join("xiao-diagnostics.exe");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(source.parent().expect("source parent")).expect("创建源目录");
    fs::write(&source, b"diagnostics").expect("写入源组件");
    assert!(
        !stage_diagnostics_component(
            source.to_str().expect("source utf8"),
            source.to_str().expect("source utf8")
        )
        .expect("同文件判断")
    );
    assert!(
        stage_diagnostics_component(
            source.to_str().expect("source utf8"),
            destination.to_str().expect("destination utf8")
        )
        .expect("复制组件")
    );
    assert_eq!(fs::read(destination).expect("读取目标组件"), b"diagnostics");
    let _ = fs::remove_dir_all(root);
}

#[test]
/// 调试构建没有诊断组件路径时必须在协议层明确拒绝。
fn debug_build_requires_diagnostics_path() {
    let response = dispatch(ProtocolRequest::Build {
        request_id: "debug-build".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: OptimizationConfig {
            level: 0,
            debug: true,
            diagnostics: None,
        },
        source: SourceIdentity {
            module: "main".to_owned(),
            path: None,
            text: "value = 1\n".to_owned(),
        },
        output: "build/debug-main.exe".to_owned(),
        llvm_ir_output: None,
        toolchain: ToolchainSpec {
            clang: "clang".to_owned(),
            ..ToolchainSpec::default()
        },
        config_text: None,
    });
    let ProtocolResponse::Error { error, .. } = response else {
        panic!("缺少诊断路径应返回协议错误");
    };
    assert_eq!(error.code, BUILD_ERROR_CODE);
    assert!(error.message.contains("xiao-diagnostics"));
}
