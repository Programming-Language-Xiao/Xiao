//! B6 的固定负载测量入口。每一项在五个独立进程中按 F-3 采样。

#[path = "perf/cases.rs"]
#[allow(dead_code)] // B6 复用 B1 的输入和独立黄金，采样统一由 F-3 控制器驱动。
mod b1_cases;
#[path = "numeric_perf/b6_cases.rs"]
mod b6_cases;
#[path = "numeric_perf/calibration.rs"]
mod calibration;
#[path = "numeric_perf/cases.rs"]
mod cases;
#[path = "numeric_perf/identity.rs"]
mod identity;
#[path = "numeric_perf/memory.rs"]
mod memory;
#[path = "numeric_perf/native.rs"]
mod native;
#[path = "numeric_perf/process.rs"]
mod process;
#[path = "numeric_perf/worker.rs"]
mod worker;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use xiao_artifacts::Digest256;
use xiao_driver::{FrontendCompiler, FrontendRequest};

const BATCHES: usize = 5;
const MEMORY_LIMIT: u64 = 64 * 1024 * 1024;
const CATALOG: &str = "tests/fixtures/20o-b2-b6/catalog.json";

#[derive(Debug, Clone, Copy)]
enum Backend {
    Vm,
    Native(u8),
}

impl Backend {
    fn label(self) -> &'static str {
        match self {
            Self::Vm => "vm",
            Self::Native(0) => "o0",
            Self::Native(3) => "o3",
            _ => unreachable!(),
        }
    }

    fn seconds(self) -> f64 {
        match self {
            Self::Vm => 10.0,
            Self::Native(_) => 2.0,
        }
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
}

fn digest(data: &[u8]) -> String {
    Digest256::of_bytes(data).to_string()
}

fn compile(
    load: &b6_cases::Load,
    directory: &Path,
    backend: Backend,
) -> (PathBuf, Vec<String>, String) {
    let path = directory.join("program.xiao");
    fs::write(&path, &load.source).unwrap();
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text_at(&load.source, &path))
        .unwrap_or_else(|error| panic!("{} {}: {error:?}", load.spec.case, load.spec.profile));
    match backend {
        Backend::Vm => {
            let program = xiao_bytecode::lower_program(&artifact.ir);
            let raw =
                xiao_bytecode::encode_xiaoc(&program, xiao_bytecode::XiaocMetadata::new("b2-b6"))
                    .unwrap();
            let file = directory.join("program.xiaoc");
            fs::write(&file, &raw).unwrap();
            (
                std::env::current_exe().unwrap(),
                vec!["--worker-vm".into(), file.display().to_string()],
                digest(&raw),
            )
        }
        Backend::Native(level) => {
            let (program, fingerprint) = native::build(root(), directory, &artifact, level);
            // 验证工具的时钟桩仅截取标记。保留实际编译产物所对应的降低指纹。
            (program, Vec::new(), fingerprint)
        }
    }
}

fn measure(load: &b6_cases::Load, directory: &Path, backend: Backend) -> serde_json::Value {
    let (exe, args, fingerprint) = compile(load, directory, backend);
    let executable = identity::file(&exe);
    let files: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|path| identity::file(&path.unwrap().path()))
        .collect();
    let mut batches = Vec::new();
    let mut errors = Vec::new();
    for index in 0..BATCHES {
        match process::batch(
            &exe,
            &args,
            directory,
            &load.spec.case,
            &load.spec.profile,
            index,
        ) {
            Ok(batch) => batches.push(batch),
            Err(reason) => errors.push(serde_json::json!({"batch":index,"reason":reason})),
        }
    }
    assert_eq!(
        identity::file(&exe).sha256,
        executable.sha256,
        "执行文件在采样中发生变化"
    );
    for file in &files {
        assert_eq!(
            identity::file(Path::new(&file.path)).sha256,
            file.sha256,
            "采样产物发生变化"
        );
    }
    let mut seconds: Vec<_> = batches
        .iter()
        .map(|batch| batch.sample.seconds / batch.sample.repeats as f64)
        .collect();
    seconds.sort_by(f64::total_cmp);
    let median = (seconds.len() == BATCHES).then(|| seconds[BATCHES / 2]);
    let resident: Option<Vec<_>> = batches
        .iter()
        .map(|batch| batch.resident_increment_upper_bytes)
        .collect();
    let peak = resident.and_then(|readings| readings.into_iter().max());
    let calibrated = batches.len() == BATCHES && batches.iter().all(|batch| batch.calibrated);
    let status = if !errors.is_empty()
        || median.is_some_and(|value| value > backend.seconds())
        || peak.is_some_and(|value| value > MEMORY_LIMIT)
    {
        "failed"
    } else if !calibrated || peak.is_none() {
        "unverifiable"
    } else {
        "passed"
    };
    serde_json::json!({
        "load":load.spec,"backend":backend.label(),"status":status,
        "executable":executable,"artifacts":files,"fingerprint":fingerprint,
        "samples":batches,"batch_errors":errors,"seconds_limit":backend.seconds(),
        "normalized_seconds_sorted":seconds,"median_seconds":median,
        "resident_increment_upper_bytes":peak,"resident_limit_bytes":MEMORY_LIMIT,
        "calibrated":calibrated,
    })
}

fn assert_run_identity(fixed: &identity::Identity) {
    assert_eq!(
        identity::sources_b6(root()),
        fixed.files,
        "运行前后源码发生变化"
    );
    if let Some(runtime) = &fixed.runtime {
        assert_eq!(
            identity::file(Path::new(&runtime.path)).sha256,
            runtime.sha256,
            "运行期间 Runtime 发生变化"
        );
    }
    for tool in fixed.tools.values() {
        assert_eq!(
            identity::file(Path::new(&tool.file.path)).sha256,
            tool.file.sha256,
            "运行期间工具发生变化"
        );
    }
}

fn write_json(path: &Path, value: &serde_json::Value) {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    let file = fs::File::create_new(path).unwrap();
    use std::io::Write;
    (&file).write_all(&bytes).unwrap();
}

fn main() {
    let mut args = std::env::args().skip(1);
    let first = args
        .next()
        .expect("用法：--catalog 或 --backend vm|o0|o3 --output <新目录>");
    if first == "--worker-vm" {
        let loader = xiao_driver::standard_library_module_loader();
        worker::vm(
            Path::new(&args.next().expect("缺少测量字节码")),
            Some(&loader),
        );
        return;
    }
    let loads = b6_cases::catalog(root());
    let catalog = serde_json::json!({
        "schema":1,
        "platforms":["windows","linux","macos"],
        "backends":["vm","o0","o3"],
        "batches":BATCHES,
        "loads":loads.iter().map(|load| &load.spec).collect::<Vec<_>>(),
    });
    if first == "--catalog" {
        assert!(args.next().is_none(), "目录模式无需其他参数");
        println!("{}", serde_json::to_string_pretty(&catalog).unwrap());
        return;
    }
    assert_eq!(first, "--backend");
    let backend = match args.next().as_deref() {
        Some("vm") => Backend::Vm,
        Some("o0") => Backend::Native(0),
        Some("o3") => Backend::Native(3),
        _ => panic!("预期 vm、o0 或 o3"),
    };
    let mut suite = "all".to_owned();
    let mut case = None;
    let mut output = None;
    while let Some(arg) = args.next() {
        let value = args.next().unwrap_or_else(|| panic!("{arg} 缺少值"));
        match arg.as_str() {
            "--suite" => suite = value,
            "--case" => case = Some(value),
            "--output" => output = Some(PathBuf::from(value)),
            _ => panic!("未识别参数 {arg}"),
        }
    }
    if cfg!(debug_assertions) {
        panic!("B6 需要 release 构建");
    }
    let raw_catalog = fs::read(root().join(CATALOG)).unwrap();
    assert_eq!(
        catalog,
        serde_json::from_slice::<serde_json::Value>(&raw_catalog).unwrap(),
        "B6 生成的负载偏离冻结清单"
    );
    let plan: Vec<_> = loads
        .iter()
        .filter(|load| {
            (suite == "all" || suite == load.spec.suite)
                && case.as_ref().is_none_or(|case| case == &load.spec.case)
        })
        .collect();
    assert!(!plan.is_empty(), "B6 选择没有匹配固定负载");
    let mut fixed = identity::capture(root());
    fixed.files = identity::sources_b6(root());
    assert!(fixed.runtime.is_some(), "需要本轮新构建的 release Runtime");
    let output = output.expect("缺少 --output");
    fs::create_dir(&output).expect("B6 需要独占输出目录");
    let output = output.canonicalize().unwrap();
    write_json(
        &output.join("run.json"),
        &serde_json::json!({
            "identity":fixed,"catalog_sha256":digest(&raw_catalog),"backend":backend.label(),"suite":suite,"case":case,
            "abi":xiao_runtime_abi::ABI_ENCODED_VERSION,"codegen":xiao_codegen_llvm::CODEGEN_VERSION,
            "batches":BATCHES,"calibration_limit":process::MAX_REPEAT,
            "expected":plan.iter().map(|load| &load.spec).collect::<Vec<_>>(),
        }),
    );
    let clock = Instant::now();
    let mut prev = BTreeMap::new();
    let mut records = Vec::new();
    for (index, load) in plan.iter().enumerate() {
        let folder = output.join(format!("{index:03}"));
        fs::create_dir(&folder).unwrap();
        println!(
            "B6-2 {} {} {} start",
            backend.label(),
            load.spec.case,
            load.spec.profile
        );
        let mut row = std::panic::catch_unwind(|| measure(load, &folder, backend)).unwrap_or_else(|panic| {
            let reason = panic.downcast_ref::<String>().map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied()).unwrap_or("测量进程异常");
            serde_json::json!({"load":load.spec,"backend":backend.label(),"status":"failed","reason":reason})
        });
        let key = (&load.spec.case, &load.spec.profile);
        let median = row["median_seconds"]
            .as_f64()
            .filter(|_| row["calibrated"] == true);
        if let Some(ref previous) = load.spec.double_of {
            let prior: Option<&f64> = prev.get(&(&load.spec.case, previous));
            let ratio = median
                .zip(prior.copied())
                .filter(|&(_, value)| value > 0.0)
                .map(|(now, prior)| now / prior);
            let limit = load.spec.ratio_limit.unwrap();
            row["doubling_ratio"] = serde_json::json!(ratio);
            if ratio.is_some_and(|value| value > f64::from(limit)) {
                row["status"] = "failed".into();
            } else if ratio.is_none() && row["status"] == "passed" {
                row["status"] = "unverifiable".into();
            }
        }
        if let Some(median) = median {
            prev.insert(key, median);
        }
        write_json(&folder.join("result.json"), &row);
        println!(
            "B6-2 {} {} {} {}",
            backend.label(),
            load.spec.case,
            load.spec.profile,
            row["status"]
        );
        records.push(row);
    }
    assert_run_identity(&fixed);
    let passed = records.iter().all(|record| record["status"] == "passed");
    write_json(
        &output.join("result.json"),
        &serde_json::json!({
            "complete":records.len()==plan.len(),"passed":passed,"selected":plan.len(),"backend":backend.label(),
            "suite":suite,"case":case,"records":records,"elapsed_seconds":clock.elapsed().as_secs_f64(),
        }),
    );
    if !passed {
        std::process::exit(1);
    }
}
