//! time 的 TAC 调用适配与 VM 运行控制。

use super::{BoundArgument, Carrier, Fault, Vm, VmEventSink, attach_fault_span};
use xiao_intrinsics::{IntrinsicDecl, VmBinding, time_parameters};
use xiao_runtime::{RuntimeError, RuntimeValue, time};

impl<C: Carrier, S: VmEventSink> Vm<'_, C, S> {
    pub(super) fn execute_time(
        &mut self,
        declaration: &IntrinsicDecl,
        arguments: &[BoundArgument],
        span: xiao_ir::IrSpan,
    ) -> Result<Option<RuntimeValue>, Fault> {
        self.time_value(declaration, arguments)
            .map(Some)
            .map_err(|fault| attach_fault_span(fault, span.start, span.end))
    }

    fn time_value(
        &mut self,
        declaration: &IntrinsicDecl,
        arguments: &[BoundArgument],
    ) -> Result<RuntimeValue, Fault> {
        let mismatch = || Fault::Error(RuntimeError::type_mismatch("time 必填形参", "调用形态"));
        let parameters = time_parameters(declaration.id).ok_or_else(mismatch)?;
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
        let number = if parameters.len() == 2 {
            Some(time::int_argument(values[0].ok_or_else(mismatch)?).map_err(Fault::Error)?)
        } else {
            None
        };
        let unit = time::unit_argument(values[parameters.len() - 1].ok_or_else(mismatch)?)
            .map_err(Fault::Error)?;
        if declaration.capability == xiao_intrinsics::Capability::Clock {
            time::require_clock(self.capabilities).map_err(Fault::Error)?;
        }
        let unit = unit
            .with_str(time::TimeUnit::parse)
            .and_then(|result| result)
            .map_err(Fault::Error)?;
        match declaration.vm_binding {
            VmBinding::TimeUtcNow => self
                .clock
                .utc_now(unit)
                .map(RuntimeValue::Int)
                .map_err(Fault::Error),
            VmBinding::TimeMonotonic => self
                .clock
                .monotonic(unit)
                .map(RuntimeValue::Int)
                .map_err(Fault::Error),
            VmBinding::TimeFormatUtc => time::format_utc(number.ok_or_else(mismatch)?, unit)
                .map(RuntimeValue::Str)
                .map_err(Fault::Error),
            VmBinding::TimeSleep => {
                let duration = time::sleep_duration(number.ok_or_else(mismatch)?, unit)
                    .map_err(Fault::Error)?;
                let clock = self.clock.clone();
                let stopped = clock
                    .sleep(duration, || {
                        if self
                            .cancellation
                            .as_ref()
                            .is_some_and(|source| source.cancellation_requested())
                        {
                            return Some(Fault::Cancelled);
                        }
                        if let Err(fault) = self.concurrent_checkpoint() {
                            return Some(fault);
                        }
                        if self
                            .cancellation
                            .as_ref()
                            .is_some_and(|source| source.deadline_reached())
                        {
                            return Some(Fault::Cancelled);
                        }
                        None
                    })
                    .map_err(Fault::Error)?;
                stopped.map_or(Ok(RuntimeValue::None), Err)
            }
            _ => Err(mismatch()),
        }
    }
}
