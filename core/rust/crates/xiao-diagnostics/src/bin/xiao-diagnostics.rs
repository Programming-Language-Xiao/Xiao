//! Xiao 独立诊断终端进程。
//!
//! 该进程只消费 Runtime 发送的结构化事件并绘制 TUI；它没有执行用户代码的入口。

use std::collections::VecDeque;
use std::env;
use std::io::{self, Write};
use std::net::TcpStream;
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
    if let Err(error) = run(arguments) {
        eprintln!("X11-DIAGNOSTIC-CHANNEL-001: {error}");
        std::process::exit(70);
    }
}

/// 诊断终端进程从命令行接收的连接参数。
struct Arguments {
    endpoint: String,
    token: String,
}

impl Arguments {
    /// 解析独立进程的连接端点和一次性令牌。
    fn parse<I>(mut values: I) -> Result<Self, String>
    where
        I: Iterator<Item = String>,
    {
        let mut endpoint = None;
        let mut token = None;
        while let Some(value) = values.next() {
            match value.as_str() {
                "--connect" => endpoint = values.next(),
                "--token" => token = values.next(),
                "--help" | "-h" => {
                    return Err("用法：xiao-diagnostics --connect <地址> --token <令牌>".to_owned());
                }
                other => return Err(format!("未知参数：{other}")),
            }
        }
        Ok(Self {
            endpoint: endpoint.ok_or_else(|| "缺少 --connect".to_owned())?,
            token: token.ok_or_else(|| "缺少 --token".to_owned())?,
        })
    }
}

/// 建立握手并持续渲染诊断消息。
fn run(arguments: Arguments) -> Result<(), DiagnosticFrameError> {
    let mut stream = TcpStream::connect(&arguments.endpoint)?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    write_message(
        &mut stream,
        &DiagnosticMessage::Hello {
            protocol_version: DIAGNOSTIC_PROTOCOL_VERSION,
            token: arguments.token,
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
