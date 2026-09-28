//! 只追加本地化文本，不覆盖协议中原有的错误身份或预览消息。

use std::collections::BTreeMap;

use xiao_i18n::{Fallback, LocaleContext, MessageParam, MessageRenderer, builtin_renderer};

use super::message::{ProtocolErrorBody, ProtocolParam, ProtocolReport, ProtocolResponse};
use super::request::ProtocolError;
use super::run::protocol_error_response;

/// 旧客户端没有 locale 时沿用原响应；有效语言只在请求入口创建一次。
pub(super) fn with_run_locale(
    request_id: String,
    locale: Option<String>,
    execute: impl FnOnce() -> ProtocolResponse,
) -> ProtocolResponse {
    let Some(locale) = locale else {
        return execute();
    };
    let context = match LocaleContext::from_config(&locale) {
        Ok(context) => context,
        Err(_) => {
            return protocol_error_response(
                Some(request_id),
                &ProtocolError::request("locale", "只接受 zh、zh-CN、en 或 en-US"),
            );
        }
    };
    let mut response = execute();
    let renderer = builtin_renderer();
    match &mut response {
        ProtocolResponse::Result {
            diagnostics,
            report,
            ..
        } => {
            for diagnostic in diagnostics {
                diagnostic.text = Some(render_text(
                    &renderer,
                    &context,
                    &diagnostic.message_id,
                    &diagnostic.params,
                    &diagnostic.message,
                ));
            }
            if let Some(report) = report {
                localize_report(report, &renderer, &context);
            }
        }
        ProtocolResponse::Error { error, report, .. } => {
            localize_error(error, &renderer, &context);
            if let Some(report) = report {
                localize_report(report, &renderer, &context);
            }
        }
        _ => {}
    }
    response
}

fn localize_report(
    report: &mut ProtocolReport,
    renderer: &MessageRenderer,
    locale: &LocaleContext,
) {
    report.text = Some(render_text(
        renderer,
        locale,
        &report.message_id,
        &report.params,
        &report.message,
    ));
    if let Some(cause) = &mut report.cause {
        localize_report(cause, renderer, locale);
    }
    for suppressed in &mut report.suppressed {
        localize_report(suppressed, renderer, locale);
    }
}

fn localize_error(
    error: &mut ProtocolErrorBody,
    renderer: &MessageRenderer,
    locale: &LocaleContext,
) {
    error.text = Some(render_text(
        renderer,
        locale,
        &error.message_id,
        &BTreeMap::new(),
        &error.message,
    ));
}

fn render_text(
    renderer: &MessageRenderer,
    locale: &LocaleContext,
    id: &str,
    params: &BTreeMap<String, ProtocolParam>,
    original: &str,
) -> String {
    let params = params
        .iter()
        .map(|(key, value)| {
            let param = match value {
                ProtocolParam::Text(value) => MessageParam::Text(value.clone()),
                ProtocolParam::Integer(value) => MessageParam::Integer(*value),
                ProtocolParam::Boolean(value) => MessageParam::Boolean(*value),
            };
            (key.clone(), param)
        })
        .collect();
    let rendered = renderer.render(locale, id, &params);
    if locale.tag() == "zh-CN" && rendered.fallback == Fallback::Identity && !rendered.format_failed
    {
        original.to_owned()
    } else {
        rendered.text
    }
}
