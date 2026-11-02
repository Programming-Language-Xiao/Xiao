//! B1-IMP 最大合法资源向量；使用实际公开模块，独立黄金，不将总耗时当作 F10。

use std::time::Duration;

use xiao_driver::{DriverRequest, FrontendRequest, FrontendVmDriver};
use xiao_vm::VmEvent;

/// 资源用例可包含输入构造及多次调用；单次已编译调用的 F10 预算由性能夹具另测。
fn assert_output(source: String, expected: &str) {
    let result = FrontendVmDriver::new().run(
        &DriverRequest::new(FrontendRequest::from_text(source))
            .with_timeout(Duration::from_secs(60)),
    );
    let actual: String = result
        .as_executed()
        .into_iter()
        .flat_map(|execution| execution.events())
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        result.is_success(),
        "code={:?}; report={:?}; stdout={actual:?}",
        result.code(),
        result.report()
    );
    assert_eq!(actual, expected);
}

#[test]
/// 编码与解码同时命中 65,536 项/131,072 字节的等号边界，解码逐项核对独立全字节模式。
fn hex_accepts_the_frozen_maximum_input_and_output() {
    let all_hex = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff";
    let expected = serde_json::to_string(&all_hex.repeat(256)).unwrap();
    let source = format!(
        "from std.hex import encode, decode\nvalues = []\nindex = 0\nwhile index < 65536\n    array_push(values, index % 256)\n    index = index + 1\nexpected = {expected}\nprint(encode(values) == expected)\nrestored = decode(expected)\nrestored_count = 0\nfor byte in restored\n    if byte != restored_count % 256\n        raise Error(code = \"HEX_MAX_GOLDEN_MISMATCH\")\n    restored_count = restored_count + 1\nprint(restored_count)\nprint(values[65535])\n"
    );
    assert_output(source, "true\n65536\n255\n");
}
