//! B3-3 教程和能力边界经生产 VM 执行。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use xiao_driver::{DriverRequest, FrontendRequest, FrontendVmDriver};
use xiao_intrinsics::{CAPABILITY_FILESYSTEM, CAPABILITY_STDOUT};
use xiao_vm::VmEvent;

/// 冻结的 fs 教程。
const SOURCE: &str = include_str!("../../../../../tests/fixtures/docs/fs/tutorial.xiao");
/// 独立的逐字节黄金输出。
const GOLDEN: &[u8] = include_bytes!("../../../../../tests/fixtures/docs/fs/tutorial.stdout");
/// 测试子进程应使用的导入别名。
const CHILD_ALIAS: &str = "XIAO_B3_3_TEST_FS_ALIAS";
/// 从子进程单独执行的教程测试名称。
const TUTORIAL_TEST: &str = "fs_tutorial_matches_vm_golden";

#[cfg(feature = "fs-native-test")]
#[path = "support/b3_fs_native.rs"]
mod native;

/// 为教程提供独立的 cwd。
struct TaskDirectory(PathBuf);

impl TaskDirectory {
    /// 排他创建一份测试目录。
    fn new() -> Self {
        /// 在当前进程内区分测试实例。
        static NEXT: AtomicU32 = AtomicU32::new(0);
        loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("xiao-fs-tutorial-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("创建独占工作目录失败：{error}"),
            }
        }
    }
}

impl Drop for TaskDirectory {
    /// 回收教程夹具目录。
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

/// 通过生产 VM 执行并读取原始输出事件。
fn vm_stdout(source: &str, capabilities: u64) -> String {
    let request = DriverRequest::new(FrontendRequest::from_text(source))
        .with_capabilities(capabilities)
        .unwrap();
    let outcome = FrontendVmDriver::new().run(&request);
    assert!(outcome.is_success(), "{outcome:?}");
    outcome
        .as_executed()
        .unwrap()
        .events()
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
/// 两种导入路径均核对黄金字节和运行后的目录状态。
fn fs_tutorial_matches_vm_golden() {
    if let Ok(alias) = std::env::var(CHILD_ALIAS) {
        assert!(matches!(alias.as_str(), "std" | "stdlib"));
        let source = SOURCE.replacen("from std.fs ", &format!("from {alias}.fs "), 1);
        assert_eq!(
            vm_stdout(&source, CAPABILITY_STDOUT | CAPABILITY_FILESYSTEM).as_bytes(),
            GOLDEN
        );
        return;
    }
    let stdlib = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../stdlib");
    assert!(stdlib.join("fs.xiao").is_file());
    for alias in ["std", "stdlib"] {
        let task = TaskDirectory::new();
        let demo = task.0.join("demo");
        fs::create_dir(&demo).unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TUTORIAL_TEST, "--nocapture"])
            .env(CHILD_ALIAS, alias)
            .env("XIAO_STDLIB_ROOT", &stdlib)
            .current_dir(&task.0)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(stdout.contains("1 passed; 0 failed"), "{stdout}");
        assert_eq!(fs::read_dir(&demo).unwrap().count(), 0);
        println!("B3-3-VM module=fs alias={alias} exact=true demo_empty=true");
    }
}

#[test]
/// 无权限时在非法路径校验之前拒绝。
fn fs_permission_and_validation_precede_host_access() {
    let source = r#"from std.fs import list_dir
try
    list_dir("")
catch failure as Error
    print(error_code(failure))
"#;
    assert_eq!(vm_stdout(source, CAPABILITY_STDOUT), "X06-RUNTIME-026\n");
    assert_eq!(
        vm_stdout(source, CAPABILITY_STDOUT | CAPABILITY_FILESYSTEM),
        "X06-RUNTIME-012\n"
    );
}
