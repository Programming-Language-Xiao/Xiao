//! 06-A 生命周期诊断编号与构造辅助。
//!
//! 诊断沿用 `xiao-diagnostics` 的稳定 `code`、`message_id`、参数和源码区间
//! 字段。文案只是中文预览，11C 会根据同一消息身份重新本地化。

use xiao_diagnostics::{Diagnostic, DiagnosticParam, Severity};
use xiao_source::SourceSpan;

use crate::graph::GraphError;
use crate::model::{EscapeReason, ValueId};

/// 强引用环诊断编号。
pub const STRONG_CYCLE_CODE: &str = "X06-LIFETIME-001";
/// 无效所有权边诊断编号。
pub const INVALID_EDGE_CODE: &str = "X06-LIFETIME-002";
/// 作用域或值不存在诊断编号。
pub const UNKNOWN_ID_CODE: &str = "X06-LIFETIME-003";
/// 生命周期事实相互冲突诊断编号。
pub const FACT_CONFLICT_CODE: &str = "X06-LIFETIME-004";
/// 无法静态确定、需要 Runtime 检查诊断编号。
pub const DYNAMIC_CHECK_CODE: &str = "X06-LIFETIME-005";

/// 为强引用环创建错误诊断。
#[must_use]
pub fn strong_cycle(span: Option<SourceSpan>, values: &[ValueId]) -> Diagnostic {
    let names = values
        .iter()
        .map(|id| id.get().to_string())
        .collect::<Vec<_>>()
        .join(",");
    Diagnostic::new(
        STRONG_CYCLE_CODE,
        "x06.lifetime.strong_cycle",
        Severity::Error,
        span,
        format!("检测到强引用环: [{names}]，请改用 Weak"),
    )
    .with_params([("values".to_owned(), DiagnosticParam::Text(names))])
}

/// 为无效图边创建错误诊断。
#[must_use]
pub fn invalid_edge(span: Option<SourceSpan>, error: &GraphError) -> Diagnostic {
    let reason = error.to_string();
    Diagnostic::new(
        INVALID_EDGE_CODE,
        "x06.lifetime.invalid_edge",
        Severity::Error,
        span,
        format!("所有权图边无效: {reason}"),
    )
    .with_params([("reason".to_owned(), DiagnosticParam::Text(reason))])
}

/// 为未知作用域/值创建错误诊断。
#[must_use]
pub fn unknown_id(span: Option<SourceSpan>, id: ValueId) -> Diagnostic {
    Diagnostic::new(
        UNKNOWN_ID_CODE,
        "x06.lifetime.unknown_value",
        Severity::Error,
        span,
        format!("找不到值 {} 的生命周期记录", id.get()),
    )
    .with_params([(
        "value".to_owned(),
        DiagnosticParam::Integer(i128::from(id.get())),
    )])
}

/// 为动态边界创建警告诊断。
#[must_use]
pub fn dynamic_check(span: SourceSpan, value: Option<ValueId>, reason: EscapeReason) -> Diagnostic {
    let value_text = value
        .map(|id| id.get().to_string())
        .unwrap_or_else(|| "expression".to_owned());
    let reason_text = match reason {
        EscapeReason::Returned => "函数返回值跨越生命周期边界",
        EscapeReason::CapturedByClosure => "值被闭包捕获",
        EscapeReason::StoredInLongerLivedContainer => "值被存入更长生命周期的容器",
        EscapeReason::CrossThread => "值可能跨线程传递",
        EscapeReason::DynamicValue => "值的具体类型需要运行时检查",
        EscapeReason::Unknown => "生命周期分析信息不完整",
    };
    Diagnostic::new(
        DYNAMIC_CHECK_CODE,
        "x06.lifetime.dynamic_check",
        Severity::Warning,
        Some(span),
        format!("值 {value_text} 的生命周期需要运行时检查：{reason_text}"),
    )
    .with_params([
        ("value".to_owned(), DiagnosticParam::Text(value_text)),
        (
            "reason".to_owned(),
            DiagnosticParam::Text(reason_text.to_owned()),
        ),
    ])
}
