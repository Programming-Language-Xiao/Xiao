//! VM 日志实参绑定与运行 stdout 适配。

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use super::{BoundArgument, Carrier, Fault, Vm, VmEvent, VmEventSink, attach_fault_span};
use crate::CancellationSource;
use xiao_intrinsics::{IntrinsicDecl, log_parameters};
use xiao_runtime::{
    RuntimeError, RuntimeValue,
    log::{Failure, LogOutput, LogState},
    time::TimeContext,
};

/// 由根 VM、子任务和释放钩子共享的运行日志状态。
pub(super) struct RunLog {
    state: LogState<Output>,
    clock: Arc<TimeContext>,
}

/// 可交给同一运行子任务的嵌入式输出端。
type HostOutput = Box<dyn LogOutput + Send>;

/// 生产入口使用内存 stdout 捕获，嵌入入口可安装自己的同步输出端。
enum Output {
    Capture(LogCapture),
    Host(HostOutput),
}

impl Output {
    /// 将输出操作交给当前运行的实际目标。
    fn target(&mut self) -> &mut dyn LogOutput {
        match self {
            Self::Capture(capture) => capture,
            Self::Host(target) => target.as_mut(),
        }
    }
}

impl Write for Output {
    /// 借读待写字节并报告目标已消费的长度。
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.target().write(bytes)
    }

    /// 同步完成目标的逐记录刷新。
    fn flush(&mut self) -> io::Result<()> {
        self.target().flush()
    }
}

impl LogOutput for Output {
    /// 在开始交付前调用实际输出端的接纳检查。
    fn admit(&mut self, bytes: usize) -> io::Result<()> {
        self.target().admit(bytes)
    }
}

#[derive(Default)]
/// 内存 stdout 的记录序列与运行结束标记。
struct Captured {
    records: Vec<String>,
    closed: bool,
}

/// 捕获发生在 write 内，成功写出的记录可在运行结果中交付。
#[derive(Clone, Default)]
pub(super) struct LogCapture(Arc<Mutex<Captured>>);

impl LogCapture {
    /// 顺序取出当前记录，根运行结束时一并关闭捕获端。
    fn take_records(&self, close: bool) -> Vec<String> {
        let mut capture = self.0.lock().expect("VM 日志捕获锁未中毒");
        capture.closed |= close;
        std::mem::take(&mut capture.records)
    }
}

impl Write for LogCapture {
    /// 同步写入一条完整记录，报告内存捕获端的关闭及分配失败。
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let text = std::str::from_utf8(bytes).map_err(|_| io::ErrorKind::InvalidData)?;
        let mut record = String::new();
        record
            .try_reserve_exact(text.len())
            .map_err(|_| io::ErrorKind::OutOfMemory)?;
        record.push_str(text);
        let mut capture = self.0.lock().expect("VM 日志捕获锁未中毒");
        if capture.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        capture
            .records
            .try_reserve(1)
            .map_err(|_| io::ErrorKind::OutOfMemory)?;
        capture.records.push(record);
        Ok(bytes.len())
    }

    /// 每次写入均已在内存捕获端交付。
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl LogOutput for LogCapture {
    /// 已关闭的运行停止接纳新日志。
    fn admit(&mut self, _: usize) -> io::Result<()> {
        if self.0.lock().expect("VM 日志捕获锁未中毒").closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        Ok(())
    }
}

impl<C: Carrier, S: VmEventSink> Vm<'_, C, S> {
    /// 为下一次运行安装可报告接纳、写入和刷新失败的日志 stdout。
    pub fn set_log_output(&mut self, output: impl LogOutput + Send + 'static) {
        self.next_log_output = Some(Box::new(output));
    }

    /// 根 VM 建立本次执行的日志额度和 stdout。
    pub(super) fn begin_log_output(&mut self) {
        let output = if let Some(output) = self.next_log_output.take() {
            Output::Host(output)
        } else {
            let capture = LogCapture::default();
            self.log_capture = Some(capture.clone());
            Output::Capture(capture)
        };
        self.set_shared_log(Some(Arc::new(RunLog {
            state: LogState::new(output),
            clock: self.clock.clone(),
        })));
    }

    /// 安装当前运行的日志，并让表析构帧借用同一状态。
    pub(super) fn set_shared_log(&mut self, log: Option<Arc<RunLog>>) {
        *self.session.tables.log.borrow_mut() =
            log.as_ref().map(Arc::downgrade).unwrap_or_default();
        self.log = log;
    }

    /// 向运行结果交付剩余记录，释放根 VM 持有的运行日志。
    pub(super) fn finish_log_output(&mut self) {
        self.publish_log_output(true);
        self.log_capture = None;
        self.set_shared_log(None);
    }

    /// 根 VM 把已捕获的记录交给结果接收器。
    fn publish_log_output(&mut self, close: bool) {
        if let Some(capture) = &self.log_capture {
            for record in capture.take_records(close) {
                self.sink.record_log_output(record);
            }
        }
    }

    /// 输出事件沿用既有顺序，根 VM 先交付此前已捕获的日志。
    pub(super) fn record_output_event(&mut self, event: VmEvent) {
        if matches!(event, VmEvent::IntrinsicOutput { .. }) {
            self.publish_log_output(false);
        }
        self.sink.record(event);
    }

    /// 为可恢复的日志错误补齐调用区间，无值返回。
    pub(super) fn execute_log(
        &mut self,
        declaration: &IntrinsicDecl,
        arguments: &[BoundArgument],
        span: xiao_ir::IrSpan,
    ) -> Result<Option<RuntimeValue>, Fault> {
        let result = self.write_log(declaration, arguments);
        self.publish_log_output(false);
        result
            .map(|()| None)
            .map_err(|fault| attach_fault_span(fault, span.start, span.end))
    }

    /// 按位置和关键字绑定借读实参，保留既有运行控制通道。
    fn write_log(
        &mut self,
        declaration: &IntrinsicDecl,
        arguments: &[BoundArgument],
    ) -> Result<(), Fault> {
        let mismatch = || Fault::Error(RuntimeError::type_mismatch("log 必填形参", "调用形态"));
        let parameters = log_parameters(declaration.id).ok_or_else(mismatch)?;
        if arguments.len() != parameters.len() {
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
        let values = values.map(|value| value.expect("形参长度和槽位已校验"));
        let log = self.log.clone().ok_or_else(|| {
            Fault::Fatal(xiao_diagnostics::FatalError::runtime_invariant(
                "缺少当前运行的日志状态",
            ))
        })?;
        let deadline = self
            .cancellation
            .as_ref()
            .and_then(CancellationSource::deadline);
        log.state
            .write(values, self.capabilities, &log.clock, deadline, || {
                self.host_checkpoint()
            })
            .map_err(|failure| match failure {
                Failure::Runtime(error) => Fault::Error(error),
                Failure::Interrupted(control) => control,
            })
    }
}

#[cfg(test)]
#[path = "log_intrinsics_tests.rs"]
/// VM 输出失败、任务取消与根运行控制回归。
mod tests;
