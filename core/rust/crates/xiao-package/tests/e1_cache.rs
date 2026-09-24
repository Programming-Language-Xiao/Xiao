//! 11A-E1 本地依赖、共享缓存和环境映射规格测试。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use xiao_codegen_llvm::{TargetDescription, Toolchain, ToolchainVersions};
use xiao_config::parse_config_project;
use xiao_package::{
    CACHE_INVALID_INPUT_CODE, CACHE_OBJECT_CORRUPT_CODE, CacheError, CacheLayout, CacheStore,
    EnvironmentLayout, build_environment_metadata_with_mappings,
    materialize_environment_from_graph, materialize_global_environment_from_graph,
    materialize_package_mappings, read_environment_metadata, resolve_package_object,
    resolve_project, source_directory_digest,
};
use xiao_source::SourceFile;

/// 为并行缓存测试生成隔离目录后缀。
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

/// 一个隔离的缓存和项目测试工作区。
struct TempWorkspace {
    path: PathBuf,
}

impl TempWorkspace {
    /// 创建临时工作区目录。
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("xiao-e1-cache-{stamp}-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).expect("create temporary cache fixture");
        Self { path }
    }

    /// 返回工作区内的相对路径。
    fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.path.join(relative)
    }

    /// 写入一个 UTF-8 fixture 文件。
    fn write(&self, relative: impl AsRef<Path>, text: &str) {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        fs::write(path, text).expect("write fixture file");
    }
}

impl Drop for TempWorkspace {
    /// 清理临时工作区。
    fn drop(&mut self) {
        make_writable(&self.path);
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// 打开注入临时 `XIAO_HOME` 的缓存。
fn open_cache(workspace: &TempWorkspace) -> CacheStore {
    let home = workspace.path("isolated-xiao-home");
    let layout = CacheLayout::from_xiao_home(Some(&home), &workspace.path)
        .expect("注入的绝对 XIAO_HOME 应有效");
    CacheStore::open(layout).expect("隔离缓存应可创建")
}

/// 创建测试用的稳定工具链描述。
fn toolchain() -> Toolchain {
    Toolchain::new("clang").with_versions(ToolchainVersions {
        clang: "clang 18".to_owned(),
        ..ToolchainVersions::default()
    })
}

/// 创建无依赖测试包配置。
fn app_config(name: &str) -> String {
    format!("[project]\nname = \"{name}\"\nversion = \"0.1.0\"\n")
}

/// 创建声明共享本地依赖的测试包配置。
fn app_with_shared_config(name: &str) -> String {
    format!(
        "[project]\nname = \"{name}\"\nversion = \"0.1.0\"\n[dependencies]\nshared = {{ path = \"../shared\" }}\n"
    )
}

#[test]
/// `XIAO_HOME` 空值回退用户域，绝对路径注入隔离，相对路径稳定拒绝。
fn cache_layout_obeys_xiao_home_boundary() {
    let workspace = TempWorkspace::new();
    let fallback = workspace.path("user");
    let fallback_layout = CacheLayout::from_xiao_home(None, &fallback).expect("空值应回退");
    assert_eq!(fallback_layout.xiao_home(), fallback.join(".xiao"));
    let empty_layout =
        CacheLayout::from_xiao_home(Some(Path::new("")), &fallback).expect("空字符串应回退");
    assert_eq!(empty_layout, fallback_layout);

    let injected = workspace.path("custom-xiao-home");
    let layout = CacheLayout::from_xiao_home(Some(&injected), &fallback).expect("注入路径");
    assert_eq!(layout.cache_root(), injected.join("cache"));
    assert_eq!(layout.environments_root(), injected.join("envs"));
    assert_eq!(
        layout.source_objects_root(),
        injected.join("cache/objects/source/sha256")
    );

    let error = CacheLayout::from_xiao_home(Some(Path::new("relative")), &fallback)
        .expect_err("相对 XIAO_HOME 必须拒绝");
    assert_eq!(error.code(), CACHE_INVALID_INPUT_CODE);
}

#[test]
/// 规范化目录树的遍历稳定，空目录参与摘要且不同根路径不改变摘要。
fn source_digest_is_deterministic_and_path_independent() {
    let workspace = TempWorkspace::new();
    workspace.write("one/config.xiao", app_config("same").as_str());
    workspace.write("one/src/main.xiao", "return 1\n");
    fs::create_dir_all(workspace.path("one/empty")).expect("create empty directory");
    workspace.write("two/config.xiao", app_config("same").as_str());
    workspace.write("two/src/main.xiao", "return 1\n");
    fs::create_dir_all(workspace.path("two/empty")).expect("create matching empty directory");

    let first = source_directory_digest(workspace.path("one")).expect("计算首个摘要");
    let second = source_directory_digest(workspace.path("two")).expect("计算第二个摘要");
    assert_eq!(first, second);
    fs::create_dir_all(workspace.path("two/extra")).expect("create different empty directory");
    assert_ne!(
        first,
        source_directory_digest(workspace.path("two")).expect("计算不同摘要")
    );
    assert_eq!(first.len(), 64);
    assert!(
        first
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
}

#[test]
/// 相同内容只保存一个对象，源目录变化产生新对象，旧快照仍可只读读取。
fn cache_import_deduplicates_and_preserves_immutable_snapshots() {
    let workspace = TempWorkspace::new();
    for root in ["source-a", "source-b"] {
        workspace.write(format!("{root}/config.xiao"), app_config("same").as_str());
        workspace.write(format!("{root}/main.xiao"), "return 1\n");
    }
    let cache = open_cache(&workspace);
    let first = cache
        .import_source_directory(workspace.path("source-a"))
        .expect("导入首个源码对象");
    let second = cache
        .import_source_directory(workspace.path("source-b"))
        .expect("导入相同源码对象");
    assert_eq!(first.reference, second.reference);
    assert_eq!(first.path, second.path);
    assert_eq!(
        fs::read_dir(first.path.parent().expect("对象分片目录"))
            .expect("读取对象分片")
            .count(),
        1
    );
    assert!(
        fs::metadata(first.path.join("main.xiao"))
            .expect("对象文件")
            .permissions()
            .readonly()
    );
    assert_eq!(
        cache
            .read_source_file(&first.reference.digest, "main.xiao")
            .expect("只读读取"),
        b"return 1\n"
    );

    workspace.write("source-a/main.xiao", "return 2\n");
    let changed = cache
        .import_source_directory(workspace.path("source-a"))
        .expect("源目录变化后产生新对象");
    assert_ne!(changed.reference.digest, first.reference.digest);
    assert_eq!(
        cache
            .read_source_file(&first.reference.digest, "main.xiao")
            .expect("读取旧快照"),
        b"return 1\n"
    );
    assert_eq!(
        cache
            .read_source_file(&changed.reference.digest, "main.xiao")
            .expect("读取新快照"),
        b"return 2\n"
    );
}

#[test]
/// 修改缓存字节必须在读取前报错并隔离损坏对象。
fn corrupted_object_is_rejected_and_quarantined() {
    let workspace = TempWorkspace::new();
    workspace.write("source/config.xiao", app_config("corruptible").as_str());
    workspace.write("source/main.xiao", "return 1\n");
    let cache = open_cache(&workspace);
    let object = cache
        .import_source_directory(workspace.path("source"))
        .expect("导入对象");
    make_writable(&object.path);
    fs::write(object.path.join("main.xiao"), "tampered\n").expect("篡改缓存对象");

    let error = cache
        .verify_source_object(&object.reference.digest)
        .expect_err("损坏对象不能通过校验");
    assert_eq!(error.code(), CACHE_OBJECT_CORRUPT_CODE);
    let CacheError::ObjectCorrupt { quarantine, .. } = error else {
        panic!("应返回对象完整性错误");
    };
    let quarantine = quarantine.expect("损坏对象应被隔离");
    assert!(quarantine.exists());
    assert!(!object.path.exists());
}

#[test]
/// 两个项目和一个全局环境共享对象摘要，但各自保存独立映射且项目目录无源码副本。
fn project_and_global_environments_share_objects_without_links() {
    let workspace = TempWorkspace::new();
    workspace.write("shared/config.xiao", app_config("shared").as_str());
    workspace.write("shared/lib.xiao", "export const value = 1\n");
    workspace.write(
        "project-a/config.xiao",
        app_with_shared_config("project_a").as_str(),
    );
    workspace.write(
        "project-b/config.xiao",
        app_with_shared_config("project_b").as_str(),
    );

    let graph_a = resolve_project(workspace.path("project-a"));
    let graph_b = resolve_project(workspace.path("project-b"));
    assert!(
        graph_a.is_success(),
        "project-a diagnostics: {:?}",
        graph_a.diagnostics
    );
    assert!(
        graph_b.is_success(),
        "project-b diagnostics: {:?}",
        graph_b.diagnostics
    );
    let document_a =
        parse_config_project(&SourceFile::from_text(&app_with_shared_config("project_a")))
            .expect("project-a 配置");
    let document_b =
        parse_config_project(&SourceFile::from_text(&app_with_shared_config("project_b")))
            .expect("project-b 配置");
    let target = TargetDescription::host();
    let toolchain = toolchain();
    let cache = open_cache(&workspace);

    let metadata_a = materialize_environment_from_graph(
        workspace.path("project-a"),
        None,
        &document_a,
        &toolchain,
        &target,
        &graph_a.graph,
        &cache,
    )
    .expect("物化 project-a 环境");
    let metadata_b = materialize_environment_from_graph(
        workspace.path("project-b"),
        None,
        &document_b,
        &toolchain,
        &target,
        &graph_b.graph,
        &cache,
    )
    .expect("物化 project-b 环境");
    let global = materialize_global_environment_from_graph(
        "default",
        &document_a,
        &toolchain,
        &target,
        &graph_a.graph,
        &cache,
    )
    .expect("物化全局环境");
    let mapping_a = metadata_a
        .package_mappings
        .iter()
        .find(|mapping| mapping.package.name == "shared")
        .expect("project-a shared 映射");
    let mapping_b = metadata_b
        .package_mappings
        .iter()
        .find(|mapping| mapping.package.name == "shared")
        .expect("project-b shared 映射");
    let mapping_global = global
        .package_mappings
        .iter()
        .find(|mapping| mapping.package.name == "shared")
        .expect("全局 shared 映射");
    assert_eq!(mapping_a.object, mapping_b.object);
    assert_eq!(mapping_a.object, mapping_global.object);

    let loaded_a =
        read_environment_metadata(workspace.path("project-a/.venv/.xiao-environment.json"))
            .expect("读取 project-a 元数据");
    let loaded_b =
        read_environment_metadata(workspace.path("project-b/.venv/.xiao-environment.json"))
            .expect("读取 project-b 元数据");
    let shared_identity = graph_a
        .graph
        .nodes
        .keys()
        .find(|identity| identity.name == "shared")
        .expect("shared identity");
    let resolved = resolve_package_object(&loaded_a, shared_identity, &cache)
        .expect("解析 project-a shared 对象");
    assert_eq!(resolved.reference, mapping_a.object);
    assert_eq!(loaded_a.to_json(), metadata_a.to_json());
    assert_eq!(loaded_b.to_json(), metadata_b.to_json());
    assert_eq!(
        fs::read_dir(workspace.path("project-a/.venv"))
            .expect("读取项目环境")
            .count(),
        1
    );
    assert!(
        !fs::symlink_metadata(workspace.path("project-a/.venv/.xiao-environment.json"))
            .expect("读取环境元数据文件")
            .file_type()
            .is_symlink()
    );

    let mappings = materialize_package_mappings(&graph_a.graph, &cache).expect("重复物化映射");
    let rebuilt = build_environment_metadata_with_mappings(
        &EnvironmentLayout::default_for_project(workspace.path("project-a")),
        &document_a,
        &toolchain,
        &target,
        mappings,
    );
    assert_eq!(rebuilt.to_json(), metadata_a.to_json());

    let mut isolated = loaded_a.clone();
    isolated.package_mappings.pop();
    assert_ne!(isolated.to_json(), loaded_b.to_json());
    assert_eq!(
        loaded_b.package_mappings.len(),
        metadata_b.package_mappings.len()
    );
}

#[test]
/// E1 规格目录由本集成测试真实加载，而不是只靠目录登记。
fn cache_spec_snapshot_is_executed() {
    let snapshot: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11a-cache/valid.json"
    )))
    .expect("E1 规格快照必须是有效 JSON");
    assert_eq!(snapshot["stage"], "11A-E1");
    assert_eq!(snapshot["status"], "verified-static");
    assert!(
        snapshot["cases"]
            .as_array()
            .is_some_and(|cases| !cases.is_empty())
    );
}

/// 为损坏隔离测试递归解除对象只读属性。
fn make_writable(path: &Path) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if metadata.file_type().is_symlink() {
        return;
    }
    if metadata.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                make_writable(&entry.path());
            }
        }
    }
    let mut permissions = metadata.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    let _ = fs::set_permissions(path, permissions);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .map(|value| value.permissions())
            .unwrap_or_else(|_| metadata.permissions());
        permissions.set_mode(if metadata.is_dir() { 0o755 } else { 0o644 });
        let _ = fs::set_permissions(path, permissions);
    }
}
