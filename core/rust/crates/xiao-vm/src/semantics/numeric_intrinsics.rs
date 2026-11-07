//! intrinsic 33 的单一 Runtime 数值入口；只绑定已求值实参并附加调用位置。

use super::{BoundArgument, Fault, attach_fault_span};
use xiao_diagnostics::XiaoError;
use xiao_runtime::RuntimeValue;
use xiao_source::SourceSpan;

/// 显式精度除法按形参绑定，保留源码求值顺序和可恢复错误身份。
pub(super) fn divide(
    arguments: &[BoundArgument],
    span: SourceSpan,
) -> Result<Option<RuntimeValue>, Fault> {
    let shape: Vec<_> = arguments
        .iter()
        .map(|argument| {
            (
                if argument.keyword.is_some() {
                    "keyword"
                } else {
                    "positional"
                },
                argument.keyword.as_deref(),
            )
        })
        .collect();
    let order = xiao_intrinsics::lfloat_divide_argument_order(&shape).ok_or_else(|| {
        attach_fault_span(
            Fault::Error(XiaoError::type_mismatch("lfloat_divide四参数", "调用形态")),
            span.start(),
            span.end(),
        )
    })?;
    xiao_runtime::lfloat_divide(
        &arguments[order[0]].value,
        &arguments[order[1]].value,
        &arguments[order[2]].value,
        &arguments[order[3]].value,
    )
    .map(Some)
    .map_err(|error| attach_fault_span(Fault::Error(error), span.start(), span.end()))
}
