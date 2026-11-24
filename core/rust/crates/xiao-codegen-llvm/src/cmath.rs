//! 静态/动态后端共用的 cmath 参数与符号元数据，不包含数学实现。
use xiao_intrinsics::{IntrinsicId, cmath_parameters};
use xiao_ir::IrCallArgument;

/// 按冻结形参返回源码实参排列；求值仍由调用方按源码顺序完成。
pub(crate) fn argument_order(id: u32, arguments: &[IrCallArgument]) -> Option<Vec<usize>> {
    let parameters = cmath_parameters(IntrinsicId::new(id)?)?;
    intrinsic_argument_order(parameters, arguments)
}

pub(crate) fn intrinsic_argument_order(
    parameters: &[(&str, xiao_intrinsics::ValueType)],
    arguments: &[IrCallArgument],
) -> Option<Vec<usize>> {
    if arguments.len() != parameters.len() {
        return None;
    }
    let mut order = vec![None; parameters.len()];
    let mut positional = 0;
    let mut named = false;
    for (index, argument) in arguments.iter().enumerate() {
        let slot = match (argument.kind.as_str(), &argument.name) {
            ("positional", None) if !named => {
                let slot = positional;
                positional += 1;
                slot
            }
            ("keyword", Some(name)) if !name.backticked => {
                named = true;
                parameters
                    .iter()
                    .position(|(expected, _)| *expected == name.text)?
            }
            _ => return None,
        };
        if order.get_mut(slot)?.replace(index).is_some() {
            return None;
        }
    }
    order.into_iter().collect()
}
