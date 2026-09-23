//! X0-A Rust 侧共享协议夹具与双向帧契约。

use xiao_driver::protocol::{
    CORE_VERSION, FRAME_LENGTH_BYTES, PROTOCOL_VERSION, ProtocolRequest, ProtocolResponse,
    decode_frame, dispatch, encode_frame,
};

/// Rust 与 TypeScript 共用的 hello 请求样本。
const HELLO_FIXTURE: &str =
    include_str!("../../../../../tests/spec/11x0-protocol/hello-request.json");
/// Rust 与 TypeScript 共用的运行响应样本。
const RESPONSE_FIXTURE: &str =
    include_str!("../../../../../tests/spec/11x0-protocol/run-response.json");
/// Rust 与 TypeScript 共用的 debug 运行请求样本。
const DEBUG_REQUEST_FIXTURE: &str =
    include_str!("../../../../../tests/spec/11x0-protocol/debug-run-request.json");
/// Rust 与 TypeScript 共用的项目测试请求样本。
const TEST_REQUEST_FIXTURE: &str =
    include_str!("../../../../../tests/spec/11x0-protocol/test-request.json");
/// Rust 与 TypeScript 共用的项目测试响应样本。
const TEST_RESPONSE_FIXTURE: &str =
    include_str!("../../../../../tests/spec/11x0-protocol/test-response.json");

#[test]
/// Rust 能读取 TypeScript 共用的请求样本并生成同一帧格式。
fn shared_hello_fixture_round_trips() {
    let request: ProtocolRequest = serde_json::from_str(HELLO_FIXTURE).expect("hello fixture");
    let frame = encode_frame(&request).expect("encode");
    assert_eq!(
        u64::from_be_bytes(frame[..FRAME_LENGTH_BYTES].try_into().unwrap()) as usize,
        frame.len() - FRAME_LENGTH_BYTES
    );
    let decoded: ProtocolRequest = decode_frame(&frame[FRAME_LENGTH_BYTES..]).expect("decode");
    assert_eq!(decoded, request);
    let ProtocolResponse::Hello { accepted, .. } = dispatch(request) else {
        panic!("hello must produce hello response");
    };
    assert!(accepted);
}

#[test]
/// Rust 能读取 TypeScript 生成形状的结果样本并保留机器字段。
fn shared_response_fixture_round_trips() {
    let response: ProtocolResponse =
        serde_json::from_str(RESPONSE_FIXTURE).expect("response fixture");
    let frame = encode_frame(&response).expect("encode");
    let decoded: ProtocolResponse = decode_frame(&frame[FRAME_LENGTH_BYTES..]).expect("decode");
    assert_eq!(decoded, response);
}

#[test]
/// 共享样本使用冻结的协议与统一核心版本。
fn fixtures_use_frozen_versions() {
    let request: serde_json::Value = serde_json::from_str(HELLO_FIXTURE).expect("hello fixture");
    assert_eq!(request["protocol_version"], PROTOCOL_VERSION);
    assert_eq!(request["core_version"], CORE_VERSION);
}

#[test]
/// debug 请求的诊断配置在 Rust 侧按结构化字段保留。
fn debug_fixture_round_trips_without_text_parsing() {
    let request: ProtocolRequest =
        serde_json::from_str(DEBUG_REQUEST_FIXTURE).expect("debug request fixture");
    let ProtocolRequest::Run { optimization, .. } = &request else {
        panic!("fixture must be a run request");
    };
    assert!(optimization.debug);
    let config = optimization
        .diagnostics
        .as_ref()
        .expect("diagnostic config");
    assert_eq!(config.file_level.as_deref(), Some("debug"));
    assert_eq!(config.focus.len(), 1);
    assert!(!config.focus[0].mirror);
    let frame = encode_frame(&request).expect("encode");
    let decoded: ProtocolRequest = decode_frame(&frame[FRAME_LENGTH_BYTES..]).expect("decode");
    assert_eq!(decoded, request);
}

#[test]
/// 项目测试请求和聚合响应在 Rust 侧保持共享夹具形状与顺序。
fn test_fixtures_round_trip_without_text_parsing() {
    let request: ProtocolRequest =
        serde_json::from_str(TEST_REQUEST_FIXTURE).expect("test request fixture");
    let ProtocolRequest::Test { cases, options, .. } = &request else {
        panic!("fixture must be a test request");
    };
    assert_eq!(cases.len(), 2);
    assert_eq!(cases[0].path.as_deref(), Some("tests/nested/a-first.xiao"));
    assert_eq!(cases[1].path.as_deref(), Some("tests/z-last.xiao"));
    assert_eq!(options.timeout_ms, Some(5000));
    let frame = encode_frame(&request).expect("encode test request");
    let decoded: ProtocolRequest = decode_frame(&frame[FRAME_LENGTH_BYTES..]).expect("decode");
    assert_eq!(decoded, request);

    let response: ProtocolResponse =
        serde_json::from_str(TEST_RESPONSE_FIXTURE).expect("test response fixture");
    let ProtocolResponse::TestResult {
        total,
        passed,
        failed,
        tests,
        ..
    } = &response
    else {
        panic!("fixture must be a test result");
    };
    assert_eq!((*total, *passed, *failed), (2, 1, 1));
    assert_eq!(tests[0].path, "tests/nested/a-first.xiao");
    let response_frame = encode_frame(&response).expect("encode test response");
    let decoded_response: ProtocolResponse =
        decode_frame(&response_frame[FRAME_LENGTH_BYTES..]).expect("decode response");
    assert_eq!(decoded_response, response);
}
