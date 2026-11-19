//! 使用发布版 Runtime 静态库验证组件声明和 ELF 裁剪结果。

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    RUNTIME_SYMBOL_PREFIX, components_for_symbols, inspect_artifact, reachable_runtime_components,
    runtime_link_dependencies, unclassified_runtime_symbols,
};
use crate::{TargetDescription, Toolchain};

fn runtime_exports(llvm_nm: &Path, runtime: &Path) -> BTreeSet<String> {
    let output = Command::new(llvm_nm)
        .args(["--defined-only", "--extern-only", "--format=posix"])
        .arg(runtime)
        .output()
        .unwrap_or_else(|error| panic!("运行 {} 失败：{error}", llvm_nm.display()));
    assert!(
        output.status.success(),
        "读取 Runtime 静态库导出失败：{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let listing = String::from_utf8(output.stdout).unwrap();
    let exports: BTreeSet<_> = listing
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let symbol = fields.next()?;
            let kind = fields.next()?;
            // POSIX 格式也列出归档成员标题，实际符号行另有单字符类型。
            (kind.len() == 1 && symbol.starts_with(RUNTIME_SYMBOL_PREFIX)).then_some(symbol)
        })
        .map(str::to_owned)
        .collect();
    assert!(!exports.is_empty(), "Runtime 静态库没有 Xiao ABI 导出");
    exports
}

#[test]
#[ignore = "Linux 原生维护需要 release Runtime、clang 和 LLVM 工具"]
fn runtime_components_match_release_archive_exports() {
    let target = TargetDescription::host();
    let runtime = PathBuf::from(env::var_os("XIAO_RUNTIME_LIBRARY").expect("XIAO_RUNTIME_LIBRARY"));
    let clang = PathBuf::from(env::var_os("XIAO_CLANG").expect("XIAO_CLANG"));
    let llvm_as = PathBuf::from(env::var_os("XIAO_LLVM_AS").expect("XIAO_LLVM_AS"));
    let llvm_nm = env::var_os("XIAO_LLVM_NM")
        .map(PathBuf::from)
        .unwrap_or_else(|| llvm_as.with_file_name("llvm-nm"));
    let exports = runtime_exports(&llvm_nm, &runtime);
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain = Toolchain::new(clang)
        .with_llvm_as(llvm_as)
        .with_runtime_library(runtime)
        .probe_native_static_libraries(rustc, &target)
        .unwrap();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = env::temp_dir().join(format!("xiao-runtime-link-{}-{stamp}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let executable = root.join("probe");
    let mut mismatches = Vec::new();
    for symbol in &exports {
        // 以 volatile 保存地址，让链接器保留唯一根符号。探针无需调用 ABI 函数。
        let text = format!(
            "declare void @{symbol}()\ndefine i32 @main() {{\nentry:\n\
             %slot = alloca ptr\nstore volatile ptr @{symbol}, ptr %slot\nret i32 0\n}}\n"
        );
        let declared = reachable_runtime_components(&text);
        toolchain
            .compile_with_runtime(&text, &target, &executable, None)
            .unwrap_or_else(|error| panic!("链接 {symbol} 的探针失败：{error}"));
        let inspection = inspect_artifact(&executable, &target).unwrap();
        assert!(
            inspection.symbol_table_readable && inspection.symbols.contains(symbol),
            "ELF 缺少根符号 {symbol}"
        );
        // 归档导出集合排除 ELF 的 FILE 符号等对象内部记录。
        let linked: Vec<_> = inspection
            .symbols
            .into_iter()
            .filter(|symbol| exports.contains(symbol))
            .collect();
        let observed = components_for_symbols(&linked);
        let unclassified = unclassified_runtime_symbols(&linked);
        let missing: Vec<_> = runtime_link_dependencies(symbol)
            .iter()
            .copied()
            .filter(|dependency| !linked.iter().any(|actual| actual == dependency))
            .collect();
        if declared != observed || !missing.is_empty() || !unclassified.is_empty() {
            mismatches.push(format!(
                "{symbol} 声明={declared:?} 观测={observed:?} 缺少依赖符号={missing:?} 未分类={unclassified:?} 符号={linked:?}"
            ));
        }
    }
    fs::remove_file(executable).unwrap();
    fs::remove_dir(root).unwrap();
    assert!(mismatches.is_empty(), "\n{}", mismatches.join("\n"));
    println!(
        "RUNTIME-COMPONENTS exports={} target={} result=passed",
        exports.len(),
        target.triple
    );
}
