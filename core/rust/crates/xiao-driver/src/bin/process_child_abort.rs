//! Unix 信号退出与 Windows 异常码归一的确定性子程序。

/// 不安装 Xiao 的崩溃处理器，确保父进程观察到真实 SIGTERM 或 Windows 状态码。
fn main() {
    #[cfg(unix)]
    unsafe {
        libc::raise(libc::SIGTERM);
    }
    #[cfg(windows)]
    std::process::exit(0xC000_0005_u32 as i32);
    #[cfg(not(windows))]
    std::process::exit(92);
}
