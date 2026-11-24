//! time 的借读 C ABI 和原生运行时钟。

use super::*;
use crate::time::{self, TimeContext, TimeUnit};
use std::sync::Arc;
use xiao_intrinsics::validate_capability_mask;

thread_local! {
    static ENTRY_CLOCK: RefCell<Option<Arc<TimeContext>>> = const { RefCell::new(None) };
}

pub(super) fn begin_entry() {
    #[cfg(feature = "test-failpoints")]
    if let (Ok(wall), Ok(mono)) = (
        std::env::var("XIAO_TEST_TIME_WALL_CLOCK_UNIX_MS"),
        std::env::var("XIAO_TEST_TIME_MONOTONIC_NS"),
    ) && let (Ok(wall), Ok(mono)) = (wall.parse(), mono.parse())
    {
        ENTRY_CLOCK.replace(Some(Arc::new(TimeContext::fixed(wall, mono))));
        return;
    }
    ENTRY_CLOCK.replace(Some(Arc::new(TimeContext::host())));
}

pub(super) fn entry_clock() -> Option<Arc<TimeContext>> {
    ENTRY_CLOCK.with_borrow(Clone::clone)
}

pub(super) struct ClockGuard(Option<Arc<TimeContext>>);

impl Drop for ClockGuard {
    fn drop(&mut self) {
        ENTRY_CLOCK.replace(self.0.take());
    }
}

pub(super) fn enter_clock(clock: Option<Arc<TimeContext>>) -> ClockGuard {
    ClockGuard(ENTRY_CLOCK.replace(clock))
}

fn current_clock() -> Arc<TimeContext> {
    ENTRY_CLOCK.with_borrow_mut(|clock| {
        clock
            .get_or_insert_with(|| Arc::new(TimeContext::host()))
            .clone()
    })
}

fn validate_inputs<const N: usize>(
    inputs: [&XiaoValue; N],
    tags: [XiaoValueTag; N],
    output: Option<&XiaoValue>,
    capabilities: u64,
    needs_clock: bool,
) -> Result<(), i32> {
    if output.is_some_and(|out| inputs.iter().any(|input| std::ptr::eq(*input, out))) {
        return Err(XiaoAbiStatus::InvalidArgument.code());
    }
    if inputs.iter().any(|value| !value.tag.is_known())
        || output.is_some_and(|value| !value.tag.is_known())
        || validate_capability_mask(capabilities).is_err()
    {
        return Err(XiaoAbiStatus::InvalidArgument.code());
    }
    for value in inputs {
        unsafe { super::abi_value_validation::validate_structure(value, false) }?;
    }
    if let Some(value) = output {
        unsafe { super::abi_value_validation::validate_structure(value, true) }?;
    }
    for (value, expected) in inputs.iter().zip(tags) {
        if value.tag != expected {
            let expected = if expected == XiaoValueTag::Int {
                "int"
            } else {
                "str"
            };
            return Err(status_from_error(&RuntimeError::type_mismatch(
                expected,
                format!("{:?}", value.tag),
            )));
        }
    }
    if needs_clock {
        status(time::require_clock(capabilities))?;
    }
    Ok(())
}

fn parse_unit(value: &XiaoValue) -> Result<TimeUnit, i32> {
    let handle = unsafe { expect_strong(value.payload.handle, RuntimeTypeTag::String) }?;
    let value = status(StringHandle::from_strong_handle(handle))?;
    status(value.with_str(TimeUnit::parse).and_then(|unit| unit))
}

fn read_clock(
    unit: *const XiaoValue,
    out: *mut XiaoValue,
    capabilities: u64,
    read: impl FnOnce(&TimeContext, TimeUnit) -> RuntimeResult<i64>,
) -> Result<(), i32> {
    let (Some(unit), Some(output)) = (unsafe { (unit.as_ref(), out.as_ref()) }) else {
        return Err(XiaoAbiStatus::Null.code());
    };
    validate_inputs(
        [unit],
        [XiaoValueTag::Str],
        Some(output),
        capabilities,
        true,
    )?;
    let unit = parse_unit(unit)?;
    let clock = current_clock();
    let value = XiaoValue::int(status(read(&clock, unit))?);
    unsafe { write_value(out, value) }
}

/// 读取有符号 Unix 时间。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_time_utc_now(
    unit: *const XiaoValue,
    out: *mut XiaoValue,
    capabilities: u64,
) -> i32 {
    read_clock(unit, out, capabilities, TimeContext::utc_now).map_or_else(|error| error, |()| 0)
}

/// 读取本次应用的单调时间。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_time_monotonic(
    unit: *const XiaoValue,
    out: *mut XiaoValue,
    capabilities: u64,
) -> i32 {
    read_clock(unit, out, capabilities, TimeContext::monotonic).map_or_else(|error| error, |()| 0)
}

/// 生成拥有的 UTC 字符串并替换输出槽。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_time_format_utc(
    unix_time: *const XiaoValue,
    unit: *const XiaoValue,
    out: *mut XiaoValue,
    capabilities: u64,
) -> i32 {
    let run = || {
        let (Some(value), Some(unit), Some(output)) =
            (unsafe { (unix_time.as_ref(), unit.as_ref(), out.as_ref()) })
        else {
            return Err(XiaoAbiStatus::Null.code());
        };
        validate_inputs(
            [value, unit],
            [XiaoValueTag::Int, XiaoValueTag::Str],
            Some(output),
            capabilities,
            false,
        )?;
        let unit = parse_unit(unit)?;
        let result = status(time::format_utc(unsafe { value.payload.i64_value }, unit))?;
        let result = value_from_owned_handle(XiaoValueTag::Str, result.into_strong_handle());
        unsafe { write_value(out, result) }
    };
    run().map_or_else(|error| error, |()| 0)
}

/// 按当前运行的单调时钟和取消通道等待。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_time_sleep(
    duration: *const XiaoValue,
    unit: *const XiaoValue,
    capabilities: u64,
) -> i32 {
    let run = || {
        let (Some(value), Some(unit)) = (unsafe { (duration.as_ref(), unit.as_ref()) }) else {
            return Err(XiaoAbiStatus::Null.code());
        };
        validate_inputs(
            [value, unit],
            [XiaoValueTag::Int, XiaoValueTag::Str],
            None,
            capabilities,
            true,
        )?;
        let unit = parse_unit(unit)?;
        let duration = status(time::sleep_duration(
            unsafe { value.payload.i64_value },
            unit,
        ))?;
        let interrupted = status(current_clock().sleep(duration, || {
            let status = super::abi_concurrency::cancellation_status();
            (status != 0).then_some(status)
        }))?;
        interrupted.map_or(Ok(()), Err)
    };
    run().map_or_else(|error| error, |()| 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xiao_intrinsics::CAPABILITY_CLOCK;

    fn string(value: &str) -> XiaoValue {
        value_from_owned_handle(
            XiaoValueTag::Str,
            StringHandle::new(value).unwrap().into_strong_handle(),
        )
    }

    fn pending_code() -> String {
        PENDING_ERROR.with_borrow(|error| match error.as_ref() {
            Some(PendingError::Recoverable(error)) => error.code().to_owned(),
            _ => panic!("预期可恢复错误"),
        })
    }

    #[test]
    fn validates_borrowed_inputs_and_preserves_owned_output_on_failure() {
        let mut unit = string("ms");
        let mut invalid = string("MS");
        let mut out = string("old");
        let old = unsafe { out.payload.raw };
        let unknown = XiaoValue {
            tag: XiaoValueTag::from_raw(999),
            payload: XiaoValuePayload { raw: 0 },
        };
        let missing = XiaoValue {
            tag: XiaoValueTag::Str,
            payload: XiaoValuePayload {
                handle: std::ptr::null_mut(),
            },
        };
        set_pending_runtime_error(RuntimeError::invalid_value("原有错误"));
        let pointer = &raw mut unit;
        assert_eq!(
            xiao_runtime_time_utc_now(pointer, pointer, CAPABILITY_CLOCK),
            2
        );
        for (value, expected) in [
            (std::ptr::null(), 1),
            (&raw const unknown, 2),
            (&raw const missing, 3),
        ] {
            assert_eq!(
                xiao_runtime_time_utc_now(value, &raw mut out, CAPABILITY_CLOCK),
                expected
            );
            assert_eq!(pending_code(), "X06-RUNTIME-012");
        }
        xiao_runtime_error_clear();
        assert_eq!(
            xiao_runtime_time_utc_now(&XiaoValue::int(4), &mut out, 0),
            6
        );
        assert_eq!(pending_code(), "X06-RUNTIME-002");
        xiao_runtime_error_clear();
        assert_eq!(xiao_runtime_time_utc_now(&invalid, &mut out, 0), 6);
        assert_eq!(pending_code(), "X06-RUNTIME-026");
        xiao_runtime_error_clear();
        assert_eq!(
            xiao_runtime_time_monotonic(&invalid, &mut out, CAPABILITY_CLOCK),
            6
        );
        assert_eq!(pending_code(), "X06-RUNTIME-012");
        let allocation = crate::allocation_testing::StringAllocationFailure::after(0);
        xiao_runtime_error_clear();
        assert_eq!(
            xiao_runtime_time_format_utc(&XiaoValue::int(0), &unit, &mut out, 0),
            6
        );
        assert!(allocation.was_triggered());
        assert_eq!(pending_code(), "X06-RUNTIME-011");
        assert_eq!(unsafe { out.payload.raw }, old);
        assert_eq!(
            xiao_runtime_time_format_utc(&XiaoValue::int(0), &unit, &mut out, 0),
            0
        );
        assert_eq!(pending_code(), "X06-RUNTIME-011");
        assert_ne!(unsafe { out.payload.raw }, old);
        xiao_runtime_error_clear();
        assert_eq!(
            xiao_runtime_time_sleep(&XiaoValue::int(60_001), &unit, CAPABILITY_CLOCK),
            6
        );
        assert_eq!(pending_code(), time::TIME_SLEEP_LIMIT_EXCEEDED);
        assert_eq!(
            xiao_runtime_time_sleep(&XiaoValue::int(0), &unit, CAPABILITY_CLOCK),
            0
        );
        for value in [&mut unit, &mut invalid, &mut out] {
            xiao_runtime_value_release_strong(value);
        }
        xiao_runtime_error_clear();
    }

    #[test]
    fn host_wait_respects_duration_and_cooperative_cancel() {
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let mut unit = string("ms");
        for milliseconds in [1, 10, 100] {
            let mut samples = Vec::new();
            for _ in 0..5 {
                let start = Instant::now();
                assert_eq!(
                    xiao_runtime_time_sleep(&XiaoValue::int(milliseconds), &unit, CAPABILITY_CLOCK),
                    0
                );
                let elapsed = start.elapsed();
                assert!(elapsed >= Duration::from_millis(milliseconds as u64));
                samples.push(elapsed.as_nanos());
            }
            println!("B3-1-SLEEP requested_ms={milliseconds} samples_ns={samples:?}");
        }
        xiao_runtime_value_release_strong(&mut unit);

        let mut cancellation_samples = Vec::new();
        for _ in 0..5 {
            let runtime = crate::RuntimeHandle::create();
            let (ready, started) = mpsc::channel();
            let child = crate::scheduler::spawn_cooperative(&runtime, move |_| {
                reset_child_error();
                let mut unit = string("ms");
                ready.send(()).unwrap();
                let status =
                    xiao_runtime_time_sleep(&XiaoValue::int(60_000), &unit, CAPABILITY_CLOCK);
                let result = (status, pending_code(), Instant::now());
                xiao_runtime_value_release_strong(&mut unit);
                reset_child_error();
                result
            })
            .unwrap();
            started.recv_timeout(Duration::from_secs(2)).unwrap();
            let cancel_at = Instant::now();
            child.cancel();
            let (status, code, completed) = child.join().unwrap();
            assert_eq!(
                (status, code.as_str()),
                (6, crate::CONCURRENCY_CANCELLED_CODE)
            );
            cancellation_samples.push(completed.duration_since(cancel_at));
            runtime.close();
        }
        let nanos: Vec<_> = cancellation_samples
            .iter()
            .map(Duration::as_nanos)
            .collect();
        println!("B3-1-SLEEP cancellation_ns={nanos:?}");
        cancellation_samples.sort_unstable();
        assert!(cancellation_samples[2] <= Duration::from_millis(100));
        assert!(cancellation_samples[4] <= Duration::from_secs(1));
    }

    fn reset_child_error() {
        super::super::abi_concurrency::reset_cancellation_delivery();
        xiao_runtime_error_clear();
    }
}
