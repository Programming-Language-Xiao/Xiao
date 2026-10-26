//! 22C 真实源码到 `.xiaoc`/`.xar` 的并发 feature 传播回归。

use xiao_artifacts::{ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR};
use xiao_bytecode::{XiaocMetadata, decode_xiaoc, encode_xiaoc, lower_program};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_runtime_abi::{
    ABI_ENCODED_VERSION, REQUIRED_FEATURE_CANCEL_WAIT, REQUIRED_FEATURE_MULTI_RUNTIME,
    REQUIRED_FEATURE_SERIALIZATION_V1, REQUIRED_FEATURE_THREADS,
};
use xiao_xar::{DiagnosticObjectOptions, XarBuilder, decode_xar};

fn concurrent_xiaoc() -> Vec<u8> {
    let source = "from std.concurrent import runtime_create, spawn, join\n\
def work(int value) -> int\n    return value\n\
rt = runtime_create()\nthread = spawn(rt, work, 1)\nresult = join(thread)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("并发源码应通过前端");
    let lowered = lower_program(&artifact.ir);
    encode_xiaoc(&lowered, XiaocMetadata::new("concurrent-main"))
        .expect("并发源码应编码为 `.xiaoc`")
}

#[test]
fn source_concurrency_types_reach_xiaoc_required_features() {
    let file = decode_xiaoc(&concurrent_xiaoc()).expect("真实并发源码产物应可解码");
    assert_eq!(
        file.header.required_features,
        xiao_runtime_abi::XIAOC_REQUIRED_FEATURE_THREADS
            | xiao_runtime_abi::XIAOC_REQUIRED_FEATURE_MULTI_RUNTIME
            | xiao_runtime_abi::XIAOC_REQUIRED_FEATURE_SERIALIZATION_V1
            | xiao_runtime_abi::XIAOC_REQUIRED_FEATURE_CANCEL_WAIT
    );
    assert_eq!(
        file.header.runtime_abi_min as u64, ABI_ENCODED_VERSION,
        "`.xiaoc` header 必须与 TAC Runtime ABI 使用同一 1.17 编码"
    );
    assert_eq!(
        file.program.abi.runtime_abi_version, file.header.runtime_abi_min,
        "TAC ABI 字段与 `.xiaoc` header 必须一致"
    );
}

#[test]
fn source_features_are_shifted_into_xar_concurrency_bits() {
    let xiaoc = concurrent_xiaoc();
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
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    };
    let mut builder = XarBuilder::new(index);
    builder
        .add_xiaoc(
            "main.xiaoc",
            "main",
            "portable",
            &xiaoc,
            DiagnosticObjectOptions::default(),
        )
        .expect("`.xar` 应接收并发 `.xiaoc`");
    let archive = decode_xar(&builder.build().expect("归档应编码")).expect("归档应解码");
    assert_eq!(
        archive.index().required_features,
        REQUIRED_FEATURE_THREADS
            | REQUIRED_FEATURE_MULTI_RUNTIME
            | REQUIRED_FEATURE_SERIALIZATION_V1
            | REQUIRED_FEATURE_CANCEL_WAIT
    );
}
