//! 以原生运行上下文经 C ABI 验证日志故障和运行控制。

use std::io::{self, Write};
use std::sync::Mutex;

use crate::abi::abi_log::xiao_runtime_log_write;
use crate::log::LogOutput;

use super::*;

/// 日志的精确联合掩码。
const MASK: u64 = 0x22;
/// 固定时间戳和测试消息组成的完整记录长度。
const RECORD_BYTES: usize = 24 + b" INFO secret\n".len();

/// 注入宿主 stdout 的错误位置。
#[derive(Clone, Copy, Default)]
enum Fault {
    #[default]
    None,
    Admission,
    Partial,
    Flush,
    Timeout,
}

/// 捕获已经写到宿主的字节及刷新次数。
#[derive(Default)]
struct Capture {
    bytes: Vec<u8>,
    flushes: usize,
    fault: Fault,
}

/// 每次运行安装同一捕获端，外层持有观测引用。
struct Output(Arc<Mutex<Capture>>);

impl LogOutput for Output {
    /// 接纳故障发生在记账和写入前。
    fn admit(&mut self, _: usize) -> io::Result<()> {
        if matches!(self.0.lock().unwrap().fault, Fault::Admission) {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        Ok(())
    }
}

impl Write for Output {
    /// 短写保留前七字节。期限用例模拟已开始的同步宿主调用。
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut capture = self.0.lock().unwrap();
        let count = match capture.fault {
            Fault::Partial if capture.bytes.is_empty() => 7,
            Fault::Partial => return Err(io::ErrorKind::BrokenPipe.into()),
            Fault::Timeout => {
                std::thread::sleep(Duration::from_millis(10_020));
                bytes.len()
            }
            _ => bytes.len(),
        };
        capture.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    /// 记录刷新结果，无隐式重试。
    fn flush(&mut self) -> io::Result<()> {
        let mut capture = self.0.lock().unwrap();
        capture.flushes += 1;
        if matches!(capture.fault, Fault::Flush) {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        Ok(())
    }
}

/// 保存运行共享状态并装入真实原生执行线程的 TLS。
fn context(control: NativeControl, capture: &Arc<Mutex<Capture>>) -> EnvironmentGuard {
    enter_environment(Some(Arc::new(NativeEnvironment {
        snapshot: EnvironmentSnapshot::default(),
        capabilities: MASK,
        control,
        log: LogState::new(NativeStdout::with_output(Output(Arc::clone(capture)))),
    })))
}

/// 使用真实 ABI 转换、借读及释放，返回状态和待处理错误码。
fn call(mask: u64) -> (i32, Option<String>) {
    let mut values = ["info", "secret", "stdout"]
        .map(|text| runtime_to_value(&RuntimeValue::new_string(text).unwrap()).unwrap());
    let status = unsafe { xiao_runtime_log_write(&values[0], &values[1], &values[2], mask) };
    for value in values.iter_mut().rev() {
        xiao_runtime_value_release(value);
    }
    let error = PENDING_ERROR.with_borrow(|pending| match pending {
        Some(PendingError::Recoverable(error)) => {
            assert!(!error.to_string().contains("secret"));
            if error.code() == "X06-RUNTIME-026" {
                let missing = if mask & 2 == 0 { "clock" } else { "stdout" };
                assert_eq!(
                    error.context().get("capability"),
                    Some(&crate::DiagnosticParam::Text(missing.into()))
                );
            }
            Some(error.code().to_owned())
        }
        Some(PendingError::Fatal(error)) => Some(error.code().to_owned()),
        None => None,
    });
    xiao_runtime_error_clear();
    (status, error)
}

/// 两项能力都要实际检查，目标和 ABI 预检走生产入口。
#[test]
fn native_log_checks_both_capabilities_before_admission() {
    let capture = Arc::new(Mutex::new(Capture::default()));
    let _entry = context(NativeControl::default(), &capture);
    for mask in [0, 2, 32] {
        assert_eq!(call(mask), (6, Some("X06-RUNTIME-026".into())));
    }
    assert!(capture.lock().unwrap().bytes.is_empty());
    assert_eq!(call(MASK), (0, None));
    assert_eq!(capture.lock().unwrap().flushes, 1);
}

/// 接纳不计费，部分写入和刷新失败均保留配额及实际写入的记录前缀。
#[test]
fn native_log_faults_preserve_bytes_and_attempts() {
    let capture = Arc::new(Mutex::new(Capture::default()));
    let _entry = context(NativeControl::default(), &capture);
    for (fault, expected_bytes, expected_flushes) in [
        (Fault::Admission, 0, 0),
        (Fault::Partial, 7, 0),
        (Fault::Flush, 7 + RECORD_BYTES, 1),
    ] {
        capture.lock().unwrap().fault = fault;
        assert_eq!(call(MASK), (6, Some("LOG_OUTPUT_FAILED".into())));
        let capture = capture.lock().unwrap();
        assert_eq!(capture.bytes.len(), expected_bytes);
        assert_eq!(capture.flushes, expected_flushes);
    }
    capture.lock().unwrap().fault = Fault::None;
    for _ in 2..1_024 {
        assert_eq!(call(MASK), (0, None));
    }
    assert_eq!(call(MASK), (6, Some("LOG_LIMIT_EXCEEDED".into())));
}

/// 确认安全点的运行取消优先级和截止错误沿 Fatal 原样交付。
#[test]
fn native_log_run_cancellation_precedes_deadline() {
    let cancel = std::env::temp_dir().join(format!("xiao-log-cancel-{}", std::process::id()));
    std::fs::write(&cancel, []).unwrap();
    let capture = Arc::new(Mutex::new(Capture::default()));
    let _entry = context(
        NativeControl {
            deadline_ns: Some(0),
            cancellation: Some(cancel.clone()),
        },
        &capture,
    );
    assert_eq!(call(MASK), (8, Some("X09-DRIVER-002".into())));
    std::fs::write(&cancel, b"1").unwrap();
    assert_eq!(call(MASK), (8, Some("X09-DRIVER-001".into())));
    std::fs::remove_file(cancel).unwrap();
    let capture = capture.lock().unwrap();
    assert!(capture.bytes.is_empty());
    assert_eq!(capture.flushes, 0);
}

/// 独立任务的取消从工作线程上下文进入日志安全点，使用可恢复状态。
#[test]
fn native_log_task_cancellation_keeps_run_quota() {
    let capture = Arc::new(Mutex::new(Capture::default()));
    let _entry = context(NativeControl::default(), &capture);
    let run = entry_environment();
    let (ready, ready_rx) = std::sync::mpsc::channel();
    let (go, go_rx) = std::sync::mpsc::channel();
    let runtime = crate::scheduler::RuntimeHandle::current();
    let worker = crate::scheduler::spawn_cooperative(&runtime, move |_| {
        let _guard = enter_environment(run);
        xiao_runtime_error_checkpoint_reset();
        ready.send(()).unwrap();
        go_rx.recv().unwrap();
        call(MASK)
    })
    .unwrap();
    ready_rx.recv().unwrap();
    worker.cancel();
    go.send(()).unwrap();
    assert_eq!(
        worker.join_interruptible(|| false).unwrap(),
        (6, Some("X06-RUNTIME-027".into()))
    );
    assert!(capture.lock().unwrap().bytes.is_empty());
    assert_eq!(call(MASK), (0, None));
    assert_eq!(capture.lock().unwrap().flushes, 1);
}

/// 同步宿主调用返回时复查真实的十秒操作期限，已交付字节不能回滚。
#[test]
#[ignore = "原生 ABI 专项使用真实十秒期限"]
fn native_log_operation_timeout_keeps_written_bytes() {
    let capture = Arc::new(Mutex::new(Capture::default()));
    capture.lock().unwrap().fault = Fault::Timeout;
    let _entry = context(NativeControl::default(), &capture);
    let started = Instant::now();
    assert_eq!(call(MASK), (6, Some("LOG_TIMEOUT".into())));
    assert!(started.elapsed() >= Duration::from_secs(10));
    let capture = capture.lock().unwrap();
    assert_eq!(capture.bytes.len(), RECORD_BYTES);
    assert_eq!(capture.flushes, 0);
}
