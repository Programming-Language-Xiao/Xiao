//! 平台级不可恢复异常报告。
//!
//! 处理器只写入预先确定的静态字节串，不分配、不取锁、不访问消息目录，也不把平台
//! 异常转换成可恢复 Xiao 错误。

use std::sync::Once;

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
        install_unix_signal(libc::SIGABRT);
    }
}

#[cfg(unix)]
unsafe fn install_alternate_signal_stack() {
    let size = (libc::SIGSTKSZ as usize).saturating_mul(4);
    let stack = vec![0_u8; size].into_boxed_slice();
    let stack = Box::leak(stack);
    let alternate = libc::stack_t {
        ss_sp: stack.as_mut_ptr().cast(),
        ss_flags: 0,
        ss_size: stack.len(),
    };
    let _ = libc::sigaltstack(&alternate, std::ptr::null_mut());
}

#[cfg(unix)]
unsafe fn install_unix_signal(signal: libc::c_int) {
    let mut action: libc::sigaction = std::mem::zeroed();
    action.sa_sigaction = signal_handler as usize;
    action.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
    libc::sigemptyset(&mut action.sa_mask);
    let _ = libc::sigaction(signal, &action, std::ptr::null_mut());
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
        libc::SIGILL => {
            b"xiao-error class=fatal code=X07-FATAL-005 message_id=fatal.hardware params={} span=<none> exit_code=132\n"
        }
        _ => {
            b"xiao-error class=fatal code=X07-FATAL-006 message_id=fatal.internal params={} span=<none> exit_code=134\n"
        }
    };
    unsafe {
        let _ = libc::write(libc::STDERR_FILENO, message.as_ptr().cast(), message.len());
        libc::_exit(128 + signal);
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
unsafe extern "system" fn unhandled_exception(_: *mut ExceptionPointers) -> i32 {
    const STD_ERROR_HANDLE: u32 = u32::MAX - 11;
    let message = b"xiao-error class=fatal code=X07-FATAL-005 message_id=fatal.hardware params={} span=<none> exit_code=139\r\n";
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
        ExitProcess(139);
    }
}

#[cfg(not(any(unix, windows)))]
fn install() {}
