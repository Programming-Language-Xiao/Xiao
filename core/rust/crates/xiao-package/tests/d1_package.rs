//! 11A-D1 本地路径包契约与依赖图规格测试。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use xiao_package::{
    PACKAGE_DEPENDENCY_CYCLE_CODE, PACKAGE_IDENTITY_CONFLICT_CODE, PACKAGE_MISSING_DEPENDENCY_CODE,
    resolve_project,
};

/// 为并行测试 fixture 提供进程内唯一的临时目录后缀。
static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);

/// 一个测试用隔离包项目。
struct TempProject {
    path: PathBuf,
}

impl TempProject {
    /// 创建一个不会与其他测试共享的临时包项目根目录。
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("xiao-package-{stamp}-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).expect("create temporary package project");
        Self { path }
    }

    /// 写入项目根下的 UTF-8 配置文件。
    fn write(&self, relative: impl AsRef<Path>, text: &str) {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create package fixture directory");
        }
        fs::write(path, text).expect("write package fixture");
    }
}

impl Drop for TempProject {
    /// 测试结束后递归删除 fixture 目录。
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// 一条包图规格快照用例。
#[derive(Debug, Deserialize)]
struct SnapshotCase {
    name: String,
    files: BTreeMap<String, String>,
    expect: String,
    #[serde(default)]
    nodes: Vec<String>,
    #[serde(default)]
    resolution_order: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
}

/// 一份包图规格快照文件。
#[derive(Debug, Deserialize)]
struct SnapshotFile {
    stage: String,
    status: String,
    cases: Vec<SnapshotCase>,
}

/// 读取并执行一份包图规格快照。
fn assert_snapshot(raw: &str) {
    let snapshot: SnapshotFile = serde_json::from_str(raw).expect("包规格 JSON 必须有效");
    assert_eq!(snapshot.stage, "11A-D1");
    assert_eq!(snapshot.status, "verified-static");
    assert!(!snapshot.cases.is_empty(), "包规格不应为空");

    for case in snapshot.cases {
        let project = TempProject::new();
        for (path, source) in case.files {
            project.write(path, &source);
        }
        let result = resolve_project(&project.path);
        let actual_diagnostics = result
            .diagnostics
            .iter()
            .map(|entry| entry.diagnostic.code().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            actual_diagnostics, case.diagnostics,
            "规格用例: {}",
            case.name
        );
        assert_eq!(
            result.is_success(),
            case.expect == "success",
            "规格用例: {}",
            case.name
        );

        if case.expect == "success" {
            let actual_nodes = result
                .graph
                .nodes
                .keys()
                .map(|identity| format!("{}@{}", identity.name, identity.version))
                .collect::<Vec<_>>();
            assert_eq!(actual_nodes, case.nodes, "规格用例: {}", case.name);
            let actual_order = result
                .graph
                .resolution_order
                .iter()
                .map(|identity| format!("{}@{}", identity.name, identity.version))
                .collect::<Vec<_>>();
            assert_eq!(
                actual_order, case.resolution_order,
                "规格用例: {}",
                case.name
            );
            for identity in result.graph.nodes.keys() {
                assert!(identity.source.source_id.starts_with("path:"));
                assert!(identity.source.alias.is_none());
                assert!(!identity.source.display_name.is_empty());
            }
        }
    }
}

#[test]
/// 本地路径包正例和反例必须由真实解析器入口读取。
fn package_spec_snapshots_are_executed() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11a-package/valid.json"
    )));
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11a-package/errors.json"
    )));
}

#[test]
/// 包依赖应保留版本和来源约束而不把展示名当作来源身份。
fn preserves_dependency_constraints_and_source_shape() {
    let project = TempProject::new();
    project.write(
        "config.xiao",
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies]\ncore = { path = \"core\", version = \"^1.4\", source = \"local\" }\n",
    );
    project.write(
        "core/config.xiao",
        "[project]\nname = \"core\"\nversion = \"1.4.2\"\n",
    );

    let result = resolve_project(&project.path);
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    let root = result.graph.root.expect("root package");
    let dependency = result.graph.nodes[&root]
        .dependencies
        .get("core")
        .expect("core dependency");
    assert_eq!(dependency.version.as_deref(), Some("^1.4"));
    assert_eq!(dependency.source.as_deref(), Some("local"));
    assert_eq!(
        dependency
            .target
            .as_ref()
            .map(|target| target.name.as_str()),
        Some("core")
    );
    assert_eq!(root.source.alias, None);
    assert_ne!(root.source.source_id, root.source.display_name);
}

#[test]
/// D1 的三个包粒度错误必须使用稳定编号。
fn exposes_stable_package_diagnostic_codes() {
    assert_eq!(PACKAGE_IDENTITY_CONFLICT_CODE, "X05-PACKAGE-001");
    assert_eq!(PACKAGE_MISSING_DEPENDENCY_CODE, "X05-PACKAGE-002");
    assert_eq!(PACKAGE_DEPENDENCY_CYCLE_CODE, "X05-PACKAGE-003");
}
