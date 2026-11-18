//! B5精确ABI1.19、固定旧基线拒绝与同版本容器字段回归。
use std::path::Path;
use xiao_codegen_llvm::{CODEGEN_VERSION, NativeMetadata, NativeRuntimeSupport, TargetDescription};
use xiao_driver::{ArchiveBuildRequest, CancellationToken, FrontendCompiler, FrontendRequest};
use xiao_runtime_abi::{ABI_ENCODED_VERSION, ABI_MINOR_VERSION};

fn fixture(name: &str) -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    std::fs::read(root.join("tests/fixtures/compatibility/b5").join(name)).unwrap()
}

#[test]
/// 两类旧字节由各自旧工具真实生成，不通过修改当前头模拟历史。
fn frozen_abi18_baselines_are_rejected_before_entry() {
    for baseline in ["b1", "b4"] {
        let bytes = fixture(&format!("{baseline}-main.xiaoc"));
        let error = xiao_bytecode::decode_xiaoc(&bytes).unwrap_err();
        assert!(
            matches!(error,xiao_bytecode::XiaocError::InvalidField{ref field,..} if field=="runtime_abi")
        );
        let bytes = fixture(&format!("{baseline}-main.xar"));
        assert_eq!(
            xiao_driver::run_archive(&bytes, Default::default())
                .unwrap_err()
                .code(),
            "X17-XAR-007"
        );
        let meta = NativeMetadata::decode(&fixture(&format!("{baseline}-main.meta"))).unwrap();
        assert_eq!(
            meta.runtime_abi_min,
            xiao_runtime_abi::encoded_abi_version(1, 18)
        );
        assert_eq!(
            meta.generator_version,
            if baseline == "b1" { 20 } else { 21 }
        );
        assert_eq!(
            meta.validate(
                &TargetDescription::host(),
                &NativeRuntimeSupport::default(),
                0
            )
            .unwrap_err()
            .code,
            "X17-XAR-007"
        );
    }
}

#[test]
/// 新头、指令流、归档索引和原生元数据使用同一个精确版本，schema保持既有常量。
fn current_artifacts_bind_exact_abi19_and_current_codegen() {
    assert_eq!(ABI_MINOR_VERSION, 19);
    assert_eq!(CODEGEN_VERSION, 22);
    let source = "print(\"B5-current\")\n";
    let product = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap();
    let tac = xiao_bytecode::lower_program(&product.ir);
    let bytes = xiao_bytecode::encode_xiaoc(&tac, Default::default()).unwrap();
    let file = xiao_bytecode::decode_xiaoc(&bytes).unwrap();
    assert_eq!(u64::from(file.header.runtime_abi_min), ABI_ENCODED_VERSION);
    assert_eq!(file.header.runtime_abi_min, file.header.runtime_abi_max);
    assert_eq!(
        u64::from(file.program.abi.runtime_abi_version),
        ABI_ENCODED_VERSION
    );
    let metadata = NativeMetadata::for_program(&product.ir, &TargetDescription::host()).unwrap();
    assert_eq!(metadata.generator_version, CODEGEN_VERSION);
    assert_eq!(metadata.runtime_abi_min, ABI_ENCODED_VERSION);
    assert_eq!(metadata.runtime_abi_max, ABI_ENCODED_VERSION);
    assert_eq!(metadata.stdlib.runtime_abi_minor, 19);
    assert_eq!(metadata.stdlib.stdlib_interface_version, 1);
    let path = std::env::temp_dir().join(format!("b5-current-{}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    let input = path.join("main.xiao");
    std::fs::write(&input, source).unwrap();
    let mut request = ArchiveBuildRequest::new(input);
    request.cache_root = Some(path.join("cache"));
    request.output = Some(path.join("main.xar"));
    let built = xiao_driver::build_archive(&request, &CancellationToken::new()).unwrap();
    assert_eq!(built.index.runtime_abi_min, ABI_ENCODED_VERSION);
    assert_eq!(built.index.runtime_abi_max, ABI_ENCODED_VERSION);
    assert_eq!(
        (built.index.schema_major, built.index.schema_minor),
        (
            xiao_artifacts::INDEX_SCHEMA_MAJOR,
            xiao_artifacts::INDEX_SCHEMA_MINOR
        )
    );
    assert!(
        xiao_driver::run_archive(&std::fs::read(built.output).unwrap(), Default::default())
            .unwrap()
            .result
            .is_success()
    );
}
