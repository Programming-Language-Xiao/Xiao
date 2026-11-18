//! 真实产品的固定输入和独立黄金；数组黄金逐项检查，不做身份相等比较。

use std::path::Path;

/// 单个公开入口的一种固定数据分布。
pub(super) struct Workload {
    pub(super) id: String,
    pub(super) module: &'static str,
    pub(super) function: &'static str,
    pub(super) signature: &'static str,
    pub(super) arguments: String,
    pub(super) call: String,
    pub(super) result: &'static str,
    pub(super) initial: &'static str,
    pub(super) prepare: String,
    pub(super) check: String,
    pub(super) input_bytes_or_items: usize,
    pub(super) output_bytes_or_items: usize,
}

/// JSON 的引号和本工具使用的非控制 Unicode 与 Xiao 字符串语法一致。
fn literal(text: &str) -> String {
    serde_json::to_string(text).unwrap()
}

/// 独立整型数组模式，输入在窗口外准备。
fn bytes_prepare(size: usize, pattern: &str) -> String {
    format!(
        "prepared = []\nprepare_index = 0\nwhile prepare_index < {size}\n    array_push(prepared, {pattern})\n    prepare_index = prepare_index + 1\n"
    )
}

/// 逐项核对数组，不允许同一错误算法往返掩盖差异。
fn array_check(size: usize, pattern: &str) -> String {
    format!(
        "check_index = 0\nfor check_value in actual\n    if check_value != {pattern}\n        raise Error(code = \"B1_PERF_GOLDEN\")\n    check_index = check_index + 1\nprint(check_index == {size})\n"
    )
}

/// CLI 允许选择单个入口或全部；非 ASCII、两种模式和最坏 CSV 单独登记。
pub(super) const IDS: &[&str] = &[
    "math.abs",
    "math.min",
    "math.max",
    "math.clamp",
    "math.gcd",
    "math.pow",
    "statistics.sum",
    "statistics.min",
    "statistics.max",
    "fmt.format_int",
    "fmt.pad_left",
    "fmt.pad_right",
    "fmt.scan",
    "hex.encode",
    "hex.decode",
    "base64.encode",
    "base64.encode_url",
    "base64.decode",
    "base64.decode_url",
    "url.encode_ascii",
    "url.encode_unicode",
    "url.decode_ascii",
    "url.decode_unicode",
    "csv.parse_ascii",
    "csv.parse_quotes",
    "csv.parse_multiline",
    "csv.parse_fields",
    "csv.format_ascii",
    "csv.format_quotes",
    "csv.format_multiline",
    "csv.format_fields",
];

/// size=None 为最大合法输入；Some(n) 为固定模式的翻倍规模，实际规模写入结果。
pub(super) fn workload(id: &str, size: Option<usize>) -> Workload {
    let mut result = Workload {
        id: id.to_owned(),
        module: "",
        function: "",
        signature: "str value",
        arguments: "prepared".to_owned(),
        call: String::new(),
        result: "str",
        initial: "\"\"",
        prepare: String::new(),
        check: "print(actual == expected)\n".to_owned(),
        input_bytes_or_items: 0,
        output_bytes_or_items: 0,
    };
    let parts: Vec<_> = id.split('.').collect();
    let module = parts[0];
    let variant = parts[1];
    match module {
        "math" => {
            result.module = "math";
            result.result = "int";
            result.initial = "0";
            result.signature = "int left, int right, int upper";
            result.arguments = "left_input, right_input, upper_input".to_owned();
            let (function, left, right, upper, expected, call) = match variant {
                "abs" => (
                    "abs",
                    "-9223372036854775807",
                    "0",
                    "0",
                    "9223372036854775807",
                    "abs(left)",
                ),
                "min" => (
                    "min",
                    "-9223372036854775807 - 1",
                    "9223372036854775807",
                    "0",
                    "-9223372036854775807 - 1",
                    "min(left, right)",
                ),
                "max" => (
                    "max",
                    "-9223372036854775807 - 1",
                    "9223372036854775807",
                    "0",
                    "9223372036854775807",
                    "max(left, right)",
                ),
                "clamp" => (
                    "clamp",
                    "-9223372036854775807 - 1",
                    "0",
                    "9223372036854775807",
                    "0",
                    "clamp(left, right, upper)",
                ),
                "gcd" => (
                    "gcd",
                    "7540113804746346429",
                    "4660046610375530309",
                    "0",
                    "1",
                    "gcd(left, right)",
                ),
                "pow" => (
                    "pow",
                    "-1",
                    "9223372036854775807",
                    "0",
                    "-1",
                    "pow(left, right)",
                ),
                _ => panic!("未知数学入口"),
            };
            result.function = function;
            result.call = call.to_owned();
            result.prepare = format!(
                "left_input = {left}\nright_input = {right}\nupper_input = {upper}\nexpected = {expected}\n"
            );
            result.input_bytes_or_items = 3;
            result.output_bytes_or_items = 1;
        }
        "statistics" => {
            result.module = "statistics";
            result.signature = "[int] value";
            result.result = "int";
            result.initial = "0";
            result.function = match variant {
                "sum" => "sum",
                "min" => "min",
                "max" => "max",
                _ => panic!("未知统计入口"),
            };
            let n = size.unwrap_or(65_536);
            result.call = format!("{}(value)", result.function);
            result.prepare = bytes_prepare(n, "1")
                + &format!("expected = {}\n", if variant == "sum" { n } else { 1 });
            result.input_bytes_or_items = n;
            result.output_bytes_or_items = 1;
        }
        "fmt" if variant == "format_int" => {
            result.module = "fmt";
            result.function = "format_int";
            result.signature = "int value";
            result.call = "format_int(value, 2)".to_owned();
            result.prepare = format!(
                "prepared = -9223372036854775807 - 1\nexpected = {}\n",
                literal(&("-1".to_owned() + &"0".repeat(63)))
            );
            result.input_bytes_or_items = 1;
            result.output_bytes_or_items = 65;
        }
        "fmt" => {
            result.module = "fmt";
            let n = size.unwrap_or(65_536);
            if variant == "scan" {
                result.function = "pad_left";
                result.call = "pad_left(value, 0, \".\")".to_owned();
                let text = "x".repeat(n);
                result.prepare = format!(
                    "prepared = {}\nexpected = {}\n",
                    literal(&text),
                    literal(&text)
                );
                result.input_bytes_or_items = n;
                result.output_bytes_or_items = n;
            } else {
                result.function = if variant == "pad_left" {
                    "pad_left"
                } else {
                    "pad_right"
                };
                let text = "🚀".repeat(n / 4);
                let padding = "😀".repeat(n * 3 / 4);
                let expected = if variant == "pad_left" {
                    format!("{padding}{text}")
                } else {
                    format!("{text}{padding}")
                };
                result.call = format!("{}(value, {n}, \"😀\")", result.function);
                result.prepare = format!(
                    "prepared = {}\nexpected = {}\n",
                    literal(&text),
                    literal(&expected)
                );
                result.input_bytes_or_items = n;
                result.output_bytes_or_items = expected.len();
            }
        }
        "hex" => {
            result.module = "hex";
            if variant == "encode" {
                result.function = "encode";
                result.signature = "[int] value";
                result.call = "encode(value)".to_owned();
                let n = size.unwrap_or(65_536);
                result.prepare =
                    bytes_prepare(n, "255") + &format!("expected = {}\n", literal(&"ff".repeat(n)));
                result.input_bytes_or_items = n;
                result.output_bytes_or_items = n * 2;
            } else {
                result.function = "decode";
                result.call = "decode(value)".to_owned();
                result.result = "[int]";
                result.initial = "[]";
                let n = size.unwrap_or(131_072);
                result.prepare = format!("prepared = {}\n", literal(&"ff".repeat(n / 2)));
                result.check = array_check(n / 2, "255");
                result.input_bytes_or_items = n;
                result.output_bytes_or_items = n / 2;
            }
        }
        "base64" => {
            result.module = "base64";
            let url_safe = variant.ends_with("_url");
            let quartet = if url_safe { "____" } else { "////" };
            if variant.starts_with("encode") {
                result.function = "encode";
                result.signature = "[int] value";
                result.call = format!("encode(value, {url_safe})");
                let n = size.unwrap_or(65_536);
                let mut expected = quartet.repeat(n / 3);
                match n % 3 {
                    1 => expected.push_str(if url_safe { "_w==" } else { "/w==" }),
                    2 => expected.push_str(if url_safe { "__8=" } else { "//8=" }),
                    _ => {}
                }
                result.prepare =
                    bytes_prepare(n, "255") + &format!("expected = {}\n", literal(&expected));
                result.input_bytes_or_items = n;
                result.output_bytes_or_items = expected.len();
            } else {
                result.function = "decode";
                result.call = format!("decode(value, {url_safe})");
                result.result = "[int]";
                result.initial = "[]";
                let (text, count) = if let Some(n) = size {
                    (quartet.repeat(n / 4), n / 4 * 3)
                } else {
                    (
                        quartet.repeat(21_845) + if url_safe { "_w==" } else { "/w==" },
                        65_536,
                    )
                };
                result.prepare = format!("prepared = {}\n", literal(&text));
                result.check = array_check(count, "255");
                result.input_bytes_or_items = text.len();
                result.output_bytes_or_items = count;
            }
        }
        "url" => {
            result.module = "url";
            let unicode = variant.ends_with("unicode");
            let unit = if unicode { "🚀" } else { " " };
            let escaped = if unicode { "%F0%9F%9A%80" } else { "%20" };
            if variant.starts_with("encode") {
                result.function = "encode_component";
                result.call = "encode_component(value)".to_owned();
                let n = size.unwrap_or(65_536);
                let text = unit.repeat(n / unit.len());
                let expected = escaped.repeat(n / unit.len());
                result.prepare = format!(
                    "prepared = {}\nexpected = {}\n",
                    literal(&text),
                    literal(&expected)
                );
                result.input_bytes_or_items = text.len();
                result.output_bytes_or_items = expected.len();
            } else {
                result.function = "decode_component";
                result.call = "decode_component(value)".to_owned();
                let n = size.unwrap_or(196_608);
                let text = escaped.repeat(n / escaped.len()) + &"x".repeat(n % escaped.len());
                let expected = unit.repeat(n / escaped.len()) + &"x".repeat(n % escaped.len());
                result.prepare = format!(
                    "prepared = {}\nexpected = {}\n",
                    literal(&text),
                    literal(&expected)
                );
                result.input_bytes_or_items = text.len();
                result.output_bytes_or_items = expected.len();
            }
        }
        "csv" => {
            result.module = "csv";
            let is_parse = variant.starts_with("parse");
            let suffix = variant.split('_').nth(1).unwrap();
            let (fields, record) = if suffix == "fields" {
                // 每 64 字节引入一个字段；最大记录恰好 65,536 字节/1,024 字段。
                let count = size.unwrap_or(65_536) / 64;
                let mut fields = vec!["x".repeat(63); count];
                fields[0].push('x');
                let record = fields.join(",");
                (fields, record)
            } else if let Some(n) = size {
                let content = if suffix == "quotes" {
                    "\""
                } else if suffix == "multiline" {
                    "\n"
                } else {
                    "x"
                };
                let raw_size = if is_parse && suffix == "quotes" {
                    (n - 2) / 2
                } else if is_parse && suffix == "multiline" {
                    n - 2
                } else {
                    n
                };
                let lengths: Vec<_> = if raw_size <= 16_384 {
                    vec![raw_size]
                } else {
                    vec![16_384, raw_size - 16_384]
                };
                let fields: Vec<_> = lengths
                    .iter()
                    .map(|&length| content.repeat(length))
                    .collect();
                let record = if suffix == "ascii" {
                    fields.join(",")
                } else {
                    fields
                        .iter()
                        .map(|field| format!("\"{}\"", field.replace('"', "\"\"")))
                        .collect::<Vec<_>>()
                        .join(",")
                };
                (fields, record)
            } else if suffix == "quotes" {
                (
                    vec!["\"".repeat(16_384), "\"".repeat(16_381), String::new()],
                    format!(
                        "\"{}\",\"{}\",",
                        "\"\"".repeat(16_384),
                        "\"\"".repeat(16_381)
                    ),
                )
            } else if suffix == "multiline" {
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
                (fields, record)
            } else {
                let fields = vec![
                    "x".repeat(16_384),
                    "x".repeat(16_384),
                    "x".repeat(16_384),
                    "x".repeat(16_381),
                ];
                let record = fields.join(",");
                (fields, record)
            };
            if is_parse {
                result.function = "parse_record";
                result.call = "parse_record(value)".to_owned();
                result.result = "[str]";
                result.initial = "[]";
                result.prepare = format!(
                    "prepared = {}\nexpected_fields = {}\n",
                    literal(&record),
                    serde_json::to_string(&fields).unwrap()
                );
                result.check = array_check(fields.len(), "expected_fields[check_index]");
                result.input_bytes_or_items = record.len();
                result.output_bytes_or_items = fields.len();
            } else {
                result.function = "format_record";
                result.signature = "[str] value";
                result.call = "format_record(value)".to_owned();
                result.prepare = format!(
                    "prepared = {}\nexpected = {}\n",
                    serde_json::to_string(&fields).unwrap(),
                    literal(&record)
                );
                result.input_bytes_or_items = fields.iter().map(String::len).sum();
                result.output_bytes_or_items = record.len();
            }
        }
        _ => panic!("未知入口 {id}"),
    }
    result
}

/// VM 使用未改动的整个产品源码，原生使用真实标准库导入；只加测试 wrapper 与验证。
pub(super) fn source(
    root: &Path,
    case: &Workload,
    repeats: usize,
    samples: usize,
    native: bool,
) -> String {
    let product = if native {
        format!("from std.{} import {}\n", case.module, case.function)
    } else {
        std::fs::read_to_string(root.join(format!("stdlib/{}.xiao", case.module))).unwrap()
    };
    let check = case
        .check
        .lines()
        .map(|line| format!("    {line}\n"))
        .collect::<String>();
    let configuration = if native {
        "batch_repeats = 0\nfor config_digit in input(\"\")\n    config_point = int(config_digit)\n    if config_point < 48 or config_point > 57\n        raise Error(code = \"B1_PERF_CONFIGURATION\")\n    batch_repeats = batch_repeats * 10 + config_point - 48\nsample_limit = 0\nfor sample_digit in input(\"\")\n    sample_point = int(sample_digit)\n    if sample_point < 48 or sample_point > 57\n        raise Error(code = \"B1_PERF_CONFIGURATION\")\n    sample_limit = sample_limit * 10 + sample_point - 48\n".to_owned()
    } else {
        format!("batch_repeats = {repeats}\nsample_limit = {samples}\n")
    };
    let signature = format!("{}, int batch_repeats", case.signature);
    let arguments = format!("{}, batch_repeats", case.arguments);
    format!(
        "{product}\ndef __b1_measure({signature}) -> {}\n    actual = {}\n    iteration = 0\n    print(0)\n    while iteration < batch_repeats\n        actual = {}\n        iteration = iteration + 1\n    print(1)\n    return actual\n{}{configuration}\nsample = 0\nwhile sample < sample_limit\n    actual = __b1_measure({arguments})\n{check}    sample = sample + 1\n",
        case.result, case.initial, case.call, case.prepare
    )
}
