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
