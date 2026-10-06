//! 19B 兼容矩阵逐格回归。

use xiao_artifacts::{ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR};
use xiao_bytecode::{XiaocMetadata, decode_xiaoc, encode_xiaoc, lower_program};
use xiao_driver::{
    CompatibilityAction, CompatibilityAxis, CompatibilityEvidence, CompatibilityStatus,
    FrontendCompiler, FrontendRequest, TargetSupportStatus, compatibility_matrix,
    current_compatibility_versions, target_support_matrix,
};
use xiao_runtime_abi::ABI_ENCODED_VERSION;

#[test]
/// 每格都必须携带版本标签和四选一行为。
fn every_generated_cell_has_an_explicit_action_and_stable_axis() {
    let cells = compatibility_matrix();
    assert_eq!(cells.len(), 18);
    for cell in &cells {
        assert!(!cell.artifact.is_empty());
        assert!(!cell.runtime.is_empty());
        assert!(!cell.axis.as_str().is_empty());
        match &cell.action {
            CompatibilityAction::Reject { code } => assert!(!code.is_empty()),
            CompatibilityAction::Direct
            | CompatibilityAction::Regenerate
            | CompatibilityAction::Migrate => {}
        }
        assert!(matches!(
            cell.evidence,
            CompatibilityEvidence::Verified | CompatibilityEvidence::Unverified
        ));
        assert_eq!(cell.status, CompatibilityStatus::Current);
    }
}

#[test]
/// `.xiaoc` 主/次版本格子必须对应真实编码器产物和真实解码器错误。
fn xiaoc_version_cells_are_backed_by_the_authoritative_codec() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text("value = 1\n"))
        .expect("源码应通过前端");
    let bytes = encode_xiaoc(
        &lower_program(artifact.ir()),
        XiaocMetadata::new("compatibility-matrix"),
    )
    .expect("应编码合法 xiaoc");
    assert!(decode_xiaoc(&bytes).is_ok());

    let current = current_compatibility_versions();
    let minor_cell = compatibility_matrix()
        .into_iter()
        .find(|cell| {
            cell.axis == CompatibilityAxis::XiaocFormat
                && cell.artifact == format!("{}.{}", current.xiaoc.0, current.xiaoc.1 + 1)
        })
        .expect("必须有 xiaoc 次版本格子");
    assert_eq!(minor_cell.evidence, CompatibilityEvidence::Verified);
    let mut future_minor = bytes.clone();
    future_minor[10..12].copy_from_slice(&(current.xiaoc.1 + 1).to_le_bytes());
    assert!(matches!(
        decode_xiaoc(&future_minor),
        Err(xiao_bytecode::XiaocError::UnsupportedMinorVersion(_))
    ));

    let major_cell = compatibility_matrix()
        .into_iter()
        .find(|cell| {
            cell.axis == CompatibilityAxis::XiaocFormat
                && cell.artifact == format!("{}.{}", current.xiaoc.0 + 1, current.xiaoc.1)
        })
        .expect("必须有 xiaoc 主版本格子");
    assert_eq!(major_cell.evidence, CompatibilityEvidence::Verified);
    let mut future_major = bytes;
    future_major[8..10].copy_from_slice(&(current.xiaoc.0 + 1).to_le_bytes());
    assert!(matches!(
        decode_xiaoc(&future_major),
        Err(xiao_bytecode::XiaocError::UnsupportedMajorVersion(_))
    ));
}

#[test]
/// 索引主版本格子必须对应真实 ArchiveIndex 编解码器的拒绝行为。
fn index_major_cell_is_backed_by_the_authoritative_codec() {
    let index = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: Vec::new(),
        dependency_lock_digest: "0".repeat(64),
        runtime_abi_min: ABI_ENCODED_VERSION,
        runtime_abi_max: ABI_ENCODED_VERSION,
        platform: "portable".to_owned(),
        debug_activation: false,
        language_locale: "zh-CN".to_owned(),
    };
    let mut bytes = index.encode().expect("应编码合法归档索引");
    assert!(ArchiveIndex::decode(&bytes).is_ok());
    bytes[1] = (INDEX_SCHEMA_MAJOR + 1) as u8;
    assert!(matches!(
        ArchiveIndex::decode(&bytes),
        Err(xiao_artifacts::ArtifactError::UnsupportedIndexVersion { .. })
    ));

    let cell = compatibility_matrix()
        .into_iter()
        .find(|cell| {
            cell.axis == CompatibilityAxis::IndexSchema
                && cell.artifact == format!("{}.{}", INDEX_SCHEMA_MAJOR + 1, INDEX_SCHEMA_MINOR)
        })
        .expect("必须有索引主版本格子");
    assert_eq!(cell.evidence, CompatibilityEvidence::Verified);
}

#[test]
/// 不支持目标必须出现在代码生成的平台清单中，并与未验证目标区分。
fn target_support_matrix_distinguishes_unsupported_and_unverified() {
    let cells = target_support_matrix();
    assert!(cells.iter().any(|cell| {
        cell.target == "riscv64-unknown-linux-gnu"
            && cell.status == TargetSupportStatus::Unsupported
    }));
    assert!(cells.iter().any(|cell| {
        cell.target == "x86_64-pc-windows-msvc"
            && matches!(
                cell.status,
                TargetSupportStatus::Supported | TargetSupportStatus::Unverified
            )
    }));
}

#[test]
/// 当前版本直接运行，优化指纹变化重新生成。
fn current_version_cells_are_direct_and_mismatches_are_explicit() {
    let versions = current_compatibility_versions();
    let xiaoc = format!("{}.{}", versions.xiaoc.0, versions.xiaoc.1);
    assert!(compatibility_matrix().iter().any(|cell| {
        cell.axis == CompatibilityAxis::XiaocFormat
            && cell.artifact == xiaoc
            && cell.runtime == xiaoc
            && cell.action == CompatibilityAction::Direct
    }));
    assert!(compatibility_matrix().iter().any(|cell| {
        cell.axis == CompatibilityAxis::OptimizationFingerprint
            && cell.action == CompatibilityAction::Regenerate
    }));
}
