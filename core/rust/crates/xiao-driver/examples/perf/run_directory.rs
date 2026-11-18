//! 每次诊断/测量独占新的产物目录；不能按PID复用历史原生可执行文件。

use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 原子创建新目录并保留现场；时钟或PID碰撞时递增后缀，不接纳已有目录。
pub(super) fn create(prefix: &str) -> io::Result<PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    create_in(&std::env::temp_dir(), prefix, nonce)
}

/// 只在新建成功后返回；AlreadyExists不会被当作本次产物目录。
fn create_in(parent: &Path, prefix: &str, mut nonce: u128) -> io::Result<PathBuf> {
    loop {
        let path = parent.join(format!("{prefix}-{}-{nonce}", std::process::id()));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                nonce = nonce.checked_add(1).ok_or(error)?;
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
/// 用旧缓存和同名候选固定复现碰撞，不依赖操作系统恰好复用PID。
mod tests {
    use super::{create, create_in};

    #[test]
    /// 历史程序必须保留；连续调用碰到相同候选也只能返回新的空目录。
    fn old_native_programs_are_never_reused_by_a_new_run() {
        let parent = create("xiao-b1-directory-test").unwrap();
        let old = parent.join(format!("cached-{}-0", std::process::id()));
        std::fs::create_dir(&old).unwrap();
        let old_program = old.join("math.abs-o3-3-1.exe");
        std::fs::write(&old_program, b"previous product").unwrap();

        let first = create_in(&parent, "cached", 0).unwrap();
        let second = create_in(&parent, "cached", 0).unwrap();
        assert_ne!(first, old);
        assert_ne!(second, old);
        assert_ne!(first, second);
        assert!(!first.join("math.abs-o3-3-1.exe").exists());
        assert_eq!(std::fs::read(&old_program).unwrap(), b"previous product");

        std::fs::remove_file(old_program).unwrap();
        for directory in [old, first, second] {
            std::fs::remove_dir(directory).unwrap();
        }
        std::fs::remove_dir(parent).unwrap();
    }
}
