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
/// 两端进制边界与左右填充均按参数域先报错，无需填充也不能跳过 fill 校验。
fn fmt_checks_both_radix_bounds_and_padding_argument_domains() {
    assert_output(
        "from std.fmt import format_int, pad_left, pad_right\nfor radix in [1, 37]\n    try\n        print(format_int(0, radix))\n    catch invalid_radix as Error\n        print(error_code(invalid_radix))\ntry\n    print(pad_left(\"x\", -1, \"\"))\ncatch left_width as Error\n    print(error_code(left_width))\ntry\n    print(pad_right(\"x\", -1, \"\"))\ncatch right_width as Error\n    print(error_code(right_width))\ntry\n    print(pad_left(\"x\", 0, \"\"))\ncatch left_fill as Error\n    print(error_code(left_fill))\ntry\n    print(pad_right(\"x\", 0, \"ab\"))\ncatch right_fill as Error\n    print(error_code(right_fill))\n".to_owned(),
        "FMT_INVALID_RADIX\nFMT_INVALID_RADIX\nFMT_INVALID_WIDTH\nFMT_INVALID_WIDTH\nFMT_INVALID_FILL\nFMT_INVALID_FILL\n",
    );
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

/// 一个最大输入调用与独立文字黄金；参数和打印不用于 F10 计时。
fn url_vector(function: &str, input: &str, expected: &str) {
    let input = serde_json::to_string(input).unwrap();
    let expected = serde_json::to_string(expected).unwrap();
    assert_output(
        format!("from std.url import {function}\nprint({function}({input}) == {expected})\n"),
        "true\n",
    );
}

#[test]
/// 保留字符外的全部 ASCII 触发三倍膨胀，实际结果恰好 196,608 字节。
fn url_encoding_reaches_the_ascii_output_budget() {
    url_vector(
        "encode_component",
        &" ".repeat(65_536),
        &"%20".repeat(65_536),
    );
}

#[test]
/// 四字节码点的最大输入同样达到编码输出上限。
fn url_encoding_reaches_the_unicode_output_budget() {
    url_vector(
        "encode_component",
        &"🚀".repeat(16_384),
        &"%F0%9F%9A%80".repeat(16_384),
    );
}

#[test]
/// 解码最大转义文本，返回恰好 65,536 字节 ASCII。
fn url_decoding_reaches_the_ascii_result_budget() {
    url_vector(
        "decode_component",
        &"%20".repeat(65_536),
        &" ".repeat(65_536),
    );
}

#[test]
/// 多码点状态必须在每个四字节序列之间复位，不能丢失边界。
fn url_decoding_reaches_the_unicode_result_budget() {
    url_vector(
        "decode_component",
        &"%F0%9F%9A%80".repeat(16_384),
        &"🚀".repeat(16_384),
    );
}

#[test]
/// 有效码点达到结果上限+1 即失败；之后的坏转义不覆盖首错，无效当前码点则先报 UTF-8。
fn url_limits_and_mixed_error_order_are_streamed() {
    for (text, class, code) in [
        (
            "x".repeat(65_537) + "%G0",
            "ResourceError",
            "URL_LIMIT_EXCEEDED",
        ),
        (
            "x".repeat(65_536) + "%41",
            "ResourceError",
            "URL_LIMIT_EXCEEDED",
        ),
        ("x".repeat(65_536) + "%FF", "Error", "URL_INVALID_UTF8"),
    ] {
        let input = serde_json::to_string(&text).unwrap();
        assert_output(
            format!(
                "from std.url import decode_component\ntry\n    print(decode_component({input}))\ncatch failure as {class}\n    print(error_code(failure))\n"
            ),
            &format!("{code}\n"),
        );
    }
    let input = serde_json::to_string(&("🚀".repeat(16_384) + "x")).unwrap();
    assert_output(
        format!(
            "from std.url import encode_component\ntry\n    print(encode_component({input}))\ncatch failure as ResourceError\n    print(error_code(failure))\n"
        ),
        "URL_LIMIT_EXCEEDED\n",
    );
}

/// 独立记录文本分别验证格式化与解析，不用产品往返产生黄金。
fn csv_vector(fields: &[String], record: &str) {
    let encoded_fields = serde_json::to_string(fields).unwrap();
    let encoded_record = serde_json::to_string(record).unwrap();
    let source = format!(
        "from std.csv import parse_record, format_record\nfields = {encoded_fields}\nexpected = {encoded_record}\nprint(format_record(fields) == expected)\nrestored = parse_record(expected)\nindex = 0\nfor field in restored\n    if field != fields[index]\n        raise Error(code = \"CSV_RESOURCE_GOLDEN_MISMATCH\")\n    index = index + 1\nprint(index)\n"
    );
    assert_output(source, &format!("true\n{}\n", fields.len()));
}

#[test]
/// 四字段的 E 恰好 65,536，单字段恰好 16,384 字节。
fn csv_accepts_the_maximum_ascii_record() {
    let fields = vec![
        "x".repeat(16_384),
        "x".repeat(16_384),
        "x".repeat(16_384),
        "x".repeat(16_381),
    ];
    let record = fields.join(",");
    assert_eq!(record.len(), 65_536);
    csv_vector(&fields, &record);
}

#[test]
/// 全部数据引号被双写，尾空字段保留，真实状态机扫描完整 64 KiB。
fn csv_accepts_the_maximum_quoted_record() {
    let fields = vec!["\"".repeat(16_384), "\"".repeat(16_381), String::new()];
    let record = format!(
        "\"{}\",\"{}\",",
        "\"\"".repeat(16_384),
        "\"\"".repeat(16_381)
    );
    assert_eq!(record.len(), 65_536);
    csv_vector(&fields, &record);
}

#[test]
/// 换行本身不双写，但四个字段都必须有外引号，E 达到等号。
fn csv_accepts_the_maximum_multiline_record() {
    let fields = vec![
        "\n".repeat(16_384),
        "\n".repeat(16_384),
        "\n".repeat(16_384),
        "\n".repeat(16_373),
    ];
    let record = fields
        .iter()
        .map(|field| format!("\"{field}\""))
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(record.len(), 65_536);
    csv_vector(&fields, &record);
}

#[test]
/// 字节限额与码点数量不同，四字节字段不能被按字符数放宽。
fn csv_accepts_the_maximum_unicode_record() {
    let fields = vec![
        "🚀".repeat(4_096),
        "🚀".repeat(4_096),
        "🚀".repeat(4_096),
        "🚀".repeat(4_095) + "a",
    ];
    let record = fields.join(",");
    assert_eq!(record.len(), 65_536);
    csv_vector(&fields, &record);
}

#[test]
/// 字段数等于 1,024 合法；空字段既不能丢弃也不能额外加引号。
fn csv_accepts_the_maximum_field_count() {
    csv_vector(&vec![String::new(); 1_024], &",".repeat(1_023));
}

#[test]
/// S 只是预检；E 等号+1、全引号膨胀、字段/总量/数量超限都稳定拒绝。
fn csv_rejects_every_format_budget_dimension() {
    let cases = [
        vec![
            "x".repeat(16_384),
            "x".repeat(16_384),
            "x".repeat(16_384),
            "x".repeat(16_382),
        ],
        vec!["\"".repeat(16_384), "\"".repeat(16_381), "x".to_owned()],
        vec!["\"".repeat(16_384); 4],
        vec!["\n".repeat(16_384); 4],
        vec![
            "x".repeat(16_384),
            "x".repeat(16_384),
            "x".repeat(16_384),
            "x".repeat(16_384),
            "x".to_owned(),
        ],
        vec!["🚀".repeat(4_096) + "x"],
        vec![String::new(); 1_025],
    ];
    for fields in cases {
        let input = serde_json::to_string(&fields).unwrap();
        assert_output(
            format!(
                "from std.csv import format_record\ntry\n    print(format_record({input}))\ncatch failure as ResourceError\n    print(error_code(failure))\n"
            ),
            "CSV_LIMIT_EXCEEDED\n",
        );
    }
}

#[test]
/// 解析按状态机位置决定语法/预算首错，不因未来的错误回头改写身份。
fn csv_parse_budget_and_syntax_priority_is_positional() {
    for (record, class, code) in [
        (
            ",".repeat(1_024) + "\"",
            "ResourceError",
            "CSV_LIMIT_EXCEEDED",
        ),
        (
            "a\"".to_owned() + &",".repeat(1_024),
            "Error",
            "CSV_INVALID_RECORD",
        ),
        (
            "\"".to_owned() + &"\"\"".repeat(16_385),
            "ResourceError",
            "CSV_LIMIT_EXCEEDED",
        ),
        (
            "\"".to_owned() + &"x".repeat(65_536),
            "ResourceError",
            "CSV_LIMIT_EXCEEDED",
        ),
        (
            "🚀".repeat(4_096) + "x",
            "ResourceError",
            "CSV_LIMIT_EXCEEDED",
        ),
    ] {
        let input = serde_json::to_string(&record).unwrap();
        assert_output(
            format!(
                "from std.csv import parse_record\ntry\n    print(parse_record({input}))\ncatch failure as {class}\n    print(error_code(failure))\n"
            ),
            &format!("{code}\n"),
        );
    }
}
