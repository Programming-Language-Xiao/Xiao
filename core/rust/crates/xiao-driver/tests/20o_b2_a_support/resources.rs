//! 语言级值错误、源码位置与结果分配失败；不测A6时间或峰值预算。

use super::{native_output, repository};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use xiao_codegen_llvm::{CodegenOptions, NativeBuild, TargetDescription, Toolchain};
use xiao_driver::{DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver};
use xiao_vm::VmEvent;

/// 独立列出错误与应归因的源码表达式，不从Runtime实现生成期望值。
fn errors() -> [(&'static str, &'static str, &'static str, &'static str); 6] {
    [
        (
            "narrow",
            "print(int(lfloat(\"9223372036854775808\")))\n",
            "int(lfloat(\"9223372036854775808\"))",
            "X06-RUNTIME-009",
        ),
        (
            "syntax",
            "print(lfloat(\"1__2\"))\n",
            "lfloat(\"1__2\")",
            "X06-RUNTIME-012",
        ),
        (
            "zero",
            "lint value = 1\nprint(value / 0)\n",
            "value / 0",
            "X06-RUNTIME-013",
        ),
        (
            "admission",
            "print(lfloat(\"1e100001\"))\n",
            "lfloat(\"1e100001\")",
            "X06-RUNTIME-033",
        ),
        (
            "quotient",
            "lfloat a = 1e100000\nlfloat b = 1e-100000\nprint(a // b)\n",
            "a // b",
            "X06-RUNTIME-033",
        ),
        (
            "nonterminating",
            "lint value = 1\nvalue / 3\n",
            "value / 3",
            "X06-RUNTIME-034",
        ),
    ]
}

/// 受控结果分配失败在两后端使用同一完整源码，不伪造真实OOM。
const ALLOCATION_SOURCE: &str = "lfloat value = 1.50\ntry\n    str(value)\ncatch failure as ResourceError\n    print(error_code(failure))\nprint(value)\n";

/// VM直接核对结构化错误码及精确字节区间；值错误不能提前变成编译拒绝。
pub(super) fn vm_resources() {
    for (name, source, expression, code) in errors() {
        let request = FrontendRequest::from_text(source);
        let artifact = FrontendCompiler::new().compile(&request).unwrap();
        let outcome = FrontendVmDriver::new().run_artifact(
            &artifact,
            &DriverRequest::new(request).with_timeout(Duration::from_secs(10)),
        );
        let report = outcome
            .report()
            .unwrap_or_else(|| panic!("{name}: {outcome:?}"));
        assert_eq!(report.code, code, "{name}");
        let span = report.location.expect("数值错误必须有源码位置");
        let start = source.find(expression).unwrap();
        assert_eq!(
            (span.start(), span.end()),
            (start, start + expression.len()),
            "{name}"
        );
        println!(
            "B2-A5-ERROR case={name} backend=vm code={code} span={start}..{} result=success",
            start + expression.len()
        );
    }
    // 没有字符串字面量的源码：先构造lfloat，再在str结果构造处定向失败。
    let source = ALLOCATION_SOURCE;
    let request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new().compile(&request).unwrap();
    let fail = xiao_runtime::allocation_testing::StringAllocationFailure::after(0);
    let outcome = FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(request));
    assert!(fail.was_triggered());
    assert!(outcome.is_success(), "{outcome:?}");
    let stdout: String = outcome
        .as_executed()
        .unwrap()
        .events()
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(stdout, "X06-RUNTIME-011\n1.50\n");
    println!(
        "B2-A5-ERROR case=allocation backend=vm code=X06-RUNTIME-011 source_unchanged=true result=success"
    );
}

/// 仅测试LLVM替换报告出口，读取现有snapshot而不take/clear挂起错误或打开GUI。
pub(super) fn native_resources() {
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
    let root = std::env::temp_dir().join(format!(
        "xiao-b2-a5-errors-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let object = root.join("observer.o");
    let compiled = Command::new(&toolchain.clang)
        .args(["-target", target.triple.as_str(), "-std=c11", "-c"])
        .arg(repository().join("tests/fixtures/docs/numeric/a5-observer.c"))
        .arg("-o")
        .arg(&object)
        .output()
        .unwrap();
    assert!(compiled.status.success(), "{compiled:?}");
    for (name, source, expression, code) in errors() {
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .unwrap();
        for level in [0, 3] {
            let options = CodegenOptions::for_target(target.clone())
                .with_optimization_level(level)
                .unwrap();
            let module = NativeBuild::new().lower(&artifact.ir, &options).unwrap();
            let text = module
                .text
                .replace("@xiao_runtime_error_report(", "@xiao_a5_test_report(");
            toolchain.validate_llvm_ir(&text, &target).unwrap();
            let llvm = root.join(format!("{name}-o{level}.ll"));
            fs::write(&llvm, text).unwrap();
            let executable = root.join(format!("{name}-o{level}{}", std::env::consts::EXE_SUFFIX));
            let linked = Command::new(&toolchain.clang)
                .args(["-target", target.triple.as_str(), &format!("-O{level}")])
                .arg(&llvm)
                .arg(&object)
                .arg(toolchain.runtime_library.as_ref().unwrap())
                .args(&toolchain.native_static_libraries)
                .arg("-o")
                .arg(&executable)
                .output()
                .unwrap();
            assert!(linked.status.success(), "{name}: {linked:?}");
            let output = native_output(&executable);
            assert!(!output.status.success(), "{name}必须运行时失败");
            assert!(output.stdout.is_empty());
            let start = source.find(expression).unwrap();
            let expected = format!(
                "A5-ERROR code={code} span={start}..{}\n",
                start + expression.len()
            );
            assert_eq!(
                String::from_utf8(output.stderr)
                    .unwrap()
                    .replace("\r\n", "\n"),
                expected,
                "{name} O{level}"
            );
            println!(
                "B2-A5-ERROR case={name} backend=native optimization={level} code={code} artifact={} result=success",
                executable.display()
            );
        }
    }
    native_lifetimes(&target, &toolchain, &root);
}

/// 成功/转换失败/右侧失败时，标记的真实字符串临时值在下一表达式前恰好释放一次。
fn native_lifetimes(target: &TargetDescription, toolchain: &Toolchain, root: &std::path::Path) {
    let cases = [
        (
            "cast-success",
            "str(lfloat(1))\nstr(lfloat(2))\n",
            "",
            false,
        ),
        (
            "cast-failure",
            "lfloat value = 1.50\ntry\n    lint(str(value))\ncatch failure as TypeError\n    int caught = 1\nstr(lfloat(2))\nprint(value)\n",
            "1.50\n",
            false,
        ),
        (
            "right-failure",
            "try\n    str(lfloat(1)) + str(lfloat(\"bad\"))\ncatch failure as TypeError\n    int caught = 1\nstr(lfloat(2))\n",
            "",
            false,
        ),
        (
            "allocation",
            ALLOCATION_SOURCE,
            "X06-RUNTIME-011\n1.50\n",
            true,
        ),
    ];
    for (name, source, expected, allocation) in cases {
        let object = root.join(format!("{name}-shim.o"));
        let mut compiler = Command::new(&toolchain.clang);
        compiler
            .args(["-target", target.triple.as_str(), "-std=c11", "-c"])
            .arg(repository().join("tests/fixtures/docs/numeric/a5-lifetime-shim.c"));
        if allocation {
            compiler.arg("-DA5_ALLOCATION_FAILURE=1");
        }
        let compiled = compiler.arg("-o").arg(&object).output().unwrap();
        assert!(compiled.status.success(), "{name}: {compiled:?}");
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .unwrap();
        for level in [0, 3] {
            let options = CodegenOptions::for_target(target.clone())
                .with_optimization_level(level)
                .unwrap();
            let module = NativeBuild::new().lower(&artifact.ir, &options).unwrap();
            let mut text = module.text;
            for (original, replacement) in [
                ("@xiao_runtime_value_cast(", "@xiao_a5_test_cast("),
                (
                    "@xiao_runtime_value_release_strong(",
                    "@xiao_a5_test_release_strong(",
                ),
                (
                    "@xiao_runtime_value_release_any(",
                    "@xiao_a5_test_release_any(",
                ),
            ] {
                text = text.replace(original, replacement);
            }
            toolchain.validate_llvm_ir(&text, target).unwrap();
            let llvm = root.join(format!("lifetime-{name}-o{level}.ll"));
            fs::write(&llvm, text).unwrap();
            let executable = root.join(format!(
                "lifetime-{name}-o{level}{}",
                std::env::consts::EXE_SUFFIX
            ));
            let linked = Command::new(&toolchain.clang)
                .args(["-target", target.triple.as_str(), &format!("-O{level}")])
                .arg(&llvm)
                .arg(&object)
                .arg(toolchain.runtime_library.as_ref().unwrap())
                .args(&toolchain.native_static_libraries)
                .arg("-o")
                .arg(&executable)
                .output()
                .unwrap();
            assert!(linked.status.success(), "{name}: {linked:?}");
            let output = native_output(&executable);
            assert_eq!(output.status.code(), Some(0), "{name} O{level}: {output:?}");
            assert!(output.stderr.is_empty(), "{name} O{level}: {output:?}");
            assert_eq!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .replace("\r\n", "\n"),
                expected
            );
            if allocation {
                println!(
                    "B2-A5-ALLOCATION optimization={level} code=X06-RUNTIME-011 out=unchanged source_unchanged=true artifact={} result=success",
                    executable.display()
                );
            } else {
                println!(
                    "B2-A5-CLEANUP case={name} optimization={level} temporary=once artifact={} result=success",
                    executable.display()
                );
            }
        }
    }
}
