//! 一次新建目录内编译O0/O3各一份完整测量程序，之后仅向新进程发送配置。

use std::path::{Path, PathBuf};
use std::process::Command;
use xiao_codegen_llvm::{CodegenOptions, NativeBuild, TargetDescription, Toolchain};
use xiao_driver::FrontendArtifact;

/// 使用本轮明确指定的release Runtime及宿主工具链。
pub fn build(
    repository: &Path,
    directory: &Path,
    artifact: &FrontendArtifact,
    level: u8,
) -> PathBuf {
    let required =
        |key| PathBuf::from(std::env::var_os(key).unwrap_or_else(|| panic!("缺少{key}")));
    let target = TargetDescription::host();
    assert_eq!(std::env::var("XIAO_TARGET_TRIPLE").unwrap(), target.triple);
    let toolchain = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .unwrap();
    let options = CodegenOptions::for_target(target.clone())
        .with_optimization_level(level)
        .unwrap();
    let module = NativeBuild::new().lower(&artifact.ir, &options).unwrap();
    let text = module.text.replace(
        "@xiao_runtime_print_values_with_capabilities",
        "@xiao_a6_measure_print",
    );
    assert_ne!(text, module.text);
    toolchain.validate_llvm_ir(&text, &target).unwrap();
    let llvm = directory.join(format!("numeric-o{level}.ll"));
    let object = directory.join(format!("clock-o{level}.o"));
    let executable = directory.join(format!("numeric-o{level}{}", std::env::consts::EXE_SUFFIX));
    assert!(!executable.exists());
    std::fs::write(&llvm, text).unwrap();
    let output = Command::new(&toolchain.clang)
        .args(["-target", target.triple.as_str(), "-std=c11", "-c"])
        .arg(repository.join("tests/fixtures/20o-b2-a6/clock-shim.c"))
        .arg("-o")
        .arg(&object)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut linker = Command::new(&toolchain.clang);
    linker
        .args(["-target", target.triple.as_str(), &format!("-O{level}")])
        .arg(&llvm)
        .arg(&object)
        .arg(toolchain.runtime_library.as_ref().unwrap())
        .args(&toolchain.native_static_libraries);
    if cfg!(windows) {
        linker.arg("-lpsapi");
    }
    let output = linker.arg("-o").arg(&executable).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}
