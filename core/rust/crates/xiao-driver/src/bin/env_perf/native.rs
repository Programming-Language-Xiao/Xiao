//! 仅在示例产物上接入可重复调用的原生嵌入入口和采样输出。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use xiao_codegen_llvm::{CodegenOptions, NativeBuild, TargetDescription, Toolchain};
use xiao_driver::FrontendArtifact;

use super::{CAPABILITIES, file_info, write_json};

pub fn required(name: &str) -> PathBuf {
    std::env::var_os(name)
        .unwrap_or_else(|| panic!("缺少 {name}"))
        .into()
}

pub fn toolchain(target: &TargetDescription) -> Toolchain {
    assert_eq!(
        std::env::var("XIAO_TARGET_TRIPLE").expect("缺少 XIAO_TARGET_TRIPLE"),
        target.triple
    );
    Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries("rustc", target)
        .unwrap()
}

pub fn build_shim(
    directory: &Path,
    logs: &Path,
    target: &TargetDescription,
    tools: &Toolchain,
) -> PathBuf {
    let source = directory.join("native.c");
    fs::write(&source, include_bytes!("native.c")).unwrap();
    let object = directory.join("env-shim.o");
    let mut command = Command::new(&tools.clang);
    command
        .args(["-target", &target.triple, "-std=c11", "-O2", "-c"])
        .arg(format!("-DXIAO_B3_MASK={CAPABILITIES}"))
        .arg(&source)
        .arg("-o")
        .arg(&object);
    run(command, "shim-compile", directory, logs);
    object
}

pub fn build(
    directory: &Path,
    logs: &Path,
    target: &TargetDescription,
    tools: &Toolchain,
    shim: &Path,
    artifact: &FrontendArtifact,
    level: u8,
) -> PathBuf {
    let options = CodegenOptions::for_target(target.clone())
        .with_optimization_level(level)
        .unwrap()
        .with_capabilities(CAPABILITIES)
        .unwrap();
    let module = NativeBuild::new().lower(&artifact.ir, &options).unwrap();
    let mut text = module.text;
    for (from, to) in [
        (
            "define internal i32 @xiao_application_main()",
            "define i32 @xiao_application_main()",
        ),
        (
            "define i32 @main()",
            "define internal i32 @xiao_b3_unused_standalone_main()",
        ),
        (
            "@xiao_runtime_print_values_with_capabilities",
            "@xiao_b3_measure_print",
        ),
    ] {
        assert!(text.contains(from), "样例 LLVM 接线缺少 {from}");
        text = text.replace(from, to);
    }
    let llvm = directory.join(format!("measurement-o{level}.ll"));
    fs::write(&llvm, text).unwrap();
    let executable = directory.join(format!(
        "measurement-o{level}{}",
        std::env::consts::EXE_SUFFIX
    ));
    let mut command = Command::new(&tools.clang);
    command
        .args(["-target", &target.triple, &format!("-O{level}")])
        .arg(&llvm)
        .arg(shim)
        .arg(tools.runtime_library.as_ref().unwrap())
        .args(&tools.native_static_libraries);
    if cfg!(windows) {
        command.arg("-lpsapi");
    }
    command.arg("-o").arg(&executable);
    run(command, &format!("link-o{level}"), directory, logs);
    write_json(
        &logs.join(format!("artifact-o{level}.json")),
        &serde_json::json!({
            "codegen_fingerprint": module.codegen_fingerprint,
            "llvm": file_info(&llvm),
            "shim": file_info(shim),
            "executable": file_info(&executable),
        }),
    );
    executable
}

fn run(mut command: Command, label: &str, cwd: &Path, directory: &Path) {
    let output = command.current_dir(cwd).output().unwrap();
    let stdout = directory.join(format!("{label}.stdout.log"));
    let stderr = directory.join(format!("{label}.stderr.log"));
    fs::write(&stdout, &output.stdout).unwrap();
    fs::write(&stderr, &output.stderr).unwrap();
    write_json(
        &directory.join(format!("{label}.json")),
        &serde_json::json!({
            "program": command.get_program(),
            "args": command.get_args().collect::<Vec<_>>(),
            "cwd": cwd,
            "exit": output.status.code(),
            "stdout": file_info(&stdout),
            "stderr": file_info(&stderr),
        }),
    );
    assert!(output.status.success(), "{label}: {}", stderr.display());
}
