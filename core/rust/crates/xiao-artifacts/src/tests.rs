//! 内容寻址对象、索引及 20E 要求字段的格式回归。

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};
use xiao_bytecode::{
    TAC_BYTECODE_ABI_VERSION, TAC_RUNTIME_ABI_VERSION, TAC_VERSION, TacAbi, TacProgram,
    XiaocMetadata, encode_xiaoc,
};

fn temp_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "xiao-artifacts-{label}-{}-{stamp}",
        std::process::id()
    ))
}

fn valid_xiaoc() -> Vec<u8> {
    let program = TacProgram {
        version: TAC_VERSION,
        abi: TacAbi {
            bytecode_abi_version: TAC_BYTECODE_ABI_VERSION,
            runtime_abi_version: TAC_RUNTIME_ABI_VERSION,
            ir_version: 1,
            language_version: "0.1.0".to_owned(),
            target: "portable".to_owned(),
        },
        constants: Default::default(),
        signatures: Default::default(),
        functions: Vec::new(),
        categories: Default::default(),
        plans: Vec::new(),
        selection_plans: Vec::new(),
        broadcast_assignment_plans: Vec::new(),
        random_seed_plans: Vec::new(),
        table_definitions: Vec::new(),
        unsupported: Vec::new(),
    };
    encode_xiaoc(&program, XiaocMetadata::default()).unwrap()
}

fn make_writable(path: &Path) {
    let mut permissions = fs::metadata(path).unwrap().permissions();
    #[cfg(unix)]
    permissions.set_mode(0o644);
    #[cfg(windows)]
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
fn digest_and_sharded_xiaoc_object_are_deterministic() {
    let root = temp_root("digest");
    let store = ArtifactStore::open(&root).unwrap();
    let bytes = valid_xiaoc();
    let object = store.put_xiaoc(&bytes).unwrap();
    assert_eq!(object.digest.as_hex().len(), 64);
    assert_eq!(
        object.path.file_name().unwrap().to_string_lossy(),
        format!("{}.xiaoc", object.digest)
    );
    assert_eq!(
        object.path.parent().unwrap().file_name().unwrap(),
        &object.digest.as_hex()[..2]
    );
    assert_eq!(store.read(ObjectKind::Xiaoc, object.digest).unwrap(), bytes);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn corrupt_object_is_quarantined() {
    let root = temp_root("corrupt");
    let store = ArtifactStore::open(&root).unwrap();
    let object = store.put(ObjectKind::Native, b"native-one").unwrap();
    make_writable(&object.path);
    fs::write(&object.path, b"tampered").unwrap();
    let error = store.read(ObjectKind::Native, object.digest).unwrap_err();
    assert!(matches!(error, ArtifactError::CorruptObject { .. }));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn objects_are_read_only_and_hash_collisions_quarantine_both_inputs() {
    let root = temp_root("collision");
    let store = ArtifactStore::open(&root).unwrap();
    let incoming = b"incoming-content";
    let digest = Digest256::of_bytes(incoming);
    let existing = store.object_path(ObjectKind::Native, digest);
    fs::create_dir_all(existing.parent().unwrap()).unwrap();
    fs::write(&existing, b"different-content").unwrap();
    let error = store.put(ObjectKind::Native, incoming).unwrap_err();
    assert!(matches!(error, ArtifactError::HashCollision { .. }));
    assert!(!existing.exists());
    let quarantined = fs::read_dir(root.join("quarantine"))
        .unwrap()
        .filter_map(Result::ok)
        .count();
    assert_eq!(quarantined, 2);

    let object = store.put(ObjectKind::Native, b"immutable").unwrap();
    assert!(fs::metadata(object.path).unwrap().permissions().readonly());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn interrupted_object_write_cleans_temporary_input() {
    struct FailingReader {
        emitted: bool,
    }

    impl Read for FailingReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.emitted {
                return Err(io::Error::other("测试中断"));
            }
            self.emitted = true;
            buffer[..4].copy_from_slice(b"part");
            Ok(4)
        }
    }

    let root = temp_root("interrupted-write");
    let store = ArtifactStore::open(&root).unwrap();
    let error = store
        .put_reader(ObjectKind::Native, &mut FailingReader { emitted: false })
        .unwrap_err();
    assert!(matches!(error, ArtifactError::Io(_)));
    assert_eq!(fs::read_dir(root.join("tmp")).unwrap().count(), 0);
    let _ = fs::remove_dir_all(root);
}

const INDEX_FUZZ_ROUNDS: usize = 512;
const INDEX_FUZZ_MAX_INPUT: usize = 64 * 1024;

/// xorshift64*；只用于让变异可按种子重放。
struct FuzzRng(u64);

impl FuzzRng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, upper: usize) -> usize {
        (self.next() % upper as u64) as usize
    }
}

/// 返回变异后的字节和按顺序记录的变异步骤。
fn mutate_index_bytes(base: &[u8], seed: u64, round: usize) -> (Vec<u8>, Vec<String>) {
    let mut rng = FuzzRng(seed ^ (round as u64 + 1).wrapping_mul(0x9E37_79B9));
    let mut bytes = base.to_vec();
    let mut steps = Vec::new();
    for _ in 0..=rng.below(4) {
        if bytes.is_empty() {
            break;
        }
        let at = rng.below(bytes.len());
        match rng.below(4) {
            0 => {
                let mask = (rng.next() as u8) | 1;
                bytes[at] ^= mask;
                steps.push(format!("xor[{at}]^={mask:#04x}"));
            }
            1 => {
                bytes.remove(at);
                steps.push(format!("remove[{at}]"));
            }
            2 => {
                let value = rng.next() as u8;
                bytes.insert(at, value);
                steps.push(format!("insert[{at}]={value:#04x}"));
            }
            _ => {
                bytes.truncate(at);
                steps.push(format!("truncate@{at}"));
            }
        }
    }
    (bytes, steps)
}

#[test]
fn seeded_index_mutations_never_panic_and_are_rejected_without_io_errors() {
    let digest = Digest256::of_bytes(b"fuzz-index");
    let archive = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: vec![ArchiveEntry {
            logical_path: "main.xiaoc".to_owned(),
            object_kind: ObjectKind::Xiaoc,
            digest,
            module: "main".to_owned(),
            target: "portable".to_owned(),
            length: 1,
        }],
        dependency_lock_digest: String::new(),
        runtime_abi_min: 1,
        runtime_abi_max: 1,
        platform: "portable".to_owned(),
        debug_activation: false,
        language_locale: "zh-CN".to_owned(),
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    };
    let global = GlobalIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        records: vec![GlobalRecord {
            request_key: "fuzz".to_owned(),
            object_kind: ObjectKind::Native,
            digest,
            target: "portable".to_owned(),
            optimization_level: 0,
            codegen_version: 1,
            length: 1,
        }],
    };
    let seeds = [archive.encode().unwrap(), global.encode().unwrap()];
    assert!(
        ArchiveIndex::decode(&seeds[0]).is_ok(),
        "基线归档索引必须可解码"
    );
    assert!(
        GlobalIndex::decode(&seeds[1]).is_ok(),
        "基线全局索引必须可解码"
    );
    for (seed_index, base) in seeds.iter().enumerate() {
        let seed = 0x0019_a004_u64 + seed_index as u64;
        let (mut accepted, mut rejected) = (0_usize, 0_usize);
        for round in 0..INDEX_FUZZ_ROUNDS {
            let (candidate, steps) = mutate_index_bytes(base, seed, round);
            assert!(candidate.len() <= INDEX_FUZZ_MAX_INPUT, "round={round}");
            let context = format!("kind={seed_index} seed={seed:#x} round={round} steps={steps:?}");
            let result = std::panic::catch_unwind(|| {
                if seed_index == 0 {
                    ArchiveIndex::decode(&candidate).map(|_| ())
                } else {
                    GlobalIndex::decode(&candidate).map(|_| ())
                }
            })
            .unwrap_or_else(|_| panic!("索引解析 panic：{context}"));
            match result {
                Ok(()) => accepted += 1,
                Err(ArtifactError::Io(error)) => {
                    panic!("内存输入不应产生 IO 错误（{error}）：{context}")
                }
                Err(_) => rejected += 1,
            }
        }
        assert!(
            rejected > INDEX_FUZZ_ROUNDS / 4,
            "kind={seed_index} 变异应有相当部分被拒绝，实际 rejected={rejected} accepted={accepted}"
        );
    }
}

#[test]
fn scan_quarantines_objects_with_invalid_names_or_shards() {
    let root = temp_root("scan-boundary");
    let store = ArtifactStore::open(&root).unwrap();
    let shard = root.join("objects/native/sha256/00");
    fs::create_dir_all(&shard).unwrap();
    fs::write(shard.join("not-a-digest.bin"), b"bad-name").unwrap();
    fs::write(
        shard.join(format!("{}.bin", Digest256::of_bytes(b"wrong-shard"))),
        b"wrong-shard",
    )
    .unwrap();
    assert!(store.scan(ObjectKind::Native).unwrap().is_empty());
    assert_eq!(fs::read_dir(root.join("quarantine")).unwrap().count(), 2);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn indexes_round_trip_deterministically_and_reject_unknown_major() {
    let digest = Digest256::of_bytes(b"xiaoc");
    let index = GlobalIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        records: vec![GlobalRecord {
            request_key: "request".to_owned(),
            object_kind: ObjectKind::Xiaoc,
            digest,
            target: "x86_64-unknown-linux-gnu".to_owned(),
            optimization_level: 2,
            codegen_version: 2,
            length: 5,
        }],
    };
    let first = index.encode().unwrap();
    assert_eq!(first, index.encode().unwrap());
    assert_eq!(GlobalIndex::decode(&first).unwrap(), index);
    let bad = GlobalIndex {
        schema_major: 2,
        ..index.clone()
    };
    assert!(matches!(
        bad.encode(),
        Err(ArtifactError::UnsupportedIndexVersion { .. })
    ));

    let mut wrong_type = first.clone();
    wrong_type.extend_from_slice(&[0x20, 0x63]);
    assert!(matches!(
        GlobalIndex::decode(&wrong_type),
        Err(ArtifactError::Index(_))
    ));

    let archive = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: vec![ArchiveEntry {
            logical_path: "objects/xiaoc/main.xiaoc".to_owned(),
            object_kind: ObjectKind::Xiaoc,
            digest,
            module: "main".to_owned(),
            target: "portable".to_owned(),
            length: 5,
        }],
        dependency_lock_digest: String::new(),
        runtime_abi_min: 0,
        runtime_abi_max: 0,
        platform: String::new(),
        debug_activation: false,
        language_locale: String::new(),
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    };
    assert_eq!(
        ArchiveIndex::decode(&archive.encode().unwrap()).unwrap(),
        archive
    );

    let replacement = GlobalRecord {
        request_key: "request".to_owned(),
        object_kind: ObjectKind::Native,
        digest,
        target: "x86_64-unknown-linux-gnu".to_owned(),
        optimization_level: 3,
        codegen_version: 2,
        length: 5,
    };
    let mut updated = index;
    updated.upsert(replacement.clone());
    assert_eq!(updated.find("request"), Some(&replacement));
}

#[test]
fn archive_index_entry_validation_is_symmetric() {
    let missing_entry = [0x08, 0x01, 0x10, 0x00, 0x28, 0x01, 0x30, 0x00];
    let empty_entry = [0x08, 0x01, 0x10, 0x00, 0x1a, 0x00, 0x28, 0x01, 0x30, 0x00];
    let non_empty_entry = [
        0x08, 0x01, 0x10, 0x00, 0x1a, 0x03, b'm', b'a', b'i', 0x28, 0x01, 0x30, 0x01, 0x68, 0x01,
        0x72, 0x05, b'0', b'.', b'1', b'.', b'0', 0x80, 0x01, 0x00,
    ];
    assert!(ArchiveIndex::decode(&missing_entry).is_err());
    assert!(ArchiveIndex::decode(&empty_entry).is_err());
    assert_eq!(ArchiveIndex::decode(&non_empty_entry).unwrap().entry, "mai");
}

#[test]
fn archive_index_rejects_unknown_required_features() {
    let index = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: Vec::new(),
        dependency_lock_digest: String::new(),
        runtime_abi_min: 0,
        runtime_abi_max: 0,
        platform: String::new(),
        debug_activation: false,
        language_locale: String::new(),
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    };
    let mut bytes = without_archive_field(&index.encode().unwrap(), 6);
    put_varint_field(&mut bytes, 6, ARCHIVE_REQUIREMENTS_FEATURE | (1 << 7));
    assert!(matches!(
        ArchiveIndex::decode(&bytes),
        Err(ArtifactError::InvalidArchiveRequirements(_))
    ));
}

#[test]
fn archive_index_extended_metadata_round_trips_without_changing_old_fields() {
    let index = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: Vec::new(),
        dependency_lock_digest: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .to_owned(),
        runtime_abi_min: 0x0001_0007,
        runtime_abi_max: 0x0001_0008,
        platform: "x86_64-pc-windows-msvc".to_owned(),
        debug_activation: true,
        language_locale: "zh-CN".to_owned(),
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    };
    assert_eq!(
        ArchiveIndex::decode(&index.encode().unwrap()).unwrap(),
        index
    );
}

/// 不含对象的索引夹具；版本字段显式设置，不依赖隐式当前值。
fn requirements_index() -> ArchiveIndex {
    ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: Vec::new(),
        dependency_lock_digest: String::new(),
        runtime_abi_min: 0,
        runtime_abi_max: 0,
        platform: String::new(),
        debug_activation: false,
        language_locale: String::new(),
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    }
}

/// 删除指定 protobuf 字段，其余原始字节保持不变，用于缺字段负例。
fn without_archive_field(bytes: &[u8], omitted: u32) -> Vec<u8> {
    let mut reader = ProtoReader::new(bytes);
    let mut output = Vec::new();
    loop {
        let start = reader.offset;
        let Some((field, wire)) = reader.next_key().unwrap() else {
            break;
        };
        reader.skip(wire).unwrap();
        if field != omitted {
            output.extend_from_slice(&bytes[start..reader.offset]);
        }
    }
    output
}

#[test]
/// 新字段往返、模块排序去重与重复编码均稳定；零能力字段也必须存在。
fn archive_requirements_round_trip_is_canonical() {
    let mut index = requirements_index();
    index.required_features = xiao_runtime_abi::REQUIRED_FEATURE_THREADS;
    index.stdlib_modules = vec![
        "stdlib.text".to_owned(),
        "stdlib.os".to_owned(),
        "stdlib.text".to_owned(),
    ];
    let bytes = index.encode().unwrap();
    assert_eq!(bytes, index.encode().unwrap());
    index.stdlib_modules = vec!["stdlib.os".to_owned(), "stdlib.text".to_owned()];
    let decoded = ArchiveIndex::decode(&bytes).unwrap();
    assert_eq!(decoded, index);
    assert_eq!(decoded.encode().unwrap(), bytes);
    assert!(matches!(
        ArchiveIndex::decode(&without_archive_field(&bytes, 16)),
        Err(ArtifactError::ArchiveVersionIncompatible(_))
    ));
    assert!(matches!(
        ArchiveIndex::decode(&without_archive_field(&bytes, 6)),
        Err(ArtifactError::ArchiveVersionIncompatible(_))
    ));
}

#[test]
/// 旧归档和任意必填要求字段缺失均不可静默兼容。
fn archive_requires_new_feature_and_version_fields() {
    let legacy = [0x08, 1, 0x10, 0, 0x1a, 1, b'x', 0x28, 1, 0x30, 0];
    assert!(matches!(
        ArchiveIndex::decode(&legacy),
        Err(ArtifactError::ArchiveVersionIncompatible(_))
    ));
    let bytes = requirements_index().encode().unwrap();
    for omitted in [6, 13, 14, 16] {
        assert!(
            matches!(
                ArchiveIndex::decode(&without_archive_field(&bytes, omitted)),
                Err(ArtifactError::ArchiveVersionIncompatible(_))
            ),
            "字段 {omitted}"
        );
    }
    let mut unset = requirements_index();
    unset.language_version.clear();
    assert!(matches!(
        unset.encode(),
        Err(ArtifactError::ArchiveVersionIncompatible(_))
    ));
}

#[test]
/// 冻结的 17A 读取规则只认识 feature=0；GlobalIndex 仍使用该原值。
fn new_archive_feature_rejects_legacy_reader_without_changing_global_index() {
    let bytes = requirements_index().encode().unwrap();
    let mut legacy_reader = ProtoReader::new(&bytes);
    let mut legacy_features = 0;
    while let Some((field, wire)) = legacy_reader.next_key().unwrap() {
        if field == 6 {
            legacy_features = legacy_reader.varint(wire).unwrap();
        } else {
            legacy_reader.skip(wire).unwrap();
        }
    }
    assert_ne!(
        legacy_features, INDEX_REQUIRED_FEATURES,
        "旧读取器会错误地放行新归档"
    );
    assert_eq!(legacy_features, ARCHIVE_REQUIREMENTS_FEATURE);
    let global = GlobalIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        records: Vec::new(),
    };
    let bytes = global.encode().unwrap();
    assert_eq!(GlobalIndex::decode(&bytes).unwrap(), global);
    let mut reader = ProtoReader::new(&bytes);
    let mut features = None;
    while let Some((field, wire)) = reader.next_key().unwrap() {
        if field == 5 {
            features = Some(reader.varint(wire).unwrap());
        } else {
            reader.skip(wire).unwrap();
        }
    }
    assert_eq!(features, Some(0));
}

#[test]
/// 低位和最高位的未知能力均拒绝；标准库接口版本不能被截断为 u32。
fn archive_rejects_unknown_capabilities_and_overflowing_interface_version() {
    for bit in [7, 63] {
        let mut index = requirements_index();
        index.required_capabilities = 1 << bit;
        assert!(matches!(
            index.encode(),
            Err(ArtifactError::InvalidArchiveRequirements(_))
        ));
        let mut raw = without_archive_field(&requirements_index().encode().unwrap(), 16);
        put_varint_field(&mut raw, 16, 1 << bit);
        assert!(matches!(
            ArchiveIndex::decode(&raw),
            Err(ArtifactError::InvalidArchiveRequirements(_))
        ));
    }
    let mut raw = without_archive_field(&requirements_index().encode().unwrap(), 13);
    put_varint_field(&mut raw, 13, 1 << 32);
    assert!(matches!(
        ArchiveIndex::decode(&raw),
        Err(ArtifactError::InvalidArchiveRequirements(_))
    ));

    let mut features = requirements_index();
    features.required_features = 1 << 7;
    assert!(matches!(
        features.encode(),
        Err(ArtifactError::InvalidArchiveRequirements(_))
    ));
}

#[test]
fn xiaoc_validation_comes_from_authoritative_codec() {
    let root = temp_root("authoritative-xiaoc-validation");
    let store = ArtifactStore::open(&root).unwrap();
    let error = store.put_xiaoc(&[]).unwrap_err();
    match error {
        ArtifactError::InvalidObject { reason, .. } => {
            assert!(reason.contains("XIAOC-001"));
            assert!(!reason.contains("不是完整规范 `.xiaoc` 文件"));
        }
        other => panic!("`.xiaoc` 校验错误类型错误：{other:?}"),
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn index_store_commits_global_snapshot_atomically() {
    let root = temp_root("index");
    let store = IndexStore::open(&root).unwrap();
    let index = GlobalIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        records: Vec::new(),
    };
    let path = store.write_global(&index).unwrap();
    assert!(path.is_file());
    assert_eq!(store.read_global().unwrap(), index);
    assert!(!root.join("global.index.tmp").exists());
    assert!(!root.join("global.index.lock").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn indexes_remain_readable_while_their_write_locks_are_held() {
    let root = temp_root("readonly-index");
    let store = IndexStore::open(&root).unwrap();
    let global = GlobalIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        records: Vec::new(),
    };
    store.write_global(&global).unwrap();
    let archive_path = root.join("sample.index.pb");
    let archive = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: Vec::new(),
        dependency_lock_digest: String::new(),
        runtime_abi_min: 0,
        runtime_abi_max: 0,
        platform: String::new(),
        debug_activation: false,
        language_locale: String::new(),
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    };
    store.write_archive(&archive_path, &archive).unwrap();

    let _global_lock = xiao_lock::EntryLock::acquire(&root.join("global.index.lock")).unwrap();
    let _archive_lock =
        xiao_lock::EntryLock::acquire(&archive_path.with_extension("index.lock")).unwrap();
    assert_eq!(store.read_global().unwrap(), global);
    assert_eq!(store.read_archive(&archive_path).unwrap(), archive);
    drop((_global_lock, _archive_lock));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn global_index_can_rebuild_from_verified_objects_and_query_both_directions() {
    let root = temp_root("rebuild");
    let artifacts = ArtifactStore::open(root.join("objects")).unwrap();
    let object = artifacts.put(ObjectKind::Native, b"native").unwrap();
    let indexes = IndexStore::open(root.join("indexes")).unwrap();
    indexes
        .rebuild_global_atomic(&artifacts, |reference| {
            Some(GlobalRecord {
                request_key: "request".to_owned(),
                object_kind: reference.kind,
                digest: reference.digest,
                target: "x86_64-unknown-linux-gnu".to_owned(),
                optimization_level: 2,
                codegen_version: 2,
                length: reference.length,
            })
        })
        .unwrap();
    let index = indexes.read_global().unwrap();
    assert_eq!(index.find("request").unwrap().digest, object.digest);
    assert_eq!(
        index
            .find_by_digest(ObjectKind::Native, object.digest)
            .count(),
        1
    );
    assert_eq!(
        index
            .find_matching("x86_64-unknown-linux-gnu", 2, 2)
            .count(),
        1
    );
    assert!(!root.join("indexes/global.index.lock").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cleanup_plan_protects_project_lock_references_and_apply_is_explicit() {
    let root = temp_root("maintenance");
    let store = ArtifactStore::open(&root).unwrap();
    let protected = store.put(ObjectKind::Source, b"protected-source").unwrap();
    let removable = store.put(ObjectKind::Native, b"removable-native").unwrap();
    let lock_path = root.join("xiao.lock.json");
    fs::write(
        &lock_path,
        format!(
            r#"{{"lock_version":2,"config_fingerprint":"config","root":{{"name":"main","version":"0.1.0","source":{{"source_id":"path:/main","alias":null,"display_name":"main"}}}},"packages":{{"main@0.1.0[path:/main]":{{"name":"main","version":"0.1.0","source":{{"source_id":"path:/main","alias":null,"display_name":"main"}},"content_digest":"{}","dependencies":{{}},"precompiled_variants":[],"target_conditions":[]}}}}}}"#,
            protected.digest
        ),
    )
    .unwrap();
    let references = store
        .collect_references(Some(&lock_path), &[], &[])
        .unwrap();
    let plan = store.plan_cleanup(&references).unwrap();
    assert_eq!(plan.candidates(), std::slice::from_ref(&removable));
    assert!(!root.join("maintenance.lock").exists());
    assert!(store.read(ObjectKind::Source, protected.digest).is_ok());

    let report = store.apply_cleanup(&plan, &references).unwrap();
    assert_eq!(report.removed, vec![removable]);
    assert!(matches!(
        store.read(ObjectKind::Native, Digest256::of_bytes(b"removable-native")),
        Err(ArtifactError::Io(error)) if error.kind() == io::ErrorKind::NotFound
    ));
    assert!(!root.join("maintenance.lock").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cleanup_reference_source_failure_is_conservative() {
    let root = temp_root("maintenance-failure");
    let store = ArtifactStore::open(&root).unwrap();
    let object = store.put(ObjectKind::Native, b"keep-on-error").unwrap();
    let lock_path = root.join("broken.lock.json");
    fs::write(&lock_path, b"not-json").unwrap();
    assert!(
        store
            .collect_references(Some(&lock_path), &[], &[])
            .is_err()
    );
    fs::write(
        &lock_path,
        br#"{"lock_version":2,"config_fingerprint":"x","root":{},"packages":{}}"#,
    )
    .unwrap();
    assert!(
        store
            .collect_references(Some(&lock_path), &[], &[])
            .is_err()
    );
    assert!(store.read(ObjectKind::Native, object.digest).is_ok());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn conservative_reference_set_protects_objects_when_sources_are_unavailable() {
    let root = temp_root("maintenance-protect-all");
    let store = ArtifactStore::open(&root).unwrap();
    let object = store
        .put(ObjectKind::Native, b"protect-without-source")
        .unwrap();
    let plan = store.plan_cleanup(&ReferenceSet::protect_all()).unwrap();
    assert!(plan.candidates().is_empty());
    let report = store
        .apply_cleanup(&plan, &ReferenceSet::protect_all())
        .unwrap();
    assert!(report.removed.is_empty());
    assert!(store.read(object.kind, object.digest).is_ok());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cleanup_revalidates_and_quarantines_object_changed_after_plan() {
    let root = temp_root("maintenance-corrupt");
    let store = ArtifactStore::open(&root).unwrap();
    let object = store.put(ObjectKind::Native, b"before-plan").unwrap();
    let plan = store.plan_cleanup(&ReferenceSet::new()).unwrap();
    make_writable(&object.path);
    fs::write(&object.path, b"after-plan-tamper").unwrap();
    let report = store.apply_cleanup(&plan, &ReferenceSet::new()).unwrap();
    assert_eq!(report.removed, Vec::<ObjectReference>::new());
    assert_eq!(report.quarantined.len(), 1);
    assert!(!object.path.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cleanup_plan_defers_invalid_object_isolation_until_apply() {
    let root = temp_root("maintenance-invalid");
    let store = ArtifactStore::open(&root).unwrap();
    let shard = root.join("objects/native/sha256/00");
    fs::create_dir_all(&shard).unwrap();
    let invalid = shard.join("bad-name.bin");
    fs::write(&invalid, b"invalid-name").unwrap();
    let plan = store.plan_cleanup(&ReferenceSet::new()).unwrap();
    assert!(plan.candidates().is_empty());
    assert!(invalid.exists());
    let report = store.apply_cleanup(&plan, &ReferenceSet::new()).unwrap();
    assert_eq!(report.removed.len(), 0);
    assert_eq!(report.quarantined.len(), 1);
    assert!(!invalid.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cleanup_rechecks_references_added_after_plan_creation() {
    let root = temp_root("maintenance-new-reference");
    let store = ArtifactStore::open(&root).unwrap();
    let object = store.put(ObjectKind::Native, b"newly-referenced").unwrap();
    let plan = store.plan_cleanup(&ReferenceSet::new()).unwrap();
    assert_eq!(plan.candidates(), std::slice::from_ref(&object));

    let mut current_references = ReferenceSet::new();
    current_references.insert(CacheReference::exact(object.kind, object.digest));
    let report = store.apply_cleanup(&plan, &current_references).unwrap();
    assert!(report.removed.is_empty());
    assert!(store.read(object.kind, object.digest).is_ok());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cleanup_archive_reference_protects_its_object() {
    let root = temp_root("maintenance-archive-reference");
    let store = ArtifactStore::open(root.join("cache")).unwrap();
    let object = store.put(ObjectKind::Xiaoc, &valid_xiaoc()).unwrap();
    let index_store = IndexStore::open(root.join("indexes")).unwrap();
    let archive_path = root.join("archive-index.pb");
    let archive = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: vec![ArchiveEntry {
            logical_path: "objects/xiaoc/main.xiaoc".to_owned(),
            object_kind: ObjectKind::Xiaoc,
            digest: object.digest,
            module: "main".to_owned(),
            target: "portable".to_owned(),
            length: object.length,
        }],
        dependency_lock_digest: String::new(),
        runtime_abi_min: 0,
        runtime_abi_max: 0,
        platform: String::new(),
        debug_activation: false,
        language_locale: String::new(),
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    };
    index_store.write_archive(&archive_path, &archive).unwrap();
    let references = store
        .collect_references(None, std::slice::from_ref(&archive_path), &[])
        .unwrap();
    let plan = store.plan_cleanup(&references).unwrap();
    assert!(plan.candidates().is_empty());
    let report = store.apply_cleanup(&plan, &references).unwrap();
    assert!(report.removed.is_empty());
    assert!(store.read(ObjectKind::Xiaoc, object.digest).is_ok());
    let _ = fs::remove_dir_all(root);
}
