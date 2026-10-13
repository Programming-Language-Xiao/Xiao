//! Xiao 官方标准库的工程期元数据边界。
//!
//! 20C 官方标准库的工程期元数据边界。标准库源码由顶层 `stdlib/` 保存，
//! 工具链专用加载器负责提供它；本 crate 不把官方库注册为普通包依赖。

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// 返回官方标准库的固定源码命名空间。
pub const STANDARD_LIBRARY_NAMESPACE: &str = "stdlib";

/// 官方标准库文件模块身份的固定前缀。
pub const STANDARD_LIBRARY_PREFIX: &str = "stdlib.";

/// 允许工具链覆盖标准库源码根目录的环境变量。
pub const STANDARD_LIBRARY_ROOT_ENV: &str = "XIAO_STDLIB_ROOT";

/// 返回官方标准库规范源码根目录。
///
/// 发布工程可以通过 [`STANDARD_LIBRARY_ROOT_ENV`] 指向随工具链分发的源码；
/// 未设置时使用当前仓库中的顶层 `stdlib/`，便于源码树内的 VM/LLVM 回归。
#[must_use]
pub fn source_root() -> PathBuf {
    if let Some(root) = std::env::var_os(STANDARD_LIBRARY_ROOT_ENV) {
        return PathBuf::from(root);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .map_or_else(|| PathBuf::from("stdlib"), |root| root.join("stdlib"))
}
