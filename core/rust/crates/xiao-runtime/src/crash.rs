//! 平台级不可恢复异常报告。
//!
//! 处理器只写入预先确定的静态字节串，不分配、不取锁、不访问消息目录，也不把平台
//! 异常转换成可恢复 Xiao 错误。

use std::sync::Once;

/// 按进程 API 契约保留正常退出码；Unix 信号与 Windows SEH 分别归一。
pub(crate) fn process_exit_code(status: std::process::ExitStatus) -> i64 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status
            .code()
            .map(i64::from)
            .unwrap_or_else(|| 128 + i64::from(status.signal().unwrap_or(0)))
    }
    #[cfg(windows)]
    {
        let code = status.code().unwrap_or(0) as u32;
        if code & 0xC000_0000 == 0xC000_0000 {
            i64::from(windows_exception_exit_code(code))
        } else {
            i64::from(code)
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        i64::from(status.code().unwrap_or(0))
    }
}

static INSTALL: Once = Once::new();

/// 安装当前平台的最小未处理异常报告器。
pub(crate) fn install_platform_failure_reporter() {
    INSTALL.call_once(install);
}

#[cfg(unix)]
fn install() {
    unsafe {
        install_alternate_signal_stack();
        install_unix_signal(libc::SIGSEGV);
        install_unix_signal(libc::SIGBUS);
        install_unix_signal(libc::SIGFPE);
        install_unix_signal(libc::SIGILL);
        install_unix_signal(libc::SIGTRAP);
        install_unix_signal(libc::SIGABRT);
    }
}

#[cfg(unix)]
unsafe fn install_alternate_signal_stack() {
    let size = libc::SIGSTKSZ.saturating_mul(4);
    let stack = vec![0_u8; size].into_boxed_slice();
    let stack = Box::leak(stack);
    let alternate = libc::stack_t {
        ss_sp: stack.as_mut_ptr().cast(),
        ss_flags: 0,
        ss_size: stack.len(),
    };
    unsafe {
        let _ = libc::sigaltstack(&alternate, std::ptr::null_mut());
    }
}

#[cfg(unix)]
unsafe fn install_unix_signal(signal: libc::c_int) {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = signal_handler as *const () as usize;
        action.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
        libc::sigemptyset(&mut action.sa_mask);
        let _ = libc::sigaction(signal, &action, std::ptr::null_mut());
    }
}

#[cfg(unix)]
/// 按故障类别返回冻结的退出码；BSD 的信号编号不等同于 Linux。
fn unix_signal_exit_code(signal: libc::c_int) -> libc::c_int {
    match signal {
        libc::SIGSEGV | libc::SIGBUS => 139,
        libc::SIGFPE => 136,
        libc::SIGILL | libc::SIGTRAP => 132,
        _ => 134,
    }
}

#[cfg(unix)]
extern "C" fn signal_handler(signal: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    let message = match signal {
        libc::SIGSEGV | libc::SIGBUS => {
            b"xiao-error class=fatal code=X07-FATAL-005 message_id=fatal.hardware params={} span=<none> exit_code=139\n"
        }
        libc::SIGFPE => {
            b"xiao-error class=fatal code=X07-FATAL-005 message_id=fatal.hardware params={} span=<none> exit_code=136\n"
        }
        // AArch64 的 llvm.trap 降为 brk，macOS 报 SIGTRAP；与 x86 的 SIGILL
        // 同属外部硬件故障，保持跨平台诊断身份和退出码一致。
        libc::SIGILL | libc::SIGTRAP => {
            b"xiao-error class=fatal code=X07-FATAL-005 message_id=fatal.hardware params={} span=<none> exit_code=132\n"
        }
        _ => {
            b"xiao-error class=fatal code=X07-FATAL-006 message_id=fatal.internal params={} span=<none> exit_code=134\n"
        }
    };
    unsafe {
        let _ = libc::write(libc::STDERR_FILENO, message.as_ptr().cast(), message.len());
        libc::_exit(unix_signal_exit_code(signal));
    }
}

#[cfg(windows)]
fn install() {
    unsafe {
        SetUnhandledExceptionFilter(Some(unhandled_exception));
    }
}

#[cfg(windows)]
#[repr(C)]
struct ExceptionRecord {
    code: u32,
    flags: u32,
    record: *mut ExceptionRecord,
    address: *mut std::ffi::c_void,
    parameters_count: u32,
    information: [usize; 15],
}

#[cfg(windows)]
#[repr(C)]
struct ExceptionPointers {
    record: *mut ExceptionRecord,
    context: *mut std::ffi::c_void,
}

#[cfg(windows)]
type ExceptionFilter = unsafe extern "system" fn(*mut ExceptionPointers) -> i32;

#[cfg(windows)]
/// 把 Windows SEH 异常类别映射到与 Unix 信号处理器相同的退出码。
fn windows_exception_exit_code(code: u32) -> u32 {
    match code {
        0xC0000005 => 139,              // EXCEPTION_ACCESS_VIOLATION
        0xC000001D => 132,              // EXCEPTION_ILLEGAL_INSTRUCTION
        0xC0000094 | 0xC000008E => 136, // integer / floating divide by zero
        _ => 134,
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetUnhandledExceptionFilter(filter: Option<ExceptionFilter>) -> Option<ExceptionFilter>;
    fn GetStdHandle(standard_handle: u32) -> *mut std::ffi::c_void;
    fn WriteFile(
        handle: *mut std::ffi::c_void,
        buffer: *const std::ffi::c_void,
        length: u32,
        written: *mut u32,
        overlapped: *mut std::ffi::c_void,
    ) -> i32;
    fn ExitProcess(code: u32) -> !;
}

#[cfg(windows)]
unsafe extern "system" fn unhandled_exception(pointers: *mut ExceptionPointers) -> i32 {
    const STD_ERROR_HANDLE: u32 = u32::MAX - 11;
    let exception_code = unsafe {
        pointers
            .as_ref()
            .and_then(|pointers| pointers.record.as_ref())
            .map_or(0, |record| record.code)
    };
    let exit_code = windows_exception_exit_code(exception_code);
    let message = match exit_code {
        139 => b"xiao-error class=fatal code=X07-FATAL-005 message_id=fatal.hardware params={} span=<none> exit_code=139\r\n" as &[u8],
        132 => b"xiao-error class=fatal code=X07-FATAL-005 message_id=fatal.hardware params={} span=<none> exit_code=132\r\n",
        136 => b"xiao-error class=fatal code=X07-FATAL-005 message_id=fatal.hardware params={} span=<none> exit_code=136\r\n",
        _ => b"xiao-error class=fatal code=X07-FATAL-005 message_id=fatal.hardware params={} span=<none> exit_code=134\r\n",
    };
    unsafe {
        let handle = GetStdHandle(STD_ERROR_HANDLE);
        let mut written = 0;
        let _ = WriteFile(
            handle,
            message.as_ptr().cast(),
            message.len() as u32,
            &mut written,
            std::ptr::null_mut(),
        );
        ExitProcess(exit_code);
    }
}

#[cfg(not(any(unix, windows)))]
fn install() {}

#[cfg(all(test, windows))]
mod tests {
    use super::{process_exit_code, windows_exception_exit_code};

    #[test]
    fn windows_exception_classes_share_unix_exit_codes() {
        assert_eq!(windows_exception_exit_code(0xC0000005), 139);
        assert_eq!(windows_exception_exit_code(0xC000001D), 132);
        assert_eq!(windows_exception_exit_code(0xC0000094), 136);
        assert_eq!(windows_exception_exit_code(0xC000008E), 136);
        assert_eq!(windows_exception_exit_code(0xC0000000), 134);
    }

    #[test]
    /// 子进程的普通 OS 码保持原值，NTSTATUS 异常复用现有故障类别。
    fn process_exit_codes_preserve_normal_values_and_map_exceptions() {
        use std::os::windows::process::ExitStatusExt;
        for (raw, expected) in [
            (7, 7),
            (0x8000_0001, 0x8000_0001),
            (0xC000_0005, 139),
            (0xC000_001D, 132),
            (0xC000_0094, 136),
            (0xC000_0000, 134),
        ] {
            assert_eq!(
                process_exit_code(std::process::ExitStatus::from_raw(raw)),
                expected
            );
        }
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    use super::{process_exit_code, unix_signal_exit_code};

    #[test]
    fn hardware_signals_use_frozen_exit_codes() {
        assert_eq!(unix_signal_exit_code(libc::SIGSEGV), 139);
        assert_eq!(unix_signal_exit_code(libc::SIGBUS), 139);
        assert_eq!(unix_signal_exit_code(libc::SIGFPE), 136);
        assert_eq!(unix_signal_exit_code(libc::SIGILL), 132);
        assert_eq!(unix_signal_exit_code(libc::SIGTRAP), 132);
        assert_eq!(unix_signal_exit_code(libc::SIGABRT), 134);
    }

    #[test]
    /// 进程 API 使用 128+signal，不能误用 Runtime 自身的硬件故障退出码。
    fn process_status_preserves_exit_codes_and_signal_numbers() {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            process_exit_code(std::process::ExitStatus::from_raw(7 << 8)),
            7
        );
        for signal in [libc::SIGTERM, libc::SIGBUS, libc::SIGABRT] {
            assert_eq!(
                process_exit_code(std::process::ExitStatus::from_raw(signal)),
                128 + i64::from(signal)
            );
        }
    }
}
