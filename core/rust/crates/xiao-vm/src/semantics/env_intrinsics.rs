//! env 实参绑定与运行控制。

use std::sync::Arc;

use super::{BoundArgument, Carrier, Fault, Vm, VmEventSink, attach_fault_span};
use xiao_intrinsics::{IntrinsicDecl, VmBinding, env_parameters};
use xiao_runtime::{
    RuntimeError, RuntimeValue,
    environment::{Failure, Operation},
};

impl<C: Carrier, S: VmEventSink> Vm<'_, C, S> {
    pub(super) fn execute_env(
        &mut self,
        declaration: &IntrinsicDecl,
        arguments: &[BoundArgument],
        span: xiao_ir::IrSpan,
    ) -> Result<Option<RuntimeValue>, Fault> {
        self.env_value(declaration, arguments)
            .map(Some)
            .map_err(|fault| attach_fault_span(fault, span.start, span.end))
    }

    fn env_value(
        &mut self,
        declaration: &IntrinsicDecl,
        arguments: &[BoundArgument],
    ) -> Result<RuntimeValue, Fault> {
        let mismatch = || Fault::Error(RuntimeError::type_mismatch("env 必填形参", "调用形态"));
        let parameters = env_parameters(declaration.id).ok_or_else(mismatch)?;
        if parameters.len() != arguments.len() {
            return Err(mismatch());
        }
        let mut values = [None; 2];
        let mut named = false;
        for (position, argument) in arguments.iter().enumerate() {
            let slot = match &argument.keyword {
                Some(keyword) => {
                    named = true;
                    parameters
                        .iter()
                        .position(|(name, _)| name == keyword)
                        .ok_or_else(mismatch)?
                }
                None if !named => position,
                None => return Err(mismatch()),
            };
            if values[slot].replace(&argument.value).is_some() {
                return Err(mismatch());
            }
        }
        let values = values.map(|value| value.unwrap_or(&RuntimeValue::None));
        let operation = match declaration.vm_binding {
            VmBinding::EnvGet => Operation::Get,
            VmBinding::EnvSet => Operation::Set,
            VmBinding::EnvUnset => Operation::Unset,
            _ => return Err(mismatch()),
        };
        let snapshot = Arc::clone(&self.environment_snapshot);
        snapshot
            .execute(
                operation,
                &values[..parameters.len()],
                self.capabilities,
                || self.host_checkpoint(),
            )
            .map_err(|error| match error {
                Failure::Runtime(error) => Fault::Error(error),
                Failure::Interrupted(control) => control,
            })
    }
}
