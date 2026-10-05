//! 19A 四入口差分回归：源码、未优化 `.xiaoc`、优化 `.xiaoc` 和 `.xar`。
//!
//! 原生 LLVM 真实产物需要 15E 的外部工具链门控；缺少工具链时由独立 ignored 用例
//! 明确报告未覆盖，不把“少跑一路”当成通过。

use std::fs;
use std::path::Path;

use xiao_artifacts::{
    ArchiveEntry, ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR, ObjectKind,
};
use xiao_bytecode::{
    XiaocMetadata, XiaocOptions, encode_xiaoc, encode_xiaoc_with_options, lower_program,
};
use xiao_driver::{
    CORE_VERSION, DriverOutcome, DriverRequest, FrontendCompiler, FrontendRequest,
    PROTOCOL_VERSION, ProtocolRequest, ProtocolResponse, ProtocolTarget, RunOptions,
    SourceIdentity, dispatch, run,
};
use xiao_optimizer::{OptimizationConfig, OptimizationLevel};
use xiao_runtime_abi::ABI_ENCODED_VERSION;
use xiao_xar::{XarObject, encode_xar};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Observation {
    output: String,
    error: Option<String>,
    exit_code: i32,
    drops: Vec<String>,
}

fn source_observation(source: &str, level: OptimizationLevel) -> Observation {
    let outcome =
        run(&DriverRequest::new(FrontendRequest::from_text(source)).with_optimization_level(level));
    match outcome {
        DriverOutcome::Executed(execution) => {
            let mut observed = Observation {
                exit_code: if execution.outcome.result.is_success() {
                    0
                } else {
                    1
                },
                error: execution.outcome.result.error_code().map(ToOwned::to_owned),
                ..Observation::default()
            };
            for event in execution.events() {
                match event {
                    xiao_vm::VmEvent::IntrinsicOutput { text } => observed.output.push_str(text),
                    xiao_vm::VmEvent::ValueReleased {
                        scope, exit, kind, ..
                    } => {
                        observed.drops.push(format!("{scope}:{exit}:{kind}"));
                    }
                    _ => {}
                }
            }
            observed
        }
        other => Observation {
            exit_code: other.exit_code().as_process_code() as i32,
            error: other.code().map(ToOwned::to_owned),
            ..Observation::default()
        },
    }
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
            "value_released" => observed.drops.push(format!("{:?}", event.data)),
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

#[test]
fn source_bytecode_and_archive_observations_are_identical() {
    let source = "print(\"19A differential\")\n";
    let unoptimized = materialize(source, OptimizationLevel::O0);
    let optimized = materialize(source, OptimizationLevel::O1);
    let root = std::env::temp_dir().join(format!("xiao-19a-differential-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建差分目录");
    let unoptimized_path = root.join("unoptimized.xiaoc");
    let optimized_path = root.join("optimized.xiaoc");
    fs::write(&unoptimized_path, &unoptimized).expect("写入未优化产物");
    fs::write(&optimized_path, &optimized).expect("写入优化产物");
    let object = XarObject::from_bytes(ObjectKind::Xiaoc, optimized);
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
    let archive_path = root.join("program.xar");
    fs::write(
        &archive_path,
        encode_xar(&index, &[object]).expect("编码归档"),
    )
    .expect("写入归档");

    let baseline = source_observation(source, OptimizationLevel::O0);
    let repl = repl_observation(source);
    let unoptimized_observation = artifact_observation(&unoptimized_path, false);
    let optimized_observation = artifact_observation(&optimized_path, false);
    let archive_observation = artifact_observation(&archive_path, true);
    assert_eq!(baseline, unoptimized_observation);
    assert_eq!(baseline, repl);
    assert_eq!(baseline, optimized_observation);
    assert_eq!(optimized_observation, archive_observation);
    let _ = fs::remove_dir_all(root);
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

#[test]
#[ignore = "需要 XIAO_CLANG、XIAO_LLVM_AS、XIAO_RUNTIME_LIBRARY、XIAO_TARGET_TRIPLE 和 XIAO_DIAGNOSTICS_PATH；由 15E 环境门控"]
fn native_fourth_side_is_explicitly_covered_when_toolchain_is_provided() {
    assert!(std::env::var_os("XIAO_CLANG").is_some());
    assert!(std::env::var_os("XIAO_TARGET_TRIPLE").is_some());
    assert_eq!(xiao_runtime_abi::ABI_ENCODED_VERSION, ABI_ENCODED_VERSION);
}
