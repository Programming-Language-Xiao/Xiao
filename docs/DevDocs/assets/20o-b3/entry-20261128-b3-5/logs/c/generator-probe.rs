//! 用同一份源码和工具链生成实际产物，记录生成器版本单独变化的指纹输入。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::json;
use xiao_artifacts::Digest256;
use xiao_codegen_llvm::{CODEGEN_VERSION, CodegenOptions, TargetDescription, Toolchain};
use xiao_driver::{FrontendNativeDriver, FrontendRequest, NativeBuildRequest};
use xiao_runtime_abi::ABI_ENCODED_VERSION;

/// 在工具指纹之外独立复算动态生成指纹，以便只替换版本参数。
fn fingerprint(options: &CodegenOptions, version: u32) -> String {
    let input = format!(
        "{version};dynamic;optimization={};abi={ABI_ENCODED_VERSION};capabilities={:016x};locale={};{}",
        options.optimization_level,
        options.capabilities,
        options.locale,
        options.target.fingerprint_fields()
    );
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    format!("xiao-codegen-{version}-{hash:016x}")
}

/// 用文件原件计算 SHA-256。
fn sha(path: &Path) -> String {
    Digest256::of_bytes(&fs::read(path).unwrap()).as_hex()
}

/// 将 stdout、stderr、链接及生成身份绑定到一次实际构建。
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [source, root, optimization, mask] = args.as_slice() else {
        panic!("usage: generator_probe SOURCE ROOT OPTIMIZATION CAPABILITIES");
    };
    let root = Path::new(root);
    fs::create_dir_all(root).unwrap();
    let target = TargetDescription::host();
    let path = |name| PathBuf::from(std::env::var_os(name).expect(name));
    let tools = Toolchain::new(path("XIAO_CLANG"))
        .with_llvm_as(path("XIAO_LLVM_AS"))
        .with_llc(path("XIAO_LLC"))
        .with_runtime_library(path("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries(path("XIAO_RUSTC"), &target)
        .unwrap()
        .probe_versions()
        .unwrap();
    let optimization = optimization.to_str().unwrap().parse::<u8>().unwrap();
    let mask = mask.to_str().unwrap().parse::<u64>().unwrap();
    let frontend = FrontendRequest::from_text(fs::read_to_string(source).unwrap());
    let options = CodegenOptions::for_target(target.clone())
        .with_locale(frontend.context.locale.clone())
        .with_optimization_level(optimization)
        .unwrap()
        .with_capabilities(mask)
        .unwrap();
    let exe = root.join(format!("program{}", std::env::consts::EXE_SUFFIX));
    let ll = root.join("program.ll");
    let request = NativeBuildRequest::new(frontend, target.clone(), tools.clone(), &exe)
        .with_codegen_options(options.clone())
        .with_llvm_ir_output(&ll);
    let result = match FrontendNativeDriver::new().build(&request) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let artifact = &result.native;
    let codegen = &artifact.module.codegen_fingerprint;
    assert_eq!(*codegen, fingerprint(&options, CODEGEN_VERSION));
    let plan = &artifact.native_optimization_report.plan.fingerprint;
    let tool_fingerprint = |version| {
        tools
            .fingerprint_with_optimization(&target, version, optimization, plan)
            .to_string()
    };
    assert_eq!(artifact.toolchain_fingerprint.to_string(), tool_fingerprint(CODEGEN_VERSION));
    let output = Command::new(&exe).output().unwrap();
    fs::write(root.join("stdout"), &output.stdout).unwrap();
    fs::write(root.join("stderr"), &output.stderr).unwrap();
    let evidence = &artifact.artifact_runtime;
    let binary_sha256 = sha(&exe);
    let map = evidence.coff_link_identity.as_ref().map(|map| {
        assert_eq!(binary_sha256, map.artifact_sha256);
        assert_eq!(sha(&map.map_path), map.map_sha256);
        json!({"path": map.map_path, "sha256": map.map_sha256})
    });
    let metadata = artifact.native_metadata.as_ref().unwrap();
    let evidence = json!({
        "generator": CODEGEN_VERSION,
        "optimization": optimization,
        "capabilities": mask,
        "source": {"path": Path::new(source), "sha256": sha(Path::new(source))},
        "runtime": {
            "path": tools.runtime_library,
            "sha256": sha(&path("XIAO_RUNTIME_LIBRARY")),
            "abi": metadata.runtime_abi_min,
        },
        "target": metadata.target,
        "stdlib_interface": metadata.stdlib.stdlib_interface_version,
        "stdlib": format!("{:?}", metadata.stdlib),
        "binary": {"path": exe, "sha256": binary_sha256},
        "ir": {"path": ll, "sha256": sha(&ll)},
        "map": map,
        "exit": output.status.code(),
        "verification": evidence.verification.as_str(),
        "declared_components": evidence.declared_components,
        "observed_components": evidence.observed_components,
        "unclassified_runtime_symbols": evidence.unclassified_runtime_symbols,
        "runtime_symbols": evidence.runtime_symbols,
        "fingerprints": {"codegen": codegen, "toolchain": tool_fingerprint(CODEGEN_VERSION)},
        "version_parameter_28_only": {
            "codegen": fingerprint(&options, 28),
            "toolchain": tool_fingerprint(28),
        },
    });
    fs::write(root.join("verification.json"), serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    println!("{}", serde_json::to_string(&evidence).unwrap());
    ExitCode::from(u8::try_from(output.status.code().unwrap()).unwrap())
}
