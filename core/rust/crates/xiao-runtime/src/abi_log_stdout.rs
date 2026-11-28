//! 日志同步直写 stdout 的平台边界，状态和配额由调用方管理。

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;
    use std::io;
    use std::ptr;

    /// Windows 标准输出的句柄编号。
    const STD_OUTPUT_HANDLE: u32 = u32::MAX - 10;
    /// GetFileType 返回的管道类型。
    const FILE_TYPE_PIPE: u32 = 3;
    /// NT 查询类和返回记录中的已连接状态。
    const FILE_PIPE_LOCAL_INFORMATION: u32 = 24;
    const FILE_PIPE_CONNECTED_STATE: u32 = 3;

    #[repr(C)]
    struct IoStatusBlock {
        status: usize,
        information: usize,
    }

    #[repr(C)]
    struct PipeLocalInformation {
        _pipe_type: u32,
        _configuration: u32,
        _maximum_instances: u32,
        _current_instances: u32,
        _inbound_quota: u32,
        _read_data_available: u32,
        _outbound_quota: u32,
        write_quota_available: u32,
        state: u32,
        _end: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(which: u32) -> *mut c_void;
        fn GetFileType(handle: *mut c_void) -> u32;
        fn GetConsoleMode(handle: *mut c_void, mode: *mut u32) -> i32;
        fn WriteFile(
            handle: *mut c_void,
            data: *const c_void,
            len: u32,
            written: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        fn WriteConsoleW(
            handle: *mut c_void,
            data: *const u16,
            len: u32,
            written: *mut u32,
            reserved: *mut c_void,
        ) -> i32;
    }

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQueryInformationFile(
            handle: *mut c_void,
            status: *mut IoStatusBlock,
            info: *mut PipeLocalInformation,
            length: u32,
            class: u32,
        ) -> i32;
        fn RtlNtStatusToDosError(status: i32) -> u32;
    }

    /// 借用可用的宿主 stdout，不接管句柄。
    fn stdout_handle() -> io::Result<(*mut c_void, u32)> {
        let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        if handle.is_null() {
            return Err(io::ErrorKind::NotConnected.into());
        }
        let kind = unsafe { GetFileType(handle) };
        if kind == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok((handle, kind))
    }

    /// 查询管道的可写配额，避免为等待另一端消费数据而阻塞。
    fn pipe_quota(handle: *mut c_void) -> io::Result<usize> {
        let mut status = std::mem::MaybeUninit::uninit();
        let mut info = std::mem::MaybeUninit::<PipeLocalInformation>::uninit();
        let result = unsafe {
            NtQueryInformationFile(
                handle,
                status.as_mut_ptr(),
                info.as_mut_ptr(),
                std::mem::size_of::<PipeLocalInformation>() as u32,
                FILE_PIPE_LOCAL_INFORMATION,
            )
        };
        if result != 0 {
            return Err(io::Error::from_raw_os_error(
                unsafe { RtlNtStatusToDosError(result) } as i32,
            ));
        }
        let info = unsafe { info.assume_init() };
        if info.state != FILE_PIPE_CONNECTED_STATE {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        Ok(info.write_quota_available as usize)
    }

    /// 区分缺失或失效的 stdout 和可用的宿主流。
    pub fn check() -> io::Result<()> {
        let (handle, kind) = stdout_handle()?;
        if kind == FILE_TYPE_PIPE {
            pipe_quota(handle)?;
        }
        Ok(())
    }

    /// 只查询写端的容量。等待和取消由共享日志交付控制。
    pub fn write_ready() -> io::Result<bool> {
        let (handle, kind) = stdout_handle()?;
        Ok(kind != FILE_TYPE_PIPE || pipe_quota(handle)? != 0)
    }

    /// 管道和文件按原始字节直写，真实控制台用宽字符避免受代码页影响。
    pub fn write(bytes: &[u8]) -> io::Result<usize> {
        let (handle, kind) = stdout_handle()?;
        let mut mode = 0;
        if unsafe { GetConsoleMode(handle, &mut mode) } != 0 {
            return write_console(handle, bytes);
        }
        let bytes = if kind == FILE_TYPE_PIPE {
            let quota = pipe_quota(handle)?;
            if quota == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "stdout 管道暂不可写",
                ));
            }
            &bytes[..bytes.len().min(quota)]
        } else {
            bytes
        };
        let mut written = 0;
        let ok = unsafe {
            WriteFile(
                handle,
                bytes.as_ptr().cast(),
                bytes.len() as u32,
                &mut written,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(written as usize)
    }

    /// 将控制台报告的宽字符进度映射回完整的 UTF-8 字符边界。
    fn write_console(handle: *mut c_void, bytes: &[u8]) -> io::Result<usize> {
        let text = std::str::from_utf8(bytes).map_err(io::Error::other)?;
        let mut wide = [0; crate::log::MAX_RECORD_BYTES];
        let mut len = 0;
        for unit in text.encode_utf16() {
            wide[len] = unit;
            len += 1;
        }
        let mut written = 0;
        let ok = unsafe {
            WriteConsoleW(
                handle,
                wide.as_ptr(),
                len as u32,
                &mut written,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut remaining = written as usize;
        for (offset, ch) in text.char_indices() {
            if remaining == 0 {
                return Ok(offset);
            }
            remaining = remaining
                .checked_sub(ch.len_utf16())
                .ok_or(io::ErrorKind::InvalidData)?;
        }
        if remaining == 0 {
            Ok(bytes.len())
        } else {
            Err(io::ErrorKind::InvalidData.into())
        }
    }
}

#[cfg(unix)]
mod platform {
    use std::io;
    use std::mem::MaybeUninit;
    use std::ptr;

    /// 检查 stdout 描述符及访问模式。Rust 标准流可能忽略关闭的 stdout。
    pub fn check() -> io::Result<()> {
        let flags = unsafe { libc::fcntl(libc::STDOUT_FILENO, libc::F_GETFL) };
        if flags == -1 {
            return Err(io::Error::last_os_error());
        }
        if flags & libc::O_ACCMODE == libc::O_RDONLY {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(())
    }

    /// 检查流的可写状态，把等待交给共享日志期限。
    pub fn write_ready() -> io::Result<bool> {
        let mut fd = libc::pollfd {
            fd: libc::STDOUT_FILENO,
            events: libc::POLLOUT,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut fd, 1, 0) };
        if ready < 0 {
            return Err(io::Error::last_os_error());
        }
        if fd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        Ok(ready != 0 && fd.revents & libc::POLLOUT != 0)
    }

    /// 实际写入的同步系统调用在返回后检查运行控制。
    pub fn write(bytes: &[u8]) -> io::Result<usize> {
        // 单个写入至多使用 POSIX 保证的最小 PIPE_BUF，将控制交还给 LogState。
        write_without_sigpipe(&bytes[..bytes.len().min(512)])
    }

    /// 仅在当前线程屏蔽本次 write 产生的 SIGPIPE，保留宿主自己的信号设置。
    fn write_without_sigpipe(bytes: &[u8]) -> io::Result<usize> {
        let mut blocked = MaybeUninit::uninit();
        let mut old = MaybeUninit::uninit();
        unsafe {
            libc::sigemptyset(blocked.as_mut_ptr());
            libc::sigaddset(blocked.as_mut_ptr(), libc::SIGPIPE);
            let err = libc::pthread_sigmask(libc::SIG_BLOCK, blocked.as_ptr(), old.as_mut_ptr());
            if err != 0 {
                return Err(io::Error::from_raw_os_error(err));
            }
            let was_pending = sigpipe_pending();
            let count = libc::write(libc::STDOUT_FILENO, bytes.as_ptr().cast(), bytes.len());
            let result = if count < 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::EPIPE) && !was_pending && sigpipe_pending() {
                    let mut signal = 0;
                    libc::sigwait(blocked.as_ptr(), &mut signal);
                }
                Err(error)
            } else {
                Ok(count as usize)
            };
            libc::pthread_sigmask(libc::SIG_SETMASK, old.as_ptr(), ptr::null_mut());
            result
        }
    }

    /// 判断当前线程是否已有待交付的 SIGPIPE。
    fn sigpipe_pending() -> bool {
        let mut pending = MaybeUninit::uninit();
        unsafe {
            libc::sigpending(pending.as_mut_ptr()) == 0
                && libc::sigismember(pending.as_ptr(), libc::SIGPIPE) == 1
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use std::io;

    /// 无宿主 stdout 适配的目标拒绝接纳。
    pub fn check() -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// 无宿主 stdout 适配的目标拒绝查询写入状态。
    pub fn write_ready() -> io::Result<bool> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// 无宿主 stdout 适配的目标拒绝写入。
    pub fn write(_bytes: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::Unsupported.into())
    }
}

pub(super) use platform::{check, write, write_ready};
