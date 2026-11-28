//! 使用故障输出端实测 VM 的异常路径和输出边界。

use super::*;
use crate::{CancellationToken, RecordingSink, RunResult, StackCarrier, VmOptions};
use std::time::{Duration, Instant};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_runtime::current_cancellation_token;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 输出端可触发中断或错误的阶段。
enum Stage {
    Admit,
    Write,
    Retry,
    Flush,
}

/// 支持短写、阶段回调和独立字节捕获的测试输出端。
struct Writer {
    bytes: Arc<Mutex<Vec<u8>>>,
    /// 记录一次实际分块，测试可在写入和后续调用之间中断。
    first: Option<usize>,
    observe: Box<dyn FnMut(Stage) -> io::Result<()> + Send>,
}

impl Write for Writer {
    /// 在实际追加分块前通知测试，记录成功消费的字节。
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self.first.take().unwrap_or(bytes.len()).min(bytes.len());
        let mut recorded = self.bytes.lock().unwrap();
        let stage = if recorded.is_empty() {
            Stage::Write
        } else {
            Stage::Retry
        };
        (self.observe)(stage)?;
        recorded.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    /// 在刷新阶段交付测试故障。
    fn flush(&mut self) -> io::Result<()> {
        (self.observe)(Stage::Flush)
    }
}

impl LogOutput for Writer {
    /// 在 Runtime 计费和写入之前交付测试故障。
    fn admit(&mut self, _: usize) -> io::Result<()> {
        (self.observe)(Stage::Admit)
    }
}

/// 创建可装进 VM 的输出端和测试可读取的同一份字节捕获。
fn writer(
    first: Option<usize>,
    observe: impl FnMut(Stage) -> io::Result<()> + Send + 'static,
) -> (Writer, Arc<Mutex<Vec<u8>>>) {
    let bytes = Arc::new(Mutex::new(Vec::new()));
    (
        Writer {
            bytes: bytes.clone(),
            first,
            observe: Box::new(observe),
        },
        bytes,
    )
}

/// 通过前端与 TAC lowerer 编译真实调用。
fn program(source: &str) -> xiao_bytecode::TacProgram {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap();
    xiao_bytecode::lower_program(artifact.ir())
}

/// 共同的三字符串调用，消息用于检查诊断省略。
const CALL: &str = "os_log_write(\"info\", \"private\", \"stdout\")";
/// 真实宿主墙钟的 ASCII 数字位置及其余确定字节。
const TEMPLATE: &[u8] = b"0000-00-00T00:00:00.000Z INFO private\n";

/// 对照真实墙钟输出中已经写入的字节前缀。
fn assert_record_prefix(bytes: &[u8], count: usize) {
    assert_eq!(bytes.len(), count);
    for (&actual, &expected) in bytes.iter().zip(TEMPLATE) {
        if expected == b'0' {
            assert!(actual.is_ascii_digit());
        } else {
            assert_eq!(actual, expected);
        }
    }
}

#[test]
/// 四种输出拒绝保留阶段、调用区间和已写前缀。
fn vm_reports_admission_write_and_flush_failures_at_the_call_site() {
    let source = format!("{CALL}\n");
    let tac = program(&source);
    for (fault, partial, actual, label) in [
        (Stage::Admit, None, 0, "接纳"),
        (Stage::Write, None, 0, "写入"),
        (Stage::Retry, Some(7), 7, "写入"),
        (Stage::Flush, None, TEMPLATE.len(), "刷新"),
    ] {
        let (writer, bytes) = writer(partial, move |stage| {
            if stage == fault {
                Err(io::ErrorKind::BrokenPipe.into())
            } else {
                Ok(())
            }
        });
        let mut vm = Vm::<StackCarrier, _>::new(&tac, VmOptions::new(), RecordingSink::new());
        vm.set_log_output(writer);
        let RunResult::Error(error) = vm.run() else {
            panic!("输出必须失败 {fault:?}")
        };
        assert_eq!(error.code(), "LOG_OUTPUT_FAILED");
        assert_eq!(
            error.location(),
            xiao_source::SourceSpan::new(0, CALL.len())
        );
        assert_eq!(
            error.report().params.get("stage"),
            Some(&xiao_runtime::DiagnosticParam::Text(label.to_owned()))
        );
        assert_record_prefix(&bytes.lock().unwrap(), actual);
        assert!(!format!("{:?}", error.report()).contains("private"));
    }
}

#[test]
/// 根取消及较早截止在接纳后和短写后均执行释放与 finally。
fn vm_run_controls_leave_written_bytes_and_finish_cleanup() {
    let source = format!(
        "try\n    text = \"held\"\n    {CALL}\ncatch err as ResourceError\n    print(\"caught\")\nfinally\n    print(\"cleaned\")\n"
    );
    let tac = program(&source);
    for (stop, amount) in [(Stage::Admit, 0), (Stage::Write, 7)] {
        for expires in [false, true] {
            let token = CancellationToken::new();
            let deadline = expires.then(|| Instant::now() + Duration::from_millis(500));
            let mut control = CancellationSource::new().with_token(token.clone());
            if let Some(deadline) = deadline {
                control = control.with_deadline(deadline);
            }
            let (writer, bytes) = writer(Some(7), move |stage| {
                if stage == stop {
                    if let Some(deadline) = deadline {
                        std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
                    } else {
                        token.cancel();
                    }
                }
                Ok(())
            });
            let mut vm = Vm::<StackCarrier, _>::new(&tac, VmOptions::new(), RecordingSink::new());
            vm.set_cancellation_source(Some(control));
            vm.set_log_output(writer);
            assert!(matches!(vm.run(), RunResult::Cancelled));
            assert_record_prefix(&bytes.lock().unwrap(), amount);
            let events = vm.into_sink();
            assert_eq!(
                events.count_of(|event| matches!(event, VmEvent::IntrinsicOutput { text } if text == "cleaned\n")),
                1
            );
            assert_eq!(
                events.count_of(
                    |event| matches!(event, VmEvent::IntrinsicOutput { text } if text == "caught\n")
                ),
                0
            );
            assert!(events.count_of(|event| matches!(event, VmEvent::ValueReleased { .. })) > 0);
        }
    }
}

#[test]
/// 子 VM 中断保留已写字节，join 仍交付 027。
fn vm_task_cancellation_from_output_is_delivered_by_join() {
    let source = format!(
        r#"from std.concurrent import current_runtime, spawn, join
def worker() -> int
    try
        {CALL}
    finally
        print("child finally")
    return 0
child = spawn(current_runtime(), worker)
join(child)
"#
    );
    let tac = program(&source);
    for (stop, amount) in [(Stage::Admit, 0), (Stage::Write, 7)] {
        let (writer, bytes) = writer(Some(7), move |stage| {
            if stage == stop {
                current_cancellation_token().cancel();
            }
            Ok(())
        });
        let mut vm = Vm::<StackCarrier, _>::new(&tac, VmOptions::new(), RecordingSink::new());
        vm.set_log_output(writer);
        let RunResult::Error(error) = vm.run() else {
            panic!("join 应交付子任务取消")
        };
        assert_eq!(error.code(), "X06-RUNTIME-027");
        assert_record_prefix(&bytes.lock().unwrap(), amount);
        assert_eq!(vm.into_sink().count_of(|event|
            matches!(event, VmEvent::IntrinsicOutput { text } if text == "child finally\n")
        ), 1);
    }
}

#[test]
/// 关闭真实管道读端后，VM 把宿主错误报告为日志输出失败。
fn vm_reports_a_closed_pipe_as_output_failure() {
    /// 直接把宿主管道写端交给 VM。
    struct Pipe(std::io::PipeWriter);
    impl Write for Pipe {
        /// 交付宿主写入状态。
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            self.0.write(data)
        }
        /// 查询宿主刷新结果。
        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
    impl LogOutput for Pipe {
        /// 接纳管道写入请求，断管由宿主 write 报告。
        fn admit(&mut self, _: usize) -> io::Result<()> {
            Ok(())
        }
    }
    let (reader, write_end) = std::io::pipe().unwrap();
    drop(reader);
    let tac = program(&format!("{CALL}\n"));
    let mut vm = Vm::<StackCarrier, _>::new(&tac, VmOptions::new(), RecordingSink::new());
    vm.set_log_output(Pipe(write_end));
    let RunResult::Error(error) = vm.run() else {
        panic!("已关闭的管道应报告写入失败")
    };
    assert_eq!(error.code(), "LOG_OUTPUT_FAILED");
}
