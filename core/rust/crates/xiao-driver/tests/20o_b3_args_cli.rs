//! B3-2 的程序快照、分隔和冻结教程 VM 回归。

use std::fs;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use xiao_driver::{
    ArchiveBuildRequest, CORE_VERSION, CancellationToken, DriverOutcome, DriverRequest,
    FrontendCompiler, FrontendRequest, FrontendVmDriver, PROTOCOL_VERSION, ProtocolRequest,
    ProtocolResponse, ProtocolTarget, RunOptions, SourceIdentity, build_archive, dispatch,
    protocol::RunArguments,
};
use xiao_intrinsics::{CAPABILITY_PROCESS, CAPABILITY_STDOUT};
use xiao_runtime::arguments::ArgumentsSnapshot;
use xiao_vm::VmEvent;

/// 冻结的 args 教程。
const ARGS_SOURCE: &str = include_str!("../../../../../tests/fixtures/docs/args/tutorial.xiao");
/// 教程使用的合成快照。
const ARGS_INPUT: &str =
    include_str!("../../../../../tests/fixtures/docs/args/tutorial.input.json");
/// args 教程的独立黄金。
const ARGS_GOLDEN: &[u8] =
    include_bytes!("../../../../../tests/fixtures/docs/args/tutorial.stdout");
/// 冻结的 CLI 教程。
const CLI_SOURCE: &str = include_str!("../../../../../tests/fixtures/docs/cli/tutorial.xiao");
/// CLI 教程的独立黄金。
const CLI_GOLDEN: &[u8] = include_bytes!("../../../../../tests/fixtures/docs/cli/tutorial.stdout");

#[cfg(feature = "args-cli-native-test")]
#[path = "support/b3_args_cli_native.rs"]
mod native;

/// 从 VM 事件取得程序标准输出。
fn stdout(result: &DriverOutcome) -> String {
    result
        .as_executed()
        .unwrap()
        .events()
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// 核对同一协议响应形状下的退出码及原始输出。
fn protocol_stdout(response: ProtocolResponse) -> String {
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
        .filter_map(|event| event.data.get("text").and_then(serde_json::Value::as_str))
        .collect()
}

/// 通过实际协议帧编解码使用冻结的请求形状。
fn request_frame(request: ProtocolRequest) -> ProtocolResponse {
    let frame = xiao_driver::encode_frame(&request).unwrap();
    dispatch(xiao_driver::decode_frame(&frame[xiao_driver::FRAME_LENGTH_BYTES..]).unwrap())
}

/// 构造与 CLI 生产路径一致的源码请求。
fn source_request(options: RunOptions) -> ProtocolRequest {
    ProtocolRequest::Run {
        request_id: "b3-2-source".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        locale: None,
        target: ProtocolTarget::host(),
        optimization: Default::default(),
        source: SourceIdentity {
            module: "main".to_owned(),
            path: None,
            text: ARGS_SOURCE.to_owned(),
        },
        options,
    }
}

#[test]
/// 两个模块在两个导入别名下均读取正式夹具的独立黄金。
fn both_import_paths_match_the_vm_tutorials() {
    let input: RunArguments = serde_json::from_str(ARGS_INPUT).unwrap();
    let snapshot = Arc::new(ArgumentsSnapshot::try_from(input).unwrap());
    for (module, source, expected) in [
        ("args", ARGS_SOURCE, ARGS_GOLDEN),
        ("cli", CLI_SOURCE, CLI_GOLDEN),
    ] {
        for prefix in ["std", "stdlib"] {
            let source = source.replacen("from std.", &format!("from {prefix}."), 1);
            let mut request = DriverRequest::new(FrontendRequest::from_text(source))
                .with_capabilities(
                    CAPABILITY_STDOUT
                        | if module == "args" {
                            CAPABILITY_PROCESS
                        } else {
                            0
                        },
                )
                .unwrap();
            if module == "args" {
                request = request.with_arguments_snapshot(Arc::clone(&snapshot));
            }
            let result = FrontendVmDriver::new().run(&request);
            assert!(result.is_success(), "{prefix}.{module}: {result:?}");
            assert_eq!(stdout(&result).as_bytes(), expected, "{prefix}.{module}");
            println!("B3-2-VM module={module} alias={prefix} exact=true");
        }
    }
}

#[test]
/// 子 VM 使用父运行的快照，重用驱动器时重新注入。
fn args_snapshot_is_shared_with_children_and_scoped_to_one_run() {
    let source = r#"from std.args import program_name, argv
from std.concurrent import current_runtime, spawn, join
def child() -> str
    child_args = argv()
    result = program_name()
    for item in child_args
        result = result + "[" + item + "]"
    array_push(child_args, "child-only")
    return result
job = spawn(current_runtime(), child)
print("child=" + join(job))
print("main=" + child())
"#;
    let mut driver = FrontendVmDriver::new();
    let request = DriverRequest::new(FrontendRequest::from_text(source));
    let populated = request.clone().with_arguments_snapshot(Arc::new(
        ArgumentsSnapshot::new("demo.xiao".into(), vec!["".into(), "�".into(), "--".into()])
            .unwrap(),
    ));
    let result = driver.run(&populated);
    assert!(result.is_success(), "{result:?}");
    assert_eq!(
        stdout(&result),
        "child=demo.xiao[][�][--]\nmain=demo.xiao[][�][--]\n"
    );
    let result = driver.run(&request);
    assert!(result.is_success(), "{result:?}");
    assert_eq!(stdout(&result), "child=\nmain=\n");

    for call in ["program_name()", "argv()"] {
        let request = DriverRequest::new(FrontendRequest::from_text(format!(
            "from std.args import program_name, argv\n{call}\n"
        )))
        .with_capabilities(0)
        .unwrap();
        let result = driver.run(&request);
        assert_eq!(result.code(), Some("X06-RUNTIME-026"), "{result:?}");
    }
}

#[test]
/// 空组可独立修改，合法的 NUL 保留在第二组。
fn cli_returns_two_fresh_groups_and_keeps_nul() {
    let source = r#"from std.cli import split_terminator
source = ["--", chr(0), "--", ""]
groups = split_terminator(source)
array_push(groups[0], "local")
print(groups[0][0])
print(groups[1][0] == chr(0))
print(groups[1][1])
print("[" + groups[1][2] + "]")
print(source[0])
for group in split_terminator([])
    print("empty")
for group in split_terminator(["head", "--"])
    print("group")
    for item in group
        print(item)
"#;
    let result =
        FrontendVmDriver::new().run(&DriverRequest::new(FrontendRequest::from_text(source)));
    assert!(result.is_success(), "{result:?}");
    assert_eq!(
        stdout(&result),
        "local\ntrue\n--\n[]\n--\nempty\nempty\ngroup\nhead\ngroup\n"
    );
}

#[test]
/// 两个导入别名拒绝形状错误，动态值按实际数组及元素类型检查。
fn cli_checks_static_and_dynamic_argument_types() {
    for prefix in ["std", "stdlib"] {
        let import = format!("from {prefix}.cli import split_terminator");
        let valid = FrontendRequest::from_text(format!("{import}\nsplit_terminator([])\n"));
        let artifact = FrontendCompiler::new().compile(&valid).unwrap();
        let target = xiao_codegen_llvm::TargetDescription::host();
        let options = xiao_codegen_llvm::CodegenOptions::for_target(target);
        let module = xiao_codegen_llvm::lower_program(&artifact.ir, &options).unwrap();
        assert!(module.text.contains("array_type"), "{prefix}");
        for call in [
            "split_terminator(\"bad\")",
            "split_terminator([[\"bad\"]])",
            "split_terminator()",
            "split_terminator(other = [\"ok\"])",
        ] {
            let request = FrontendRequest::from_text(format!("{import}\n{call}\n"));
            let error = FrontendCompiler::new().compile(&request).unwrap_err();
            assert!(
                error
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code() == "X04-TYPE-002"),
                "{prefix} {call}: {error:?}"
            );
        }
        let request = DriverRequest::new(FrontendRequest::from_text(cli_dynamic_types(&import)));
        let result = FrontendVmDriver::new().run(&request);
        assert!(result.is_success(), "{prefix}: {result:?}");
        assert_eq!(
            stdout(&result),
            "X06-RUNTIME-002\nX06-RUNTIME-002\nok\n",
            "{prefix}"
        );
    }
}

/// VM 和原生使用同一组动态形状。
fn cli_dynamic_types(import: &str) -> String {
    format!(
        "{import}\n{}",
        r#"for item in ["bad", [["bad"]], ["ok"]]
    try
        groups = split_terminator(values = item)
        print(groups[0][0])
    catch failure as TypeError
        print(error_code(failure))
"#,
    )
}

#[test]
/// 用 Unicode 字符精确触及单项和总字节上限。
fn cli_limits_count_and_utf8_bytes_before_grouping() {
    let result = FrontendVmDriver::new().run(&DriverRequest::new(FrontendRequest::from_text(
        cli_limits_source(),
    )));
    assert!(result.is_success(), "{result:?}");
    assert_eq!(stdout(&result), CLI_LIMITS_GOLDEN);
}

const CLI_LIMITS_GOLDEN: &str =
    "ok\nCLI_LIMIT_EXCEEDED\nok\nCLI_LIMIT_EXCEEDED\nok\nCLI_LIMIT_EXCEEDED\n";

/// 两种后端以同一语料验证三项上限与等于上限的情况。
fn cli_limits_source() -> String {
    let prefix = r#"from stdlib.cli import split_terminator
def check([str] values)
    try
        split_terminator(values)
        print("ok")
    catch failure as ResourceError
        print(error_code(failure))
"#;
    let full = "🧪".repeat(1024);
    format!(
        "{prefix}\nstr items = []\ni = 0\nwhile i < 1024\n    array_push(items, \"\")\n    i = i + 1\ncheck(items)\narray_push(items, \"\")\ncheck(items)\ncheck([\"{full}\"])\ncheck([\"{full}a\"])\nstr bytes = []\ni = 0\nwhile i < 16\n    array_push(bytes, \"{full}\")\n    i = i + 1\ncheck(bytes)\narray_push(bytes, \"a\")\ncheck(bytes)\n"
    )
}

#[test]
/// 同一套参数快照可从源码、已编码字节码与删源归档传入。
fn protocol_replays_frozen_args_from_all_three_entries() {
    let hello = request_frame(ProtocolRequest::Hello {
        request_id: "b3-2-hello".to_owned(),
        protocol_version: 1,
        core_version: 1,
    });
    let ProtocolResponse::Hello {
        accepted,
        capabilities,
        ..
    } = hello
    else {
        panic!("{hello:?}");
    };
    assert!(accepted && capabilities.iter().any(|name| name == "arguments"));

    let populated = RunOptions {
        arguments: Some(serde_json::from_str(ARGS_INPUT).unwrap()),
        ..Default::default()
    };
    let empty = RunOptions {
        arguments: None,
        ..Default::default()
    };
    assert!(
        serde_json::to_value(&empty)
            .unwrap()
            .get("arguments")
            .is_none()
    );
    assert_eq!(
        protocol_stdout(request_frame(source_request(populated.clone()))).as_bytes(),
        ARGS_GOLDEN
    );
    assert_eq!(
        protocol_stdout(request_frame(source_request(empty.clone()))),
        "program=\n"
    );

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("xiao-b3-2-replay-{}-{unique}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let input = root.join("main.xiao");
    let xiaoc = root.join("main.xiaoc");
    fs::write(&input, ARGS_SOURCE).unwrap();
    let source = FrontendRequest::from_file(&input).unwrap();
    let artifact = FrontendCompiler::new().compile(&source).unwrap();
    let program = xiao_bytecode::lower_program(&artifact.ir);
    let bytes = xiao_bytecode::encode_xiaoc(&program, Default::default()).unwrap();
    fs::write(&xiaoc, &bytes).unwrap();
    let mut archive = ArchiveBuildRequest::new(&input);
    archive.cache_root = Some(root.join("cache"));
    archive.output = Some(root.join("main.xar"));
    let built = build_archive(&archive, &CancellationToken::new()).unwrap();
    let archive_bytes = fs::read(&built.output).unwrap();
    assert!(
        built
            .index
            .stdlib_modules
            .contains(&"stdlib.args".to_owned())
    );
    fs::remove_file(input).unwrap();
    for (options, golden) in [(populated, ARGS_GOLDEN), (empty, b"program=\n".as_slice())] {
        let bytecode = request_frame(ProtocolRequest::RunXiaoc {
            request_id: "b3-2-xiaoc".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            locale: None,
            path: xiaoc.display().to_string(),
            options: options.clone(),
            diagnostics: None,
            debug: false,
        });
        let archive = request_frame(ProtocolRequest::RunArchive {
            request_id: "b3-2-xar".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            locale: None,
            path: built.output.display().to_string(),
            options,
            diagnostics: None,
            debug: false,
        });
        assert_eq!(protocol_stdout(bytecode).as_bytes(), golden);
        assert_eq!(protocol_stdout(archive).as_bytes(), golden);
    }
    assert_eq!(fs::read(xiaoc).unwrap(), bytes);
    assert_eq!(fs::read(built.output).unwrap(), archive_bytes);
    // 成功后只清理本测试以随机后缀创建的目录。
    fs::remove_dir_all(root).unwrap();
}

#[test]
/// 快照非法与超限都在执行之前返回，错误中不带参数文本。
fn protocol_rejects_invalid_and_oversized_arguments_without_echoing_them() {
    const PRIVATE: &str = "private-marker";
    for (program_name, argv, code) in [
        (format!("{PRIVATE}\0"), vec![], "X09-DRIVER-004"),
        (
            format!("{PRIVATE}\0"),
            vec![String::new(); 1025],
            "X09-DRIVER-005",
        ),
        (PRIVATE.repeat(1024), vec![], "X09-DRIVER-005"),
    ] {
        let request = source_request(RunOptions {
            arguments: Some(RunArguments { program_name, argv }),
            ..Default::default()
        });
        assert!(!format!("{request:?}").contains(PRIVATE));
        let response = request_frame(request);
        assert!(!format!("{response:?}").contains(PRIVATE));
        let ProtocolResponse::Error {
            error, exit_code, ..
        } = response
        else {
            panic!("{response:?}");
        };
        assert_eq!(exit_code, 2);
        assert_eq!(error.code, code);
    }
}
