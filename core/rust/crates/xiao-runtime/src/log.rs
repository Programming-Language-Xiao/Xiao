//! std.log 的参数校验、运行级配额和同步交付。

use std::fmt::Write as _;
use std::io::{self, Write};
use std::sync::{Mutex, TryLockError};
use std::time::{Duration, Instant};
use xiao_intrinsics::Capability;

use crate::{
    ALLOCATION_CODE, DiagnosticParam, RuntimeError, RuntimeErrorKind, RuntimeResult, RuntimeValue,
    resources,
    time::{self, TimeContext, TimeUnit},
};

/// UTC 毫秒文本的固定宽度。
const UTC_MILLIS_BYTES: usize = 24;
/// 单条消息的 UTF-8 字节上限。
const MAX_MESSAGE_BYTES: usize = 4_096;
/// 含时间、级别、分隔符与 LF 的单条编码上限。
const MAX_RECORD_BYTES: usize = 4_128;
/// 同次运行开始交付的最多记录数。
const MAX_ATTEMPTS: usize = 1_024;
/// 同次运行按完整编码累计的上限。
const MAX_RUN_BYTES: usize = 1_048_576;
/// 从首个运行控制安全点起算的单调期限。
const OPERATION_LIMIT: Duration = Duration::from_secs(10);
/// 等待输出权时交回运行控制的最长轮询间隔。
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// 日志输出端在交付前接纳一条记录，再按 Write 的字节数报告进度。
///
/// 接纳只预检资源。写入和刷新同步发生，适配器应使用受控等待，或在无法受控等待时
/// 返回错误。Runtime 在每次同步调用前后复查运行控制。
pub trait LogOutput: Write {
    /// 接纳下一条记录及其完整编码长度。
    fn admit(&mut self, bytes: usize) -> io::Result<()>;
}

/// 日志错误保留 Runtime 可恢复错误和调用方的运行控制身份。
#[derive(Debug)]
pub enum Failure<C> {
    /// 语言可捕获的 Runtime 错误。
    Runtime(RuntimeError),
    /// 原有运行控制通道，由调用者交付。
    Interrupted(C),
}

impl<C> From<RuntimeError> for Failure<C> {
    /// 保留可恢复错误的稳定身份和参数。
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

/// 一次运行中所有任务共享的输出端和已开始尝试的额度。
pub struct LogState<O> {
    output: Mutex<O>,
    // 运行控制之前预检额度。此锁只保护账目，不跨墙钟或输出操作持有。
    usage: Mutex<Usage>,
}

impl<O: LogOutput> LogState<O> {
    /// 用调用方的 stdout 输出端建立新运行，额度从零开始。
    pub fn new(output: O) -> Self {
        Self {
            output: Mutex::new(output),
            usage: Mutex::new(Usage::default()),
        }
    }

    /// 按冻结形参顺序借读字符串，写入完整记录并刷新。
    pub fn write<C>(
        &self,
        [level, message, target]: [&RuntimeValue; 3],
        capabilities: u64,
        clock: &TimeContext,
        run_deadline: Option<Instant>,
        checkpoint: impl FnMut() -> Result<(), C>,
    ) -> Result<(), Failure<C>> {
        self.write_with_clocks(
            [level, message, target],
            capabilities,
            || clock.utc_now(TimeUnit::Milliseconds),
            Control {
                now: Instant::now,
                checkpoint,
                run_deadline,
            },
        )
    }

    /// 依次检查语言类型、授权、参数和工作量，允许测试独立计数取钟次数。
    fn write_with_clocks<C>(
        &self,
        [level, message, target]: [&RuntimeValue; 3],
        capabilities: u64,
        wall_millis: impl FnOnce() -> RuntimeResult<i64>,
        control: Control<impl Fn() -> Instant, impl FnMut() -> Result<(), C>>,
    ) -> Result<(), Failure<C>> {
        let RuntimeValue::Str(level) = level else {
            return Err(RuntimeError::type_mismatch("str", level.type_name()).into());
        };
        let RuntimeValue::Str(message) = message else {
            return Err(RuntimeError::type_mismatch("str", message.type_name()).into());
        };
        let RuntimeValue::Str(target) = target else {
            return Err(RuntimeError::type_mismatch("str", target.type_name()).into());
        };
        resources::require_capability(capabilities, Capability::ClockAndStdout)?;
        let level = level.with_str(parse_level)??;
        if !target.with_str(|target| target == "stdout")? {
            return Err(RuntimeError::invalid_value("日志目标必须是 stdout").into());
        }
        message.with_str(|message| {
            if message
                .as_bytes()
                .iter()
                .any(|&ch| matches!(ch, b'\r' | b'\n' | 0))
            {
                return Err(RuntimeError::invalid_value("日志消息含 CR、LF 或 NUL").into());
            }
            if message.len() > MAX_MESSAGE_BYTES {
                return Err(limit("消息字节", MAX_MESSAGE_BYTES).into());
            }
            let bytes = checked_add(message.len(), UTC_MILLIS_BYTES + level.len() + 3)?;
            if bytes > MAX_RECORD_BYTES {
                return Err(limit("记录字节", MAX_RECORD_BYTES).into());
            }
            self.usage
                .lock()
                .expect("日志配额锁未中毒")
                .with_attempt(bytes)?;
            self.deliver(level, message, bytes, wall_millis, control)
        })?
    }

    /// 持有一个输出权直至完成全部分块和刷新，在首个写入前一次性计费。
    fn deliver<C>(
        &self,
        level: &str,
        message: &str,
        bytes: usize,
        wall_millis: impl FnOnce() -> RuntimeResult<i64>,
        mut control: Control<impl Fn() -> Instant, impl FnMut() -> Result<(), C>>,
    ) -> Result<(), Failure<C>> {
        let deadline = (control.now)()
            .checked_add(OPERATION_LIMIT)
            .ok_or_else(|| RuntimeError::numeric_overflow("日志期限超出时钟范围"))?;
        let mut output = loop {
            let now = control.check(deadline)?;
            match self.output.try_lock() {
                Ok(output) => break output,
                Err(TryLockError::WouldBlock) => {
                    let stop = control
                        .run_deadline
                        .map_or(deadline, |end| end.min(deadline));
                    std::thread::sleep(stop.saturating_duration_since(now).min(POLL_INTERVAL));
                }
                Err(TryLockError::Poisoned(_)) => {
                    return Err(output_failure("接纳", io::ErrorKind::BrokenPipe.into()).into());
                }
            }
        };
        // 其余任务可能在等待输出权时消耗最后的额度。
        let charged = self
            .usage
            .lock()
            .expect("日志配额锁未中毒")
            .with_attempt(bytes)?;
        control.check(deadline)?;
        let timestamp = wall_millis();
        control.check(deadline)?;
        let record = encode(timestamp?, level, message, bytes)?;
        control.check(deadline)?;
        let accepted = output.admit(bytes);
        control.check(deadline)?;
        accepted.map_err(|error| output_failure("接纳", error))?;

        *self.usage.lock().expect("日志配额锁未中毒") = charged;
        let mut remaining = record.as_bytes();
        while !remaining.is_empty() {
            let written = output.write(remaining);
            control.check(deadline)?;
            let count = written.map_err(|error| output_failure("写入", error))?;
            if count == 0 {
                return Err(output_failure("写入", io::ErrorKind::WriteZero.into()).into());
            }
            remaining = remaining
                .get(count..)
                .ok_or_else(|| output_failure("写入", io::ErrorKind::InvalidData.into()))?;
        }
        let flushed = output.flush();
        control.check(deadline)?;
        flushed.map_err(|error| output_failure("刷新", error))?;
        Ok(())
    }
}

/// 操作的单调时间源与运行控制回调，共同约束输出等待。
struct Control<N, F> {
    now: N,
    checkpoint: F,
    run_deadline: Option<Instant>,
}

impl<N: Fn() -> Instant, F> Control<N, F> {
    /// 先交付调用者的运行中断，再判断独立日志期限。
    fn check<C>(&mut self, deadline: Instant) -> Result<Instant, Failure<C>>
    where
        F: FnMut() -> Result<(), C>,
    {
        let now = (self.now)();
        (self.checkpoint)().map_err(Failure::Interrupted)?;
        if now >= deadline {
            return Err(RuntimeError::new(
                RuntimeErrorKind::Resource,
                "LOG_TIMEOUT",
                "runtime.log_timeout",
                "日志操作超过单调时钟期限",
            )
            .into());
        }
        Ok(now)
    }
}

#[derive(Clone, Copy, Default)]
/// 已开始交付的条数及其完整记录字节。
struct Usage {
    attempts: usize,
    bytes: usize,
}

impl Usage {
    /// 计算可提交的新账目，额度检查失败时保留现有值。
    fn with_attempt(self, bytes: usize) -> RuntimeResult<Self> {
        let attempts = checked_add(self.attempts, 1)?;
        if attempts > MAX_ATTEMPTS {
            return Err(limit("运行日志尝试次数", MAX_ATTEMPTS));
        }
        let bytes = checked_add(self.bytes, bytes)?;
        if bytes > MAX_RUN_BYTES {
            return Err(limit("运行日志累计字节", MAX_RUN_BYTES));
        }
        Ok(Self { attempts, bytes })
    }
}

/// 定宽的配额加法沿用 Runtime 算术溢出身份。
fn checked_add(left: usize, right: usize) -> RuntimeResult<usize> {
    left.checked_add(right)
        .ok_or_else(|| RuntimeError::numeric_overflow("日志计数超出整数范围"))
}

/// 严格接受冻结的小写拼写并返回 ASCII 大写级别。
fn parse_level(level: &str) -> RuntimeResult<&'static str> {
    match level {
        "debug" => Ok("DEBUG"),
        "info" => Ok("INFO"),
        "warn" => Ok("WARN"),
        "error" => Ok("ERROR"),
        _ => Err(RuntimeError::invalid_value(
            "日志级别必须是 debug、info、warn 或 error",
        )),
    }
}

/// 在受控缓冲区中构造含 LF 的完整记录。
fn encode(timestamp: i64, level: &str, message: &str, bytes: usize) -> RuntimeResult<String> {
    time::with_utc_format(timestamp, TimeUnit::Milliseconds, |timestamp| {
        #[cfg(any(test, feature = "test-failpoints"))]
        crate::allocation_testing::check_string_allocation()?;
        let mut record = String::new();
        record.try_reserve_exact(bytes).map_err(|_| {
            RuntimeError::new(
                RuntimeErrorKind::Resource,
                ALLOCATION_CODE,
                "runtime.allocation",
                "无法分配日志记录缓冲区",
            )
        })?;
        writeln!(record, "{timestamp} {level} {message}").expect("String 格式化必须成功");
        debug_assert_eq!(record.len(), bytes);
        Ok(record)
    })
}

/// 在超限报告中记录命中的资源和上界。
fn limit(resource: &str, limit: usize) -> RuntimeError {
    RuntimeError::new(
        RuntimeErrorKind::Resource,
        "LOG_LIMIT_EXCEEDED",
        "runtime.log_limit_exceeded",
        "日志超过资源限额",
    )
    .with_param("resource", DiagnosticParam::Text(resource.to_owned()))
    .with_param("limit", DiagnosticParam::Integer(limit as i128))
}

/// 保留阶段、错误种类及可用的宿主编号，避免输出端错误文本泄露消息。
fn output_failure(stage: &str, error: io::Error) -> RuntimeError {
    let mut failure = RuntimeError::new(
        RuntimeErrorKind::Resource,
        "LOG_OUTPUT_FAILED",
        "runtime.log_output_failed",
        "日志 stdout 交付失败",
    )
    .with_param("stage", DiagnosticParam::Text(stage.to_owned()))
    .with_param("reason", DiagnosticParam::Text(error.kind().to_string()));
    if let Some(code) = error.raw_os_error() {
        failure = failure.with_param("os_code", DiagnosticParam::Integer(i128::from(code)));
    }
    failure
}

#[cfg(test)]
#[path = "log_tests.rs"]
/// 冻结的记录字节、预算、并发和故障回归。
mod tests;
