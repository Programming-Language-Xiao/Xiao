//! 22E 公开虚拟模块、三种产物特性位与真实原生元数据验收。

use std::time::Duration;
use xiao_driver::{DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver};
use xiao_runtime_abi::*;

/// 从真实源码创建同源 IR/TAC/`.xiaoc`，不手写并发计划。
fn artifact(source: &str) -> xiao_driver::FrontendArtifact {
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("公开源码通过前端")
}

/// 用当前标准库身份和精确 ABI 建立真实归档索引。
fn index() -> xiao_artifacts::ArchiveIndex {
    xiao_artifacts::ArchiveIndex {
        schema_major: xiao_artifacts::INDEX_SCHEMA_MAJOR,
        schema_minor: xiao_artifacts::INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: Vec::new(),
        dependency_lock_digest: "0".repeat(64),
        runtime_abi_min: ABI_ENCODED_VERSION,
        runtime_abi_max: ABI_ENCODED_VERSION,
        platform: "portable".to_owned(),
        debug_activation: false,
        language_locale: "zh-CN".to_owned(),
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    }
}

#[test]
/// 当前 Runtime、空池和单项操作不带入无关能力；三个载体沿用同一最小并集。
fn concurrency_artifacts_use_minimal_shared_feature_sets() {
    let cases = [
        ("plain", "print(1)\n", 0),
        (
            "current",
            "from std.concurrent import current_runtime\nrt = current_runtime()\n",
            0,
        ),
        (
            "multi",
            "from std.concurrent import runtime_create, runtime_close\nrt = runtime_create()\nruntime_close(rt)\n",
            REQUIRED_FEATURE_MULTI_RUNTIME,
        ),
        (
            "pool",
            "from std.concurrent import current_runtime, pool_create, shutdown\nrt = current_runtime()\npool = pool_create(rt, 1)\nshutdown(pool)\n",
            REQUIRED_FEATURE_TASK_POOL,
        ),
        (
            "shared",
            "from std.concurrent import shared_read, read\nv = shared_read(7)\nprint(read(v))\n",
            REQUIRED_FEATURE_SHARED_HANDLES | REQUIRED_FEATURE_SERIALIZATION_V1,
        ),
        (
            "threads",
            "from std.concurrent import current_runtime, spawn, join\ndef work() -> int\n    return 7\nrt = current_runtime()\nt = spawn(rt, work)\nprint(join(t))\n",
            REQUIRED_FEATURE_THREADS
                | REQUIRED_FEATURE_SERIALIZATION_V1
                | REQUIRED_FEATURE_CANCEL_WAIT,
        ),
    ];
    for (label, source, expected) in cases {
        let ir = artifact(source).ir;
        assert_eq!(
            xiao_ir::required_concurrency_features(&ir).unwrap(),
            expected,
            "{label}: IR"
        );
        let program = xiao_bytecode::lower_program(&ir);
        let bytes = xiao_bytecode::encode_xiaoc(&program, xiao_bytecode::XiaocMetadata::new(label))
            .unwrap();
        let file = xiao_bytecode::decode_xiaoc(&bytes).unwrap();
        assert_eq!(
            file.header.required_features,
            expected >> 1,
            "{label}: xiaoc"
        );
        let mut builder = xiao_xar::XarBuilder::new(index());
        builder
            .add_xiaoc("main.xiaoc", "main", "portable", &bytes, Default::default())
            .unwrap();
        let xar = xiao_xar::decode_xar(&builder.build().unwrap()).unwrap();
        assert_eq!(xar.index().required_features, expected, "{label}: xar");
        if !ir.concurrency_calls.is_empty() {
            let native = xiao_codegen_llvm::NativeMetadata::for_program(
                &ir,
                &xiao_codegen_llvm::TargetDescription::host(),
            )
            .unwrap();
            assert_eq!(native.required_features, expected, "{label}: native");
        }
        let mut unknown = bytes.clone();
        unknown[48..56].copy_from_slice(&(1_u64 << 63).to_le_bytes());
        assert!(matches!(
            xiao_bytecode::decode_xiaoc(&unknown),
            Err(xiao_bytecode::XiaocError::UnknownRequiredFeature(63))
        ));
        if expected != 0 {
            let mut missing = bytes;
            missing[48..56].fill(0);
            assert!(
                xiao_bytecode::decode_xiaoc(&missing).is_err(),
                "{label}: 缺位必须拒绝"
            );
        }
    }
}

#[test]
/// 虚拟模块只有冻结的 17 项签名，错误导入/数量/类型均由统一前端拒绝。
fn public_concurrent_signatures_remain_virtual_and_frozen() {
    assert_eq!(xiao_types::std_concurrent_signatures().len(), 17);
    assert!(
        !xiao_standard_library::source_root()
            .join("concurrent.xiao")
            .exists()
    );
    for source in [
        "from std.concurrent import missing\n",
        "from std.concurrent import current_runtime\ncurrent_runtime(1)\n",
        "from std.concurrent import pool_create\npool_create(1, 2)\n",
    ] {
        assert!(
            FrontendCompiler::new()
                .compile(&FrontendRequest::from_text(source))
                .is_err()
        );
    }
}

#[test]
/// 32 位原生并发目标在任何外部工具启动前以冻结的目标错误码拒绝。
fn public_concurrency_rejects_32_bit_target_before_toolchain() {
    use xiao_codegen_llvm::*;
    let artifact = artifact("from std.concurrent import current_runtime\nrt = current_runtime()\n");
    let target = TargetDescription::new(
        "i686-unknown-linux-gnu",
        32,
        Endian::Little,
        ObjectFormat::Elf,
    )
    .unwrap();
    assert!(
        matches!(lower_program(&artifact.ir, &CodegenOptions::for_target(target)),
        Err(CodegenError::NativeMetadata { code, .. }) if code == "X20-TARGET-001")
    );
}

/// 直接从权威 UseDocs 的代码块取样，避免教程与回归另存两份源码。
fn documentation_examples() -> Vec<(
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
)> {
    let text = include_str!("../../../../../docs/UseDocs/language/concurrency/README.md");
    let blocks = text
        .split("```xiao")
        .skip(1)
        .map(|block| {
            block
                .split_once("```")
                .unwrap()
                .0
                .trim_start_matches(['\r', '\n'])
        })
        .collect::<Vec<_>>();
    assert_eq!(blocks.len(), 5, "新增教程必须同步黄金输出列表");
    ["thread", "pool", "shared", "detached", "cancel"]
        .into_iter()
        .zip(blocks)
        .zip(["7\n", "12\n", "9\n", "main\n", ""])
        .enumerate()
        .map(|(index, ((label, source), stdout))| {
            (
                label,
                source,
                stdout,
                (index == 4).then_some("X06-RUNTIME-027"),
            )
        })
        .collect()
}

#[test]
/// 教程全部经过 VM，五个示例的并发操作并集恰好覆盖冻结的 17 个入口。
fn concurrent_documentation_examples_are_executable_and_cover_all_entries() {
    let mut operations = std::collections::BTreeSet::new();
    for (label, source, stdout, code) in documentation_examples() {
        let artifact = artifact(source);
        operations.extend(
            artifact
                .ir
                .concurrency_calls
                .iter()
                .map(|call| call.operation.clone()),
        );
        let vm = FrontendVmDriver::new().run_artifact(
            &artifact,
            &DriverRequest::new(FrontendRequest::from_text(source))
                .with_timeout(Duration::from_secs(10)),
        );
        assert_eq!(vm.code(), code, "{label}");
        let output = vm
            .as_executed()
            .unwrap()
            .events()
            .iter()
            .filter_map(|event| match event {
                xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(output, stdout, "{label}");
    }
    assert_eq!(operations.len(), 17);
}

#[test]
/// 兼容矩阵的 `.xiaoc`/`.xar` 旧新 Runtime 格子通过真实产物预检而非常量比较。
fn concurrent_portable_runtime_matrix_is_verified_before_entry() {
    let source = documentation_examples()[0].1;
    let artifact = artifact(source);
    let program = xiao_bytecode::lower_program(&artifact.ir);
    let bytes =
        xiao_bytecode::encode_xiaoc(&program, xiao_bytecode::XiaocMetadata::new("matrix")).unwrap();
    let file = xiao_bytecode::decode_xiaoc(&bytes).unwrap();
    xiao_bytecode::validate_runtime_requirements(
        &file.header,
        ABI_ENCODED_VERSION,
        XIAOC_SUPPORTED_CONCURRENCY_FEATURES,
    )
    .unwrap();
    assert!(matches!(
        xiao_bytecode::validate_runtime_requirements(
            &file.header,
            encoded_abi_version(1, 16),
            XIAOC_SUPPORTED_CONCURRENCY_FEATURES
        ),
        Err(xiao_bytecode::XiaocError::InvalidField { .. })
    ));
    let mut builder = xiao_xar::XarBuilder::new(index());
    builder
        .add_xiaoc("main.xiaoc", "main", "portable", &bytes, Default::default())
        .unwrap();
    let xar = builder.build().unwrap();
    let current = xiao_xar::run_archive(&xar, Default::default()).unwrap();
    assert!(current.result.is_success());
    let old = xiao_xar::XarRunOptions {
        runtime_abi: encoded_abi_version(1, 16),
        ..Default::default()
    };
    assert_eq!(
        xiao_xar::run_archive(&xar, old).unwrap_err().code(),
        "X17-XAR-007"
    );
    let missing = xiao_xar::XarRunOptions {
        runtime_features: 1,
        ..Default::default()
    };
    assert_eq!(
        xiao_xar::run_archive(&xar, missing).unwrap_err().code(),
        "X17-XAR-007"
    );
    let cells = xiao_driver::concurrency_compatibility_matrix();
    assert_eq!(cells.len(), 6);
    assert!(
        cells
            .iter()
            .all(|cell| cell.evidence == xiao_driver::CompatibilityEvidence::Verified)
    );
}

/// 短输出示例也施加进程截止时间，原生挂起不可阻塞 CI。
fn execute(executable: &std::path::Path) -> std::process::Output {
    let mut child = std::process::Command::new(executable)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("原生教程超过 10 秒");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.wait_with_output().unwrap()
}

/// E-0 首次取消发生在 finally 内调用的 helper；两侧都必须退出，差异限于已登记的额外 027。
fn finally_cancellation_source() -> &'static str {
    r#"from std.concurrent import current_runtime, shared_mut, spawn, join, cancel, lock, lock_read, lock_write
def read_flag(shared_mut<int> flag) -> int
    guard = lock(flag)
    return lock_read(guard)
def signal(shared_mut<int> flag) -> none
    guard = lock(flag)
    lock_write(guard, 1)
def spin() -> none
    while true
        value = 1
def work(shared_mut<int> ready) -> int
    try
        value = 1
    finally
        signal(ready)
        spin()
    return 0
rt = current_runtime()
ready = shared_mut(0)
thread = spawn(rt, work, ready)
while read_flag(ready) == 0
    value = 1
cancel(thread)
join(thread)
"#
}

#[test]
#[ignore = "需要受控 LLVM/Runtime；三平台原生作业精确执行"]
/// 公开 NativeBuild 读取真实产物段，校验失败不启动用户程序；旧 Runtime、未知位与目标拒绝。
fn public_native_concurrency_metadata_and_release_matrix() {
    use xiao_codegen_llvm::*;
    let target = TargetDescription::host();
    assert_eq!(std::env::var("XIAO_TARGET_TRIPLE").unwrap(), target.triple);
    let toolchain = Toolchain::new(std::env::var_os("XIAO_CLANG").unwrap())
        .with_llvm_as(std::env::var_os("XIAO_LLVM_AS").unwrap())
        .with_llc(std::env::var_os("XIAO_LLC").unwrap())
        .with_runtime_library(std::env::var_os("XIAO_RUNTIME_LIBRARY").unwrap())
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .unwrap();
    let source = "from std.concurrent import current_runtime, spawn, join\ndef work(int value) -> int\n    return value + 6\nrt = current_runtime()\nthread = spawn(rt, work, 1)\nprint(join(thread))\n";
    let frontend = artifact(source);
    let request = FrontendRequest::from_text(source);
    let vm = FrontendVmDriver::new().run_artifact(
        &frontend,
        &DriverRequest::new(request.clone()).with_timeout(Duration::from_secs(10)),
    );
    assert!(vm.is_success());
    let root = std::env::temp_dir().join(format!("xiao-22e-native-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let output = root.join(if cfg!(windows) {
        "public.exe"
    } else {
        "public.bin"
    });
    let built = xiao_driver::FrontendNativeDriver::new()
        .build_artifact(
            &frontend,
            &xiao_driver::NativeBuildRequest::new(
                request,
                target.clone(),
                toolchain.clone(),
                &output,
            ),
        )
        .unwrap();
    let meta = built.native.native_metadata.unwrap();
    let support = NativeRuntimeSupport::default();
    assert_eq!(
        validate_native_metadata_file(&output, &target, &support, meta.required_features, true)
            .unwrap(),
        Some(meta.clone())
    );
    assert_eq!(
        validate_native_metadata_file(
            &output,
            &target,
            &NativeRuntimeSupport {
                runtime_abi: encoded_abi_version(1, 16),
                ..support.clone()
            },
            0,
            true
        )
        .unwrap_err()
        .code,
        "X17-XAR-007"
    );
    let bytes = std::fs::read(&output).unwrap();
    let record = meta.encode().unwrap();
    let start = bytes
        .windows(record.len())
        .position(|value| value == record)
        .expect("正式产物段含记录");
    for (offset, expected) in [(0, "X17-XAR-007"), (8, "X17-XAR-007"), (39, "X17-XAR-011")] {
        let mut bad = bytes.clone();
        bad[start + offset] ^= 0x80;
        let path = root.join(format!("bad-{offset}"));
        std::fs::write(&path, bad).unwrap();
        assert_eq!(
            validate_native_metadata_file(&path, &target, &support, 0, true)
                .unwrap_err()
                .code,
            expected
        );
    }
    let result = execute(&output);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"7\n");
    assert!(result.stderr.is_empty());
    let response = xiao_driver::dispatch(xiao_driver::ProtocolRequest::Verify {
        request_id: "native-release".to_owned(),
        protocol_version: xiao_driver::PROTOCOL_VERSION,
        core_version: xiao_driver::CORE_VERSION,
        path: output.display().to_string(),
        detail: true,
    });
    let xiao_driver::ProtocolResponse::Result {
        value: Some(value),
        exit_code,
        ..
    } = response
    else {
        panic!("verify 必须返回原生产物报告");
    };
    assert_eq!(exit_code, 0);
    let report: serde_json::Value = serde_json::from_str(&value.value).unwrap();
    assert_eq!(report["release_report"]["metadata_validation"], "verified");
    assert_eq!(
        report["native_metadata"]["generator_version"],
        CODEGEN_VERSION
    );
    let stripped = root.join(if cfg!(windows) {
        "stripped.exe"
    } else {
        "stripped.bin"
    });
    std::fs::copy(&output, &stripped).unwrap();
    let stripped_status =
        std::process::Command::new(std::env::var_os("XIAO_STRIP").expect("XIAO_STRIP"))
            .arg("--strip-all")
            .arg(&stripped)
            .status()
            .unwrap();
    assert!(stripped_status.success());
    assert_eq!(
        validate_native_metadata_file(&stripped, &target, &support, 0, true).unwrap(),
        Some(meta.clone())
    );
    let legacy_path = root.join(if cfg!(windows) {
        "legacy.exe"
    } else {
        "legacy.bin"
    });
    xiao_driver::FrontendNativeDriver::new()
        .build(&xiao_driver::NativeBuildRequest::new(
            FrontendRequest::from_text("value = true\n"),
            target.clone(),
            toolchain.clone(),
            &legacy_path,
        ))
        .unwrap();
    assert_eq!(
        validate_native_metadata_file(&legacy_path, &target, &support, 0, false).unwrap(),
        None
    );
    assert_eq!(
        validate_native_metadata_file(
            &legacy_path,
            &target,
            &support,
            meta.required_features,
            true
        )
        .unwrap_err()
        .code,
        "X17-XAR-007"
    );
    let mut wrong_target = target.clone();
    wrong_target.triple = "x86_64-unknown-other".to_owned();
    assert_eq!(
        validate_native_metadata_file(&output, &wrong_target, &support, 0, true)
            .unwrap_err()
            .code,
        "X17-XAR-007"
    );
    for (label, source, stdout, code) in documentation_examples() {
        let artifact = artifact(source);
        let path = root.join(format!(
            "{label}.{}",
            if cfg!(windows) { "exe" } else { "bin" }
        ));
        let result = xiao_driver::FrontendNativeDriver::new()
            .build_artifact(
                &artifact,
                &xiao_driver::NativeBuildRequest::new(
                    FrontendRequest::from_text(source),
                    target.clone(),
                    toolchain.clone(),
                    &path,
                ),
            )
            .unwrap();
        assert!(result.native.native_metadata.is_some());
        let execution = execute(&path);
        assert_eq!(
            execution.status.code(),
            Some(if code.is_some() { 3 } else { 0 }),
            "{label}"
        );
        assert_eq!(
            String::from_utf8(execution.stdout)
                .unwrap()
                .replace("\r\n", "\n"),
            stdout,
            "{label}"
        );
        if let Some(code) = code {
            assert!(String::from_utf8_lossy(&execution.stderr).contains(&format!("code={code} ")));
        } else {
            assert!(execution.stderr.is_empty(), "{label}");
        }
        println!("22E-EXAMPLE {label}: ok");
    }
    let source = finally_cancellation_source();
    let artifact = artifact(source);
    let vm = FrontendVmDriver::new().run_artifact(
        &artifact,
        &DriverRequest::new(FrontendRequest::from_text(source))
            .with_timeout(Duration::from_secs(10)),
    );
    assert_eq!(vm.code(), Some("X06-RUNTIME-027"));
    assert_eq!(vm.exit_code().as_process_code(), 3);
    assert_eq!(
        vm.report()
            .unwrap()
            .suppressed
            .iter()
            .map(|error| error.code.as_str())
            .collect::<Vec<_>>(),
        ["X06-RUNTIME-027"]
    );
    let path = root.join(if cfg!(windows) {
        "first-finally.exe"
    } else {
        "first-finally.bin"
    });
    xiao_driver::FrontendNativeDriver::new()
        .build_artifact(
            &artifact,
            &xiao_driver::NativeBuildRequest::new(
                FrontendRequest::from_text(source),
                target.clone(),
                toolchain.clone(),
                &path,
            ),
        )
        .unwrap();
    let execution = execute(&path);
    assert_eq!(execution.status.code(), Some(3));
    assert!(execution.stdout.is_empty());
    let stderr = String::from_utf8(execution.stderr).unwrap();
    assert!(stderr.contains("class=recoverable code=X06-RUNTIME-027 "));
    assert!(
        !stderr.contains("suppressed:"),
        "E-0 只允许已登记的单层 027 差异"
    );
    println!(
        "22E-E0 first-finally-cancellation: primary=027; vm-suppressed=[027]; native-suppressed=[]"
    );
    println!(
        "22E-RELEASE target={} abi={} features={:#x} generator={} metadata=verified",
        target.triple, support.runtime_abi, meta.required_features, meta.generator_version
    );
    std::fs::remove_dir_all(root).unwrap();
}
