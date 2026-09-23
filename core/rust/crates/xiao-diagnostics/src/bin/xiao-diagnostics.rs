//! Xiao 独立诊断终端进程。
//!
//! 该进程只消费 Runtime 发送的结构化事件并绘制 TUI；它没有执行用户代码的入口。

use std::collections::VecDeque;
use std::env;
use std::io::{self, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use xiao_diagnostics::window::{
    DIAGNOSTIC_PROTOCOL_VERSION, DiagnosticFrameError, DiagnosticMessage, DiagnosticMetrics,
    read_message, write_message,
};

/// 窗口滚动区域保留的最大事件行数。
const MAX_VISIBLE_EVENTS: usize = 80;

/// 诊断终端进程入口。
fn main() {
    let arguments = match Arguments::parse(env::args().skip(1)) {
        Ok(arguments) => arguments,
        Err(message) => {
            eprintln!("X11-DIAGNOSTIC-START-001: {message}");
            std::process::exit(70);
        }
    };
    if let Err(error) = if arguments.standalone {
        run_standalone(arguments.parent_pid, arguments.ready_file.as_deref())
    } else {
        run(arguments)
    } {
        eprintln!("X11-DIAGNOSTIC-CHANNEL-001: {error}");
        std::process::exit(70);
    }
}

/// 诊断终端进程从命令行接收的连接参数。
struct Arguments {
    endpoint: Option<String>,
    token: Option<String>,
    standalone: bool,
    parent_pid: Option<u32>,
    ready_file: Option<PathBuf>,
}

impl Arguments {
    /// 解析独立进程的连接端点和一次性令牌。
    fn parse<I>(mut values: I) -> Result<Self, String>
    where
        I: Iterator<Item = String>,
    {
        let mut endpoint = None;
        let mut token = None;
        let mut standalone = false;
        let mut parent_pid = None;
        let mut ready_file = None;
        while let Some(value) = values.next() {
            match value.as_str() {
                "--connect" => endpoint = values.next(),
                "--token" => token = values.next(),
                "--standalone" => standalone = true,
                "--parent-pid" => {
                    parent_pid = values
                        .next()
                        .ok_or_else(|| "缺少 --parent-pid 的值".to_owned())?
                        .parse::<u32>()
                        .ok();
                    if parent_pid.is_none() {
                        return Err("--parent-pid 必须是正整数".to_owned());
                    }
                }
                "--ready-file" => {
                    ready_file = Some(PathBuf::from(
                        values
                            .next()
                            .ok_or_else(|| "缺少 --ready-file 的值".to_owned())?,
                    ));
                }
                "--help" | "-h" => {
                    return Err("用法：xiao-diagnostics --connect <地址> --token <令牌>".to_owned());
                }
                other => return Err(format!("未知参数：{other}")),
            }
        }
        if !standalone && (endpoint.is_none() || token.is_none()) {
            return Err("缺少 --connect 或 --token".to_owned());
        }
        Ok(Self {
            endpoint,
            token,
            standalone,
            parent_pid,
            ready_file,
        })
    }
}

/// 建立握手并持续渲染诊断消息。
fn run(arguments: Arguments) -> Result<(), DiagnosticFrameError> {
    let mut stream = TcpStream::connect(arguments.endpoint.as_deref().unwrap_or_default())?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    write_message(
        &mut stream,
        &DiagnosticMessage::Hello {
            protocol_version: DIAGNOSTIC_PROTOCOL_VERSION,
            token: arguments.token.unwrap_or_default(),
            renderer: format!("xiao-diagnostics/{}", env!("CARGO_PKG_VERSION")),
        },
    )?;
    let Some(DiagnosticMessage::Ready { .. }) = read_message(&mut stream)? else {
        return Err(DiagnosticFrameError::Json(
            "Runtime 未确认诊断窗口就绪".to_owned(),
        ));
    };
    stream.set_read_timeout(None)?;
    let color = env::var_os("NO_COLOR").is_none()
        && env::var("TERM").map(|term| term != "dumb").unwrap_or(true);
    let mut events = VecDeque::with_capacity(MAX_VISIBLE_EVENTS);
    let mut metrics = DiagnosticMetrics::default();
    loop {
        match read_message(&mut stream)? {
            Some(DiagnosticMessage::Event { event }) => {
                if events.len() == MAX_VISIBLE_EVENTS {
                    events.pop_front();
                }
                events.push_back(format_event(&event, color));
                render(&events, &metrics, color)?;
            }
            Some(DiagnosticMessage::Final {
                metrics: final_metrics,
            }) => {
                metrics = final_metrics;
                render(&events, &metrics, color)?;
            }
            Some(DiagnosticMessage::Close { reason }) => {
                render(&events, &metrics, color)?;
                if !reason.is_empty() {
                    eprintln!("诊断会话结束：{reason}");
                }
                return Ok(());
            }
            Some(DiagnosticMessage::Ready { .. } | DiagnosticMessage::Hello { .. }) => {}
            None => return Ok(()),
        }
    }
}

/// 原生启动 shim 使用的无连接诊断窗口模式。
fn run_standalone(
    parent_pid: Option<u32>,
    ready_file: Option<&std::path::Path>,
) -> Result<(), DiagnosticFrameError> {
    println!("Xiao diagnostics");
    println!("原生调试产物已启动；诊断事件通道已就绪。");
    io::stdout().flush()?;
    if let Some(path) = ready_file {
        std::fs::write(path, b"ready\n")?;
    }
    let Some(parent_pid) = parent_pid else {
        thread::sleep(Duration::from_secs(1));
        return Ok(());
    };
    while process_is_alive(parent_pid) {
        thread::sleep(Duration::from_millis(200));
    }
    Ok(())
}

/// 判断生成该窗口的原生产物是否仍在运行。
#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    let result = unsafe { kill(pid as i32, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(1)
}

/// 判断生成该窗口的原生产物是否仍在运行。
#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    /// Windows 原生进程句柄。
    type Handle = *mut std::ffi::c_void;
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
        fn GetExitCodeProcess(handle: Handle, code: *mut u32) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
    }
    /// 只查询父进程存活状态所需的最小权限。
    const QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    /// `GetExitCodeProcess` 表示进程尚未退出的固定值。
    const STILL_ACTIVE: u32 = 259;
    let handle = unsafe { OpenProcess(QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return false;
    }
    let mut code = 0;
    let alive = unsafe { GetExitCodeProcess(handle, &mut code) != 0 && code == STILL_ACTIVE };
    unsafe { CloseHandle(handle) };
    alive
}

/// 未覆盖平台按“父进程仍在”保守处理，避免窗口在入口前立即消失。
#[cfg(not(any(unix, windows)))]
fn process_is_alive(_pid: u32) -> bool {
    true
}

/// 把结构化事件转换成单行终端摘要。
fn format_event(event: &xiao_diagnostics::window::DiagnosticEvent, color: bool) -> String {
    let level = if color {
        match event.level.as_str() {
            "error" => "\x1b[31mERROR\x1b[0m",
            "fatal" => "\x1b[35mFATAL\x1b[0m",
            "warn" => "\x1b[33mWARN \x1b[0m",
            "debug" => "\x1b[36mDEBUG\x1b[0m",
            "trace" => "\x1b[90mTRACE\x1b[0m",
            _ => "\x1b[32mINFO \x1b[0m",
        }
    } else {
        match event.level.as_str() {
            "error" => "ERROR",
            "fatal" => "FATAL",
            "warn" => "WARN ",
            "debug" => "DEBUG",
            "trace" => "TRACE",
            _ => "INFO ",
        }
    };
    let location = event.source.as_deref().unwrap_or("<runtime>");
    let module = event.module.as_deref().unwrap_or("main");
    format!(
        "{level} {:>10} {module} @ {location} :: {}",
        event.monotonic_ns, event.event_type
    )
}

/// 重绘滚动区域和固定状态栏。
fn render(events: &VecDeque<String>, metrics: &DiagnosticMetrics, color: bool) -> io::Result<()> {
    let mut output = String::new();
    output.push_str("\x1b[2J\x1b[H");
    output.push_str("Xiao diagnostics\n\n");
    for event in events {
        output.push_str(event);
        output.push('\n');
    }
    let status = format!(
        "运行 {:>6} ms | 内存 {:>8} / 峰值 {:>8} B | 错误 {:>3} | 断点 {:>3} | 钩子 {:>3}",
        metrics.elapsed_ms,
        metrics.current_memory_bytes,
        metrics.peak_memory_bytes,
        metrics.error_count,
        metrics.breakpoint_hits,
        metrics.hook_count,
    );
    if color {
        output.push_str("\x1b[7m");
    }
    output.push_str(&status);
    if color {
        output.push_str("\x1b[0m");
    }
    output.push('\n');
    let mut stdout = io::stdout().lock();
    stdout.write_all(output.as_bytes())?;
    stdout.flush()
}

#[cfg(test)]
/// 独立模式和连接模式的命令行边界测试。
mod tests {
    use super::Arguments;
    use std::path::PathBuf;

    #[test]
    /// 独立模式接受父进程编号并保留缺省就绪文件。
    fn standalone_arguments_accept_parent_pid() {
        let arguments = Arguments::parse(
            [
                "--standalone".to_owned(),
                "--parent-pid".to_owned(),
                "42".to_owned(),
            ]
            .into_iter(),
        )
        .expect("独立模式参数");
        assert!(arguments.standalone);
        assert_eq!(arguments.parent_pid, Some(42));
        assert_eq!(arguments.ready_file, None);
    }

    #[test]
    /// 手工预览独立模式时可以不提供父进程编号。
    fn standalone_arguments_allow_missing_parent_for_manual_preview() {
        let arguments =
            Arguments::parse(["--standalone".to_owned()].into_iter()).expect("独立预览参数");
        assert!(arguments.standalone);
        assert_eq!(arguments.parent_pid, None);
        assert_eq!(arguments.ready_file, None);
    }

    #[test]
    /// 非数字父进程编号必须在启动前拒绝。
    fn invalid_parent_pid_is_rejected() {
        let error = Arguments::parse(
            [
                "--standalone".to_owned(),
                "--parent-pid".to_owned(),
                "nope".to_owned(),
            ]
            .into_iter(),
        )
        .err()
        .expect("非法 pid");
        assert!(error.contains("正整数"));
    }

    #[test]
    /// 连接模式必须同时携带端点和一次性令牌。
    fn connected_mode_requires_endpoint_and_token() {
        let error = Arguments::parse(std::iter::empty())
            .err()
            .expect("缺少连接参数");
        assert!(error.contains("--connect"));
    }

    #[test]
    /// 原生启动桥可以传入一次性就绪标记路径。
    fn standalone_arguments_accept_ready_file() {
        let arguments = Arguments::parse(
            [
                "--standalone".to_owned(),
                "--ready-file".to_owned(),
                "ready.marker".to_owned(),
            ]
            .into_iter(),
        )
        .expect("独立模式就绪文件");
        assert_eq!(arguments.ready_file, Some(PathBuf::from("ready.marker")));
    }
}
