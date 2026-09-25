//! E3B 缓存条目的跨进程排他创建与陈旧锁回收。

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::diagnostics::SOURCE_CACHE_IO_CODE;
use crate::source::SourceError;

/// 避免进程退出后或锁暂时被占用时无限阻塞。
const LOCK_WAIT: Duration = Duration::from_secs(30);
/// 未写完的锁仅在确实长期无人完成写入后回收。
const INCOMPLETE_LOCK_GRACE: Duration = Duration::from_secs(60);
static NEXT_LOCK: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize, Eq, PartialEq, Serialize)]
struct LockOwner {
    pid: u32,
    created_at_ms: u64,
    nonce: u64,
}

pub(crate) struct EntryLock {
    path: PathBuf,
    file: Option<File>,
    owner: Vec<u8>,
}

impl EntryLock {
    pub(crate) fn acquire(path: &Path) -> Result<Self, SourceError> {
        let parent = path
            .parent()
            .ok_or_else(|| lock_error(path, "锁路径缺少父目录"))?;
        fs::create_dir_all(parent).map_err(|error| lock_error(path, error))?;
        let started = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(mut file) => {
                    let owner = serde_json::to_vec(&LockOwner {
                        pid: std::process::id(),
                        created_at_ms: now_ms(),
                        nonce: NEXT_LOCK.fetch_add(1, Ordering::Relaxed),
                    })
                    .map_err(|error| lock_error(path, error))?;
                    if let Err(error) = file.write_all(&owner).and_then(|()| file.sync_all()) {
                        drop(file);
                        let _ = fs::remove_file(path);
                        return Err(lock_error(path, error));
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
                        return Err(lock_error(path, "等待缓存条目锁超时"));
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => return Err(lock_error(path, error)),
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

fn stale_owner(path: &Path) -> Result<bool, SourceError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(lock_error(path, error)),
    };
    let stale = if let Ok(owner) = serde_json::from_slice::<LockOwner>(&bytes) {
        owner.created_at_ms <= now_ms() && process_alive(owner.pid) == Some(false)
    } else {
        match fs::metadata(path).and_then(|metadata| metadata.modified()) {
            Ok(modified) => SystemTime::now()
                .duration_since(modified)
                .is_ok_and(|age| age >= INCOMPLETE_LOCK_GRACE),
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(lock_error(path, error)),
        }
    };
    if stale && fs::read(path).ok().as_deref() == Some(bytes.as_slice()) {
        match fs::remove_file(path) {
            Ok(()) => return Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(true),
            Err(error) => return Err(lock_error(path, error)),
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

fn lock_error(path: &Path, error: impl std::fmt::Display) -> SourceError {
    SourceError::new(
        SOURCE_CACHE_IO_CODE,
        format!("缓存锁 {}：{error}", path.display()),
    )
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
