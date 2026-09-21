//! Xiao 原生程序使用的稳定 C ABI 边界。
//!
//! N0-A 只登记边界，不把完整 Runtime 链接进静态标量产物。句柄的载荷和生命周期由后续
//! Runtime 实现负责；本 crate 的公开类型刻意不包含任何 Rust 内部布局。

use std::io::{self, Write};

/// 当前原生 Runtime ABI 的主版本。
pub const ABI_VERSION: u32 = 1;

/// 一个只可由 Runtime 实现解释的不透明对象句柄。
///
/// 该类型没有可构造的公开字段。生成代码只能传递指针和长度，不能假设对象头、计数器
/// 或载荷的布局。
#[repr(C)]
pub struct XiaoOpaqueHandle {
    _private: [u8; 0],
}

/// C ABI 中使用的不透明句柄指针类型。
pub type XiaoHandle = *mut XiaoOpaqueHandle;

/// 用于 ABI 边界的固定宽度源码区间。
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XiaoAbiSpan {
    /// 起始字节偏移。
    pub start: u64,
    /// 结束字节偏移（不包含）。
    pub end: u64,
}

/// 返回 ABI 主版本，供生成代码和链接器兼容检查使用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_abi_version() -> u32 {
    ABI_VERSION
}

/// 保留一个不透明句柄并返回同一地址。
///
/// N0-A 不创建 Runtime 对象，因此该入口只提供稳定的链接符号和空句柄语义。真正的计数
/// 操作由 N0-B 的 Runtime 实现替换；空指针始终安全地原样返回。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_retain(handle: XiaoHandle) -> XiaoHandle {
    handle
}

/// 释放一个不透明句柄。
///
/// 当前实现不触碰载荷，因为 N0-A 不拥有 Runtime 对象。调用方可以无条件传入空指针。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_release(_handle: XiaoHandle) {}

/// 把一个固定宽度整数写到标准输出，返回 C 风格状态码。
///
/// 这是后续最小观察面所需的单一 ABI 入口，不是 builtin 分派机制。N0-A 默认生成的纯
/// 静态程序不调用它，因此不会因为它而链接完整 Runtime。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_write_i64(value: i64) -> i32 {
    let result = writeln!(io::stdout(), "{value}");
    if result.is_ok() { 0 } else { 1 }
}
