//! 只追加本地化文本，不覆盖协议中原有的错误身份或预览消息。

use std::collections::BTreeMap;

use xiao_i18n::{
    LocaleContext, MessageParam, MessageRenderer, builtin_renderer, render_with_original_message,
};

use super::message::{
    ProtocolDiagnostic, ProtocolErrorBody, ProtocolParam, ProtocolReport, ProtocolResponse,
};
use super::request::{ProtocolError, ProtocolRequest};
use super::run::protocol_error_response;

/// 旧客户端没有 locale 时沿用原响应；有效语言只在请求入口创建一次。
pub(super) fn with_locale(
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
            localize_diagnostics(diagnostics, &renderer, &context);
            if let Some(report) = report {
                localize_report(report, &renderer, &context);
            }
        }
        ProtocolResponse::TestResult { tests, .. } => {
            for test in tests {
                localize_diagnostics(&mut test.diagnostics, &renderer, &context);
                if let Some(report) = &mut test.report {
                    localize_report(report, &renderer, &context);
                }
                if let Some(error) = &mut test.error {
                    localize_error(error, &renderer, &context);
                }
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

pub(super) fn request_locale(request: &ProtocolRequest) -> Option<String> {
    match request {
        ProtocolRequest::Run { locale, .. }
        | ProtocolRequest::RunArchive { locale, .. }
        | ProtocolRequest::Test { locale, .. }
        | ProtocolRequest::Build { locale, .. }
        | ProtocolRequest::Environment { locale, .. }
        | ProtocolRequest::ReplPackages { locale, .. }
        | ProtocolRequest::Package { locale, .. } => locale.clone(),
        ProtocolRequest::Hello { .. }
        | ProtocolRequest::Cancel { .. }
        | ProtocolRequest::Shutdown { .. } => None,
    }
}

fn localize_diagnostics(
    diagnostics: &mut [ProtocolDiagnostic],
    renderer: &MessageRenderer,
    locale: &LocaleContext,
) {
    for diagnostic in diagnostics {
        diagnostic.text = Some(render_text(
            renderer,
            locale,
            &diagnostic.message_id,
            &diagnostic.params,
            &diagnostic.message,
        ));
    }
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
    if locale.tag() == "zh-CN" {
        return original.to_owned();
    }
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
    render_with_original_message(renderer, locale, id, &params, original).text
}
