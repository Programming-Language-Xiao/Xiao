//! B3-1 的正式 time 教程入口。

use xiao_driver::{DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver};
use xiao_intrinsics::{CAPABILITY_CLOCK, CAPABILITY_STDOUT};
use xiao_runtime::time::TimeContext;
use xiao_vm::VmEvent;

const SOURCE: &str = include_str!("../../../../../tests/fixtures/docs/time/tutorial.xiao");
const GOLDEN: &[u8] = include_bytes!("../../../../../tests/fixtures/docs/time/tutorial.stdout");
const INPUT: &str = include_str!("../../../../../tests/fixtures/docs/time/tutorial.input.json");
const CAPABILITIES: u64 = CAPABILITY_CLOCK | CAPABILITY_STDOUT;

fn input() -> (i64, u64) {
    let value: serde_json::Value = serde_json::from_str(INPUT).unwrap();
    (
        value["wall_clock_unix_ms"].as_i64().unwrap(),
        value["monotonic_elapsed_ns"].as_u64().unwrap(),
    )
}

fn sources() -> [(&'static str, String); 2] {
    [
        ("std", SOURCE.to_owned()),
        (
            "stdlib",
            SOURCE.replacen("from std.time import ", "from stdlib.time import ", 1),
        ),
    ]
}

#[test]
fn time_tutorial_matches_vm_golden() {
    let (wall, monotonic) = input();
    for (prefix, source) in sources() {
        let request = DriverRequest::new(FrontendRequest::from_text(source))
            .with_capabilities(CAPABILITIES)
            .unwrap();
        let result = TimeContext::fixed(wall, monotonic)
            .with_vm_clock(|| FrontendVmDriver::new().run(&request));
        assert!(result.is_success(), "{prefix}: {result:?}");
        let output = result
            .as_executed()
            .unwrap()
            .events()
            .iter()
            .filter_map(|event| match event {
                VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(output.as_bytes(), GOLDEN, "{prefix}");
        println!("B3-1-TUTORIAL backend=vm alias={prefix} exact=true");
    }
}

#[test]
fn time_vm_errors_and_child_clock_follow_the_frozen_contract() {
    let source = r#"from std.time import utc_now, monotonic, format_utc, sleep
from std.concurrent import current_runtime, spawn, join
def elapsed() -> int
    sleep(1, "ns")
    return monotonic("ns")
sleep(1, "ns")
t = spawn(current_runtime(), elapsed)
print(join(t))
print(monotonic("ns"))
try
    sleep(60001, "ms")
catch err as ResourceError
    print(error_code(err))
try
    format_utc(253402300800000, "ms")
catch err as TypeError
    print(error_code(err))
print(utc_now("ms"))
"#;
    let request = DriverRequest::new(FrontendRequest::from_text(source));
    let result = TimeContext::fixed(946_684_800_000, 7)
        .with_vm_clock(|| FrontendVmDriver::new().run(&request));
    assert!(result.is_success(), "{result:?}");
    assert_eq!(
        vm_stdout(&result),
        "9\n9\nTIME_SLEEP_LIMIT_EXCEEDED\nX06-RUNTIME-012\n946684800000\n"
    );
    for (call, code) in [
        ("utc_now(\"MS\")", Some("X06-RUNTIME-026")),
        ("monotonic(\"ms\")", Some("X06-RUNTIME-026")),
        ("sleep(0, \"ns\")", Some("X06-RUNTIME-026")),
        ("format_utc(0, \"ms\")", None),
    ] {
        let source =
            format!("from stdlib.time import utc_now, monotonic, format_utc, sleep\n{call}\n");
        let request = DriverRequest::new(FrontendRequest::from_text(source))
            .with_capabilities(0)
            .unwrap();
        let result = FrontendVmDriver::new().run(&request);
        assert_eq!(result.code(), code, "{call}: {result:?}");
        assert_eq!(result.is_success(), code.is_none(), "{call}");
    }
}

#[test]
fn time_driver_control_interrupts_a_long_sleep() {
    use std::time::{Duration, Instant};

    let source = "value = os_time_monotonic(\"ns\")\nos_time_sleep(60000, \"ms\")\n";
    let request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new().compile(&request).unwrap();
    let deadline = Duration::from_millis(250);
    let start = Instant::now();
    let result = FrontendVmDriver::new().run_artifact(
        &artifact,
        &DriverRequest::new(request).with_timeout(deadline),
    );
    assert_eq!(
        result.code(),
        Some(xiao_driver::DRIVER_TIMEOUT_CODE),
        "{result:?}"
    );
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn reserved_intrinsic_ids_fail_before_execution() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text("os_time_monotonic(\"ns\")\n"))
        .unwrap();
    let tac = xiao_bytecode::lower_program(&artifact.ir);
    assert!(
        tac.signatures
            .iter()
            .any(|sig| sig.intrinsic_id() == Some(50))
    );
    for id in 53..=63 {
        let mut invalid = tac.clone();
        let mut signatures = xiao_bytecode::sig::CallSigTable::new();
        for signature in tac.signatures.iter() {
            signatures.intern(if signature.intrinsic_id() == Some(50) {
                xiao_bytecode::sig::CallSig::intrinsic(id)
            } else {
                signature.clone()
            });
        }
        invalid.signatures = signatures;
        assert!(xiao_bytecode::encode_xiaoc(&invalid, Default::default()).is_err());
        let result = xiao_vm::run_request(&xiao_vm::RunRequest::new(&artifact.ir, &invalid));
        assert!(!result.result.is_success(), "reserved {id}");
        assert!(
            result.events.iter().all(|event| !matches!(
                event,
                VmEvent::ModuleLoaded { .. } | VmEvent::IntrinsicOutput { .. }
            )),
            "reserved {id} reached the application: {result:?}"
        );
    }
}

fn vm_stdout(result: &xiao_driver::DriverOutcome) -> String {
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

#[test]
#[cfg(feature = "time-native-test")]
#[ignore = "需要当前任务 Runtime 和指定 LLVM 工具链"]
fn time_tutorial_matches_native_golden() {
    use std::path::PathBuf;
    use xiao_codegen_llvm::{ArtifactVerification, CodegenOptions, TargetDescription, Toolchain};
    use xiao_driver::{FrontendNativeDriver, NativeBuildRequest};

    let required =
        |name: &str| PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("缺少 {name}")));
    let target = TargetDescription::host();
    assert_eq!(std::env::var("XIAO_TARGET_TRIPLE").unwrap(), target.triple);
    let tools = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries("rustc", &target)
        .unwrap();
    let root = std::env::temp_dir().join(format!("xiao-b3-1-tutorial-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let (wall, mono) = input();
    let clock_source = "from std.time import utc_now, monotonic, sleep\nprint(utc_now(\"ms\"))\nsleep(1, \"ns\")\nprint(monotonic(\"ns\"))\n";
    let clock_request = FrontendRequest::from_text(clock_source);
    let clock_artifact = FrontendCompiler::new().compile(&clock_request).unwrap();
    let clock_executable = root.join(format!("clock{}", std::env::consts::EXE_SUFFIX));
    let clock_build = NativeBuildRequest::new(
        clock_request,
        target.clone(),
        tools.clone(),
        &clock_executable,
    )
    .with_codegen_options(
        CodegenOptions::for_target(target.clone())
            .with_capabilities(CAPABILITIES)
            .unwrap(),
    );
    FrontendNativeDriver::new()
        .build_artifact(&clock_artifact, &clock_build)
        .unwrap();
    assert_eq!(
        run(&clock_executable, &root, wall, mono),
        format!("{wall}\n{}\n", mono + 1).as_bytes()
    );
    for (prefix, source) in sources() {
        for level in [0, 3] {
            let directory = root.join(format!("{prefix}-o{level}"));
            std::fs::create_dir(&directory).unwrap();
            let executable = directory.join(format!("tutorial{}", std::env::consts::EXE_SUFFIX));
            let options = CodegenOptions::for_target(target.clone())
                .with_optimization_level(level)
                .unwrap()
                .with_capabilities(CAPABILITIES)
                .unwrap();
            let request = FrontendRequest::from_text(source.clone());
            let artifact = FrontendCompiler::new().compile(&request).unwrap();
            let build =
                NativeBuildRequest::new(request, target.clone(), tools.clone(), &executable)
                    .with_codegen_options(options);
            let result = FrontendNativeDriver::new()
                .build_artifact(&artifact, &build)
                .unwrap();
            let composition = &result.native.artifact_runtime;
            assert_eq!(
                composition.verification,
                ArtifactVerification::Complete,
                "{directory:?}"
            );
            assert_eq!(
                composition.declared_components,
                composition.observed_components
            );
            assert!(composition.unclassified_runtime_symbols.is_empty());
            let metadata = result.native.native_metadata.as_ref().unwrap();
            assert_eq!(
                metadata.runtime_abi_min,
                xiao_runtime_abi::ABI_ENCODED_VERSION
            );
            assert_eq!(
                metadata.generator_version,
                xiao_codegen_llvm::CODEGEN_VERSION
            );
            if target.object_format == xiao_codegen_llvm::ObjectFormat::Coff {
                let link = composition
                    .coff_link_identity
                    .as_ref()
                    .expect("MSVC time 构建保留 COFF map");
                assert_eq!(link.map_path, executable.with_file_name("tutorial.exe.map"));
                assert!(link.map_path.is_file());
                let hash = |path: &std::path::Path| {
                    xiao_artifacts::Digest256::of_bytes(&std::fs::read(path).unwrap()).as_hex()
                };
                assert_eq!(hash(&executable), link.artifact_sha256);
                assert_eq!(hash(&link.map_path), link.map_sha256);
                println!(
                    "B3-1-COFF artifact={} sha256={} map={} map_sha256={}",
                    executable.display(),
                    link.artifact_sha256,
                    link.map_path.display(),
                    link.map_sha256
                );
            }
            let output = run(&executable, &directory, wall, mono);
            assert_eq!(output, GOLDEN, "{prefix} O{level}");
            println!(
                "B3-1-TUTORIAL backend=native alias={prefix} opt={level} verification={} exact=true artifact={}",
                composition.verification.as_str(),
                executable.display()
            );
        }
    }
}

#[cfg(feature = "time-native-test")]
fn run(
    executable: &std::path::Path,
    directory: &std::path::Path,
    wall: i64,
    monotonic: u64,
) -> Vec<u8> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let mut child = Command::new(executable)
        .current_dir(directory)
        .env("XIAO_TEST_TIME_WALL_CLOCK_UNIX_MS", wall.to_string())
        .env("XIAO_TEST_TIME_MONOTONIC_NS", monotonic.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("time 程序未在测试截止前完成：{output:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}
