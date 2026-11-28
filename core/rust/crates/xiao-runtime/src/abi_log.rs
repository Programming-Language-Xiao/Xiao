//! 日志 C ABI 与原生运行 stdout 适配。

use std::io::{self, Write};

use super::{abi_env::entry_environment, abi_value_validation::validate_structure, *};
use crate::log::{Failure, LogOutput, LogState};
use xiao_intrinsics::validate_capability_mask;

/// 每次原生运行的同步输出端。实际写入从宿主 stdout 获取，无用户空间缓冲。
#[derive(Default)]
pub(super) struct NativeStdout {
    #[cfg(test)]
    output: Option<Box<dyn LogOutput + Send>>,
}

impl LogOutput for NativeStdout {
    /// 在日志尝试入账前检查 stdout 能否接纳记录。
    fn admit(&mut self, bytes: usize) -> io::Result<()> {
        #[cfg(test)]
        if let Some(output) = &mut self.output {
            return output.admit(bytes);
        }
        let _ = bytes;
        stdout::check()
    }

    /// 首次写入和短写后的等待由 LogState 检查控制及期限。
    fn write_ready(&mut self) -> Option<io::Result<bool>> {
        #[cfg(test)]
        if let Some(output) = &mut self.output {
            return output.write_ready();
        }
        Some(stdout::write_ready())
    }
}

impl Write for NativeStdout {
    /// 返回宿主实际接纳的字节数，短写由共享日志交付处理。
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        #[cfg(test)]
        if let Some(output) = &mut self.output {
            return output.write(bytes);
        }
        stdout::write(bytes)
    }

    /// 逐条完成刷新并检查流。直写的字节已交给内核，无需等待管道消费或同步磁盘。
    fn flush(&mut self) -> io::Result<()> {
        #[cfg(test)]
        if let Some(output) = &mut self.output {
            return output.flush();
        }
        stdout::check()
    }
}

/// 沿挂起错误通道报告 C 结构失败，保留具体 ABI 状态。
fn invalid_structure(status: XiaoAbiStatus, reason: &str) -> i32 {
    set_pending_runtime_error(RuntimeError::invalid_value(reason));
    status.code()
}

/// 在无原生运行入口的嵌入调用中拒绝实际日志交付。
fn missing_entry() -> Result<(), i32> {
    set_pending_error(PendingError::Fatal(FatalError::internal(
        "日志调用缺少原生运行上下文",
    )));
    Err(XiaoAbiStatus::Fatal.code())
}

/// 借读级别、消息和目标，向本次原生运行的 stdout 交付并刷新一条记录。
///
/// # Safety
///
/// 非空且对齐的入参指针须指向调用期间可读的 ABI 槽。非空的拥有句柄须来自本 Runtime，
/// 调用者在整个调用期间持有输入。嵌入宿主通过既有原生运行入口建立运行边界。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xiao_runtime_log_write(
    level: *const XiaoValue,
    message: *const XiaoValue,
    target: *const XiaoValue,
    capabilities: u64,
) -> i32 {
    let execute = || {
        let inputs = [level, message, target];
        if inputs
            .iter()
            .any(|input| input.is_null() || !input.is_aligned())
        {
            return Err(invalid_structure(
                XiaoAbiStatus::Null,
                "日志输入槽为空或未对齐",
            ));
        }
        for (index, input) in inputs.iter().enumerate() {
            if inputs[..index].iter().any(|previous| {
                input.addr().abs_diff(previous.addr()) < std::mem::size_of::<XiaoValue>()
            }) {
                return Err(invalid_structure(
                    XiaoAbiStatus::InvalidArgument,
                    "日志输入槽重叠",
                ));
            }
        }
        // SAFETY: 调用方保证槽可读，且强引用覆盖调用。
        let inputs = inputs.map(|input| unsafe { &*input });
        if inputs.iter().any(|input| !input.tag.is_known())
            || validate_capability_mask(capabilities).is_err()
        {
            return Err(invalid_structure(
                XiaoAbiStatus::InvalidArgument,
                "日志输入标签或能力掩码无效",
            ));
        }
        for input in inputs {
            if unsafe { validate_structure(input, false) }.is_err() {
                return Err(invalid_structure(XiaoAbiStatus::Null, "日志输入槽形状无效"));
            }
        }
        let mut values = std::array::from_fn(|_| RuntimeValue::None);
        for (slot, input) in values.iter_mut().zip(inputs) {
            if input.tag != XiaoValueTag::Str {
                return Err(status_from_error(&RuntimeError::type_mismatch(
                    "str",
                    format!("{:?}", input.tag),
                )));
            }
            // ABI 字符串构造器严格解码 UTF-8，STR 强句柄只存放不可变的 Rust String。
            *slot = unsafe { value_to_runtime(input) }?;
        }

        let entry = entry_environment();
        let unbound;
        let (log, granted, deadline) = match entry.as_deref() {
            Some(entry) => (
                &entry.log,
                capabilities & entry.capabilities,
                entry.deadline(),
            ),
            None => {
                // 嵌入端也使用同一套预检。首次安全点即拒绝交付。
                unbound = LogState::new(NativeStdout::default());
                (&unbound, capabilities, None)
            }
        };
        match log.write(
            values.each_ref(),
            granted,
            &super::abi_time::current_clock(),
            deadline,
            || {
                entry
                    .as_deref()
                    .map_or_else(missing_entry, |entry| entry.checkpoint())
            },
        ) {
            Ok(()) => Ok(()),
            Err(Failure::Runtime(error)) => Err(status_from_error(&error)),
            Err(Failure::Interrupted(status)) => Err(status),
        }
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(execute)) {
        Ok(result) => result.map_or_else(|status| status, |()| XiaoAbiStatus::Ok.code()),
        Err(_) => {
            set_pending_error(PendingError::Fatal(FatalError::internal(
                "日志 C ABI 发生内部故障",
            )));
            XiaoAbiStatus::Fatal.code()
        }
    }
}

/// 原生 stdout 直接借用宿主流的实现。
#[path = "abi_log_stdout.rs"]
mod stdout;

#[cfg(test)]
impl NativeStdout {
    /// 为一次原生运行注入可观测输出端。
    pub(super) fn with_output(output: impl LogOutput + Send + 'static) -> Self {
        Self {
            output: Some(Box::new(output)),
        }
    }
}
