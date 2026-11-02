//! B1 字符串释放差异的显式复现；记录实际失败，不进入默认测试或生产入口。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use xiao_codegen_llvm::{CodegenOptions, TargetDescription, Toolchain};
use xiao_driver::{
    DriverRequest, FrontendCompiler, FrontendNativeDriver, FrontendRequest, FrontendVmDriver,
    NativeBuildRequest,
};
use xiao_runtime::{start_release_trace, take_release_events};
use xiao_vm::VmEvent;

/// 与现有原生门禁使用同一显式环境；未配置就失败，不登记为执行。
fn toolchain() -> (TargetDescription, Toolchain) {
    let target = TargetDescription::host();
    assert_eq!(
        std::env::var("XIAO_TARGET_TRIPLE").expect("需要 XIAO_TARGET_TRIPLE"),
        target.triple
    );
    let required = |name: &str| {
        let path = PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("需要 {name}")));
        assert!(path.is_file(), "{name} 文件不存在");
        path
    };
    let toolchain = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .expect("探测 Runtime 链接依赖");
    (target, toolchain)
}

/// 产物留在独立临时目录，完整日志/IR/LLVM/trace 可以继续归因。
fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let directory =
        std::env::temp_dir().join(format!("xiao-b1-release-diagnostic-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let full = fs::read_to_string(root.join("tests/fixtures/20o-b1/imp/string-index/product.xiao"))
        .unwrap();
    let golden: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/20o-b1/imp/string-index/golden.json"))
            .unwrap(),
    )
    .unwrap();
    let expected_full = golden["stdout"].as_str().unwrap();
    let success = full.split_once("try\n").expect("已知错误段").0;
    let expected_success = expected_full.split_once("X06-RUNTIME-017\n").unwrap().0;
    let (target, toolchain) = toolchain();
    let mut records = Vec::new();
    let mut mismatch = false;
    for (case, source, expected) in [
        ("success", success, expected_success),
        ("with-bounds", full.as_str(), expected_full),
    ] {
        let source_path = directory.join(format!("{case}.xiao"));
        fs::write(&source_path, source).unwrap();
        let request = FrontendRequest::from_text_at(source, &source_path);
        let artifact = FrontendCompiler::new().compile(&request).unwrap();
        fs::write(
            directory.join(format!("{case}.ir.json")),
            serde_json::to_vec_pretty(&artifact.ir).unwrap(),
        )
        .unwrap();
        let trace = start_release_trace();
        let vm = FrontendVmDriver::new().run_artifact(
            &artifact,
            &DriverRequest::new(request.clone()).with_timeout(Duration::from_secs(10)),
        );
        let events = take_release_events();
        drop(trace);
        assert!(vm.is_success(), "{case}: {vm:?}");
        let stdout: String = vm
            .as_executed()
            .unwrap()
            .events()
            .iter()
            .filter_map(|event| match event {
                VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(stdout, expected);
        let vm_destroy = events
            .iter()
            .filter(|event| event.action.as_str() == "destroy")
            .count();
        let vm_trace: String = events
            .iter()
            .map(|event| {
                format!(
                    "{}\t{}\t{}\n",
                    event.sequence,
                    event.object_id,
                    event.action.as_str()
                )
            })
            .collect();
        fs::write(directory.join(format!("{case}-vm.trace.tsv")), vm_trace).unwrap();
        for level in [0, 3] {
            let executable =
                directory.join(format!("{case}-o{level}{}", std::env::consts::EXE_SUFFIX));
            let options = CodegenOptions::for_target(target.clone())
                .with_optimization_level(level)
                .unwrap();
            let native = FrontendNativeDriver::new()
                .build_artifact(
                    &artifact,
                    &NativeBuildRequest::new(
                        request.clone(),
                        target.clone(),
                        toolchain.clone(),
                        &executable,
                    )
                    .with_codegen_options(options),
                )
                .unwrap();
            fs::write(
                directory.join(format!("{case}-o{level}.ll")),
                &native.native.module.text,
            )
            .unwrap();
            let trace_path = directory.join(format!("{case}-o{level}.trace.tsv"));
            let output = Command::new(&executable)
                .env("XIAO_RUNTIME_RELEASE_TRACE_PATH", &trace_path)
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(0));
            assert_eq!(output.stdout, expected.as_bytes());
            assert!(output.stderr.is_empty());
            let native_trace = fs::read_to_string(&trace_path).unwrap();
            let native_destroy = native_trace
                .lines()
                .filter(|line| line.ends_with("\tdestroy"))
                .count();
            mismatch |= vm_destroy != native_destroy;
            let record = serde_json::json!({"case":case,"optimization":level,"vm_destroy":vm_destroy,"native_destroy":native_destroy,"matched":vm_destroy == native_destroy});
            println!("{record}");
            records.push(record);
        }
    }
    fs::write(
        directory.join("results.json"),
        serde_json::to_vec_pretty(&records).unwrap(),
    )
    .unwrap();
    println!("artifacts={}", directory.display());
    if mismatch {
        std::process::exit(1);
    }
}
