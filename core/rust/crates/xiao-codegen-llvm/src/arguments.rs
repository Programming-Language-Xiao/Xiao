//! 固定参数调用的形参绑定计划；只排列已经按源码顺序求值的实参，不求值表达式。

use xiao_ir::{IrCallArgument, IrParameter};

/// 返回各形参对应的源码实参下标；保留原位置调用，命名调用只扩展无默认值的普通固定形参。
///
/// 调用方必须先按原实参顺序求值，再应用此排列。展开、未知/重复名称和缺参均返回 None，
/// 不在后端补默认值或构造可变参数，也不重写前端已冻结的调用规则。
pub(crate) fn fixed_argument_order(
    parameters: &[IrParameter],
    arguments: &[IrCallArgument],
) -> Option<Vec<usize>> {
    if arguments.len() != parameters.len() {
        return None;
    }
    if arguments
        .iter()
        .all(|argument| argument.kind == "positional" && argument.name.is_none())
    {
        return Some((0..arguments.len()).collect());
    }
    if parameters.iter().any(|parameter| {
        !matches!(
            parameter.kind.as_str(),
            "positional" | "positional_or_keyword"
        ) || parameter.default.is_some()
    }) {
        return None;
    }
    let mut order = vec![None; parameters.len()];
    let mut positional = 0;
    let mut saw_keyword = false;
    for (source, argument) in arguments.iter().enumerate() {
        let destination = match (argument.kind.as_str(), &argument.name) {
            ("positional", None) => {
                if saw_keyword {
                    return None;
                }
                let index = positional;
                positional += 1;
                index
            }
            ("keyword", Some(name)) => {
                saw_keyword = true;
                parameters.iter().position(|parameter| {
                    parameter.name.text == name.text && parameter.name.backticked == name.backticked
                })?
            }
            _ => return None,
        };
        let slot = order.get_mut(destination)?;
        if slot.replace(source).is_some() {
            return None;
        }
    }
    order.into_iter().collect()
}
