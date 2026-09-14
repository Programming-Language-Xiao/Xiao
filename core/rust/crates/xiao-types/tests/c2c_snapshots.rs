//! C2-C 集合运算快照测试。
//!
//! 快照只比较阶段状态、稳定诊断身份和结构化参数，不比较本地化展示文本。

use std::collections::BTreeMap;

use serde::Deserialize;
use xiao_diagnostics::{DiagnosticParam, Severity};
use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::TypeChecker;

/// 一条 C2-C 快照用例。
#[derive(Debug, Deserialize)]
struct SnapshotCase {
    /// 用例名称。
    name: String,
    /// Xiao 源码。
    source: String,
    /// 期望状态：`success` 或 `error`。
    expect: String,
    /// 稳定诊断期望。
    #[serde(default)]
    diagnostics: Vec<DiagnosticExpectation>,
}

/// 一份 C2-C 快照文件。
#[derive(Debug, Deserialize)]
struct SnapshotFile {
    /// 阶段标识。
    stage: String,
    /// 快照状态。
    status: String,
    /// 用例列表。
    cases: Vec<SnapshotCase>,
}

/// 一条诊断的稳定字段期望。
#[derive(Debug, Deserialize)]
struct DiagnosticExpectation {
    /// 稳定编号。
    code: String,
    /// 消息目录键。
    message_id: String,
    /// 结构化参数。
    #[serde(default)]
    params: BTreeMap<String, serde_json::Value>,
}

/// 读取并执行 C2-C 快照。
fn assert_snapshot(raw: &str) {
    let snapshot: SnapshotFile = serde_json::from_str(raw).expect("C2-C 快照 JSON 必须有效");
    assert_eq!(snapshot.stage, "03E");
    assert_eq!(snapshot.status, "verified-static");
    for case in snapshot.cases {
        let source = SourceFile::from_text(&case.source);
        let parsed = Parser::new(&source).parse();
        assert!(
            parsed.diagnostics.is_empty(),
            "快照用例 {} 不应包含解析诊断：{:?}",
            case.name,
            parsed.diagnostics
        );
        let program = parsed.program.expect("快照应产生程序 AST");
        let result = TypeChecker::check(&source, &program);
        assert_eq!(
            result.has_errors(),
            case.expect == "error",
            "快照用例 {} 的成功/失败状态不符",
            case.name
        );
        assert_diagnostics(&case.name, result.diagnostics(), &case.diagnostics);
    }
}

/// 比较稳定诊断身份、级别和结构化参数。
fn assert_diagnostics(
    case_name: &str,
    actual: &[xiao_diagnostics::Diagnostic],
    expected: &[DiagnosticExpectation],
) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "快照用例 {case_name} 的诊断数量不符"
    );
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(actual.severity(), Severity::Error);
        assert_eq!(
            actual.code(),
            expected.code,
            "快照用例 {case_name} 第 {index} 条编号不符"
        );
        assert_eq!(
            actual.message_id(),
            expected.message_id,
            "快照用例 {case_name} 第 {index} 条消息键不符"
        );
        assert_params(case_name, index, actual.params(), &expected.params);
    }
}

/// 逐项验证参数的机器类型和值。
fn assert_params(
    case_name: &str,
    index: usize,
    actual: &BTreeMap<String, DiagnosticParam>,
    expected: &BTreeMap<String, serde_json::Value>,
) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "快照用例 {case_name} 第 {index} 条参数数量不符"
    );
    for (name, expected_value) in expected {
        let actual_value = actual
            .get(name)
            .unwrap_or_else(|| panic!("快照用例 {case_name} 缺少参数 {name}"));
        match (actual_value, expected_value) {
            (DiagnosticParam::Boolean(actual), serde_json::Value::Bool(expected)) => {
                assert_eq!(actual, expected);
            }
            (DiagnosticParam::Integer(actual), serde_json::Value::Number(expected)) => {
                assert_eq!(
                    actual,
                    &expected
                        .to_string()
                        .parse::<i128>()
                        .expect("快照整数参数应可表示为 i128")
                );
            }
            (DiagnosticParam::Text(actual), serde_json::Value::String(expected)) => {
                assert_eq!(actual, expected);
            }
            (actual, expected) => panic!(
                "快照用例 {case_name} 第 {index} 条参数 {name} 类型不符：{actual:?} / {expected:?}"
            ),
        }
    }
}

#[test]
/// 合法快照覆盖四种集合运算、空交集、比较和原地形式。
fn valid_snapshot_is_stable() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/05-containers/c2c-valid.json"
    )));
}

#[test]
/// 错误快照覆盖集合/标量混用和原地结果类型冲突。
fn error_snapshot_is_stable() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/05-containers/c2c-errors.json"
    )));
}
