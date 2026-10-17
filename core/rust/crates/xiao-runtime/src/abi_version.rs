//! Runtime ABI 版本查询和启动兼容检查；比较规则与 ABI 契约测试共用。

use xiao_runtime_abi::{ABI_MAJOR_VERSION, ABI_MINOR_VERSION, is_abi_compatible};

/// ABI 版本主入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_abi_version() -> u32 {
    ABI_MAJOR_VERSION
}

/// ABI 版本次入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_abi_minor_version() -> u32 {
    ABI_MINOR_VERSION
}

/// 检查产物版本要求，并在用户代码开始前安装平台故障报告器。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_abi_is_compatible(required_major: u32, required_minor: u32) -> i32 {
    crate::crash::install_platform_failure_reporter();
    i32::from(is_abi_compatible(
        required_major,
        required_minor,
        ABI_MAJOR_VERSION,
        ABI_MINOR_VERSION,
    ))
}
