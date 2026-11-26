//! B3 的 UTC 墙钟、运行内单调时钟、格式化和可中断等待。

#[cfg(feature = "test-failpoints")]
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use xiao_intrinsics::CAPABILITY_CLOCK;

use crate::{RuntimeError, RuntimeErrorKind, RuntimeResult, RuntimeValue, StringHandle};

/// 超过冻结等待上限时的可恢复错误码。
pub const TIME_SLEEP_LIMIT_EXCEEDED: &str = "TIME_SLEEP_LIMIT_EXCEEDED";

/// UTC 秒和小数部分的纳秒进位。
const NS_PER_SECOND: i64 = 1_000_000_000;
/// 毫秒换算单位。
const NS_PER_MILLISECOND: i64 = 1_000_000;
/// 一次等待允许接纳的最大时长，含边界。
const MAX_SLEEP: Duration = Duration::from_secs(60);
/// 宿主等待每次把控制权交回安全点的间隔。
const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[cfg(feature = "test-failpoints")]
thread_local! {
    static VM_CLOCK_OVERRIDE: RefCell<Option<Arc<TimeContext>>> = const { RefCell::new(None) };
}

/// time 的显式计量单位。
#[derive(Clone, Copy, Debug)]
pub enum TimeUnit {
    /// 整数毫秒。
    Milliseconds,
    /// 整数纳秒。
    Nanoseconds,
}

impl TimeUnit {
    /// 核对仅有的小写单位。
    pub fn parse(raw: &str) -> RuntimeResult<Self> {
        match raw {
            "ms" => Ok(Self::Milliseconds),
            "ns" => Ok(Self::Nanoseconds),
            _ => Err(RuntimeError::invalid_value("时间单位必须是 ms 或 ns")),
        }
    }

    /// 把运行输入的计量单位换算为纳秒比例。
    fn nanoseconds(self) -> i64 {
        match self {
            Self::Milliseconds => NS_PER_MILLISECOND,
            Self::Nanoseconds => 1,
        }
    }

    /// 先按整数单位向下取整，再检查输出范围。
    pub(crate) fn convert(self, nanos: i128) -> RuntimeResult<i64> {
        i64::try_from(nanos.div_euclid(i128::from(self.nanoseconds())))
            .map_err(|_| RuntimeError::numeric_overflow("时间读数超出 int 范围"))
    }
}

/// 同一应用及其子任务共享的时钟。
#[derive(Debug)]
pub struct TimeContext {
    source: ClockSource,
    elapsed: Mutex<Duration>,
}

/// 模拟时钟的墙钟固定，宿主时钟保留实际起点。
#[derive(Debug)]
enum ClockSource {
    Host(Instant),
    Fixed { wall_nanoseconds: i128 },
}

impl Default for TimeContext {
    /// 普通应用从当前宿主单调钟建立自己的起点。
    fn default() -> Self {
        Self::host()
    }
}

impl TimeContext {
    /// 建立本次 VM 运行的时钟，受控测量可在当前线程注入一个实例。
    #[must_use]
    pub fn for_vm_run() -> Arc<Self> {
        #[cfg(feature = "test-failpoints")]
        if let Some(clock) = VM_CLOCK_OVERRIDE.with_borrow(Clone::clone) {
            return clock;
        }
        Arc::new(Self::host())
    }

    /// 在当前线程的一次受控 VM 调用中注入时钟。
    #[cfg(feature = "test-failpoints")]
    pub fn with_vm_clock<T>(self, run: impl FnOnce() -> T) -> T {
        /// 嵌套或 panic 展开时恢复线程已有的测试时钟。
        struct Restore(Option<Arc<TimeContext>>);
        impl Drop for Restore {
            /// 恢复进入受控调用前的覆盖值。
            fn drop(&mut self) {
                VM_CLOCK_OVERRIDE.replace(self.0.take());
            }
        }
        let _restore = Restore(VM_CLOCK_OVERRIDE.replace(Some(Arc::new(self))));
        run()
    }

    /// 捕获应用进入前的单调钟起点。
    #[must_use]
    pub fn host() -> Self {
        Self {
            source: ClockSource::Host(Instant::now()),
            elapsed: Mutex::new(Duration::ZERO),
        }
    }

    /// 嵌入测量使用的受控钟。等待推进单调钟，墙钟保持给定值。
    #[must_use]
    pub fn fixed(wall_clock_unix_ms: i64, monotonic_elapsed_ns: u64) -> Self {
        Self {
            source: ClockSource::Fixed {
                wall_nanoseconds: i128::from(wall_clock_unix_ms) * i128::from(NS_PER_MILLISECOND),
            },
            elapsed: Mutex::new(Duration::from_nanos(monotonic_elapsed_ns)),
        }
    }

    /// 读取 UTC，纪元前的读数按单位向下取整。
    pub fn utc_now(&self, unit: TimeUnit) -> RuntimeResult<i64> {
        let nanos = match self.source {
            ClockSource::Fixed { wall_nanoseconds } => wall_nanoseconds,
            ClockSource::Host(_) => match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_nanos() as i128,
                Err(before_epoch) => -(before_epoch.duration().as_nanos() as i128),
            },
        };
        unit.convert(nanos)
    }

    /// 读取本次运行的经过时间，沿同一运行已发布的读数钳制回退。
    pub fn monotonic(&self, unit: TimeUnit) -> RuntimeResult<i64> {
        unit.convert(self.elapsed().as_nanos() as i128)
    }

    /// 更新并读取已发布的纳秒级单调下界。
    fn elapsed(&self) -> Duration {
        let mut elapsed = self.elapsed.lock().expect("单调时钟锁未中毒");
        if let ClockSource::Host(start) = self.source {
            *elapsed = (*elapsed).max(Instant::now().saturating_duration_since(start));
        }
        *elapsed
    }

    /// 按单调钟等待，返回调用方在安全点观察到的中断。
    pub fn sleep<T>(
        &self,
        duration: Duration,
        mut interruption: impl FnMut() -> Option<T>,
    ) -> RuntimeResult<Option<T>> {
        let interrupted = interruption();
        if interrupted.is_some() || duration.is_zero() {
            return Ok(interrupted);
        }
        let deadline = self
            .elapsed()
            .checked_add(duration)
            .ok_or_else(|| RuntimeError::numeric_overflow("sleep 期限超出时钟范围"))?;
        loop {
            let remaining = deadline.saturating_sub(self.elapsed());
            if let Some(error) = interruption() {
                return Ok(Some(error));
            }
            if remaining.is_zero() {
                return Ok(None);
            }
            match self.source {
                ClockSource::Host(_) => std::thread::sleep(remaining.min(POLL_INTERVAL)),
                ClockSource::Fixed { .. } => {
                    let mut elapsed = self.elapsed.lock().expect("单调时钟锁未中毒");
                    *elapsed = (*elapsed).max(deadline);
                }
            }
        }
    }
}

/// 检查已求值的字符串类型，输入由调用者保持强引用。
pub fn unit_argument(value: &RuntimeValue) -> RuntimeResult<&StringHandle> {
    match value {
        RuntimeValue::Str(unit) => Ok(unit),
        _ => Err(RuntimeError::type_mismatch("str", value.type_name())),
    }
}

/// 核对 int 类型，保留定宽整型语义。
pub fn int_argument(value: &RuntimeValue) -> RuntimeResult<i64> {
    match value {
        RuntimeValue::Int(number) => Ok(*number),
        _ => Err(RuntimeError::type_mismatch("int", value.type_name())),
    }
}

/// 在读取宿主时钟之前核对授权。
pub fn require_clock(capabilities: u64) -> RuntimeResult<()> {
    if capabilities & CAPABILITY_CLOCK == 0 {
        return Err(RuntimeError::capability_unavailable("clock"));
    }
    Ok(())
}

/// 按冻结的 60 秒上限接纳等待时长。
pub fn sleep_duration(duration: i64, unit: TimeUnit) -> RuntimeResult<Duration> {
    let duration = u64::try_from(duration)
        .map_err(|_| RuntimeError::invalid_value("sleep 时长必须大于或等于零"))?;
    let duration = match unit {
        TimeUnit::Milliseconds => Duration::from_millis(duration),
        TimeUnit::Nanoseconds => Duration::from_nanos(duration),
    };
    if duration > MAX_SLEEP {
        return Err(RuntimeError::new(
            RuntimeErrorKind::Resource,
            TIME_SLEEP_LIMIT_EXCEEDED,
            "runtime.time_sleep_limit_exceeded",
            "sleep 时长超过 60 秒",
        ));
    }
    Ok(duration)
}

/// 前推格里高利历的 UTC 文本。按单位固定输出 24 或 30 字节。
pub fn format_utc(unix_time: i64, unit: TimeUnit) -> RuntimeResult<StringHandle> {
    let (scale, precision) = match unit {
        TimeUnit::Milliseconds => {
            if !(-62_135_596_800_000..=253_402_300_799_999).contains(&unix_time) {
                return Err(RuntimeError::invalid_value("UTC 毫秒超出 0001～9999 年"));
            }
            (1_000, 3)
        }
        TimeUnit::Nanoseconds => (NS_PER_SECOND, 9),
    };
    let seconds = unix_time.div_euclid(scale);
    let fraction = unix_time.rem_euclid(scale);
    let (year, month, day) = civil_date(seconds.div_euclid(86_400));
    let time = seconds.rem_euclid(86_400);
    let hour = time / 3_600;
    let minute = time % 3_600 / 60;
    let second = time % 60;
    StringHandle::new(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{fraction:0precision$}Z"
    ))
}

/// 将 Unix 日偏移映射到从三月起算的 400 年格里高利周期。
fn civil_date(unix_days: i64) -> (i64, i64, i64) {
    let days = unix_days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let march_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * march_month + 2) / 5 + 1;
    let month = if march_month < 10 {
        march_month + 3
    } else {
        march_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
/// 时间算术和受控等待回归。
mod tests {
    use super::*;

    #[test]
    /// 覆盖年界、闰日、纳秒极值及向下取整。
    fn formats_frozen_bounds_and_negative_fractions() {
        for (unit, input, expected) in [
            (
                TimeUnit::Milliseconds,
                -62_135_596_800_000,
                "0001-01-01T00:00:00.000Z",
            ),
            (TimeUnit::Milliseconds, -1, "1969-12-31T23:59:59.999Z"),
            (
                TimeUnit::Milliseconds,
                951_782_400_000,
                "2000-02-29T00:00:00.000Z",
            ),
            (
                TimeUnit::Milliseconds,
                253_402_300_799_999,
                "9999-12-31T23:59:59.999Z",
            ),
            (
                TimeUnit::Nanoseconds,
                i64::MIN,
                "1677-09-21T00:12:43.145224192Z",
            ),
            (TimeUnit::Nanoseconds, -1, "1969-12-31T23:59:59.999999999Z"),
            (
                TimeUnit::Nanoseconds,
                i64::MAX,
                "2262-04-11T23:47:16.854775807Z",
            ),
        ] {
            assert_eq!(
                format_utc(input, unit).unwrap().to_string().unwrap(),
                expected
            );
        }
        for out_of_bounds in [-62_135_596_800_001, 253_402_300_800_000] {
            assert_eq!(
                format_utc(out_of_bounds, TimeUnit::Milliseconds)
                    .unwrap_err()
                    .code(),
                "X06-RUNTIME-012"
            );
        }
        assert_eq!(TimeUnit::Milliseconds.convert(-1).unwrap(), -1);
        assert_eq!(
            TimeContext::fixed(i64::MAX, 0)
                .utc_now(TimeUnit::Nanoseconds)
                .unwrap_err()
                .code(),
            "X06-RUNTIME-009"
        );
    }

    #[test]
    /// 受控等待逐项验证时长边界和安全点的优先顺序。
    fn controlled_sleep_checks_control_before_completion() {
        let clock = TimeContext::fixed(946_684_800_000, 0);
        for (duration, unit) in [
            (0, TimeUnit::Milliseconds),
            (1, TimeUnit::Nanoseconds),
            (1, TimeUnit::Milliseconds),
            (60_000, TimeUnit::Milliseconds),
        ] {
            let wait = sleep_duration(duration, unit).unwrap();
            let start = clock.elapsed();
            assert_eq!(clock.sleep(wait, || None::<()>).unwrap(), None);
            assert_eq!(clock.elapsed() - start, wait);
        }
        assert_eq!(
            sleep_duration(60_000_000_001, TimeUnit::Nanoseconds)
                .unwrap_err()
                .code(),
            TIME_SLEEP_LIMIT_EXCEEDED
        );
        assert_eq!(
            sleep_duration(-1, TimeUnit::Milliseconds)
                .unwrap_err()
                .code(),
            "X06-RUNTIME-012"
        );
        assert_eq!(
            clock.sleep(Duration::ZERO, || Some("取消")).unwrap(),
            Some("取消")
        );
        let deadline = clock.elapsed() + Duration::from_nanos(1);
        assert_eq!(
            clock
                .sleep(Duration::from_nanos(1), || {
                    (clock.elapsed() >= deadline).then_some("取消")
                })
                .unwrap(),
            Some("取消")
        );
    }
}
