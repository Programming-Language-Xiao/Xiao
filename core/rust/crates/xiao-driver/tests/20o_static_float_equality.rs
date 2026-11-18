//! 静态Float/Sfloat按位相等/不等，以及生成器22的永久回归。
#[path = "support/float_equality.rs"]
mod corpus;

use xiao_codegen_llvm::{
    BuildRequest, CODEGEN_VERSION, CodegenOptions, EntryObservation, NativeBuild, NativeMetadata,
    NativeRuntimeSupport, TargetDescription, Toolchain,
};
use xiao_driver::{DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver};
use xiao_vm::VmEvent;

/// 同源VM的打印结果必须符合人工黄金，不能只比较两种错误实现是否一致。
fn verify_vm(case: &corpus::Case) {
    let result = FrontendVmDriver::new().run(&DriverRequest::new(FrontendRequest::from_text(
        case.source(true),
    )));
    assert!(result.is_success(), "{}: {:?}", case.name, result.report());
    let stdout: String = result
        .as_executed()
        .unwrap()
        .events()
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(stdout, format!("{}\n", case.expected), "{}", case.name);
}

#[test]
/// 真形参避免常量折叠；同时覆盖VM语义、静态位宽、混合提升和未变的排序。
fn real_parameters_preserve_language_equality_and_ordering() {
    let cases = corpus::cases();
    assert_eq!(cases.len(), 62);
    for case in cases {
        verify_vm(&case);
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(case.source(false)))
            .unwrap();
        let module = NativeBuild::new()
            .lower(&artifact.ir, &CodegenOptions::default())
            .unwrap();
        assert!(!module.text.contains("fcmp eq "), "{}", case.name);
        assert!(!module.text.contains("fcmp une "), "{}", case.name);
        if let Some(bits) = case.bits {
            let float = if bits == "i64" { "double" } else { "float" };
            assert!(
                module.text.contains(&format!("bitcast {float} ")),
                "{}",
                case.name
            );
            let predicate = if case.name.ends_with("-eq") {
                "eq"
            } else {
                "ne"
            };
            assert!(
                module.text.contains(&format!("icmp {predicate} {bits} ")),
                "{}",
                case.name
            );
        }
        if case.name.starts_with("original-pow") {
            assert!(module.text.contains("@xiao_runtime_cmath_pow"));
            assert!(!module.text.contains("llvm.pow.f64"));
        }
    }
}

#[test]
/// 新校验器仍接纳合法21元数据；模拟旧理解上限时22被明确拒绝，ABI与格式不变。
fn generator22_keeps_metadata_readability_separate_from_cache_identity() {
    assert_eq!(CODEGEN_VERSION, 22);
    assert_eq!(xiao_runtime_abi::ABI_MINOR_VERSION, 19);
    let target = TargetDescription::host();
    let ir = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text("value = 1\n"))
        .unwrap()
        .ir;
    let current = NativeMetadata::for_program(&ir, &target).unwrap();
    assert_eq!(current.generator_version, 22);
    assert_eq!(current.stdlib.stdlib_interface_version, 1);
    let mut older = current.clone();
    older.generator_version = 21;
    NativeMetadata::decode(&older.encode().unwrap())
        .unwrap()
        .validate(&target, &NativeRuntimeSupport::default(), 0)
        .unwrap();
    let old_support = NativeRuntimeSupport {
        generator_version: 21,
        ..NativeRuntimeSupport::default()
    };
    assert_eq!(
        current.validate(&target, &old_support, 0).unwrap_err().code,
        "X17-XAR-007"
    );
    older.validate(&target, &old_support, 0).unwrap();
    let toolchain = Toolchain::new("clang");
    assert_ne!(
        toolchain.fingerprint(&target, 21),
        toolchain.fingerprint(&target, 22)
    );
}

#[test]
#[ignore = "需要显式LLVM工具及本任务Runtime；独立取证必须精确执行此项"]
/// 真实llvm-as及原生O0/O3逐条核对静态/动态，不以字符串断言替代执行。
fn native_static_dynamic_comparisons_match_vm_o0_o3() {
    use serde_json::json;
    use std::path::PathBuf;
    use std::process::Command;

    let explicit = |name| PathBuf::from(std::env::var_os(name).expect(name));
    let llvm_as = explicit("XIAO_LLVM_AS");
    let runtime = explicit("XIAO_RUNTIME_LIBRARY");
    assert!(runtime.is_file() && llvm_as.is_file());
    let target = TargetDescription::host();
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain = Toolchain::new(explicit("XIAO_CLANG"))
        .with_llvm_as(&llvm_as)
        .with_llc(explicit("XIAO_LLC"))
        .with_runtime_library(runtime)
        .probe_versions()
        .unwrap()
        .probe_native_static_libraries(rustc, &target)
        .unwrap();
    let root = explicit("XIAO_FLOAT_EQUALITY_OUTPUT");
    assert!(!root.exists(), "必须使用全新原生观察目录");
    std::fs::create_dir_all(&root).unwrap();
    let mut rows = Vec::new();
    for case in corpus::cases() {
        verify_vm(&case);
        for printed in [false, true] {
            let source = case.source(printed);
            let artifact = FrontendCompiler::new()
                .compile(&FrontendRequest::from_text(&source))
                .unwrap();
            for level in [0, 3] {
                let mode = if printed { "dynamic" } else { "static" };
                let stem = format!("{}-{mode}-o{level}", case.name);
                let llvm = root.join(format!("{stem}.ll"));
                let bitcode = root.join(format!("{stem}.bc"));
                let executable = root.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
                let mut options = CodegenOptions::for_target(target.clone())
                    .with_entry_observation(if printed {
                        EntryObservation::Ignore
                    } else {
                        EntryObservation::ExitCode
                    });
                options.optimization_level = level;
                let module = NativeBuild::new().lower(&artifact.ir, &options).unwrap();
                std::fs::write(&llvm, &module.text).unwrap();
                let assembled = Command::new(&llvm_as)
                    .arg(&llvm)
                    .arg("-o")
                    .arg(&bitcode)
                    .output()
                    .unwrap();
                assert!(
                    assembled.status.success(),
                    "{stem}: {}",
                    String::from_utf8_lossy(&assembled.stderr)
                );
                let built = NativeBuild::new()
                    .build(
                        &BuildRequest::new(
                            artifact.ir.clone(),
                            target.clone(),
                            toolchain.clone(),
                            &executable,
                        )
                        .with_options(options),
                    )
                    .unwrap();
                assert_eq!(built.module.text, module.text);
                let output = Command::new(&built.executable).output().unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(if printed { 0 } else { i32::from(case.expected) }),
                    "{stem}"
                );
                assert_eq!(
                    String::from_utf8_lossy(&output.stdout),
                    if printed {
                        format!("{}\n", case.expected)
                    } else {
                        String::new()
                    },
                    "{stem}"
                );
                assert!(output.stderr.is_empty(), "{stem}: {:?}", output.stderr);
                rows.push(json!({
                    "case": case.name, "mode": mode, "optimization": level,
                    "expected": case.expected, "source": source,
                    "llvm": llvm, "bitcode": bitcode, "executable": executable,
                    "actual_uses_runtime": module.uses_runtime,
                    "codegen_fingerprint": module.codegen_fingerprint,
                    "toolchain_fingerprint": format!("{:?}", built.toolchain_fingerprint),
                    "assembly_exit_code": assembled.status.code(),
                    "assembly_stdout": String::from_utf8_lossy(&assembled.stdout),
                    "assembly_stderr": String::from_utf8_lossy(&assembled.stderr),
                    "exit_code": output.status.code(),
                    "stdout": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
                }));
            }
        }
    }
    assert_eq!(rows.len(), 248);
    let report = json!({"generator": CODEGEN_VERSION, "runtime_abi": xiao_runtime_abi::ABI_ENCODED_VERSION,
        "cases": 62, "native_observations": rows.len(), "passed": true, "rows": rows});
    std::fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("FLOAT-EQUALITY cases=62 native=248 llvm_as=248 vm=62 result=passed");
}
