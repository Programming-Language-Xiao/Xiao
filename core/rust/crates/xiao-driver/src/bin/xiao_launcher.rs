//! 分发的 xiao 启动入口，先检查原生参数编码，再启动相邻 CLI。

use std::env;
use std::ffi::OsStr;
use std::process::{Command, ExitCode};

const INVALID_ARGUMENT: &str = "XIAO_LAUNCHER_INVALID_ARGUMENT";
/// 分发入口的实际路径，供文件关联和相邻工具发现使用。
const LAUNCHER_PATH: &str = "XIAO_LAUNCHER_PATH";

/// 校验读取过的宿主参数，记录第一个无法转为 UTF-8 的位置。
fn cli_command() -> std::io::Result<Command> {
    let launcher = env::current_exe()?;
    let cli = launcher.with_file_name(format!("xiao-cli{}", env::consts::EXE_SUFFIX));
    let mut command = Command::new(cli);
    command.env(LAUNCHER_PATH, launcher);
    command.env_remove(INVALID_ARGUMENT);
    let mut first_invalid = None;
    for (index, value) in env::args_os().skip(1).enumerate() {
        match value.into_string() {
            Ok(value) => {
                command.arg(value);
            }
            Err(_) => {
                first_invalid.get_or_insert(index);
                command.arg(OsStr::new(""));
            }
        }
    }
    if let Some(index) = first_invalid {
        command.env(INVALID_ARGUMENT, index.to_string());
    }
    Ok(command)
}

#[cfg(unix)]
/// 保留终端和退出状态，交给相邻 CLI 处理信号。
fn execute(mut command: Command) -> std::io::Result<ExitCode> {
    use std::os::unix::process::CommandExt;
    Err(command.exec())
}

#[cfg(windows)]
/// 转发子进程退出状态，让 CLI 在同一控制台处理 Ctrl+C。
fn execute(mut command: Command) -> std::io::Result<ExitCode> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetConsoleCP() -> u32;
        fn SetConsoleCtrlHandler(handler: extern "system" fn(u32) -> i32, add: i32) -> i32;
    }

    extern "system" fn handle_interrupt(event: u32) -> i32 {
        i32::from(matches!(event, 0 | 1))
    }

    // 只有与子进程共享控制台时，启动器才需要屏蔽自己的中断。
    if unsafe { GetConsoleCP() != 0 && SetConsoleCtrlHandler(handle_interrupt, 1) == 0 } {
        return Err(std::io::Error::last_os_error());
    }
    let status = command.status()?;
    if let Some(code) = status.code() {
        std::process::exit(code);
    }
    Ok(ExitCode::from(70))
}

/// 保留宿主进程诊断，避免将原始参数写入日志。
fn main() -> ExitCode {
    cli_command().and_then(execute).unwrap_or_else(|error| {
        eprintln!("X11-CLI-CORE-001: 无法启动相邻 CLI：{error}");
        ExitCode::from(70)
    })
}
