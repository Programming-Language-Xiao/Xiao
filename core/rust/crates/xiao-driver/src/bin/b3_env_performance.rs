//! B3-4E env 性能入口。使用固定的合成快照与 release VM、原生 O0/O3。

/// 复用数值性能的冻结校准规则。
#[path = "../../examples/numeric_perf/calibration.rs"]
mod calibration;
/// 复用 OS 驻留内存读数。
#[path = "../../examples/numeric_perf/memory.rs"]
mod memory;
/// 原生工具链与测量程序构建。
#[path = "env_perf/native.rs"]
mod native;
/// 独立子进程校准与原始记录。
#[path = "env_perf/process.rs"]
mod process;
/// 固定合成输入和 Xiao 负载。
#[path = "env_perf/workload.rs"]
mod workload;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{Value, json};
use xiao_artifacts::Digest256;
use xiao_codegen_llvm::TargetDescription;
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_intrinsics::{CAPABILITY_CLOCK, CAPABILITY_ENVIRONMENT, CAPABILITY_STDOUT};
use xiao_vm::{
    CompiledModule, EnvironmentSnapshot, ExecutableModule, ModuleLoader, RunResult, VmEvent,
    VmOptions,
};

/// 采样请求的环境、标准输出和时钟能力。
const CAPABILITIES: u64 = CAPABILITY_ENVIRONMENT | CAPABILITY_STDOUT | CAPABILITY_CLOCK;
/// 每个组合的正式批次数。
const BATCHES: usize = 5;
/// 驻留增量上限。
const MEMORY_LIMIT: u64 = 64 * 1024 * 1024;
/// 相邻规模中位数比上限。
const RATIO_LIMIT: f64 = 3.0;
/// 命令行帮助文本。
const HELP: &str =
    "cargo +1.96.0 run --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --release \
--bin b3_env_performance -- --output-dir DIR --artifact-dir DIR --identity JSON [--probe]

VM、原生 O0/O3 各五批，两个输出目录均须新建。--probe 只预热并验证负载。
身份文件须含 baseline_commit、product_commit、target、runtime.sha256 和 toolchain。
原生工具取 XIAO_TARGET_TRIPLE、XIAO_CLANG、XIAO_LLVM_AS、XIAO_LLC、XIAO_RUNTIME_LIBRARY。";

/// 命令行解析后的输出、产物、身份和探针选项。
#[derive(Default)]
struct Options {
    output: PathBuf,
    artifacts: PathBuf,
    identity: PathBuf,
    probe: bool,
}

/// 一个负载的源码、字节码、可移植产物和两个原生优化档。
struct Program {
    source: Value,
    bytecode: Value,
    portable: PathBuf,
    native: [PathBuf; 2],
}

/// 校验发布 Runtime 身份并执行性能采样。
fn main() {
    assert!(
        !xiao_runtime::test_failpoints_enabled(),
        "VM 性能入口要求 Runtime 关闭 test-failpoints"
    );
    let mut args = std::env::args_os().skip(1);
    let mut options = Options::default();
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--help" | "-h") => {
                println!("{HELP}");
                return;
            }
            Some("--worker-vm") => {
                let bytecode = PathBuf::from(args.next().expect("缺少字节码"));
                let case = args.next().expect("缺少负载");
                vm_worker(&bytecode, case == "snapshot");
                return;
            }
            Some("--output-dir") => options.output = args.next().expect("缺少输出目录").into(),
            Some("--artifact-dir") => options.artifacts = args.next().expect("缺少产物目录").into(),
            Some("--identity") => options.identity = args.next().expect("缺少身份文件").into(),
            Some("--probe") => options.probe = true,
            _ => panic!("未知参数 {argument:?}"),
        }
    }
    const {
        assert!(!cfg!(debug_assertions), "采样必须使用 release");
    }
    assert!(
        !options.output.as_os_str().is_empty()
            && !options.artifacts.as_os_str().is_empty()
            && !options.identity.as_os_str().is_empty(),
        "{HELP}"
    );
    if !run(&options) {
        std::process::exit(1);
    }
}

/// 创建独占目录并返回绝对路径。
fn create_directory(path: &Path) -> PathBuf {
    fs::create_dir(path).unwrap_or_else(|error| panic!("{} {error}", path.display()));
    std::path::absolute(path).unwrap()
}

/// 读取文件并生成路径、字节数和 SHA-256 回执。
fn file_info(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("{} {error}", path.display()));
    json!({"path": path, "bytes": bytes.len(), "sha256": Digest256::of_bytes(&bytes).as_hex()})
}

/// 以独占方式写入规范化 JSON 文件。
fn write_json(path: &Path, value: &Value) {
    let mut file = fs::File::create_new(path).unwrap();
    serde_json::to_writer_pretty(&mut file, value).unwrap();
    writeln!(file).unwrap();
}

/// 为所有非快照负载生成字节码和两个原生优化档。
fn programs(
    artifacts: &Path,
    output: &Path,
    target: &TargetDescription,
) -> BTreeMap<&'static str, Program> {
    let directory = create_directory(&output.join("build"));
    let tools = native::toolchain(target);
    let shim = native::build_shim(artifacts, &directory, target, &tools);
    workload::OPERATIONS
        .iter()
        .copied()
        .filter(|case| *case != "snapshot")
        .map(|case| {
            let work = create_directory(&artifacts.join(case));
            let logs = create_directory(&directory.join(case));
            let source = workload::source(case);
            let source_path = work.join("measurement.xiao");
            fs::write(&source_path, &source).unwrap();
            let artifact = FrontendCompiler::new()
                .compile(&FrontendRequest::from_text_at(source, &source_path))
                .unwrap_or_else(|error| panic!("{case}: {error:?}"));
            let program = xiao_bytecode::lower_program(&artifact.ir);
            let bytecode = xiao_bytecode::encode_xiaoc(
                &program,
                xiao_bytecode::XiaocMetadata::new("b3-env-perf"),
            )
            .unwrap();
            let portable = work.join("measurement.xiaoc");
            fs::write(&portable, &bytecode).unwrap();
            let binaries = [0, 3]
                .map(|level| native::build(&work, &logs, target, &tools, &shim, &artifact, level));
            let program = Program {
                source: file_info(&source_path),
                bytecode: file_info(&portable),
                portable,
                native: binaries,
            };
            fs::copy(&source_path, logs.join("measurement.xiao")).unwrap();
            (case, program)
        })
        .collect()
}

/// 执行所有后端的五批采样并写出汇总。
fn run(options: &Options) -> bool {
    let target = TargetDescription::host();
    let identity: Value = serde_json::from_slice(&fs::read(&options.identity).unwrap()).unwrap();
    let runtime_path = native::required("XIAO_RUNTIME_LIBRARY");
    let runtime = file_info(&runtime_path);
    assert_eq!(identity["target"], target.triple);
    assert_eq!(identity["runtime"]["sha256"], runtime["sha256"]);
    for name in ["baseline_commit", "product_commit"] {
        assert!(
            identity[name].as_str().is_some_and(|hash| hash.len() == 40),
            "{name}"
        );
    }
    assert!(identity["toolchain"].is_object());

    let output = create_directory(&options.output);
    let artifacts = create_directory(&options.artifacts);
    let batch_dir = create_directory(&output.join("batches"));
    let task_dir = create_directory(&artifacts.join("tasks"));
    let identity_path = output.join("identity.json");
    write_json(&identity_path, &identity);
    let identity_sha256 = file_info(&identity_path)["sha256"].clone();
    let programs = programs(&artifacts, &output, &target);
    let vm = std::env::current_exe().unwrap();
    let mut records = Vec::new();

    for (backend, level, limit) in [
        ("vm", None, 10.0),
        ("native-o0", Some(0), 2.0),
        ("native-o3", Some(3), 2.0),
    ] {
        for case in workload::OPERATIONS {
            let full = matches!(case, "full-replace" | "overflow");
            let sizes = if full { &[1_024][..] } else { &workload::SIZES };
            let program = &programs[if case == "snapshot" { "get" } else { case }];
            let executable = match level {
                None => &vm,
                Some(0) => &program.native[0],
                Some(_) => &program.native[1],
            };
            let args: Vec<OsString> = if level.is_none() {
                vec![
                    "--worker-vm".into(),
                    program.portable.clone().into_os_string(),
                    case.into(),
                ]
            } else {
                vec![case.into()]
            };
            let binary = file_info(executable);
            let mut previous = None;
            for &size in sizes {
                let combo = format!("{backend}-{case}-{size}");
                let pairs = workload::input(size, full);
                let data = serde_json::to_vec(&pairs).unwrap();
                let input = json!({
                    "pairs": size,
                    "value_bytes": workload::value_bytes(full),
                    "logical_bytes": pairs.iter().map(|(name, value)| name.len() + value.len()).sum::<usize>(),
                    "json_bytes": data.len(),
                    "sha256": Digest256::of_bytes(&data).as_hex(),
                });
                let logs = create_directory(&batch_dir.join(&combo));
                let tasks = create_directory(&task_dir.join(&combo));
                let mut batches = Vec::new();
                let mut peaks = Vec::new();
                let mut times = Vec::new();
                let mut failed = false;
                for index in 1..=if options.probe { 1 } else { BATCHES } {
                    let batch = format!("batch-{index}");
                    let task = create_directory(&tasks.join(&batch));
                    fs::write(task.join("snapshot.json"), &data).unwrap();
                    let stdout = logs.join(format!("{batch}.stdout.log"));
                    let stderr = logs.join(format!("{batch}.stderr.log"));
                    let trial =
                        process::measure(executable, &args, &task, &stdout, &stderr, options.probe);
                    failed |= trial.error.is_some();
                    let peak = trial.peak_increment();
                    let sample = trial.sample();
                    let seconds = sample.map(|value| value.seconds / value.repeats as f64);
                    let calibrated = sample.is_some_and(|value| value.seconds >= 0.05);
                    peaks.push(peak);
                    times.push(seconds.filter(|_| calibrated));
                    let record = logs.join(format!("{batch}.json"));
                    write_json(
                        &record,
                        &json!({
                            "schema": 1,
                            "baseline_commit": identity["baseline_commit"],
                            "product_commit": identity["product_commit"],
                            "target": target.triple,
                            "identity_sha256": identity_sha256,
                            "runtime_sha256": runtime["sha256"],
                            "backend": backend,
                            "optimization": level,
                            "case": case,
                            "batch": index,
                            "probe": options.probe,
                            "input": input,
                            "executable": binary,
                            "argv": args,
                            "cwd": task,
                            "stdout": file_info(&stdout),
                            "stderr": file_info(&stderr),
                            "process": trial,
                            "formal_sample": sample,
                            "calibrated": calibrated,
                            "seconds_per_workload": seconds,
                            "resident_increment_upper_bytes": peak,
                        }),
                    );
                    batches.push(file_info(&record));
                }
                let mut values: Vec<_> = times.iter().flatten().copied().collect();
                values.sort_by(f64::total_cmp);
                let median = (values.len() == BATCHES).then(|| values[BATCHES / 2]);
                let ratio = median.zip(previous).map(|(current, prior)| current / prior);
                previous = median;
                let peak = peaks.iter().flatten().copied().max();
                let status = if failed
                    || median.is_some_and(|value| value > limit)
                    || peak.is_some_and(|value| value > MEMORY_LIMIT)
                    || ratio.is_some_and(|value| value > RATIO_LIMIT)
                {
                    "fail"
                } else if options.probe {
                    "probe-pass"
                } else if median.is_none()
                    || peaks.contains(&None)
                    || (size > 256 && !full && ratio.is_none())
                {
                    "unverifiable"
                } else {
                    "pass"
                };
                println!("{combo} {status} median={median:?} peak={peak:?} ratio={ratio:?}");
                records.push(json!({
                    "backend": backend, "optimization": level, "case": case,
                    "input": input, "source": program.source, "bytecode": program.bytecode,
                    "executable": binary, "batches": batches,
                    "seconds_per_workload": times, "median_seconds": median,
                    "seconds_limit": limit, "resident_increment_upper_bytes": peak,
                    "memory_limit_bytes": MEMORY_LIMIT, "doubling_ratio": ratio,
                    "doubling_ratio_limit": RATIO_LIMIT, "status": status,
                }));
            }
        }
    }
    let runtime_unchanged = file_info(&runtime_path) == runtime;
    let passed =
        !options.probe && runtime_unchanged && records.iter().all(|row| row["status"] == "pass");
    write_json(
        &output.join("performance.json"),
        &json!({
            "schema": 1, "identity": identity, "identity_sha256": identity_sha256, "runtime": runtime,
            "target": target.triple, "capabilities": CAPABILITIES,
            "runtime_abi": xiao_runtime_abi::ABI_ENCODED_VERSION,
            "codegen": xiao_codegen_llvm::CODEGEN_VERSION,
            "stdlib_interface": xiao_standard_library::STDLIB_INTERFACE_VERSION,
            "protocol": {
                "batches_per_case": if options.probe { 1 } else { BATCHES },
                "warmup_repetitions": 1, "minimum_seconds": 0.05, "calibration_target_seconds": 0.1,
                "maximum_growth_factor": 16, "maximum_repeats": calibration::MAX_REPEAT,
                "fixed_calls_per_workload": workload::FIXED_CALLS, "release": true,
            },
            "probe": options.probe, "runtime_unchanged": runtime_unchanged,
            "complete": !options.probe, "passed": passed, "records": records,
        }),
    );
    if options.probe {
        records.iter().all(|row| row["status"] == "probe-pass")
    } else {
        passed
    }
}

/// 为 VM 采样预编译的标准库模块集合。
#[derive(Debug)]
struct VmModules(BTreeMap<&'static str, ExecutableModule>);

impl VmModules {
    /// 加载 env 与 time 标准库模块。
    fn prepare() -> Self {
        let loader = xiao_driver::standard_library_module_loader();
        Self(
            ["stdlib.env", "stdlib.time"]
                .into_iter()
                .map(|name| (name, loader.load(name).unwrap().unwrap()))
                .collect(),
        )
    }
}

impl ModuleLoader for VmModules {
    /// 判断模块是否为标准库或已预编译模块。
    fn contains(&self, identity: &str) -> bool {
        identity == "stdlib" || self.0.contains_key(identity)
    }

    /// 拒绝动态编译；采样只使用预编译模块。
    fn compile(&self, identity: &str) -> Result<Option<CompiledModule>, String> {
        Err(format!("样例使用已编译的模块 {identity}"))
    }

    /// 返回预编译模块的克隆。
    fn load(&self, identity: &str) -> Result<Option<ExecutableModule>, String> {
        Ok(self.0.get(identity).map(|module| ExecutableModule {
            program: module.program.clone(),
            source_name: module.source_name.clone(),
            instance_tables: module.instance_tables.clone(),
        }))
    }

    /// 采样负载没有模块级环境扩展。
    fn environment(&self, _: &str) -> Option<&str> {
        None
    }
}

/// 作为 VM 子进程执行测量，按 stdin 指令返回原始样本。
fn vm_worker(bytecode: &Path, measure_snapshot: bool) {
    let mut stdout = io::stdout().lock();
    let baseline =
        json!({"event": "baseline", "pid": std::process::id(), "memory": memory::resident().ok()});
    writeln!(stdout, "{baseline}").unwrap();
    stdout.flush().unwrap();
    let bytecode = fs::read(bytecode).unwrap();
    let pairs: Vec<(String, String)> =
        serde_json::from_slice(&fs::read("snapshot.json").unwrap()).unwrap();
    let modules = VmModules::prepare();

    for line in io::stdin().lock().lines() {
        let repeats: usize = line.unwrap().parse().unwrap();
        if repeats == 0 {
            break;
        }
        let mut seconds = 0.0;
        for _ in 0..repeats {
            let start = measure_snapshot.then(Instant::now);
            let snapshot = EnvironmentSnapshot::from_pairs(&pairs).unwrap();
            let snapshot_seconds = start.map(|start| start.elapsed().as_secs_f64());
            let result = xiao_vm::run_xiaoc_production_with_environment(
                &bytecode,
                VmOptions::default(),
                1024,
                None,
                CAPABILITIES,
                Some(&modules),
                xiao_bytecode::SUPPORTED_REQUIRED_FEATURES,
                None,
                snapshot,
            )
            .unwrap();
            assert!(
                matches!(result.result, RunResult::Success),
                "{:?}",
                result.result
            );
            let mut output = result.events.iter().filter_map(|event| {
                if let VmEvent::IntrinsicOutput { text } = event {
                    Some(text)
                } else {
                    None
                }
            });
            let nanos = output
                .next()
                .expect("缺少 env 输出")
                .trim()
                .parse::<u64>()
                .expect("env 黄金校验失败");
            assert!(output.next().is_none(), "出现多余的 env 输出");
            seconds += snapshot_seconds.unwrap_or(nanos as f64 / 1e9);
        }
        let sample = json!({"event": "sample", "repeats": repeats, "seconds": seconds, "memory": memory::resident().ok()});
        writeln!(stdout, "{sample}").unwrap();
        stdout.flush().unwrap();
    }
}
