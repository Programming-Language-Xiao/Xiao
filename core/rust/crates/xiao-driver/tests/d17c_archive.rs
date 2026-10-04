//! 17C/17D 归档运行的成功路径和校验前拒绝回归。

use xiao_artifacts::{
    ArchiveEntry, ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR, ObjectKind,
};
use xiao_bytecode::{TAC_RUNTIME_ABI_VERSION, XiaocMetadata, encode_xiaoc, lower_program};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_runtime_abi::ABI_ENCODED_VERSION;
use xiao_xar::{
    XarObject, XarRunOptions, audit_archive, encode_xar, run_archive,
    run_archive_with_event_observer,
};

fn archive_for(source: &str) -> Vec<u8> {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("源码应通过前端");
    let program = lower_program(&artifact.ir);
    assert_eq!(program.abi.runtime_abi_version, TAC_RUNTIME_ABI_VERSION);
    let xiaoc = encode_xiaoc(&program, XiaocMetadata::new("main")).expect("应编码 .xiaoc");
    let object = XarObject::from_bytes(ObjectKind::Xiaoc, xiaoc);
    let index = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: vec![ArchiveEntry {
            logical_path: "main.xiaoc".to_owned(),
            object_kind: ObjectKind::Xiaoc,
            digest: object.digest,
            module: "main".to_owned(),
            target: "portable".to_owned(),
            length: object.bytes.len() as u64,
        }],
        dependency_lock_digest: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_owned(),
        runtime_abi_min: ABI_ENCODED_VERSION,
        runtime_abi_max: ABI_ENCODED_VERSION,
        platform: "portable".to_owned(),
        debug_activation: false,
        language_locale: "zh-CN".to_owned(),
    };
    encode_xar(&index, &[object]).expect("应编码归档")
}

#[test]
fn archive_success_preserves_intrinsic_output_and_success_result() {
    let archive = archive_for("print(\"hello world!\")\n");
    let audit = audit_archive(
        &archive,
        &XarRunOptions {
            runtime_abi: ABI_ENCODED_VERSION,
            ..Default::default()
        },
    )
    .expect("审计记录应可生成");
    let audit_json = audit.to_json_bytes().expect("审计记录应可序列化");
    assert!(
        String::from_utf8(audit_json)
            .unwrap()
            .contains("archive_digest")
    );
    let outcome = run_archive(
        &archive,
        XarRunOptions {
            runtime_abi: ABI_ENCODED_VERSION,
            ..Default::default()
        },
    )
    .expect("归档应成功运行");
    assert!(outcome.result.is_success());
    assert!(outcome.events.iter().any(|event| {
        matches!(event, xiao_vm::VmEvent::IntrinsicOutput { text } if text == "hello world!\n")
    }));
}

#[test]
fn invalid_archive_produces_no_execution_events() {
    let mut archive = archive_for("print(\"must not run\")\n");
    let marker = b"objects/xiaoc/";
    let object_position = archive
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("归档对象路径");
    let digest_position = object_position + marker.len() + 10;
    archive[digest_position] ^= 1;
    let mut events = Vec::new();
    let result = run_archive_with_event_observer(
        &archive,
        XarRunOptions {
            runtime_abi: ABI_ENCODED_VERSION,
            ..Default::default()
        },
        |event| events.push(format!("{event:?}")),
    );
    assert!(result.is_err());
    assert!(events.is_empty());
}
