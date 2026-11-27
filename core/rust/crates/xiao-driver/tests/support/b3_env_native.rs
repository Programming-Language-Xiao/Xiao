use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use xiao_codegen_llvm::{
    ArtifactVerification, CODEGEN_VERSION, CodegenOptions, NativeArtifact, NativeRuntimeSupport,
    TargetDescription, Toolchain,
};
use xiao_driver::{
    FrontendNativeDriver, FrontendRequest, NativeBuildRequest, ProtocolResponse, dispatch,
};
use xiao_intrinsics::CAPABILITY_PROCESS;
use xiao_runtime_abi::{NATIVE_ENVIRONMENT_NONCE_ENV, NATIVE_ENVIRONMENT_STDIN_PREFIX};

use super::native_artifact::{record, run_injected, toolchain};
use super::{
    CANCEL, DROP_ENV, ENV, GOLDEN, LOOKUP, NAME, READ_WRITE, STDOUT, options, request, stdout,
    test_dir,
};

const LEGACY_MARKER: &str = "--__xiao-run-environment-stdin";
const PRINT_ARGV: &str = "from std.args import argv\nfor value in argv()\n    print(value)\n";

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
