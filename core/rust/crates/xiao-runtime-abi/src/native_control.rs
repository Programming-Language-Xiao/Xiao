//! Rust 启动器与原生 Runtime 共用的进程间参数。

use std::io;

/// 仅携带运行控制，环境快照仍从子进程宿主捕获。
pub const CONTROL_PREFIX: &str = "--__xiao-run-control=";
/// 与随机握手绑定的单调截止时间。
pub const DEADLINE_ENV: &str = "XIAO_INTERNAL_RUN_DEADLINE_NS";
/// 与随机握手绑定的取消通知文件。
pub const CANCEL_ENV: &str = "XIAO_INTERNAL_RUN_CANCEL_FILE";
/// 原生运行被宿主拒绝或中断时的进程退出码。
pub const INTERRUPTED_EXIT: i32 = 2;

/// 驱动器与 Runtime 共享的运行级中断原因。
#[derive(Clone, Copy, Debug)]
pub enum Interruption {
    /// 执行方显式取消本次运行。
    Cancelled,
    /// 运行截止已到达。
    Deadline,
}

impl Interruption {
    /// 返回冻结的诊断身份。
    pub const fn code(self) -> &'static str {
        match self {
            Self::Cancelled => "X09-DRIVER-001",
            Self::Deadline => "X09-DRIVER-002",
        }
    }

    /// 返回不含运行输入的诊断文本。
    pub const fn message(self) -> &'static str {
        match self {
            Self::Cancelled => "原生运行被取消",
            Self::Deadline => "原生运行超过截止时间",
        }
    }

    /// 判断挂起错误是否沿用驱动器的中断退出映射。
    pub fn has_code(code: &str) -> bool {
        code == Self::Cancelled.code() || code == Self::Deadline.code()
    }
}

/// 读取同一宿主进程间可比较的单调纳秒时间。
#[cfg(unix)]
pub fn monotonic_ns() -> io::Result<u128> {
    let mut ticks = std::mem::MaybeUninit::<libc::timespec>::uninit();
    // SAFETY: 系统调用只写入该 timespec，成功后即已初始化。
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, ticks.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let ticks = unsafe { ticks.assume_init() };
    Ok(ticks.tv_sec as u128 * 1_000_000_000 + ticks.tv_nsec as u128)
}

/// 读取同一宿主进程间可比较的单调纳秒时间。
#[cfg(windows)]
pub fn monotonic_ns() -> io::Result<u128> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn QueryPerformanceCounter(value: *mut i64) -> i32;
        fn QueryPerformanceFrequency(value: *mut i64) -> i32;
    }
    /// 进程内缓存的 QPC 频率。
    static FREQUENCY: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
    let frequency = *FREQUENCY.get_or_init(|| {
        let mut value = 0;
        // SAFETY: 系统只写入一个 LARGE_INTEGER。
        if unsafe { QueryPerformanceFrequency(&mut value) } != 0 {
            value
        } else {
            0
        }
    });
    let mut ticks = 0;
    // SAFETY: 系统只写入一个 LARGE_INTEGER。
    if frequency <= 0 || unsafe { QueryPerformanceCounter(&mut ticks) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(ticks as u128 * 1_000_000_000 / frequency as u128)
}

/// 其他平台的共享单调时钟暂未接线。
#[cfg(not(any(unix, windows)))]
pub fn monotonic_ns() -> io::Result<u128> {
    Err(io::ErrorKind::Unsupported.into())
}
