//! Xiao 官方标准库的工程期元数据边界。
//!
//! 20C 官方标准库的工程期元数据边界。标准库源码由顶层 `stdlib/` 保存，
//! 工具链专用加载器负责提供它；本 crate 不把官方库注册为普通包依赖。

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
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

/// 当前官方标准库接口身份的主版本。
///
/// 该版本只描述标准库公开接口，不改变既有产物兼容矩阵的版本轴。
pub const STDLIB_INTERFACE_VERSION: u32 = 1;

/// 官方标准库的并列兼容身份块。
///
/// 20C 先把这组可读字段提供给构建前检查和后续产物元数据接线；
/// 驱动器兼容矩阵仍保留原有产物版本轴；归档层复用此权威身份。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StandardLibraryIdentity {
    /// Xiao 语言语义版本。
    pub language_version: String,
    /// 官方标准库公开接口主版本。
    pub stdlib_interface_version: u32,
    /// Runtime ABI 主版本，来自 `xiao-runtime-abi`。
    pub runtime_abi_major: u32,
    /// Runtime ABI 次版本，来自 `xiao-runtime-abi`。
    pub runtime_abi_minor: u32,
    /// 目标平台标签。
    pub target: String,
}

/// 返回由语言版本、标准库接口版本和 Runtime ABI 常量生成的身份块。
#[must_use]
pub fn current_standard_library_identity() -> StandardLibraryIdentity {
    StandardLibraryIdentity {
        language_version: "0.1.0".to_owned(),
        stdlib_interface_version: STDLIB_INTERFACE_VERSION,
        runtime_abi_major: xiao_runtime_abi::ABI_MAJOR_VERSION,
        runtime_abi_minor: xiao_runtime_abi::ABI_MINOR_VERSION,
        target: "portable".to_owned(),
    }
}
