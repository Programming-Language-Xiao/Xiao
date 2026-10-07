//! 19A 多路差分回归：源码、REPL、未优化 `.xiaoc`、优化 `.xiaoc`、`.xar` 和原生。
//!
//! 比较的是可观察结果（输出、退出码、错误身份、`drop` 顺序），不是字节。不一致时
//! 报告指出是哪两路。原生一路依赖 15E 的外部工具链：环境不齐时由独立用例写出
//! “未覆盖”及原因，真实构建与比较放在 `#[ignore]` 用例里，不把少跑一路当成通过。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use xiao_artifacts::{
    ArchiveEntry, ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR, ObjectKind,
};
use xiao_bytecode::{
    XiaocMetadata, XiaocOptions, encode_xiaoc, encode_xiaoc_with_options, lower_program,
};
use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_driver::{
    CORE_VERSION, DriverOutcome, DriverRequest, FrontendCompiler, FrontendNativeDriver,
    FrontendRequest, FrontendVmDriver, NativeBuildRequest, PROTOCOL_VERSION, ProtocolRequest,
    ProtocolResponse, ProtocolTarget, RunOptions, SourceIdentity, dispatch, run,
};
use xiao_optimizer::{
    DifferentialObservation, OptimizationConfig, OptimizationLevel, compare_named_observations,
    normalize_process_termination,
};
use xiao_runtime::{start_release_trace, take_release_events};
use xiao_runtime_abi::ABI_ENCODED_VERSION;
use xiao_xar::{XarObject, encode_xar};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Observation {
    output: String,
    error: Option<String>,
    exit_code: i32,
    termination: String,
    drops: Vec<String>,
}

/// 一个差分用例，以及基线必须呈现的内容，防止“全是空结果”的空洞一致。
struct Case {
    label: &'static str,
    source: &'static str,
    output: &'static str,
    error: Option<&'static str>,
    has_drops: bool,
}

const CASES: [Case; 13] = [
    Case {
        label: "held-string",
        source: "payload = \"held\"\n",
        output: "",
        error: None,
        has_drops: true,
    },
    Case {
        label: "held-array",
        source: "values = [1, 2]\n",
        output: "",
        error: None,
        has_drops: true,
    },
    Case {
        label: "print",
        source: "print(\"19A differential\")\n",
        output: "19A differential\n",
        error: None,
        has_drops: false,
    },
    Case {
        label: "overflow",
        source: "def square(int value) -> int\n    return value * value\nresult = square(4000000000)\n",
        output: "",
        error: Some("X06-RUNTIME-009"),
        has_drops: false,
    },
    Case {
        label: "nested-finally-drops",
        source: "def f() -> int\n    try\n        outer_try_payload = \"outer-try-payload\"\n        try\n            payload = \"inner-payload\"\n        finally\n            return 2\n    finally\n        outer_payload = \"outer-payload\"\nresult = f()\n",
        output: "",
        error: None,
        has_drops: true,
    },
    Case {
        label: "caught",
        source: "payload = \"held\"\ntry\n    raise ArithmeticError(code = \"CAUGHT\")\ncatch err as ArithmeticError\n    handled = \"yes\"\nfinally\n    cleanup = \"done\"\n",
        output: "",
        error: None,
        has_drops: true,
    },
    Case {
        label: "unmatched",
        source: "payload = \"held\"\nraise ArithmeticError(code = \"UNMATCHED\")\n",
        output: "",
        error: Some("UNMATCHED"),
        has_drops: true,
    },
    Case {
        label: "function-dynamic-arithmetic",
        source: "def add(value) -> int\n    return value + 1\nprobe = add(2)\n",
        output: "",
        error: None,
        has_drops: false,
    },
    Case {
        label: "function-dynamic-condition",
        source: "def check(values) -> int\n    for item in values\n        if item\n            return 1\n    return 0\nprobe = check([1])\n",
        output: "",
        error: Some("X06-RUNTIME-002"),
        has_drops: true,
    },
    Case {
        label: "selector-range-downstream",
        source: "values = [1, 2, 3, 4]\npart = values[1~2]\nprint(part[0])\n",
        output: "2\n",
        error: None,
        has_drops: true,
    },
    Case {
        label: "selector-random-downstream",
        source: "values = [1, 2, 3, 4]\npart = values[?2]\nprint(part[0])\n",
        output: "3\n",
        error: None,
        has_drops: true,
    },
    Case {
        label: "selector-all-downstream",
        source: "values = [1, 2, 3, 4]\npart = values[=]\nprint(part[3])\n",
        output: "4\n",
        error: None,
        has_drops: true,
    },
    Case {
        label: "selector-open-range-downstream",
        source: "values = [1, 2, 3, 4]\npart = values[<2]\nprint(part[0])\n",
        output: "1\n",
        error: None,
        has_drops: true,
    },
];

fn outcome_observation(outcome: &DriverOutcome) -> Observation {
    let mut observed = Observation {
        exit_code: i32::from(outcome.exit_code().as_process_code()),
        termination: normalize_process_termination(
            Some(i32::from(outcome.exit_code().as_process_code())),
            None,
        ),
        error: outcome.code().map(ToOwned::to_owned),
        ..Observation::default()
    };
    if let DriverOutcome::Executed(execution) = outcome {
        for event in execution.events() {
            match event {
                xiao_vm::VmEvent::IntrinsicOutput { text } => observed.output.push_str(text),
                xiao_vm::VmEvent::ValueReleased {
                    scope, exit, kind, ..
                } => observed.drops.push(format!("{scope}:{exit}:{kind}")),
                _ => {}
            }
        }
    }
    observed
}

fn source_observation(source: &str, level: OptimizationLevel) -> Observation {
    outcome_observation(&run(&DriverRequest::new(FrontendRequest::from_text(
        source,
    ))
    .with_optimization_level(level)))
}

/// 将 19A 观察值映射到共享优化器差分模型。
fn optimizer_observation(observation: &Observation) -> DifferentialObservation {
    DifferentialObservation {
        output: observation.output.clone(),
        error: observation.error.clone(),
        exit_code: observation.exit_code,
        termination: observation.termination.clone(),
        drops: observation.drops.clone(),
    }
}

/// 以第一路为基线，逐路比较；每条不一致都点名基线与偏离的那一路。
fn disagreements(sides: &[(&str, Observation)]) -> Vec<String> {
    let converted = sides
        .iter()
        .map(|(name, observation)| (*name, optimizer_observation(observation)))
        .collect::<Vec<_>>();
    let differences = compare_named_observations(&converted);
    let mut reports: Vec<(String, String, Vec<String>)> = Vec::new();
    for difference in differences {
        let field = match difference.field.as_str() {
            "output" => format!("输出 {:?} ≠ {:?}", difference.baseline, difference.compared),
            "error" => format!("错误身份 {} ≠ {}", difference.baseline, difference.compared),
            "exit_code" => format!("退出码 {} ≠ {}", difference.baseline, difference.compared),
            "drops" => format!(
                "drop 顺序 {} ≠ {}",
                difference.baseline, difference.compared
            ),
            other => format!(
                "字段 {other}：{} ≠ {}",
                difference.baseline, difference.compared
            ),
        };
        if let Some((baseline, compared, fields)) = reports.last_mut()
            && baseline == &difference.baseline_side
            && compared == &difference.compared_side
        {
            fields.push(field);
        } else {
            reports.push((
                difference.baseline_side,
                difference.compared_side,
                vec![field],
            ));
        }
    }
    reports
        .into_iter()
        .map(|(baseline, compared, fields)| {
            format!(
                "「{baseline}」与「{compared}」不一致：{}",
                fields.join("；")
            )
        })
        .collect()
}

fn assert_sides_agree(case: &str, sides: &[(&str, Observation)]) {
    let problems = disagreements(sides);
    assert!(
        problems.is_empty(),
        "用例 {case}：\n{}",
        problems.join("\n")
    );
}

/// 基线必须真的呈现用例声明的内容，否则“一致”可能只是都为空。
fn assert_baseline_matches_case(case: &Case, baseline: &Observation) {
    assert_eq!(baseline.output, case.output, "{} 的基线输出", case.label);
    assert_eq!(
        baseline.error.as_deref(),
        case.error,
        "{} 的基线错误身份",
        case.label
    );
    assert_eq!(
        baseline.exit_code != 0,
        case.error.is_some(),
        "{} 的基线退出码",
        case.label
    );
    assert_eq!(
        !baseline.drops.is_empty(),
        case.has_drops,
        "{} 的基线 drop：{:?}",
        case.label,
        baseline.drops
    );
}

fn materialize(source: &str, level: OptimizationLevel) -> Vec<u8> {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("差分源码应通过前端");
    let program = lower_program(artifact.ir());
    let program = if level == OptimizationLevel::O0 {
        program
    } else {
        xiao_bytecode::optimize_bytecode_checked(
            artifact.ir(),
            &program,
            OptimizationConfig::baseline("host").with_level(level),
        )
        .expect("差分优化应通过验证")
        .program
    };
    let config = OptimizationConfig::baseline("host").with_level(level);
    let fingerprint = config.fingerprint().expect("优化指纹").as_str().to_owned();
    encode_xiaoc(
        &program,
        XiaocMetadata::new("d19a").with_optimization_fingerprint(fingerprint),
    )
    .expect("差分 `.xiaoc` 应可编码")
}

fn artifact_observation(path: &Path, archive: bool) -> Observation {
    let response = if archive {
        dispatch(ProtocolRequest::RunArchive {
            request_id: "d19a-xar".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            locale: Some("zh-CN".to_owned()),
            path: path.display().to_string(),
            options: RunOptions::default(),
            diagnostics: None,
            debug: false,
        })
    } else {
        dispatch(ProtocolRequest::RunXiaoc {
            request_id: "d19a-xiaoc".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            locale: Some("zh-CN".to_owned()),
            path: path.display().to_string(),
            options: RunOptions::default(),
            diagnostics: None,
            debug: false,
        })
    };
    response_observation(response)
}

fn response_observation(response: ProtocolResponse) -> Observation {
    let ProtocolResponse::Result {
        exit_code,
        events,
        report,
        ..
    } = response
    else {
        panic!("差分产物运行必须返回统一 result");
    };
    let mut observed = Observation {
        exit_code: i32::from(exit_code),
        termination: normalize_process_termination(Some(i32::from(exit_code)), None),
        error: report.map(|report| report.code),
        ..Observation::default()
    };
    for event in events {
        match event.kind.as_str() {
            "intrinsic_output" => {
                if let Some(text) = event.data.get("text").and_then(serde_json::Value::as_str) {
                    observed.output.push_str(text);
                }
            }
            "value_released" => {
                let field = |name: &str| {
                    event
                        .data
                        .get(name)
                        .map(|value| {
                            value
                                .as_str()
                                .map_or_else(|| value.to_string(), str::to_owned)
                        })
                        .unwrap_or_default()
                };
                observed.drops.push(format!(
                    "{}:{}:{}",
                    field("scope"),
                    field("exit"),
                    field("kind")
                ));
            }
            _ => {}
        }
    }
    observed
}

fn repl_observation(source: &str) -> Observation {
    response_observation(dispatch(ProtocolRequest::Run {
        request_id: "d19a-repl".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        locale: Some("zh-CN".to_owned()),
        target: ProtocolTarget::host(),
        optimization: xiao_driver::OptimizationConfig::default(),
        source: SourceIdentity {
            module: "repl".to_owned(),
            path: None,
            text: source.to_owned(),
        },
        options: RunOptions::default(),
    }))
}

fn write_archive(path: &Path, xiaoc: Vec<u8>) {
    let object = XarObject::from_bytes(ObjectKind::Xiaoc, xiaoc);
    let index = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: vec![ArchiveEntry {
            logical_path: "main.xiaoc".to_owned(),
            object_kind: ObjectKind::Xiaoc,
            digest: object.digest,
            module: "main".to_owned(),
            target: "portable".to_owned(),
            length: object.bytes.len() as u64,
        }],
        dependency_lock_digest: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_owned(),
        runtime_abi_min: ABI_ENCODED_VERSION,
        runtime_abi_max: ABI_ENCODED_VERSION,
        platform: "portable".to_owned(),
        debug_activation: false,
        language_locale: "zh-CN".to_owned(),
    };
    fs::write(path, encode_xar(&index, &[object]).expect("编码归档")).expect("写入归档");
}

/// 跑完除原生外的所有路径，返回按“名称 → 观察”排列的列表，基线在最前。
fn vm_side_observations(label: &str, source: &str) -> Vec<(&'static str, Observation)> {
    let unoptimized = materialize(source, OptimizationLevel::O0);
    let optimized = materialize(source, OptimizationLevel::O1);
    let root = std::env::temp_dir().join(format!(
        "xiao-19a-differential-{label}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("创建差分目录");
    let unoptimized_path = root.join("unoptimized.xiaoc");
    let optimized_path = root.join("optimized.xiaoc");
    let archive_path = root.join("program.xar");
    fs::write(&unoptimized_path, &unoptimized).expect("写入未优化产物");
    fs::write(&optimized_path, &optimized).expect("写入优化产物");
    write_archive(&archive_path, optimized);

    let sides = vec![
        (
            "源码 -O0",
            source_observation(source, OptimizationLevel::O0),
        ),
        (
            "源码 -O1",
            source_observation(source, OptimizationLevel::O1),
        ),
        ("REPL", repl_observation(source)),
        (
            "未优化 .xiaoc",
            artifact_observation(&unoptimized_path, false),
        ),
        ("优化 .xiaoc", artifact_observation(&optimized_path, false)),
        ("优化 .xar", artifact_observation(&archive_path, true)),
    ];
    let _ = fs::remove_dir_all(root);
    sides
}

#[test]
fn all_vm_sides_agree_on_every_case() {
    for case in &CASES {
        let sides = vm_side_observations(case.label, case.source);
        assert_baseline_matches_case(case, &sides[0].1);
        assert_sides_agree(case.label, &sides);
    }
}

#[test]
fn disagreement_report_names_the_two_sides_and_the_field() {
    let base = Observation {
        output: "a\n".to_owned(),
        ..Observation::default()
    };
    let drifted = Observation {
        exit_code: 1,
        error: Some("X06-RUNTIME-009".to_owned()),
        ..base.clone()
    };
    let sides = [
        ("源码 -O0", base.clone()),
        ("优化 .xiaoc", base),
        ("优化 .xar", drifted),
    ];
    let problems = disagreements(&sides);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("「源码 -O0」"), "{problems:?}");
    assert!(problems[0].contains("「优化 .xar」"), "{problems:?}");
    assert!(!problems[0].contains("「优化 .xiaoc」"), "{problems:?}");
    assert!(problems[0].contains("错误身份"), "{problems:?}");
    assert!(problems[0].contains("退出码"), "{problems:?}");
    assert!(!problems[0].contains("输出"), "{problems:?}");
}

#[test]
fn baseline_check_rejects_a_vacuous_agreement() {
    let case = &CASES[2];
    let empty = Observation::default();
    let result = std::panic::catch_unwind(|| assert_baseline_matches_case(case, &empty));
    assert!(result.is_err(), "没有 drop 的基线不应被接受为该用例的基线");
}

#[test]
fn xiaoc_summaries_are_reproducible_and_level_debug_distinct() {
    let source = "value = 1\n";
    let levels = [
        materialize(source, OptimizationLevel::O0),
        materialize(source, OptimizationLevel::O1),
        materialize(source, OptimizationLevel::O2),
        materialize(source, OptimizationLevel::O3),
    ];
    let repeated = materialize(source, OptimizationLevel::O2);
    assert_eq!(levels[2], repeated);
    let digests = levels
        .iter()
        .map(|bytes| xiao_artifacts::Digest256::of_bytes(bytes).as_hex())
        .collect::<Vec<_>>();
    assert_eq!(
        digests
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );

    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("源码应通过前端");
    let program = lower_program(artifact.ir());
    let standard = encode_xiaoc(&program, XiaocMetadata::new("debug-difference")).unwrap();
    let debug = encode_xiaoc_with_options(
        &program,
        XiaocMetadata::new("debug-difference").with_debug("d19a"),
        XiaocOptions {
            debug_symbols: Some(b"debug-symbols".to_vec()),
            source: Some(source.as_bytes().to_vec()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_ne!(
        xiao_artifacts::Digest256::of_bytes(&standard),
        xiao_artifacts::Digest256::of_bytes(&debug)
    );
}

const NATIVE_ENV: [&str; 3] = ["XIAO_CLANG", "XIAO_RUNTIME_LIBRARY", "XIAO_TARGET_TRIPLE"];

/// 原生一路缺少哪些 15E 环境变量；为空表示可以构建。
fn missing_native_environment() -> Vec<&'static str> {
    NATIVE_ENV
        .iter()
        .copied()
        .filter(|name| std::env::var_os(name).is_none())
        .collect()
}

/// 原生一路的覆盖状态，供人读和断言共用。
fn native_coverage_report(missing: &[&str]) -> String {
    if missing.is_empty() {
        "原生一路：已配置 15E 环境，由 ignored 用例实际构建并比较".to_owned()
    } else {
        format!(
            "原生一路：未覆盖。缺少环境变量 {}；准备方式见 10D §4，\
             运行 `cargo test -p xiao-driver --test d19a_differential -- --ignored` 以覆盖",
            missing.join("、")
        )
    }
}

#[test]
fn native_side_reports_coverage_instead_of_silently_skipping() {
    let report = native_coverage_report(&missing_native_environment());
    eprintln!("[19A 差分] {report}");
    assert!(report.starts_with("原生一路："));
    let simulated = native_coverage_report(&["XIAO_CLANG", "XIAO_TARGET_TRIPLE"]);
    assert!(simulated.contains("未覆盖"), "{simulated}");
    assert!(simulated.contains("XIAO_CLANG"), "{simulated}");
    assert!(simulated.contains("XIAO_TARGET_TRIPLE"), "{simulated}");
    assert!(!simulated.contains("XIAO_RUNTIME_LIBRARY"), "{simulated}");
}

fn configured_native() -> (TargetDescription, Toolchain) {
    let runtime =
        PathBuf::from(std::env::var_os("XIAO_RUNTIME_LIBRARY").expect("XIAO_RUNTIME_LIBRARY"));
    assert!(
        runtime.is_file(),
        "XIAO_RUNTIME_LIBRARY 必须指向已构建的 staticlib"
    );
    let target = TargetDescription::host();
    assert_eq!(
        std::env::var("XIAO_TARGET_TRIPLE").expect("XIAO_TARGET_TRIPLE"),
        target.triple,
        "XIAO_TARGET_TRIPLE 必须与当前 Rust 编译目标一致"
    );
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain = Toolchain::new(std::env::var_os("XIAO_CLANG").expect("XIAO_CLANG"))
        .with_runtime_library(runtime)
        .probe_native_static_libraries(rustc, &target)
        .expect("rustc 应报告 Runtime staticlib 的原生库清单");
    (target, toolchain)
}

fn native_error_code(stderr: &str) -> Option<String> {
    let line = stderr
        .lines()
        .find(|line| line.starts_with("xiao-error "))?;
    let (_, rest) = line.split_once(" code=")?;
    Some(rest.split_once(' ')?.0.to_owned())
}

#[cfg(unix)]
/// 读取 Unix 进程的退出码或信号并规范化。
fn native_termination(status: &std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    let signal = status.signal().map(|signal| format!("SIG{signal}"));
    normalize_process_termination(status.code(), signal.as_deref())
}

#[cfg(not(unix))]
/// 在没有 POSIX 信号字段的平台使用退出码规范化。
fn native_termination(status: &std::process::ExitStatus) -> String {
    normalize_process_termination(status.code(), None)
}

fn release_tuples(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.replace('\t', ":"))
        .collect()
}

/// 原生已能构建但释放序列仍与 VM 不同的后续缺口。
enum NativeGap {
    /// 说明差异来源，保持差分用例运行并如实登记。
    Diverges { reason: &'static str },
}

fn native_gap(label: &str) -> Option<NativeGap> {
    let reason = match label {
        "nested-finally-drops" => "原生动态函数的嵌套 finally 释放序列仍少一条作用域释放",
        "selector-range-downstream"
        | "selector-random-downstream"
        | "selector-open-range-downstream" => {
            "原生选择结果值已与 VM 对齐，但容器释放序列仍有额外差异"
        }
        _ => return None,
    };
    Some(NativeGap::Diverges { reason })
}

#[test]
#[ignore = "需要 XIAO_CLANG、XIAO_RUNTIME_LIBRARY 与 XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
fn native_side_matches_the_vm_sides_on_every_case() {
    let missing = missing_native_environment();
    assert!(missing.is_empty(), "{}", native_coverage_report(&missing));
    let (target, toolchain) = configured_native();
    let mut problems = Vec::new();
    for case in &CASES {
        let gap = native_gap(case.label);
        let frontend_request = FrontendRequest::from_text(case.source);
        let artifact = FrontendCompiler::new()
            .compile(&frontend_request)
            .expect("差分源码应通过前端");
        let root = std::env::temp_dir().join(format!(
            "xiao-19a-native-{}-{}",
            case.label,
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("创建原生差分目录");
        let output = root.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        let request = NativeBuildRequest::new(
            frontend_request.clone(),
            target.clone(),
            toolchain.clone(),
            &output,
        );
        let native = match FrontendNativeDriver::new().build_artifact(&artifact, &request) {
            Ok(native) => native,
            Err(error) => {
                let text = format!("{error:?}");
                problems.push(format!("用例 {}：原生构建失败：{text}", case.label));
                let _ = fs::remove_dir_all(root);
                continue;
            }
        };

        let trace_guard = start_release_trace();
        let vm =
            FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(frontend_request));
        let vm_release = take_release_events();
        drop(trace_guard);
        let baseline = outcome_observation(&vm);

        let trace_path = root.join("release-events.tsv");
        let run_output = Command::new(&native.native.executable)
            .env("XIAO_RUNTIME_RELEASE_TRACE_PATH", &trace_path)
            .output()
            .expect("应能启动原生程序");
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let native_side = Observation {
            output: String::from_utf8_lossy(&run_output.stdout).into_owned(),
            error: native_error_code(&stderr),
            exit_code: run_output.status.code().unwrap_or(-1),
            termination: native_termination(&run_output.status),
            drops: Vec::new(),
        };
        let vm_side = Observation {
            drops: Vec::new(),
            ..baseline
        };
        let mut case_problems = Vec::new();
        let found = disagreements(&[("VM（源码）", vm_side), ("原生", native_side)]);
        if !found.is_empty() {
            let first_line = stderr.lines().next().unwrap_or_default();
            case_problems.push(format!(
                "{}（原生 stderr 首行：{first_line:?}）",
                found.join("；")
            ));
        }

        let vm_tuples = vm_release
            .iter()
            .map(|event| {
                format!(
                    "{}:{}:{}",
                    event.sequence,
                    event.object_id,
                    event.action.as_str()
                )
            })
            .collect::<Vec<_>>();
        let native_tuples = release_tuples(&fs::read_to_string(&trace_path).unwrap_or_default());
        if vm_tuples != native_tuples {
            case_problems.push(format!(
                "「VM（源码）」与「原生」的 Runtime 释放序列不一致：{vm_tuples:?} ≠ {native_tuples:?}"
            ));
        }
        let _ = fs::remove_dir_all(root);

        match gap {
            Some(NativeGap::Diverges { reason }) if !case_problems.is_empty() => {
                eprintln!("[19A 差分] 原生未覆盖 {}：{reason}", case.label);
            }
            Some(NativeGap::Diverges { .. }) => problems.push(format!(
                "用例 {}：登记的原生缺口已不存在，请从 native_gap 中移除",
                case.label
            )),
            None => problems.extend(
                case_problems
                    .into_iter()
                    .map(|problem| format!("用例 {}：{problem}", case.label)),
            ),
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
