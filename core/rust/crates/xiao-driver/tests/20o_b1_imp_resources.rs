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

/// 两种模式各自验证最大数组、独立编码文本、逐字节解码与输入不变。
fn base64_maximum(url_safe: bool) {
    let mut expected = if url_safe { "____" } else { "////" }.repeat(21_845);
    expected.push_str(if url_safe { "_w==" } else { "/w==" });
    assert_eq!(expected.len(), 87_384);
    let expected = serde_json::to_string(&expected).unwrap();
    let source = format!(
        "from std.base64 import encode, decode\nvalues = []\nindex = 0\nwhile index < 65536\n    array_push(values, 255)\n    index = index + 1\nexpected = {expected}\nprint(encode(values, {url_safe}) == expected)\nrestored = decode(expected, {url_safe})\nrestored_count = 0\nfor byte in restored\n    if byte != 255\n        raise Error(code = \"BASE64_MAX_GOLDEN_MISMATCH\")\n    restored_count = restored_count + 1\nprint(restored_count)\nprint(values[65535])\n"
    );
    assert_output(source, "true\n65536\n255\n");
}

#[test]
/// 标准模式的最大合法结果必须保留双补位，不能只检查输入长度。
fn base64_standard_accepts_the_frozen_maximum() {
    base64_maximum(false);
}

#[test]
/// URL-safe 在同样的最大输入仍强制补位，不接受省略或标准字符。
fn base64_url_safe_accepts_the_frozen_maximum() {
    base64_maximum(true);
}

#[test]
/// 相同的合法文本长度分别解码为上限+1/+2；无效尾位先于结果预算报编码错误。
fn base64_checks_actual_decoded_size_after_the_last_quartet() {
    for (suffix, class, expected) in [
        ("AAA=", "ResourceError", "BASE64_LIMIT_EXCEEDED\n"),
        ("AAAA", "ResourceError", "BASE64_LIMIT_EXCEEDED\n"),
        ("AAB=", "Error", "BASE64_INVALID_ENCODING\n"),
    ] {
        let text = serde_json::to_string(&("A".repeat(87_380) + suffix)).unwrap();
        let source = format!(
            "from std.base64 import decode\ntry\n    print(decode({text}, false))\ncatch failure as {class}\n    print(error_code(failure))\n"
        );
        assert_output(source, expected);
    }
}
