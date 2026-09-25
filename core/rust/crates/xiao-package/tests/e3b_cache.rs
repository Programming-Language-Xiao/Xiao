//! 11A-E3B：真实本地源驱动的快照、四类缓存、离线判定及进程锁。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use sha2::{Digest, Sha256};
use xiao_config::parse_config_text;
use xiao_package::{
    ArtifactReference, CacheLayout, CacheStore, ConfiguredSource, FederationCache, IndexPackage,
    IndexSnapshot, LOCKFILE_VERSION, LocalDirectoryAdapter, LockFile, LockedPackage,
    MAX_PARALLEL_SOURCES, MetadataCache, PackageIdentity, PackageShard, PackageSourceAdapter,
    SOURCE_CACHE_CORRUPT_CODE, SOURCE_CACHE_IO_CODE, SOURCE_SNAPSHOT_OWNER_CODE,
    SOURCE_UNAVAILABLE_CODE, SnapshotManifest, SnapshotStatus, SnapshotStore, SourceDescriptor,
    SourceError, SourceResolver, fingerprint_config, jcs_digest, select_source,
    source_list_fingerprint,
};

/// 防止并行测试在同一毫秒使用相同隔离路径。
static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

/// 每例拥有独立的本地源及 XIAO_HOME，清理时验证实际目录仍在测试专用根下。
struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new() -> Self {
        let base = std::env::temp_dir().join("xiao-e3b-tests");
        fs::create_dir_all(&base).unwrap();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = base.join(format!(
            "{}-{nonce}-{}",
            std::process::id(),
            NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }

    fn cache(&self) -> CacheStore {
        CacheStore::open(self.layout()).unwrap()
    }

    fn layout(&self) -> CacheLayout {
        let home = self.root.join("home");
        CacheLayout::from_xiao_home(Some(&home), &self.root).unwrap()
    }

    fn source(&self, name: &str, version: &str, order: usize) -> ConfiguredSource {
        let path = self.root.join(name);
        fs::create_dir_all(path.join("index")).unwrap();
        fs::create_dir_all(path.join("artifacts")).unwrap();
        let descriptor = SourceDescriptor::new(
            "path",
            &path.to_string_lossy().replace('\\', "/"),
            Some(name),
            None,
            1,
        )
        .unwrap();
        let body = b"body";
        let artifact = ArtifactReference {
            location: "artifacts/demo".to_owned(),
            length: body.len() as u64,
            digest: format!("{:x}", Sha256::digest(body)),
        };
        fs::write(path.join("artifacts/demo"), body).unwrap();
        let shard = PackageShard {
            protocol_version: 1,
            source_id: descriptor.source.source_id.clone(),
            snapshot_id: "one".to_owned(),
            packages: vec![IndexPackage {
                name: "demo".to_owned(),
                version: version.to_owned(),
                variant: "any".to_owned(),
                withdrawn: false,
                dependencies: Vec::new(),
                features: Vec::new(),
                target: None,
                abi: None,
                xiao_range: None,
                runtime_range: None,
                source_artifact: artifact,
                binary_artifacts: Vec::new(),
            }],
        };
        let shard_text = serde_json::to_string(&shard).unwrap();
        fs::write(path.join("index/demo.json"), &shard_text).unwrap();
        let manifest = SnapshotManifest {
            protocol_version: 1,
            source_id: descriptor.source.source_id.clone(),
            snapshot_id: "one".to_owned(),
            shards: BTreeMap::from([("demo".to_owned(), jcs_digest(&shard_text).unwrap())]),
            mirrors: Vec::new(),
            expires_at: None,
            signature: None,
        };
        fs::write(
            path.join("snapshot.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        ConfiguredSource {
            descriptor,
            config_order: order,
            imported_from: None,
        }
    }

    fn config(&self, template: &str) -> xiao_config::ConfigDocument {
        parse_config_text(
            &template
                .replace(
                    "{FIRST}",
                    &self.root.join("first").to_string_lossy().replace('\\', "/"),
                )
                .replace(
                    "{SECOND}",
                    &self
                        .root
                        .join("second")
                        .to_string_lossy()
                        .replace('\\', "/"),
                ),
        )
        .unwrap()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let base = std::env::temp_dir().join("xiao-e3b-tests");
        if let (Ok(root), Ok(base)) = (self.root.canonicalize(), base.canonicalize()) {
            if root.parent() == Some(base.as_path()) {
                let _ = fs::remove_dir_all(&self.root);
            }
        }
    }
}

/// 从已导入的 E1 源码对象制作结构上有效的单包锁文件。
fn locked(
    source: &ConfiguredSource,
    cache: &CacheStore,
    document: &xiao_config::ConfigDocument,
) -> LockFile {
    let identity = PackageIdentity {
        name: "demo".to_owned(),
        version: "1.0".to_owned(),
        source: source.descriptor.source.clone(),
    };
    let digest = cache
        .import_source_directory(Path::new(&source.descriptor.location))
        .unwrap()
        .reference
        .digest;
    let package = LockedPackage {
        name: identity.name.clone(),
        version: identity.version.clone(),
        source: identity.source.clone(),
        content_digest: digest,
        dependencies: BTreeMap::new(),
        precompiled_variants: Vec::new(),
        target_conditions: Vec::new(),
    };
    LockFile {
        lock_version: LOCKFILE_VERSION,
        config_fingerprint: fingerprint_config(document),
        root: identity.clone(),
        packages: BTreeMap::from([(identity.to_string(), package)]),
    }
}

#[test]
/// 真实读取双份规格输入并验证四类目录、首源选择及源移走后的离线命中。
fn cache_policy_fixture_offline_fast_path() {
    let valid: Value = serde_json::from_str(include_str!(
        "../../../../../tests/spec/11a-cache-policy/valid.json"
    ))
    .unwrap();
    let workspace = Workspace::new();
    let versions = valid["versions"].as_array().unwrap();
    let sources = [
        workspace.source("first", versions[0].as_str().unwrap(), 0),
        workspace.source("second", versions[1].as_str().unwrap(), 1),
    ];
    let document = workspace.config(valid["config_template"].as_str().unwrap());
    let cache = workspace.cache();
    let lockfile = locked(&sources[0], &cache, &document);
    let resolver = SourceResolver::new(LocalDirectoryAdapter, cache);
    let names = [valid["package"].as_str().unwrap().to_owned()];
    let first = resolver
        .resolve(&document, &sources, None, &names, false)
        .unwrap();
    assert!(!first.fast_path);
    assert_eq!(first.reports[0].status, SnapshotStatus::Fresh);
    assert_eq!(valid["expected_online_status"], "fresh");
    assert_eq!(
        select_source(
            &sources,
            &first.snapshots,
            &first.records,
            "demo",
            None,
            |_| true
        )
        .unwrap()[0]
            .key
            .config_order,
        valid["expected_selected_order"].as_u64().unwrap() as usize
    );
    let layout = workspace.layout();
    assert!(layout.source_objects_root().exists());
    assert!(layout.metadata_objects_root().exists());
    assert!(layout.snapshots_root().exists());
    assert!(layout.federation_root().join("index.json").exists());
    fs::rename(
        workspace.root.join("first"),
        workspace.root.join("first-offline"),
    )
    .unwrap();
    fs::rename(
        workspace.root.join("second"),
        workspace.root.join("second-offline"),
    )
    .unwrap();
    let cached = resolver
        .resolve(&document, &sources, Some(&lockfile), &names, true)
        .unwrap();
    assert!(cached.fast_path);
    assert_eq!(cached.reports[0].status, SnapshotStatus::Cached);
    assert_eq!(valid["expected_offline_status"], "cached");
    assert!(cached.reports[0].observed_at_ms.is_some());
    assert_eq!(
        cached.records[0].snapshot_digest,
        first.records[0].snapshot_digest
    );
    let cached_again = resolver
        .resolve(&document, &sources, None, &names, true)
        .unwrap();
    assert!(!cached_again.fast_path);
    assert_eq!(cached_again.reports[0].status, SnapshotStatus::Cached);
}

#[test]
/// 第一源无法验证且无旧快照时不能偷偷选择第二源。
fn missing_priority_source_is_not_an_empty_source() {
    let errors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../tests/spec/11a-cache-policy/errors.json"
    ))
    .unwrap();
    for case in errors {
        let workspace = Workspace::new();
        let versions = case["versions"].as_array().unwrap();
        let sources = [
            workspace.source("first", versions[0].as_str().unwrap(), 0),
            workspace.source("second", versions[1].as_str().unwrap(), 1),
        ];
        let missing = case["unavailable_order"].as_u64().unwrap() as usize;
        assert_eq!(
            sources[missing].descriptor.source.alias.as_deref(),
            case["expected_source"].as_str()
        );
        fs::rename(
            workspace
                .root
                .join(case["expected_source"].as_str().unwrap()),
            workspace.root.join("unavailable"),
        )
        .unwrap();
        let document =
            parse_config_text("[project]\nname = \"demo\"\nversion = \"1.0\"\n").unwrap();
        let resolver = SourceResolver::new(LocalDirectoryAdapter, workspace.cache());
        let failure = resolver
            .resolve(
                &document,
                &sources,
                None,
                &[case["package"].as_str().unwrap().to_owned()],
                false,
            )
            .unwrap_err();
        assert_eq!(failure.code, case["expected_code"].as_str().unwrap());
        assert!(
            failure
                .message
                .contains(&sources[missing].descriptor.source.source_id)
        );
    }
}

#[test]
fn unavailable_later_source_does_not_block_prior_candidate() {
    let workspace = Workspace::new();
    let sources = [
        workspace.source("first", "1.0", 0),
        workspace.source("second", "9.0", 1),
    ];
    fs::rename(
        workspace.root.join("second"),
        workspace.root.join("offline-second"),
    )
    .unwrap();
    let document = parse_config_text("[project]\nname = \"demo\"\nversion = \"1.0\"\n").unwrap();
    let resolver = SourceResolver::new(LocalDirectoryAdapter, workspace.cache());
    let result = resolver
        .resolve(&document, &sources, None, &["demo".to_owned()], false)
        .unwrap();
    assert_eq!(result.reports[0].status, SnapshotStatus::Fresh);
    assert_eq!(result.reports[1].status, SnapshotStatus::Unavailable);
    assert_eq!(
        select_source(
            &sources,
            &result.snapshots,
            &result.records,
            "demo",
            None,
            |_| true
        )
        .unwrap()[0]
            .key
            .config_order,
        0
    );
}

#[test]
fn cache_infrastructure_failure_does_not_become_source_unavailable() {
    let workspace = Workspace::new();
    let source = workspace.source("first", "1.0", 0);
    let index_path = workspace.layout().federation_root().join("index.json");
    fs::create_dir_all(index_path).unwrap();
    let resolver = SourceResolver::new(LocalDirectoryAdapter, workspace.cache());
    let document = parse_config_text("[project]\nname = \"demo\"\nversion = \"1.0\"\n").unwrap();
    let error = resolver
        .resolve(&document, &[source], None, &["demo".to_owned()], false)
        .unwrap_err();
    assert_eq!(error.code, SOURCE_CACHE_IO_CODE);
}

#[test]
fn changed_config_and_snapshot_refresh_instead_of_reusing_old_lock() {
    let workspace = Workspace::new();
    let source = workspace.source("first", "1.0", 0);
    let first = parse_config_text("[project]\nname = \"demo\"\nversion = \"1.0\"\n").unwrap();
    let second = parse_config_text("[project]\nname = \"demo\"\nversion = \"2.0\"\n").unwrap();
    let cache = workspace.cache();
    let lockfile = locked(&source, &cache, &first);
    let resolver = SourceResolver::new(LocalDirectoryAdapter, cache);
    let names = ["demo".to_owned()];
    resolver
        .resolve(&first, std::slice::from_ref(&source), None, &names, false)
        .unwrap();

    let directory = workspace.root.join("first");
    let shard_path = directory.join("index/demo.json");
    let mut shard: PackageShard = serde_json::from_slice(&fs::read(&shard_path).unwrap()).unwrap();
    shard.snapshot_id = "two".to_owned();
    shard.packages[0].version = "2.0".to_owned();
    let shard_text = serde_json::to_string(&shard).unwrap();
    fs::write(shard_path, &shard_text).unwrap();
    let manifest_path = directory.join("snapshot.json");
    let mut manifest: SnapshotManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.snapshot_id = "two".to_owned();
    manifest
        .shards
        .insert("demo".to_owned(), jcs_digest(&shard_text).unwrap());
    fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    let result = resolver
        .resolve(
            &second,
            std::slice::from_ref(&source),
            Some(&lockfile),
            &names,
            false,
        )
        .unwrap();
    assert!(!result.fast_path);
    assert_eq!(result.reports[0].status, SnapshotStatus::Fresh);
    assert_eq!(result.records[0].package.version, "2.0");
    assert_eq!(result.snapshots[0].snapshot_id.as_deref(), Some("two"));
}

#[test]
fn lock_from_later_source_cannot_skip_priority_validation() {
    let workspace = Workspace::new();
    let sources = [
        workspace.source("first", "1.0", 0),
        workspace.source("second", "1.0", 1),
    ];
    let document = parse_config_text("[project]\nname = \"demo\"\nversion = \"1.0\"\n").unwrap();
    let cache = workspace.cache();
    let lockfile = locked(&sources[1], &cache, &document);
    let resolver = SourceResolver::new(LocalDirectoryAdapter, cache);
    let names = ["demo".to_owned()];
    resolver
        .resolve(&document, &sources, None, &names, false)
        .unwrap();
    let result = resolver
        .resolve(&document, &sources, Some(&lockfile), &names, true)
        .unwrap();
    assert!(!result.fast_path);
    assert_eq!(
        select_source(
            &sources,
            &result.snapshots,
            &result.records,
            "demo",
            None,
            |_| true
        )
        .unwrap()[0]
            .key
            .config_order,
        0
    );
}

#[test]
/// 快照不可变版本和当前指向分离，摘要或归属不对时不覆盖已验证旧值。
fn snapshot_history_and_metadata_integrity() {
    let workspace = Workspace::new();
    let source = workspace.source("first", "1.0", 0);
    let layout = workspace.layout();
    let adapter = LocalDirectoryAdapter;
    let mut index = adapter.read_snapshot(&source.descriptor).unwrap();
    let store = SnapshotStore::new(layout.clone());
    assert!(store.current(&index.manifest.source_id).unwrap().is_none());
    store.save(&index).unwrap();
    let source_digest = format!("{:x}", Sha256::digest(index.manifest.source_id.as_bytes()));
    let directory = layout.snapshots_root().join(source_digest);
    index.manifest.snapshot_id = "two".to_owned();
    index.digest = jcs_digest(&serde_json::to_string(&index.manifest).unwrap()).unwrap();
    store.save(&index).unwrap();
    assert!(directory.join("one.json").exists());
    assert_eq!(
        store
            .current(&index.manifest.source_id)
            .unwrap()
            .unwrap()
            .index,
        index
    );
    index.manifest.snapshot_id = "../unsafe".to_owned();
    assert!(store.save(&index).is_err());
    assert_eq!(
        store
            .current(&source.descriptor.source.source_id)
            .unwrap()
            .unwrap()
            .index
            .manifest
            .snapshot_id,
        "two"
    );
    let other = workspace.source("second", "1.0", 1);
    let other_index = adapter.read_snapshot(&other.descriptor).unwrap();
    let snapshot_path = directory.join("two.json");
    let original = fs::read(&snapshot_path).unwrap();
    let other_snapshot = serde_json::json!({
        "index": {"manifest": other_index.manifest, "digest": other_index.digest},
        "observed_at_ms": store.current(&source.descriptor.source.source_id).unwrap().unwrap().observed_at_ms
    });
    fs::write(&snapshot_path, serde_json::to_vec(&other_snapshot).unwrap()).unwrap();
    assert_eq!(
        store
            .current(&source.descriptor.source.source_id)
            .unwrap_err()
            .code,
        SOURCE_SNAPSHOT_OWNER_CODE
    );
    fs::write(&snapshot_path, original).unwrap();
    fs::remove_file(&snapshot_path).unwrap();
    assert_eq!(
        store
            .current(&source.descriptor.source.source_id)
            .unwrap_err()
            .code,
        SOURCE_CACHE_CORRUPT_CODE
    );

    let metadata = MetadataCache::new(layout.clone());
    let first = adapter
        .read_package(
            &source.descriptor,
            &adapter.read_snapshot(&source.descriptor).unwrap(),
            "demo",
        )
        .unwrap();
    let digest = metadata.store(&first[0]).unwrap();
    assert_eq!(metadata.store(&first[0]).unwrap(), digest);
    let second = adapter
        .read_package(
            &other.descriptor,
            &adapter.read_snapshot(&other.descriptor).unwrap(),
            "demo",
        )
        .unwrap();
    assert_eq!(metadata.store(&second[0]).unwrap(), digest);
    fs::write(
        layout
            .metadata_objects_root()
            .join(&digest[..2])
            .join(&digest)
            .join("metadata.json"),
        b"{}",
    )
    .unwrap();
    assert_eq!(
        metadata.read(&digest).unwrap_err().code,
        SOURCE_CACHE_CORRUPT_CODE
    );
}

#[test]
/// 源数超过八个时分批执行，快源不能抢占最前源的合并顺序。
fn parallel_reads_are_bounded_and_ordered() {
    let workspace = Workspace::new();
    let sources = (0..MAX_PARALLEL_SOURCES + 2)
        .map(|order| workspace.source(&format!("source-{order}"), &format!("{order}.0"), order))
        .collect::<Vec<_>>();
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let adapter = DelayAdapter {
        active: active.clone(),
        peak: peak.clone(),
        fail_package_once: AtomicBool::new(false),
    };
    let resolver = SourceResolver::new(adapter, workspace.cache());
    let document = parse_config_text("[project]\nname = \"demo\"\nversion = \"1.0\"\n").unwrap();
    let resolution = resolver
        .resolve(&document, &sources, None, &["demo".to_owned()], false)
        .unwrap();
    assert_eq!(
        resolution
            .snapshots
            .iter()
            .map(|item| item.config_order)
            .collect::<Vec<_>>(),
        (0..sources.len()).collect::<Vec<_>>()
    );
    assert_eq!(resolution.records[0].package.version, "0.0");
    assert!(peak.load(Ordering::Relaxed) <= MAX_PARALLEL_SOURCES);
    assert!(peak.load(Ordering::Relaxed) > 1);
}

/// 只注入延迟与一次失败，真实索引仍由目录适配器读取。
struct DelayAdapter {
    active: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    fail_package_once: AtomicBool,
}

impl PackageSourceAdapter for DelayAdapter {
    fn read_snapshot(&self, source: &SourceDescriptor) -> Result<IndexSnapshot, SourceError> {
        let count = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(count, Ordering::SeqCst);
        if source.location.ends_with("source-0") {
            thread::sleep(Duration::from_millis(60));
        } else {
            thread::sleep(Duration::from_millis(12));
        }
        let result = LocalDirectoryAdapter.read_snapshot(source);
        self.active.fetch_sub(1, Ordering::SeqCst);
        result
    }

    fn read_package(
        &self,
        source: &SourceDescriptor,
        index: &IndexSnapshot,
        name: &str,
    ) -> Result<Vec<IndexPackage>, SourceError> {
        if name == "other" && self.fail_package_once.swap(false, Ordering::SeqCst) {
            return Err(SourceError {
                code: SOURCE_UNAVAILABLE_CODE,
                message: "分片读取中断".to_owned(),
            });
        }
        LocalDirectoryAdapter.read_package(source, index, name)
    }

    fn read_artifact(
        &self,
        source: &SourceDescriptor,
        artifact: &ArtifactReference,
    ) -> Result<Vec<u8>, SourceError> {
        LocalDirectoryAdapter.read_artifact(source, artifact)
    }
}

#[test]
/// 读取第二片失败不会持久化先读到的候选；重试才发布完整缓存。
fn partial_read_failure_leaves_no_snapshot_or_metadata() {
    let workspace = Workspace::new();
    let source = workspace.source("first", "1.0", 0);
    let adapter = DelayAdapter {
        active: Arc::new(AtomicUsize::new(0)),
        peak: Arc::new(AtomicUsize::new(0)),
        fail_package_once: AtomicBool::new(true),
    };
    let resolver = SourceResolver::new(adapter, workspace.cache());
    let document = parse_config_text("[project]\nname = \"demo\"\nversion = \"1.0\"\n").unwrap();
    let requested = ["demo".to_owned(), "other".to_owned()];
    assert_eq!(
        resolver
            .resolve(
                &document,
                std::slice::from_ref(&source),
                None,
                &requested,
                false
            )
            .unwrap_err()
            .code,
        SOURCE_UNAVAILABLE_CODE
    );
    assert!(
        SnapshotStore::new(workspace.layout())
            .current(&source.descriptor.source.source_id)
            .unwrap()
            .is_none()
    );
    assert!(!workspace.layout().metadata_objects_root().exists());
    assert!(
        resolver
            .resolve(&document, &[source], None, &requested, false)
            .is_ok()
    );
}

#[test]
/// 进程子测试通过环境变量拿到同一隔离源，正常测试入口不作任何修改。
fn process_child() {
    let Ok(root) = std::env::var("XIAO_E3B_CHILD_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let source = SourceDescriptor::new(
        "path",
        &root.join("source").to_string_lossy().replace('\\', "/"),
        None,
        None,
        1,
    )
    .unwrap();
    let index = LocalDirectoryAdapter.read_snapshot(&source).unwrap();
    let home = root.join("home");
    let layout = CacheLayout::from_xiao_home(Some(&home), &root).unwrap();
    SnapshotStore::new(layout).save(&index).unwrap();
}

#[test]
/// 两个真实进程争同一快照条目，崩溃进程留下的锁按 PID 判活后回收。
fn parallel_process_lock_and_stale_owner_recovery() {
    let workspace = Workspace::new();
    let source = workspace.source("source", "1.0", 0);
    let exe = std::env::current_exe().unwrap();
    let mut children = (0..2)
        .map(|_| {
            Command::new(&exe)
                .args(["--exact", "process_child", "--nocapture"])
                .env("XIAO_E3B_CHILD_ROOT", &workspace.root)
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    let layout = workspace.layout();
    let store = SnapshotStore::new(layout.clone());
    let index = LocalDirectoryAdapter
        .read_snapshot(&source.descriptor)
        .unwrap();
    assert_eq!(
        store
            .current(&index.manifest.source_id)
            .unwrap()
            .unwrap()
            .index,
        index
    );
    let directory = layout.snapshots_root().join(format!(
        "{:x}",
        Sha256::digest(index.manifest.source_id.as_bytes())
    ));
    assert!(!directory.join("current.lock").exists());
    fs::write(
        directory.join("current.lock"),
        r#"{"pid":999999,"created_at_ms":1,"nonce":7}"#,
    )
    .unwrap();
    store.save(&index).unwrap();
    assert!(!directory.join("current.lock").exists());
    let federation = FederationCache::new(layout);
    assert!(
        federation
            .read("unknown", &source_list_fingerprint(&[source]))
            .unwrap()
            .is_none()
    );
}
