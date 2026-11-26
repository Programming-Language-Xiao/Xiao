//! 独立任务目录内运行 fs 原生教程与权限边界。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{CAPABILITY_FILESYSTEM, CAPABILITY_STDOUT, GOLDEN, SOURCE};
use xiao_codegen_llvm::{
    ArtifactVerification, CODEGEN_VERSION, CodegenOptions, NativeRuntimeSupport, ObjectFormat,
    TargetDescription, Toolchain, read_native_metadata,
};
use xiao_driver::{FrontendNativeDriver, FrontendRequest, NativeBuildRequest};

const DENIED: &str = r#"from std.fs import write_text, list_dir, remove, rename, metadata
try
    write_text("demo/prohibited", "x", false)
catch failure as Error
    print(error_code(failure))
try
    list_dir("")
catch failure as Error
    print(error_code(failure))
try
    remove("demo")
catch failure as Error
    print(error_code(failure))
try
    rename("demo", "different", false)
catch failure as Error
    print(error_code(failure))
try
    metadata("demo")
catch failure as Error
    print(error_code(failure))
"#;

fn digest(path: &Path) -> String {
    xiao_artifacts::Digest256::of_bytes(&fs::read(path).unwrap()).as_hex()
}

#[test]
#[ignore = "需要固定目标、发布 Runtime、LLVM 和独立任务目录"]
fn fs_tutorial_matches_native_golden() {
    let required = |name: &str| PathBuf::from(std::env::var_os(name).expect(name));
    let target = TargetDescription::host();
    assert_eq!(std::env::var("XIAO_TARGET_TRIPLE").unwrap(), target.triple);
    let toolchain = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries("rustc", &target)
        .unwrap()
        .probe_versions()
        .unwrap();
    assert_eq!(CODEGEN_VERSION, 25);
    let previous = required("XIAO_B3_2_NATIVE_ARTIFACT");
    let metadata = read_native_metadata(&previous, &target).unwrap().unwrap();
    assert_eq!(metadata.generator_version, 24);
    metadata.validate(&target, &Default::default(), 0).unwrap();
    let old_fingerprint = toolchain.fingerprint(&target, 24);
    let new_fingerprint = toolchain.fingerprint(&target, 25);
    assert_ne!(old_fingerprint, new_fingerprint);
    println!(
        "B3-3-VERSION {}",
        serde_json::json!({
            "baseline": 24, "current": 25, "older_metadata": "accepted",
            "baseline_fingerprint": old_fingerprint.to_string(),
            "current_fingerprint": new_fingerprint.to_string()
        })
    );

    let root = std::env::temp_dir().join(format!("xiao-b3-3-native-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let build_and_run = |case: &str, source: &str, level, capabilities, golden: &[u8]| {
        let task = root.join(case);
        let demo = task.join("demo");
        fs::create_dir_all(&demo).unwrap();
        let executable = task.join(format!("program{}", std::env::consts::EXE_SUFFIX));
        let options = CodegenOptions::for_target(target.clone())
            .with_optimization_level(level)
            .unwrap()
            .with_capabilities(capabilities)
            .unwrap();
        let result = FrontendNativeDriver::new()
            .build(
                &NativeBuildRequest::new(
                    FrontendRequest::from_text(source),
                    target.clone(),
                    toolchain.clone(),
                    &executable,
                )
                .with_codegen_options(options),
            )
            .unwrap_or_else(|error| panic!("{case}：{error:?}"));
        let composition = &result.native.artifact_runtime;
        assert_eq!(composition.verification, ArtifactVerification::Complete);
        assert_eq!(
            composition.declared_components,
            composition.observed_components
        );
        assert!(composition.unclassified_runtime_symbols.is_empty());
        assert!(
            composition
                .observed_components
                .iter()
                .any(|name| name == "fs")
        );
        let metadata = result.native.native_metadata.unwrap();
        assert_eq!(metadata.generator_version, CODEGEN_VERSION);
        assert_eq!(
            metadata.runtime_abi_min,
            xiao_runtime_abi::ABI_ENCODED_VERSION
        );
        assert_eq!(metadata.runtime_abi_max, metadata.runtime_abi_min);
        assert_eq!(metadata.stdlib.stdlib_interface_version, 1);
        let older = NativeRuntimeSupport {
            generator_version: 24,
            ..Default::default()
        };
        assert_eq!(
            metadata.validate(&target, &older, 0).unwrap_err().code,
            "X17-XAR-007"
        );
        let artifact_sha256 = digest(&executable);
        let coff = composition.coff_link_identity.as_ref();
        if target.object_format == ObjectFormat::Coff {
            let binding = coff.expect("MSVC 产物的绑定链接 map");
            assert_eq!(binding.artifact_sha256, artifact_sha256);
            assert_eq!(binding.map_sha256, digest(&binding.map_path));
        }
        let output = Command::new(&executable)
            .current_dir(&task)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{case}: {output:?}");
        assert!(output.stderr.is_empty(), "{case}: {output:?}");
        assert_eq!(output.stdout, golden, "{case}");
        assert_eq!(fs::read_dir(&demo).unwrap().count(), 0, "{case}");
        assert!(!task.join("different").exists());
        fs::write(task.join("stdout.bin"), &output.stdout).unwrap();
        fs::write(task.join("stderr.bin"), &output.stderr).unwrap();
        println!(
            "B3-3-NATIVE {}",
            serde_json::json!({
                "case": case, "target": target.triple, "optimization": level, "capabilities": capabilities,
                "path": executable, "sha256": artifact_sha256, "exit": output.status.code(),
                "demo_empty": true, "verification": "complete", "components": composition.declared_components,
                "observed": composition.observed_components, "unclassified": composition.unclassified_runtime_symbols,
                "coff_map": coff.map(|binding| &binding.map_path), "coff_map_sha256": coff.map(|binding| &binding.map_sha256),
                "generator": metadata.generator_version, "old_verifier": "X17-XAR-007"
            })
        );
    };
    for level in [0, 3] {
        for alias in ["std", "stdlib"] {
            let source = SOURCE.replacen("from std.fs ", &format!("from {alias}.fs "), 1);
            build_and_run(
                &format!("fs-{alias}-o{level}"),
                &source,
                level,
                CAPABILITY_FILESYSTEM | CAPABILITY_STDOUT,
                GOLDEN,
            );
        }
        build_and_run(
            &format!("fs-denied-o{level}"),
            DENIED,
            level,
            CAPABILITY_STDOUT,
            b"X06-RUNTIME-026\n".repeat(5).as_slice(),
        );
    }
}
