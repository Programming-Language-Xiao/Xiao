//! 10Z 性能对照驱动器。
//!
//! 该二进制与 09R3 的研究基准程序隔离。它从 manifest.json 和 baseline.json
//! 读取输入，先做 Java、原生、VM 三侧语义互校，再把已经构建好的程序交给
//! 独立进程计时。构建和前端时间不在计时窗口内，原始样本和主机快照写入
//! 驱动器自己的报告。

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use xiao_artifacts::Digest256;
use xiao_bytecode::{XiaocMetadata, encode_xiaoc, lower_program};
use xiao_codegen_llvm::{CODEGEN_VERSION, CodegenOptions, TargetDescription, Toolchain};
use xiao_driver::{FrontendCompiler, FrontendNativeDriver, FrontendRequest, NativeBuildRequest};
use xiao_types::SeededRandom;

/// 10Z 驱动器内部实现声明。
const BOOTSTRAP_RESAMPLES: usize = 10_000;
/// 10Z 驱动器内部实现声明。
const BOOTSTRAP_SEED: u128 = 19_015;
/// 10Z 驱动器内部实现声明。
const CONFIDENCE_LEVEL: f64 = 0.95;
/// 10Z 驱动器内部实现声明。
const REPORT_VERSION: u32 = 1;

#[derive(Debug)]
/// 10Z 驱动器内部实现声明。
struct DriverError(String);

/// 10Z 驱动器内部实现声明。
type Result<T> = std::result::Result<T, DriverError>;

impl std::fmt::Display for DriverError {
    /// 10Z 驱动器内部实现声明。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DriverError {}

impl From<io::Error> for DriverError {
    /// 10Z 驱动器内部实现声明。
    fn from(error: io::Error) -> Self {
        Self(error.to_string())
    }
}

/// 10Z 驱动器内部实现声明。
fn driver_error(message: impl Into<String>) -> DriverError {
    DriverError(message.into())
}

#[derive(Clone, Debug, Deserialize)]
/// 10Z 驱动器内部实现声明。
struct Manifest {
    format_version: u8,
    opcode_min: u8,
    opcode_max: u8,
    warmup_iterations: usize,
    measurement_iterations: usize,
    max_call_depth: usize,
    benchmarks: Vec<BenchmarkSpec>,
}

#[derive(Clone, Debug, Deserialize)]
/// 10Z 驱动器内部实现声明。
struct BenchmarkSpec {
    id: String,
    family: String,
    source: String,
    entry: String,
    arguments: Vec<i64>,
    expected_outcome: String,
    #[serde(default)]
    expected_error_code: Option<String>,
    #[serde(default)]
    expected_value: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
/// 10Z 驱动器内部实现声明。
struct Baseline {
    baseline_id: String,
    status: String,
    runtime: BaselineRuntime,
    native: BaselineNative,
    protocol: BaselineProtocol,
}

#[derive(Clone, Debug, Deserialize)]
/// 10Z 驱动器内部实现声明。
struct BaselineRuntime {
    distribution: String,
    major_version: u32,
    jvm_args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
/// 10Z 驱动器内部实现声明。
struct BaselineNative {
    optimizer: String,
    optimization_level: String,
    xiao_passes_registered: bool,
}

#[derive(Clone, Debug, Deserialize)]
/// 10Z 驱动器内部实现声明。
struct BaselineProtocol {
    warmup_iterations: usize,
    measurement_iterations: usize,
    confidence_level: f64,
    interval: String,
    bootstrap_resamples: usize,
    bootstrap_seed: u128,
    noise_policy: String,
}

#[derive(Clone, Debug)]
/// 10Z 驱动器内部实现声明。
struct PreparedBenchmark {
    spec: BenchmarkSpec,
    xiaoc_path: PathBuf,
    native_path: PathBuf,
    preparation_error: Option<String>,
    max_call_depth: usize,
}

#[derive(Clone, Debug)]
/// 10Z 驱动器内部实现声明。
enum SideOutcome {
    Success { value: i64, raw: String },
    Error { code: String, raw: String },
    Failed { reason: String, raw: String },
}

#[derive(Clone, Debug, Serialize)]
/// 10Z 驱动器内部实现声明。
struct SideObservation {
    status: String,
    value: Option<i64>,
    error_code: Option<String>,
    raw_output: String,
    matches_expected: bool,
    reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
/// 10Z 驱动器内部实现声明。
struct SemanticReport {
    sides: BTreeMap<String, SideObservation>,
    comparable: bool,
    reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
/// 10Z 驱动器内部实现声明。
struct BootstrapSummary {
    statistic: String,
    resamples: usize,
    seed: u128,
    confidence_level: f64,
    point_estimate_ns: u128,
    lower_ns: u128,
    upper_ns: u128,
}

#[derive(Clone, Debug, Serialize)]
/// 10Z 驱动器内部实现声明。
struct TimingSummary {
    samples_ns: Vec<u128>,
    median_ns: u128,
    bootstrap: BootstrapSummary,
}

#[derive(Clone, Debug, Serialize)]
/// 10Z 驱动器内部实现声明。
struct CaseReport {
    id: String,
    family: String,
    semantic: SemanticReport,
    performance_status: String,
    performance: Option<BTreeMap<String, TimingSummary>>,
    reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
/// 10Z 驱动器内部实现声明。
struct ProtocolReport {
    warmup_iterations: usize,
    measurement_iterations: usize,
    max_call_depth: usize,
    timer: String,
    statistic: String,
    bootstrap_resamples: usize,
    bootstrap_seed: u128,
    confidence_level: f64,
    noise_policy: String,
}

#[derive(Clone, Debug, Serialize)]
/// 10Z 驱动器内部实现声明。
struct HostSnapshot {
    os: String,
    arch: String,
    cpu_model: String,
    kernel: String,
    memory: String,
    parallelism: usize,
    load: Option<String>,
    background_processes: Vec<String>,
    process_listing_note: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
/// 10Z 驱动器内部实现声明。
struct EnvironmentReport {
    java_version_text: String,
    java_version_text_sha256: String,
    javac_version_text: String,
    bun_version_text: Option<String>,
    rustc_version_text: String,
    clang_version_text: String,
    jvm_args: Vec<String>,
    build_fingerprint: String,
    java_major_expected: u32,
    java_distribution_expected: String,
    native_optimizer: String,
    native_optimization_level: String,
    xiao_passes_registered: bool,
    target_triple: String,
    host: HostSnapshot,
}

#[derive(Clone, Debug, Serialize)]
/// 10Z 驱动器内部实现声明。
struct DeterminismEvidence {
    first: Vec<u8>,
    second: Vec<u8>,
    byte_identical: bool,
}

#[derive(Clone, Debug, Serialize)]
/// 10Z 驱动器内部实现声明。
struct DriverReport {
    report_version: u32,
    stage: String,
    status: String,
    baseline_id: String,
    baseline_status: String,
    manifest_format_version: u8,
    opcode_range: [u8; 2],
    protocol: ProtocolReport,
    environment: EnvironmentReport,
    bootstrap_determinism: DeterminismEvidence,
    cases: Vec<CaseReport>,
}

#[derive(Clone, Debug)]
/// 10Z 驱动器内部实现声明。
struct Cli {
    id: Option<String>,
    output: Option<PathBuf>,
    self_test: bool,
    worker_vm: Option<PathBuf>,
    worker_max_call_depth: Option<usize>,
    help: bool,
}

/// 10Z 驱动器内部实现声明。
fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("10Z-PERFORMANCE-DRIVER-ERROR: {error}");
            ExitCode::from(1)
        }
    }
}

/// 10Z 驱动器内部实现声明。
fn real_main() -> Result<()> {
    let cli = parse_cli(env::args_os().skip(1))?;
    if cli.help {
        print_help();
        return Ok(());
    }
    if let Some(path) = cli.worker_vm {
        return run_vm_worker(&path, cli.worker_max_call_depth);
    }
    if cli.self_test {
        return run_self_test();
    }

    let package_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_path = package_root.join("manifest.json");
    let baseline_path = package_root.join("baseline.json");
    let manifest: Manifest = read_json(&manifest_path, "manifest.json")?;
    let baseline: Baseline = read_json(&baseline_path, "baseline.json")?;
    validate_protocol(&manifest, &baseline)?;
    let selected = select_benchmarks(&manifest.benchmarks, cli.id.as_deref())?;

    let dependencies = prepare_dependencies(&baseline)?;
    let temporary_root = temporary_root();
    fs::create_dir_all(&temporary_root)?;
    let result = run_driver(
        &package_root,
        &manifest,
        &baseline,
        selected,
        &dependencies,
        &temporary_root,
    );
    if let Err(cleanup_error) = fs::remove_dir_all(&temporary_root) {
        eprintln!("10Z-PERFORMANCE-DRIVER-WARN: 清理临时目录失败：{cleanup_error}");
    }
    let report = result?;
    let output = cli
        .output
        .unwrap_or_else(|| package_root.join("target/10z-performance-driver.json"));
    let output = if output.is_absolute() {
        output
    } else {
        env::current_dir()?.join(output)
    };
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| driver_error(format!("编码驱动器报告失败：{error}")))?;
    fs::write(&output, bytes)?;
    println!(
        "10Z-PERFORMANCE-DRIVER report={} cases={} status={}",
        output.display(),
        report.cases.len(),
        report.status
    );
    eprintln!("10Z-PERFORMANCE-DRIVER note=本机/CI 数字仅作功能证据，不写入受控性能结论");
    Ok(())
}

/// 10Z 驱动器内部实现声明。
fn parse_cli<I>(args: I) -> Result<Cli>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    let mut id = None;
    let mut output = None;
    let mut self_test = false;
    let mut worker_vm = None;
    let mut worker_max_call_depth = None;
    let mut help = false;
    let mut index = 0;
    while index < args.len() {
        let argument = args[index].to_string_lossy();
        match argument.as_ref() {
            "--id" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| driver_error("--id 需要基准 ID"))?;
                id = Some(value.to_string_lossy().into_owned());
            }
            "--output" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| driver_error("--output 需要路径"))?;
                output = Some(PathBuf::from(value));
            }
            "--self-test" => self_test = true,
            "--worker-vm" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| driver_error("--worker-vm 需要 .xiaoc 路径"))?;
                worker_vm = Some(PathBuf::from(value));
            }
            "--max-call-depth" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| driver_error("--max-call-depth 需要正整数"))?;
                worker_max_call_depth =
                    Some(value.to_string_lossy().parse::<usize>().map_err(|error| {
                        driver_error(format!("--max-call-depth 不是正整数：{error}"))
                    })?);
            }
            "--help" | "-h" => {
                help = true;
            }
            unknown => return Err(driver_error(format!("未知参数：{unknown}"))),
        }
        index += 1;
    }
    if self_test
        && (id.is_some()
            || output.is_some()
            || worker_vm.is_some()
            || worker_max_call_depth.is_some())
    {
        return Err(driver_error("--self-test 不能与其它运行参数同时使用"));
    }
    if worker_vm.is_some() && (id.is_some() || output.is_some() || self_test) {
        return Err(driver_error("--worker-vm 不能与其它运行参数同时使用"));
    }
    if worker_vm.is_none() && worker_max_call_depth.is_some() {
        return Err(driver_error("--max-call-depth 只能与 --worker-vm 同时使用"));
    }
    Ok(Cli {
        id,
        output,
        self_test,
        worker_vm,
        worker_max_call_depth,
        help,
    })
}

/// 10Z 驱动器内部实现声明。
fn print_help() {
    println!(
        "用法：performance_driver [--id <benchmark>] [--output <path>]\n\
         自检：performance_driver --self-test\n\
         内部 VM worker：performance_driver --worker-vm <path> --max-call-depth <n>"
    );
}

/// 10Z 驱动器内部实现声明。
fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, label: &str) -> Result<T> {
    let bytes = fs::read(path).map_err(|error| {
        driver_error(format!("读取 {label} 失败（{}）：{error}", path.display()))
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| driver_error(format!("解析 {label} 失败（{}）：{error}", path.display())))
}

/// 10Z 驱动器内部实现声明。
fn validate_protocol(manifest: &Manifest, baseline: &Baseline) -> Result<()> {
    if manifest.warmup_iterations != baseline.protocol.warmup_iterations
        || manifest.measurement_iterations != baseline.protocol.measurement_iterations
    {
        return Err(driver_error(format!(
            "manifest 与 baseline 的轮次不一致：manifest={}/{} baseline={}/{}",
            manifest.warmup_iterations,
            manifest.measurement_iterations,
            baseline.protocol.warmup_iterations,
            baseline.protocol.measurement_iterations
        )));
    }
    if baseline.protocol.interval != "percentile-bootstrap"
        || baseline.protocol.bootstrap_resamples != BOOTSTRAP_RESAMPLES
        || baseline.protocol.bootstrap_seed != BOOTSTRAP_SEED
        || (baseline.protocol.confidence_level - CONFIDENCE_LEVEL).abs() > f64::EPSILON
    {
        return Err(driver_error("baseline 的 bootstrap 协议不是 10Z 冻结值"));
    }
    if manifest.warmup_iterations == 0 || manifest.measurement_iterations == 0 {
        return Err(driver_error("预热和测量次数必须大于零"));
    }
    if manifest.max_call_depth == 0 {
        return Err(driver_error("max_call_depth 必须大于零"));
    }
    if manifest.benchmarks.is_empty() {
        return Err(driver_error("manifest 没有基准"));
    }
    let mut ids = BTreeSet::new();
    for benchmark in &manifest.benchmarks {
        if !ids.insert(&benchmark.id) {
            return Err(driver_error(format!(
                "manifest 中存在重复基准 ID：{}",
                benchmark.id
            )));
        }
        if benchmark.arguments.len() != 1 {
            return Err(driver_error(format!(
                "{} 需要恰好一个入口参数（收到 {}）",
                benchmark.id,
                benchmark.arguments.len()
            )));
        }
        match benchmark.expected_outcome.as_str() {
            "success" if benchmark.expected_value.is_some() => {}
            "error" if benchmark.expected_error_code.is_some() => {}
            _ => {
                return Err(driver_error(format!(
                    "{} 的 expected_outcome 与期望字段不匹配",
                    benchmark.id
                )));
            }
        }
    }
    Ok(())
}

/// 10Z 驱动器内部实现声明。
fn select_benchmarks<'a>(
    benchmarks: &'a [BenchmarkSpec],
    id: Option<&str>,
) -> Result<Vec<&'a BenchmarkSpec>> {
    if let Some(id) = id {
        let benchmark = benchmarks
            .iter()
            .find(|benchmark| benchmark.id == id)
            .ok_or_else(|| driver_error(format!("manifest 中没有基准：{id}")))?;
        Ok(vec![benchmark])
    } else {
        Ok(benchmarks.iter().collect())
    }
}

#[derive(Clone, Debug)]
/// 10Z 驱动器内部实现声明。
struct Dependencies {
    java: OsString,
    javac: OsString,
    target: TargetDescription,
    toolchain: Toolchain,
    java_version_text: String,
    java_version_sha256: String,
    javac_version_text: String,
    bun_version_text: Option<String>,
    rustc_version_text: String,
    clang_version_text: String,
    build_fingerprint: String,
}

/// 10Z 驱动器内部实现声明。
fn prepare_dependencies(baseline: &Baseline) -> Result<Dependencies> {
    let java = OsString::from("java");
    let javac = OsString::from("javac");
    let java_output = run_checked_command(Command::new(&java).arg("-version"), "java -version")?;
    let java_version_text = combined_output(&java_output);
    if java_version_text.trim().is_empty() {
        return Err(driver_error("java -version 没有输出版本文本"));
    }
    if !java_version_matches(&java_version_text, baseline.runtime.major_version) {
        return Err(driver_error(format!(
            "java 版本与 baseline 不符：期望 major={}，实际文本={}",
            baseline.runtime.major_version,
            java_version_text.trim()
        )));
    }
    let javac_output = run_checked_command(Command::new(&javac).arg("-version"), "javac -version")?;
    let javac_version_text = combined_output(&javac_output);
    if javac_version_text.trim().is_empty() {
        return Err(driver_error("javac -version 没有输出版本文本"));
    }
    let clang_value = env::var_os("XIAO_CLANG")
        .ok_or_else(|| driver_error("缺少 XIAO_CLANG；不能静默跳过原生侧"))?;
    let clang = PathBuf::from(clang_value);
    ensure_command_works(&clang, "--version", "XIAO_CLANG")?;
    let runtime = env::var_os("XIAO_RUNTIME_LIBRARY")
        .map(PathBuf::from)
        .ok_or_else(|| driver_error("缺少 XIAO_RUNTIME_LIBRARY；不能静默跳过原生侧"))?;
    if !runtime.is_file() {
        return Err(driver_error(format!(
            "XIAO_RUNTIME_LIBRARY 不是文件：{}",
            runtime.display()
        )));
    }
    let target = TargetDescription::host();
    let target_triple = env::var("XIAO_TARGET_TRIPLE")
        .map_err(|_| driver_error("缺少 XIAO_TARGET_TRIPLE；原生目标必须显式固定"))?;
    if target_triple != target.triple {
        return Err(driver_error(format!(
            "XIAO_TARGET_TRIPLE={} 与宿主目标 {} 不一致",
            target_triple, target.triple
        )));
    }
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let toolchain = Toolchain::new(&clang)
        .with_runtime_library(&runtime)
        .probe_native_static_libraries(&rustc, &target)
        .map_err(|error| driver_error(format!("探测 Runtime 原生静态库失败：{error}")))?
        .probe_versions()
        .map_err(|error| driver_error(format!("读取 LLVM 工具链版本失败：{error}")))?;
    let native_level = parse_native_level(&baseline.native.optimization_level)?;
    let build_fingerprint = toolchain
        .fingerprint_with_optimization(&target, CODEGEN_VERSION, native_level, "baseline")
        .as_str()
        .to_owned();
    let rustc_version_text = toolchain.versions.rustc.clone().unwrap_or_default();
    let clang_version_text = toolchain.versions.clang.clone();
    if baseline.runtime.major_version == 0 {
        return Err(driver_error("baseline.runtime.major_version 不能为零"));
    }
    Ok(Dependencies {
        java,
        javac,
        target,
        toolchain,
        java_version_sha256: Digest256::of_bytes(java_version_text.as_bytes()).as_hex(),
        java_version_text,
        javac_version_text,
        bun_version_text: command_version("bun", "--version"),
        rustc_version_text,
        clang_version_text,
        build_fingerprint,
    })
}

/// 10Z 驱动器内部实现声明。
fn command_version(program: &str, argument: &str) -> Option<String> {
    let output = Command::new(program).arg(argument).output().ok()?;
    output
        .status
        .success()
        .then(|| combined_output(&output).trim().to_owned())
}

/// 10Z 驱动器内部实现声明。
fn java_version_matches(text: &str, expected_major: u32) -> bool {
    let version = text
        .split('"')
        .nth(1)
        .or_else(|| {
            text.split_whitespace().find(|part| {
                part.chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_digit())
            })
        })
        .unwrap_or_default();
    let first = version.split('.').next().unwrap_or_default();
    let major = first.parse::<u32>().ok();
    match major {
        Some(1) => {
            version
                .split('.')
                .nth(1)
                .and_then(|value| value.parse::<u32>().ok())
                == Some(expected_major)
        }
        Some(value) => value == expected_major,
        None => false,
    }
}

/// 10Z 驱动器内部实现声明。
fn ensure_command_works(path: &Path, argument: &str, label: &str) -> Result<()> {
    let output = Command::new(path).arg(argument).output().map_err(|error| {
        driver_error(format!("无法执行 {label}（{}）：{error}", path.display()))
    })?;
    if !output.status.success() {
        return Err(driver_error(format!(
            "{label} 执行失败（{}）：{}",
            path.display(),
            combined_output(&output)
        )));
    }
    Ok(())
}

/// 10Z 驱动器内部实现声明。
fn run_checked_command(command: &mut Command, label: &str) -> Result<Output> {
    let output = command
        .output()
        .map_err(|error| driver_error(format!("无法执行 {label}：{error}")))?;
    if !output.status.success() {
        return Err(driver_error(format!(
            "{label} 缺失或执行失败：{}",
            combined_output(&output)
        )));
    }
    Ok(output)
}

/// 10Z 驱动器内部实现声明。
fn combined_output(output: &Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    text
}

/// 10Z 驱动器内部实现声明。
fn temporary_root() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    env::temp_dir().join(format!(
        "xiao-10z-performance-{}-{timestamp}",
        std::process::id()
    ))
}

/// 10Z 驱动器内部实现声明。
fn safe_file_name(id: &str) -> String {
    id.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

/// 10Z 驱动器内部实现声明。
fn native_file_name(id: &str) -> String {
    let name = safe_file_name(id);
    if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name
    }
}

/// 10Z 驱动器内部实现声明。
fn run_driver(
    package_root: &Path,
    manifest: &Manifest,
    baseline: &Baseline,
    selected: Vec<&BenchmarkSpec>,
    dependencies: &Dependencies,
    temporary_root: &Path,
) -> Result<DriverReport> {
    let classes = temporary_root.join("java-classes");
    fs::create_dir_all(&classes)?;
    let java_source = package_root.join("java/Benchmark.java");
    run_checked_command(
        Command::new(&dependencies.javac)
            .arg("-encoding")
            .arg("UTF-8")
            .arg("-d")
            .arg(&classes)
            .arg(&java_source),
        "javac Benchmark.java",
    )?;

    let environment = EnvironmentReport {
        java_version_text: dependencies.java_version_text.clone(),
        java_version_text_sha256: dependencies.java_version_sha256.clone(),
        javac_version_text: dependencies.javac_version_text.clone(),
        bun_version_text: dependencies.bun_version_text.clone(),
        rustc_version_text: dependencies.rustc_version_text.clone(),
        clang_version_text: dependencies.clang_version_text.clone(),
        jvm_args: baseline.runtime.jvm_args.clone(),
        build_fingerprint: dependencies.build_fingerprint.clone(),
        java_major_expected: baseline.runtime.major_version,
        java_distribution_expected: baseline.runtime.distribution.clone(),
        native_optimizer: baseline.native.optimizer.clone(),
        native_optimization_level: baseline.native.optimization_level.clone(),
        xiao_passes_registered: baseline.native.xiao_passes_registered,
        target_triple: dependencies.target.triple.clone(),
        host: collect_host_snapshot(),
    };
    let mut prepared = Vec::new();
    let compiler = FrontendCompiler::new();
    let native_driver = FrontendNativeDriver::new();
    let native_level = parse_native_level(&baseline.native.optimization_level)?;
    for spec in selected {
        let source_path = package_root.join(&spec.source);
        let xiaoc_path = temporary_root.join(format!("{}.xiaoc", safe_file_name(&spec.id)));
        let native_path = temporary_root.join(native_file_name(&spec.id));
        let source = match fs::read_to_string(&source_path) {
            Ok(source) => source,
            Err(error) => {
                prepared.push(unavailable_benchmark(
                    spec,
                    xiaoc_path,
                    native_path,
                    format!("读取源码失败：{error}"),
                    manifest.max_call_depth,
                ));
                continue;
            }
        };
        let source = append_entry_call(&source, spec);
        let frontend_request = FrontendRequest::from_text_at(&source, &source_path);
        let artifact = match compiler.compile(&frontend_request) {
            Ok(artifact) => artifact,
            Err(error) => {
                prepared.push(unavailable_benchmark(
                    spec,
                    xiaoc_path,
                    native_path,
                    format!("前端语义校验失败：{error}"),
                    manifest.max_call_depth,
                ));
                continue;
            }
        };
        let program = lower_program(artifact.ir());
        let xiaoc = encode_xiaoc(
            &program,
            XiaocMetadata::new(format!("benchmark-{}", spec.id)),
        );
        let xiaoc = match xiaoc {
            Ok(xiaoc) => xiaoc,
            Err(error) => {
                prepared.push(unavailable_benchmark(
                    spec,
                    xiaoc_path,
                    native_path,
                    format!("编码 .xiaoc 失败：{error}"),
                    manifest.max_call_depth,
                ));
                continue;
            }
        };
        fs::write(&xiaoc_path, xiaoc)?;
        let codegen_options = CodegenOptions::for_target(dependencies.target.clone())
            .with_optimization_level(native_level)
            .map_err(|error| driver_error(format!("原生优化级别无效：{error}")))?;
        let request = NativeBuildRequest::new(
            frontend_request.clone(),
            dependencies.target.clone(),
            dependencies.toolchain.clone(),
            &native_path,
        )
        .with_codegen_options(codegen_options);
        if let Err(error) = native_driver.build_artifact(&artifact, &request) {
            prepared.push(unavailable_benchmark(
                spec,
                xiaoc_path,
                native_path,
                format!("原生构建失败：{error}"),
                manifest.max_call_depth,
            ));
            continue;
        }
        prepared.push(PreparedBenchmark {
            spec: spec.clone(),
            xiaoc_path,
            native_path,
            preparation_error: None,
            max_call_depth: manifest.max_call_depth,
        });
    }

    let mut cases = Vec::new();
    for benchmark in &prepared {
        cases.push(run_case(
            benchmark,
            manifest,
            baseline,
            dependencies,
            &classes,
        )?);
    }
    let status = if cases.iter().all(|case| {
        case.performance_status == "measured" || case.performance_status == "data-insufficient"
    }) {
        "development-evidence"
    } else {
        "failed"
    };
    Ok(DriverReport {
        report_version: REPORT_VERSION,
        stage: "10Z".to_owned(),
        status: status.to_owned(),
        baseline_id: baseline.baseline_id.clone(),
        baseline_status: baseline.status.clone(),
        manifest_format_version: manifest.format_version,
        opcode_range: [manifest.opcode_min, manifest.opcode_max],
        protocol: ProtocolReport {
            warmup_iterations: manifest.warmup_iterations,
            measurement_iterations: manifest.measurement_iterations,
            max_call_depth: manifest.max_call_depth,
            timer: "std::time::Instant; whole child-process wall-clock".to_owned(),
            statistic: baseline.protocol.interval.clone(),
            bootstrap_resamples: baseline.protocol.bootstrap_resamples,
            bootstrap_seed: baseline.protocol.bootstrap_seed,
            confidence_level: baseline.protocol.confidence_level,
            noise_policy: baseline.protocol.noise_policy.clone(),
        },
        environment,
        bootstrap_determinism: bootstrap_determinism_evidence(),
        cases,
    })
}

/// 10Z 驱动器内部实现声明。
fn unavailable_benchmark(
    spec: &BenchmarkSpec,
    xiaoc_path: PathBuf,
    native_path: PathBuf,
    reason: String,
    max_call_depth: usize,
) -> PreparedBenchmark {
    PreparedBenchmark {
        spec: spec.clone(),
        xiaoc_path,
        native_path,
        preparation_error: Some(reason),
        max_call_depth,
    }
}

/// 10Z 驱动器内部实现声明。
fn append_entry_call(source: &str, spec: &BenchmarkSpec) -> String {
    let argument = spec
        .arguments
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{source}\nprint({}({argument}))\n", spec.entry)
}

/// 10Z 驱动器内部实现声明。
fn parse_native_level(value: &str) -> Result<u8> {
    let level = value
        .strip_prefix('O')
        .ok_or_else(|| driver_error(format!("原生优化级别格式错误：{value}")))?
        .parse::<u8>()
        .map_err(|error| driver_error(format!("原生优化级别格式错误：{value}：{error}")))?;
    if level > 3 {
        return Err(driver_error(format!("原生优化级别超出 O0-O3：{value}")));
    }
    Ok(level)
}

/// 10Z 驱动器内部实现声明。
fn run_case(
    benchmark: &PreparedBenchmark,
    manifest: &Manifest,
    baseline: &Baseline,
    dependencies: &Dependencies,
    classes: &Path,
) -> Result<CaseReport> {
    if let Some(reason) = &benchmark.preparation_error {
        let mut sides = BTreeMap::new();
        for side in ["java", "native", "vm"] {
            sides.insert(
                side.to_owned(),
                SideObservation {
                    status: "unavailable".to_owned(),
                    value: None,
                    error_code: None,
                    raw_output: String::new(),
                    matches_expected: false,
                    reason: Some(reason.clone()),
                },
            );
        }
        let semantic = SemanticReport {
            sides,
            comparable: false,
            reason: Some(reason.clone()),
        };
        return Ok(CaseReport {
            id: benchmark.spec.id.clone(),
            family: benchmark.spec.family.clone(),
            semantic,
            performance_status: "data-insufficient".to_owned(),
            performance: None,
            reason: Some(reason.clone()),
        });
    }
    let java = run_java_once(benchmark, baseline, dependencies, classes)?;
    let native = run_native_once(benchmark)?;
    let vm = run_vm_once(benchmark)?;
    let mut observations = BTreeMap::new();
    observations.insert(
        "java".to_owned(),
        compare_expected(&benchmark.spec, "java", java),
    );
    observations.insert(
        "native".to_owned(),
        compare_expected(&benchmark.spec, "native", native),
    );
    observations.insert("vm".to_owned(), compare_expected(&benchmark.spec, "vm", vm));
    let mismatch = observations
        .values()
        .find(|observation| !observation.matches_expected);
    let semantic = SemanticReport {
        comparable: mismatch.is_none(),
        reason: mismatch.and_then(|observation| observation.reason.clone()),
        sides: observations,
    };
    if !semantic.comparable {
        return Ok(CaseReport {
            id: benchmark.spec.id.clone(),
            family: benchmark.spec.family.clone(),
            performance_status: "data-insufficient".to_owned(),
            reason: semantic.reason.clone(),
            semantic,
            performance: None,
        });
    }

    let mut timings = BTreeMap::new();
    for side in ["java", "native", "vm"] {
        let samples = measure_side(
            benchmark,
            side,
            manifest.warmup_iterations,
            manifest.measurement_iterations,
            baseline,
            dependencies,
            classes,
        )?;
        timings.insert(side.to_owned(), summarize_samples(&samples));
    }
    Ok(CaseReport {
        id: benchmark.spec.id.clone(),
        family: benchmark.spec.family.clone(),
        semantic,
        performance_status: "measured".to_owned(),
        performance: Some(timings),
        reason: None,
    })
}

/// 10Z 驱动器内部实现声明。
fn compare_expected(spec: &BenchmarkSpec, side: &str, outcome: SideOutcome) -> SideObservation {
    match outcome {
        SideOutcome::Success { value, raw } => {
            let matches = spec.expected_outcome == "success" && spec.expected_value == Some(value);
            let reason = if matches {
                None
            } else if spec.expected_outcome == "error" {
                Some(format!(
                    "{side} 返回 success={value}，但清单要求 error={}",
                    spec.expected_error_code.as_deref().unwrap_or("<missing>")
                ))
            } else {
                Some(format!(
                    "{side} 返回值 {value}，清单期望 {:?}",
                    spec.expected_value
                ))
            };
            SideObservation {
                status: "success".to_owned(),
                value: Some(value),
                error_code: None,
                raw_output: raw,
                matches_expected: matches,
                reason,
            }
        }
        SideOutcome::Error { code, raw } => {
            let matches = spec.expected_outcome == "error"
                && spec.expected_error_code.as_deref() == Some(code.as_str());
            let reason = if matches {
                None
            } else {
                Some(format!(
                    "{side} 返回错误 {code}，清单期望 {}",
                    spec.expected_error_code
                        .as_deref()
                        .or_else(|| spec.expected_value.map(|_| "success"))
                        .unwrap_or("<missing>")
                ))
            };
            SideObservation {
                status: "error".to_owned(),
                value: None,
                error_code: Some(code),
                raw_output: raw,
                matches_expected: matches,
                reason,
            }
        }
        SideOutcome::Failed { reason, raw } => SideObservation {
            status: "failed".to_owned(),
            value: None,
            error_code: None,
            raw_output: raw,
            matches_expected: false,
            reason: Some(format!("{side} 执行失败：{reason}")),
        },
    }
}

/// 10Z 驱动器内部实现声明。
fn run_java_once(
    benchmark: &PreparedBenchmark,
    baseline: &Baseline,
    dependencies: &Dependencies,
    classes: &Path,
) -> Result<SideOutcome> {
    let argument = benchmark.spec.arguments[0].to_string();
    let output = Command::new(&dependencies.java)
        .args(&baseline.runtime.jvm_args)
        .arg("-cp")
        .arg(classes)
        .arg("Benchmark")
        .arg(&benchmark.spec.id)
        .arg(argument)
        .output()
        .map_err(|error| driver_error(format!("java 执行 {} 失败：{error}", benchmark.spec.id)))?;
    parse_java_or_worker_output(&output, "java")
}

/// 10Z 驱动器内部实现声明。
fn run_native_once(benchmark: &PreparedBenchmark) -> Result<SideOutcome> {
    let output = Command::new(&benchmark.native_path)
        .output()
        .map_err(|error| driver_error(format!("原生执行 {} 失败：{error}", benchmark.spec.id)))?;
    parse_native_output(&output)
}

/// 10Z 驱动器内部实现声明。
fn run_vm_once(benchmark: &PreparedBenchmark) -> Result<SideOutcome> {
    let executable = env::current_exe()
        .map_err(|error| driver_error(format!("定位 VM worker 可执行文件失败：{error}")))?;
    let output = Command::new(executable)
        .arg("--worker-vm")
        .arg(&benchmark.xiaoc_path)
        .arg("--max-call-depth")
        .arg(benchmark.max_call_depth.to_string())
        .output()
        .map_err(|error| {
            driver_error(format!(
                "VM worker 执行 {} 失败：{error}",
                benchmark.spec.id
            ))
        })?;
    parse_java_or_worker_output(&output, "vm")
}

/// 10Z 驱动器内部实现声明。
fn parse_java_or_worker_output(output: &Output, side: &str) -> Result<SideOutcome> {
    let raw = combined_output(output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    if let Some(value) = line.strip_prefix("success\t") {
        return value.trim().parse::<i64>().map_or_else(
            |error| {
                Ok(SideOutcome::Failed {
                    reason: format!("{side} success 值不是整数：{error}"),
                    raw: raw.clone(),
                })
            },
            |value| {
                Ok(SideOutcome::Success {
                    value,
                    raw: raw.clone(),
                })
            },
        );
    }
    if let Some(code) = line.strip_prefix("error\t") {
        return Ok(SideOutcome::Error {
            code: code.trim().to_owned(),
            raw,
        });
    }
    if let Some(code) = find_error_code(&raw) {
        return Ok(SideOutcome::Error { code, raw });
    }
    Ok(SideOutcome::Failed {
        reason: if output.status.success() {
            format!("{side} 没有产生 success/error 协议行")
        } else {
            format!("进程退出状态 {}", output.status)
        },
        raw,
    })
}

/// 10Z 驱动器内部实现声明。
fn parse_native_output(output: &Output) -> Result<SideOutcome> {
    let raw = combined_output(output);
    if let Some(code) = find_error_code(&raw) {
        return Ok(SideOutcome::Error { code, raw });
    }
    if !output.status.success() {
        return Ok(SideOutcome::Failed {
            reason: format!("进程退出状态 {}", output.status),
            raw,
        });
    }
    match String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<i64>()
    {
        Ok(value) => Ok(SideOutcome::Success { value, raw }),
        Err(error) => Ok(SideOutcome::Failed {
            reason: format!("原生输出不是单个整数：{error}"),
            raw,
        }),
    }
}

/// 10Z 驱动器内部实现声明。
fn find_error_code(text: &str) -> Option<String> {
    text.split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .find(|token| token.starts_with('X') && token.matches('-').count() >= 2)
        .map(ToOwned::to_owned)
}

/// 10Z 驱动器内部实现声明。
fn measure_side(
    benchmark: &PreparedBenchmark,
    side: &str,
    warmup_iterations: usize,
    measurement_iterations: usize,
    baseline: &Baseline,
    dependencies: &Dependencies,
    classes: &Path,
) -> Result<Vec<u128>> {
    for _ in 0..warmup_iterations {
        let (output, _) = run_timed_side(benchmark, side, baseline, dependencies, classes)?;
        let observation = match side {
            "java" | "vm" => parse_java_or_worker_output(&output, side)?,
            "native" => parse_native_output(&output)?,
            _ => return Err(driver_error(format!("未知计时侧：{side}"))),
        };
        ensure_matches(&benchmark.spec, side, observation)?;
    }
    let mut samples = Vec::with_capacity(measurement_iterations);
    for _ in 0..measurement_iterations {
        let (output, elapsed) = run_timed_side(benchmark, side, baseline, dependencies, classes)?;
        let observation = match side {
            "java" | "vm" => parse_java_or_worker_output(&output, side)?,
            "native" => parse_native_output(&output)?,
            _ => return Err(driver_error(format!("未知计时侧：{side}"))),
        };
        ensure_matches(&benchmark.spec, side, observation)?;
        samples.push(elapsed.as_nanos());
    }
    Ok(samples)
}

/// 10Z 驱动器内部实现声明。
fn run_timed_side(
    benchmark: &PreparedBenchmark,
    side: &str,
    baseline: &Baseline,
    dependencies: &Dependencies,
    classes: &Path,
) -> Result<(Output, std::time::Duration)> {
    let mut command = match side {
        "java" => {
            let argument = benchmark.spec.arguments[0].to_string();
            let mut command = Command::new(&dependencies.java);
            command
                .args(&baseline.runtime.jvm_args)
                .arg("-cp")
                .arg(classes)
                .arg("Benchmark")
                .arg(&benchmark.spec.id)
                .arg(argument);
            command
        }
        "native" => Command::new(&benchmark.native_path),
        "vm" => {
            let executable = env::current_exe()
                .map_err(|error| driver_error(format!("定位 VM worker 可执行文件失败：{error}")))?;
            let mut command = Command::new(executable);
            command
                .arg("--worker-vm")
                .arg(&benchmark.xiaoc_path)
                .arg("--max-call-depth")
                .arg(benchmark.max_call_depth.to_string());
            command
        }
        _ => return Err(driver_error(format!("未知计时侧：{side}"))),
    };
    command.stdin(Stdio::null());
    let start = Instant::now();
    let output = command
        .output()
        .map_err(|error| driver_error(format!("启动 {side} 计时进程失败：{error}")))?;
    let elapsed = start.elapsed();
    Ok((output, elapsed))
}

/// 10Z 驱动器内部实现声明。
fn ensure_matches(spec: &BenchmarkSpec, side: &str, outcome: SideOutcome) -> Result<()> {
    let observation = compare_expected(spec, side, outcome);
    if observation.matches_expected {
        Ok(())
    } else {
        Err(driver_error(observation.reason.unwrap_or_else(|| {
            format!("{side} 计时后输出与清单不一致")
        })))
    }
}

/// 10Z 驱动器内部实现声明。
fn summarize_samples(samples: &[u128]) -> TimingSummary {
    assert!(!samples.is_empty(), "计时样本不能为空");
    let median = percentile(samples, 0.5);
    let mut random = SeededRandom::new(BOOTSTRAP_SEED);
    let mut statistics = Vec::with_capacity(BOOTSTRAP_RESAMPLES);
    for _ in 0..BOOTSTRAP_RESAMPLES {
        let mut resample = Vec::with_capacity(samples.len());
        for _ in samples {
            let index = (random.next_u64() as usize) % samples.len();
            resample.push(samples[index]);
        }
        statistics.push(percentile(&resample, 0.5));
    }
    statistics.sort_unstable();
    let lower_index = ((BOOTSTRAP_RESAMPLES as f64 * 0.025).floor() as usize)
        .min(BOOTSTRAP_RESAMPLES.saturating_sub(1));
    let upper_index = ((BOOTSTRAP_RESAMPLES as f64 * 0.975).ceil() as usize)
        .saturating_sub(1)
        .min(BOOTSTRAP_RESAMPLES.saturating_sub(1));
    TimingSummary {
        samples_ns: samples.to_vec(),
        median_ns: median,
        bootstrap: BootstrapSummary {
            statistic: "median".to_owned(),
            resamples: BOOTSTRAP_RESAMPLES,
            seed: BOOTSTRAP_SEED,
            confidence_level: CONFIDENCE_LEVEL,
            point_estimate_ns: median,
            lower_ns: statistics[lower_index],
            upper_ns: statistics[upper_index],
        },
    }
}

/// 10Z 驱动器内部实现声明。
fn percentile(samples: &[u128], quantile: f64) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let position = ((sorted.len().saturating_sub(1) as f64) * quantile).round() as usize;
    sorted[position.min(sorted.len().saturating_sub(1))]
}

/// 10Z 驱动器内部实现声明。
fn bootstrap_determinism_evidence() -> DeterminismEvidence {
    let samples = [11_u128, 13, 17, 19, 23];
    let first = serde_json::to_vec(&summarize_samples(&samples)).unwrap_or_default();
    let second = serde_json::to_vec(&summarize_samples(&samples)).unwrap_or_default();
    DeterminismEvidence {
        byte_identical: first == second,
        first,
        second,
    }
}

/// 10Z 驱动器内部实现声明。
fn collect_host_snapshot() -> HostSnapshot {
    let (load, process_command) = if cfg!(target_os = "linux") {
        (
            fs::read_to_string("/proc/loadavg")
                .ok()
                .map(|value| value.trim().to_owned()),
            Some(("ps", vec!["-eo", "pid,comm,%cpu,%mem", "--no-headers"])),
        )
    } else if cfg!(target_os = "windows") {
        (
            Some("unavailable: Windows load counter is not portable in this driver".to_owned()),
            Some(("tasklist", vec!["/fo", "csv", "/nh"])),
        )
    } else {
        (
            Some("unavailable: host load source is not implemented for this OS".to_owned()),
            Some(("ps", vec!["-eo", "pid,comm,%cpu,%mem"])),
        )
    };
    let mut background_processes = Vec::new();
    let mut process_listing_note = None;
    if let Some((program, arguments)) = process_command {
        match Command::new(program).args(arguments).output() {
            Ok(output) if output.status.success() => {
                background_processes = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .take(200)
                    .map(ToOwned::to_owned)
                    .collect();
            }
            Ok(output) => {
                process_listing_note = Some(format!("进程列表退出状态 {}", output.status))
            }
            Err(error) => process_listing_note = Some(format!("读取进程列表失败：{error}")),
        }
    }
    HostSnapshot {
        os: env::consts::OS.to_owned(),
        arch: env::consts::ARCH.to_owned(),
        cpu_model: host_cpu_model(),
        kernel: host_kernel(),
        memory: host_memory(),
        parallelism: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        load,
        background_processes,
        process_listing_note,
    }
}

/// 10Z 驱动器内部实现声明。
fn host_cpu_model() -> String {
    if cfg!(target_os = "linux") {
        fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|value| {
                value.lines().find_map(|line| {
                    line.strip_prefix("model name:")
                        .or_else(|| line.strip_prefix("Hardware:"))
                        .map(str::trim)
                        .filter(|model| !model.is_empty())
                        .map(ToOwned::to_owned)
                })
            })
            .unwrap_or_else(|| "unavailable: /proc/cpuinfo model missing".to_owned())
    } else if cfg!(target_os = "windows") {
        env::var("PROCESSOR_IDENTIFIER")
            .unwrap_or_else(|_| "unavailable: PROCESSOR_IDENTIFIER missing".to_owned())
    } else {
        command_version("sysctl", "-n hw.model")
            .unwrap_or_else(|| "unavailable: sysctl hw.model failed".to_owned())
    }
}

/// 10Z 驱动器内部实现声明。
fn host_kernel() -> String {
    if cfg!(target_os = "linux") {
        command_version("uname", "-sr").unwrap_or_else(|| "unavailable: uname failed".to_owned())
    } else if cfg!(target_os = "windows") {
        command_version("cmd", "/c ver").unwrap_or_else(|| "unavailable: ver failed".to_owned())
    } else {
        "unavailable: kernel query is not implemented for this OS".to_owned()
    }
}

/// 10Z 驱动器内部实现声明。
fn host_memory() -> String {
    if cfg!(target_os = "linux") {
        fs::read_to_string("/proc/meminfo")
            .ok()
            .map(|value| value.lines().take(3).collect::<Vec<_>>().join("; "))
            .unwrap_or_else(|| "unavailable: /proc/meminfo read failed".to_owned())
    } else {
        "unavailable: memory query is not implemented for this OS".to_owned()
    }
}

/// 10Z 驱动器内部实现声明。
fn run_vm_worker(path: &Path, max_call_depth: Option<usize>) -> Result<()> {
    let bytes = fs::read(path).map_err(|error| {
        driver_error(format!(
            "读取 VM worker 产物失败（{}）：{error}",
            path.display()
        ))
    })?;
    let mut options = xiao_vm::VmOptions::default();
    if let Some(max_call_depth) = max_call_depth {
        if max_call_depth == 0 {
            return Err(driver_error("VM worker 的 max_call_depth 必须大于零"));
        }
        options.max_call_depth = max_call_depth;
    }
    let outcome =
        xiao_vm::run_xiaoc_production(&bytes, options, xiao_vm::DEFAULT_EVENT_CAPACITY, None)
            .map_err(|error| driver_error(format!("加载 VM worker 产物失败：{error}")))?;
    if outcome.result.is_success() {
        let output = outcome
            .events
            .iter()
            .filter_map(|event| match event {
                xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        let value = output
            .trim()
            .parse::<i64>()
            .map_err(|error| driver_error(format!("VM worker 成功但输出不是整数：{error}")))?;
        println!("success\t{value}");
    } else {
        let code = outcome.result.error_code().unwrap_or("X09-VM-UNKNOWN");
        println!("error\t{code}");
    }
    Ok(())
}

/// 10Z 驱动器内部实现声明。
fn run_self_test() -> Result<()> {
    let evidence = bootstrap_determinism_evidence();
    if !evidence.byte_identical {
        return Err(driver_error("bootstrap 自证失败：两次输出不同"));
    }
    let expected = BenchmarkSpec {
        id: "self-test".to_owned(),
        family: "self-test".to_owned(),
        source: String::new(),
        entry: String::new(),
        arguments: vec![0],
        expected_outcome: "success".to_owned(),
        expected_error_code: None,
        expected_value: Some(42),
    };
    let bad = compare_expected(
        &expected,
        "synthetic",
        SideOutcome::Success {
            value: 41,
            raw: "success\t41".to_owned(),
        },
    );
    if bad.matches_expected {
        return Err(driver_error("语义闸门自证失败：错误期望值被接受"));
    }
    println!(
        "10Z-PERFORMANCE-DRIVER self-test bootstrap_byte_identical={} semantic_guard_rejects_wrong_value=true",
        evidence.byte_identical
    );
    Ok(())
}

#[cfg(test)]
/// 10Z 驱动器自检模块。
mod tests {
    use std::ffi::OsString;

    use super::{
        BenchmarkSpec, SideOutcome, compare_expected, java_version_matches, parse_cli,
        parse_native_level, summarize_samples,
    };

    #[test]
    /// 10Z 驱动器内部实现声明。
    fn bootstrap_is_deterministic() {
        let first = serde_json::to_vec(&summarize_samples(&[10_u128, 20, 30])).unwrap();
        let second = serde_json::to_vec(&summarize_samples(&[10_u128, 20, 30])).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    /// 10Z 驱动器内部实现声明。
    fn semantic_guard_rejects_wrong_value() {
        let spec = BenchmarkSpec {
            id: "test".to_owned(),
            family: "test".to_owned(),
            source: String::new(),
            entry: String::new(),
            arguments: vec![1],
            expected_outcome: "success".to_owned(),
            expected_error_code: None,
            expected_value: Some(7),
        };
        let observation = compare_expected(
            &spec,
            "synthetic",
            SideOutcome::Success {
                value: 8,
                raw: "success\t8".to_owned(),
            },
        );
        assert!(!observation.matches_expected);
    }

    #[test]
    /// 10Z 驱动器帮助参数不应被报告为运行失败。
    fn help_is_a_successful_cli_state() {
        let cli = parse_cli([OsString::from("--help")]).expect("help should parse");
        assert!(cli.help);
    }

    #[test]
    /// 10Z 驱动器按 Java 版本字符串识别旧式与现代主版本。
    fn java_version_major_matching_is_stable() {
        assert!(java_version_matches("openjdk version \"21.0.4\"", 21));
        assert!(java_version_matches("java version \"1.8.0_491\"", 8));
        assert!(!java_version_matches("openjdk version \"17.0.12\"", 21));
    }

    #[test]
    /// 10Z 驱动器只接受冻结的 O0 到 O3 原生级别。
    fn native_level_parser_rejects_unfrozen_values() {
        assert_eq!(parse_native_level("O2").unwrap(), 2);
        assert!(parse_native_level("O4").is_err());
        assert!(parse_native_level("fast").is_err());
    }
}
