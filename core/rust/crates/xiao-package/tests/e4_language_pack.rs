//! 11C-2 资源型语言包与 11A 不可变缓存的集成回归。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use xiao_i18n::digest_resources;
use xiao_package::{
    CacheError, CacheLayout, CacheStore, LANGUAGE_PACK_OBJECT_KIND, LanguagePackCacheError,
};

static NEXT_WORKSPACE: AtomicU64 = AtomicU64::new(0);

struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xiao-e4-language-pack-{stamp}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create cache fixture");
        Self { root }
    }

    fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative)
    }

    fn cache(&self) -> CacheStore {
        let home = self.path("xiao-home");
        let layout = CacheLayout::from_xiao_home(Some(&home), &self.root).expect("cache layout");
        CacheStore::open(layout).expect("cache store")
    }

    fn write_pack(&self, plugin_id: &str, text: &str) -> PathBuf {
        let root = self.path(plugin_id);
        let relative_path = "catalogs/fr-FR.json".to_owned();
        let catalog = serde_json::to_vec(&json!({
            "version": 1,
            "locale": "fr-FR",
            "entries": [{ "id": "plugin.ready", "text": text, "params": {} }],
        }))
        .expect("catalog JSON");
        let resources = BTreeMap::from([(relative_path.clone(), catalog.clone())]);
        let (content_length, sha256) = digest_resources(&resources);
        let manifest = json!({
            "manifest_version": 1,
            "plugin_id": plugin_id,
            "package_version": "1.0.0",
            "catalogs": { "fr-FR": relative_path },
            "catalog_version": 1,
            "xiao_version": "*",
            "runtime_abi": "*",
            "content_length": content_length,
            "sha256": sha256,
        });
        fs::create_dir_all(root.join("catalogs")).expect("catalog directory");
        fs::write(
            root.join("xiao-language-pack.json"),
            serde_json::to_vec(&manifest).expect("manifest JSON"),
        )
        .expect("manifest");
        fs::write(root.join(relative_path), catalog).expect("catalog");
        root
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        make_writable(&self.root);
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn make_writable(root: &Path) {
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return;
    };
    if metadata.file_type().is_symlink() {
        return;
    }
    if metadata.is_dir() {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                make_writable(&entry.path());
            }
        }
    }
    let mut permissions = metadata.permissions();
    #[cfg(not(unix))]
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(if metadata.is_dir() { 0o755 } else { 0o644 });
    }
    let _ = fs::set_permissions(root, permissions);
}

#[test]
fn language_objects_are_isolated_read_only_and_reusable() {
    let workspace = Workspace::new();
    let pack_root = workspace.write_pack("org.example.cache", "ready");
    let cache = workspace.cache();
    let object = cache
        .import_language_pack_directory(&pack_root)
        .expect("import language pack");

    let relative = object
        .path
        .strip_prefix(cache.layout().language_pack_objects_root())
        .expect("language object under language root")
        .to_string_lossy()
        .replace('\\', "/");
    assert!(relative.starts_with("catalog-v1/xiao-v"));
    assert!(relative.contains(&format!("/abi-{}/", object.key.runtime_abi)));
    assert!(relative.ends_with(&object.key.content_digest));
    assert!(
        object.path.starts_with(
            cache
                .layout()
                .cache_root()
                .join("objects")
                .join(LANGUAGE_PACK_OBJECT_KIND)
        )
    );
    assert!(
        fs::metadata(object.path.join("xiao-language-pack.json"))
            .expect("manifest metadata")
            .permissions()
            .readonly()
    );
    assert!(
        fs::metadata(object.path.join("catalogs/fr-FR.json"))
            .expect("catalog metadata")
            .permissions()
            .readonly()
    );

    let second_cache = CacheStore::open(cache.layout().clone()).expect("second cache instance");
    let reused = second_cache
        .import_language_pack_directory(&pack_root)
        .expect("reuse language pack");
    assert_eq!(reused.path, object.path);
    assert_eq!(reused.key, object.key);
    let loaded = second_cache
        .load_language_pack_object(&object.key)
        .expect("load verified object");
    assert_eq!(loaded.manifest().plugin_id, "org.example.cache");
}

#[test]
fn corrupt_language_object_is_rejected_and_quarantined_without_touching_source_cache() {
    let workspace = Workspace::new();
    let pack_root = workspace.write_pack("org.example.corrupt", "ready");
    let source_root = workspace.path("source");
    fs::create_dir_all(&source_root).expect("source root");
    fs::write(
        source_root.join("config.xiao"),
        "[project]\nname = \"demo\"\n",
    )
    .expect("source file");

    let cache = workspace.cache();
    let language_object = cache
        .import_language_pack_directory(&pack_root)
        .expect("language object");
    let source_object = cache
        .import_source_directory(&source_root)
        .expect("source object");
    assert!(
        !source_object.path.starts_with(
            cache
                .layout()
                .cache_root()
                .join("objects")
                .join(LANGUAGE_PACK_OBJECT_KIND)
        )
    );
    assert!(source_object.path.exists());

    make_writable(&language_object.path);
    fs::write(
        language_object.path.join("catalogs/fr-FR.json"),
        br#"{"version":1,"locale":"fr-FR","entries":[{"id":"plugin.ready","text":"tampered","params":{}}]}"#,
    )
    .expect("tamper language object");
    let error = cache
        .verify_language_pack_object(&language_object.key)
        .expect_err("tampered object must reject");
    assert!(matches!(
        error,
        LanguagePackCacheError::Cache(CacheError::ObjectCorrupt {
            quarantine: Some(_),
            ..
        })
    ));
    assert!(!language_object.path.exists());
    assert!(source_object.path.exists());
}
