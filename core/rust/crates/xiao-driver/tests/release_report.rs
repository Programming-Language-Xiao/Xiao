//! 19B 发布报告字段回归。

use std::fs;

use xiao_artifacts::{
    ArchiveEntry, ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR, ObjectKind,
};
use xiao_bytecode::{XiaocMetadata, encode_xiaoc, lower_program};
use xiao_driver::{
    CORE_VERSION, FrontendCompiler, FrontendRequest, PROTOCOL_VERSION, ProtocolRequest,
    ProtocolResponse, dispatch,
};
use xiao_runtime_abi::ABI_ENCODED_VERSION;
use xiao_xar::{XarObject, encode_xar};

#[test]
/// `.xiaoc` 报告必须包含 Rust 计算的完整性和可复现性字段。
fn verify_detail_reports_rust_computed_integrity_and_reproducibility_fields() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text("value = 1\n"))
        .expect("源码应通过前端");
    let bytes = encode_xiaoc(
        &lower_program(artifact.ir()),
        XiaocMetadata::new("release-report"),
    )
    .expect("应编码 xiaoc");
    let path = std::env::temp_dir().join(format!("xiao-19b-report-{}.xiaoc", std::process::id()));
    fs::write(&path, bytes).expect("写入产物");
    let response = dispatch(ProtocolRequest::Verify {
        request_id: "release-report".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        path: path.display().to_string(),
        detail: true,
    });
    let ProtocolResponse::Result {
        value: Some(value), ..
    } = response
    else {
        panic!("verify detail 应返回结果");
    };
    let summary: serde_json::Value = serde_json::from_str(&value.value).expect("验证摘要 JSON");
    let report = &summary["release_report"];
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["signature"]["status"], "unsigned");
    assert_eq!(report["reproducibility"]["status"], "not-measured");
    assert_eq!(
        report["artifact_sha256"].as_str().unwrap_or_default().len(),
        64
    );
    assert_eq!(report["xiaoc_objects"].as_array().map(Vec::len), Some(1));
    let _ = fs::remove_file(path);
}

#[test]
/// `.xar` 报告必须列出成员和内容寻址对象摘要。
fn verify_detail_reports_archive_members_and_object_digests() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text("value = 1\n"))
        .expect("源码应通过前端");
    let xiaoc = encode_xiaoc(
        &lower_program(artifact.ir()),
        XiaocMetadata::new("archive-report"),
    )
    .expect("应编码 xiaoc");
    let object = XarObject::from_bytes(ObjectKind::Xiaoc, xiaoc);
    let index = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: vec![ArchiveEntry {
            logical_path: "main.xiaoc".to_owned(),
            object_kind: ObjectKind::Xiaoc,
            digest: object.digest,
            module: "archive-report".to_owned(),
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
    let bytes = encode_xar(&index, &[object]).expect("应编码 xar");
    let path = std::env::temp_dir().join(format!("xiao-19b-report-{}.xar", std::process::id()));
    fs::write(&path, bytes).expect("写入归档");
    let response = dispatch(ProtocolRequest::Verify {
        request_id: "release-report-xar".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        path: path.display().to_string(),
        detail: true,
    });
    let ProtocolResponse::Result {
        value: Some(value), ..
    } = response
    else {
        panic!("verify detail 应返回结果");
    };
    let summary: serde_json::Value = serde_json::from_str(&value.value).expect("验证摘要 JSON");
    let report = &summary["release_report"];
    assert_eq!(report["target_platform"], "portable");
    assert_eq!(report["optimization_fingerprint"], "xiao-opt-unset");
    assert_eq!(report["archive_members"].as_array().map(Vec::len), Some(2));
    assert_eq!(report["xiaoc_objects"].as_array().map(Vec::len), Some(1));
    let _ = fs::remove_file(path);
}
