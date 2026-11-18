//! 独立测量可执行文件：只替换已有 print 调用为测试时钟 shim，不改生产 Runtime 或 ABI。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use xiao_codegen_llvm::{CodegenOptions, NativeBuild, TargetDescription, Toolchain};
use xiao_driver::{FrontendCompiler, FrontendRequest};

use super::{Sample, cases};

/// 显式工具链与已有精确目标一致，不能以缺少配置返回通过。
fn toolchain() -> (TargetDescription, Toolchain) {
    let target = TargetDescription::host();
    assert_eq!(
        std::env::var("XIAO_TARGET_TRIPLE").expect("需要目标三元组"),
        target.triple
    );
    let required =
        |name: &str| PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("需要 {name}")));
    let toolchain = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .unwrap();
    (target, toolchain)
}

/// 窗口内没有终端 I/O；golden 比较在测量结束后，失败立即拒绝。
pub(super) fn samples(
    root: &Path,
    directory: &Path,
    case: &cases::Workload,
    repeats: usize,
    count: usize,
    level: u8,
) -> Vec<Sample> {
    let (target, toolchain) = toolchain();
    let stem = format!(
        "{}-o{level}-{}-{}",
        case.id, case.input_bytes_or_items, case.output_bytes_or_items
    );
    let llvm_path = directory.join(format!("{stem}.ll"));
    let object = directory.join("clock-shim.o");
    let executable = directory.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    if !executable.is_file() {
        let source = cases::source(root, case, repeats, count, true);
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .unwrap();
        let options = CodegenOptions::for_target(target.clone())
            .with_optimization_level(level)
            .unwrap();
        let module = NativeBuild::new().lower(&artifact.ir, &options).unwrap();
        let text = module.text.replace(
            "@xiao_runtime_print_values_with_capabilities",
            "@xiao_b1_measure_print",
        );
        assert_ne!(text, module.text);
        toolchain.validate_llvm_ir(&text, &target).unwrap();
        std::fs::write(&llvm_path, text).unwrap();
        let compiled = Command::new(&toolchain.clang)
            .args(["-target", &target.triple, "-std=c11", "-c"])
            .arg(root.join("tests/fixtures/20o-b1/performance/clock-shim.c"))
            .arg("-o")
            .arg(&object)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let mut linker = Command::new(&toolchain.clang);
        linker
            .args(["-target", &target.triple, &format!("-O{level}")])
            .arg(&llvm_path)
            .arg(&object)
            .arg(toolchain.runtime_library.as_ref().unwrap())
            .args(&toolchain.native_static_libraries);
        if cfg!(windows) {
            linker.arg("-lpsapi");
        }
        let linked = linker.arg("-o").arg(&executable).output().unwrap();
        assert!(
            linked.status.success(),
            "{}",
            String::from_utf8_lossy(&linked.stderr)
        );
    }
    let mut child = Command::new(&executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // 配置在窗口外逐码点解读；int(str) 是码点接口，不承担多字符数字解析。
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{repeats:02}\n{count:02}\n").as_bytes())
        .unwrap();
    // 输出只有六条测量与黄金结果，不会填满管道；独立看门狗避免失控进程悬挂 CI。
    let deadline = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if deadline.elapsed() >= Duration::from_secs(60) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "原生测量超过 60 秒看门狗：{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let mut validated = 0;
    let mut samples = Vec::with_capacity(count);
    for line in stdout.lines() {
        if line == "true" {
            validated += 1;
        } else if let Some(values) = line.strip_prefix("B1-PERF-SAMPLE ") {
            let mut fields = values.split_whitespace();
            samples.push(Sample {
                seconds: fields.next().unwrap().parse::<f64>().unwrap(),
                resident_increment_upper_bytes: fields.next().unwrap().parse::<u64>().unwrap(),
            });
            assert!(fields.next().is_none());
        } else {
            panic!("非预期输出 {line:?}");
        }
    }
    assert_eq!(validated, count);
    assert_eq!(samples.len(), count);
    samples
}
