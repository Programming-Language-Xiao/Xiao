//! 宿主的目录项原子移动与非递归删除。

use std::fs::{self, FileType};
use std::io;
use std::path::Path;

/// 通过宿主目录项操作移动，按覆盖标记选择提交方式。
pub(super) fn rename(source: &Path, destination: &Path, overwrite: bool) -> io::Result<()> {
    if overwrite {
        fs::rename(source, destination)
    } else {
        rename_no_replace(source, destination)
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
/// 在一次内核调用中检查目标并移动目录项。
fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let source = CString::new(source.as_os_str().as_bytes())?;
    let destination = CString::new(destination.as_os_str().as_bytes())?;
    // SAFETY: 两条路径都在调用期间持有以 NUL 结尾的缓冲区。
    let result = unsafe {
        #[cfg(target_os = "linux")]
        {
            libc::syscall(
                libc::SYS_renameat2,
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                destination.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        }
        #[cfg(target_os = "macos")]
        {
            libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL)
        }
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
/// 在 Windows 上使用禁止覆盖与跨卷复制的移动入口。
fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        /// 移动目录项，标记决定覆盖和跨卷规则。
        fn MoveFileExW(source: *const u16, destination: *const u16, flags: u32) -> i32;
    }

    /// 以宿主绝对路径规则保留驱动器 cwd 与 UNC 路径。
    fn wide(path: &Path) -> io::Result<Vec<u16>> {
        let path = std::path::absolute(path)?;
        let raw: Vec<u16> = path.as_os_str().encode_wide().collect();
        let mut encoded = Vec::new();
        use std::path::{Component, Prefix};
        let mut tail = raw.as_slice();
        if raw.len() >= 248
            && let Some(Component::Prefix(prefix)) = path.components().next()
        {
            match prefix.kind() {
                Prefix::Disk(_) => encoded.extend(r"\\?\".encode_utf16()),
                Prefix::UNC(_, _) => {
                    encoded.extend(r"\\?\UNC\".encode_utf16());
                    tail = &raw[2..];
                }
                _ => {}
            }
        }
        encoded.extend_from_slice(tail);
        encoded.push(0);
        Ok(encoded)
    }

    let source = wide(source)?;
    let destination = wide(destination)?;
    // SAFETY: 路径缓冲区终止于 NUL。flags=0 使宿主拒绝覆盖与跨卷复制。
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
/// 没有对应原子入口的目标返回宿主不支持。
fn rename_no_replace(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}

/// 识别宿主报告的跨卷或原子提交不支持。
pub(super) fn cannot_rename_atomically(error: &io::Error, overwrite: bool) -> bool {
    if matches!(
        error.kind(),
        io::ErrorKind::Unsupported | io::ErrorKind::CrossesDevices
    ) {
        return true;
    }
    #[cfg(unix)]
    if !overwrite && matches!(error.raw_os_error(), Some(libc::ENOSYS | libc::EINVAL)) {
        // 宿主内核或文件系统可能不支持原子禁止覆盖。
        return true;
    }
    #[cfg(windows)]
    if matches!(error.raw_os_error(), Some(1 | 17 | 50 | 120)) {
        // ERROR_INVALID_FUNCTION / NOT_SAME_DEVICE / NOT_SUPPORTED / CALL_NOT_IMPLEMENTED
        return true;
    }
    let _ = overwrite;
    false
}

/// 避免把暂存名称选为最终目录项的另一种拼写。
pub(super) fn matches_target_name(destination: &Path, name: &str) -> bool {
    let Some(target) = destination.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    #[cfg(windows)]
    let target = target
        .split(':')
        .next()
        .unwrap_or(target)
        .trim_end_matches([' ', '.']);
    target.eq_ignore_ascii_case(name)
}

/// 按宿主规则删除单个目录项，包括 Windows 的目录链接。
pub(super) fn remove(path: &Path, file_type: FileType) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        if file_type.is_symlink_dir() {
            return fs::remove_dir(path);
        }
    }
    if file_type.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    }
}
