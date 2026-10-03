//! Xiao 缓存使用的唯一跨进程排他锁实现。
//!
//! 锁文件只保存进程号、创建时间和随机序列，不保存凭据、令牌或绝对路径。
//! 锁的 owner 内容在释放前会再次比对，避免误删后来取得同一路径的锁。

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// 锁等待上限，避免进程退出后无限阻塞。
pub const LOCK_WAIT: Duration = Duration::from_secs(30);
/// 无法解析 owner 时，只有超过该时长才允许回收未完成锁。
pub const INCOMPLETE_LOCK_GRACE: Duration = Duration::from_secs(60);

static NEXT_LOCK: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize, Eq, PartialEq, Serialize)]
struct LockOwner {
    pid: u32,
    created_at_ms: u64,
    nonce: u64,
}

/// 已取得的跨进程排他锁。
pub struct EntryLock {
    path: PathBuf,
    file: Option<File>,
    owner: Vec<u8>,
}

impl EntryLock {
    /// 创建父目录并取得锁；遇到活动锁会等待、回收可证明陈旧的锁或超时失败。
    pub fn acquire(path: &Path) -> Result<Self, LockError> {
        let parent = path
            .parent()
            .ok_or_else(|| LockError::new(path, "锁路径缺少父目录"))?;
        fs::create_dir_all(parent).map_err(|error| LockError::new(path, error))?;
        let started = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(mut file) => {
                    let owner = serde_json::to_vec(&LockOwner {
                        pid: std::process::id(),
                        created_at_ms: now_ms(),
                        nonce: NEXT_LOCK.fetch_add(1, Ordering::Relaxed),
                    })
                    .map_err(|error| LockError::new(path, error))?;
                    if let Err(error) = file.write_all(&owner).and_then(|()| file.sync_all()) {
                        drop(file);
                        let _ = fs::remove_file(path);
                        return Err(LockError::new(path, error));
                    }
                    return Ok(Self {
                        path: path.to_path_buf(),
                        file: Some(file),
                        owner,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if stale_owner(path)? {
                        continue;
                    }
                    if started.elapsed() >= LOCK_WAIT {
                        return Err(LockError::new(path, "等待缓存条目锁超时"));
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => return Err(LockError::new(path, error)),
            }
        }
    }
}

impl Drop for EntryLock {
    fn drop(&mut self) {
        self.file.take();
        if fs::read(&self.path).ok().as_deref() == Some(self.owner.as_slice()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// 锁路径及底层原因，不携带任何上层 crate 的诊断编号。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockError {
    path: PathBuf,
    reason: String,
}

impl LockError {
    fn new(path: &Path, reason: impl std::fmt::Display) -> Self {
        Self {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        }
    }

    /// 返回发生错误的锁路径。
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl std::fmt::Display for LockError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl std::error::Error for LockError {}

fn stale_owner(path: &Path) -> Result<bool, LockError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(LockError::new(path, error)),
    };
    let stale = if let Ok(owner) = serde_json::from_slice::<LockOwner>(&bytes) {
        owner.created_at_ms <= now_ms() && process_alive(owner.pid) == Some(false)
    } else {
        match fs::metadata(path).and_then(|metadata| metadata.modified()) {
            Ok(modified) => SystemTime::now()
                .duration_since(modified)
                .is_ok_and(|age| age >= INCOMPLETE_LOCK_GRACE),
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(LockError::new(path, error)),
        }
    };
    if stale && fs::read(path).ok().as_deref() == Some(bytes.as_slice()) {
        match fs::remove_file(path) {
            Ok(()) => return Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(true),
            Err(error) => return Err(LockError::new(path, error)),
        }
    }
    Ok(false)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(unix)]
fn process_alive(pid: u32) -> Option<bool> {
    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    let pid = i32::try_from(pid).ok()?;
    if unsafe { kill(pid, 0) } == 0 {
        return Some(true);
    }
    match io::Error::last_os_error().raw_os_error() {
        Some(3) => Some(false),
        Some(1) => Some(true),
        _ => None,
    }
}

#[cfg(windows)]
fn process_alive(pid: u32) -> Option<bool> {
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn GetExitCodeProcess(handle: *mut c_void, exit_code: *mut u32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    let handle = unsafe { OpenProcess(0x1000, 0, pid) };
    if handle.is_null() {
        return match io::Error::last_os_error().raw_os_error() {
            Some(87) => Some(false),
            _ => None,
        };
    }
    let mut exit_code = 0;
    let result = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
    unsafe { CloseHandle(handle) };
    (result != 0).then_some(exit_code == 259)
}

#[cfg(not(any(unix, windows)))]
fn process_alive(_pid: u32) -> Option<bool> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "xiao-lock-{label}-{}-{}",
            std::process::id(),
            NEXT_LOCK.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn owner_lock_is_removed_on_drop() {
        let path = temp_path("drop");
        {
            let _lock = EntryLock::acquire(&path).unwrap();
            assert!(path.is_file());
        }
        assert!(!path.exists());
    }

    #[test]
    fn owner_payload_contains_only_lock_identity_fields() {
        let path = temp_path("owner");
        let lock = EntryLock::acquire(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("pid"));
        assert!(text.contains("created_at_ms"));
        assert!(text.contains("nonce"));
        assert!(!text.contains(path.to_string_lossy().as_ref()));
        drop(lock);
        assert!(!path.exists());
    }
}
