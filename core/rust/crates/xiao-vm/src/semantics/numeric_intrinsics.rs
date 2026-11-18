//! intrinsic 33～48 的共享 Runtime 数值入口；仅绑定已求值实参，不经过 C ABI。

use super::{BoundArgument, Fault, attach_fault_span};
use xiao_diagnostics::XiaoError;
use xiao_intrinsics::{IntrinsicDecl, VmBinding, cmath_parameters};
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

/// Cmath 的 Rust Result 统一转入 VM 可捕获错误，并附上原调用源码区间。
pub(super) fn cmath(
    declaration: &IntrinsicDecl,
    arguments: &[BoundArgument],
    span: xiao_ir::IrSpan,
) -> Result<Option<RuntimeValue>, Fault> {
    cmath_value(declaration, arguments)
        .map(Some)
        .map_err(|error| attach_fault_span(Fault::Error(error), span.start, span.end))
}

/// 按冻结形参绑定已求值实参；先校验完整形状/类型，再按 VmBinding 进入唯一数学内核。
fn cmath_value(
    declaration: &IntrinsicDecl,
    arguments: &[BoundArgument],
) -> xiao_runtime::RuntimeResult<RuntimeValue> {
    let mismatch = || XiaoError::type_mismatch("cmath 必填 float 参数", "调用形态");
    let parameters = cmath_parameters(declaration.id).ok_or_else(mismatch)?;
    if arguments.len() != parameters.len() {
        return Err(mismatch());
    }
    let mut slots = [None; 2];
    let mut saw_keyword = false;
    for (index, argument) in arguments.iter().enumerate() {
        let slot = if let Some(name) = &argument.keyword {
            saw_keyword = true;
            parameters
                .iter()
                .position(|(expected, _)| name == expected)
                .ok_or_else(mismatch)?
        } else if !saw_keyword {
            index
        } else {
            return Err(mismatch());
        };
        if slots[slot].replace(&argument.value).is_some() {
            return Err(mismatch());
        }
    }
    let mut values = [0.0; 2];
    for (index, value) in slots[..parameters.len()].iter().enumerate() {
        let value = value.ok_or_else(mismatch)?;
        let RuntimeValue::Float(number) = value else {
            return Err(XiaoError::type_mismatch("float", value.type_name()));
        };
        values[index] = *number;
    }
    let [a, b] = values;
    use xiao_runtime as runtime;
    let result = match declaration.vm_binding {
        VmBinding::CmathSqrt => runtime::cmath_sqrt(a),
        VmBinding::CmathFloor => runtime::cmath_floor(a),
        VmBinding::CmathCeil => runtime::cmath_ceil(a),
        VmBinding::CmathRound => runtime::cmath_round(a),
        VmBinding::CmathTrunc => runtime::cmath_trunc(a),
        VmBinding::CmathAbs => runtime::cmath_abs(a),
        VmBinding::CmathMin => runtime::cmath_min(a, b),
        VmBinding::CmathMax => runtime::cmath_max(a, b),
        VmBinding::CmathPow => runtime::cmath_pow(a, b),
        VmBinding::CmathSin => runtime::cmath_sin(a),
        VmBinding::CmathCos => runtime::cmath_cos(a),
        VmBinding::CmathTan => runtime::cmath_tan(a),
        VmBinding::CmathExp => runtime::cmath_exp(a),
        VmBinding::CmathLn => runtime::cmath_ln(a),
        VmBinding::CmathLog10 => runtime::cmath_log10(a),
        _ => return Err(mismatch()),
    }?;
    Ok(RuntimeValue::Float(result))
}
