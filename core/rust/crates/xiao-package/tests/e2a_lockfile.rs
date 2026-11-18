//! 11A-E2A 锁文件、诊断和环境映射原子更新的规格测试。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use xiao_codegen_llvm::{TargetDescription, Toolchain, ToolchainVersions};
use xiao_config::{ConfigDocument, parse_config_project};
use xiao_package::{
    CacheLayout, CacheStore, ENVIRONMENT_METADATA_FILE, LOCKFILE_INVALID_CODE, LOCKFILE_VERSION,
    LockFile, LockFileWriteStatus, LockedDependency, build_lockfile, fingerprint_config,
    generate_or_reuse_lockfile, lockfile_path, materialize_environment,
    materialize_package_mappings, read_environment_metadata, read_lockfile, resolve_project,
    source_directory_digest, update_environment_mappings, update_environment_metadata,
    validate_lockfile, write_lockfile,
};
use xiao_source::SourceFile;

/// 为并行测试生成隔离工作区后缀。
static NEXT_WORKSPACE: AtomicU64 = AtomicU64::new(0);

/// 临时项目及全局缓存隔离目录。
struct TempWorkspace {
    path: PathBuf,
}

impl TempWorkspace {
    /// 创建不会触及真实用户目录的临时工作区。
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "xiao-e2a-lockfile-{stamp}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated workspace");
        Self { path }
    }

    /// 返回隔离工作区内的绝对路径。
    fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.path.join(relative)
    }

    /// 写入规格夹具的 UTF-8 文件内容。
    fn write(&self, relative: impl AsRef<Path>, text: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
        fs::write(path, text).expect("write fixture");
    }

    /// 注入专属于测试工作区的 XIAO_HOME。
    fn cache(&self) -> CacheStore {
        let home = self.path("isolated-xiao-home");
        let layout = CacheLayout::from_xiao_home(Some(&home), &self.path)
            .expect("inject absolute XIAO_HOME");
        CacheStore::open(layout).expect("open isolated cache")
    }

    /// 从当前项目配置创建已通过校验的文档。
    fn document(&self, project: &str) -> ConfigDocument {
        let text = fs::read_to_string(self.path(format!("{project}/config.xiao")))
            .expect("read root config");
        parse_config_project(&SourceFile::from_text(&text)).expect("parse root config")
    }
}

impl Drop for TempWorkspace {
    /// 清理可能含只读缓存对象的临时目录。
    fn drop(&mut self) {
        make_writable(&self.path);
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// 带输入文件、变化和预期输出的一条规格用例。
#[derive(Deserialize)]
struct SnapshotCase {
    name: String,
    project: String,
    files: BTreeMap<String, String>,
    #[serde(default)]
    after: BTreeMap<String, String>,
    expect: String,
    #[serde(default)]
    packages: Vec<String>,
    #[serde(default)]
    edges: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    write: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
    lock_version: Option<u32>,
}

/// 有阶段标识和非空用例数组的规格快照。
#[derive(Deserialize)]
struct Snapshot {
    stage: String,
    status: String,
    cases: Vec<SnapshotCase>,
}

/// 读取并校验 E2A 规格快照外层结构。
fn read_snapshot(raw: &str) -> Snapshot {
    let snapshot: Snapshot = serde_json::from_str(raw).expect("E2A spec must be valid JSON");
    assert_eq!(snapshot.stage, "11A-E2A");
    assert_eq!(snapshot.status, "verified-static");
    assert!(!snapshot.cases.is_empty());
    snapshot
}

/// 根据夹具文件树物化测试输入。
fn populate(workspace: &TempWorkspace, files: &BTreeMap<String, String>) {
    for (path, contents) in files {
        workspace.write(path, contents);
    }
}

#[test]
/// 逐例验证完整图、JSON 字节确定性和二次不重写。
fn valid_lockfile_spec_is_executed() {
    let snapshot = read_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11a-lockfile/valid.json"
    )));
    for case in snapshot.cases {
        assert_eq!(case.expect, "success", "{}", case.name);
        let workspace = TempWorkspace::new();
        populate(&workspace, &case.files);
        let project = workspace.path(&case.project);
        let resolution = resolve_project(&project);
        assert!(
            resolution.is_success(),
            "{}: {:?}",
            case.name,
            resolution.diagnostics
        );
        let document = workspace.document(&case.project);
        let cache = workspace.cache();
        let before = source_directory_digest(&project).expect("source digest before lock");
        let first = build_lockfile(&resolution.graph, &document, &cache).expect("build lockfile");
        assert_eq!(first.lock_version, LOCKFILE_VERSION);
        if first
            .packages
            .values()
            .all(|package| package.source_artifact.is_none())
        {
            let mut previous = first.clone();
            previous.lock_version = 1;
            assert_eq!(
                LockFile::from_json(&previous.to_json())
                    .unwrap()
                    .lock_version,
                1
            );
        }
        assert_eq!(first.config_fingerprint, fingerprint_config(&document));
        let packages = first
            .packages
            .values()
            .map(|package| format!("{}@{}", package.name, package.version))
            .collect::<Vec<_>>();
        assert_eq!(packages, case.packages, "{}", case.name);
        for package in first.packages.values() {
            assert!(package.source.source_id.starts_with("path:"));
            assert_eq!(package.content_digest.len(), 64);
            assert!(package.precompiled_variants.is_empty());
            assert!(package.target_conditions.is_empty());
            let expected = &case.edges[&package.name];
            assert_eq!(
                package.dependencies.keys().collect::<Vec<_>>(),
                expected.iter().collect::<Vec<_>>(),
                "{}",
                case.name
            );
        }
        if case.name == "complete-transitive-local-graph" {
            let app = first
                .packages
                .values()
                .find(|package| package.name == "app")
                .expect("app");
            let edge = &app.dependencies["core"];
            assert_eq!(edge.version_constraint.as_deref(), Some("^1.0"));
            assert_eq!(edge.source_reference.as_deref(), Some("local"));
        }
        if case.name == "development-dependency-retains-kind" {
            let app = first
                .packages
                .values()
                .find(|package| package.name == "app")
                .expect("app");
            assert_eq!(app.dependencies["testkit"].kind, "devdependencies");
        }
        let path = lockfile_path(&project);
        let created = generate_or_reuse_lockfile(&project, &resolution.graph, &document, &cache)
            .expect("create lockfile");
        assert_eq!(
            format!("{created:?}").to_lowercase(),
            case.write[0],
            "{}",
            case.name
        );
        let bytes = fs::read(&path).expect("lockfile bytes");
        let modified = fs::metadata(&path)
            .expect("lockfile metadata")
            .modified()
            .expect("mtime");
        assert_eq!(
            String::from_utf8(bytes.clone()).expect("UTF-8"),
            first.to_json()
        );
        assert_eq!(read_lockfile(&path).expect("read lockfile"), first);
        assert_eq!(
            validate_lockfile(&path, &resolution.graph, &document).expect("reuse"),
            first
        );
        let second = generate_or_reuse_lockfile(&project, &resolution.graph, &document, &cache)
            .expect("reuse lockfile without rewriting");
        assert_eq!(
            format!("{second:?}").to_lowercase(),
            case.write[1],
            "{}",
            case.name
        );
        assert_eq!(fs::read(&path).expect("lockfile reused bytes"), bytes);
        assert_eq!(
            fs::metadata(&path)
                .expect("lockfile metadata")
                .modified()
                .expect("mtime"),
            modified
        );
        workspace.write(
            format!("{}/.xiao.lock.json.tmp-interrupted", case.project),
            "{partial",
        );
        assert_eq!(
            source_directory_digest(&project).expect("source digest after lock"),
            before
        );
    }
}

#[test]
/// 逐例验证配置、源码、路径变化及未来版本的独立诊断。
fn error_lockfile_spec_is_executed() {
    let snapshot = read_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11a-lockfile/errors.json"
    )));
    for case in snapshot.cases {
        assert_eq!(case.expect, "error", "{}", case.name);
        let workspace = TempWorkspace::new();
        populate(&workspace, &case.files);
        let project = workspace.path(&case.project);
        let cache = workspace.cache();
        let initial = resolve_project(&project);
        assert!(initial.is_success(), "{}", case.name);
        let document = workspace.document(&case.project);
        generate_or_reuse_lockfile(&project, &initial.graph, &document, &cache)
            .expect("create initial lockfile");
        let path = lockfile_path(&project);
        let original = fs::read(&path).expect("initial lock bytes");
        let code = if let Some(version) = case.lock_version {
            let text = String::from_utf8(original.clone())
                .expect("JSON UTF-8")
                .replace(
                    &format!("\"lock_version\": {LOCKFILE_VERSION}"),
                    &format!("\"lock_version\": {version}"),
                );
            let code = LockFile::from_json(&text)
                .expect_err("future version must fail")
                .code();
            fs::write(&path, &text).expect("place future lockfile");
            assert_eq!(
                read_lockfile(&path)
                    .expect_err("future file must fail")
                    .code(),
                code
            );
            assert_eq!(
                generate_or_reuse_lockfile(&project, &initial.graph, &document, &cache)
                    .expect_err("generation must not overwrite a future lockfile")
                    .code(),
                code
            );
            assert_eq!(
                fs::read(&path).expect("future lockfile retained"),
                text.as_bytes()
            );
            code
        } else {
            populate(&workspace, &case.after);
            let current = resolve_project(&project);
            assert!(
                current.is_success(),
                "{}: {:?}",
                case.name,
                current.diagnostics
            );
            let document = workspace.document(&case.project);
            let error = validate_lockfile(&path, &current.graph, &document)
                .expect_err("stale lockfile must fail");
            let code = error.code();
            assert_eq!(fs::read(&path).expect("old lock bytes"), original);
            assert_eq!(
                generate_or_reuse_lockfile(&project, &current.graph, &document, &cache)
                    .expect("update stale lockfile"),
                LockFileWriteStatus::Updated
            );
            validate_lockfile(&path, &current.graph, &document).expect("updated lockfile");
            code
        };
        assert_eq!(case.diagnostics, vec![code.to_owned()], "{}", case.name);
    }
}

#[test]
/// 注入写入中断残留，并验证已存在元数据可被连续原子替换。
fn environment_mapping_update_replaces_existing_file_without_partial_json() {
    let workspace = TempWorkspace::new();
    workspace.write(
        "project/config.xiao",
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n",
    );
    let project = workspace.path("project");
    let document = workspace.document("project");
    let toolchain = Toolchain::new("clang").with_versions(ToolchainVersions {
        clang: "clang 18".to_owned(),
        ..ToolchainVersions::default()
    });
    let target = TargetDescription::host();
    let initial = materialize_environment(&project, None, &document, &toolchain, &target)
        .expect("create environment");
    let path = project.join(".venv").join(ENVIRONMENT_METADATA_FILE);
    workspace.write(
        "project/.venv/.xiao-environment.json.tmp-interrupted",
        "{half",
    );
    assert_eq!(
        read_environment_metadata(&path).expect("old metadata intact"),
        initial
    );

    let cache = workspace.cache();
    let graph = resolve_project(&project);
    assert!(graph.is_success());
    let mappings = materialize_package_mappings(&graph.graph, &cache).expect("cache mappings");
    let updated = update_environment_mappings(&path, mappings.clone()).expect("first replacement");
    assert_eq!(
        read_environment_metadata(&path).expect("new metadata"),
        updated
    );
    assert_eq!(updated.package_mappings, mappings);
    let cleared = update_environment_mappings(&path, Vec::new()).expect("second replacement");
    assert!(
        read_environment_metadata(&path)
            .expect("second metadata")
            .package_mappings
            .is_empty()
    );
    update_environment_metadata(&path, &updated).expect("third replacement");
    assert_eq!(
        read_environment_metadata(&path).expect("final metadata"),
        updated
    );
    assert_eq!(cleared.lockfile_summary, None);
    assert_eq!(
        update_environment_metadata(workspace.path("missing/.xiao-environment.json"), &updated)
            .expect_err("update cannot create a missing environment")
            .code(),
        "X05-ENV-003"
    );
}

#[test]
/// 拒绝部分包图、缺根包、不可达包和依赖环。
fn invalid_graph_or_unreachable_lock_entry_is_rejected() {
    let workspace = TempWorkspace::new();
    workspace.write(
        "project/config.xiao",
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n",
    );
    let project = workspace.path("project");
    let cache = workspace.cache();
    let document = workspace.document("project");
    let mut graph = resolve_project(&project).graph;
    graph.resolution_order.clear();
    assert_eq!(
        build_lockfile(&graph, &document, &cache)
            .expect_err("partial graph")
            .code(),
        LOCKFILE_INVALID_CODE
    );
    let complete = resolve_project(&project).graph;
    let mut lockfile = build_lockfile(&complete, &document, &cache).expect("valid graph");
    let mut orphan = lockfile
        .packages
        .values()
        .next()
        .expect("root package")
        .clone();
    orphan.name = "orphan".to_owned();
    orphan.source.source_id.push_str("-orphan");
    let key = format!(
        "{}@{}[{}]",
        orphan.name, orphan.version, orphan.source.source_id
    );
    lockfile.packages.insert(key, orphan);
    assert_eq!(
        lockfile
            .validate(&lockfile_path(&project))
            .expect_err("unreachable package")
            .code(),
        LOCKFILE_INVALID_CODE
    );
    lockfile.packages.clear();
    assert_eq!(
        write_lockfile(lockfile_path(&project), &lockfile)
            .expect_err("missing root")
            .code(),
        LOCKFILE_INVALID_CODE
    );
    let mut cyclic = build_lockfile(&complete, &document, &cache).expect("valid graph");
    let root = cyclic.root.clone();
    cyclic
        .packages
        .get_mut(&root.to_string())
        .expect("root package")
        .dependencies
        .insert(
            root.name.clone(),
            LockedDependency {
                kind: "dependencies".to_owned(),
                version_constraint: None,
                source_reference: None,
                config_path: "config.xiao".to_owned(),
                target: root,
            },
        );
    assert_eq!(
        LockFile::from_json(&cyclic.to_json())
            .expect_err("cyclic lockfile")
            .code(),
        LOCKFILE_INVALID_CODE
    );
}

#[test]
/// 拒绝覆盖已存在但无效的锁文件，而不是静默擦除诊断信息。
fn invalid_existing_lockfile_is_not_overwritten() {
    let workspace = TempWorkspace::new();
    workspace.write(
        "project/config.xiao",
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n",
    );
    let project = workspace.path("project");
    let cache = workspace.cache();
    let document = workspace.document("project");
    let graph = resolve_project(&project).graph;
    let lockfile = build_lockfile(&graph, &document, &cache).expect("valid graph");
    let path = lockfile_path(&project);
    fs::write(&path, "{broken").expect("place invalid lockfile");
    assert_eq!(
        write_lockfile(&path, &lockfile)
            .expect_err("invalid lockfile must not be overwritten")
            .code(),
        LOCKFILE_INVALID_CODE
    );
    assert_eq!(
        fs::read_to_string(&path).expect("invalid file retained"),
        "{broken"
    );
}

/// 清理只读缓存对象时递归恢复测试临时目录权限。
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
