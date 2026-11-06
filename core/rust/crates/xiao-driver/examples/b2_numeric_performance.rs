//! B2-A6独立进程测量：固定语言程序、一次预热、五批校准、中位数与OS保守峰值。

/// 有界校准规则，不依赖真实时钟或产品实现。
#[path = "numeric_perf/calibration.rs"]
mod calibration;
/// 固定输入族、独立黄金与完整语言程序生成。
#[path = "numeric_perf/cases.rs"]
mod cases;
/// 实际源码、锁、Runtime与编译器内容身份。
#[path = "numeric_perf/identity.rs"]
mod identity;
/// 平台OS当前驻留和历史峰值的可靠读数。
#[path = "numeric_perf/memory.rs"]
mod memory;
/// 仅测量程序使用的原生构建与时钟shim链接。
#[path = "numeric_perf/native.rs"]
mod native;
/// 新建测量进程、协议、看门狗及原始输出保存。
#[path = "numeric_perf/process.rs"]
mod process;
/// 复用既有独占运行目录，避免旧产物覆盖。
#[path = "perf/run_directory.rs"]
mod run_directory;
/// 只在完整语言调用标记间计时的release VM入口。
#[path = "numeric_perf/worker.rs"]
mod worker;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use xiao_driver::{FrontendCompiler, FrontendRequest};

/// 本次选择、身份和产物集中保存，子集测量不能被误读为全量A6验收。
struct Report {
    /// 独占创建的本轮JSON报告。
    output: PathBuf,
    /// 实际读取源码、工具和Runtime的固定身份。
    identity: identity::Identity,
    /// vm或native，决定独立时间预算。
    backend: String,
    /// 用户选择的负载；子集不能声明全量验收。
    selected_case: String,
    /// 用户选择的规模；全量同时要求case和profile为all。
    selected_profile: String,
    /// 本轮实际生成的源程序、字节码或原生文件身份。
    artifacts: Vec<identity::File>,
}

impl Report {
    /// 每条完成后持久化；完整性只表示所选队列，全量验收另要求full_catalog。
    fn save(
        &self,
        records: &[serde_json::Value],
        complete: bool,
        source_unchanged: bool,
        elapsed: f64,
    ) {
        let full_catalog = self.selected_case == "all" && self.selected_profile == "all";
        let passed = complete
            && source_unchanged
            && !records.is_empty()
            && records.iter().all(|row| row["status"] == "passed");
        let report = serde_json::json!({"schema":1,"scope":"B2-A6 controlled language performance; no B1 721 replay","identity":self.identity,"backend":self.backend,"selected_case":self.selected_case,"selected_profile":self.selected_profile,"full_catalog":full_catalog,"runtime_abi":xiao_runtime_abi::ABI_ENCODED_VERSION,"codegen":xiao_codegen_llvm::CODEGEN_VERSION,"complete":complete,"passed":passed,"full_catalog_passed":full_catalog&&passed,"source_unchanged":source_unchanged,"protocol":{"batches":5,"warmup_per_process":1,"minimum_batch_seconds":0.05,"calibration_target_seconds":0.1,"maximum_growth_factor":16,"maximum_repeat":process::MAX_REPEAT,"memory_limit_bytes":67108864,"memory":"same PID baseline after VM/Runtime init, before numeric input/warmup; OS high-water increment maximum across five processes","timing":"complete language call/repeat loop; configuration, input preparation, validation, compilation/linking excluded","formal_sample":"last retained calibration batch in each warmed process"},"artifacts":self.artifacts,"records":records,"controller_seconds":elapsed});
        std::fs::write(&self.output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

/// 最大输入和n/2n均有独立进程，原生两个优化级别分别建立统计。
fn main() {
    let mut arguments = std::env::args().skip(1);
    let mut backend = "vm".to_owned();
    let mut selected = "all".to_owned();
    let mut profile = "all".to_owned();
    let mut output = None;
    let mut artifact_root = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--worker-vm" => {
                worker::vm(&PathBuf::from(arguments.next().expect("缺少字节码")));
                return;
            }
            "--backend" => backend = arguments.next().expect("缺少backend"),
            "--case" => selected = arguments.next().expect("缺少case"),
            "--profile" => profile = arguments.next().expect("缺少profile"),
            "--output" => output = Some(PathBuf::from(arguments.next().expect("缺少output"))),
            "--artifact-root" => {
                artifact_root = Some(PathBuf::from(arguments.next().expect("缺少artifact-root")))
            }
            "--list" => {
                let root = repository();
                for case in cases::catalog(root) {
                    println!("{} {} {}", case.id, case.complexity, case.dimension);
                }
                return;
            }
            _ => panic!("未知A6参数：{argument}"),
        }
    }
    if cfg!(debug_assertions) {
        panic!("A6必须使用release VM/Runtime");
    }
    assert!(matches!(backend.as_str(), "vm" | "native"));
    assert!(matches!(profile.as_str(), "all" | "n" | "2n" | "maximum"));
    let output = output.expect("必须显式提供新的--output路径");
    let guard = std::fs::File::create_new(&output).expect("A6输出不可覆盖；请提供新路径");
    drop(guard);
    let root = repository();
    let catalog: Vec<_> = cases::catalog(root)
        .into_iter()
        .filter(|case| selected == "all" || case.id == selected)
        .collect();
    assert!(!catalog.is_empty(), "负载名没有匹配，不能登记空测量");
    assert!(
        catalog.iter().any(|case| case
            .profiles
            .iter()
            .any(|item| profile == "all" || item.label == profile)),
        "规模没有匹配"
    );
    let fixed = identity::capture(root);
    let directory = if let Some(path) = artifact_root {
        std::fs::create_dir(&path).expect("测量产物根不可复用");
        path.canonicalize().unwrap()
    } else {
        run_directory::create("xiao-b2-a6").unwrap()
    };
    let begun = Instant::now();
    println!("B2-A6-ARTIFACTS {}", directory.display());
    let source = cases::source(&catalog);
    let source_path = directory.join("measurement.xiao");
    std::fs::write(&source_path, &source).unwrap();
    let mut report = Report {
        output,
        identity: fixed,
        backend: backend.clone(),
        selected_case: selected.clone(),
        selected_profile: profile.clone(),
        artifacts: vec![identity::file(&source_path)],
    };
    let mut records = Vec::new();
    report.save(&records, false, true, begun.elapsed().as_secs_f64());
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text_at(source, &source_path))
        .expect("A6完整源程序必须通过正式前端");
    let mut executables = Vec::new();
    if backend == "vm" {
        let program = xiao_bytecode::lower_program(&artifact.ir);
        let bytes = xiao_bytecode::encode_xiaoc(
            &program,
            xiao_bytecode::XiaocMetadata::new("b2-a6-measurement"),
        )
        .unwrap();
        let portable = directory.join("measurement.xiaoc");
        std::fs::write(&portable, bytes).unwrap();
        report.artifacts.push(identity::file(&portable));
        executables.push((
            0,
            std::env::current_exe().unwrap(),
            vec!["--worker-vm".into(), portable.display().to_string()],
        ));
    } else {
        for level in [0, 3] {
            let executable = native::build(root, &directory, &artifact, level);
            report.artifacts.push(identity::file(&executable));
            executables.push((level, executable, Vec::new()));
        }
    }
    drop(artifact);
    report.save(&records, false, true, begun.elapsed().as_secs_f64());
    let mut all_passed = true;
    for (level, executable, worker_args) in executables {
        let path = directory.join(format!("optimization-{level}"));
        std::fs::create_dir(&path).unwrap();
        let mut previous: BTreeMap<String, f64> = BTreeMap::new();
        for case in &catalog {
            for input in &case.profiles {
                if profile != "all" && profile != input.label {
                    continue;
                }
                let mut batches = Vec::new();
                let mut error = None;
                for index in 0..5 {
                    match process::batch(
                        &executable,
                        &worker_args,
                        &path,
                        &case.id,
                        &input.label,
                        index,
                    ) {
                        Ok(batch) => batches.push(batch),
                        Err(reason) => {
                            error = Some(reason);
                            break;
                        }
                    }
                }
                let times: Vec<_> = batches
                    .iter()
                    .map(|batch| batch.sample.seconds / batch.sample.repeats as f64)
                    .collect();
                let median = if times.len() == 5 {
                    let mut sorted = times.clone();
                    sorted.sort_by(f64::total_cmp);
                    Some(sorted[2])
                } else {
                    None
                };
                let memory: Option<Vec<_>> = batches
                    .iter()
                    .map(|batch| batch.resident_increment_upper_bytes)
                    .collect();
                let peak = memory.and_then(|values| values.into_iter().max());
                let ratio = if input.label == "2n" {
                    median.and_then(|value| previous.get(&case.id).map(|before| value / before))
                } else {
                    None
                };
                if input.label == "n"
                    && let Some(value) = median
                {
                    previous.insert(case.id.clone(), value);
                }
                let limit = if backend == "vm" { 10.0 } else { 2.0 };
                let ratio_limit = if case.complexity == "linear" {
                    3.0
                } else {
                    5.0
                };
                let calibrated = batches.len() == 5 && batches.iter().all(|batch| batch.calibrated);
                let failed = error.is_some()
                    || median.is_some_and(|seconds| seconds > limit)
                    || peak.is_some_and(|bytes| bytes > 64 * 1024 * 1024)
                    || ratio.is_some_and(|value| value > ratio_limit);
                let status = if failed {
                    "failed"
                } else if !calibrated
                    || peak.is_none()
                    || (input.label == "2n" && profile == "all" && ratio.is_none())
                {
                    "unverifiable"
                } else {
                    "passed"
                };
                all_passed &= status == "passed";
                println!(
                    "B2-A6-PERF case={} profile={} optimization={level} median={median:?} peak={peak:?} ratio={ratio:?} result={status}",
                    case.id, input.label
                );
                records.push(serde_json::json!({"case":case.id,"profile":input.label,"size":input.size,"maximum":input.maximum,"dimension":case.dimension,"complexity":case.complexity,"optimization":level,"status":status,"error":error,"batches":batches,"normalized_seconds":times,"median_seconds":median,"seconds_limit":limit,"resident_increment_upper_bytes":peak,"ratio":ratio,"ratio_limit":ratio_limit}));
                let unchanged = identity::sources(root) == report.identity.files;
                report.save(&records, false, unchanged, begun.elapsed().as_secs_f64());
                assert!(unchanged, "测量期间源码发生变化；不能认领新文件身份");
            }
        }
    }
    let unchanged = identity::sources(root) == report.identity.files;
    report.save(&records, true, unchanged, begun.elapsed().as_secs_f64());
    if !all_passed || !unchanged {
        std::process::exit(1);
    }
}

/// 固定仓库位置，不按进程cwd寻找或静默接受外部stdlib。
fn repository() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
}
