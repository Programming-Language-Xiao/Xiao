//! Baseline-only probe; it does not apply a compiler or version change.
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use xiao_codegen_llvm::{
    BuildRequest, CODEGEN_VERSION, CodegenOptions, EntryObservation, NativeBuild,
    TargetDescription, Toolchain,
};
use xiao_driver::{DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver};
use xiao_runtime::RuntimeValue;
use xiao_runtime_abi::ABI_ENCODED_VERSION;
use xiao_syntax::BinaryOperator;
use xiao_vm::VmEvent;

fn output_record(output: Output) -> Value {
    json!({
        "exit_code": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout).into_owned(),
        "stderr": String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn vm_record(source: &str) -> Value {
    let request = DriverRequest::new(FrontendRequest::from_text(source));
    let result = FrontendVmDriver::new().run(&request);
    let mut stdout = String::new();
    if let Some(execution) = result.as_executed() {
        for event in execution.events() {
            if let VmEvent::IntrinsicOutput { text } = event {
                stdout.push_str(text);
            }
        }
    }
    json!({
        "success": result.is_success(), "code": result.code(), "stdout": stdout,
        "report_debug": format!("{:?}", result.report()),
    })
}

fn native_record(
    source: &str, label: &str, mode: &str, root: &Path,
    assembler: &Path, target: &TargetDescription, toolchain: &Toolchain,
) -> Value {
    let artifact = match FrontendCompiler::new().compile(&FrontendRequest::from_text(source)) {
        Ok(artifact) => artifact,
        Err(error) => return json!({"phase": "frontend", "error": format!("{error:?}")}),
    };
    let mut observations = Vec::new();
    for level in [0, 3] {
        let mut options = CodegenOptions::for_target(target.clone()).with_entry_observation(
            if mode == "static" { EntryObservation::ExitCode } else { EntryObservation::Ignore },
        );
        options.optimization_level = level;
        let module = match NativeBuild::new().lower(&artifact.ir, &options) {
            Ok(module) => module,
            Err(error) => {
                observations.push(json!({
                    "optimization": level, "phase": "lowering", "error": format!("{error:?}"),
                }));
                continue;
            }
        };
        let stem = format!("{label}-{mode}-o{level}");
        let llvm_path = root.join(format!("{stem}.ll"));
        let bitcode_path = root.join(format!("{stem}.bc"));
        let executable = root.join(format!("{stem}.exe"));
        fs::write(&llvm_path, module.text.as_bytes()).expect("write LLVM observation");
        let assembly = Command::new(assembler)
            .arg(&llvm_path).arg("-o").arg(&bitcode_path)
            .output().expect("launch actual llvm-as");
        let assembly_accepted = assembly.status.success();
        let mut row = json!({
            "optimization": level, "actual_uses_runtime": module.uses_runtime,
            "llvm_path": llvm_path, "bitcode_path": bitcode_path,
            "contains_invalid_fcmp_eq": module.text.contains("fcmp eq "),
            "contains_numeric_fcmp_une": module.text.contains("fcmp une "),
            "assembly": output_record(assembly), "native_executed": false,
            "observation": if mode == "static" {
                "test-only entry boolean observation: exit 0 means false, exit 1 means true"
            } else {
                "normal exit status and printed boolean"
            },
        });
        if assembly_accepted {
            let request = BuildRequest::new(
                artifact.ir.clone(), target.clone(), toolchain.clone(), &executable,
            ).with_options(options);
            match NativeBuild::new().build(&request) {
                Ok(built) => {
                    let execution = Command::new(&built.executable)
                        .output().expect("run baseline native observation");
                    row["native_executed"] = json!(true);
                    row["executable"] = json!(built.executable);
                    row["build_text_matches_recorded"] = json!(built.module.text == module.text);
                    row["execution"] = output_record(execution);
                }
                Err(error) => {
                    row["build_error"] = json!(format!("{error:?}"));
                }
            }
        }
        observations.push(row);
    }
    json!({
        "source": source, "runtime_checks_debug": format!("{:?}", artifact.ir.runtime_checks),
        "observations": observations,
    })
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(args.next().expect("fresh output directory"));
    let runtime = PathBuf::from(args.next().expect("fixed Runtime static library"));
    let clang = PathBuf::from(args.next().expect("clang path"));
    let assembler = PathBuf::from(args.next().expect("llvm-as path"));
    let llc = PathBuf::from(args.next().expect("llc path"));
    let rustc = PathBuf::from(args.next().expect("Rust 1.96.0 compiler path"));
    assert!(args.next().is_none(), "exactly six arguments are required");
    assert_eq!(CODEGEN_VERSION, 21, "this is an unchanged baseline probe");
    assert_eq!(ABI_ENCODED_VERSION, 65555, "baseline ABI must remain 1.19");
    assert!(!root.exists(), "never reuse an observation directory");
    fs::create_dir(&root).expect("create fresh observation directory");
    let target = TargetDescription::host();
    let toolchain = Toolchain::new(clang)
        .with_llvm_as(&assembler).with_llc(llc).with_runtime_library(runtime)
        .probe_versions().expect("observe actual tool versions")
        .probe_native_static_libraries(rustc.as_os_str(), &target)
        .expect("observe actual Runtime native library requirements");
    let cases = [
        ("float-zero-eq", "float", "float", "==", "0.0", "-0.0"),
        ("float-zero-ne", "float", "float", "!=", "0.0", "-0.0"),
        ("float-negative-zero-eq", "float", "float", "==", "-0.0", "-0.0"),
        ("float-finite-eq", "float", "float", "==", "1.5", "1.5"),
        ("float-different-eq", "float", "float", "==", "1.5", "2.0"),
        ("sfloat-zero-eq", "sfloat", "sfloat", "==", "sfloat(0.0)", "sfloat(-0.0)"),
        ("sfloat-zero-ne", "sfloat", "sfloat", "!=", "sfloat(0.0)", "sfloat(-0.0)"),
        ("sfloat-negative-zero-eq", "sfloat", "sfloat", "==", "sfloat(-0.0)", "sfloat(-0.0)"),
        ("sfloat-finite-eq", "sfloat", "sfloat", "==", "sfloat(1.5)", "sfloat(1.5)"),
        ("sfloat-different-eq", "sfloat", "sfloat", "==", "sfloat(1.5)", "sfloat(2.0)"),
        ("mixed-integer-float", "int", "float", "==", "1", "1.0"),
        ("mixed-rounded-integer", "int", "float", "==", "9007199254740993", "9007199254740992.0"),
        ("mixed-sint-sfloat", "sint", "sfloat", "==", "sint(1)", "sfloat(1.0)"),
        ("mixed-sfloat-float", "sfloat", "float", "==", "sfloat(1.5)", "1.5"),
        ("integer-control", "int", "int", "==", "1", "1"),
        ("boolean-control", "bool", "bool", "==", "true", "false"),
    ];
    let mut rows = Vec::new();
    for (label, left_type, right_type, operator, left, right) in cases {
        let prefix = format!("def compare({left_type} left, {right_type} right) -> bool
    return left {operator} right
");
        let static_source = format!("{prefix}compare({left}, {right})
");
        let printed_source = format!("{prefix}print(compare({left}, {right}))
");
        rows.push(json!({
            "case": label, "operator": operator,
            "vm": vm_record(&printed_source),
            "static": native_record(&static_source, label, "static", &root,
                                     &assembler, &target, &toolchain),
            "dynamic": native_record(&printed_source, label, "dynamic", &root,
                                      &assembler, &target, &toolchain),
        }));
    }
    let float_zero = RuntimeValue::Float(0.0);
    let float_negative_zero = RuntimeValue::Float(-0.0);
    let sfloat_zero = RuntimeValue::Sfloat(0.0);
    let sfloat_negative_zero = RuntimeValue::Sfloat(-0.0);
    println!("{}", json!({
        "scope": "unchanged generator-21 baseline observations; no repair acceptance",
        "runtime_abi": ABI_ENCODED_VERSION, "codegen_version": CODEGEN_VERSION,
        "reference": {
            "rust_numeric_float_zero_equality": 0.0_f64 == -0.0_f64,
            "runtime_language_float_zero_eq": float_zero
                .language_binary(BinaryOperator::Equal, &float_negative_zero).unwrap().as_bool(),
            "runtime_language_float_zero_ne": float_zero
                .language_binary(BinaryOperator::NotEqual, &float_negative_zero).unwrap().as_bool(),
            "runtime_language_sfloat_zero_eq": sfloat_zero
                .language_binary(BinaryOperator::Equal, &sfloat_negative_zero).unwrap().as_bool(),
            "runtime_language_sfloat_zero_ne": sfloat_zero
                .language_binary(BinaryOperator::NotEqual, &sfloat_negative_zero).unwrap().as_bool(),
        },
        "cases": rows,
    }));
}
