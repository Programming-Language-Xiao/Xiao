//! 11A-E3A：共享 JCS 向量及离线包源契约。

use std::collections::BTreeMap;
use std::fs;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use xiao_config::parse_config_text;
use xiao_package::{
    ArtifactReference, ConfiguredSource, IndexPackage, LocalDirectoryAdapter, PackageShard,
    PackageSourceAdapter, SOURCE_ALIAS_CONFLICT_CODE, SOURCE_AMBIGUOUS_CODE,
    SOURCE_DIGEST_MISMATCH_CODE, SOURCE_INVALID_CODE, SOURCE_UNAVAILABLE_CODE,
    SOURCE_UNKNOWN_REFERENCE_CODE, SOURCE_UNSUPPORTED_VERSION_CODE, SnapshotStatus,
    SourceDeclaration, SourceDescriptor, SourceList, SourceSnapshot, canonicalize_json,
    expand_source_lists, federate, jcs_digest, select_source, source_declarations, source_id,
    source_list_fingerprint,
};

#[test]
/// 真实加载跨语言向量，逐例比较规范字节与哈希。
fn jcs_shared_vectors() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../tests/spec/11a-jcs/vectors.json"
    ))
    .unwrap();
    for vector in vectors {
        let input = vector["input"].as_str().unwrap();
        let label = vector["name"].as_str().unwrap();
        if let Some(code) = vector.get("error") {
            assert!(vector["expected_bytes"].as_str().unwrap().is_empty());
            assert!(vector["sha256"].as_str().unwrap().is_empty());
            assert_eq!(
                canonicalize_json(input).unwrap_err().code,
                code.as_str().unwrap(),
                "{label}"
            );
        } else {
            assert_eq!(
                canonicalize_json(input).unwrap(),
                vector["expected_bytes"].as_str().unwrap().as_bytes(),
                "{label}"
            );
            assert_eq!(
                jcs_digest(input).unwrap(),
                vector["sha256"].as_str().unwrap(),
                "{label}"
            );
        }
    }
}

#[test]
/// 配置输入经静态解析后，校验直接源、导入顺序及负例编号。
fn source_specs_read_config_tree_and_preserve_order() {
    let valid: Value = serde_json::from_str(include_str!(
        "../../../../../tests/spec/11a-source/valid.json"
    ))
    .unwrap();
    let document = parse_config_text(valid["config"].as_str().unwrap()).unwrap();
    let declarations = source_declarations(&document).unwrap();
    let direct = declarations
        .iter()
        .filter_map(|item| match item {
            SourceDeclaration::Direct(source) => Some(source),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        direct
            .iter()
            .map(|source| &source.source.source_id)
            .collect::<Vec<_>>(),
        valid["ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        direct
            .iter()
            .map(|source| source.source.alias.as_deref().unwrap())
            .collect::<Vec<_>>(),
        valid["aliases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>()
    );
    let list = SourceList::parse(r#"{ "sources": [{"location":"https://import.example","alias":"third","kind":"static"}], "protocol_version":1 }"#).unwrap();
    assert_eq!(list.digest, valid["import_digest"].as_str().unwrap());
    let lists = BTreeMap::from([(valid["import"].as_str().unwrap().to_owned(), list)]);
    let expanded = expand_source_lists(&declarations, &lists).unwrap();
    assert_eq!(
        expanded
            .iter()
            .map(|entry| entry.descriptor.source.source_id.as_str())
            .collect::<Vec<_>>(),
        valid["final_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        expanded
            .iter()
            .map(|entry| entry.config_order)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert!(expanded[0].imported_from.is_none());
    assert_eq!(
        expanded[2].imported_from.as_ref().unwrap().digest,
        valid["import_digest"].as_str().unwrap()
    );
    let mut changed_origin = expanded.clone();
    changed_origin[2].imported_from = None;
    assert_ne!(
        source_list_fingerprint(&expanded),
        source_list_fingerprint(&changed_origin)
    );
    assert_eq!(
        expand_source_lists(&declarations, &BTreeMap::new())
            .unwrap_err()
            .code,
        SOURCE_UNKNOWN_REFERENCE_CODE
    );
    let wrong = BTreeMap::from([(
        valid["import"].as_str().unwrap().to_owned(),
        SourceList::parse(r#"{"protocol_version":1,"sources":[]}"#).unwrap(),
    )]);
    assert_eq!(
        expand_source_lists(&declarations, &wrong).unwrap_err().code,
        SOURCE_DIGEST_MISMATCH_CODE
    );

    let errors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../tests/spec/11a-source/errors.json"
    ))
    .unwrap();
    for error in errors {
        let code = error["code"].as_str().unwrap();
        let text = error["config"].as_str().unwrap();
        if code.starts_with("X05-CONFIG") {
            assert!(
                parse_config_text(text)
                    .unwrap_err()
                    .iter()
                    .any(|diagnostic| diagnostic.code() == code),
                "{text}"
            );
        } else {
            let document = parse_config_text(text).unwrap();
            let declarations = source_declarations(&document);
            let actual = match declarations {
                Err(error) => error.code,
                Ok(declarations) => {
                    expand_source_lists(&declarations, &BTreeMap::new())
                        .unwrap_err()
                        .code
                }
            };
            assert_eq!(actual, code, "{text}");
        }
    }
}

/// 构造不需要访问网络的配置源。
fn direct(kind: &str, location: &str, alias: &str, order: usize) -> ConfiguredSource {
    ConfiguredSource {
        descriptor: SourceDescriptor::new(kind, location, Some(alias), None, 1).unwrap(),
        config_order: order,
        imported_from: None,
    }
}

/// 构造具有固定包名的候选版本元数据。
fn candidate(version: &str) -> IndexPackage {
    IndexPackage {
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
        source_artifact: ArtifactReference {
            location: "artifacts/demo".to_owned(),
            length: 4,
            digest: "body".to_owned(),
        },
        binary_artifacts: Vec::new(),
    }
}

/// 构造无文件系统依赖的索引快照。
fn snapshot(source: &ConfiguredSource, version: &str, status: SnapshotStatus) -> SourceSnapshot {
    SourceSnapshot {
        config_order: source.config_order,
        source_id: source.descriptor.source.source_id.clone(),
        snapshot_id: Some("snapshot-1".to_owned()),
        snapshot_digest: Some("a".repeat(64)),
        status,
        candidates: if status == SnapshotStatus::Unavailable {
            Vec::new()
        } else {
            vec![candidate(version)]
        },
    }
}

#[test]
/// 验证来源保留、顺序优先、不可达阻断和歧义诊断。
fn source_selection_preserves_origin_and_rejects_ambiguity() {
    let first = direct("static", "https://first.example", "first", 0);
    let second = direct("static", "https://second.example", "second", 1);
    let sources = [first.clone(), second.clone()];
    let snapshots = [
        snapshot(&second, "9.0", SnapshotStatus::Fresh),
        snapshot(&first, "1.0", SnapshotStatus::Cached),
    ];
    let records = federate(&snapshots).unwrap();
    assert_eq!(records.len(), 2);
    let reordered = federate(&[snapshots[1].clone(), snapshots[0].clone()]).unwrap();
    assert_eq!(
        serde_json::to_vec(&records).unwrap(),
        serde_json::to_vec(&reordered).unwrap()
    );
    assert_ne!(records[0].key.source_id, records[1].key.source_id);
    let selected = select_source(&sources, &snapshots, &records, "demo", None, |_| true).unwrap();
    assert_eq!(selected[0].package.version, "1.0");
    assert_eq!(
        select_source(
            &sources,
            &snapshots,
            &records,
            "demo",
            Some("second"),
            |_| true
        )
        .unwrap()[0]
            .package
            .version,
        "9.0"
    );
    assert_eq!(
        select_source(
            &sources,
            &snapshots,
            &records,
            "demo",
            None,
            |record| record.package.version == "9.0"
        )
        .unwrap()[0]
            .key
            .config_order,
        1
    );
    assert_eq!(
        select_source(
            &sources,
            &snapshots,
            &records,
            "demo",
            Some("missing"),
            |_| true
        )
        .unwrap_err()
        .code,
        SOURCE_UNKNOWN_REFERENCE_CODE
    );
    let unavailable = [
        snapshot(&first, "1.0", SnapshotStatus::Unavailable),
        snapshots[0].clone(),
    ];
    assert_eq!(
        select_source(
            &sources,
            &unavailable,
            &federate(&unavailable).unwrap(),
            "demo",
            None,
            |_| true
        )
        .unwrap_err()
        .code,
        SOURCE_UNAVAILABLE_CODE
    );
    let colliding_order = [
        first,
        direct("static", "https://second.example", "second", 0),
    ];
    assert_eq!(
        select_source(&colliding_order, &snapshots, &records, "demo", None, |_| {
            true
        })
        .unwrap_err()
        .code,
        SOURCE_AMBIGUOUS_CODE
    );
    assert_eq!(
        select_source(
            &[sources[0].clone(), sources[0].clone()],
            &snapshots,
            &records,
            "demo",
            Some("first"),
            |_| true
        )
        .unwrap_err()
        .code,
        SOURCE_AMBIGUOUS_CODE
    );
    let duplicate = federate(&[snapshots[1].clone(), snapshots[1].clone()]).unwrap_err();
    assert_eq!(duplicate.code, SOURCE_INVALID_CODE);

    let fingerprint = source_list_fingerprint(&sources);
    assert_eq!(fingerprint.ordered_sources.len(), 2);
    let renamed = [
        direct("static", "https://first.example", "renamed", 0),
        sources[1].clone(),
    ];
    assert_eq!(fingerprint, source_list_fingerprint(&renamed));
    assert_eq!(
        fingerprint,
        source_list_fingerprint(&[sources[1].clone(), sources[0].clone()])
    );
    assert_ne!(
        fingerprint,
        source_list_fingerprint(&[
            direct("static", "https://second.example", "second", 0),
            direct("static", "https://first.example", "first", 1)
        ])
    );
    assert_eq!(SOURCE_ALIAS_CONFLICT_CODE, "X05-SOURCE-002");
}

#[test]
/// 保持本地身份兼容，同时避免把不同远程协议混为一个源。
fn source_ids_normalize_without_merging_distinct_protocols() {
    assert_eq!(
        source_id("git-index", "HTTPS://GitHub.COM:443/org/repo.git/").unwrap(),
        "git-index:https://github.com/org/repo"
    );
    assert_ne!(
        source_id("static", "http://example.org").unwrap(),
        source_id("static", "https://example.org").unwrap()
    );
    assert_eq!(source_id("path", "C:\\demo\\").unwrap(), "path:C:/demo");
    assert_eq!(source_id("path", "").unwrap(), "path:");
    assert_eq!(
        SourceDescriptor::new("path", "relative/path", Some("relative"), None, 1)
            .unwrap_err()
            .code,
        SOURCE_INVALID_CODE
    );
    assert_eq!(
        source_id("static", "https://EXAMPLE.ORG:0443/a").unwrap(),
        "static:https://example.org/a"
    );
    assert_eq!(
        source_id("static", "https://[::1]:443/a").unwrap(),
        "static:https://[::1]/a"
    );
    assert_eq!(
        source_id("static", "https://example.org:bogus/index")
            .unwrap_err()
            .code,
        SOURCE_INVALID_CODE
    );
    assert_eq!(
        SourceDescriptor::new("static", "https://example.org", None, None, 8)
            .unwrap_err()
            .code,
        SOURCE_UNSUPPORTED_VERSION_CODE
    );
}

#[test]
/// 用隔离目录验证清单、分片、正文和摘要的独立读取边界。
fn local_adapter_separates_metadata_from_body_and_checks_digests() {
    let root = std::env::temp_dir().join(format!(
        "xiao-e3a-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("index")).unwrap();
    fs::create_dir(root.join("artifacts")).unwrap();
    let descriptor =
        SourceDescriptor::new("path", root.to_str().unwrap(), Some("local"), None, 1).unwrap();
    let body = b"body";
    let artifact = ArtifactReference {
        location: "artifacts/demo".to_owned(),
        length: body.len() as u64,
        digest: format!("{:x}", Sha256::digest(body)),
    };
    fs::write(root.join("artifacts/demo"), body).unwrap();
    let mut package = candidate("1.0");
    package.source_artifact = artifact.clone();
    let shard = PackageShard {
        protocol_version: 1,
        source_id: descriptor.source.source_id.clone(),
        snapshot_id: "one".to_owned(),
        packages: vec![package.clone()],
    };
    let shard_text = serde_json::to_string(&shard).unwrap();
    fs::write(root.join("index/demo.json"), &shard_text).unwrap();
    let manifest = json!({ "protocol_version": 1, "source_id": descriptor.source.source_id, "snapshot_id": "one", "shards": { "demo": jcs_digest(&shard_text).unwrap() } });
    fs::write(
        root.join("snapshot.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let adapter = LocalDirectoryAdapter;
    let snapshot = adapter.read_snapshot(&descriptor).unwrap();
    assert_eq!(
        adapter
            .read_package(&descriptor, &snapshot, "demo")
            .unwrap(),
        [package]
    );
    assert!(
        adapter
            .read_package(&descriptor, &snapshot, "absent")
            .unwrap()
            .is_empty()
    );
    assert_eq!(adapter.read_artifact(&descriptor, &artifact).unwrap(), body);
    fs::write(root.join("index/demo.json"), "{}").unwrap();
    assert_eq!(
        adapter
            .read_package(&descriptor, &snapshot, "demo")
            .unwrap_err()
            .code,
        SOURCE_DIGEST_MISMATCH_CODE
    );
    let bad_manifest = json!({ "protocol_version": 99, "source_id": descriptor.source.source_id, "snapshot_id": "one", "shards": {} });
    fs::write(root.join("snapshot.json"), bad_manifest.to_string()).unwrap();
    assert_eq!(
        adapter.read_snapshot(&descriptor).unwrap_err().code,
        SOURCE_UNSUPPORTED_VERSION_CODE
    );
    fs::remove_file(root.join("snapshot.json")).unwrap();
    fs::remove_file(root.join("index/demo.json")).unwrap();
    fs::remove_file(root.join("artifacts/demo")).unwrap();
    fs::remove_dir(root.join("index")).unwrap();
    fs::remove_dir(root.join("artifacts")).unwrap();
    fs::remove_dir(root).unwrap();
}
