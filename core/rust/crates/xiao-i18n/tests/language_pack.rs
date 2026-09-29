//! L3 资源型语言包插件的清单、冲突、摘要和降级回归。

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use xiao_i18n::{
    Fallback, LanguagePack, LanguagePackActivation, LanguagePackCacheKey,
    LanguagePackCompatibility, LanguagePackError, LanguagePackRegistry, LanguagePackRuntime,
    LocaleContext, MessageParam, digest_resources,
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct PackFixture {
    root: PathBuf,
}

impl PackFixture {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xiao-language-pack-{stamp}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create language pack fixture");
        Self { root }
    }

    fn write_pack(
        &self,
        plugin_id: &str,
        locale: &str,
        entries: Vec<Value>,
        xiao_version: &str,
    ) -> PathBuf {
        let relative_path = format!("catalogs/{locale}.json");
        let catalog = serde_json::to_vec(&json!({
            "version": 1,
            "locale": locale,
            "entries": entries,
        }))
        .expect("serialize catalog");
        let resources = BTreeMap::from([(relative_path.clone(), catalog.clone())]);
        let (content_length, sha256) = digest_resources(&resources);
        let manifest = json!({
            "manifest_version": 1,
            "plugin_id": plugin_id,
            "package_version": "1.0.0",
            "catalogs": { locale: relative_path },
            "catalog_version": 1,
            "xiao_version": xiao_version,
            "runtime_abi": "*",
            "content_length": content_length,
            "sha256": sha256,
        });
        fs::create_dir_all(self.root.join("catalogs")).expect("create catalog directory");
        fs::write(
            self.root.join("xiao-language-pack.json"),
            serde_json::to_vec(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");
        fs::write(self.root.join(relative_path), catalog).expect("write catalog");
        self.root.clone()
    }

    fn write_extra(&self, relative_path: &str, content: &str) {
        let path = self.root.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create extra resource directory");
        }
        fs::write(path, content).expect("write extra resource");
    }
}

impl Drop for PackFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn entry(id: &str, text: &str, params: Value) -> Value {
    json!({ "id": id, "text": text, "params": params })
}

fn current_compatibility() -> LanguagePackCompatibility {
    LanguagePackCompatibility::current()
}

#[test]
fn valid_pack_registers_new_locale_and_keeps_fallback_chain() {
    let fixture = PackFixture::new();
    let root = fixture.write_pack(
        "org.example.french",
        "fr-FR",
        vec![
            entry("xiao.status.cancelled", "demande annulée", json!({})),
            entry(
                "plugin.greeting",
                "bonjour {name}",
                json!({ "name": "text" }),
            ),
        ],
        "*",
    );
    let pack = LanguagePack::load_directory(&root, &current_compatibility()).expect("valid pack");
    assert_eq!(pack.manifest().plugin_id, "org.example.french");
    assert_eq!(pack.content_length(), pack.manifest().content_length);
    assert!(
        pack.cache_key()
            .expect("cache key")
            .relative_path()
            .contains("catalog-v1/")
    );

    let mut registry = LanguagePackRegistry::new();
    registry.register(&pack).expect("register new locale");
    let renderer = registry.renderer().expect("renderer");
    let greeting = renderer.render(
        &LocaleContext::new("fr-FR"),
        "plugin.greeting",
        &BTreeMap::from([(String::from("name"), MessageParam::Text("星崽".to_owned()))]),
    );
    assert_eq!(greeting.text, "bonjour 星崽");
    assert_eq!(greeting.fallback, Fallback::Exact);

    let missing_translation = renderer.render(
        &LocaleContext::new("fr-FR"),
        "xiao.status.ready",
        &BTreeMap::new(),
    );
    assert_eq!(missing_translation.text, "ready");
    assert_eq!(missing_translation.fallback, Fallback::English);
}

#[test]
fn digest_and_compatibility_errors_are_rejected() {
    let fixture = PackFixture::new();
    let root = fixture.write_pack(
        "org.example.bad-digest",
        "fr-FR",
        vec![entry("plugin.ready", "ready", json!({}))],
        "*",
    );
    fs::write(
        root.join("catalogs/fr-FR.json"),
        br#"{"version":1,"locale":"fr-FR","entries":[{"id":"plugin.ready","text":"changed","params":{}}]}"#,
    )
    .expect("change resource");
    let error = LanguagePack::load_directory(&root, &current_compatibility())
        .expect_err("digest mismatch must reject");
    assert!(matches!(error, LanguagePackError::DigestMismatch { .. }));

    let incompatible = PackFixture::new();
    let incompatible_root = incompatible.write_pack(
        "org.example.future",
        "fr-FR",
        vec![entry("plugin.ready", "ready", json!({}))],
        "^999.0.0",
    );
    let error = LanguagePack::load_directory(&incompatible_root, &current_compatibility())
        .expect_err("incompatible pack must reject");
    assert!(matches!(error, LanguagePackError::Compatibility { .. }));
}

#[test]
fn only_declared_json_resources_are_allowed_and_manifest_is_strict() {
    let fixture = PackFixture::new();
    let root = fixture.write_pack(
        "org.example.resource-only",
        "fr-FR",
        vec![entry("plugin.ready", "ready", json!({}))],
        "*",
    );
    fixture.write_extra("init.xiao", "this must never execute");
    let error = LanguagePack::load_directory(&root, &current_compatibility())
        .expect_err("extra executable-looking resource must reject");
    assert!(matches!(error, LanguagePackError::Resource { .. }));

    let strict = PackFixture::new();
    let strict_root = strict.write_pack(
        "org.example.strict",
        "fr-FR",
        vec![entry("plugin.ready", "ready", json!({}))],
        "*",
    );
    let mut manifest: Value = serde_json::from_slice(
        &fs::read(strict_root.join("xiao-language-pack.json")).expect("read manifest"),
    )
    .expect("parse manifest fixture");
    manifest["entrypoint"] = Value::String("init.xiao".to_owned());
    fs::write(
        strict_root.join("xiao-language-pack.json"),
        serde_json::to_vec(&manifest).expect("serialize invalid manifest"),
    )
    .expect("write invalid manifest");
    let error = LanguagePack::load_directory(&strict_root, &current_compatibility())
        .expect_err("unknown entrypoint field must reject");
    assert!(matches!(error, LanguagePackError::Manifest { .. }));
}

#[test]
fn duplicate_core_and_mismatched_signature_are_rejected_atomically() {
    let duplicate = PackFixture::new();
    let duplicate_root = duplicate.write_pack(
        "org.example.duplicate",
        "zh-CN",
        vec![entry("xiao.status.ready", "覆盖", json!({}))],
        "*",
    );
    let duplicate_pack =
        LanguagePack::load_directory(&duplicate_root, &current_compatibility()).expect("pack");
    let mut registry = LanguagePackRegistry::new();
    let error = registry
        .register(&duplicate_pack)
        .expect_err("built-in locale must not be overwritten");
    assert!(matches!(error, LanguagePackError::Conflict { .. }));
    assert!(registry.installed().is_empty());

    let mismatch = PackFixture::new();
    let mismatch_root = mismatch.write_pack(
        "org.example.signature",
        "fr-FR",
        vec![entry(
            "xiao.status.ready",
            "{label}",
            json!({ "label": "text" }),
        )],
        "*",
    );
    let mismatch_pack =
        LanguagePack::load_directory(&mismatch_root, &current_compatibility()).expect("pack");
    let error = registry
        .register(&mismatch_pack)
        .expect_err("built-in message signature must match");
    assert!(matches!(error, LanguagePackError::Conflict { .. }));
    assert!(registry.installed().is_empty());
}

#[test]
fn registering_the_same_plugin_replaces_its_previous_resources_atomically() {
    let first = PackFixture::new();
    let first_root = first.write_pack(
        "org.example.update",
        "fr-FR",
        vec![entry("plugin.ready", "première", json!({}))],
        "*",
    );
    let second = PackFixture::new();
    let second_root = second.write_pack(
        "org.example.update",
        "fr-FR",
        vec![entry("plugin.ready", "seconde", json!({}))],
        "*",
    );
    let first_pack =
        LanguagePack::load_directory(&first_root, &current_compatibility()).expect("first pack");
    let second_pack =
        LanguagePack::load_directory(&second_root, &current_compatibility()).expect("second pack");
    let mut registry = LanguagePackRegistry::new();
    registry.register(&first_pack).expect("register first pack");
    registry
        .register(&second_pack)
        .expect("replace updated pack");

    let rendered = registry.renderer().expect("renderer").render(
        &LocaleContext::new("fr-FR"),
        "plugin.ready",
        &BTreeMap::new(),
    );
    assert_eq!(rendered.text, "seconde");
    assert_eq!(
        registry.installed().get("org.example.update"),
        Some(&second_pack.content_digest().to_owned())
    );
}

#[test]
fn cache_key_rejects_non_semver_versions_that_could_escape_the_cache_root() {
    let error = LanguagePackCacheKey::new(1, "../escape", 1, "0".repeat(64))
        .expect_err("cache key must reject path-like Xiao versions");
    assert!(matches!(error, LanguagePackError::Manifest { .. }));
}

#[test]
fn runtime_retains_verified_directory_but_new_runtime_starts_unavailable() {
    let fixture = PackFixture::new();
    let root = fixture.write_pack(
        "org.example.retained",
        "fr-FR",
        vec![entry("plugin.ready", "ready", json!({}))],
        "*",
    );
    let pack = LanguagePack::load_directory(&root, &current_compatibility()).expect("pack");
    let digest = pack.content_digest().to_owned();
    let mut runtime = LanguagePackRuntime::new();
    assert_eq!(
        runtime.activate(Ok(pack)),
        LanguagePackActivation::Activated {
            plugin_id: "org.example.retained".to_owned(),
            digest,
        }
    );

    let error = LanguagePackError::DigestMismatch {
        plugin_id: "org.example.retained".to_owned(),
        expected: "0".repeat(64),
        actual: "1".repeat(64),
        content_length: 1,
        actual_length: 2,
    };
    assert!(matches!(
        runtime.activate(Err(error)),
        LanguagePackActivation::RetainedPrevious { .. }
    ));
    assert_eq!(
        runtime
            .renderer()
            .expect("retained renderer")
            .render(
                &LocaleContext::new("fr-FR"),
                "plugin.ready",
                &BTreeMap::new(),
            )
            .text,
        "ready"
    );

    let mut new_runtime = LanguagePackRuntime::new();
    let error = LanguagePackError::Manifest {
        path: PathBuf::from("xiao-language-pack.json"),
        message: "not verified".to_owned(),
    };
    assert!(matches!(
        new_runtime.activate(Err(error)),
        LanguagePackActivation::Unavailable { .. }
    ));
}
