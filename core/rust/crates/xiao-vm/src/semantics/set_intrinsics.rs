//! 集合变更 intrinsic 的 VM 绑定。

use super::{BoundArgument, Fault};
use xiao_diagnostics::XiaoError;
use xiao_intrinsics::{Arity, IntrinsicDecl, VmBinding};
use xiao_runtime::RuntimeValue;

/// 执行 `set_add`/`set_remove` 的二参数契约。
pub(super) fn execute(
    declaration: &IntrinsicDecl,
    arguments: &[BoundArgument],
) -> Result<Option<RuntimeValue>, Fault> {
    if declaration.signature.arity != Arity::Two
        || arguments.len() != 2
        || arguments.iter().any(|argument| argument.keyword.is_some())
    {
        return Err(Fault::Error(XiaoError::invalid_value(
            "集合变更 intrinsic 必须接收两个位置参数",
        )));
    }
    let RuntimeValue::Set(set) = &arguments[0].value else {
        return Err(Fault::Error(XiaoError::type_mismatch(
            "set",
            arguments[0].value.type_name(),
        )));
    };
    let element = &arguments[1].value;
    if declaration.vm_binding == VmBinding::SetAdd {
        set.insert(element).map_err(Fault::Error)?;
        Ok(Some(RuntimeValue::None))
    } else {
        Ok(Some(RuntimeValue::Bool(
            set.remove(element).map_err(Fault::Error)?,
        )))
    }
}
