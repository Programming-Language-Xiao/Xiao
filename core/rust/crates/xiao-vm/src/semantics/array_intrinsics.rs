//! 数组追加 intrinsic 的 VM 绑定。

use super::{BoundArgument, Fault};
use xiao_diagnostics::XiaoError;
use xiao_intrinsics::{Arity, IntrinsicDecl};
use xiao_runtime::RuntimeValue;

/// 执行 `array_push` 的二参数契约。
pub(super) fn execute(
    declaration: &IntrinsicDecl,
    arguments: &[BoundArgument],
) -> Result<Option<RuntimeValue>, Fault> {
    if declaration.signature.arity != Arity::Two
        || arguments.len() != 2
        || arguments.iter().any(|argument| argument.keyword.is_some())
    {
        return Err(Fault::Error(XiaoError::invalid_value(
            "数组追加 intrinsic 必须接收两个位置参数",
        )));
    }
    let RuntimeValue::Array(array) = &arguments[0].value else {
        return Err(Fault::Error(XiaoError::type_mismatch(
            "array",
            arguments[0].value.type_name(),
        )));
    };
    array
        .push(arguments[1].value.clone())
        .map_err(Fault::Error)?;
    Ok(Some(RuntimeValue::None))
}
