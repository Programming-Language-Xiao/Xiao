//! 真实目录索引、受限 TAR 正文和锁定复用的端到端验证。

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use sha2::{Digest, Sha256};
use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_config::parse_config_project;
use xiao_package::{
    ArtifactReference, CacheLayout, CacheStore, IndexDependency, IndexPackage, PackageOperation,
    PackageShard, SnapshotManifest, SourceDescriptor, TRUST_ARTIFACT_CODE, apply_packages,
    environment_package_view, jcs_digest, lockfile_path, read_lockfile,
};
use xiao_source::SourceFile;

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("xiao-e3d1-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    fn layout(&self) -> CacheLayout {
        CacheLayout::from_xiao_home(Some(&self.path("home")), &self.0).unwrap()
    }

    fn write(&self, relative: &str, bytes: impl AsRef<[u8]>) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn config(&self, constraint: &str) {
        let location = self.path("registry").to_string_lossy().replace('\\', "/");
        self.write("project/config.xiao", format!(
            "[project]\nname = \"app\"\nversion = \"0.1.0\"\n[sources]\nmirror = {{ kind = \"path\", location = \"{location}\" }}\n[dependencies]\ndemo = {{ version = \"{constraint}\", source = \"mirror\" }}\n"
        ));
    }

    fn publish(&self, demo_version: &str, snapshot_id: &str) {
        let source = SourceDescriptor::new(
            "path",
            &self.path("registry").to_string_lossy().replace('\\', "/"),
            Some("mirror"),
            None,
            1,
        )
        .unwrap();
        let mut packages = BTreeMap::new();
        for (name, version, dependencies) in [
            (
                "demo",
                demo_version,
                vec![IndexDependency {
                    name: "helper".into(),
                    version: "1.*".into(),
                    source: Some("mirror".into()),
                }],
            ),
            ("helper", "1.0.0", Vec::new()),
        ] {
            let mut builder = tar::Builder::new(Vec::new());
            let content = format!("[project]\nname = \"{name}\"\nversion = \"{version}\"\n");
            let mut header = tar::Header::new_ustar();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "config.xiao", Cursor::new(content))
                .unwrap();
            let probe = b"write NO_SIDE_EFFECT if executed";
            let mut header = tar::Header::new_ustar();
            header.set_size(probe.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "install.xiao", Cursor::new(probe))
                .unwrap();
            let bytes = builder.into_inner().unwrap();
            self.write(&format!("registry/artifacts/{name}-{version}.tar"), &bytes);
            let package = IndexPackage {
                name: name.into(),
                version: version.into(),
                variant: "any".into(),
                withdrawn: false,
                dependencies,
                features: Vec::new(),
                target: None,
                abi: None,
                xiao_range: None,
                runtime_range: None,
                source_artifact: ArtifactReference {
                    location: format!("artifacts/{name}-{version}.tar"),
                    length: bytes.len() as u64,
                    digest: format!("{:x}", Sha256::digest(&bytes)),
                },
                binary_artifacts: Vec::new(),
            };
            packages.insert(name.to_owned(), package);
        }
        let shards = packages
            .into_iter()
            .map(|(name, package)| {
                let text = serde_json::to_string(&PackageShard {
                    protocol_version: 1,
                    source_id: source.source.source_id.clone(),
                    snapshot_id: snapshot_id.into(),
                    packages: vec![package],
                })
                .unwrap();
                self.write(&format!("registry/index/{name}.json"), &text);
                (name, jcs_digest(&text).unwrap())
            })
            .collect();
        self.write(
            "registry/snapshot.json",
            serde_json::to_vec(&SnapshotManifest {
                protocol_version: 1,
                source_id: source.source.source_id,
                snapshot_id: snapshot_id.into(),
                shards,
                mirrors: Vec::new(),
                expires_at: None,
                signature: None,
            })
            .unwrap(),
        );
    }

    fn run(
        &self,
        operation: PackageOperation,
    ) -> Result<xiao_package::PackageOperationResult, xiao_package::PackageSyncError> {
        let text = fs::read_to_string(self.path("project/config.xiao")).unwrap();
        let document = parse_config_project(&SourceFile::from_text(&text)).unwrap();
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
}

fn sync() -> PackageOperation {
    PackageOperation::Sync {
        keep_extra: false,
        locked: false,
        frozen: false,
    }
}

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

impl Drop for Workspace {
    fn drop(&mut self) {
        writable(&self.0);
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn remote_closure_resolves_transitive_packages_and_reuses_pinned_cache() {
    let workspace = Workspace::new();
    workspace.config("1.*");
    workspace.publish("1.0.0", "snapshot-one");
    let result = workspace.run(sync()).unwrap();
    assert_eq!(result.lock_status.as_deref(), Some("created"));
    let lock_path = lockfile_path(workspace.path("project"));
    let bytes = fs::read(&lock_path).unwrap();
    let lock = read_lockfile(&lock_path).unwrap();
    assert_eq!(lock.lock_version, 2);
    let demo = lock
        .packages
        .values()
        .find(|item| item.name == "demo")
        .unwrap();
    assert_eq!(demo.dependencies["helper"].target.version, "1.0.0");
    assert_ne!(
        demo.content_digest,
        demo.source_artifact.as_ref().unwrap().digest
    );
    let snapshot = &lock.source_snapshots[&demo.source.source_id];
    assert_eq!(snapshot.snapshot_id, "snapshot-one");
    let mappings = environment_package_view(result.environment_path).unwrap();
    assert_eq!(mappings.len(), 3);
    assert!(!workspace.path("project/NO_SIDE_EFFECT").exists());
    assert!(!workspace.path("NO_SIDE_EFFECT").exists());
    fs::remove_dir_all(workspace.path("registry")).unwrap();
    assert_eq!(
        workspace.run(sync()).unwrap().lock_status.as_deref(),
        Some("reused")
    );
    assert_eq!(fs::read(lock_path).unwrap(), bytes);
}

#[test]
fn locked_digest_overrides_index_and_never_becomes_package_missing() {
    let workspace = Workspace::new();
    workspace.config("1.*");
    workspace.publish("1.0.0", "snapshot-one");
    workspace.run(PackageOperation::Lock).unwrap();
    let lock_path = lockfile_path(workspace.path("project"));
    let mut lock = read_lockfile(&lock_path).unwrap();
    let package = lock
        .packages
        .values_mut()
        .find(|item| item.name == "demo")
        .unwrap();
    let digest = package.content_digest.clone();
    package.source_artifact.as_mut().unwrap().digest = "f".repeat(64);
    fs::write(&lock_path, lock.to_json()).unwrap();
    let object_path = CacheStore::open(workspace.layout())
        .unwrap()
        .layout()
        .source_object_path(&digest)
        .unwrap();
    writable(&object_path);
    fs::remove_dir_all(object_path).unwrap();
    let error = workspace.run(sync()).unwrap_err();
    assert_eq!(error.code, TRUST_ARTIFACT_CODE, "{error:?}");
    assert!(!workspace.path("project/.venv").exists());
}

#[test]
fn remote_archive_identity_must_match_the_index() {
    for config in [
        Some("[project]\nname = \"impostor\"\nversion = \"1.0.0\"\n"),
        Some("[project]\nname = \"demo\"\nversion = \"2.0.0\"\n"),
        Some("[project]\nname = 123\nversion = \"1.0.0\"\n"),
        None,
    ] {
        let workspace = Workspace::new();
        workspace.config("1.*");
        workspace.publish("1.0.0", "snapshot-one");
        let mut archive = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_ustar();
        let readme = b"safe archive";
        header.set_size(readme.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(&mut header, "README.md", Cursor::new(readme))
            .unwrap();
        if let Some(config) = config {
            let mut header = tar::Header::new_ustar();
            header.set_size(config.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive
                .append_data(&mut header, "config.xiao", Cursor::new(config))
                .unwrap();
        }
        let body = archive.into_inner().unwrap();
        workspace.write("registry/artifacts/demo-1.0.0.tar", &body);
        let shard_path = "registry/index/demo.json";
        let mut shard: PackageShard =
            serde_json::from_slice(&fs::read(workspace.path(shard_path)).unwrap()).unwrap();
        shard.packages[0].source_artifact.length = body.len() as u64;
        shard.packages[0].source_artifact.digest = format!("{:x}", Sha256::digest(&body));
        let shard_text = serde_json::to_string(&shard).unwrap();
        workspace.write(shard_path, &shard_text);
        let mut manifest: SnapshotManifest =
            serde_json::from_slice(&fs::read(workspace.path("registry/snapshot.json")).unwrap())
                .unwrap();
        manifest
            .shards
            .insert("demo".into(), jcs_digest(&shard_text).unwrap());
        workspace.write(
            "registry/snapshot.json",
            serde_json::to_vec(&manifest).unwrap(),
        );
        let error = workspace.run(sync()).unwrap_err();
        assert_eq!(error.code, TRUST_ARTIFACT_CODE, "{error:?}");
        assert!(!lockfile_path(workspace.path("project")).exists());
        assert!(!workspace.path("project/.venv").exists());
    }
}

#[test]
fn explicit_update_changes_version_but_regular_sync_does_not() {
    let workspace = Workspace::new();
    workspace.config("1.*");
    workspace.publish("1.0.0", "snapshot-one");
    workspace.run(sync()).unwrap();
    workspace.publish("1.2.0", "snapshot-two");
    workspace.run(sync()).unwrap();
    let lock_path = lockfile_path(workspace.path("project"));
    let locked = read_lockfile(&lock_path).unwrap();
    assert_eq!(
        locked
            .packages
            .values()
            .find(|item| item.name == "demo")
            .unwrap()
            .version,
        "1.0.0"
    );
    workspace.run(PackageOperation::Update).unwrap();
    let updated = read_lockfile(&lock_path).unwrap();
    assert_eq!(
        updated
            .packages
            .values()
            .find(|item| item.name == "demo")
            .unwrap()
            .version,
        "1.2.0"
    );
}

#[test]
fn remote_closure_spec_vectors_are_executed() {
    let valid: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11a-remote-closure/valid.json"
    )))
    .unwrap();
    for case in valid["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let workspace = Workspace::new();
        workspace.config(case["constraint"].as_str().unwrap());
        workspace.publish(case["published"].as_str().unwrap(), "snapshot-one");
        workspace
            .run(sync())
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let locked = read_lockfile(lockfile_path(workspace.path("project"))).unwrap();
        assert_eq!(
            locked
                .packages
                .values()
                .find(|package| package.name == "demo")
                .unwrap()
                .version,
            case["selected"].as_str().unwrap(),
            "{name}"
        );
        assert!(!workspace.path("project/NO_SIDE_EFFECT").exists(), "{name}");
    }
    let errors: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/11a-remote-closure/errors.json"
    )))
    .unwrap();
    for case in errors["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let workspace = Workspace::new();
        workspace.config(case["constraint"].as_str().unwrap());
        workspace.publish(case["published"].as_str().unwrap(), "snapshot-one");
        let path = "registry/index/demo.json";
        let mut shard: PackageShard =
            serde_json::from_slice(&fs::read(workspace.path(path)).unwrap()).unwrap();
        shard.packages[0].target = Some(case["target"].as_str().unwrap().into());
        let text = serde_json::to_string(&shard).unwrap();
        workspace.write(path, &text);
        let mut manifest: SnapshotManifest =
            serde_json::from_slice(&fs::read(workspace.path("registry/snapshot.json")).unwrap())
                .unwrap();
        manifest
            .shards
            .insert("demo".into(), jcs_digest(&text).unwrap());
        workspace.write(
            "registry/snapshot.json",
            serde_json::to_vec(&manifest).unwrap(),
        );
        let error = workspace.run(sync()).unwrap_err();
        assert_eq!(error.code, case["diagnostic"].as_str().unwrap(), "{name}");
        assert!(error.message.contains("条件"), "{name}: {error}");
        assert!(!workspace.path("project/.venv").exists(), "{name}");
    }
}
