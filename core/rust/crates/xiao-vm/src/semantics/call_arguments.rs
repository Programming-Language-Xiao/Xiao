//! TAC 运行期的位置、关键字和变长实参绑定。

use super::{
    BoundArgument, Carrier, Fault, Frame, RuntimeValue, TacFunction, TacProgram, Vm, VmEventSink,
};
use xiao_bytecode::TacArgument;

impl<C: Carrier, S: VmEventSink> Vm<'_, C, S> {
    /// 按 TAC 顺序求值实参，保留给绑定器使用的关键字。
    pub(super) fn bind(&mut self, arguments: &[TacArgument]) -> Result<Vec<BoundArgument>, Fault> {
        let mut bound = Vec::with_capacity(arguments.len());
        for argument in arguments {
            bound.push(BoundArgument {
                keyword: argument.name.clone(),
                value: self.read(argument.value)?,
            });
        }
        Ok(bound)
    }
}

/// 按调用签名把实参绑定到形参寄存器。
///
/// 位置实参按声明顺序填充；关键字实参按形参名重排——这正是 R1 冻结的
/// 「关键字参数由被调用方按名重排」，调用方只写实际提供的槽位。
pub(super) fn bind_arguments<C: Carrier>(
    function: &TacFunction,
    program: &TacProgram,
    frame: &mut Frame<C>,
    arguments: &[BoundArgument],
) -> Result<(), Fault> {
    use xiao_bytecode::sig::ParamKind;
    use xiao_runtime::{ArrayHandle, DictHandle, DictKind};

    let signature = function.signature.and_then(|id| program.signatures.get(id));
    let mut next_positional = 0_usize;
    let mut var_args = Vec::new();
    let mut keyword_args = Vec::new();
    for argument in arguments {
        let slot = match &argument.keyword {
            None => {
                if signature
                    .is_some_and(|signature| signature.var_args_slot == Some(next_positional))
                {
                    var_args.push(argument.value.clone());
                    continue;
                }
                let slot = next_positional;
                next_positional = next_positional.saturating_add(1);
                slot
            }
            Some(name) => {
                let slot = signature.and_then(|signature| {
                    signature
                        .parameter_names
                        .iter()
                        .enumerate()
                        .position(|(index, item)| {
                            item == name
                                && !matches!(
                                    signature.parameter_kinds[index],
                                    ParamKind::VarArgs | ParamKind::VarKeywords
                                )
                        })
                });
                if slot.is_none()
                    && signature.is_some_and(|signature| signature.kw_args_slot.is_some())
                {
                    keyword_args.push((name.clone(), argument.value.clone()));
                    continue;
                }
                slot.unwrap_or(usize::MAX)
            }
        };
        if let Some(register) = function.parameters.get(slot) {
            frame.carrier.write(*register, argument.value.clone());
        }
    }
    if let Some(signature) = signature {
        if let Some(register) = signature
            .var_args_slot
            .and_then(|slot| function.parameters.get(slot))
        {
            let array = ArrayHandle::new(var_args).map_err(Fault::Error)?;
            frame.carrier.write(*register, RuntimeValue::Array(array));
        }
        if let Some(register) = signature
            .kw_args_slot
            .and_then(|slot| function.parameters.get(slot))
        {
            let dictionary =
                DictHandle::new(DictKind::Table, keyword_args).map_err(Fault::Error)?;
            frame
                .carrier
                .write(*register, RuntimeValue::DictTable(dictionary));
        }
    }
    Ok(())
}
