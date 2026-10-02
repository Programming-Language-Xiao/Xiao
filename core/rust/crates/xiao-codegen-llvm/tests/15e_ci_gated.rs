//! 15E 真实工具链产物门控；默认跳过，只由平台复现脚本显式执行。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use xiao_codegen_llvm::{
    ArtifactAcceptanceMode, BaselineCondition, CodegenOptions, EntryObservation, NativeBuild,
    PerformanceSample, TargetDescription, Toolchain, compare_artifact_bytes, inspect_symbol_table,
    measure_baseline, verify_artifact_mode,
};
use xiao_ir::{
    IrEntryMode, IrExpression, IrExpressionKind, IrName, IrProgram, IrSpan, IrStatement,
    IrStatementKind, IrTableMember, IrTableSignature, IrType,
};

/// 返回真实产物门控夹具使用的固定源码区间。
fn span() -> IrSpan {
    IrSpan::new(0, 1)
}

/// 构造测试用的未转义名称。
fn name(text: &str) -> IrName {
    IrName {
        text: text.to_owned(),
        backticked: false,
        span: span(),
    }
}

/// 构造实际调用 Runtime 的最小动态表程序。
fn runtime_program() -> IrProgram {
    let mut program = IrProgram::new(
        IrEntryMode::Script,
        vec![IrStatement {
            kind: IrStatementKind::Expression {
                value: IrExpression {
                    kind: IrExpressionKind::NewCall {
                        callee: Box::new(IrExpression {
                            kind: IrExpressionKind::Name {
                                name: name("Record"),
                            },
                            ty: IrType::Table {
                                name: "Record".to_owned(),
                                kind: "constructor".to_owned(),
                            },
                            span: span(),
                        }),
                        arguments: Vec::new(),
                    },
                    ty: IrType::Table {
                        name: "Record".to_owned(),
                        kind: "instance".to_owned(),
                    },
                    span: span(),
                },
            },
            span: span(),
            leading_docs: Vec::new(),
        }],
        span(),
    );
    program.table_signatures = vec![IrTableSignature {
        name: "Record".to_owned(),
        kind: "instance".to_owned(),
        members: vec![IrTableMember {
            name: "count".to_owned(),
            method: false,
            public: true,
            ty: IrType::Scalar {
                name: "int".to_owned(),
            },
            span: span(),
        }],
        span: span(),
    }];
    program
}

/// 读取并严格校验 15E 所需的真实工具链环境。
fn configured_environment() -> (TargetDescription, Toolchain, PathBuf, PathBuf) {
    let configured_triple = std::env::var("XIAO_TARGET_TRIPLE")
        .expect("显式运行 --ignored 时 XIAO_TARGET_TRIPLE 必须已设置；准备方式见 10D §4");
    let target = TargetDescription::host();
    assert_eq!(
        configured_triple, target.triple,
        "目标三元组必须与 Rust host 一致"
    );
    let clang = std::env::var_os("XIAO_CLANG")
        .expect("显式运行 --ignored 时 XIAO_CLANG 必须已设置；准备方式见 10D §4");
    let llvm_as = std::env::var_os("XIAO_LLVM_AS")
        .expect("显式运行 --ignored 时 XIAO_LLVM_AS 必须已设置；准备方式见 10D §4");
    let llc = std::env::var_os("XIAO_LLC")
        .expect("显式运行 --ignored 时 XIAO_LLC 必须已设置；准备方式见 10D §4");
    let diagnostics = std::env::var_os("XIAO_DIAGNOSTICS_PATH")
        .expect("显式运行 --ignored 时 XIAO_DIAGNOSTICS_PATH 必须已设置；准备方式见 10D §4");
    let runtime = std::env::var_os("XIAO_RUNTIME_LIBRARY")
        .map(PathBuf::from)
        .expect("显式运行 --ignored 时 XIAO_RUNTIME_LIBRARY 必须已设置；准备方式见 10D §4");
    assert!(
        runtime.is_file(),
        "XIAO_RUNTIME_LIBRARY 必须指向已构建的 Runtime staticlib: {}",
        runtime.display()
    );
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain = Toolchain::new(clang)
        .with_llvm_as(llvm_as)
        .with_llc(llc)
        .with_runtime_library(runtime)
        .probe_native_static_libraries(rustc, &target)
        .expect("Runtime 原生库清单必须可查询")
        .probe_versions()
        .expect("Runtime 原生库清单和 LLVM 工具版本必须可查询");
    (target, toolchain, PathBuf::from(diagnostics), strip_path())
}

/// 读取 strip 工具路径；显式门控缺少该变量必须失败。
fn strip_path() -> PathBuf {
    std::env::var_os("XIAO_STRIP")
        .map(PathBuf::from)
        .expect("显式运行 --ignored 时 XIAO_STRIP 必须已设置；准备方式见 10D §4")
}

/// 用真实 clang 在指定优化级别生成链接后产物。
fn build_artifact(
    root: &Path,
    name: &str,
    level: u8,
    debug: bool,
    target: &TargetDescription,
    toolchain: &Toolchain,
    diagnostics: &Path,
) -> xiao_codegen_llvm::NativeArtifact {
    let output = root.join(if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name.to_owned()
    });
    let mut options = CodegenOptions::for_target(target.clone())
        .with_entry_observation(EntryObservation::Ignore)
        .with_optimization_level(level)
        .expect("O0-O3 必须是受支持的优化级别");
    if debug {
        options = options.with_debug_startup(diagnostics.to_string_lossy());
    }
    let request = xiao_codegen_llvm::BuildRequest::new(
        runtime_program(),
        target.clone(),
        toolchain.clone(),
        output,
    )
    .with_options(options);
    NativeBuild::new()
        .build(&request)
        .expect("真实 LLVM 产物必须构建成功")
}

/// 对真实产物执行 strip，并把工具失败提升为测试失败。
fn strip_artifact(path: &Path, strip: &Path) {
    let output = Command::new(strip)
        .args(["--strip-all", &path.to_string_lossy()])
        .output()
        .expect("strip 工具必须可执行；准备方式见 10D §4");
    assert!(
        output.status.success(),
        "strip 必须成功：{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// 启动真实产物并合并其标准输出和标准错误。
fn run_artifact(path: &Path) -> (Option<i32>, String) {
    let output = Command::new(path).output().expect("真实产物必须可启动");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.code(), text)
}

#[test]
#[ignore = "需要 XIAO_CLANG / XIAO_LLVM_AS / XIAO_LLC / XIAO_RUNTIME_LIBRARY / XIAO_TARGET_TRIPLE / XIAO_DIAGNOSTICS_PATH / XIAO_STRIP；准备方式见 10D §4"]
/// 在真实链接产物上逐优化级别验收 Release、Debug 和 Stripped 三种模式。
fn real_artifact_strip_modes_all_optimization_levels() {
    let (target, toolchain, diagnostics, strip) = configured_environment();
    let root = std::env::temp_dir().join(format!("xiao-15e-strip-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建 15E 临时目录");
    for level in 0..=3 {
        let release = build_artifact(
            &root,
            &format!("release-{level}"),
            level,
            false,
            &target,
            &toolchain,
            &diagnostics,
        );
        let release_report = verify_artifact_mode(
            &release.executable,
            &target,
            ArtifactAcceptanceMode::Release,
            &release.module.runtime_components,
        )
        .expect("Release 产物必须可验收");
        assert!(release_report.accepted);
        assert!(!release_report.diagnostic_symbols_present);
        let (status, stderr) = run_artifact(&release.executable);
        assert_eq!(status, Some(0), "Release 产物运行失败：{stderr}");

        let debug = build_artifact(
            &root,
            &format!("debug-{level}"),
            level,
            true,
            &target,
            &toolchain,
            &diagnostics,
        );
        let debug_report = verify_artifact_mode(
            &debug.executable,
            &target,
            ArtifactAcceptanceMode::Debug,
            &debug.module.runtime_components,
        )
        .expect("Debug 产物必须可验收");
        assert!(debug_report.accepted);
        assert!(debug_report.diagnostic_symbols_present);

        let stripped = root.join(if cfg!(target_os = "windows") {
            format!("stripped-{level}.exe")
        } else {
            format!("stripped-{level}")
        });
        fs::copy(&release.executable, &stripped).expect("复制真实 Release 产物");
        strip_artifact(&stripped, &strip);
        let stripped_report =
            verify_artifact_mode(&stripped, &target, ArtifactAcceptanceMode::Stripped, &[])
                .expect("Stripped 产物必须给出明确验收结果");
        assert!(stripped_report.accepted);
        assert!(!stripped_report.diagnostic_symbols_present);
        assert!(stripped_report.diagnostic.is_some());
        let (status, stderr) = run_artifact(&stripped);
        assert_eq!(status, Some(0), "Stripped 产物运行失败：{stderr}");
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
#[ignore = "需要 XIAO_CLANG / XIAO_LLVM_AS / XIAO_LLC / XIAO_RUNTIME_LIBRARY / XIAO_TARGET_TRIPLE / XIAO_DIAGNOSTICS_PATH；准备方式见 10D §4"]
/// 优化后的真实 Debug 产物保留调试激活位，并把开窗成功或稳定失败暴露出来。
fn real_debug_activation_survives_all_optimization_levels() {
    let (target, toolchain, diagnostics, _) = configured_environment();
    let root = std::env::temp_dir().join(format!("xiao-15e-debug-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建 15E 调试临时目录");
    for level in 0..=3 {
        let artifact = build_artifact(
            &root,
            &format!("debug-window-{level}"),
            level,
            true,
            &target,
            &toolchain,
            &diagnostics,
        );
        let report = verify_artifact_mode(
            &artifact.executable,
            &target,
            ArtifactAcceptanceMode::Debug,
            &artifact.module.runtime_components,
        )
        .expect("Debug 产物必须可验收");
        assert!(report.diagnostic_symbols_present);
        let (status, output) = run_artifact(&artifact.executable);
        if status != Some(0) {
            assert!(
                output.contains("X11-DIAGNOSTIC-START-001") || status == Some(70),
                "Debug 产物失败必须有稳定诊断：exit={status:?}, output={output}"
            );
        }
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
#[ignore = "需要 XIAO_CLANG / XIAO_LLVM_AS / XIAO_LLC / XIAO_RUNTIME_LIBRARY / XIAO_TARGET_TRIPLE / XIAO_DIAGNOSTICS_PATH；准备方式见 10D §4"]
/// 从真实产物读取符号顺序，并确认 Debug 产物路径能被产物比较记录或归一化。
fn real_symbol_table_and_debug_path_evidence() {
    let (target, toolchain, diagnostics, _) = configured_environment();
    let root = std::env::temp_dir().join(format!("xiao-15e-evidence-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建 15E 证据临时目录");
    let release = build_artifact(
        &root,
        "symbols",
        0,
        false,
        &target,
        &toolchain,
        &diagnostics,
    );
    let symbols = inspect_symbol_table(&release.executable, &target).expect("读取真实符号表");
    assert_eq!(symbols.normalized_order, {
        let mut sorted = symbols.original_order.clone();
        sorted.sort();
        sorted
    });
    if symbols.status == xiao_codegen_llvm::SymbolTableStatus::Readable {
        assert!(!symbols.original_order.is_empty(), "可读符号表不能为空");
    }

    let first = build_artifact(
        &root,
        "debug-path-a",
        0,
        true,
        &target,
        &toolchain,
        &diagnostics,
    );
    let second = build_artifact(
        &root,
        "debug-path-b",
        0,
        true,
        &target,
        &toolchain,
        &diagnostics,
    );
    let first_bytes = fs::read(&first.executable).expect("读取第一份真实产物");
    let second_bytes = fs::read(&second.executable).expect("读取第二份真实产物");
    let report = compare_artifact_bytes(&first_bytes, &second_bytes, target.object_format, &[]);
    assert!(
        report
            .normalized_fields
            .iter()
            .any(|field| field.starts_with("artifact-path@")),
        "真实 Debug 产物必须记录调试路径归一化字段：{report:?}"
    );
    println!("15E debug path evidence: {report:?}");
    let _ = fs::remove_dir_all(root);
}

#[test]
#[ignore = "需要 XIAO_CLANG / XIAO_LLVM_AS / XIAO_LLC / XIAO_RUNTIME_LIBRARY / XIAO_TARGET_TRIPLE / XIAO_DIAGNOSTICS_PATH；准备方式见 10D §4"]
/// 同一输入两次真实构建必须逐字节一致，或只包含已登记的产物归一化字段。
fn real_artifact_reproducibility_is_byte_comparable() {
    let (target, toolchain, diagnostics, _) = configured_environment();
    let root = std::env::temp_dir().join(format!("xiao-15e-repro-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建 15E 可复现临时目录");
    let first = build_artifact(&root, "repeat", 0, false, &target, &toolchain, &diagnostics);
    let first_bytes = fs::read(&first.executable).expect("读取第一次真实产物");
    let second = build_artifact(&root, "repeat", 0, false, &target, &toolchain, &diagnostics);
    let second_bytes = fs::read(&second.executable).expect("读取第二次真实产物");
    let report = compare_artifact_bytes(&first_bytes, &second_bytes, target.object_format, &[]);
    assert!(report.passed(), "真实产物不可复现：{report:?}");
    println!("15E artifact reproducibility: {report:?}");
    let _ = fs::remove_dir_all(root);
}

#[test]
#[ignore = "需要 XIAO_CLANG / XIAO_LLVM_AS / XIAO_LLC / XIAO_RUNTIME_LIBRARY / XIAO_TARGET_TRIPLE / XIAO_DIAGNOSTICS_PATH；准备方式见 10D §4"]
/// 在当前 CI runner 上按目标平台独立采集 15B 五维性能样本，不跨平台混合。
fn ci_performance_baseline_is_platform_scoped() {
    let (target, toolchain, diagnostics, _) = configured_environment();
    let root = std::env::temp_dir().join(format!("xiao-15e-baseline-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建 15E 基线临时目录");
    let mut samples = Vec::new();
    for index in 0..3 {
        let start = Instant::now();
        let artifact = build_artifact(
            &root,
            &format!("baseline-{index}"),
            0,
            false,
            &target,
            &toolchain,
            &diagnostics,
        );
        let compile_ms = start.elapsed().as_secs_f64() * 1000.0;
        let start = Instant::now();
        let (status, output) = run_artifact(&artifact.executable);
        let startup_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(status, Some(0), "基线产物启动失败：{output}");
        samples.push(PerformanceSample {
            compile_ms,
            startup_ms,
            runtime_ms: startup_ms,
            peak_memory_bytes: peak_memory_bytes(),
            artifact_bytes: fs::metadata(&artifact.executable)
                .expect("读取基线产物体积")
                .len(),
        });
    }
    let environment = std::env::var("XIAO_BASELINE_ENVIRONMENT")
        .or_else(|_| std::env::var("RUNNER_NAME"))
        .unwrap_or_else(|_| format!("ci-runner:{}", target.triple));
    let baseline = measure_baseline(
        BaselineCondition {
            target: target.triple.clone(),
            toolchain: toolchain.versions.clang.clone(),
            environment,
            repetitions: samples.len(),
        },
        samples,
        20.0,
    )
    .expect("CI 基线样本必须满足 15B 模型");
    assert!(
        baseline.within_noise_threshold(),
        "CI 基线噪声超阈值：{baseline:?}"
    );
    println!("15E platform baseline: {baseline:?}");
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
/// 读取当前测试进程的 Unix 峰值常驻内存。
fn peak_memory_bytes() -> u64 {
    #[repr(C)]
    /// `getrusage` 使用的时间值布局。
    struct TimeValue {
        seconds: i64,
        microseconds: i64,
    }
    #[repr(C)]
    /// `getrusage` 的最小字段布局；只读取峰值 RSS。
    struct ResourceUsage {
        user: TimeValue,
        system: TimeValue,
        max_rss: i64,
        rest: [i64; 11],
    }
    unsafe extern "C" {
        fn getrusage(who: i32, usage: *mut ResourceUsage) -> i32;
    }
    let mut usage = std::mem::MaybeUninit::<ResourceUsage>::zeroed();
    // RUSAGE_SELF 的值在 Unix 系统上固定为 0；单位在 macOS 为字节，Linux 为 KiB。
    let result = unsafe { getrusage(0, usage.as_mut_ptr()) };
    if result != 0 {
        return 0;
    }
    let value = unsafe { usage.assume_init().max_rss.max(0) as u64 };
    if cfg!(target_os = "macos") {
        value
    } else {
        value.saturating_mul(1024)
    }
}

#[cfg(windows)]
/// 读取当前测试进程的 Windows 峰值工作集。
fn peak_memory_bytes() -> u64 {
    #[repr(C)]
    /// Windows `PROCESS_MEMORY_COUNTERS` 的 C 布局。
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
    }
    let mut counters = ProcessMemoryCounters {
        cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    if ok == 0 {
        0
    } else {
        counters.peak_working_set_size as u64
    }
}

#[cfg(not(any(unix, windows)))]
/// 不支持的平台没有可移植的峰值内存查询，返回零并保留其他四维样本。
fn peak_memory_bytes() -> u64 {
    0
}
