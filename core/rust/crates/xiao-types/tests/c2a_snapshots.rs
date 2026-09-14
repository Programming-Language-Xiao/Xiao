//! C2-A 集合规格快照测试。
//!
//! 快照只固定静态检查的机器契约：阶段、成功/失败状态、诊断编号、消息键和
//! 语言无关参数。展示文本故意不写入快照，避免国际化目录变更破坏语义测试。

use std::collections::BTreeMap;

use serde::Deserialize;
use xiao_diagnostics::{DiagnosticParam, Severity};
use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::TypeChecker;

/// 一条 C2-A 快照用例。
#[derive(Debug, Deserialize)]
struct SnapshotCase {
    /// 用例名称，用于失败定位。
    name: String,
    /// 要解析和检查的 Xiao 源码。
    source: String,
    /// 期望状态：`success` 或 `error`。
    expect: String,
    /// 按产生顺序排列的诊断期望。
    #[serde(default)]
    diagnostics: Vec<DiagnosticExpectation>,
}

/// 一份 C2-A 集合快照文件。
#[derive(Debug, Deserialize)]
struct SnapshotFile {
    /// 阶段标识。
    stage: String,
    /// 快照状态。
    status: String,
    /// 用例列表。
    cases: Vec<SnapshotCase>,
}

/// 一条不依赖展示语言的诊断期望。
#[derive(Debug, Deserialize)]
struct DiagnosticExpectation {
    /// 稳定机器诊断编号。
    code: String,
    /// 稳定消息目录键。
    message_id: String,
    /// 消息插值参数；缺省为空。
    #[serde(default)]
    params: BTreeMap<String, serde_json::Value>,
}

/// 读取并执行一份 C2-A 类型快照。
fn assert_snapshot(raw: &str) {
    let snapshot: SnapshotFile = serde_json::from_str(raw).expect("C2-A 快照 JSON 必须有效");
    assert_eq!(snapshot.stage, "03C");
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
        let program = parsed.program.expect("快照应始终产生程序 AST");
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

/// 比较诊断的稳定身份、严重级别和结构化参数。
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
        assert_eq!(actual.severity(), Severity::Error, "快照诊断级别不符");
        assert_eq!(
            actual.code(),
            expected.code,
            "快照用例 {case_name} 的第 {index} 条编号不符"
        );
        assert_eq!(
            actual.message_id(),
            expected.message_id,
            "快照用例 {case_name} 的第 {index} 条消息键不符"
        );
        assert_snapshot_params(case_name, index, actual.params(), &expected.params);
    }
}

/// 比较参数键集合，并逐项验证参数的机器类型和值。
fn assert_snapshot_params(
    case_name: &str,
    index: usize,
    actual: &BTreeMap<String, DiagnosticParam>,
    expected: &BTreeMap<String, serde_json::Value>,
) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "快照用例 {case_name} 的第 {index} 条参数数量不符"
    );
    for (name, expected_value) in expected {
        let actual_value = actual
            .get(name)
            .unwrap_or_else(|| panic!("快照用例 {case_name} 缺少参数 {name}"));
        match (actual_value, expected_value) {
            (DiagnosticParam::Boolean(actual), serde_json::Value::Bool(expected)) => {
                assert_eq!(actual, expected, "参数 {name} 值不符");
            }
            (DiagnosticParam::Integer(actual), serde_json::Value::Number(expected)) => {
                let expected = expected
                    .to_string()
                    .parse::<i128>()
                    .expect("快照整数参数必须能表示为 i128");
                assert_eq!(actual, &expected, "参数 {name} 值不符");
            }
            (DiagnosticParam::Text(actual), serde_json::Value::String(expected)) => {
                assert_eq!(actual, expected, "参数 {name} 值不符");
            }
            (actual, expected) => panic!(
                "快照用例 {case_name} 的参数 {name} 类型不符：实际 {actual:?}，期望 {expected:?}"
            ),
        }
    }
}

#[test]
/// 合法集合快照固定空字典/空集合、标量类型、显式类型和成员判断边界。
fn valid_snapshot_is_stable() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/05-containers/c2a-valid.json"
    )));
}

#[test]
/// 错误集合快照固定类型冲突、哈希、重复、构造器、成员和索引诊断。
fn error_snapshot_is_stable() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/05-containers/c2a-errors.json"
    )));
}
