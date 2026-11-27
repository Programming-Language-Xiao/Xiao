//! B3-4C 的源码、运行入口与归档环境回归。

use std::fs;
use std::io::{Cursor, Write};
use std::process::{Command, Stdio};

use xiao_driver::{
    ArchiveBuildRequest, CORE_VERSION, CancellationToken, FrontendCompiler, FrontendRequest,
    PROTOCOL_VERSION, ProtocolRequest, ProtocolResponse, ProtocolTarget, RunEnvironment,
    RunOptions, SourceIdentity, build_archive, dispatch, read_frame, write_frame,
};
use xiao_intrinsics::{CAPABILITY_ENVIRONMENT as ENV, CAPABILITY_STDOUT as STDOUT};

const NAME: &str = "XIAO_B3_TEST_ENV";
const LOOKUP: &str =
    "from std.env import get\nfor value in get(\"XIAO_B3_TEST_ENV\")\n    print(value)\n";
const READ_WRITE: &str = r#"from std.env import get, set, unset
for value in get("XIAO_B3_TEST_ENV")
    print("unexpected<" + value + ">")
print("missing")
set(value = "", name = "XIAO_B3_TEST_ENV")
old = get(name = "XIAO_B3_TEST_ENV")
set("XIAO_B3_TEST_ENV", "updated")
for value in get("XIAO_B3_TEST_ENV")
    print(value)
print(unset("XIAO_B3_TEST_ENV"))
print(unset(name = "XIAO_B3_TEST_ENV"))
for value in old
    print("empty<" + value + ">")
for value in get("XIAO_B3_TEST_ENV")
    print("unexpected<" + value + ">")
print("missing")
"#;
const GOLDEN: &str = "missing\nupdated\ntrue\nfalse\nempty<>\nmissing\n";
const DROP_ENV: &str = r#"from std.env import set
[[Item]]
    def drop(self) -> none
        for value in os_env_get("XIAO_B3_TEST_ENV")
            print(value)
set("XIAO_B3_TEST_ENV", "seen-by-drop")
item = new Item()
"#;
const CANCEL: &str = r#"from std.concurrent import current_runtime, shared_mut, spawn, join, cancel, lock, lock_read, lock_write
from std.env import get, set
def read_flag(shared_mut<int> flag) -> int
    guard = lock(flag)
    return lock_read(guard)
def write_flag(shared_mut<int> flag, int value) -> none
    guard = lock(flag)
    lock_write(guard, value)
def worker(shared_mut<int> ready) -> int
    try
        set("XIAO_B3_TEST_ENV", "from-worker")
        write_flag(ready, 1)
        while read_flag(ready) == 1
            value = 1
    finally
        print("cancel-finally")
    return 0
rt = current_runtime()
ready = shared_mut(0)
thread = spawn(rt, worker, ready)
while read_flag(ready) == 0
    value = 1
for item in get("XIAO_B3_TEST_ENV")
    print(item)
cancel(thread)
join(thread)
"#;

#[path = "support/b3_env_native.rs"]
mod native;
#[path = "support/b3_env_artifact.rs"]
mod native_artifact;
use native_artifact::test_dir;

fn options(environment: Option<Vec<(String, String)>>, capabilities: u64) -> RunOptions {
    RunOptions {
        environment: environment.map(RunEnvironment),
        capabilities,
        ..RunOptions::default()
    }
}

fn request(source: &str, options: RunOptions) -> ProtocolRequest {
    ProtocolRequest::Run {
        request_id: "b3-env-source".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        locale: Some("en-US".to_owned()),
        target: ProtocolTarget::host(),
        optimization: Default::default(),
        source: SourceIdentity {
            module: "main".to_owned(),
            path: None,
            text: source.to_owned(),
        },
        options,
    }
}

fn stdout(response: ProtocolResponse) -> String {
    let ProtocolResponse::Result {
        exit_code, events, ..
    } = response
    else {
        panic!("{response:?}");
    };
    assert_eq!(exit_code, 0, "{events:?}");
    events
        .iter()
        .filter(|event| event.kind == "intrinsic_output")
        .map(|event| event.data["text"].as_str().unwrap())
        .collect()
}

fn rejected(response: ProtocolResponse, code: &str) {
    let ProtocolResponse::Error {
        exit_code,
        error,
        report,
        ..
    } = response
    else {
        panic!("{response:?}");
    };
    assert_eq!(exit_code, 2);
    assert_eq!(error.code, code, "{error:?}");
    assert!(report.is_none());
}

#[test]
fn vm_position_keyword_and_owned_environment_results() {
    for alias in ["std", "stdlib"] {
        let source = READ_WRITE.replace("std.env", &format!("{alias}.env"));
        let frontend = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(&source))
            .unwrap();
        assert_eq!(
            frontend
                .ir
                .modules
                .iter()
                .filter(|module| module.name == "stdlib.env")
                .count(),
            1
        );
        let response = dispatch(request(&source, options(Some(vec![]), ENV | STDOUT)));
        assert_eq!(stdout(response), GOLDEN);
        println!("环境 VM 导入 {alias}.env，位置和命名参数、空值及拥有结果一致");
    }
    let source = r#"import std.env as short
import stdlib.env as long
short.set("XIAO_B3_TEST_ENV", "dual")
for value in long.get("XIAO_B3_TEST_ENV")
    print(value)
"#;
    assert_eq!(
        stdout(dispatch(request(source, options(None, ENV | STDOUT)))),
        "dual\n"
    );
}

#[test]
fn vm_environment_errors_and_byte_limits() {
    let source = format!(
        r#"from std.env import get, set, unset
def check(str name, str value)
    try
        get(name)
        print("get")
    catch failure as Error
        print(error_code(failure))
    try
        set(name, value)
        print("set")
    catch failure as Error
        print(error_code(failure))
    try
        unset(name)
        print("unset")
    catch failure as Error
        print(error_code(failure))
check("{}", "{}")
check("{}", "")
check("{}", "{}")
check("name=bad", "")
"#,
        "é".repeat(128),
        "v".repeat(65_536),
        "é".repeat(128) + "b",
        "é".repeat(128),
        "v".repeat(65_537)
    );
    let full = concat!(
        "get\nset\nunset\n",
        "X06-RUNTIME-012\nENV_LIMIT_EXCEEDED\nX06-RUNTIME-012\n",
        "get\nENV_LIMIT_EXCEEDED\nunset\n",
        "X06-RUNTIME-012\nX06-RUNTIME-012\nX06-RUNTIME-012\n"
    );
    assert_eq!(
        stdout(dispatch(request(&source, options(None, ENV | STDOUT)))),
        full
    );
    assert_eq!(
        stdout(dispatch(request(&source, RunOptions::default()))),
        "X06-RUNTIME-026\n".repeat(12)
    );
    println!("环境 VM 校验 256/257 字节名称、65,536/65,537 字节值和默认权限");
}

#[test]
fn vm_snapshots_are_distinct_and_invalid_input_is_rejected_before_execution() {
    assert_eq!(
        stdout(dispatch(request(LOOKUP, options(None, ENV | STDOUT)))),
        ""
    );
    assert_eq!(
        stdout(dispatch(request(
            LOOKUP,
            options(Some(vec![]), ENV | STDOUT)
        ))),
        ""
    );
    let injected = options(Some(vec![(NAME.into(), "provided".into())]), ENV | STDOUT);
    assert_eq!(
        stdout(dispatch(request(LOOKUP, injected.clone()))),
        "provided\n"
    );
    let mutation = format!("from std.env import set\nset(\"{NAME}\", \"changed\")\n");
    assert_eq!(stdout(dispatch(request(&mutation, injected.clone()))), "");
    assert_eq!(stdout(dispatch(request(LOOKUP, injected))), "provided\n");
    let marker = "print(\"unreachable\")\n";
    for (pairs, code) in [
        (vec![("=C:".into(), "drive".into())], "X09-DRIVER-006"),
        (vec![("name".into(), "nul\0".into())], "X09-DRIVER-006"),
        (vec![("name".into(), "x".repeat(65_537))], "X09-DRIVER-007"),
        (
            (0..1025).map(|i| (i.to_string(), "".into())).collect(),
            "X09-DRIVER-007",
        ),
    ] {
        rejected(
            dispatch(request(marker, options(Some(pairs), ENV | STDOUT))),
            code,
        );
    }
    rejected(
        dispatch(request(marker, options(None, 0x100))),
        "X11-PROTOCOL-002",
    );
}

#[test]
fn vm_environment_is_shared_with_workers_and_uses_run_control() {
    let response = dispatch(request(CANCEL, options(None, ENV | STDOUT)));
    let ProtocolResponse::Result {
        exit_code,
        report,
        events,
        ..
    } = response
    else {
        panic!("{response:?}");
    };
    assert_eq!(exit_code, 3);
    assert_eq!(report.unwrap().code, "X06-RUNTIME-027");
    let output: String = events
        .iter()
        .filter(|event| event.kind == "intrinsic_output")
        .map(|event| event.data["text"].as_str().unwrap())
        .collect();
    assert_eq!(output, "from-worker\ncancel-finally\n");

    let mut expired = options(None, ENV | STDOUT);
    expired.timeout_ms = Some(0);
    rejected(dispatch(request(LOOKUP, expired)), "X09-DRIVER-002");
    let token = CancellationToken::new();
    token.cancel();
    let request = xiao_driver::DriverRequest::new(FrontendRequest::from_text(LOOKUP))
        .with_capabilities(ENV | STDOUT)
        .unwrap()
        .with_cancellation(token);
    let result = xiao_driver::FrontendVmDriver::new().run(&request);
    assert_eq!(result.code(), Some("X09-DRIVER-001"));
    println!("环境 VM 验证并发任务共享、任务取消 027、运行取消 001、运行截止 002");
}

#[test]
fn vm_table_drop_inherits_the_running_environment() {
    let output = stdout(dispatch(request(DROP_ENV, options(None, ENV | STDOUT))));
    assert_eq!(output, "seen-by-drop\n");
}

#[test]
fn vm_and_archive_keep_environment_as_a_run_input() {
    let root = test_dir();
    let main = root.join("main.xiao");
    fs::write(&main, LOOKUP).unwrap();
    let frontend = FrontendCompiler::new()
        .compile(&FrontendRequest::from_file(&main).unwrap())
        .unwrap();
    let program = xiao_bytecode::lower_program(&frontend.ir);
    let bytes = xiao_bytecode::encode_xiaoc(&program, Default::default()).unwrap();
    let xiaoc = root.join("main.xiaoc");
    fs::write(&xiaoc, &bytes).unwrap();
    let mut build = ArchiveBuildRequest::new(&main);
    build.cache_root = Some(root.join("cache"));
    build.output = Some(root.join("main.xar"));
    let archive = build_archive(&build, &CancellationToken::new()).unwrap();
    assert_eq!(archive.index.stdlib_modules, ["stdlib.env"]);
    assert_eq!(archive.index.required_capabilities, ENV | STDOUT);
    let xar_bytes = fs::read(&archive.output).unwrap();
    fs::remove_file(&main).unwrap();
    let xiaoc_request = |options| ProtocolRequest::RunXiaoc {
        request_id: "b3-env-xiaoc".into(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        locale: None,
        path: xiaoc.display().to_string(),
        options,
        diagnostics: None,
        debug: false,
    };
    let xar_request = |options| ProtocolRequest::RunArchive {
        request_id: "b3-env-xar".into(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        locale: None,
        path: archive.output.display().to_string(),
        options,
        diagnostics: None,
        debug: false,
    };
    assert_eq!(
        stdout(dispatch(xiaoc_request(options(None, ENV | STDOUT)))),
        ""
    );
    assert_eq!(
        stdout(dispatch(xar_request(options(None, ENV | STDOUT)))),
        ""
    );
    let provided = options(
        Some(vec![(NAME.into(), "archive-snapshot".into())]),
        ENV | STDOUT,
    );
    assert_eq!(
        stdout(dispatch(xiaoc_request(provided.clone()))),
        "archive-snapshot\n"
    );
    assert_eq!(
        stdout(dispatch(xar_request(provided))),
        "archive-snapshot\n"
    );
    let denied = dispatch(xar_request(options(None, STDOUT)));
    assert!(
        matches!(&denied, ProtocolResponse::Error { error, .. } if error.message.contains("environment"))
    );
    rejected(denied, "X06-RUNTIME-026");
    assert_eq!(fs::read(xiaoc).unwrap(), bytes);
    assert_eq!(fs::read(&archive.output).unwrap(), xar_bytes);
    println!("环境归档规范模块 stdlib.env，能力掩码 0xA0，运行输入不改变产物");
}

#[cfg(windows)]
fn invalid_os_value() -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt;
    std::ffi::OsString::from_wide(&[0xd800])
}

#[cfg(unix)]
fn invalid_os_value() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(vec![0xff])
}

fn cli_run(source: &str, options: RunOptions, invalid_host: bool) -> ProtocolResponse {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xiao-core"));
    let profile_key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    command
        .env_clear()
        .env(profile_key, std::env::var_os(profile_key).unwrap())
        .env(NAME, "host-snapshot")
        .env("XIAO_HOME", test_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if invalid_host {
        command.env("XIAO_B3_INVALID_ENCODING", invalid_os_value());
    }
    #[cfg(windows)]
    command.env("=C:", r"C:\work");
    let mut child = command.spawn().unwrap();
    let mut wire = Vec::new();
    write_frame(
        &mut wire,
        &ProtocolRequest::Hello {
            request_id: "b3-env-hello".into(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
        },
    )
    .unwrap();
    write_frame(&mut wire, &request(source, options)).unwrap();
    child.stdin.take().unwrap().write_all(&wire).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let mut reader = Cursor::new(output.stdout);
    let _: ProtocolResponse =
        serde_json::from_slice(&read_frame(&mut reader).unwrap().unwrap()).unwrap();
    serde_json::from_slice(&read_frame(&mut reader).unwrap().unwrap()).unwrap()
}

#[test]
fn cli_process_captures_only_authorized_omitted_environment() {
    assert_eq!(
        stdout(cli_run(LOOKUP, options(None, ENV | STDOUT), false)),
        "host-snapshot\n"
    );
    assert_eq!(
        stdout(cli_run(LOOKUP, options(Some(vec![]), ENV | STDOUT), true)),
        ""
    );
    let injected = options(Some(vec![(NAME.into(), "explicit".into())]), ENV | STDOUT);
    assert_eq!(stdout(cli_run(LOOKUP, injected, true)), "explicit\n");
    rejected(
        cli_run(LOOKUP, options(None, ENV | STDOUT), true),
        "X09-DRIVER-006",
    );
    let denied = r#"from std.env import get
try
    get("")
catch failure as Error
    print(error_code(failure))
"#;
    assert_eq!(
        stdout(cli_run(denied, options(None, STDOUT), true)),
        "X06-RUNTIME-026\n"
    );
    println!("环境 CLI 进程验证宿主快照、Windows 伪变量、显式绕过和无授权跳过");
}
