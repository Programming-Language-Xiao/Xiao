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

/// Windows x86_64-pc-windows-msvc 受控环境的当前构建基线（2026-10-07）。
///
/// 这张表只记录构建能力，不把被拒程序伪装成性能数据；新增拒绝或已有程序转为
/// 拒绝都会让门禁失败，原因由断言消息保留。
const EXPECTED_BUILT: [&str; 4] = [
    "deep-expression-arithmetic",
    "scalar-overflow-and-bool-parity",
    "named-local-loop",
    "deep-call-recursion",
];
const EXPECTED_REJECTED: [(&str, &str); 1] = [("container-dense", "动态表方法")];

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
    for result in &results {
        eprintln!("19D-NATIVE-PROBE {result}");
    }
    assert_eq!(
        results.len(),
        EXPECTED_BUILT.len() + EXPECTED_REJECTED.len(),
        "manifest 基准数量变化，必须先更新受控构建基线"
    );
    for benchmark in EXPECTED_BUILT {
        assert!(
            results
                .iter()
                .any(|result| result == &format!("{benchmark}:built")),
            "受控基线中的 {benchmark} 未构建成功：{results:?}"
        );
    }
    for (benchmark, reason) in EXPECTED_REJECTED {
        let result = results
            .iter()
            .find(|result| result.starts_with(&format!("{benchmark}:")))
            .unwrap_or_else(|| panic!("受控基线缺少 {benchmark}：{results:?}"));
        assert!(
            result.contains("native-rejected") && result.contains(reason),
            "{benchmark} 的拒绝原因变化：{result}"
        );
    }
}
