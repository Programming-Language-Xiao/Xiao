//! 07 错误控制流静态边界的 JSON 规格入口。

use serde::Deserialize;
use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::TypeChecker;

/// 一条错误控制流规格快照用例。
#[derive(Debug, Deserialize)]
struct SnapshotCase {
    /// 用例名称，用于失败定位。
    name: String,
    /// Xiao 源码。
    source: String,
    /// 期望状态。
    expect: String,
    /// 期望稳定诊断编号。
    #[serde(default)]
    diagnostics: Vec<String>,
}

/// 一份错误控制流规格快照。
#[derive(Debug, Deserialize)]
struct SnapshotFile {
    /// 阶段标识。
    stage: String,
    /// 快照状态。
    status: String,
    /// 用例列表。
    cases: Vec<SnapshotCase>,
}

/// 执行一个前端和类型阶段共享的规格输入。
fn diagnostics_for(source_text: &str) -> Vec<String> {
    let source = SourceFile::from_text(source_text);
    let parsed = Parser::new(&source).parse();
    if !parsed.diagnostics.is_empty() {
        return parsed
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_owned())
            .collect();
    }
    TypeChecker::check(&source, parsed.program.as_ref().expect("程序"))
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code().to_owned())
        .collect()
}

/// 读取并执行一份错误控制流规格快照。
fn assert_snapshot(raw: &str) {
    let snapshot: SnapshotFile = serde_json::from_str(raw).expect("错误控制流快照 JSON 必须有效");
    assert_eq!(snapshot.stage, "07");
    assert_eq!(snapshot.status, "verified-static");
    assert!(!snapshot.cases.is_empty(), "错误控制流快照不应为空");

    for case in snapshot.cases {
        let actual = diagnostics_for(&case.source);
        assert_eq!(actual, case.diagnostics, "快照用例: {}", case.name);
        assert_eq!(
            actual.is_empty(),
            case.expect == "success",
            "快照用例: {}",
            case.name
        );
    }
}

#[test]
/// 07 的语法与静态边界必须由真实前端/类型检查入口读取。
fn error_control_flow_snapshots_are_executed() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/07-error-control/valid.json"
    )));
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/07-error-control/errors.json"
    )));
}
