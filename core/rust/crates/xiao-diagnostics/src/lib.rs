//! Xiao 结构化诊断的最小基础层。
//!
//! 01 阶段先提供不可变的编译期诊断记录，让源码读取和词法器共享
//! 稳定的错误身份；第 07 阶段会在此基础上扩展 `XiaoError`、原因链、
//! 堆栈和运行时事件，但不会改变现有字段的机器语义。

use std::collections::BTreeMap;

use xiao_source::SourceSpan;

/// 诊断严重级别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Severity {
    /// 必须修复、会阻止当前阶段继续的错误。
    Error,
    /// 不阻止继续处理但需要用户注意的警告。
    Warning,
    /// 仅供开发者查看的信息。
    Info,
}

/// 未经本地化渲染的诊断插值参数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticParam {
    /// 类型名、路径、标识符或其他应保持原文的文本。
    Text(String),
    /// 数量、位置或其他整数语义值。
    Integer(i128),
    /// 布尔语义值，不使用本地化文本保存。
    Boolean(bool),
}

/// 按稳定参数名排列的诊断参数；不包含翻译后的句子。
pub type DiagnosticParams = BTreeMap<String, DiagnosticParam>;

/// 一条不可变的结构化诊断。
///
/// `code`、`message_id` 和 `params` 是机器接口；`message` 只是当前语言下的
/// 预览文本，后续国际化层可以根据同一身份和参数重新渲染。`span` 为空时
/// 表示诊断不对应具体源码区间。旧诊断可以暂时保留空参数，新诊断不得从
/// 已拼接的预览文本反向解析参数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// 稳定、机器可读的诊断编号。
    code: String,
    /// 语言目录使用的稳定消息键。
    message_id: String,
    /// 不依赖展示语言的插值参数。
    params: DiagnosticParams,
    /// 严重级别。
    severity: Severity,
    /// 相关源码区间；系统级诊断可以为空。
    span: Option<SourceSpan>,
    /// 当前语言的展示文本，不作为程序判断接口。
    message: String,
}

impl Diagnostic {
    /// 创建一条结构化诊断。
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        message_id: impl Into<String>,
        severity: Severity,
        span: Option<SourceSpan>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message_id: message_id.into(),
            params: DiagnosticParams::new(),
            severity,
            span,
            message: message.into(),
        }
    }

    /// 创建一条带源码区间的错误诊断。
    #[must_use]
    pub fn error_at(
        code: impl Into<String>,
        message_id: impl Into<String>,
        span: SourceSpan,
        message: impl Into<String>,
    ) -> Self {
        Self::new(code, message_id, Severity::Error, Some(span), message)
    }

    /// 为诊断附加结构化参数并返回新的完整记录。
    ///
    /// 参数名属于 `message_id` 的稳定签名；重复参数名采用最后一个值。
    #[must_use]
    pub fn with_params(
        mut self,
        params: impl IntoIterator<Item = (String, DiagnosticParam)>,
    ) -> Self {
        self.params.extend(params);
        self
    }

    /// 判断诊断是否为错误级别。
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }

    /// 返回稳定的机器诊断编号。
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// 返回可翻译的稳定消息键。
    #[must_use]
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    /// 返回原始插值参数，不解析或依赖当前展示文本。
    #[must_use]
    pub const fn params(&self) -> &DiagnosticParams {
        &self.params
    }

    /// 返回诊断严重级别。
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// 返回关联的源码区间。
    #[must_use]
    pub const fn span(&self) -> Option<SourceSpan> {
        self.span
    }

    /// 返回当前语言下的展示文本。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(test)]
/// 覆盖最小诊断结构字段和严重级别的单元测试。
mod tests {
    use super::{Diagnostic, DiagnosticParam, Severity};
    use xiao_source::SourceSpan;

    #[test]
    /// 确认错误构造器保留机器字段与源码区间。
    fn builds_error_with_span() {
        let span = SourceSpan::new(2, 3).expect("区间应有效");
        let diagnostic = Diagnostic::error_at("X01-LEX-001", "x01.lex.invalid", span, "bad");
        assert!(diagnostic.is_error());
        assert_eq!(diagnostic.severity(), Severity::Error);
        assert_eq!(diagnostic.span(), Some(span));
        assert!(diagnostic.params().is_empty());
    }

    #[test]
    /// 参数与预览译文分离，且参数名保持确定性顺序。
    fn preserves_language_independent_params() {
        let diagnostic = Diagnostic::new("E", "type.mismatch", Severity::Error, None, "预览")
            .with_params([
                (
                    "expected".to_owned(),
                    DiagnosticParam::Text("bool".to_owned()),
                ),
                ("count".to_owned(), DiagnosticParam::Integer(2)),
                ("enabled".to_owned(), DiagnosticParam::Boolean(false)),
            ]);
        assert_eq!(
            diagnostic.params().get("expected"),
            Some(&DiagnosticParam::Text("bool".to_owned()))
        );
        assert_eq!(
            diagnostic
                .params()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["count", "enabled", "expected"]
        );
        assert_eq!(diagnostic.code(), "E");
        assert_eq!(diagnostic.message_id(), "type.mismatch");
    }

    #[test]
    /// 确认非错误级别不会被误判为错误。
    fn distinguishes_non_error_levels() {
        let warning = Diagnostic::new("W", "warning", Severity::Warning, None, "warn");
        let info = Diagnostic::new("I", "info", Severity::Info, None, "info");
        assert!(!warning.is_error());
        assert!(!info.is_error());
    }
}
