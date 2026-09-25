//! E2B 项目同步、安装、冻结锁、目标选择和只读视图回归。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_config::{ConfigDocument, parse_config_project};
use xiao_package::{
    CacheLayout, ENVIRONMENT_METADATA_FILE, PackageOperation, apply_packages,
    environment_package_view, fingerprint_config, lockfile_path, read_environment_metadata,
    read_lockfile, update_environment_mappings,
};
use xiao_source::SourceFile;

/// 隔离项目和全局缓存，防止测试触碰用户目录。
struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// 创建当前测试专属的临时目录。
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("xiao-e2b-{stamp}-{}", std::process::id()));
        fs::create_dir(&root).expect("workspace");
        Self { root }
    }
    /// 拼接测试夹具的绝对路径。
    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }
    /// 写入测试项目的源码或配置。
    fn write(&self, relative: &str, content: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(path, content).expect("write fixture");
    }
    /// 构造根项目及一个本地路径依赖。
    fn setup(&self) {
        self.write("project/config.xiao", "[project]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies]\nlib = { path = \"../lib\" }\n");
        self.write(
            "lib/config.xiao",
            "[project]\nname = \"lib\"\nversion = \"1.0.0\"\n",
        );
        self.write("lib/main.xiao", "return 42\n");
    }
    /// 使用配置层的项目入口解析根文档。
    fn document(&self) -> ConfigDocument {
        let text = fs::read_to_string(self.path("project/config.xiao")).expect("config");
        parse_config_project(&SourceFile::from_text(&text)).expect("parse config")
    }
    /// 为本测试单独指定全局缓存根目录。
    fn layout(&self) -> CacheLayout {
        CacheLayout::from_xiao_home(Some(&self.path("home")), &self.root).expect("isolated cache")
    }
    /// 以真实包操作入口执行指定模式。
    fn run(
        &self,
        active: Option<&Path>,
        operation: PackageOperation,
    ) -> Result<xiao_package::PackageOperationResult, xiao_package::PackageSyncError> {
        apply_packages(
            &self.path("project"),
            active,
            &self.document(),
            &Toolchain::new("clang"),
            &TargetDescription::host(),
            operation,
            self.layout(),
        )
    }
}

impl Drop for Workspace {
    /// 清除由只读源码缓存生成的临时目录。
    fn drop(&mut self) {
        /// 恢复只读缓存对象的删除权限。
        fn writable(path: &Path) {
            let Ok(metadata) = fs::symlink_metadata(path) else {
                return;
            };
            if metadata.is_dir() {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        writable(&entry.path());
                    }
                }
            }
            let mut permissions = metadata.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = fs::set_permissions(path, permissions);
        }
        writable(&self.root);
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// 构造默认的可更新锁文件同步模式。
fn sync() -> PackageOperation {
    PackageOperation::Sync {
        keep_extra: false,
        locked: false,
        frozen: false,
    }
}

#[derive(Deserialize)]
/// 一组具备阶段标记的 E2B JSON 规格用例。
struct Snapshot {
    stage: String,
    status: String,
    cases: Vec<SpecCase>,
}

#[derive(Deserialize)]
/// 单次操作的输入和输出预期。
struct SpecCase {
    name: String,
    mode: String,
    seed: bool,
    environment: Option<String>,
    lock_status: Option<String>,
    activate: Option<bool>,
    mutation: Option<String>,
}

/// 解析并核对 E2B 规格文件的外层契约。
fn spec(raw: &str) -> Snapshot {
    let snapshot: Snapshot = serde_json::from_str(raw).expect("E2B spec JSON");
    assert_eq!(snapshot.stage, "11A-E2B");
    assert_eq!(snapshot.status, "verified-static");
    assert!(!snapshot.cases.is_empty());
    snapshot
}

#[test]
/// 遍历每条成功夹具，实际执行同步和安装。
fn valid_spec_is_executed() {
    let snapshot = spec(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11a-sync/valid.json"
    )));
    for case in snapshot.cases {
        let workspace = Workspace::new();
        workspace.setup();
        if case.seed {
            workspace.run(None, sync()).expect(&case.name);
        }
        let active = workspace.path("project/dev");
        let result = workspace
            .run(
                if case.mode == "active-sync" {
                    Some(&active)
                } else {
                    None
                },
                if case.mode == "install" {
                    PackageOperation::Install
                } else {
                    sync()
                },
            )
            .expect(&case.name);
        assert_eq!(
            result.environment_path,
            workspace.path(case.environment.as_deref().expect("environment")),
            "{}",
            case.name
        );
        assert_eq!(result.lock_status, case.lock_status, "{}", case.name);
        assert_eq!(
            result.activate,
            case.activate.expect("activation"),
            "{}",
            case.name
        );
    }
}

#[test]
/// 逐个拒绝错误夹具，确保失败不创建目标或重写锁文件。
fn error_spec_is_executed_without_new_environment_or_lock_writes() {
    let snapshot = spec(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11a-sync/errors.json"
    )));
    for case in snapshot.cases {
        let workspace = Workspace::new();
        workspace.setup();
        if case.seed {
            workspace.run(None, sync()).expect(&case.name);
        }
        match case.mutation.as_deref().expect("mutation") {
            "none" => {},
            "content" => workspace.write("lib/main.xiao", "return 43\n"),
            "nested-lock" => {
                workspace.write("lib/nested/xiao.lock.json", "original\n");
                workspace.run(None, sync()).expect("update lock for nested file");
                workspace.write("lib/nested/xiao.lock.json", "changed\n");
            },
            "future" => {
                let path = lockfile_path(workspace.path("project"));
                let mut lock = read_lockfile(&path).expect("lock");
                lock.lock_version += 1;
                fs::write(path, lock.to_json()).expect("future lock");
            },
            "cycle" => workspace.write("lib/config.xiao", "[project]\nname = \"lib\"\nversion = \"1.0.0\"\n[dependencies]\napp = { path = \"../project\" }\n"),
            other => panic!("unsupported spec mutation: {other}"),
        }
        let lock_path = lockfile_path(workspace.path("project"));
        let before = fs::read(&lock_path).ok();
        let operation = match case.mode.as_str() {
            "sync" => sync(),
            "install" | "missing-active" => PackageOperation::Install,
            "locked" => PackageOperation::Sync {
                keep_extra: false,
                locked: true,
                frozen: false,
            },
            "frozen" => PackageOperation::Sync {
                keep_extra: false,
                locked: false,
                frozen: true,
            },
            "conflict" => PackageOperation::Sync {
                keep_extra: false,
                locked: true,
                frozen: true,
            },
            other => panic!("unsupported spec mode: {other}"),
        };
        let missing = workspace.path("project/missing");
        let active = if case.mode == "missing-active" {
            Some(missing.as_path())
        } else {
            None
        };
        assert!(workspace.run(active, operation).is_err(), "{}", case.name);
        assert!(!missing.exists(), "{}", case.name);
        assert_eq!(fs::read(&lock_path).ok(), before, "{}", case.name);
        if !case.seed {
            assert!(!workspace.path("project/.venv").exists(), "{}", case.name);
        }
    }
}

#[test]
/// 首次同步、二次幂等、显式激活路径和禁止改锁模式共同回归。
fn sync_creates_reuses_and_respects_active_target() {
    let workspace = Workspace::new();
    workspace.setup();
    let first = workspace.run(None, sync()).expect("first sync");
    assert!(first.created && first.changed && first.activate);
    assert_eq!(first.lock_status.as_deref(), Some("created"));
    assert_eq!(first.environment_path, workspace.path("project/.venv"));
    assert_eq!(
        environment_package_view(&first.environment_path)
            .expect("view")
            .len(),
        2
    );
    let snapshot = fs::read(lockfile_path(workspace.path("project"))).expect("lock snapshot");
    let second = workspace.run(None, sync()).expect("idempotent sync");
    assert!(!second.created && !second.changed);
    assert_eq!(second.lock_status.as_deref(), Some("reused"));
    assert_eq!(
        fs::read(lockfile_path(workspace.path("project"))).expect("lock"),
        snapshot
    );
    let named = workspace.path("project/dev");
    let third = workspace
        .run(Some(&named), sync())
        .expect("active takes priority");
    assert_eq!(third.environment_path, named);
    assert!(third.created);
    assert_eq!(
        workspace
            .run(
                None,
                PackageOperation::Sync {
                    keep_extra: false,
                    locked: true,
                    frozen: false
                }
            )
            .expect("locked")
            .lock_status,
        None
    );
    assert_eq!(
        workspace
            .run(
                None,
                PackageOperation::Sync {
                    keep_extra: false,
                    locked: false,
                    frozen: true
                }
            )
            .expect("frozen")
            .lock_status,
        None
    );
}

#[test]
/// 更新根配置时在同一次原子写入中刷新环境指纹与包映射。
fn sync_refreshes_environment_fingerprint_after_config_change() {
    let workspace = Workspace::new();
    workspace.setup();
    let first = workspace.run(None, sync()).expect("initial sync");
    let metadata_path = first.environment_path.join(ENVIRONMENT_METADATA_FILE);
    let before = read_environment_metadata(&metadata_path).expect("first metadata");
    workspace.write("project/config.xiao", "[project]\nname = \"app\"\nversion = \"0.2.0\"\n[dependencies]\nlib = { path = \"../lib\" }\n");
    let updated = workspace.run(None, sync()).expect("updated sync");
    assert!(updated.changed);
    assert_eq!(updated.lock_status.as_deref(), Some("updated"));
    let after = read_environment_metadata(&metadata_path).expect("updated metadata");
    assert_ne!(
        after.environment_fingerprint,
        before.environment_fingerprint
    );
    assert_eq!(
        after.config_fingerprint,
        fingerprint_config(&workspace.document())
    );
    assert_eq!(after.package_mappings.len(), 2);
}

#[test]
/// 安装仅消费已有锁，并在无激活环境时写全局映射容器。
fn install_uses_active_or_global_without_touching_lock_or_project() {
    let workspace = Workspace::new();
    workspace.setup();
    assert!(workspace.run(None, PackageOperation::Install).is_err());
    workspace.run(None, sync()).expect("create lock");
    let missing_active = workspace.path("project/missing");
    assert!(
        workspace
            .run(Some(&missing_active), PackageOperation::Install)
            .is_err()
    );
    assert!(!missing_active.exists());
    let locked = fs::read(lockfile_path(workspace.path("project"))).expect("lock");
    fs::remove_dir_all(workspace.path("project/.venv")).expect("remove project environment");
    let installed = workspace
        .run(None, PackageOperation::Install)
        .expect("global install");
    assert_eq!(
        installed.environment_path,
        workspace.path("home/envs/global")
    );
    assert!(installed.created && !installed.activate);
    assert!(!workspace.path("project/.venv").exists());
    assert_eq!(
        workspace
            .run(Some(&installed.environment_path), PackageOperation::Install)
            .expect("active install")
            .environment_path,
        installed.environment_path
    );
    assert_eq!(
        fs::read(lockfile_path(workspace.path("project"))).expect("lock"),
        locked
    );
}

#[test]
/// 默认清理多余映射、反向开关保留映射及冻结模式不改锁。
fn keep_extra_and_lock_rejection_are_atomic() {
    let workspace = Workspace::new();
    workspace.setup();
    let first = workspace.run(None, sync()).expect("initial sync");
    let metadata_path = first.environment_path.join(ENVIRONMENT_METADATA_FILE);
    let mut mappings = environment_package_view(&first.environment_path).expect("mappings");
    let mut extra = mappings[0].clone();
    extra.package.name = "extra".to_owned();
    mappings.push(extra);
    mappings.sort_by(|left, right| left.package.cmp(&right.package));
    update_environment_mappings(&metadata_path, mappings).expect("add extra");
    workspace
        .run(
            None,
            PackageOperation::Sync {
                keep_extra: true,
                locked: false,
                frozen: false,
            },
        )
        .expect("keep extra");
    assert_eq!(
        environment_package_view(&first.environment_path)
            .expect("view")
            .len(),
        3
    );
    workspace.run(None, sync()).expect("prune extras");
    assert_eq!(
        environment_package_view(&first.environment_path)
            .expect("view")
            .len(),
        2
    );
    let before = fs::read(lockfile_path(workspace.path("project"))).expect("original lock");
    workspace.write("lib/main.xiao", "return 43\n");
    for operation in [
        PackageOperation::Sync {
            keep_extra: false,
            locked: true,
            frozen: false,
        },
        PackageOperation::Sync {
            keep_extra: false,
            locked: false,
            frozen: true,
        },
        PackageOperation::Install,
    ] {
        assert!(workspace.run(None, operation).is_err());
        assert_eq!(
            fs::read(lockfile_path(workspace.path("project"))).expect("lock"),
            before
        );
    }
    workspace.run(None, sync()).expect("update lock");
    assert_ne!(
        fs::read(lockfile_path(workspace.path("project"))).expect("updated lock"),
        before
    );
    assert_eq!(
        read_environment_metadata(metadata_path)
            .expect("metadata")
            .package_mappings
            .len(),
        2
    );
}

#[test]
/// 只读枚举不运行源码，未来版本锁文件与依赖环必须拒绝。
fn rejects_future_lock_cycle_and_does_not_execute_package_code_on_view() {
    let workspace = Workspace::new();
    workspace.setup();
    workspace.write(
        "lib/side-effect.xiao",
        "# 如果运行才创建 side-effect-marker\npanic(\"must not run\")\n",
    );
    let first = workspace.run(None, sync()).expect("sync");
    assert_eq!(
        environment_package_view(&first.environment_path)
            .expect("metadata only")
            .len(),
        2
    );
    assert!(!workspace.path("side-effect-marker").exists());
    let lock_path = lockfile_path(workspace.path("project"));
    let mut lock = read_lockfile(&lock_path).expect("lock");
    lock.lock_version += 1;
    fs::write(&lock_path, lock.to_json()).expect("future lock");
    assert!(
        workspace
            .run(None, sync())
            .expect_err("future lock rejected")
            .message
            .contains("版本")
    );
    workspace.write("lib/config.xiao", "[project]\nname = \"lib\"\nversion = \"1.0.0\"\n[dependencies]\napp = { path = \"../project\" }\n");
    assert!(workspace.run(None, sync()).is_err());
}
