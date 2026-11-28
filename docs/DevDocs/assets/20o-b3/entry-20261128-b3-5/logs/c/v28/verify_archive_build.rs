//! 用同一入口驱动构建真实的 27 和 28 归档及缓存记录。

use std::path::PathBuf;
use xiao_driver::{ArchiveBuildRequest, CancellationToken, build_archive};

/// 将样本编译，输出归档身份及物化缓存的逐模块结果。
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [source, archive, cache] = args.as_slice() else {
        panic!("用法：verify_archive_build SOURCE ARCHIVE CACHE");
    };
    let mut request = ArchiveBuildRequest::new(source);
    request.output = Some(archive.clone());
    request.cache_root = Some(cache.clone());
    let built = build_archive(&request, &CancellationToken::new()).unwrap();
    println!(
        "{}",
        serde_json::json!({
            "producer_generator": xiao_codegen_llvm::CODEGEN_VERSION,
            "archive_path": built.output,
            "archive_sha256": built.digest.as_hex(),
            "schema": [built.index.schema_major, built.index.schema_minor],
            "runtime_abi": [built.index.runtime_abi_min, built.index.runtime_abi_max],
            "stdlib_interface": built.index.stdlib_interface_version,
            "capabilities": built.index.required_capabilities,
            "modules": built.modules.iter().map(|module| serde_json::json!({
                "module": module.module,
                "sha256": module.digest.as_hex(),
                "hit": module.hit,
            })).collect::<Vec<_>>(),
        }),
    );
}
