//! `.xiaoc` 格式检查命令。

use std::env;
use std::fs;
use std::process::ExitCode;

use xiao_bytecode::inspect_xiaoc;

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("用法：xiaoc-check <module.xiaoc>");
        return ExitCode::from(2);
    };
    let path = path.to_string_lossy();
    let bytes = match fs::read(path.as_ref()) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("读取 {path} 失败：{error}");
            return ExitCode::from(2);
        }
    };
    match inspect_xiaoc(&bytes) {
        Ok(inspection) => {
            println!("合法 `.xiaoc`：模块 {}", inspection.module_id);
            println!("优化指纹：{}", inspection.optimization_fingerprint);
            println!("依赖摘要：{}", inspection.dependency_lock_digest);
            for (name, size) in inspection.sections {
                println!("分区 {name}: {size} 字节");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("`.xiaoc` 检查失败：{error}");
            ExitCode::from(1)
        }
    }
}
