//! Native diagnostic handshake and event forwarding ABI entry points.

use std::collections::BTreeMap;
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use xiao_diagnostics::window::{
    DIAGNOSTIC_PROTOCOL_VERSION, DiagnosticEvent, DiagnosticMessage, DiagnosticMetrics,
    default_level, read_message, write_message,
};
use xiao_runtime_abi::{XiaoAbiDiagnosticEvent, XiaoAbiStatus};

use super::{
    RuntimeDiagnosticSession, current_locale, diagnostic_session, diagnostic_session_token,
    diagnostic_start_failure, discard_diagnostic_session, restore_diagnostic_environment,
    send_diagnostic_message, utf8,
};

/// 为动态原生产物建立本机诊断监听端点；不查找目录，也不执行本地化。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_diagnostic_prepare() -> i32 {
    let listener = match TcpListener::bind(("127.0.0.1", 0)) {
        Ok(listener) => listener,
        Err(error) => return diagnostic_start_failure(format!("无法创建本机诊断端点：{error}")),
    };
    if let Err(error) = listener.set_nonblocking(true) {
        return diagnostic_start_failure(format!("无法配置本机诊断端点：{error}"));
    }
    let endpoint = match listener.local_addr() {
        Ok(endpoint) => endpoint.to_string(),
        Err(error) => return diagnostic_start_failure(format!("无法读取本机诊断端点：{error}")),
    };
    let token = diagnostic_session_token();
    let mut session = match diagnostic_session().lock() {
        Ok(session) => session,
        Err(_) => return diagnostic_start_failure("诊断会话状态已损坏"),
    };
    if session.is_some() {
        return diagnostic_start_failure("诊断会话已经存在");
    }
    let previous_endpoint = std::env::var_os("XIAO_DIAGNOSTICS_ENDPOINT");
    let previous_token = std::env::var_os("XIAO_DIAGNOSTICS_TOKEN");
    unsafe {
        std::env::set_var("XIAO_DIAGNOSTICS_ENDPOINT", &endpoint);
        std::env::set_var("XIAO_DIAGNOSTICS_TOKEN", &token);
    }
    *session = Some(RuntimeDiagnosticSession {
        listener: Some(listener),
        stream: None,
        token,
        previous_endpoint,
        previous_token,
        started: Instant::now(),
        metrics: DiagnosticMetrics::default(),
    });
    XiaoAbiStatus::Ok.code()
}

/// 等待诊断窗口完成令牌握手；用户代码只能在此成功后开始执行。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_diagnostic_ready() -> i32 {
    let mut session = match diagnostic_session().lock() {
        Ok(session) => session,
        Err(_) => return diagnostic_start_failure("诊断会话状态已损坏"),
    };
    let result = (|| -> Result<i32, String> {
        let Some(session) = session.as_mut() else {
            return Err("诊断会话尚未准备".to_owned());
        };
        let Some(listener) = session.listener.take() else {
            return if session.stream.is_some() {
                Ok(XiaoAbiStatus::Ok.code())
            } else {
                Err("诊断监听端点已经被消费".to_owned())
            };
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .map_err(|error| format!("无法配置诊断握手：{error}"))?;
                    stream
                        .set_write_timeout(Some(Duration::from_secs(2)))
                        .map_err(|error| format!("无法配置诊断握手：{error}"))?;
                    let message = match read_message(&mut stream) {
                        Ok(Some(message)) => message,
                        Ok(None) => return Err("诊断进程在握手前退出".to_owned()),
                        Err(error) => return Err(format!("诊断握手读取失败：{error}")),
                    };
                    let DiagnosticMessage::Hello {
                        protocol_version,
                        token,
                        ..
                    } = message
                    else {
                        return Err("诊断进程未发送 hello 握手".to_owned());
                    };
                    if protocol_version != DIAGNOSTIC_PROTOCOL_VERSION || token != session.token {
                        return Err("诊断进程握手版本或令牌不匹配".to_owned());
                    }
                    write_message(
                        &mut stream,
                        &DiagnosticMessage::Ready {
                            session_id: session.token.clone(),
                        },
                    )
                    .map_err(|error| format!("诊断窗口握手确认失败：{error}"))?;
                    stream
                        .set_write_timeout(Some(Duration::from_millis(20)))
                        .map_err(|error| format!("无法配置诊断事件写入：{error}"))?;
                    stream
                        .set_read_timeout(None)
                        .map_err(|error| format!("无法完成诊断握手：{error}"))?;
                    session.stream = Some(stream);
                    return Ok(XiaoAbiStatus::Ok.code());
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err("诊断进程未在 10 秒内连接".to_owned());
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(format!("等待诊断进程连接失败：{error}")),
            }
        }
    })();
    match result {
        Ok(status) => status,
        Err(message) => {
            discard_diagnostic_session(&mut session);
            diagnostic_start_failure(message)
        }
    }
}

/// 记录一个结构化诊断事件；机器字段原样透传，不查目录，也不执行本地化。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_diagnostic_event(event: *const XiaoAbiDiagnosticEvent) -> i32 {
    if event.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let event = unsafe { &*event };
    let event_type = match unsafe { utf8(event.event_type) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let code = match unsafe { utf8(event.code) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let message_id = match unsafe { utf8(event.message_id) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let mut session = match diagnostic_session().lock() {
        Ok(session) => session,
        Err(_) => return XiaoAbiStatus::RuntimeError.code(),
    };
    let Some(session) = session.as_mut() else {
        eprintln!("X11-DIAGNOSTIC-CHANNEL-001: 诊断事件没有已建立的会话");
        return XiaoAbiStatus::RuntimeError.code();
    };
    if session.stream.is_none() {
        eprintln!("X11-DIAGNOSTIC-CHANNEL-001: 诊断事件没有已建立的通道");
        return XiaoAbiStatus::RuntimeError.code();
    }
    let location = if event.location.present != 0 {
        json!({
            "start": event.location.span.start,
            "end": event.location.span.end,
        })
    } else {
        Value::Null
    };
    let mut payload = BTreeMap::new();
    payload.insert("code".to_owned(), Value::String(code.clone()));
    payload.insert("message_id".to_owned(), Value::String(message_id.clone()));
    payload.insert("location".to_owned(), location);
    let diagnostic = DiagnosticEvent {
        monotonic_ns: session
            .started
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64,
        level: default_level(&event_type).to_owned(),
        event_type: event_type.clone(),
        module: None,
        source: None,
        node: Some(event_type.clone()),
        function: None,
        error_id: None,
        locale: Some(current_locale()),
        message_id: (!message_id.is_empty()).then_some(message_id),
        params: BTreeMap::new(),
        text: None,
        payload,
    };
    session.metrics.error_count = session.metrics.error_count.saturating_add(u64::from(
        event_type.contains("error") || event_type.contains("fatal"),
    ));
    session.metrics.hook_count = session.metrics.hook_count.saturating_add(u64::from(
        event_type.starts_with("handler_") || event_type.contains("hook"),
    ));
    send_diagnostic_message(
        session,
        &DiagnosticMessage::Event {
            event: Box::new(diagnostic),
        },
    )
    .map_or_else(
        |error| {
            eprintln!("X11-DIAGNOSTIC-CHANNEL-001: {error}");
            XiaoAbiStatus::RuntimeError.code()
        },
        |_| XiaoAbiStatus::Ok.code(),
    )
}

/// 发送最终指标并关闭原生诊断会话；普通产物调用不到此入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_diagnostic_finish() {
    let Ok(mut session) = diagnostic_session().lock() else {
        return;
    };
    let Some(mut session) = session.take() else {
        return;
    };
    session.metrics.elapsed_ms = session
        .started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let metrics = session.metrics.clone();
    let _ = send_diagnostic_message(&mut session, &DiagnosticMessage::Final { metrics });
    let _ = send_diagnostic_message(
        &mut session,
        &DiagnosticMessage::Close {
            reason: "运行完成".to_owned(),
        },
    );
    restore_diagnostic_environment(&session);
}
