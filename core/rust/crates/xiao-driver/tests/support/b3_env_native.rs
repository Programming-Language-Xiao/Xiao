//! env 原生专项共用的工具链、构建入口和运行控制辅助。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use xiao_artifacts::Digest256;
use xiao_codegen_llvm::{
    ArtifactVerification, CODEGEN_VERSION, CodegenOptions, NativeArtifact, NativeRun,
    NativeRunResult, NativeRuntimeSupport, TargetDescription, Toolchain,
};
use xiao_driver::{
    CancellationToken, FrontendNativeDriver, FrontendRequest, NativeBuildRequest, ProtocolResponse,
    RunControl, dispatch,
};
use xiao_intrinsics::CAPABILITY_PROCESS;
use xiao_runtime_abi::native_control::{CANCEL_ENV, DEADLINE_ENV, monotonic_ns};
use xiao_runtime_abi::{NATIVE_ENVIRONMENT_NONCE_ENV, NATIVE_ENVIRONMENT_STDIN_PREFIX};

use super::native_artifact::{record, run_injected, toolchain};
use super::{
    CANCEL, DROP_ENV, ENV, GOLDEN, LOOKUP, NAME, READ_WRITE, STDOUT, options, request, stdout,
    test_dir,
};

/// 旧版原生环境注入的固定首参标记。
const LEGACY_MARKER: &str = "--__xiao-run-environment-stdin";
/// 打印全部应用参数的源码。
const PRINT_ARGV: &str = "from std.args import argv\nfor value in argv()\n    print(value)\n";

/// 构造固定目标、优化档和能力掩码的原生构建请求。
fn build_request(
    root: &Path,
    case: &str,
    source: &str,
    target: &TargetDescription,
    tools: &Toolchain,
    optimization: u8,
    capabilities: u64,
) -> NativeBuildRequest {
    let path = root.join(format!("{case}{}", std::env::consts::EXE_SUFFIX));
    NativeBuildRequest::new(
        FrontendRequest::from_text(source),
        target.clone(),
        tools.clone(),
        path,
    )
    .with_codegen_options(
        CodegenOptions::for_target(target.clone())
            .with_optimization_level(optimization)
            .unwrap()
            .with_capabilities(capabilities)
            .unwrap(),
    )
    .with_llvm_ir_output(root.join(format!("{case}.ll")))
}

/// 运行原生产物并保存 stdout、stderr 和可选释放轨迹。
fn run_artifact(
    artifact: &NativeArtifact,
    case: &str,
    trace: bool,
    invalid_host: bool,
    injected: Option<&[u8]>,
) -> Output {
    let mut command = Command::new(&artifact.executable);
    command.env_clear().env(NAME, "host-native");
    let root = artifact.executable.parent().unwrap();
    if trace {
        command.env(
            "XIAO_RUNTIME_RELEASE_TRACE_PATH",
            root.join(format!("{case}.release.tsv")),
        );
    }
    if invalid_host {
        command.env("XIAO_B3_TEST_INVALID_ENCODING", super::invalid_os_value());
    }
    #[cfg(windows)]
    command.env("=C:", r"C:\work");
    let output = if let Some(bytes) = injected {
        run_injected(&mut command, bytes)
    } else {
        command.output().unwrap()
    };
    fs::write(root.join(format!("{case}.stdout")), &output.stdout).unwrap();
    fs::write(root.join(format!("{case}.stderr")), &output.stderr).unwrap();
    println!(
        "环境原生 {case} 退出 {:?}，输出 {} 字节，诊断 {} 字节",
        output.status.code(),
        output.stdout.len(),
        output.stderr.len()
    );
    output
}

/// 断言释放轨迹中每个对象只销毁一次。
fn assert_released(path: &Path) {
    let trace = fs::read_to_string(path).unwrap();
    let mut objects = BTreeMap::<u64, usize>::new();
    for (sequence, line) in trace.lines().enumerate() {
        let mut fields = line.split('\t');
        assert_eq!(fields.next().unwrap().parse::<usize>().unwrap(), sequence);
        let object = fields.next().unwrap().parse::<u64>().unwrap();
        let action = fields.next().unwrap();
        assert!(fields.next().is_none());
        if matches!(action, "strong_release" | "destroy") {
            let destroyed = objects.entry(object).or_default();
            assert_eq!(*destroyed, 0, "重复释放 {object}: {line}");
            *destroyed += usize::from(action == "destroy");
        }
    }
    assert!(!objects.is_empty(), "{path:?}");
    assert!(
        objects.values().all(|&destroyed| destroyed == 1),
        "{path:?}"
    );
    println!(
        "环境原生释放追踪 {:?}，{} 个对象各销毁一次",
        path.file_name(),
        objects.len()
    );
}

#[test]
#[ignore = "需要本批 release Runtime 和固定 LLVM 工具"]
/// 验证原生 O0/O3 与 VM 一致并正确释放输出。
fn native_env_o0_o3_matches_vm_and_owns_outputs() {
    let target = TargetDescription::host();
    let tools = toolchain(&target);
    let root = test_dir();
    println!(
        "环境原生任务目录 {}，目标 {}",
        root.display(),
        target.triple
    );
    let driver = FrontendNativeDriver::new();
    for optimization in [0, 3] {
        let alias = if optimization == 0 { "std" } else { "stdlib" };
        let source = READ_WRITE.replace("std.env", &format!("{alias}.env"));
        let run = options(Some(vec![]), ENV | STDOUT);
        let expected = stdout(dispatch(request(&source, run.clone())));
        assert_eq!(expected, GOLDEN);
        let case = format!("operations-o{optimization}");
        let build = build_request(
            &root,
            &case,
            &source,
            &target,
            &tools,
            optimization,
            run.capabilities,
        );
        let result = driver.build(&build).unwrap();
        let composition = &result.native.artifact_runtime;
        assert_eq!(composition.verification, ArtifactVerification::Complete);
        assert_eq!(
            composition.declared_components,
            composition.observed_components
        );
        assert!(composition.unclassified_runtime_symbols.is_empty());
        assert!(
            composition
                .observed_components
                .iter()
                .any(|component| component == "env")
        );
        let metadata = result.native.native_metadata.as_ref().unwrap();
        assert_eq!(metadata.generator_version, CODEGEN_VERSION);
        assert_eq!(metadata.stdlib.stdlib_interface_version, 1);
        assert_eq!(
            metadata.runtime_abi_min,
            xiao_runtime_abi::ABI_ENCODED_VERSION
        );
        let old = NativeRuntimeSupport {
            generator_version: 26,
            ..Default::default()
        };
        assert_eq!(
            metadata.validate(&target, &old, 0).unwrap_err().code,
            "X17-XAR-007"
        );
        let output = run_artifact(&result.native, &case, true, true, Some(b"[]"));
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert_eq!(output.stdout, expected.as_bytes());
        assert!(output.stderr.is_empty());
        assert_released(&root.join(format!("{case}.release.tsv")));
        println!(
            "环境生成身份 {} 工具指纹 {} 声明及观察组件 {:?} COFF {:?}",
            result.native.module.codegen_fingerprint,
            result.native.toolchain_fingerprint,
            composition.declared_components,
            composition.coff_link_identity
        );

        let invalid = "from std.env import get\nget(\"invalid=name\")\n";
        let limit = format!(
            "from std.env import set\nset(\"{}\", \"\")\n",
            "x".repeat(257)
        );
        for (label, source, mask, code) in [
            ("denied", invalid, STDOUT, "X06-RUNTIME-026"),
            ("invalid", invalid, ENV | STDOUT, "X06-RUNTIME-012"),
            ("limit", limit.as_str(), ENV | STDOUT, "ENV_LIMIT_EXCEEDED"),
        ] {
            let run = options(None, mask);
            let ProtocolResponse::Result {
                exit_code, report, ..
            } = dispatch(request(source, run))
            else {
                panic!("VM 应返回环境运行错误");
            };
            assert_eq!(exit_code, 3);
            assert_eq!(report.unwrap().code, code);
            let case = format!("{label}-o{optimization}");
            let build = build_request(&root, &case, source, &target, &tools, optimization, mask);
            let result = driver.build(&build).unwrap();
            let output = run_artifact(&result.native, &case, true, mask == STDOUT, None);
            if label == "denied" {
                record(&case, source, &build, &result.native, &output);
            }
            assert_eq!(
                output.status.code(),
                Some(i32::from(exit_code)),
                "{output:?}"
            );
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8(output.stderr).unwrap().contains(code));
            assert_released(&root.join(format!("{case}.release.tsv")));
        }
    }
}

#[test]
#[ignore = "需要本批 release Runtime 和固定 LLVM 工具"]
/// 验证宿主捕获、显式注入和边界输入。
fn native_env_entry_resolves_host_and_explicit_inputs() {
    let target = TargetDescription::host();
    let tools = toolchain(&target);
    let root = test_dir();
    println!("环境原生启动任务目录 {}", root.display());
    let driver = FrontendNativeDriver::new();
    let build = build_request(&root, "entry", LOOKUP, &target, &tools, 0, ENV | STDOUT);
    let artifact = driver.build(&build).unwrap().native;
    let output = run_artifact(&artifact, "direct-valid", false, false, None);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"host-native\n");
    let output = run_artifact(&artifact, "direct-invalid", false, true, None);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("X09-DRIVER-006")
    );

    let data = serde_json::to_vec(&vec![(NAME, "explicit-native")]).unwrap();
    let output = run_artifact(&artifact, "injected", false, true, Some(&data));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"explicit-native\n");
    let mut over = Vec::new();
    serde_json::to_writer(&mut over, &vec![(NAME, "x".repeat(65_537))]).unwrap();
    let output = run_artifact(&artifact, "injected-limit", false, false, Some(&over));
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("X09-DRIVER-007")
    );

    let mut pairs = vec![(NAME, "boundary-native".to_owned())];
    let mut remaining = 1_048_576 - NAME.len() - pairs[0].1.len();
    for name in [
        "E0000", "E0001", "E0002", "E0003", "E0004", "E0005", "E0006", "E0007", "E0008", "E0009",
        "E0010", "E0011", "E0012", "E0013", "E0014", "E0015",
    ] {
        let bytes = remaining.min(65_536);
        pairs.push((name, "\u{1}".repeat(bytes - name.len())));
        remaining -= bytes;
    }
    assert_eq!(remaining, 0);
    let mut data = serde_json::to_vec(&pairs).unwrap();
    let accepted = 6 * 1_048_576 + 8 * 1_024 + 2;
    assert!(data.len() <= accepted);
    data.resize(accepted, b' ');
    let output = run_artifact(
        &artifact,
        "injected-raw-boundary",
        false,
        false,
        Some(&data),
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"boundary-native\n");
    assert!(output.stderr.is_empty());
    data.push(b' ');
    let output = run_artifact(&artifact, "injected-raw-over", false, false, Some(&data));
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("X09-DRIVER-007:")
    );

    let build = build_request(&root, "static", "result = 1\n", &target, &tools, 0, ENV);
    let artifact = driver.build(&build).unwrap().native;
    assert!(artifact.module.text.starts_with("; Xiao N0-A LLVM module"));
    let output = run_artifact(&artifact, "static-invalid-host", false, true, None);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("X09-DRIVER-006")
    );
    let output = run_artifact(&artifact, "static-injected-empty", false, true, Some(b"[]"));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty() && output.stderr.is_empty());

    let build = build_request(&root, "drop", DROP_ENV, &target, &tools, 0, ENV | STDOUT);
    let artifact = driver.build(&build).unwrap().native;
    let output = run_artifact(&artifact, "drop", false, true, Some(b"[]"));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"seen-by-drop\n");
}

#[test]
#[ignore = "需要本批 release Runtime 和固定 LLVM 工具"]
/// 验证旧注入标记作为普通首参不被吞掉。
fn native_env_preserves_legacy_marker_as_first_argument() {
    let target = TargetDescription::host();
    let tools = toolchain(&target);
    let root = test_dir();
    let source = format!("{LOOKUP}{PRINT_ARGV}");
    let mask = ENV | STDOUT | CAPABILITY_PROCESS;
    let build = build_request(&root, "legacy-marker", &source, &target, &tools, 0, mask);
    let artifact = FrontendNativeDriver::new().build(&build).unwrap().native;

    let mismatched = format!("{NATIVE_ENVIRONMENT_STDIN_PREFIX}00000000000000000000000000000000");
    for argument in [LEGACY_MARKER, &mismatched] {
        let output = Command::new(&artifact.executable)
            .env_clear()
            .env(NAME, "host-native")
            .env(
                NATIVE_ENVIRONMENT_NONCE_ENV,
                "0123456789abcdef0123456789abcdef",
            )
            .args([argument, "tail"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert_eq!(
            output.stdout,
            format!("host-native\n{argument}\ntail\n").as_bytes()
        );
        assert!(output.stderr.is_empty());
        println!("{} 首参 {argument:?} 退出 0", artifact.executable.display());
    }
}

#[test]
#[ignore = "需要本批 release Runtime 和固定 LLVM 工具"]
/// 验证驱动随机 nonce 注入显式快照。
fn native_env_driver_injects_snapshot_with_random_nonce() {
    let target = TargetDescription::host();
    let tools = toolchain(&target);
    let root = test_dir();
    let source = format!("{LOOKUP}{PRINT_ARGV}");
    let mask = ENV | STDOUT | CAPABILITY_PROCESS;
    let build = build_request(&root, "managed", &source, &target, &tools, 0, mask);
    let input = options(Some(vec![(NAME.into(), "managed-native".into())]), mask);
    let output = FrontendNativeDriver::new()
        .run_application(&build, &input, [LEGACY_MARKER, "tail"])
        .unwrap();
    assert_eq!(output.status, Some(0), "{output:?}");
    assert_eq!(
        output.stdout,
        format!("managed-native\n{LEGACY_MARKER}\ntail\n")
    );
    assert!(output.stderr.is_empty());
    println!("驱动随机数注入退出 0，输出 {:?}", output.stdout);
}

#[test]
#[ignore = "需要本批 release Runtime 和固定 LLVM 工具"]
/// 验证原生任务共享快照且取消行为与 VM 一致。
fn native_env_worker_shares_snapshot_and_cancels_like_vm() {
    let target = TargetDescription::host();
    let tools = toolchain(&target);
    let root = test_dir();
    println!("环境原生并发任务目录 {}", root.display());
    let mask = ENV | STDOUT;
    let ProtocolResponse::Result {
        exit_code,
        report,
        events,
        ..
    } = dispatch(request(CANCEL, options(None, mask)))
    else {
        panic!("VM 应返回取消故障");
    };
    assert_eq!(exit_code, 3);
    let code = report.unwrap().code;
    assert_eq!(code, "X06-RUNTIME-027");
    let expected: String = events
        .iter()
        .filter(|event| event.kind == "intrinsic_output")
        .map(|event| event.data["text"].as_str().unwrap())
        .collect();
    assert_eq!(expected, "from-worker\ncancel-finally\n");
    for optimization in [0, 3] {
        let case = format!("cancel-o{optimization}");
        let build = build_request(&root, &case, CANCEL, &target, &tools, optimization, mask);
        let artifact = FrontendNativeDriver::new().build(&build).unwrap().native;
        let output = run_artifact(&artifact, &case, false, true, Some(b"[]"));
        assert_eq!(
            output.status.code(),
            Some(i32::from(exit_code)),
            "{output:?}"
        );
        assert_eq!(output.stdout, expected.as_bytes());
        assert!(String::from_utf8(output.stderr).unwrap().contains(&code));
    }
}

/// 断言驱动中断回执的退出码、诊断和标准输出。
fn assert_driver_interruption(
    root: &Path,
    case: &str,
    output: &NativeRunResult,
    code: &str,
    stdout: &str,
) {
    fs::write(root.join(format!("{case}.stdout")), &output.stdout).unwrap();
    fs::write(root.join(format!("{case}.stderr")), &output.stderr).unwrap();
    println!(
        "B3-4-ENV-CONTROL {}",
        serde_json::json!({
            "case": case, "task": root, "exit": output.status, "diagnostic": code,
            "stdout_bytes": output.stdout.len(), "stderr_bytes": output.stderr.len(),
        })
    );
    assert_eq!(output.status, Some(2), "{output:?}");
    assert_eq!(output.stdout, stdout);
    assert!(output.stderr.contains(code), "{output:?}");
    assert!(!output.stderr.contains("X06-RUNTIME-027"), "{output:?}");
}

/// 验证构建前和 spawn 前的取消/截止拒绝路径。
#[test]
#[ignore = "需要本批 release Runtime 和固定 LLVM 工具"]
fn native_env_run_control_before_build_and_spawn() {
    let target = TargetDescription::host();
    let tools = toolchain(&target);
    let driver = FrontendNativeDriver::new();
    for optimization in [0, 3] {
        let root = test_dir();
        let case = format!("never-start-o{optimization}");
        let mask = ENV | STDOUT;
        let build = build_request(
            &root,
            &case,
            "print(\"user-code\")\n",
            &target,
            &tools,
            optimization,
            mask,
        );
        let token = CancellationToken::new();
        token.cancel();
        let run = options(None, mask);
        let result = driver
            .run_application_with_control(
                &build,
                &run,
                [] as [&str; 0],
                &RunControl::new().with_cancellation(token),
            )
            .unwrap();
        assert_driver_interruption(
            &root,
            &format!("before-build-cancel-o{optimization}"),
            &result,
            "X09-DRIVER-001",
            "",
        );
        let elapsed = xiao_driver::RunOptions {
            timeout_ms: Some(0),
            ..run.clone()
        };
        let result = driver
            .run_application(&build, &elapsed, [] as [&str; 0])
            .unwrap();
        assert_driver_interruption(
            &root,
            &format!("before-build-deadline-o{optimization}"),
            &result,
            "X09-DRIVER-002",
            "",
        );
        assert!(!build.output.exists());

        let token = CancellationToken::new();
        let args = std::iter::once_with(|| {
            token.cancel();
            "last-argument"
        });
        let result = driver
            .run_application_with_control(
                &build,
                &run,
                args,
                &RunControl::new().with_cancellation(token.clone()),
            )
            .unwrap();
        assert!(build.output.is_file());
        assert_driver_interruption(
            &root,
            &format!("before-spawn-cancel-o{optimization}"),
            &result,
            "X09-DRIVER-001",
            "",
        );
    }
}

/// 验证运行中的截止/取消在生成代码 env 安全点生效。
#[test]
#[ignore = "需要本批 release Runtime 和固定 LLVM 工具"]
fn native_env_running_control_reaches_generated_code() {
    let target = TargetDescription::host();
    let tools = toolchain(&target);
    let source = r#"from std.env import get
from std.args import argv
for value in argv()
    print(value)
while true
    get("loop")
"#;
    let mask = ENV | STDOUT | CAPABILITY_PROCESS;
    for optimization in [0, 3] {
        let root = test_dir();
        let case = format!("active-o{optimization}");
        let build = build_request(&root, &case, source, &target, &tools, optimization, mask);
        let artifact = FrontendNativeDriver::new().build(&build).unwrap().native;
        let run = NativeRun::new(&artifact);
        let output = run_injected(
            Command::new(&artifact.executable)
                .env_clear()
                .env(DEADLINE_ENV, "0"),
            b"[]",
        );
        let rejected = NativeRunResult {
            status: output.status.code(),
            stdout: String::from_utf8(output.stdout).unwrap(),
            stderr: String::from_utf8(output.stderr).unwrap(),
        };
        let case = format!("before-entry-deadline-o{optimization}");
        assert_driver_interruption(&root, &case, &rejected, "X09-DRIVER-002", "");

        let deadline = Instant::now() + Duration::from_millis(900);
        let output = run
            .run_with_control(["entered"], None, Some(deadline), None, true)
            .unwrap();
        let case = format!("active-deadline-o{optimization}");
        assert_driver_interruption(&root, &case, &output, "X09-DRIVER-002", "entered\n");
        assert!(
            output
                .stderr
                .contains("xiao-error class=fatal code=X09-DRIVER-002 ")
        );

        let start = Instant::now();
        let cancelled = || start.elapsed() >= Duration::from_millis(900);
        let output = run
            .run_with_control(
                ["entered"],
                Some(b"[]"),
                Some(start + Duration::from_secs(5)),
                Some(&cancelled),
                true,
            )
            .unwrap();
        let case = format!("active-cancel-o{optimization}");
        assert_driver_interruption(&root, &case, &output, "X09-DRIVER-001", "entered\n");
        assert!(
            output
                .stderr
                .contains("xiao-error class=fatal code=X09-DRIVER-001 ")
        );
        println!(
            "环境原生 {} SHA-256 {}，O{}，原生入口 argv 隐藏握手参数",
            artifact.executable.display(),
            Digest256::of_bytes(&fs::read(&artifact.executable).unwrap()).as_hex(),
            optimization,
        );
    }
}

/// 验证 set/unset 提交前中断和配额失败都保留旧视图。
#[test]
#[ignore = "需要本批 release Runtime 和固定 LLVM 工具"]
fn native_env_run_control_preserves_set_and_unset_budgets() {
    let target = TargetDescription::host();
    let tools = toolchain(&target);
    let source = include_bytes!("b3_env_control.c");
    let mut pairs = vec![
        ("key".to_owned(), "old".to_owned()),
        ("ref".to_owned(), "".to_owned()),
    ];
    pairs.extend((0..1_022).map(|n| (format!("E{n:04}"), String::new())));
    let mut remaining =
        (1_048_576 - 1) - pairs.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>();
    for (_, value) in &mut pairs[2..] {
        let bytes = remaining.min(65_536);
        *value = "x".repeat(bytes);
        remaining -= bytes;
    }
    assert_eq!(remaining, 0);
    let data = serde_json::to_vec(&pairs).unwrap();
    for optimization in [0, 3] {
        let root = test_dir();
        let c_file = root.join("env_control.c");
        let executable = root.join(format!("env_control{}", std::env::consts::EXE_SUFFIX));
        fs::write(&c_file, source).unwrap();
        let mut build = Command::new(&tools.clang);
        build
            .args([
                "-target",
                &target.triple,
                "-std=c11",
                &format!("-O{optimization}"),
            ])
            .arg(&c_file)
            .arg(tools.runtime_library.as_ref().unwrap())
            .args(&tools.native_static_libraries)
            .arg("-o")
            .arg(&executable);
        let output = build.output().unwrap();
        fs::write(root.join("control-build.stdout"), &output.stdout).unwrap();
        fs::write(root.join("control-build.stderr"), &output.stderr).unwrap();
        println!("{build:?} 退出 {:?}", output.status.code());
        assert!(output.status.success(), "{output:?}");
        println!(
            "环境 C ABI 探针 O{optimization} {} SHA-256 {}",
            executable.display(),
            Digest256::of_bytes(&fs::read(&executable).unwrap()).as_hex(),
        );

        for operation in ["set", "unset"] {
            for (reason, code) in [("deadline", "X09-DRIVER-002"), ("cancel", "X09-DRIVER-001")] {
                let case = format!("commit-{operation}-{reason}-o{optimization}");
                let mut command = Command::new(&executable);
                command
                    .env_clear()
                    .env("XIAO_B3_CONTROL_TEST", operation)
                    .current_dir(&root);
                let output = if reason == "deadline" {
                    let deadline = monotonic_ns().unwrap() + Duration::from_secs(3).as_nanos();
                    command.env(DEADLINE_ENV, deadline.to_string());
                    run_injected(&mut command, &data)
                } else {
                    let signal = root.join(format!("{case}.cancel"));
                    let ready = format!("{case}.ready");
                    command
                        .env(CANCEL_ENV, &signal)
                        .env("XIAO_B3_CONTROL_READY_FILE", &ready);
                    let ready = root.join(ready);
                    std::thread::scope(|scope| {
                        scope.spawn(|| {
                            let started = Instant::now();
                            while !ready.is_file() {
                                assert!(
                                    started.elapsed() < Duration::from_secs(10),
                                    "探针未完成旧值读取"
                                );
                                std::thread::sleep(Duration::from_millis(5));
                            }
                            fs::write(&signal, b"1").unwrap();
                        });
                        run_injected(&mut command, &data)
                    })
                };
                let result = NativeRunResult {
                    status: output.status.code(),
                    stdout: String::from_utf8(output.stdout).unwrap(),
                    stderr: String::from_utf8(output.stderr).unwrap(),
                };
                let old = if cfg!(windows) { "old\r\n" } else { "old\n" };
                assert_driver_interruption(&root, &case, &result, code, old);
                assert_eq!(
                    result.stderr.matches(&format!("code={code} ")).count(),
                    3,
                    "{result:?}"
                );
                assert_eq!(
                    result.stderr.matches("code=ENV_LIMIT_EXCEEDED ").count(),
                    2,
                    "{result:?}"
                );
            }
        }
    }
}
