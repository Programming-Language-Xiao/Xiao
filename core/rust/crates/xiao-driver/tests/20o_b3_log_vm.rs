//! B3-5B 的标准库、TAC、字节码及生产 VM 日志回归。

use std::io::{Cursor, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use xiao_driver::{
    CORE_VERSION, CancellationToken, DriverOutcome, DriverRequest, FrontendCompiler,
    FrontendRequest, FrontendVmDriver, PROTOCOL_VERSION, ProtocolRequest, ProtocolResponse,
    ProtocolTarget, RunOptions, SourceIdentity, read_frame, write_frame,
};
use xiao_intrinsics::{CAPABILITY_CLOCK, CAPABILITY_STDOUT, DEFAULT_CAPABILITY_MASK};
use xiao_runtime::{DiagnosticParam, time::TimeContext};
use xiao_vm::VmEvent;

/// B3-0E 冻结的教程源码候选。
const SOURCE: &str = include_str!(
    "../../../../../docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/candidates/tests/fixtures/docs/log/tutorial.xiao"
);
/// 由实施规划固定的 103 字节黄金。
const GOLDEN: &[u8] = include_bytes!(
    "../../../../../docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/candidates/tests/fixtures/docs/log/tutorial.stdout"
);
/// 冻结用例的合成输入。
const INPUT: &str = include_str!(
    "../../../../../docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/candidates/tests/fixtures/docs/log/tutorial.input.json"
);
/// 两项日志能力的精确掩码。
const MASK: u64 = CAPABILITY_CLOCK | CAPABILITY_STDOUT;
/// 黄金使用的 UTC Unix 毫秒。
const FIXED: i64 = 946_684_800_000;
/// 固定墙钟对应的 24 字节时间戳。
const STAMP: &str = "2000-01-01T00:00:00.000Z";

/// 从生产结果原样提取 stdout。
fn stdout(events: &[VmEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// 通过源码生产入口使用固定墙钟和指定能力。
fn fixed_vm(source: &str, mask: u64) -> DriverOutcome {
    let request = DriverRequest::new(FrontendRequest::from_text(source))
        .with_capabilities(mask)
        .unwrap();
    TimeContext::fixed(FIXED, 0).with_vm_clock(|| FrontendVmDriver::new().run(&request))
}

#[test]
/// 双导入共享模块身份，按冻结输出逐字核对。
fn log_tutorial_matches_vm_golden() {
    let input: serde_json::Value = serde_json::from_str(INPUT).unwrap();
    assert_eq!(input["wall_clock_unix_ms"].as_i64(), Some(FIXED));
    assert_eq!(GOLDEN.len(), 103);
    for prefix in ["std", "stdlib"] {
        let source = SOURCE.replacen("from std.log", &format!("from {prefix}.log"), 1);
        let result = fixed_vm(&source, MASK);
        assert!(result.is_success(), "{prefix}: {result:?}");
        let execution = result.as_executed().unwrap();
        assert!(execution.report().is_none());
        assert_eq!(stdout(execution.events()).as_bytes(), GOLDEN);
        assert!(execution.events().iter().any(|event| matches!(event,
            VmEvent::ModuleLoaded { module } if module == "stdlib.log")));
        println!(
            "B3-5B VM alias={prefix} stdout_bytes={} exact=true",
            GOLDEN.len()
        );
    }
}

#[test]
/// 两项授权都必须具备，缺位诊断保留调用区间。
fn production_vm_checks_both_bits_and_keeps_the_intrinsic_call_span() {
    let source = "os_log_write(\"debug\", \"\", \"stdout\")\n";
    for mask in [MASK, DEFAULT_CAPABILITY_MASK] {
        let result = fixed_vm(source, mask);
        assert!(result.is_success(), "{result:?}");
        assert_eq!(
            stdout(result.as_executed().unwrap().events()),
            format!("{STAMP} DEBUG \n")
        );
    }
    for (mask, missing) in [
        (CAPABILITY_CLOCK, "stdout"),
        (CAPABILITY_STDOUT, "clock"),
        (0, "clock"),
    ] {
        let result = fixed_vm(source, mask);
        assert_eq!(result.code(), Some("X06-RUNTIME-026"), "{result:?}");
        let execution = result.as_executed().unwrap();
        let report = execution.report().unwrap();
        assert_eq!(
            report.context.get("capability"),
            Some(&DiagnosticParam::Text(missing.to_owned()))
        );
        let span = report.location.unwrap();
        assert_eq!(&source[span.start()..span.end()], source.trim_end());
        assert!(stdout(execution.events()).is_empty());
    }
}

#[test]
/// 63 经两种 TAC 宽度及产品容器执行，畸形 UTF-8 在解码时拒绝。
fn log_id_round_trips_through_both_widths_and_xiaoc() {
    use xiao_bytecode::{OperandWidth, encode_with_width};

    let source = "os_log_write(target = \"stdout\", message = \"🦊唯一字节\", level = \"info\")\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap();
    let tac = xiao_bytecode::lower_program(artifact.ir());
    assert!(
        tac.signatures
            .iter()
            .any(|signature| signature.intrinsic_id() == Some(63))
    );
    let expected = format!("{STAMP} INFO 🦊唯一字节\n");
    for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
        let encoded = encode_with_width(&tac, width).unwrap();
        let decoded = encoded.decode().unwrap();
        assert_eq!(decoded, tac);
        let outcome = TimeContext::fixed(FIXED, 0).with_vm_clock(|| {
            xiao_vm::run_request(&xiao_vm::RunRequest::new(artifact.ir(), &decoded))
        });
        assert!(outcome.result.is_success(), "{outcome:?}");
        assert_eq!(stdout(&outcome.events), expected);

        let mut invalid = encoded.bytes;
        let message = "🦊唯一字节".as_bytes();
        let start = invalid
            .windows(message.len())
            .position(|bytes| bytes == message)
            .unwrap();
        invalid[start] = 0xff;
        assert!(xiao_bytecode::decode(&invalid).is_err());
    }
    let bytes = xiao_bytecode::encode_xiaoc(&tac, Default::default()).unwrap();
    let outcome = TimeContext::fixed(FIXED, 0)
        .with_vm_clock(|| {
            xiao_vm::run_xiaoc_production_with_capabilities(
                &bytes,
                Default::default(),
                1,
                None,
                MASK,
            )
        })
        .unwrap();
    assert!(outcome.result.is_success(), "{outcome:?}");
    assert_eq!(stdout(&outcome.events), expected);
}

#[test]
/// 子任务共享额度，调试事件压力和会话复用不妨碍 stdout。
fn child_writes_use_one_run_quota_and_capture_even_with_one_debug_event() {
    let source = r#"from std.concurrent import current_runtime, spawn, join
def worker(str message) -> int
    os_log_write("warn", message, "stdout")
    return 0
def fill() -> none
    int i = 0
    while i < 1022
        os_log_write("info", "root", "stdout")
        i = i + 1
fill()
rt = current_runtime()
one = spawn(rt, worker, "one")
two = spawn(rt, worker, "two")
join(one)
join(two)
os_log_write("error", "over", "stdout")
"#;
    let request = DriverRequest::new(FrontendRequest::from_text(source))
        .with_capabilities(MASK)
        .unwrap()
        .with_event_capacity(1);
    let mut driver = FrontendVmDriver::new();
    for _ in 0..2 {
        let result = TimeContext::fixed(FIXED, 0).with_vm_clock(|| driver.run(&request));
        assert_eq!(
            result.code(),
            Some("LOG_LIMIT_EXCEEDED"),
            "{:?}",
            result.as_executed().and_then(|run| run.report())
        );
        let out = stdout(result.as_executed().unwrap().events());
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(lines.len(), 1024);
        let root = format!("{STAMP} INFO root");
        assert!(lines[..1022].iter().all(|line| *line == root));
        let mut children = lines[1022..].to_vec();
        children.sort_unstable();
        assert_eq!(
            children,
            [format!("{STAMP} WARN one"), format!("{STAMP} WARN two")]
        );
    }
}

#[test]
/// 用旧 intrinsic 作对照，核对释放、finally 和 catch 的日志顺序。
fn table_drop_uses_the_same_log_output_and_catch_finally_keep_release_order() {
    let source = r#"[[Item]]
    def drop(self) -> none
        os_log_write("debug", "drop", "stdout")
try
    item = new Item()
    os_log_write("warn", "invalid\nmessage", "stdout")
catch err as TypeError
    print(error_code(err))
finally
    os_log_write("info", "finally", "stdout")
"#;
    let result = fixed_vm(source, MASK);
    assert!(result.is_success(), "{result:?}");
    let out = stdout(result.as_executed().unwrap().events());
    let print_source = source
        .replace(
            "os_log_write(\"debug\", \"drop\", \"stdout\")",
            "print(\"drop\")",
        )
        .replace(
            "os_log_write(\"info\", \"finally\", \"stdout\")",
            "print(\"finally\")",
        )
        .replace(
            "os_log_write(\"warn\", \"invalid\\nmessage\", \"stdout\")",
            "chr(-1)",
        );
    let control = fixed_vm(&print_source, MASK);
    assert!(control.is_success(), "{control:?}");
    assert_eq!(
        stdout(control.as_executed().unwrap().events()),
        "finally\ndrop\nX06-RUNTIME-012\n"
    );
    assert_eq!(
        out,
        format!("{STAMP} INFO finally\n{STAMP} DEBUG drop\nX06-RUNTIME-012\n")
    );
}

#[test]
/// 用两个独立的进程管道核对协议 stdout 与宿主 stderr。
fn protocol_process_keeps_stdout_capture_apart_from_stderr() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_xiao-core"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut wire = Vec::new();
    write_frame(
        &mut wire,
        &ProtocolRequest::Hello {
            request_id: "b3-log-hello".into(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
        },
    )
    .unwrap();
    write_frame(
        &mut wire,
        &ProtocolRequest::Run {
            request_id: "b3-log-stdout".into(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            language_version: "0.1.0".into(),
            runtime_version: "0.1.0".into(),
            locale: None,
            target: ProtocolTarget::host(),
            optimization: Default::default(),
            source: SourceIdentity {
                module: "main".into(),
                path: None,
                text: "from std.log import write\nwrite(\"info\", \"管道\", \"stdout\")\n".into(),
            },
            options: RunOptions {
                capabilities: MASK,
                ..Default::default()
            },
        },
    )
    .unwrap();
    child.stdin.take().unwrap().write_all(&wire).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty(), "{:?}", output.stderr);
    let mut reader = Cursor::new(output.stdout);
    let _ = read_frame(&mut reader).unwrap().unwrap();
    let response: ProtocolResponse =
        serde_json::from_slice(&read_frame(&mut reader).unwrap().unwrap()).unwrap();
    let ProtocolResponse::Result {
        exit_code,
        events,
        report,
        ..
    } = response
    else {
        panic!("{response:?}");
    };
    assert_eq!(exit_code, 0);
    assert!(report.is_none());
    let record: String = events
        .iter()
        .filter(|event| event.kind == "intrinsic_output")
        .map(|event| event.data["text"].as_str().unwrap())
        .collect();
    assert_eq!(&record[24..], " INFO 管道\n");
    for index in 0..24 {
        match STAMP.as_bytes()[index] {
            marker @ (b'-' | b'T' | b':' | b'.' | b'Z') => {
                assert_eq!(record.as_bytes()[index], marker)
            }
            _ => assert!(record.as_bytes()[index].is_ascii_digit()),
        }
    }
}

#[test]
/// 同时提供到期和取消时，驱动先报告运行取消。
fn driver_control_uses_cancellation_before_an_expired_run() {
    let request = DriverRequest::new(FrontendRequest::from_text(
        "os_log_write(\"info\", \"\", \"stdout\")\n",
    ))
    .with_timeout(Duration::ZERO);
    assert_eq!(
        FrontendVmDriver::new().run(&request).code(),
        Some("X09-DRIVER-002")
    );
    let token = CancellationToken::new();
    token.cancel();
    let request = request.with_cancellation(token);
    assert_eq!(
        FrontendVmDriver::new().run(&request).code(),
        Some("X09-DRIVER-001")
    );
}
