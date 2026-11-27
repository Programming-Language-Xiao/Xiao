//! 两个 env 原生测试共用的固定工具链、任务目录和产物回执。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use xiao_artifacts::Digest256;
use xiao_codegen_llvm::{
    ArtifactVerification, CODEGEN_VERSION, NativeArtifact, ObjectFormat, TargetDescription,
    Toolchain,
};
use xiao_driver::NativeBuildRequest;
use xiao_runtime_abi::{NATIVE_ENVIRONMENT_NONCE_ENV, NATIVE_ENVIRONMENT_STDIN_PREFIX};

/// 合成启动环境用于直接验证接纳错误与显式快照优先级。
pub(super) fn run_injected(command: &mut Command, data: &[u8]) -> Output {
    const NONCE: &str = "0123456789abcdef0123456789abcdef";
    let mut child = command
        .arg(format!("{NATIVE_ENVIRONMENT_STDIN_PREFIX}{NONCE}"))
        .env(NATIVE_ENVIRONMENT_NONCE_ENV, NONCE)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let written = child.stdin.take().unwrap().write_all(data);
    let output = child.wait_with_output().unwrap();
    if output.status.success() {
        written.unwrap();
    }
    output
}

pub(super) fn toolchain(target: &TargetDescription) -> Toolchain {
    let required = |name: &str| PathBuf::from(std::env::var_os(name).expect(name));
    assert_eq!(std::env::var("XIAO_TARGET_TRIPLE").unwrap(), target.triple);
    Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries("rustc", target)
        .unwrap()
        .probe_versions()
        .unwrap()
}

pub(super) fn test_dir() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base =
        std::env::var_os("XIAO_B3_ENV_TEST_ROOT").map_or_else(std::env::temp_dir, PathBuf::from);
    loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = base.join(format!("xiao-b3-env-{}-{id}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("{path:?}: {error}"),
        }
    }
}

fn digest(path: &Path) -> String {
    Digest256::of_bytes(&fs::read(path).unwrap()).as_hex()
}

/// 从产品构建回执取验证结果，并用磁盘原件核对链接器绑定的摘要。
pub(super) fn record(
    case: &str,
    source: &str,
    request: &NativeBuildRequest,
    artifact: &NativeArtifact,
    output: &Output,
) {
    let composition = &artifact.artifact_runtime;
    let metadata = artifact.native_metadata.as_ref().unwrap();
    let artifact_sha256 = digest(&artifact.executable);
    let coff = composition.coff_link_identity.as_ref();
    println!(
        "B3-4-ENV-NATIVE {}",
        serde_json::json!({
            "case": case,
            "target": metadata.target,
            "optimization": artifact.optimization_level,
            "capabilities": request.codegen_options.capabilities,
            "source_sha256": Digest256::of_bytes(source.as_bytes()).as_hex(),
            "path": artifact.executable,
            "artifact_sha256": artifact_sha256,
            "coff_map": coff.map(|binding| &binding.map_path),
            "coff_map_sha256": coff.map(|binding| &binding.map_sha256),
            "coff_artifact_sha256": coff.map(|binding| &binding.artifact_sha256),
            "object_format": format!("{:?}", composition.object_format),
            "verification": composition.verification.as_str(),
            "declared_components": composition.declared_components,
            "observed_components": composition.observed_components,
            "unclassified_runtime_symbols": composition.unclassified_runtime_symbols,
            "runtime_symbols": composition.runtime_symbols,
            "dependencies": composition.dependencies,
            "generator": metadata.generator_version,
            "runtime_abi": metadata.runtime_abi_min,
            "stdlib_interface": metadata.stdlib.stdlib_interface_version,
            "codegen_fingerprint": artifact.module.codegen_fingerprint,
            "toolchain_fingerprint": artifact.toolchain_fingerprint.to_string(),
            "exit": output.status.code(),
        })
    );
    assert_eq!(
        composition.verification,
        ArtifactVerification::Complete,
        "{case}"
    );
    assert_eq!(
        composition.declared_components, composition.observed_components,
        "{case}"
    );
    assert!(
        composition.unclassified_runtime_symbols.is_empty(),
        "{case}"
    );
    assert!(
        composition
            .observed_components
            .iter()
            .any(|name| name == "env"),
        "{case}"
    );
    assert_eq!(metadata.target, request.target.triple, "{case}");
    assert_eq!(
        composition.object_format, request.target.object_format,
        "{case}"
    );
    assert_eq!(
        artifact.optimization_level, request.codegen_options.optimization_level,
        "{case}"
    );
    assert_eq!(metadata.generator_version, CODEGEN_VERSION, "{case}");
    if composition.object_format == ObjectFormat::Coff {
        let binding = coff.expect("MSVC 产物须绑定本次链接的 map");
        assert_eq!(binding.artifact_sha256, artifact_sha256, "{case}");
        assert_eq!(binding.map_sha256, digest(&binding.map_path), "{case}");
    }
}
