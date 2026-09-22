//! `-debug` 的 Runtime 侧交接：启动独立终端、握手并投递结构化事件。
//!
//! 这里故意只做进程边界编排。VM 继续产生 `VmEvent`，诊断进程只消费
//! `xiao_diagnostics::window::DiagnosticMessage`，因此终端 API 不会进入语言语义核。

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use xiao_diagnostics::window::{
    DIAGNOSTIC_CHANNEL_CODE, DIAGNOSTIC_PROTOCOL_VERSION, DIAGNOSTIC_START_CODE, DiagnosticEvent,
    DiagnosticFrameError, DiagnosticMessage, DiagnosticMetrics, default_level, object_payload,
    read_message, write_message,
};
use xiao_vm::VmEvent;

/// 终端候选的来源，用于稳定失败诊断和平台复现记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalCandidate {
    /// 可执行文件或平台入口。
    pub command: String,
    /// 固定候选来源名。
    pub source: &'static str,
    /// 传给入口的参数。
    pub args: Vec<String>,
}

/// 启动诊断窗口时的可选配置。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticOptions {
    /// 终端日志等级。
    pub terminal_level: Option<String>,
    /// 文件日志等级。
    pub file_level: Option<String>,
    /// 日志目录。
    pub log_dir: Option<PathBuf>,
    /// 总日志文件。
    pub log_file: Option<PathBuf>,
    /// 堆栈详细程度。
    pub stacktrace: Option<String>,
    /// 模块/源码聚焦规则的稳定摘要。
    pub focus: Vec<DiagnosticFocus>,
}

/// 一个文件聚焦规则；实际匹配在窗口外保持无副作用。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFocus {
    /// 模块匹配项。
    pub module: Option<String>,
    /// 源码匹配项。
    pub source: Option<String>,
    /// 输出文件。
    pub output: PathBuf,
    /// 文件等级。
    pub level: Option<String>,
    /// 是否同时镜像到总日志。
    pub mirror: bool,
}

/// 启动阶段失败的结构化错误。
#[derive(Debug)]
pub struct DiagnosticStartError {
    /// 稳定错误码。
    pub code: &'static str,
    /// 面向开发者的原因。
    pub message: String,
    /// 已尝试的终端候选。
    pub candidates: Vec<TerminalCandidateSummary>,
}

/// 失败诊断中的候选摘要。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalCandidateSummary {
    /// 候选命令。
    pub command: String,
    /// 来源。
    pub source: &'static str,
    /// 失败原因。
    pub reason: String,
}

impl std::fmt::Display for DiagnosticStartError {
    /// 渲染启动失败的稳定摘要。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DiagnosticStartError {}

/// 一次已经完成启动握手的诊断会话。
pub struct DiagnosticSession {
    stream: Option<TcpStream>,
    child: Option<Child>,
    started: Instant,
    module: String,
    source: Option<String>,
    log: Option<BufWriter<File>>,
    focus_logs: Vec<FocusLog>,
    terminal_level: EventLevel,
    file_level: EventLevel,
    metrics: DiagnosticMetrics,
    log_error: Option<String>,
}

/// 一个聚焦文件目标及其独立等级过滤器。
struct FocusLog {
    writer: BufWriter<File>,
    module: Option<String>,
    source: Option<String>,
    level: EventLevel,
    mirror: bool,
}

/// 诊断事件等级的确定性排序。
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum EventLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl EventLevel {
    /// 从配置文本解析等级，未知值按 info 处理。
    fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("info").to_ascii_lowercase().as_str() {
            "trace" => Self::Trace,
            "debug" => Self::Debug,
            "warn" | "warning" => Self::Warn,
            "error" => Self::Error,
            "fatal" => Self::Fatal,
            _ => Self::Info,
        }
    }

    /// 从事件等级文本解析等级。
    fn event(value: &str) -> Self {
        Self::parse(Some(value))
    }
}

impl DiagnosticSession {
    /// 创建终端并等待窗口进程发送就绪握手。
    pub fn start(
        module: impl Into<String>,
        source: Option<String>,
        options: &DiagnosticOptions,
    ) -> Result<Self, DiagnosticStartError> {
        let listener =
            TcpListener::bind(("127.0.0.1", 0)).map_err(|error| DiagnosticStartError {
                code: DIAGNOSTIC_START_CODE,
                message: format!("无法创建本机诊断端点：{error}"),
                candidates: Vec::new(),
            })?;
        listener
            .set_nonblocking(true)
            .map_err(|error| DiagnosticStartError {
                code: DIAGNOSTIC_START_CODE,
                message: format!("无法配置诊断端点：{error}"),
                candidates: Vec::new(),
            })?;
        let endpoint = listener
            .local_addr()
            .map_err(|error| DiagnosticStartError {
                code: DIAGNOSTIC_START_CODE,
                message: format!("无法读取诊断端点：{error}"),
                candidates: Vec::new(),
            })?;
        let token = session_token();
        let renderer = find_renderer().ok_or_else(|| DiagnosticStartError {
            code: DIAGNOSTIC_START_CODE,
            message: "找不到 xiao-diagnostics 独立进程；请构建它或设置 XIAO_DIAGNOSTICS_PATH"
                .to_owned(),
            candidates: Vec::new(),
        })?;
        let (mut child, summaries) =
            launch_terminal(&renderer, &endpoint.to_string(), &token, options).map_err(
                |summaries| DiagnosticStartError {
                    code: DIAGNOSTIC_START_CODE,
                    message: "所有终端候选均无法启动；未执行用户代码".to_owned(),
                    candidates: summaries,
                },
            )?;
        let (mut stream, _) = match accept_connection(&listener, &token, child.id()) {
            Ok(connection) => connection,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DiagnosticStartError {
                    code: DIAGNOSTIC_START_CODE,
                    message: error,
                    candidates: summaries,
                });
            }
        };
        if let Err(error) = write_message(
            &mut stream,
            &DiagnosticMessage::Ready {
                session_id: token.clone(),
            },
        ) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(DiagnosticStartError {
                code: DIAGNOSTIC_START_CODE,
                message: format!("诊断窗口握手确认失败：{error}"),
                candidates: summaries,
            });
        }
        let _ = stream.set_write_timeout(Some(Duration::from_millis(20)));
        let (log, log_error) = open_log(options);
        let (focus_logs, focus_errors) = open_focus_logs(options);
        Ok(Self {
            stream: Some(stream),
            child: Some(child),
            started: Instant::now(),
            module: module.into(),
            source,
            log,
            focus_logs,
            terminal_level: EventLevel::parse(options.terminal_level.as_deref()),
            file_level: EventLevel::parse(options.file_level.as_deref()),
            metrics: DiagnosticMetrics::default(),
            log_error: log_error.or(focus_errors),
        })
    }

    /// 发送一条 VM 事件；窗口中断时只降级为文件日志/丢弃。
    pub fn record(&mut self, event: &VmEvent) {
        let diagnostic = vm_event_to_diagnostic(
            event,
            self.started.elapsed(),
            &self.module,
            self.source.as_deref(),
        );
        self.metrics.error_count = self.metrics.error_count.saturating_add(u64::from(
            diagnostic.event_type == "error_raised" || diagnostic.event_type == "fatal_raised",
        ));
        self.metrics.hook_count = self.metrics.hook_count.saturating_add(u64::from(
            diagnostic.event_type.starts_with("handler_") || diagnostic.event_type.contains("hook"),
        ));
        let level = EventLevel::event(&diagnostic.level);
        let focused = self
            .focus_logs
            .iter()
            .any(|target| target_matches(target, &diagnostic));
        let mirrored = self
            .focus_logs
            .iter()
            .any(|target| target_matches(target, &diagnostic) && target.mirror);
        if (!focused || mirrored) && level >= self.file_level {
            self.write_log(&DiagnosticMessage::Event {
                event: diagnostic.clone(),
            });
        }
        self.write_focus_logs(&diagnostic, level);
        if level >= self.terminal_level {
            self.send(&DiagnosticMessage::Event { event: diagnostic });
        }
    }

    /// 发送最终指标并关闭诊断子进程。
    pub fn finish(mut self) {
        self.metrics.elapsed_ms = self.started.elapsed().as_millis();
        self.write_log(&DiagnosticMessage::Final {
            metrics: self.metrics.clone(),
        });
        self.send(&DiagnosticMessage::Final {
            metrics: self.metrics.clone(),
        });
        self.send(&DiagnosticMessage::Close {
            reason: "运行完成".to_owned(),
        });
        if let Some(mut child) = self.child.take() {
            let _ = child.try_wait();
        }
        let _ = self.log_error.take();
    }

    /// 返回诊断累计指标，供协议结果测试使用。
    #[must_use]
    pub fn metrics(&self) -> &DiagnosticMetrics {
        &self.metrics
    }

    /// 尝试向窗口写消息；断线只关闭当前输出目标。
    fn send(&mut self, message: &DiagnosticMessage) {
        let Some(stream) = self.stream.as_mut() else {
            return;
        };
        if let Err(error) = write_message(stream, message) {
            if !matches!(error, DiagnosticFrameError::Io(ref io_error) if io_error.kind() == io::ErrorKind::WouldBlock)
            {
                self.stream = None;
            }
        }
    }

    /// 尝试追加总 JSONL 日志。
    fn write_log(&mut self, message: &DiagnosticMessage) {
        let Some(log) = self.log.as_mut() else {
            return;
        };
        if let Ok(line) = serde_json::to_string(message) {
            if writeln!(log, "{line}").and_then(|_| log.flush()).is_err() {
                self.log = None;
            }
        }
    }

    /// 按聚焦规则追加各自的 JSONL 文件。
    fn write_focus_logs(&mut self, event: &DiagnosticEvent, level: EventLevel) {
        for target in &mut self.focus_logs {
            if !target_matches(target, event) || level < target.level {
                continue;
            }
            let message = DiagnosticMessage::Event {
                event: event.clone(),
            };
            if let Ok(line) = serde_json::to_string(&message) {
                if writeln!(target.writer, "{line}")
                    .and_then(|_| target.writer.flush())
                    .is_err()
                {
                    // 单个聚焦目标失效不应影响其他目标或用户程序。
                    continue;
                }
            }
        }
    }
}

/// 判断事件是否命中一个聚焦目标。
fn target_matches(target: &FocusLog, event: &DiagnosticEvent) -> bool {
    let module_matches = target
        .module
        .as_deref()
        .is_none_or(|module| event.module.as_deref() == Some(module));
    let source_matches = target
        .source
        .as_deref()
        .is_none_or(|source| event.source.as_deref() == Some(source));
    module_matches && source_matches
}

/// 返回固定顺序的平台终端候选。
#[must_use]
pub fn terminal_candidates(
    platform: &str,
    renderer: &Path,
    endpoint: &str,
    token: &str,
) -> Vec<TerminalCandidate> {
    let renderer = renderer.display().to_string();
    let args = |prefix: &[&str]| {
        prefix
            .iter()
            .map(|value| (*value).to_owned())
            .chain([
                renderer.clone(),
                "--connect".to_owned(),
                endpoint.to_owned(),
                "--token".to_owned(),
                token.to_owned(),
            ])
            .collect::<Vec<_>>()
    };
    match platform {
        "windows" => vec![
            TerminalCandidate {
                command: "wt.exe".to_owned(),
                source: "windows-terminal",
                args: args(&["new-tab", "--title", "Xiao diagnostics", "--"]),
            },
            TerminalCandidate {
                command: "cmd.exe".to_owned(),
                source: "cmd-start",
                args: vec![
                    "/c".to_owned(),
                    format!(
                        "start \"Xiao diagnostics\" {} --connect {} --token {}",
                        windows_quote(&renderer),
                        windows_quote(endpoint),
                        windows_quote(token)
                    ),
                ],
            },
            TerminalCandidate {
                command: "powershell.exe".to_owned(),
                source: "powershell-start-process",
                args: vec![
                    "-NoProfile".to_owned(),
                    "-Command".to_owned(),
                    format!(
                        "Start-Process -FilePath '{}' -ArgumentList '--connect','{}','--token','{}'",
                        renderer.replace('\'', "''"),
                        endpoint.replace('\'', "''"),
                        token.replace('\'', "''")
                    ),
                ],
            },
        ],
        "macos" => vec![
            TerminalCandidate {
                command: "osascript".to_owned(),
                source: "terminal-osascript",
                args: vec![
                    "-e".to_owned(),
                    format!(
                        "tell application \"Terminal\" to do script \"{} --connect {} --token {}\"",
                        shell_quote(&renderer),
                        shell_quote(endpoint),
                        shell_quote(token)
                    ),
                ],
            },
            TerminalCandidate {
                command: "open".to_owned(),
                source: "terminal-open",
                args: vec![
                    "-a".to_owned(),
                    "Terminal".to_owned(),
                    "--args".to_owned(),
                    renderer.clone(),
                    "--connect".to_owned(),
                    endpoint.to_owned(),
                    "--token".to_owned(),
                    token.to_owned(),
                ],
            },
        ],
        _ => vec![
            TerminalCandidate {
                command: "x-terminal-emulator".to_owned(),
                source: "debian-terminal-alternative",
                args: args(&["-e"]),
            },
            TerminalCandidate {
                command: "gnome-terminal".to_owned(),
                source: "gnome-terminal",
                args: args(&["--"]),
            },
            TerminalCandidate {
                command: "konsole".to_owned(),
                source: "konsole",
                args: args(&["-e"]),
            },
            TerminalCandidate {
                command: "xterm".to_owned(),
                source: "xterm",
                args: args(&["-e"]),
            },
        ],
    }
}

/// 按固定候选顺序启动终端进程。
fn launch_terminal(
    renderer: &Path,
    endpoint: &str,
    token: &str,
    _options: &DiagnosticOptions,
) -> Result<(Child, Vec<TerminalCandidateSummary>), Vec<TerminalCandidateSummary>> {
    let platform = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    let candidates = terminal_candidates(platform, renderer, endpoint, token);
    let mut summaries = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let result = Command::new(&candidate.command)
            .args(&candidate.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match result {
            Ok(child) => return Ok((child, summaries)),
            Err(error) => summaries.push(TerminalCandidateSummary {
                command: candidate.command,
                source: candidate.source,
                reason: error.to_string(),
            }),
        }
    }
    Err(summaries)
}

/// 等待窗口进程连接并完成令牌握手。
fn accept_connection(
    listener: &TcpListener,
    token: &str,
    child_id: u32,
) -> Result<(TcpStream, String), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match listener.accept() {
            Ok((mut stream, address)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .map_err(|error| error.to_string())?;
                let Some(DiagnosticMessage::Hello {
                    protocol_version,
                    token: received,
                    ..
                }) = read_message(&mut stream).map_err(|error| error.to_string())?
                else {
                    return Err("诊断进程未发送 hello 握手".to_owned());
                };
                if protocol_version != DIAGNOSTIC_PROTOCOL_VERSION || received != token {
                    return Err("诊断进程握手版本或令牌不匹配".to_owned());
                }
                return Ok((stream, address.to_string()));
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(format!("诊断进程 {child_id} 未在 10 秒内连接"));
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(format!("等待诊断进程连接失败：{error}")),
        }
    }
}

/// 按显式路径、同目录和开发树顺序寻找渲染器。
fn find_renderer() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("XIAO_DIAGNOSTICS_PATH") {
        let path = PathBuf::from(path);
        // 显式覆盖路径有最高优先级；配置错误时不静默回退到另一个渲染器。
        return path.is_file().then_some(path);
    }
    let current = std::env::current_exe().ok()?;
    let name = if cfg!(windows) {
        "xiao-diagnostics.exe"
    } else {
        "xiao-diagnostics"
    };
    let sibling = current.parent()?.join(name);
    if sibling.is_file() {
        return Some(sibling);
    }
    let mut candidates = Vec::new();
    if let Some(root) = find_repository_root(current.as_path()) {
        let suffix = if cfg!(windows) { ".exe" } else { "" };
        candidates.extend([
            root.join(format!("core/rust/target/debug/xiao-diagnostics{suffix}")),
            root.join(format!("core/rust/target/release/xiao-diagnostics{suffix}")),
        ]);
    }
    candidates.into_iter().find(|path| path.is_file())
}

/// 从可执行文件位置向上寻找仓库根。
fn find_repository_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.parent()?.to_path_buf();
    loop {
        if current.join("package.json").is_file() && current.join("core/rust/Cargo.toml").is_file()
        {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// 打开总日志目标；失败只返回可记录的原因。
fn open_log(options: &DiagnosticOptions) -> (Option<BufWriter<File>>, Option<String>) {
    let path = options.log_file.clone().or_else(|| {
        options
            .log_dir
            .as_ref()
            .map(|dir| dir.join("xiao-debug.jsonl"))
    });
    let Some(path) = path else {
        return (None, None);
    };
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        if let Err(error) = fs::create_dir_all(parent) {
            return (None, Some(error.to_string()));
        }
    }
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => (Some(BufWriter::new(file)), None),
        Err(error) => (None, Some(format!("{}: {error}", path.display()))),
    }
}

/// 打开聚焦日志目标并保留每个目标的独立失败。
fn open_focus_logs(options: &DiagnosticOptions) -> (Vec<FocusLog>, Option<String>) {
    let mut targets = Vec::new();
    let mut first_error = None;
    for focus in &options.focus {
        if focus.output.as_os_str().is_empty() {
            continue;
        }
        if let Some(parent) = focus
            .output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            if let Err(error) = fs::create_dir_all(parent) {
                first_error.get_or_insert_with(|| error.to_string());
                continue;
            }
        }
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&focus.output)
        {
            Ok(file) => targets.push(FocusLog {
                writer: BufWriter::new(file),
                module: focus.module.clone(),
                source: focus.source.clone(),
                level: EventLevel::parse(focus.level.as_deref()),
                mirror: focus.mirror,
            }),
            Err(error) => {
                first_error.get_or_insert_with(|| format!("{}: {error}", focus.output.display()));
            }
        }
    }
    (targets, first_error)
}

/// 生成一次性诊断会话令牌。
fn session_token() -> String {
    /// 进程内令牌序号，避免同一纳秒内碰撞。
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "{:x}-{}-{:x}",
        nanos,
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// 为终端脚本参数做最小单引号转义。
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// 为 `cmd /c start` 的单个命令行参数做双引号转义。
fn windows_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

/// 把既有 VM 事件补齐跨进程诊断字段。
fn vm_event_to_diagnostic(
    event: &VmEvent,
    elapsed: Duration,
    module: &str,
    source: Option<&str>,
) -> DiagnosticEvent {
    let (event_type, function, payload) = match event {
        VmEvent::ModuleLoaded { module } => ("module_loaded", None, json!({ "module": module })),
        VmEvent::FunctionEntered { function, depth } => (
            "function_entered",
            Some(function.clone()),
            json!({ "function": function, "depth": depth }),
        ),
        VmEvent::FunctionReturned { function, depth } => (
            "function_returned",
            Some(function.clone()),
            json!({ "function": function, "depth": depth }),
        ),
        VmEvent::ScopeEntered { scope } => ("scope_entered", None, json!({ "scope": scope })),
        VmEvent::ScopeExited { scope, exit } => (
            "scope_exited",
            None,
            json!({ "scope": scope, "exit": exit }),
        ),
        VmEvent::HandlerEntered { scope, handler } => (
            "handler_entered",
            None,
            json!({ "scope": scope, "handler": handler }),
        ),
        VmEvent::HandlerMatched {
            scope,
            handler,
            catch_type,
        } => (
            "handler_matched",
            None,
            json!({ "scope": scope, "handler": handler, "catch_type": catch_type }),
        ),
        VmEvent::HandlerUnmatched { scope } => {
            ("handler_unmatched", None, json!({ "scope": scope }))
        }
        VmEvent::ValueReleased {
            scope,
            exit,
            value,
            kind,
        } => (
            "value_released",
            None,
            json!({ "scope": scope, "exit": exit, "value": value, "kind": kind }),
        ),
        VmEvent::ErrorRaised { code, message_id } => (
            "error_raised",
            None,
            json!({ "code": code, "message_id": message_id }),
        ),
        VmEvent::FatalRaised { code } => ("fatal_raised", None, json!({ "code": code })),
        VmEvent::StackFrame {
            function,
            depth,
            return_to,
        } => (
            "stack_frame",
            Some(function.clone()),
            json!({ "function": function, "depth": depth, "return_to": return_to.map(|value| value.get()) }),
        ),
        VmEvent::BackendLocationMissing {
            function,
            block,
            instruction,
        } => (
            "backend_location_missing",
            Some(function.clone()),
            json!({ "function": function, "block": block, "instruction": instruction }),
        ),
        VmEvent::Metrics {
            instructions,
            max_call_depth,
            max_stack_depth,
            releases,
            spill_count,
            stack_map_entries,
            call_save_count,
            dropped_events,
        } => (
            "metrics",
            None,
            json!({ "instructions": instructions, "max_call_depth": max_call_depth, "max_stack_depth": max_stack_depth, "releases": releases, "spill_count": spill_count, "stack_map_entries": stack_map_entries, "call_save_count": call_save_count, "dropped_events": dropped_events }),
        ),
    };
    DiagnosticEvent {
        monotonic_ns: elapsed.as_nanos(),
        level: default_level(event_type).to_owned(),
        event_type: event_type.to_owned(),
        module: Some(module.to_owned()),
        source: source.map(str::to_owned),
        node: Some(event_type.to_owned()),
        function,
        error_id: None,
        payload: object_payload(payload),
    }
}

/// 将启动错误转换为协议响应附加字段。
#[must_use]
pub fn start_error_details(error: &DiagnosticStartError) -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("candidates".to_owned(), Value::Array(error.candidates.iter().map(|candidate| json!({ "command": candidate.command, "source": candidate.source, "reason": candidate.reason })).collect())),
        ("phase".to_owned(), Value::String("startup".to_owned())),
        ("channel_code".to_owned(), Value::String(DIAGNOSTIC_CHANNEL_CODE.to_owned())),
    ])
}

#[cfg(test)]
/// 覆盖候选顺序、事件字段和环境依赖入口。
mod tests {
    use super::*;

    #[test]
    /// Windows/Linux/macOS 候选顺序保持冻结。
    fn terminal_candidates_have_stable_platform_order() {
        let renderer = Path::new("xiao-diagnostics");
        let windows = terminal_candidates("windows", renderer, "127.0.0.1:1", "token");
        assert_eq!(
            windows
                .iter()
                .map(|candidate| candidate.source)
                .collect::<Vec<_>>(),
            vec!["windows-terminal", "cmd-start", "powershell-start-process"]
        );
        let linux = terminal_candidates("linux", renderer, "127.0.0.1:1", "token");
        assert_eq!(
            linux
                .iter()
                .map(|candidate| candidate.source)
                .collect::<Vec<_>>(),
            vec![
                "debian-terminal-alternative",
                "gnome-terminal",
                "konsole",
                "xterm"
            ]
        );
        let macos = terminal_candidates("macos", renderer, "127.0.0.1:1", "token");
        assert_eq!(
            macos
                .iter()
                .map(|candidate| candidate.source)
                .collect::<Vec<_>>(),
            vec!["terminal-osascript", "terminal-open"]
        );
    }

    #[test]
    /// VM 事件转换后包含诊断结构要求的身份字段。
    fn runtime_events_keep_required_structured_identity() {
        let event = vm_event_to_diagnostic(
            &VmEvent::ModuleLoaded {
                module: "app".to_owned(),
            },
            Duration::from_nanos(7),
            "app",
            Some("main.xiao"),
        );
        assert_eq!(event.monotonic_ns, 7);
        assert_eq!(event.event_type, "module_loaded");
        assert_eq!(event.module.as_deref(), Some("app"));
        assert_eq!(event.source.as_deref(), Some("main.xiao"));
        assert_eq!(event.node.as_deref(), Some("module_loaded"));
        assert!(event.error_id.is_none());
    }

    #[test]
    #[ignore = "需要真实终端模拟器；按 10D 规则显式跳过环境缺失"]
    /// 在具备终端环境时验证真实窗口握手。
    fn real_terminal_session_is_environment_gated() {
        let _ = DiagnosticSession::start(
            "test",
            Some("test.xiao".to_owned()),
            &DiagnosticOptions::default(),
        )
        .expect("已准备终端时应能启动诊断会话");
    }
}
