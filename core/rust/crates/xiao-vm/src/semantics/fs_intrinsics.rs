//! fs 的 TAC 参数绑定、宿主安全点及 Runtime 调用。

use super::{BoundArgument, Carrier, Fault, Vm, VmEventSink, attach_fault_span};
use xiao_intrinsics::{IntrinsicDecl, VmBinding, fs_parameters};
use xiao_runtime::{
    RuntimeError, RuntimeValue,
    filesystem::{self, Failure, Operation},
};

impl<C: Carrier, S: VmEventSink> Vm<'_, C, S> {
    /// 值错误与取消沿 VM 已有的两条通道传播。
    pub(super) fn execute_fs(
        &mut self,
        declaration: &IntrinsicDecl,
        arguments: &[BoundArgument],
        span: xiao_ir::IrSpan,
    ) -> Result<Option<RuntimeValue>, Fault> {
        self.fs_value(declaration, arguments)
            .map(Some)
            .map_err(|fault| attach_fault_span(fault, span.start, span.end))
    }

    fn fs_value(
        &mut self,
        declaration: &IntrinsicDecl,
        arguments: &[BoundArgument],
    ) -> Result<RuntimeValue, Fault> {
        let mismatch = || Fault::Error(RuntimeError::type_mismatch("fs 必填形参", "调用形态"));
        let parameters = fs_parameters(declaration.id).ok_or_else(mismatch)?;
        if parameters.len() != arguments.len() {
            return Err(mismatch());
        }
        let mut values = [None; 3];
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
            VmBinding::FsWriteText => Operation::WriteText,
            VmBinding::FsListDir => Operation::ListDir,
            VmBinding::FsRemove => Operation::Remove,
            VmBinding::FsRename => Operation::Rename,
            VmBinding::FsMetadata => Operation::Metadata,
            _ => return Err(mismatch()),
        };
        filesystem::execute(
            operation,
            &values[..parameters.len()],
            self.capabilities,
            || self.host_checkpoint(),
        )
        .map_err(|error| match error {
            Failure::Runtime(error) => Fault::Error(error),
            Failure::Interrupted {
                mut control,
                cleanup,
            } => {
                if let Some(cleanup) = cleanup {
                    match &mut control {
                        Fault::Cancelled => self.cancelled_fs_cleanup = Some(cleanup),
                        Fault::ConcurrencyCancelled(primary) | Fault::Error(primary) => {
                            primary.push_suppressed(cleanup);
                        }
                        Fault::Fatal(_) => unreachable!("宿主安全点只交付运行控制错误"),
                    }
                }
                control
            }
        })
    }
}
