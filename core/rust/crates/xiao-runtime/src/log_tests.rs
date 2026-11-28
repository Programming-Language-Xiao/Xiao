//! 用受控时钟和可观察输出端验证日志契约。

use super::*;
use std::cell::Cell;
use std::sync::Barrier;
use std::sync::atomic::{AtomicUsize, Ordering};

use xiao_intrinsics::{CAPABILITY_CLOCK, CAPABILITY_STDOUT, DEFAULT_CAPABILITY_MASK};

/// 精确授权日志使用的两项能力。
const MASK: u64 = CAPABILITY_CLOCK | CAPABILITY_STDOUT;
/// 包含非零毫秒的 UTC 样本。
const WALL_MILLIS: i64 = 946_684_800_123;
/// 与格式化实现独立的样本期望。
const TIMESTAMP: &str = "2000-01-01T00:00:00.123Z";

#[derive(Clone, Copy, Eq, PartialEq)]
/// 指定输出端失败的阶段，部分写入在第二次写入时报错。
enum IoFault {
    Admit,
    Write,
    Partial,
    Flush,
}

/// 保存已写字节、刷新边界和注入阶段的输出端。
struct Probe {
    bytes: Vec<u8>,
    admissions: usize,
    writes: usize,
    flush_offsets: Vec<usize>,
    chunk: usize,
    fault: Option<IoFault>,
    stage: ArcStage,
}

/// 供安全点观察的共享阶段，读取时无需再次锁住输出端。
type ArcStage = std::sync::Arc<AtomicUsize>;
/// 输出端已经收到接纳请求。
const ADMITTED: usize = 1;
/// 至少写入了一个分块。
const WRITTEN: usize = 2;
/// 输出端已经收到刷新请求。
const FLUSHED: usize = 3;

impl Default for Probe {
    /// 从空的捕获端开始，写入可一次消费整条记录。
    fn default() -> Self {
        Self {
            bytes: Vec::new(),
            admissions: 0,
            writes: 0,
            flush_offsets: Vec::new(),
            chunk: usize::MAX,
            fault: None,
            stage: ArcStage::default(),
        }
    }
}

impl LogOutput for Probe {
    /// 记录接纳调用，并可注入接纳拒绝。
    fn admit(&mut self, _: usize) -> io::Result<()> {
        self.admissions += 1;
        self.stage.store(ADMITTED, Ordering::Relaxed);
        if self.fault == Some(IoFault::Admit) {
            return Err(io::ErrorKind::Unsupported.into());
        }
        Ok(())
    }
}

impl Write for Probe {
    /// 保存已写分块，必要时在指定调用拒绝写入。
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        if self.fault == Some(IoFault::Write)
            || (self.fault == Some(IoFault::Partial) && self.writes == 2)
        {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "private log message",
            ));
        }
        let count = bytes.len().min(self.chunk);
        self.bytes.extend_from_slice(&bytes[..count]);
        self.stage.store(WRITTEN, Ordering::Relaxed);
        Ok(count)
    }

    /// 记录刷新时可见的完整输出长度。
    fn flush(&mut self) -> io::Result<()> {
        self.flush_offsets.push(self.bytes.len());
        self.stage.store(FLUSHED, Ordering::Relaxed);
        if self.fault == Some(IoFault::Flush) {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        Ok(())
    }
}

/// 按冻结实参顺序构造拥有的输入字符串。
fn strings(level: &str, message: &str, target: &str) -> [RuntimeValue; 3] {
    [level, message, target].map(|text| RuntimeValue::new_string(text).unwrap())
}

/// 在双位授权和固定墙钟下交付一条记录。
fn write(state: &LogState<Probe>, level: &str, message: &str) -> Result<(), Failure<()>> {
    let args = strings(level, message, "stdout");
    state.write(
        args.each_ref(),
        MASK,
        &TimeContext::fixed(WALL_MILLIS, 0),
        None,
        || Ok(()),
    )
}

/// 提取可恢复错误并检查其稳定编号。
fn error<C: std::fmt::Debug>(result: Result<(), Failure<C>>, expected: &str) -> RuntimeError {
    let Failure::Runtime(error) = result.unwrap_err() else {
        panic!("应返回 Runtime 错误 {expected}");
    };
    assert_eq!(error.code(), expected);
    error
}

/// 同时检查已开始尝试数和完整编码的账目。
fn usage(state: &LogState<Probe>, attempts: usize, bytes: usize) {
    let usage = state.usage.lock().unwrap();
    assert_eq!((usage.attempts, usage.bytes), (attempts, bytes));
}

#[test]
/// 四级统一输出 UTC 文本，空消息和 Unicode 均保留。
fn four_levels_empty_and_multibyte_messages_use_fixed_utc_and_flush() {
    let state = LogState::new(Probe::default());
    let mut expected = String::new();
    let mut offsets = Vec::new();
    for (level, encoded, text) in [
        ("debug", "DEBUG", ""),
        ("info", "INFO", "星崽"),
        ("warn", "WARN", "  空格 "),
        ("error", "ERROR", "🦊"),
    ] {
        write(&state, level, text).unwrap();
        writeln!(expected, "{TIMESTAMP} {encoded} {text}").unwrap();
        offsets.push(expected.len());
    }
    let output = state.output.lock().unwrap();
    assert_eq!(TIMESTAMP.len(), 24);
    assert_eq!(output.bytes, expected.as_bytes());
    assert_eq!(output.flush_offsets, offsets);
    usage(&state, 4, expected.len());
}

#[test]
/// 类型、缺位、非法值及超限的组合都在宿主操作之前拒绝。
fn signature_capabilities_parameters_and_budget_precede_host_access() {
    let state = LogState::new(Probe::default());
    let reads = Cell::new(0);
    let points = Cell::new(0);
    let run = |args: &[RuntimeValue; 3], mask| {
        state.write_with_clocks(
            args.each_ref(),
            mask,
            || {
                reads.set(reads.get() + 1);
                Ok(WALL_MILLIS)
            },
            Control {
                now: Instant::now,
                checkpoint: || {
                    points.set(points.get() + 1);
                    Err("X09-DRIVER-001")
                },
                run_deadline: None,
            },
        )
    };
    let long = "x".repeat(MAX_MESSAGE_BYTES + 1);
    for index in 0..3 {
        let mut args = strings("INFO", &long, "stderr");
        args[index] = RuntimeValue::Int(1);
        error(run(&args, 0), "X06-RUNTIME-002");
    }
    for (mask, capability) in [
        (0, "clock"),
        (CAPABILITY_STDOUT, "clock"),
        (CAPABILITY_CLOCK, "stdout"),
    ] {
        let err = error(
            run(&strings("INFO", &long, "stderr"), mask),
            "X06-RUNTIME-026",
        );
        assert_eq!(
            err.report().context.get("capability"),
            Some(&DiagnosticParam::Text(capability.to_owned()))
        );
    }
    for (level, message, target) in [
        ("INFO", long.as_str(), "stdout"),
        ("bogus", "", "stdout"),
        ("", "", "stdout"),
        ("info", "\r", "stdout"),
        ("info", "\n", "stdout"),
        ("info", "\0", "stdout"),
        ("info", "", "stderr"),
        ("info", "", "file"),
        ("info", "", ""),
    ] {
        error(
            run(&strings(level, message, target), MASK),
            "X06-RUNTIME-012",
        );
    }
    error(
        run(&strings("info", &format!("{long}\n"), "stdout"), MASK),
        "X06-RUNTIME-012",
    );
    error(
        run(&strings("info", &long, "stdout"), MASK),
        "LOG_LIMIT_EXCEEDED",
    );
    assert_eq!(reads.get(), 0);
    assert_eq!(points.get(), 0);
    let output = state.output.lock().unwrap();
    assert_eq!(
        (output.admissions, output.writes, output.flush_offsets.len()),
        (0, 0, 0)
    );
    assert!(output.bytes.is_empty());
    usage(&state, 0, 0);
}

#[test]
/// 两位精确授权以及旧请求的默认授权均可写入。
fn exact_and_legacy_capability_masks_are_accepted() {
    let state = LogState::new(Probe::default());
    let args = strings("info", "", "stdout");
    for mask in [0x22, DEFAULT_CAPABILITY_MASK] {
        state
            .write(
                args.each_ref(),
                mask,
                &TimeContext::fixed(WALL_MILLIS, 0),
                None,
                || Ok::<_, ()>(()),
            )
            .unwrap();
    }
    assert_eq!(
        state.output.lock().unwrap().bytes,
        format!("{TIMESTAMP} INFO \n").repeat(2).as_bytes()
    );
}

#[test]
/// 多字节消息按 UTF-8 字节计数，验证最长编码尺寸。
fn message_and_encoded_byte_limits_include_the_boundary() {
    let state = LogState::new(Probe::default());
    let mut message = "雪".repeat(1_365);
    assert_eq!(message.len(), 4_095);
    write(&state, "error", &message).unwrap();
    message.push('x');
    write(&state, "error", &message).unwrap();
    message.push('x');
    error(write(&state, "error", &message), "LOG_LIMIT_EXCEEDED");
    usage(&state, 2, 4_127 + 4_128);
    assert_eq!(
        state.output.lock().unwrap().flush_offsets,
        [4_127, 4_127 + 4_128]
    );
}

#[test]
/// 达到条数上限后，新调用在读取时钟和采样运行控制前被拒绝。
fn attempts_include_1023_and_1024_and_reject_1025() {
    let state = LogState::new(Probe::default());
    for _ in 0..1_023 {
        write(&state, "info", "").unwrap();
    }
    usage(&state, 1_023, 1_023 * 31);
    write(&state, "info", "").unwrap();
    usage(&state, 1_024, 1_024 * 31);
    let args = strings("info", "", "stdout");
    error(
        state.write_with_clocks(
            args.each_ref(),
            MASK,
            || panic!("超额调用不读钟"),
            Control {
                now: || panic!("超额调用不进入安全点"),
                checkpoint: || Err(()),
                run_deadline: None,
            },
        ),
        "LOG_LIMIT_EXCEEDED",
    );
    assert_eq!(state.output.lock().unwrap().admissions, 1_024);
}

/// 以最大记录把运行额度填至距累计上限 64 字节。
fn fill_bytes(state: &LogState<Probe>) {
    let max = "x".repeat(MAX_MESSAGE_BYTES);
    for _ in 0..254 {
        write(state, "error", &max).unwrap();
    }
    usage(state, 254, MAX_RUN_BYTES - 64);
}

#[test]
/// 完整编码累计包含分隔符，最后一字节计入边界。
fn cumulative_bytes_include_separators_and_reject_one_over() {
    for (total, accepted) in [(1_048_575, true), (1_048_576, true), (1_048_577, false)] {
        let state = LogState::new(Probe::default());
        fill_bytes(&state);
        let result = write(
            &state,
            "error",
            &"a".repeat(total - (MAX_RUN_BYTES - 64) - 32),
        );
        if accepted {
            result.unwrap();
            usage(&state, 255, total);
            assert_eq!(state.output.lock().unwrap().bytes.len(), total);
            error(write(&state, "info", ""), "LOG_LIMIT_EXCEEDED");
        } else {
            error(result, "LOG_LIMIT_EXCEEDED");
            usage(&state, 254, MAX_RUN_BYTES - 64);
            assert_eq!(state.output.lock().unwrap().bytes.len(), MAX_RUN_BYTES - 64);
        }
    }
}

#[test]
/// 并发争用两种额度时，短写记录按实际刷新边界保持完整。
fn concurrent_writers_share_quotas_and_flush_whole_records_in_delivery_order() {
    for by_bytes in [false, true] {
        let state = LogState::new(Probe {
            chunk: 7,
            ..Probe::default()
        });
        let initial = if by_bytes {
            fill_bytes(&state);
            254
        } else {
            for _ in 0..1_016 {
                write(&state, "debug", "").unwrap();
            }
            1_016
        };
        let contenders = 16;
        let gate = Barrier::new(contenders);
        let mut successful = std::thread::scope(|scope| {
            let mut pending = Vec::new();
            for id in 0..contenders {
                let state = &state;
                let gate = &gate;
                pending.push(scope.spawn(move || {
                    let message = format!("task {id:02}                         ");
                    gate.wait();
                    match write(state, "error", &message) {
                        Ok(()) => Some(format!("{TIMESTAMP} ERROR {message}\n")),
                        Err(failure) => {
                            error(Err(failure), "LOG_LIMIT_EXCEEDED");
                            None
                        }
                    }
                }));
            }
            pending
                .into_iter()
                .filter_map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(successful.len(), if by_bytes { 1 } else { 8 });
        let output = state.output.lock().unwrap();
        let mut start = output.flush_offsets[initial - 1];
        let mut observed = Vec::new();
        for &end in &output.flush_offsets[initial..] {
            let record = std::str::from_utf8(&output.bytes[start..end]).unwrap();
            assert_eq!(record.bytes().filter(|&byte| byte == b'\n').count(), 1);
            observed.push(record.to_owned());
            start = end;
        }
        observed.sort();
        successful.sort();
        assert_eq!(observed, successful);
        assert_eq!(start, output.bytes.len());
    }
    let next_run = LogState::new(Probe::default());
    write(&next_run, "info", "").unwrap();
    usage(&next_run, 1, 31);
}

#[test]
/// 接纳拒绝不计费，写入和刷新失败保留已产生的字节与额度。
fn rejected_admission_and_failed_io_keep_the_right_quota_and_bytes() {
    /// 宿主错误字符串也含此消息，结构化报告应省略它。
    const TEXT: &str = "private log message";
    let full = format!("{TIMESTAMP} ERROR {TEXT}\n");
    for (fault, stage, bytes, writes, flushes) in [
        (IoFault::Admit, "接纳", 0, 0, 0),
        (IoFault::Write, "写入", 0, 1, 0),
        (IoFault::Partial, "写入", 7, 2, 0),
        (IoFault::Flush, "刷新", full.len(), 1, 1),
    ] {
        let chunk = if fault == IoFault::Partial {
            7
        } else {
            usize::MAX
        };
        let state = LogState::new(Probe {
            fault: Some(fault),
            chunk,
            ..Probe::default()
        });
        let err = error(write(&state, "error", TEXT), "LOG_OUTPUT_FAILED");
        assert_eq!(err.kind(), RuntimeErrorKind::Resource);
        assert_eq!(
            err.report().params.get("stage"),
            Some(&DiagnosticParam::Text(stage.to_owned()))
        );
        assert!(!format!("{:?}", err.report()).contains(TEXT));
        let output = state.output.lock().unwrap();
        assert_eq!(
            (output.admissions, output.writes, output.flush_offsets.len()),
            (1, writes, flushes)
        );
        assert_eq!(output.bytes, full.as_bytes()[..bytes]);
        let started = usize::from(fault != IoFault::Admit);
        usage(&state, started, started * full.len());
    }
}

#[test]
/// 用时钟和输出阶段组合验证四种中断身份及尝试入账边界。
fn controls_and_operation_timeout_apply_before_and_after_delivery_starts() {
    let args = strings("debug", "🦊", "stdout");
    let full = format!("{TIMESTAMP} DEBUG 🦊\n");
    for (stop_at, expected_bytes, expected_flushes, charged) in [
        (0, 0, 0, 0),
        (ADMITTED, 0, 0, 0),
        (WRITTEN, 7, 0, 1),
        (FLUSHED, full.len(), 1, 1),
    ] {
        for reason in [
            Some("X09-DRIVER-001"),
            Some("X09-DRIVER-002"),
            Some("X06-RUNTIME-027"),
            None,
        ] {
            let sink = Probe {
                chunk: 7,
                ..Probe::default()
            };
            let stage = ArcStage::clone(&sink.stage);
            let state = LogState::new(sink);
            let start = Instant::now();
            let samples = Cell::new(0);
            let reached = || stage.load(Ordering::Relaxed) >= stop_at;
            let result = state.write_with_clocks(
                args.each_ref(),
                MASK,
                || Ok(WALL_MILLIS),
                Control {
                    now: || {
                        samples.set(samples.get() + 1);
                        if samples.get() > 1 && reached() {
                            start + OPERATION_LIMIT
                        } else {
                            start
                        }
                    },
                    checkpoint: || {
                        if reached() {
                            reason.map_or(Ok(()), Err)
                        } else {
                            Ok(())
                        }
                    },
                    run_deadline: None,
                },
            );
            match reason {
                Some(reason) => {
                    assert!(matches!(result, Err(Failure::Interrupted(actual)) if actual == reason))
                }
                None => {
                    error(result, "LOG_TIMEOUT");
                }
            }
            let out = state.output.lock().unwrap();
            assert_eq!(out.bytes, full.as_bytes()[..expected_bytes]);
            assert_eq!(out.flush_offsets.len(), expected_flushes);
            usage(&state, charged, charged * full.len());
        }
    }
}

#[test]
/// 等待被中断时没有 UTC 样本和交付尝试。
fn waiting_for_output_checks_control_and_deadline_without_reading_wall_clock() {
    let state = LogState::new(Probe::default());
    let _occupied = state.output.lock().unwrap();
    let start = Instant::now();
    let args = strings("info", "waiting", "stdout");
    for reason in [
        Some("X09-DRIVER-001"),
        Some("X09-DRIVER-002"),
        Some("X06-RUNTIME-027"),
        None,
    ] {
        let points = Cell::new(0);
        let result = state.write_with_clocks(
            args.each_ref(),
            MASK,
            || panic!("等待时不读墙钟"),
            Control {
                now: || {
                    if points.get() >= 2 {
                        start + OPERATION_LIMIT
                    } else {
                        start
                    }
                },
                checkpoint: || {
                    points.set(points.get() + 1);
                    if points.get() >= 3 {
                        reason.map_or(Ok(()), Err)
                    } else {
                        Ok(())
                    }
                },
                run_deadline: Some(start + Duration::from_millis(8)),
            },
        );
        match reason {
            Some(reason) => {
                assert!(matches!(result, Err(Failure::Interrupted(actual)) if actual == reason))
            }
            None => {
                error(result, "LOG_TIMEOUT");
            }
        }
    }
    usage(&state, 0, 0);
}

#[test]
/// 按实际墙钟样本编码，并沿用取钟、格式域和分配错误。
fn wall_samples_can_go_back_and_clock_conversion_and_allocation_errors_propagate() {
    let state = LogState::new(Probe::default());
    let args = strings("info", "", "stdout");
    let run = |sample| {
        state.write_with_clocks(
            args.each_ref(),
            MASK,
            || sample,
            Control {
                now: Instant::now,
                checkpoint: || Ok::<_, ()>(()),
                run_deadline: None,
            },
        )
    };
    run(Ok(WALL_MILLIS)).unwrap();
    run(TimeUnit::Milliseconds.convert(-1)).unwrap();
    error(run(Ok(253_402_300_800_000)), "X06-RUNTIME-012");
    error(
        run(Err(RuntimeError::invalid_value("墙钟不可读取"))),
        "X06-RUNTIME-012",
    );
    error(
        run(TimeUnit::Milliseconds.convert(i128::MAX)),
        "X06-RUNTIME-009",
    );
    let failpoint = crate::allocation_testing::StringAllocationFailure::after(0);
    error(run(Ok(WALL_MILLIS)), "X06-RUNTIME-011");
    assert!(failpoint.was_triggered());
    let out = state.output.lock().unwrap();
    assert_eq!(
        out.bytes,
        b"2000-01-01T00:00:00.123Z INFO \n1969-12-31T23:59:59.999Z INFO \n"
    );
    assert_eq!(out.admissions, 2);
    usage(&state, 2, 62);
}
