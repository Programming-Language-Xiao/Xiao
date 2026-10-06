//! 19D 原生基准可构建性探测；不把后端缺口伪装成性能数据。

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_driver::{FrontendCompiler, FrontendNativeDriver, FrontendRequest, NativeBuildRequest};

#[derive(Debug, Deserialize)]
struct Manifest {
    benchmarks: Vec<BenchmarkSpec>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkSpec {
    id: String,
    source: String,
}

/// 逐项探测 19D 清单；构建失败只作为数据不足记录，不在本批修原生后端。
#[test]
#[ignore = "需要 XIAO_CLANG、XIAO_RUNTIME_LIBRARY 与 XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
fn native_benchmark_probe_reports_every_manifest_program() {
    let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("应能定位仓库根")
        .to_path_buf();
    let manifest_path = repository_root.join("tests/benchmarks/manifest.json");
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(&manifest_path).expect("读取 19D manifest"))
            .expect("解析 19D manifest");
    let target = TargetDescription::host();
    assert_eq!(
        std::env::var("XIAO_TARGET_TRIPLE").expect("XIAO_TARGET_TRIPLE"),
        target.triple,
        "探测必须使用当前 Rust 主机目标"
    );
    let runtime =
        PathBuf::from(std::env::var_os("XIAO_RUNTIME_LIBRARY").expect("XIAO_RUNTIME_LIBRARY"));
    assert!(
        runtime.is_file(),
        "Runtime staticlib 不存在：{}",
        runtime.display()
    );
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain = Toolchain::new(std::env::var_os("XIAO_CLANG").expect("XIAO_CLANG"))
        .with_runtime_library(runtime)
        .probe_native_static_libraries(rustc, &target)
        .expect("应能读取 Runtime 原生库清单");
    let probe_root =
        std::env::temp_dir().join(format!("xiao-19d-native-probe-{}", std::process::id()));
    fs::create_dir_all(&probe_root).expect("创建探测目录");
    let compiler = FrontendCompiler::new();
    let driver = FrontendNativeDriver::new();
    let mut results = Vec::new();
    for benchmark in manifest.benchmarks {
        let source_path = repository_root
            .join("tests/benchmarks")
            .join(&benchmark.source);
        let source = fs::read_to_string(&source_path).expect("读取基准源码");
        let frontend_request = FrontendRequest::from_text_at(&source, &source_path);
        let artifact = match compiler.compile(&frontend_request) {
            Ok(artifact) => artifact,
            Err(error) => {
                results.push(format!("{}:frontend-rejected:{error}", benchmark.id));
                continue;
            }
        };
        let output = probe_root.join(format!("{}-native.exe", benchmark.id));
        let request =
            NativeBuildRequest::new(frontend_request, target.clone(), toolchain.clone(), &output);
        match driver.build_artifact(&artifact, &request) {
            Ok(_) => results.push(format!("{}:built", benchmark.id)),
            Err(error) => results.push(format!("{}:native-rejected:{error}", benchmark.id)),
        }
    }
    let _ = fs::remove_dir_all(&probe_root);
    for result in results {
        eprintln!("19D-NATIVE-PROBE {result}");
    }
}
