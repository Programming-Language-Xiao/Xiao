//! 包操作的锁定、写回及环境无副作用回归。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_config::{DependencyKind, parse_config_project};
use xiao_package::{
    CacheLayout, DependencyEdit, PackageOperation, apply_dependency_edit, apply_packages,
    compare_lockfile, lockfile_path, read_lockfile, resolve_project,
};
use xiao_source::SourceFile;

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "xiao-e3d-operations-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn path(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.path(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn layout(&self) -> CacheLayout {
        CacheLayout::from_xiao_home(Some(&self.path("home")), &self.0).unwrap()
    }

    fn run(
        &self,
        operation: PackageOperation,
    ) -> Result<xiao_package::PackageOperationResult, xiao_package::PackageSyncError> {
        let config = fs::read_to_string(self.path("project/config.xiao")).unwrap();
        let document = parse_config_project(&SourceFile::from_text(&config)).unwrap();
        apply_packages(
            &self.path("project"),
            None,
            &document,
            &Toolchain::new("clang"),
            &TargetDescription::host(),
            operation,
            self.layout(),
        )
    }

    fn edit(
        &self,
        edit: DependencyEdit,
    ) -> Result<xiao_package::PackageOperationResult, xiao_package::PackageSyncError> {
        let config = fs::read_to_string(self.path("project/config.xiao")).unwrap();
        let document = parse_config_project(&SourceFile::from_text(&config)).unwrap();
        apply_dependency_edit(
            &self.path("project"),
            &document,
            &config,
            &edit,
            self.layout(),
        )
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        fn writable(path: &Path) {
            if let Ok(metadata) = fs::symlink_metadata(path) {
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
        }
        writable(&self.0);
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn add(version: &str) -> DependencyEdit {
    DependencyEdit::Add {
        name: "lib".to_owned(),
        kind: DependencyKind::Runtime,
        fields: BTreeMap::from([
            ("path".to_owned(), "../lib".to_owned()),
            ("version".to_owned(), version.to_owned()),
        ]),
    }
}

#[test]
fn lock_update_add_remove_are_distinct_and_never_touch_environment() {
    let workspace = Workspace::new();
    workspace.write(
        "project/config.xiao",
        "# 注释\n[project]\nname = 'app'\nversion = '0.1.0'\n",
    );
    workspace.write(
        "lib/config.xiao",
        "[project]\nname = \"lib\"\nversion = \"1.2.4\"\n",
    );
    let root = workspace.path("project");
    assert_eq!(
        workspace
            .run(PackageOperation::Lock)
            .unwrap()
            .lock_status
            .as_deref(),
        Some("created")
    );
    assert_eq!(
        workspace
            .run(PackageOperation::Lock)
            .unwrap()
            .lock_status
            .as_deref(),
        Some("reused")
    );
    let original = fs::read_to_string(root.join("config.xiao")).unwrap();
    let updated = workspace.run(PackageOperation::Update).unwrap();
    assert_eq!(updated.lock_status.as_deref(), Some("reused"));
    assert!(!updated.activate && !updated.created && !updated.changed);
    assert!(!root.join(".venv").exists());
    assert_eq!(
        fs::read_to_string(root.join("config.xiao")).unwrap(),
        original
    );

    let added = workspace.edit(add("1.2")).unwrap();
    assert_eq!(added.lock_status.as_deref(), Some("updated"));
    let next = fs::read_to_string(root.join("config.xiao")).unwrap();
    assert!(next.starts_with(&original));
    let lock = read_lockfile(lockfile_path(&root)).unwrap();
    assert!(lock.packages.values().any(|entry| entry.name == "lib"));
    let document = parse_config_project(&SourceFile::from_text(&next)).unwrap();
    compare_lockfile(&lock, &resolve_project(&root).graph, &document).unwrap();
    assert!(!root.join(".venv").exists());

    let removed = workspace
        .edit(DependencyEdit::Remove {
            name: "lib".to_owned(),
            kind: DependencyKind::Runtime,
        })
        .unwrap();
    assert_eq!(removed.lock_status.as_deref(), Some("updated"));
    assert_eq!(
        fs::read_to_string(root.join("config.xiao")).unwrap(),
        original + "\n[dependencies]\n"
    );
    assert!(
        read_lockfile(lockfile_path(&root))
            .unwrap()
            .packages
            .values()
            .all(|entry| entry.name != "lib")
    );
}

#[test]
fn invalid_requirement_does_not_change_config_or_existing_lock() {
    let workspace = Workspace::new();
    workspace.write(
        "project/config.xiao",
        "[project]\nname = \"app\"\nversion = \"1\"\n",
    );
    workspace.write(
        "lib/config.xiao",
        "[project]\nname = \"lib\"\nversion = \"1.2.4\"\n",
    );
    workspace.run(PackageOperation::Lock).unwrap();
    let config = fs::read(workspace.path("project/config.xiao")).unwrap();
    let lock = fs::read(workspace.path("project/xiao.lock.json")).unwrap();
    let error = workspace.edit(add("1.3")).unwrap_err();
    assert_eq!(error.code, "X05-VERSION-002");
    assert_eq!(
        fs::read(workspace.path("project/config.xiao")).unwrap(),
        config
    );
    assert_eq!(
        fs::read(workspace.path("project/xiao.lock.json")).unwrap(),
        lock
    );
}

#[test]
fn lock_never_replaces_stale_contents_but_update_does() {
    let workspace = Workspace::new();
    workspace.write(
        "project/config.xiao",
        "[project]\nname = \"app\"\nversion = \"1\"\n",
    );
    workspace.write("project/main.xiao", "return 1\n");
    workspace.run(PackageOperation::Lock).unwrap();
    let lock = fs::read(workspace.path("project/xiao.lock.json")).unwrap();
    workspace.write("project/main.xiao", "return 2\n");
    assert_eq!(
        workspace.run(PackageOperation::Lock).unwrap_err().code,
        "X05-LOCK-004"
    );
    assert_eq!(
        fs::read(workspace.path("project/xiao.lock.json")).unwrap(),
        lock
    );
    assert_eq!(
        workspace
            .run(PackageOperation::Update)
            .unwrap()
            .lock_status
            .as_deref(),
        Some("updated")
    );
    assert_ne!(
        fs::read(workspace.path("project/xiao.lock.json")).unwrap(),
        lock
    );
}

#[test]
fn explicit_wrong_source_is_never_silently_treated_as_local() {
    let workspace = Workspace::new();
    workspace.write("project/config.xiao", "[project]\nname = \"app\"\nversion = \"1\"\n[dependencies]\nlib = { path = \"../lib\", source = \"remote\" }\n");
    workspace.write(
        "lib/config.xiao",
        "[project]\nname = \"lib\"\nversion = \"1.2.4\"\n",
    );
    assert_eq!(
        workspace.run(PackageOperation::Lock).unwrap_err().code,
        "X05-SOURCE-005"
    );
    assert!(!workspace.path("project/xiao.lock.json").exists());
}
