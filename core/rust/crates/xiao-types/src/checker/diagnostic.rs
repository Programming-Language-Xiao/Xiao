//! 类型诊断与运行时检查标记的上报边界。
//!
//! 所有诊断保持原有编号、参数和去重顺序；本模块只负责把结果写入检查器状态。

use xiao_diagnostics::{Diagnostic, DiagnosticParam, Severity};
use xiao_source::SourceSpan;
use xiao_syntax::BinaryOperator;

use crate::diagnostics::*;
use crate::environment::EnvironmentError;
use crate::numeric::NumericError;
use crate::types::Type;
use crate::unify::UnifyError;

use super::{RuntimeCheck, RuntimeCheckKind, TypeChecker};

impl<'source> TypeChecker<'source> {
    /// 生成区分普通/反引号名称的环境键。
    pub(super) fn name_key(&self, name: xiao_syntax::Name) -> String {
        let prefix = if name.backticked {
            "backtick:"
        } else {
            "ascii:"
        };
        format!("{prefix}{}", name.unquoted_text(self.source))
    }

    /// 读取名称的原始源码文本用于诊断。
    pub(super) fn display_name(&self, name: xiao_syntax::Name) -> String {
        self.source.slice(name.span).to_owned()
    }

    /// 追加未定义名称诊断。
    pub(super) fn undefined_name(&mut self, name: xiao_syntax::Name) {
        self.type_error(
            UNDEFINED_NAME_CODE,
            "x02.type.undefined_name",
            name.span,
            format!("未定义名称 {}", self.display_name(name)),
        );
    }

    /// 将环境操作错误映射为稳定类型诊断。
    pub(super) fn environment_error(&mut self, span: SourceSpan, error: EnvironmentError) {
        let (code, message_id) = match error {
            EnvironmentError::Duplicate(_) => {
                (DUPLICATE_DECLARATION_CODE, "x02.type.duplicate_declaration")
            }
            EnvironmentError::Unknown(_) => (UNDEFINED_NAME_CODE, "x02.type.undefined_name"),
            EnvironmentError::Immutable(_) => {
                (ASSIGNMENT_TYPE_MISMATCH_CODE, "x02.type.assign_immutable")
            }
        };
        self.type_error(code, message_id, span, error.to_string());
    }

    /// 将数值错误按操作数/算术类别映射为稳定诊断。
    pub(super) fn numeric_error(
        &mut self,
        span: SourceSpan,
        operator: BinaryOperator,
        error: NumericError,
    ) {
        let code = if matches!(&error, NumericError::InvalidOperands { .. }) {
            INVALID_OPERANDS_CODE
        } else {
            ARITHMETIC_ERROR_CODE
        };
        self.type_error(
            code,
            "x02.type.arithmetic_error",
            span,
            format!("运算 {}：{}", operator.as_str(), error),
        );
    }

    /// 将 HM 统一失败映射为带源码区间的诊断。
    pub(super) fn unification_error(&mut self, span: SourceSpan, error: UnifyError) {
        self.type_error(
            UNIFICATION_ERROR_CODE,
            "x02.type.unification_error",
            span,
            error.to_string(),
        );
    }

    /// 追加一条错误级别的结构化类型诊断。
    pub(super) fn type_error(
        &mut self,
        code: &'static str,
        message_id: &'static str,
        span: SourceSpan,
        message: String,
    ) {
        self.type_error_with_params(code, message_id, span, message, []);
    }

    /// 保存新增诊断的稳定参数，展示文本仅作为当前阶段的预览。
    pub(super) fn type_error_with_params(
        &mut self,
        code: &'static str,
        message_id: &'static str,
        span: SourceSpan,
        message: String,
        params: impl IntoIterator<Item = (String, DiagnosticParam)>,
    ) {
        // 同一编号、同一源码位置的诊断是重复上报：两条检查路径报告了同一件事，
        // 用户会在同一行看到两遍相同的错误。同一条错误只保留首次。
        if self
            .diagnostics
            .iter()
            .any(|existing| existing.code() == code && existing.span() == Some(span))
        {
            return;
        }
        self.diagnostics.push(
            Diagnostic::new(code, message_id, Severity::Error, Some(span), message)
                .with_params(params),
        );
    }

    /// 追加一个去重前的运行时检查标记。
    pub(super) fn push_runtime_check(&mut self, span: SourceSpan, kind: RuntimeCheckKind) {
        self.runtime_checks.push(RuntimeCheck {
            span,
            kind,
            expected: None,
        });
    }

    /// 追加带声明类型边界的运行时检查。
    pub(super) fn push_runtime_check_with_expected(
        &mut self,
        span: SourceSpan,
        kind: RuntimeCheckKind,
        expected: Type,
    ) {
        self.runtime_checks.push(RuntimeCheck {
            span,
            kind,
            expected: Some(expected),
        });
    }

    /// 判断指定诊断索引之后是否已经出现错误；错误恢复表达式不再
    /// 额外注入一个看似可执行的运行时检查。
    pub(super) fn has_errors_since(&self, start: usize) -> bool {
        self.diagnostics
            .get(start..)
            .is_some_and(|diagnostics| diagnostics.iter().any(Diagnostic::is_error))
    }
}
