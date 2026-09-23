//! 内部诊断、事件、指标和错误到协议消息的映射。

use std::collections::BTreeMap;

use serde_json::{Value, json};
use xiao_diagnostics::{
    Diagnostic, DiagnosticParam, FrameKind, ReportClass, ReportRecord, Severity, StackFrame,
};
use xiao_runtime::RuntimeValue;
use xiao_vm::VmEvent;

use super::message::*;
use super::request::ProtocolError;
use crate::run::ExitCode;

/// 将诊断参数映射到协议类型。
#[must_use]
pub fn protocol_param(value: &DiagnosticParam) -> ProtocolParam {
    match value {
        DiagnosticParam::Text(value) => ProtocolParam::Text(value.clone()),
        DiagnosticParam::Integer(value) => ProtocolParam::Integer(*value),
        DiagnosticParam::Boolean(value) => ProtocolParam::Boolean(*value),
    }
}

/// 转换一张诊断参数表，并保持稳定的键排序。
pub(super) fn protocol_params(
    values: &BTreeMap<String, DiagnosticParam>,
) -> BTreeMap<String, ProtocolParam> {
    values
        .iter()
        .map(|(key, value)| (key.clone(), protocol_param(value)))
        .collect()
}

/// 将内部源码区间转换为协议区间。
pub(super) fn protocol_span(span: Option<xiao_source::SourceSpan>) -> Option<ProtocolSpan> {
    span.map(|span| ProtocolSpan {
        start: span.start(),
        end: span.end(),
    })
}

/// 将前端诊断转换为协议诊断。
#[must_use]
pub fn protocol_diagnostic(diagnostic: &Diagnostic) -> ProtocolDiagnostic {
    ProtocolDiagnostic {
        code: diagnostic.code().to_owned(),
        message_id: diagnostic.message_id().to_owned(),
        severity: match diagnostic.severity() {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
        .to_owned(),
        span: protocol_span(diagnostic.span()),
        params: protocol_params(diagnostic.params()),
        message: diagnostic.message().to_owned(),
    }
}

/// 将统一调用栈帧转换为跨进程摘要。
pub(super) fn protocol_stack_frame(frame: &StackFrame) -> ProtocolStackFrame {
    ProtocolStackFrame {
        module: frame.module.clone(),
        function: frame.function.clone(),
        source: frame.source.clone(),
        span: protocol_span(frame.span),
        backend: ProtocolBackendLocation {
            bytecode_offset: frame.backend.bytecode_offset,
            native_address: frame.backend.native_address,
            inline_depth: frame.backend.inline_depth,
        },
        kind: match frame.kind {
            FrameKind::User => "user",
            FrameKind::Runtime => "runtime",
        }
        .to_owned(),
    }
}

/// 递归转换统一错误报告及其原因链。
pub(super) fn protocol_report(report: &ReportRecord) -> ProtocolReport {
    ProtocolReport {
        class: match report.class {
            ReportClass::Recoverable => "recoverable",
            ReportClass::Fatal => "fatal",
        }
        .to_owned(),
        code: report.code.clone(),
        error_id: report.error_id,
        message_id: report.message_id.clone(),
        params: protocol_params(&report.params),
        message: report.message.clone(),
        location: protocol_span(report.location),
        context: protocol_params(&report.context),
        stack: report.stack.iter().map(protocol_stack_frame).collect(),
        cause: report.cause.as_deref().map(protocol_report).map(Box::new),
        suppressed: report.suppressed.iter().map(protocol_report).collect(),
    }
}

/// 将 VM 指标和事件丢弃计数转换为协议结构。
pub(super) fn protocol_metrics(
    metrics: xiao_vm::VmMetrics,
    dropped_events: usize,
) -> ProtocolMetrics {
    ProtocolMetrics {
        instructions: metrics.instructions,
        max_call_depth: metrics.max_call_depth,
        max_stack_depth: metrics.max_stack_depth,
        releases: metrics.releases,
        spill_count: metrics.spill_count,
        stack_map_entries: metrics.stack_map_entries,
        call_save_count: metrics.call_save_count,
        dropped_events,
    }
}

/// 将 VM 事件转换为稳定类型名和机器字段。
pub(super) fn protocol_event(event: &VmEvent) -> ProtocolEvent {
    let (kind, data) = match event {
        VmEvent::ModuleLoaded { module } => ("module_loaded", json!({ "module": module })),
        VmEvent::FunctionEntered { function, depth } => (
            "function_entered",
            json!({ "function": function, "depth": depth }),
        ),
        VmEvent::FunctionReturned { function, depth } => (
            "function_returned",
            json!({ "function": function, "depth": depth }),
        ),
        VmEvent::ScopeEntered { scope } => ("scope_entered", json!({ "scope": scope })),
        VmEvent::ScopeExited { scope, exit } => {
            ("scope_exited", json!({ "scope": scope, "exit": exit }))
        }
        VmEvent::HandlerEntered { scope, handler } => (
            "handler_entered",
            json!({ "scope": scope, "handler": handler }),
        ),
        VmEvent::HandlerMatched {
            scope,
            handler,
            catch_type,
        } => (
            "handler_matched",
            json!({ "scope": scope, "handler": handler, "catch_type": catch_type }),
        ),
        VmEvent::HandlerUnmatched { scope } => ("handler_unmatched", json!({ "scope": scope })),
        VmEvent::ValueReleased {
            scope,
            exit,
            value,
            kind,
        } => (
            "value_released",
            json!({ "scope": scope, "exit": exit, "value": value, "kind": kind }),
        ),
        VmEvent::ErrorRaised { code, message_id } => (
            "error_raised",
            json!({ "code": code, "message_id": message_id }),
        ),
        VmEvent::FatalRaised { code } => ("fatal_raised", json!({ "code": code })),
        VmEvent::StackFrame {
            function,
            depth,
            return_to,
        } => (
            "stack_frame",
            json!({
                "function": function,
                "depth": depth,
                "return_to": return_to.map(|value| value.get()),
            }),
        ),
        VmEvent::BackendLocationMissing {
            function,
            block,
            instruction,
        } => (
            "backend_location_missing",
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
            json!({
                "instructions": instructions,
                "max_call_depth": max_call_depth,
                "max_stack_depth": max_stack_depth,
                "releases": releases,
                "spill_count": spill_count,
                "stack_map_entries": stack_map_entries,
                "call_save_count": call_save_count,
                "dropped_events": dropped_events,
            }),
        ),
    };
    let data = data
        .as_object()
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default();
    ProtocolEvent {
        kind: kind.to_owned(),
        data,
    }
}

/// 将入口值转换为不泄露 Runtime 句柄的稳定摘要。
pub(super) fn protocol_value(value: &RuntimeValue) -> ProtocolValue {
    let kind = value.type_name();
    let text = match value {
        RuntimeValue::Int(value) => value.to_string(),
        RuntimeValue::Sint(value) => value.to_string(),
        RuntimeValue::Lint(value) | RuntimeValue::Lfloat(value) => value.clone(),
        RuntimeValue::Float(value) => value.to_string(),
        RuntimeValue::Sfloat(value) => value.to_string(),
        RuntimeValue::Bool(value) => value.to_string(),
        RuntimeValue::Str(value) => value
            .to_string()
            .unwrap_or_else(|_| "<invalid-string-handle>".to_owned()),
        RuntimeValue::None => "none".to_owned(),
        RuntimeValue::Table(_)
        | RuntimeValue::TableDropView(_)
        | RuntimeValue::Array(_)
        | RuntimeValue::Tuple(_)
        | RuntimeValue::DictTable(_)
        | RuntimeValue::DictColumn(_)
        | RuntimeValue::Set(_)
        | RuntimeValue::Error(_) => format!("<{kind}>"),
    };
    ProtocolValue { kind, value: text }
}

/// 返回冻结退出码对应的稳定语义名称。
pub(super) fn exit_name(code: ExitCode) -> &'static str {
    match code {
        ExitCode::Success => "success",
        ExitCode::SourceRejected => "source_rejected",
        ExitCode::ArtifactRejected => "artifact_rejected",
        ExitCode::RuntimeError => "runtime_error",
        ExitCode::Fatal => "fatal",
    }
}

/// 创建带下一步建议和结构化字段的协议错误体。
pub(super) fn protocol_error_body(
    code: impl Into<String>,
    message_id: impl Into<String>,
    message: impl Into<String>,
    phase: Option<String>,
    next_step: Option<String>,
    details: BTreeMap<String, Value>,
) -> ProtocolErrorBody {
    ProtocolErrorBody {
        code: code.into(),
        message_id: message_id.into(),
        message: message.into(),
        phase,
        next_step,
        details,
    }
}

/// 将内部协议验证错误转换为跨语言错误体。
pub(super) fn protocol_error_from_error(error: &ProtocolError) -> ProtocolErrorBody {
    let mut details = BTreeMap::new();
    if let Some(field) = error.field() {
        details.insert("field".to_owned(), Value::String(field.to_owned()));
    }
    protocol_error_body(
        error.code(),
        "x11.protocol.request",
        error.message(),
        Some("protocol".to_owned()),
        Some("修正请求字段后重试".to_owned()),
        details,
    )
}
