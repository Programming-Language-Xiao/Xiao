//! 19C 体积回归：固定输入下的 `.xiaoc`/`.xar` 提交基线。

use xiao_artifacts::{ArchiveEntry, ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR, ObjectKind};
use xiao_bytecode::{XiaocMetadata, encode_xiaoc, lower_program};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_runtime_abi::ABI_ENCODED_VERSION;
use xiao_xar::{XarObject, encode_xar};

/// 生成基线时使用的稳定源码，修改源码必须同时重新审查该回归上限。
const SIZE_REGRESSION_SOURCE: &str = "print(\"19C size baseline\")\n";
/// 2026-10-06 Windows 基线：固定源码的规范 `.xiaoc` 字节数。
const XIAOC_BASELINE_BYTES: usize = 1_256;
/// 2026-10-06 Windows 基线：同一对象的规范 `.xar` 字节数。
const XAR_BASELINE_BYTES: usize = 1_151;

fn artifacts() -> (Vec<u8>, Vec<u8>) {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(SIZE_REGRESSION_SOURCE))
        .expect("体积基线源码应通过前端");
    let xiaoc = encode_xiaoc(
        &lower_program(artifact.ir()),
        XiaocMetadata::new("19c-size-baseline").with_optimization_fingerprint("19c-baseline"),
    )
    .expect("应编码体积基线 xiaoc");
    let object = XarObject::from_bytes(ObjectKind::Xiaoc, xiaoc.clone());
    let index = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: vec![ArchiveEntry {
            logical_path: "main.xiaoc".to_owned(),
            object_kind: ObjectKind::Xiaoc,
            digest: object.digest,
            module: "19c-size-baseline".to_owned(),
            target: "portable".to_owned(),
            length: object.bytes.len() as u64,
        }],
        dependency_lock_digest: "0".repeat(64),
        runtime_abi_min: ABI_ENCODED_VERSION,
        runtime_abi_max: ABI_ENCODED_VERSION,
        platform: "portable".to_owned(),
        debug_activation: false,
        language_locale: "zh-CN".to_owned(),
    };
    let xar = encode_xar(&index, &[object]).expect("应编码体积基线 xar");
    (xiaoc, xar)
}

#[test]
fn deterministic_artifacts_do_not_grow_past_committed_baseline() {
    let (xiaoc, xar) = artifacts();
    assert_eq!(xiaoc.len(), XIAOC_BASELINE_BYTES, "xiaoc 基线变化需显式审查");
    assert_eq!(xar.len(), XAR_BASELINE_BYTES, "xar 基线变化需显式审查");
}
