//! 11 `config.xiao` 声明式子集的 JSON 规格入口。

use serde::Deserialize;
use xiao_config::parse_config_project;
use xiao_source::SourceFile;

/// 一条配置规格快照用例。
#[derive(Debug, Deserialize)]
struct SnapshotCase {
    /// 用例名称，用于失败定位。
    name: String,
    /// 配置源码。
    source: String,
    /// 期望状态。
    expect: String,
    /// 期望的稳定诊断编号。
    #[serde(default)]
    diagnostics: Vec<String>,
}

/// 一份配置规格快照。
#[derive(Debug, Deserialize)]
struct SnapshotFile {
    /// 阶段标识。
    stage: String,
    /// 快照状态。
    status: String,
    /// 用例列表。
    cases: Vec<SnapshotCase>,
}

/// 读取并执行一份配置规格快照。
fn assert_snapshot(raw: &str) {
    let snapshot: SnapshotFile = serde_json::from_str(raw).expect("配置快照 JSON 必须有效");
    assert_eq!(snapshot.stage, "11");
    assert_eq!(snapshot.status, "verified-static");
    assert!(!snapshot.cases.is_empty(), "配置快照不应为空");

    for case in snapshot.cases {
        let result = parse_config_project(&SourceFile::from_text(&case.source));
        let actual = result
            .as_ref()
            .err()
            .into_iter()
            .flatten()
            .map(|diagnostic| diagnostic.code().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(actual, case.diagnostics, "快照用例: {}", case.name);
        assert_eq!(
            result.is_ok(),
            case.expect == "success",
            "快照用例: {}",
            case.name
        );
    }
}

#[test]
/// 配置正例和反例必须由真实 `xiao-config` 入口读取。
fn config_spec_snapshots_are_executed() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11-config/valid.json"
    )));
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11-config/errors.json"
    )));
}
