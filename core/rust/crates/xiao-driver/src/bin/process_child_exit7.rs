//! 进程 API 的确定性子程序；默认退出 7，测试控制模式用于验证继承和非阻塞分离。

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// 测试模式只由 Rust 夹具的隔离进程设置，不扩展 Xiao 的 spawn 签名。
fn main() {
    if let Some(path) = std::env::var_os("XIAO_PROCESS_TEST_INHERIT") {
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("读取继承 stdin");
        let cwd = std::env::current_dir().expect("继承 cwd");
        let marker = cwd.join("inherited-cwd").is_file();
        fs::write(path, format!("{marker}\n{input}")).expect("写入继承证据");
        println!("child-stdout");
        eprintln!("child-stderr");
    }
    if let Some(root) = std::env::var_os("XIAO_PROCESS_TEST_CONTROL") {
        let root = PathBuf::from(root);
        fs::write(root.join("started"), std::process::id().to_string()).expect("记录子进程启动");
        let deadline = Instant::now() + Duration::from_secs(10);
        while !root.join("release").is_file() {
            if Instant::now() >= deadline {
                std::process::exit(91);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        fs::write(root.join("completed"), "7").expect("记录子进程自然结束");
    }
    std::process::exit(7);
}
