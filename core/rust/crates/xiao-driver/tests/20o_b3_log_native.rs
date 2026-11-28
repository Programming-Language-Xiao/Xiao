//! B3-5C 原生发布入口、固定时钟对照和仅日志的归档能力。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use xiao_artifacts::Digest256;
use xiao_codegen_llvm::{CodegenOptions, NativeArtifact, NativeRun, TargetDescription, Toolchain};
use xiao_driver::{
    ArchiveBuildRequest, CORE_VERSION, CancellationToken, DriverRequest, FrontendNativeDriver,
    FrontendRequest, FrontendVmDriver, NativeBuildRequest, PROTOCOL_VERSION, ProtocolRequest,
    ProtocolResponse, RunControl, RunOptions, build_archive, dispatch,
};
use xiao_intrinsics::{CAPABILITY_CLOCK, CAPABILITY_STDOUT};
use xiao_runtime::time::TimeContext;
use xiao_vm::VmEvent;

/// 精确的日志能力联合掩码。
const MASK: u64 = CAPABILITY_CLOCK | CAPABILITY_STDOUT;
/// 冻结黄金的 Unix 毫秒。
const WALL: i64 = 946_684_800_000;
/// 冻结教程源码候选。
const TUTORIAL: &str = include_str!(
    "../../../../../docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/candidates/tests/fixtures/docs/log/tutorial.xiao"
);
/// 独立的 UTF-8/LF 教程黄金。
const GOLDEN: &[u8] = include_bytes!(
    "../../../../../docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/candidates/tests/fixtures/docs/log/tutorial.stdout"
);
/// 仅包含一次日志调用的能力夹具。
const LOG_ONLY: &str = "from std.log import write\nwrite(target = \"stdout\", message = \"只写日志\", level = \"info\")\n";
/// 主应用与任务共用配额，并以固定顺序交付日志。
const SHARED: &str = r#"from std.concurrent import current_runtime, spawn, join
def worker(str message) -> int
    os_log_write("warn", message, "stdout")
    return 0
int count = 0
while count < 1023
    os_log_write("info", "root", "stdout")
    count = count + 1
rt = current_runtime()
task = spawn(rt, worker, "worker")
join(task)
os_log_write("error", "over", "stdout")
"#;
/// 用同步等待给取消令牌和截止留出可观测的日志安全点。
const CONTROLLED: &str = r#"os_log_write("info", "entered", "stdout")
while true
    os_time_sleep(25, "ms")
    os_log_write("info", "active", "stdout")
"#;

/// 将各专项的产物写到独立目录，可供验收脚本逐件核对。
fn case_dir(name: &str) -> PathBuf {
    let base =
        std::env::var_os("XIAO_B3_LOG_TEST_ROOT").map_or_else(std::env::temp_dir, PathBuf::from);
    let path = base.join(format!("xiao-b3-log-{}-{name}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

/// 以显式工具、目标、能力和优化档构建。
fn request(
    root: &Path,
    name: &str,
    source: &str,
    mask: u64,
    level: u8,
    fixed_clock: bool,
) -> NativeBuildRequest {
    let target = TargetDescription::host();
    assert_eq!(target.triple, std::env::var("XIAO_TARGET_TRIPLE").unwrap());
    let required = |name| PathBuf::from(std::env::var_os(name).expect(name));
    let runtime = if fixed_clock {
        std::env::var_os("XIAO_FIXED_RUNTIME_LIBRARY")
            .map_or_else(|| required("XIAO_RUNTIME_LIBRARY"), PathBuf::from)
    } else {
        required("XIAO_RUNTIME_LIBRARY")
    };
    let tools = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(runtime)
        .probe_native_static_libraries("rustc", &target)
        .unwrap()
        .probe_versions()
        .unwrap();
    let options = CodegenOptions::for_target(target.clone())
        .with_optimization_level(level)
        .unwrap()
        .with_capabilities(mask)
        .unwrap();
    let source_path = root.join(format!("{name}.xiao"));
    fs::write(source_path, source).unwrap();
    NativeBuildRequest::new(
        FrontendRequest::from_text(source),
        target,
        tools,
        root.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
    )
    .with_codegen_options(options)
    .with_llvm_ir_output(root.join(format!("{name}.ll")))
}

/// 运行用既有 time 测试特性构建的 Runtime，原样保存两个输出流。
fn run_fixed(artifact: &NativeArtifact, trace: &Path, label: &str) -> Output {
    let executable = &artifact.executable;
    let output = Command::new(executable)
        .env("XIAO_TEST_TIME_WALL_CLOCK_UNIX_MS", WALL.to_string())
        .env("XIAO_TEST_TIME_MONOTONIC_NS", "0")
        .env("XIAO_RUNTIME_RELEASE_TRACE_PATH", trace)
        .output()
        .unwrap();
    fs::write(
        executable.with_extension(format!("{label}stdout")),
        &output.stdout,
    )
    .unwrap();
    fs::write(
        executable.with_extension(format!("{label}stderr")),
        &output.stderr,
    )
    .unwrap();
    output
}

/// 用 VM 的相同固定墙钟采样 stdout 及最终诊断身份。
fn vm(source: &str, mask: u64) -> (Vec<u8>, Option<String>) {
    let request = DriverRequest::new(FrontendRequest::from_text(source))
        .with_capabilities(mask)
        .unwrap();
    let result =
        TimeContext::fixed(WALL, 0).with_vm_clock(|| FrontendVmDriver::new().run(&request));
    let stdout = result
        .as_executed()
        .unwrap()
        .events()
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_bytes()),
            _ => None,
        })
        .flatten()
        .copied()
        .collect();
    (stdout, result.code().map(str::to_owned))
}

/// 原生产物身份和实际链接观察绑定到磁盘原件。
fn record(build: &NativeBuildRequest, artifact: &NativeArtifact, exit: Option<i32>) {
    let digest = |path: &Path| Digest256::of_bytes(&fs::read(path).unwrap()).as_hex();
    let exe = &artifact.executable;
    let ll = build.llvm_ir_output.as_ref().unwrap();
    let evidence = &artifact.artifact_runtime;
    let metadata = artifact.native_metadata.as_ref().unwrap();
    let hash = digest(exe);
    let map = evidence.coff_link_identity.as_ref().map(|map| {
        assert_eq!(hash, map.artifact_sha256);
        assert_eq!(digest(&map.map_path), map.map_sha256);
        serde_json::json!({"path": map.map_path, "sha256": map.map_sha256})
    });
    let verification = exe.with_extension("verification.json");
    fs::write(
        &verification,
        serde_json::to_vec_pretty(&serde_json::json!({
            "binary": {"path": exe, "sha256": hash},
            "ir": {"path": ll, "sha256": digest(ll)},
            "map": map,
            "target": metadata.target,
            "abi": metadata.runtime_abi_min,
            "generator": metadata.generator_version,
            "stdlib_interface": metadata.stdlib.stdlib_interface_version,
            "capabilities": build.codegen_options.capabilities,
            "optimization": artifact.optimization_level,
            "codegen": artifact.module.codegen_fingerprint,
            "toolchain": artifact.toolchain_fingerprint.to_string(),
            "verification": evidence.verification.as_str(),
            "declared": evidence.declared_components,
            "observed": evidence.observed_components,
            "unclassified": evidence.unclassified_runtime_symbols,
            "symbols": evidence.runtime_symbols,
            "exit": exit,
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(
        evidence
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "xiao_runtime_log_write")
    );
    assert_eq!(evidence.declared_components, evidence.observed_components);
    assert!(evidence.unclassified_runtime_symbols.is_empty());
    println!(
        "验证 {verification:?} SHA-256 {} 退出 {exit:?}",
        digest(&verification)
    );
}

/// 每个被追踪的拥有对象恰好销毁一次，轨迹连续。
fn assert_released(path: &Path) {
    let mut live = std::collections::BTreeMap::<u64, usize>::new();
    for (sequence, row) in fs::read_to_string(path).unwrap().lines().enumerate() {
        let mut fields = row.split('\t');
        assert_eq!(fields.next().unwrap().parse::<usize>().unwrap(), sequence);
        let object = fields.next().unwrap().parse().unwrap();
        if matches!(fields.next().unwrap(), "strong_release" | "destroy") {
            let destroyed = live.entry(object).or_default();
            assert_eq!(*destroyed, 0);
            *destroyed += usize::from(row.ends_with("\tdestroy"));
        }
    }
    assert!(!live.is_empty());
    assert!(live.values().all(|&destroyed| destroyed == 1), "{path:?}");
    println!("释放 {path:?} 的 {} 个对象各销毁一次", live.len());
}

/// 双导入分别与 103 字节黄金及 VM 逐字对照。
#[test]
#[ignore = "需要显式 LLVM 工具链和启用 test-failpoints 的发布 Runtime"]
fn log_tutorial_matches_native_golden() {
    let root = case_dir("tutorial");
    for alias in ["std", "stdlib"] {
        let source = TUTORIAL.replacen("from std.log", &format!("from {alias}.log"), 1);
        let (stdout, error) = vm(&source, MASK);
        assert_eq!(error, None);
        assert_eq!(stdout, GOLDEN);
        assert_eq!(GOLDEN.len(), 103);
        for level in [0, 3] {
            let name = format!("{alias}-o{level}");
            let build = request(&root, &name, &source, MASK, level, true);
            let artifact = FrontendNativeDriver::new().build(&build).unwrap().native;
            let trace = root.join(format!("{name}.release.tsv"));
            let output = run_fixed(&artifact, &trace, "");
            record(&build, &artifact, output.status.code());
            assert_eq!(output.status.code(), Some(0));
            assert_eq!(output.stdout, stdout);
            assert!(output.stderr.is_empty(), "{output:?}");
            assert_released(&trace);
        }
    }
}

/// 运行掩码缺任意一位均由日志本身拒绝。
#[test]
#[ignore = "需要显式 LLVM 工具链和发布 Runtime"]
fn log_native_enforces_each_bit_and_releases_inputs() {
    let root = case_dir("capabilities");
    for level in [0, 3] {
        for mask in [0, CAPABILITY_CLOCK, CAPABILITY_STDOUT] {
            let name = format!("missing-{mask}-o{level}");
            let build = request(&root, &name, LOG_ONLY, mask, level, false);
            let artifact = FrontendNativeDriver::new().build(&build).unwrap().native;
            let trace = root.join(format!("{name}.release.tsv"));
            let output = run_fixed(&artifact, &trace, "");
            record(&build, &artifact, output.status.code());
            let diagnostic = String::from_utf8(output.stderr).unwrap();
            assert_eq!(output.status.code(), Some(3), "{diagnostic}");
            assert!(output.stdout.is_empty());
            assert!(diagnostic.contains("code=X06-RUNTIME-026"));
            assert!(diagnostic.contains("span=58..94"), "{diagnostic}");
            assert!(!diagnostic.contains("只写日志"));
            assert_eq!(
                vm(LOG_ONLY, mask),
                (vec![], Some("X06-RUNTIME-026".to_owned()))
            );
            assert_released(&trace);
        }
    }
}

/// 根应用和任务消耗同一份配额，重复启动重置，字节等同 VM。
#[test]
#[ignore = "需要显式 LLVM 工具链和启用 test-failpoints 的发布 Runtime"]
fn log_native_tasks_share_run_quota() {
    let root = case_dir("quota");
    let (stdout, error) = vm(SHARED, MASK);
    assert_eq!(error.as_deref(), Some("LOG_LIMIT_EXCEEDED"));
    assert_eq!(stdout.iter().filter(|&&byte| byte == b'\n').count(), 1024);
    for level in [0, 3] {
        let name = format!("quota-o{level}");
        let build = request(&root, &name, SHARED, MASK, level, true);
        let artifact = FrontendNativeDriver::new().build(&build).unwrap().native;
        for pass in 1..=2 {
            let trace = root.join(format!("{name}-{pass}.release.tsv"));
            let output = run_fixed(&artifact, &trace, &format!("{pass}."));
            record(&build, &artifact, output.status.code());
            assert_eq!(output.status.code(), Some(3), "{:?}", output.stderr);
            assert_eq!(output.stdout, stdout);
            assert!(String::from_utf8_lossy(&output.stderr).contains("code=LOG_LIMIT_EXCEEDED"));
        }
    }
}

/// 归档依赖闭包只含日志，两个缺位在执行前各自拒绝。
#[test]
fn log_only_archive_requires_both_bits() {
    let root = case_dir("archive");
    let source = root.join("main.xiao");
    let mut build = ArchiveBuildRequest::new(&source);
    build.cache_root = Some(root.join("cache"));
    let token = CancellationToken::new();
    fs::write(&source, "int value = 1\n").unwrap();
    let ordinary = build_archive(&build, &token).unwrap();
    assert!(ordinary.index.stdlib_modules.is_empty());
    assert_eq!(ordinary.index.required_capabilities, 0);
    fs::write(
        root.join("ordinary.xar"),
        fs::read(&ordinary.output).unwrap(),
    )
    .unwrap();
    fs::write(&source, LOG_ONLY).unwrap();
    let first = build_archive(&build, &token).unwrap();
    let second = build_archive(&build, &token).unwrap();
    assert_eq!(first.index.stdlib_modules, ["stdlib.log"]);
    assert_eq!(first.index.required_capabilities, MASK);
    assert!(first.modules.iter().all(|module| !module.hit));
    assert_ne!(first.digest, ordinary.digest);
    assert!(second.modules.iter().all(|module| module.hit));
    assert_eq!(first.digest, second.digest);
    let run = |capabilities| {
        TimeContext::fixed(WALL, 0).with_vm_clock(|| {
            dispatch(ProtocolRequest::RunArchive {
                request_id: "b3-5c-log-archive".into(),
                protocol_version: PROTOCOL_VERSION,
                core_version: CORE_VERSION,
                locale: None,
                path: first.output.display().to_string(),
                options: RunOptions {
                    capabilities,
                    ..RunOptions::default()
                },
                diagnostics: None,
                debug: false,
            })
        })
    };
    for (capabilities, missing) in [
        (CAPABILITY_CLOCK, "stdout"),
        (CAPABILITY_STDOUT, "clock"),
        (0, "clock"),
    ] {
        let result = run(capabilities);
        println!("归档能力 {capabilities:#04x} {result:?}");
        assert!(matches!(result, ProtocolResponse::Error { error, .. }
            if error.code == "X06-RUNTIME-026" && error.message.contains(missing)));
    }
    let authorized = run(MASK);
    let ProtocolResponse::Result {
        exit_code: 0,
        events,
        ..
    } = authorized
    else {
        panic!("{authorized:?}");
    };
    let stdout: String = events
        .iter()
        .filter_map(|event| event.data.get("text").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(stdout, "2000-01-01T00:00:00.000Z INFO 只写日志\n");
    println!(
        "归档 {:?} SHA-256 {} required={MASK:#04x} cache=miss,hit",
        first.output,
        first.digest.as_hex()
    );
}

/// 运行级取消和截止都经已有 nonce 握手进入日志安全点。
#[test]
#[ignore = "需要显式 LLVM 工具链和普通发布 Runtime"]
fn log_native_control_reaches_generated_code() {
    let root = case_dir("control");
    let driver = FrontendNativeDriver::new();
    for level in [0, 3] {
        let name = format!("control-o{level}");
        let build = request(&root, &name, CONTROLLED, MASK, level, false);
        let run = RunOptions {
            capabilities: MASK,
            ..RunOptions::default()
        };
        let token = CancellationToken::new();
        token.cancel();
        let result = driver
            .run_application_with_control(
                &build,
                &run,
                [] as [&str; 0],
                &RunControl::new().with_cancellation(token),
            )
            .unwrap();
        assert_eq!(result.status, Some(2));
        assert!(result.stderr.contains("X09-DRIVER-001"));
        assert!(result.stdout.is_empty());
        assert!(!build.output.exists());

        let driver_build = request(
            &root,
            &format!("{name}-driver"),
            CONTROLLED,
            MASK,
            level,
            false,
        );
        let token = CancellationToken::new();
        let active = std::thread::scope(|scope| {
            let args = std::iter::once_with(|| {
                scope.spawn(|| {
                    std::thread::sleep(Duration::from_secs(2));
                    token.cancel();
                });
                "argument"
            });
            driver
                .run_application_with_control(
                    &driver_build,
                    &run,
                    args,
                    &RunControl::new().with_cancellation(token.clone()),
                )
                .unwrap()
        });
        assert_eq!(active.status, Some(2), "{active:?}");
        assert!(active.stdout.contains(" INFO entered\n"), "{active:?}");
        assert!(
            active
                .stderr
                .contains("xiao-error class=fatal code=X09-DRIVER-001 "),
            "{active:?}"
        );
        fs::write(root.join(format!("{name}-driver.stdout")), &active.stdout).unwrap();
        fs::write(root.join(format!("{name}-driver.stderr")), &active.stderr).unwrap();
        let artifact = driver.build(&build).unwrap().native;
        record(&build, &artifact, active.status);
        let native = NativeRun::new(&artifact);
        for (code, cancel) in [("X09-DRIVER-001", true), ("X09-DRIVER-002", false)] {
            let start = Instant::now();
            let cancelled = || start.elapsed() >= Duration::from_secs(2);
            let result = native
                .run_with_control(
                    [] as [&str; 0],
                    None,
                    Some(start + Duration::from_secs(if cancel { 5 } else { 2 })),
                    cancel.then_some(&cancelled as &dyn Fn() -> bool),
                    true,
                )
                .unwrap();
            fs::write(root.join(format!("{name}-{code}.stdout")), &result.stdout).unwrap();
            fs::write(root.join(format!("{name}-{code}.stderr")), &result.stderr).unwrap();
            assert_eq!(result.status, Some(2), "{result:?}");
            assert!(result.stdout.contains(" INFO entered\n"), "{result:?}");
            assert!(
                result
                    .stderr
                    .contains(&format!("xiao-error class=fatal code={code} ")),
                "{result:?}"
            );
            println!(
                "运行控制 {name} {code} 退出 2，stdout {} 字节",
                result.stdout.len()
            );
        }
    }
}
