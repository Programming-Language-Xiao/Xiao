//! 归档容器、安全边界及 20E 启动前要求回归。

use super::*;
use std::sync::atomic::AtomicU64;
use xiao_artifacts::ArchiveEntry;
use xiao_bytecode::{
    TAC_BYTECODE_ABI_VERSION, TAC_RUNTIME_ABI_VERSION, TAC_VERSION, TacAbi, TacProgram,
    XiaocMetadata, XiaocOptions,
};
use xiao_config::parse_config_project;
use xiao_source::SourceFile;

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

fn index_for(bytes: &[u8]) -> (ArchiveIndex, XarObject) {
    let object = XarObject::from_bytes(ObjectKind::Source, bytes);
    let index = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiao".to_owned(),
        entries: vec![ArchiveEntry {
            logical_path: "main.xiao".to_owned(),
            object_kind: ObjectKind::Source,
            digest: object.digest,
            module: "main".to_owned(),
            target: "portable".to_owned(),
            length: object.bytes.len() as u64,
        }],
        dependency_lock_digest: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_owned(),
        runtime_abi_min: xiao_runtime_abi::ABI_ENCODED_VERSION,
        runtime_abi_max: xiao_runtime_abi::ABI_ENCODED_VERSION,
        platform: "portable".to_owned(),
        debug_activation: false,
        language_locale: "zh-CN".to_owned(),
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    };
    (index, object)
}

/// 只供格式负例使用：重新写 ZIP 长度/CRC，避免损坏外层结构掩盖索引错误。
fn raw_index_archive(index_bytes: &[u8], object: &XarObject) -> Vec<u8> {
    let mut members = vec![
        encode_member(
            XAR_INDEX_PATH.to_owned(),
            index_bytes,
            CompressionMethod::Store,
            6,
        )
        .unwrap(),
        encode_member(
            object_member_path(object.kind, object.digest),
            &object.bytes,
            CompressionMethod::Store,
            6,
        )
        .unwrap(),
    ];
    members.sort_by(|left, right| left.path.cmp(&right.path));
    write_zip(&members, false).unwrap()
}

/// 为审核负例生成外层完整、入口种类为字节码的归档索引。
fn review_index_for_xiaoc(object: &XarObject) -> ArchiveIndex {
    let (mut index, _) = index_for(&object.bytes);
    index.entry = "main.xiaoc".to_owned();
    index.entries[0].logical_path = index.entry.clone();
    index.entries[0].object_kind = object.kind;
    index.entries[0].digest = object.digest;
    index.runtime_abi_min = xiao_runtime_abi::ABI_ENCODED_VERSION;
    index.runtime_abi_max = xiao_runtime_abi::ABI_ENCODED_VERSION;
    index
}

#[test]
/// 声明不兼容时应先拒绝元数据，不能先读取/解码损坏的入口载荷。
fn review_requirements_precede_payload_decoding() {
    let object = XarObject::from_bytes(ObjectKind::Xiaoc, b"invalid xiaoc payload");
    let base = review_index_for_xiaoc(&object);
    for (version, capabilities, context, expected) in [
        (
            base.stdlib_interface_version + 1,
            0,
            xiao_intrinsics::KNOWN_CAPABILITY_MASK,
            ARCHIVE_VERSION_INCOMPATIBLE_CODE,
        ),
        (
            base.stdlib_interface_version,
            xiao_intrinsics::CAPABILITY_STDOUT,
            0,
            xiao_diagnostics::CAPABILITY_UNAVAILABLE_CODE,
        ),
    ] {
        let mut index = base.clone();
        index.stdlib_interface_version = version;
        index.required_capabilities = capabilities;
        let bytes = raw_index_archive(&index.encode().unwrap(), &object);
        let mut events = Vec::new();
        let error = run_archive_with_event_observer(
            &bytes,
            XarRunOptions {
                capabilities: context,
                ..Default::default()
            },
            |event| events.push(format!("{event:?}")),
        )
        .unwrap_err();
        assert_eq!(error.code(), expected, "{error:?}");
        assert!(events.is_empty());
    }
    // 同样的损坏载荷在元数据兼容时仍必须被完整校验拒绝。
    let bytes = raw_index_archive(&base.encode().unwrap(), &object);
    assert!(decode_xar(&bytes).is_err());
}

#[test]
/// 索引不能以当前语言版本伪装另一语言版本的字节码；写入和读取均须拒绝。
fn review_payload_language_matches_archive_index() {
    let mut file = xiao_bytecode::decode_xiaoc(&empty_xiaoc_with_optional_sections()).unwrap();
    file.metadata.language_version = "0.2.0".to_owned();
    file.program.abi.language_version = "0.2.0".to_owned();
    let object = XarObject::from_bytes(
        ObjectKind::Xiaoc,
        xiao_bytecode::encode_xiaoc(&file.program, file.metadata).unwrap(),
    );
    let index = review_index_for_xiaoc(&object);
    let encoded = encode_xar(&index, std::slice::from_ref(&object));
    let decoded = decode_xar(&raw_index_archive(&index.encode().unwrap(), &object));
    assert!(encoded.is_err(), "编码器接受了伪装的语言版本");
    assert!(decoded.is_err(), "读取器接受了伪装的语言版本");
    assert_eq!(
        XarRunError::from(encoded.unwrap_err()).code(),
        ARCHIVE_VERSION_INCOMPATIBLE_CODE
    );
    assert_eq!(
        XarRunError::from(decoded.unwrap_err()).code(),
        ARCHIVE_VERSION_INCOMPATIBLE_CODE
    );
}

#[test]
/// 读取器同样执行写入器既有的调试激活位和独立调试/源码对象约束。
fn review_payload_debug_contract_is_symmetric() {
    let file = xiao_bytecode::decode_xiaoc(&empty_xiaoc_with_optional_sections()).unwrap();
    let debug = XarObject::from_bytes(
        ObjectKind::Xiaoc,
        xiao_bytecode::encode_xiaoc(&file.program, file.metadata.clone().with_debug("review"))
            .unwrap(),
    );
    let inline = XarObject::from_bytes(ObjectKind::Xiaoc, empty_xiaoc_with_optional_sections());
    for object in [debug, inline] {
        let index = review_index_for_xiaoc(&object);
        assert!(encode_xar(&index, std::slice::from_ref(&object)).is_err());
        assert!(
            decode_xar(&raw_index_archive(&index.encode().unwrap(), &object)).is_err(),
            "读取器绕过调试约束"
        );
    }
}

#[test]
/// 入口映射和非零长度也属于已有写入契约，读取不能绕过。
fn review_index_entry_contract_is_symmetric() {
    let (mut missing, object) = index_for(b"entry");
    missing.entry = "missing.xiao".to_owned();
    let (empty, empty_object) = index_for(b"");
    for (index, object) in [(missing, object), (empty, empty_object)] {
        assert!(encode_xar(&index, std::slice::from_ref(&object)).is_err());
        assert!(
            decode_xar(&raw_index_archive(&index.encode().unwrap(), &object)).is_err(),
            "读取端绕过入口约束"
        );
    }
}

#[test]
/// 外层 ZIP 完整时，未知能力仍在索引阶段报 011，旧归档按 007 拒绝且不执行。
fn archive_requirements_fail_before_entry_resolution_or_vm_events() {
    let (mut index, object) = index_for(b"not an executable entry");
    index.runtime_abi_min = xiao_runtime_abi::ABI_ENCODED_VERSION;
    index.runtime_abi_max = xiao_runtime_abi::ABI_ENCODED_VERSION;
    let canonical = index.encode().unwrap();
    let mut unknown = canonical.clone();
    assert_eq!(unknown.pop(), Some(0));
    unknown.extend_from_slice(&[0x80, 0x01]); // 字段 16 的值为 bit 7。
    let mut legacy = canonical.clone();
    let fields = legacy
        .windows(2)
        .position(|window| window == [0x68, 1])
        .unwrap();
    legacy.truncate(fields);
    let feature = legacy
        .windows(2)
        .position(|window| window == [0x30, 1])
        .unwrap();
    legacy[feature + 1] = 0;
    for (bytes, code) in [
        (&unknown, ARCHIVE_INVALID_REQUIREMENTS_CODE),
        (&legacy, ARCHIVE_VERSION_INCOMPATIBLE_CODE),
    ] {
        let archive = raw_index_archive(bytes, &object);
        let mut events = Vec::new();
        let error = run_archive_with_event_observer(&archive, XarRunOptions::default(), |event| {
            events.push(format!("{event:?}"))
        })
        .unwrap_err();
        assert_eq!(error.code(), code, "{error:?}");
        assert!(events.is_empty());
    }
    // 此入口是 Source 对象，007 必须先于入口类型的 009。
    index.stdlib_interface_version += 1;
    let archive = encode_xar(&index, &[object]).unwrap();
    let error = run_archive(&archive, XarRunOptions::default()).unwrap_err();
    assert_eq!(error.code(), ARCHIVE_VERSION_INCOMPATIBLE_CODE);
    assert!(error.message().contains("标准库接口版本"));
}

fn empty_xiaoc_with_optional_sections() -> Vec<u8> {
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
    xiao_bytecode::encode_xiaoc_with_options(
        &program,
        XiaocMetadata::default(),
        XiaocOptions {
            debug_symbols: Some(b"debug-symbols".to_vec()),
            source: Some(b"source-text".to_vec()),
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn deterministic_round_trip_and_index_store() {
    let (index, object) = index_for(b"hello hello hello hello");
    let first = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
    let second = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
    assert_eq!(first, second);
    let archive = decode_xar(&first).unwrap();
    assert_eq!(archive.index(), &index);
    assert_eq!(archive.read_entry().unwrap(), object.bytes);
    assert_eq!(archive.members()[0].path, XAR_INDEX_PATH);
    assert_eq!(archive.members()[0].compression, CompressionMethod::Store);
    assert!(toolchain_fingerprint().contains("flate2=1.1.10"));
}

#[test]
fn zip64_round_trip_and_equal_compression_uses_store() {
    let (index, object) = index_for(b"a");
    let bytes = encode_xar_with_options(
        &index,
        std::slice::from_ref(&object),
        XarBuildOptions {
            force_zip64: true,
            ..Default::default()
        },
    )
    .unwrap();
    let archive = decode_xar(&bytes).unwrap();
    assert_eq!(archive.read_entry().unwrap(), b"a");
    assert!(
        archive
            .members()
            .iter()
            .all(|member| member.compressed_size > 0)
    );
    assert!(
        archive
            .members()
            .iter()
            .find(|member| member.path.starts_with("objects/"))
            .is_some_and(|member| member.compression == CompressionMethod::Store)
    );
}

#[test]
fn rejects_path_traversal_and_truncation() {
    let (mut index, object) = index_for(b"payload");
    index.entry = "../main.xiao".to_owned();
    assert!(encode_xar(&index, std::slice::from_ref(&object)).is_err());
    let (index, object) = index_for(b"payload");
    let bytes = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
    for length in 0..bytes.len() {
        assert!(decode_xar(&bytes[..length]).is_err());
    }
}

#[test]
fn rejects_duplicate_objects_absolute_paths_and_zip_bombs() {
    let (index, object) = index_for(b"payload");
    let duplicate = encode_xar(&index, &[object.clone(), object.clone()]);
    assert!(matches!(duplicate, Err(XarError::DuplicateObject(..))));

    let mut unsafe_index = index.clone();
    unsafe_index.entry = "C:/main.xiao".to_owned();
    assert!(encode_xar(&unsafe_index, std::slice::from_ref(&object)).is_err());

    let bomb_object = XarObject::from_bytes(ObjectKind::Source, vec![0_u8; 5_000_000]);
    let mut bomb_index = index_for(bomb_object.bytes.as_slice()).0;
    bomb_index.entries[0].digest = bomb_object.digest;
    bomb_index.entries[0].length = bomb_object.bytes.len() as u64;
    let bomb = encode_xar(&bomb_index, std::slice::from_ref(&bomb_object)).unwrap();
    assert!(decode_xar(&bomb).is_err());
}

#[test]
fn rejects_encryption_and_multi_volume_flags() {
    let (index, object) = index_for(b"flag probe");
    let bytes = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
    let central = bytes
        .windows(4)
        .position(|window| window == ZIP_CENTRAL_SIGNATURE.to_le_bytes())
        .unwrap();
    let mut encrypted = bytes.clone();
    encrypted[central + 8..central + 10].copy_from_slice(&1_u16.to_le_bytes());
    assert!(decode_xar(&encrypted).is_err());

    let mut multi_volume = bytes;
    let eocd = multi_volume.len() - 22;
    multi_volume[eocd + 4..eocd + 6].copy_from_slice(&1_u16.to_le_bytes());
    assert!(decode_xar(&multi_volume).is_err());
}

#[test]
fn rejects_all_index_logical_path_boundary_forms() {
    for path in ["", "a//b", "a/./b", "a/../b", r"a\b", "/a/b", "C:/a"] {
        let (mut index, object) = index_for(b"path-boundary");
        index.entry = path.to_owned();
        index.entries[0].logical_path = path.to_owned();
        assert!(
            encode_xar(&index, std::slice::from_ref(&object)).is_err(),
            "{path:?}"
        );
    }
    let (mut duplicate, object) = index_for(b"duplicate-logical");
    duplicate.entries.push(duplicate.entries[0].clone());
    assert!(encode_xar(&duplicate, std::slice::from_ref(&object)).is_err());
}

#[test]
fn rejects_missing_and_duplicate_index_headers() {
    let (index, object) = index_for(b"index-boundary");
    let mut missing = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
    let eocd = missing.len() - 22;
    missing[eocd + 8..eocd + 12].fill(0);
    missing[eocd + 12..eocd + 16].fill(0);
    assert!(matches!(
        decode_xar(&missing),
        Err(XarError::MissingMember(_))
    ));

    let mut duplicate = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
    let eocd = duplicate.len() - 22;
    duplicate[eocd + 10..eocd + 12].copy_from_slice(&3_u16.to_le_bytes());
    assert!(decode_xar(&duplicate).is_err());
}

#[test]
fn rejects_out_of_bounds_member_lengths_and_unknown_required_versions() {
    let (index, object) = index_for(b"length-boundary");
    let bytes = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
    let central = bytes
        .windows(4)
        .position(|window| window == ZIP_CENTRAL_SIGNATURE.to_le_bytes())
        .unwrap();
    let mut oversized = bytes.clone();
    oversized[central + 24..central + 28].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_xar(&oversized).is_err());

    let mut unknown_version = bytes;
    unknown_version[central + 6..central + 8].copy_from_slice(&46_u16.to_le_bytes());
    assert!(decode_xar(&unknown_version).is_err());
}

#[test]
fn explicit_resources_are_content_addressed_and_never_scanned() {
    let root = std::env::temp_dir().join(format!(
        "xiao-xar-resource-{}-{}",
        std::process::id(),
        NEXT_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::write(root.join("assets/logo.png"), b"logo").unwrap();
    std::fs::write(root.join("secret.txt"), b"must-not-be-scanned").unwrap();
    let resources = collect_resources(
        &root,
        &[ResourceDeclaration::new(
            "assets/logo.png",
            "assets/logo.png",
        )],
    )
    .unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].digest, Digest256::of_bytes(b"logo"));
    assert!(
        !resources
            .iter()
            .any(|resource| resource.logical_path == "secret.txt")
    );
    assert!(matches!(
        collect_resources(
            &root,
            &[ResourceDeclaration::new("assets/logo.png", "missing.bin")]
        ),
        Err(ResourceError::Diagnostic(diagnostic))
            if diagnostic.code() == MISSING_RESOURCE_CODE
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_resource_bridge_preserves_source_span() {
    let document = parse_config_project(&SourceFile::from_text(
        "[project]\nname = \"demo\"\nversion = \"1\"\n[resources]\n\"assets/a\" = \"assets/a\"\n",
    ))
    .unwrap();
    let declarations = resource_declarations_from_config(&document).unwrap();
    assert_eq!(declarations.len(), 1);
    assert!(declarations[0].span.is_some());

    let root = std::env::temp_dir().join(format!(
        "xiao-xar-config-resource-{}-{}",
        std::process::id(),
        NEXT_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::write(root.join("assets/a"), b"config-resource").unwrap();
    let resources = collect_config_resources(&root, &document).unwrap();
    assert_eq!(resources[0].bytes, b"config-resource");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resource_collection_side_effect_probe_excludes_credentials_and_environment_marker() {
    let root = std::env::temp_dir().join(format!(
        "xiao-xar-side-effect-probe-{}-{}",
        std::process::id(),
        NEXT_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::create_dir_all(root.join("xiao-home")).unwrap();
    let secret = b"XIAO_HOME_CREDENTIAL_SIDE_EFFECT_PROBE";
    std::fs::write(root.join("assets/declared"), b"declared").unwrap();
    std::fs::write(root.join("xiao-home/credentials"), secret).unwrap();
    let resources = collect_resources(
        &root,
        &[ResourceDeclaration::new(
            "assets/declared",
            "assets/declared",
        )],
    )
    .unwrap();
    let (mut index, entry_object) = index_for(b"entry");
    let resource_objects =
        append_resource_entries(&mut index, &resources, "main", "portable").unwrap();
    let mut objects = vec![entry_object];
    objects.extend(resource_objects);
    let archive = encode_xar(&index, &objects).unwrap();
    assert!(!archive.windows(secret.len()).any(|window| window == secret));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn archive_runner_checks_runtime_before_entry_execution() {
    let (index, object) = index_for(b"not-xiaoc-entry");
    let archive = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
    let version_error = run_archive(
        &archive,
        XarRunOptions {
            runtime_abi: 0,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(version_error.code(), ARCHIVE_VERSION_INCOMPATIBLE_CODE);
    let entry_error = run_archive(
        &archive,
        XarRunOptions {
            runtime_abi: xiao_runtime_abi::ABI_ENCODED_VERSION,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(entry_error.code(), ARCHIVE_VALIDATION_FAILED_CODE);
}

#[test]
fn archive_language_resolution_obeys_default_override_and_fallback() {
    let archive_default = resolve_language_locale("en-US", None);
    assert_eq!(archive_default.effective, "en-US");
    assert!(!archive_default.fallback);
    let explicit = resolve_language_locale("en-US", Some("zh-CN"));
    assert_eq!(explicit.effective, "zh-CN");
    assert!(!explicit.fallback);
    let fallback = resolve_language_locale("xx-XX", None);
    assert_eq!(fallback.effective, "zh-CN");
    assert!(fallback.fallback);
}

#[test]
fn identical_resources_share_one_physical_object_and_keep_two_mappings() {
    let (mut index, source) = index_for(b"entry");
    let digest = Digest256::of_bytes(b"same-resource");
    let resources = vec![
        CollectedResource {
            logical_path: "assets/a.bin".to_owned(),
            source_path: PathBuf::from("assets/a.bin"),
            digest,
            bytes: b"same-resource".to_vec(),
        },
        CollectedResource {
            logical_path: "assets/b.bin".to_owned(),
            source_path: PathBuf::from("assets/b.bin"),
            digest,
            bytes: b"same-resource".to_vec(),
        },
    ];
    let objects = append_resource_entries(&mut index, &resources, "main", "portable").unwrap();
    assert_eq!(objects.len(), 1);
    let mut all_objects = vec![source];
    all_objects.extend(objects);
    let archive = decode_xar(&encode_xar(&index, &all_objects).unwrap()).unwrap();
    assert_eq!(
        archive
            .members()
            .iter()
            .filter(|member| member.path.starts_with("objects/resource/"))
            .count(),
        1
    );
    assert_eq!(
        archive
            .index()
            .entries
            .iter()
            .filter(|entry| entry.object_kind == ObjectKind::Resource)
            .count(),
        2
    );
}

#[test]
fn resource_diagnostics_cover_duplicate_digest_and_undeclared_access() {
    let root = std::env::temp_dir().join(format!(
        "xiao-xar-duplicate-{}-{}",
        std::process::id(),
        NEXT_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a"), b"a").unwrap();
    let duplicate = collect_resources(
        &root,
        &[
            ResourceDeclaration::new("a", "a"),
            ResourceDeclaration::new("a", "b"),
        ],
    )
    .unwrap_err();
    assert_eq!(
        duplicate.diagnostic().unwrap().code(),
        DUPLICATE_LOGICAL_PATH_CODE
    );
    std::fs::remove_dir_all(root).unwrap();

    let mismatch = validate_resource_digest(
        "assets/a",
        Digest256::of_bytes(b"expected"),
        b"actual",
        None,
    )
    .unwrap_err();
    assert_eq!(
        mismatch.diagnostic().unwrap().code(),
        RESOURCE_DIGEST_MISMATCH_CODE
    );
    let undeclared = require_declared_resource("assets/a", &[], None).unwrap_err();
    assert_eq!(
        undeclared.diagnostic().unwrap().code(),
        UNDECLARED_RESOURCE_ACCESS_CODE
    );
}

#[test]
fn debug_and_source_objects_are_independent_and_standard_xiaoc_is_trimmed() {
    let input = empty_xiaoc_with_optional_sections();
    let standard = prepare_xiaoc_for_archive(&input, DiagnosticObjectOptions::default())
        .expect("标准 .xiaoc 应可重建");
    let standard_file = xiao_bytecode::decode_xiaoc(&standard).unwrap();
    assert!(standard_file.sections.iter().all(|section| !matches!(
        section.kind,
        Some(xiao_bytecode::XiaocSectionKind::DebugSymbols)
            | Some(xiao_bytecode::XiaocSectionKind::Source)
    )));

    let both = collect_diagnostic_objects(
        &input,
        DiagnosticObjectOptions {
            include_debug: true,
            include_source: true,
        },
    )
    .unwrap();
    assert_eq!(
        both.debug.as_ref().map(|object| object.kind),
        Some(ObjectKind::Debug)
    );
    assert_eq!(
        both.source.as_ref().map(|object| object.kind),
        Some(ObjectKind::Source)
    );
    let only_debug = collect_diagnostic_objects(
        &input,
        DiagnosticObjectOptions {
            include_debug: true,
            include_source: false,
        },
    )
    .unwrap();
    assert!(only_debug.debug.is_some());
    assert!(only_debug.source.is_none());
}

#[test]
fn standard_and_debug_packages_have_identical_runtime_observations() {
    let input = empty_xiaoc_with_optional_sections();
    let standard = prepare_xiaoc_for_archive(&input, DiagnosticObjectOptions::default())
        .expect("标准包 .xiaoc");
    let debug = prepare_xiaoc_for_archive(
        &input,
        DiagnosticObjectOptions {
            include_debug: true,
            include_source: true,
        },
    )
    .expect("调试包 .xiaoc");
    let standard_out =
        xiao_vm::run_xiaoc(&standard, xiao_vm::VmOptions::default()).expect("标准包应可执行");
    let debug_out =
        xiao_vm::run_xiaoc(&debug, xiao_vm::VmOptions::default()).expect("调试包应可执行");
    assert_eq!(
        standard_out.result.error_code(),
        debug_out.result.error_code()
    );
    assert_eq!(
        format!("{:?}", standard_out.value),
        format!("{:?}", debug_out.value)
    );
    assert_eq!(standard_out.metrics, debug_out.metrics);
    assert_eq!(standard_out.events, debug_out.events);
    assert_eq!(standard_out.dropped_events, debug_out.dropped_events);
    assert_eq!(
        standard_out.report.as_ref().map(|report| (
            &report.class,
            &report.code,
            &report.message_id,
            &report.params
        )),
        debug_out.report.as_ref().map(|report| (
            &report.class,
            &report.code,
            &report.message_id,
            &report.params
        ))
    );
}
