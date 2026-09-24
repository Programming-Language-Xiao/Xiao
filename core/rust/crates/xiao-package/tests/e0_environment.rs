//! 11A-E0 环境布局、元数据和重复创建规格测试。

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use xiao_codegen_llvm::{TargetDescription, Toolchain, ToolchainVersions};
use xiao_config::parse_config_project;
use xiao_package::{
    ENVIRONMENT_ALREADY_EXISTS_CODE, ENVIRONMENT_METADATA_VERSION, EnvironmentError,
    EnvironmentLayout, materialize_environment, read_environment_metadata,
};
use xiao_source::SourceFile;

/// 为并行环境测试生成隔离目录后缀。
static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);

/// 一个隔离的环境测试项目。
struct TempProject {
    path: PathBuf,
}

impl TempProject {
    /// 创建临时项目目录。
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "xiao-environment-{stamp}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temporary environment project");
        Self { path }
    }
}

impl Drop for TempProject {
    /// 清理临时项目目录。
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
/// 默认环境和显式环境名称必须使用冻结的目录映射。
fn environment_layout_uses_default_and_explicit_directory_names() {
    let project = TempProject::new();
    let default = EnvironmentLayout::for_project(&project.path, None).expect("默认布局");
    assert_eq!(default.logical_name, "venv");
    assert_eq!(default.directory_name, ".venv");
    assert_eq!(default.path, project.path.join(".venv"));

    let named = EnvironmentLayout::for_project(&project.path, Some("dev")).expect("显式布局");
    assert_eq!(named.logical_name, "dev");
    assert_eq!(named.directory_name, "dev");
    assert_eq!(named.path, project.path.join("dev"));
}

#[test]
/// v2 环境元数据必须稳定且重复创建必须返回专用诊断。
fn environment_metadata_is_stable_and_duplicate_creation_is_rejected() {
    let project = TempProject::new();
    let document = parse_config_project(&SourceFile::from_text(
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    ))
    .expect("配置应合法");
    let toolchain = Toolchain::new("clang").with_versions(ToolchainVersions {
        clang: "clang 18".to_owned(),
        ..ToolchainVersions::default()
    });
    let target = TargetDescription::host();

    let first = materialize_environment(&project.path, None, &document, &toolchain, &target)
        .expect("首次创建环境");
    assert_eq!(first.logical_name, "venv");
    assert_eq!(first.directory_name, ".venv");
    assert_eq!(first.metadata_version, ENVIRONMENT_METADATA_VERSION);
    assert!(first.package_mappings.is_empty());
    assert!(!first.config_fingerprint.is_empty());
    assert!(!first.toolchain_fingerprint.is_empty());
    assert!(!first.target_fingerprint.is_empty());
    assert!(!first.environment_fingerprint.is_empty());
    let json = first.to_json();
    assert!(!json.contains(project.path.to_string_lossy().as_ref()));
    let parsed: Value = serde_json::from_str(&json).expect("环境元数据必须是有效 JSON");
    assert_eq!(parsed["metadata_version"], ENVIRONMENT_METADATA_VERSION);
    assert_eq!(parsed["lockfile_summary"], Value::Null);
    assert_eq!(parsed["package_mappings"], Value::Array(Vec::new()));
    let loaded = read_environment_metadata(project.path.join(".venv/.xiao-environment.json"))
        .expect("刚写入的环境元数据应能读取");
    assert_eq!(loaded, first);

    let second = materialize_environment(&project.path, None, &document, &toolchain, &target)
        .expect_err("重复创建必须失败");
    assert!(matches!(second, EnvironmentError::AlreadyExists { .. }));
    assert_eq!(second.code(), ENVIRONMENT_ALREADY_EXISTS_CODE);
}

#[test]
/// v1 元数据按空映射兼容读取，高版本必须被稳定拒绝。
fn reads_v1_metadata_and_rejects_future_version() {
    let v1 = r#"{
      "metadata_version": 1,
      "logical_name": "venv",
      "directory_name": ".venv",
      "config_fingerprint": "config",
      "toolchain_fingerprint": "toolchain",
      "target_fingerprint": "target",
      "environment_fingerprint": "environment",
      "lockfile_summary": null
    }"#;
    let metadata = xiao_package::EnvironmentMetadata::from_json(v1).expect("v1 应兼容读取");
    assert_eq!(metadata.metadata_version, 1);
    assert!(metadata.package_mappings.is_empty());

    let future = v1.replace("\"metadata_version\": 1", "\"metadata_version\": 99");
    let error =
        xiao_package::EnvironmentMetadata::from_json(&future).expect_err("未来版本不能被猜测读取");
    assert_eq!(error.code(), "X05-ENV-004");
}
