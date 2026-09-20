//! 09R3 基准与差分驱动。
//!
//! 该工具只使用 `std::time::Instant` 计时，不依赖 `criterion` 或其他基准框架。
//! 它先由真实 Xiao 源码经过 `FrontendCompiler` 和同一份 TAC，再分别运行三种
//! 载体；报告只记录同一次构建、同一输入和同一协议下得到的事实。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use xiao_bytecode::research::{
    FORMAT_VERSION, OPCODE_MAX, OPCODE_MIN, OperandWidth, TacProgram, encode, lower_program,
};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_runtime::RuntimeValue;
use xiao_vm::research::{
    HybridCarrier, RegisterCarrier, RunOutcome, RunResult, StackCarrier, VmEvent, VmOptions,
    run_with, run_with_values,
};

/// 09R3 报告 JSON 的结构版本。
const REPORT_VERSION: u32 = 1;

/// 09R3 基准清单及固定测量协议。
#[derive(Clone, Debug, Deserialize)]
struct Manifest {
    format_version: u8,
    opcode_min: u8,
    opcode_max: u8,
    warmup_iterations: usize,
    measurement_iterations: usize,
    max_call_depth: usize,
    benchmarks: Vec<BenchmarkSpec>,
}

/// 单个真实 Xiao 源码基准的入口和期望结果。
#[derive(Clone, Debug, Deserialize)]
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

/// 三种待比较的字节码机载体。
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Machine {
    Stack,
    Register,
    Hybrid,
}

impl Machine {
    const ALL: [Self; 3] = [Self::Stack, Self::Register, Self::Hybrid];

    /// 返回报告中使用的稳定机型名称。
    const fn name(self) -> &'static str {
        match self {
            Self::Stack => "stack",
            Self::Register => "register",
            Self::Hybrid => "hybrid",
        }
    }
}

/// 前端和降低阶段产出的可执行基准。
#[derive(Clone, Debug)]
struct CompiledBenchmark {
    spec: BenchmarkSpec,
    program: TacProgram,
}

/// 可序列化的 Xiao 错误链，用于三种载体的语义逐项比较。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ErrorRecord {
    code: String,
    message_id: String,
    cause: Option<Box<ErrorRecord>>,
    suppressed: Vec<ErrorRecord>,
}

/// 可序列化的运行时值快照。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value")]
enum ValueRecord {
    Int(i64),
    Sint(i32),
    Lint(String),
    Float(u64),
    Sfloat(u32),
    Lfloat(String),
    Bool(bool),
    Str(String),
    Array(Vec<ValueRecord>),
    Tuple(Vec<ValueRecord>),
    DictTable(Vec<(String, ValueRecord)>),
    DictColumn(Vec<(String, ValueRecord)>),
    Set(Vec<ValueRecord>),
    None,
    Other(String),
}

/// 资源释放事件的稳定快照。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ReleaseRecord {
    scope: u32,
    exit: String,
    value: u32,
    kind: String,
}

/// 单次载体运行的语义、资源释放和深度观测。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct Observation {
    outcome: String,
    error_code: Option<String>,
    error: Option<ErrorRecord>,
    value: Option<ValueRecord>,
    releases: Vec<ReleaseRecord>,
    max_call_depth: usize,
}

/// 一个基准在单个载体上的结果。
#[derive(Clone, Debug, Serialize)]
struct MachineSemantic {
    machine: String,
    observation: Observation,
    matches_expected: bool,
}

/// 语义差分报告中的一个基准用例。
#[derive(Clone, Debug, Serialize)]
struct DifferentialCase {
    id: String,
    family: String,
    source: String,
    entry: String,
    arguments: Vec<i64>,
    expected_outcome: String,
    expected_error_code: Option<String>,
    expected_value: Option<i64>,
    machines: Vec<MachineSemantic>,
    all_machines_equal: bool,
}

/// 三种载体共享输入后的语义差分报告。
#[derive(Clone, Debug, Serialize)]
struct DifferentialReport {
    report_version: u32,
    stage: &'static str,
    platform: &'static str,
    format_version: u8,
    opcode_range: [u8; 2],
    protocol: ProtocolRecord,
    cases: Vec<DifferentialCase>,
    passed: bool,
}

/// 所有报告共同记录的 Rust 和计时协议。
#[derive(Clone, Debug, Serialize)]
struct ProtocolRecord {
    rust_toolchain: &'static str,
    profile: &'static str,
    opt_level: u8,
    codegen_units: u16,
    lto: bool,
    warmup_iterations: usize,
    measurement_iterations: usize,
    statistic: &'static str,
    baseline: &'static str,
    input_scale: &'static str,
}

/// 一组排序后的计时样本及其中位数、四分位数。
#[derive(Clone, Debug, Serialize)]
struct TimingSummary {
    samples_ns: Vec<u128>,
    median_ns: u128,
    q1_ns: u128,
    q3_ns: u128,
}

/// 一个基准在三种载体上的性能数据。
#[derive(Clone, Debug, Serialize)]
struct PerformanceCase {
    id: String,
    family: String,
    timings: BTreeMap<String, TimingSummary>,
    ratios_to_stack: BTreeMap<String, f64>,
}

/// 按基准族聚合的性能报告。
#[derive(Clone, Debug, Serialize)]
struct PerformanceReport {
    report_version: u32,
    stage: &'static str,
    platform: &'static str,
    format_version: u8,
    opcode_range: [u8; 2],
    protocol: ProtocolRecord,
    cases: Vec<PerformanceCase>,
    family_medians: BTreeMap<String, BTreeMap<String, u128>>,
    family_ratios_to_stack: BTreeMap<String, BTreeMap<String, f64>>,
    global_register_ratio_to_stack: f64,
    global_hybrid_ratio_to_stack: f64,
    threshold: ThresholdRecord,
}

/// R1-AD 吞吐门槛及其解释。
#[derive(Clone, Debug, Serialize)]
struct ThresholdRecord {
    throughput_improvement_required: f64,
    cold_start_and_memory_and_size_regression_allowed: f64,
    interpretation: &'static str,
}

/// 一个基准的内存与载体运行指标。
#[derive(Clone, Debug, Serialize)]
struct MemoryCase {
    id: String,
    family: String,
    machines: BTreeMap<String, MemoryMachineRecord>,
}

/// 单个载体的峰值工作集和逻辑栈指标。
#[derive(Clone, Debug, Serialize)]
struct MemoryMachineRecord {
    peak_working_set_bytes: Option<u64>,
    peak_stack_depth: usize,
    stack_map_entries: usize,
    spill_count: u64,
    call_save_count: u64,
}

/// Windows 原生内存报告。
#[derive(Clone, Debug, Serialize)]
struct MemoryReport {
    report_version: u32,
    stage: &'static str,
    platform: &'static str,
    format_version: u8,
    opcode_range: [u8; 2],
    peak_memory_measurement: &'static str,
    cases: Vec<MemoryCase>,
}

/// 一个基准在两种操作数布局下的编码体积。
#[derive(Clone, Debug, Serialize)]
struct EncodingCase {
    id: String,
    family: String,
    widths: BTreeMap<String, usize>,
}

/// Windows 原生编码体积报告。
#[derive(Clone, Debug, Serialize)]
struct EncodingReport {
    report_version: u32,
    stage: &'static str,
    platform: &'static str,
    format_version: u8,
    opcode_range: [u8; 2],
    cases: Vec<EncodingCase>,
}

/// 09R3 冻结选择及失效规则。
#[derive(Clone, Debug, Serialize)]
struct FreezeRecord {
    report_version: u32,
    stage: &'static str,
    platform_status: PlatformStatus,
    selected_machine: String,
    selection_basis: &'static str,
    family_direction_explanation: String,
    format_version: u8,
    opcode_range: [u8; 2],
    frozen_items: Vec<&'static str>,
    invalidation_rule: &'static str,
}

/// 各操作系统和开发环境的复现状态。
#[derive(Clone, Debug, Serialize)]
struct PlatformStatus {
    windows_native: &'static str,
    linux: &'static str,
    macos: &'static str,
    wsl_and_containers: &'static str,
}

/// 运行基准驱动并将报告写入固定目录。
fn main() {
    if let Err(error) = run() {
        eprintln!("09R3 基准失败：{error}");
        std::process::exit(1);
    }
}

/// 校验平台、工具链、清单并生成四份报告和冻结记录。
fn run() -> Result<(), String> {
    if !cfg!(windows) {
        return Err(
            "09R3 本轮只接受 Windows 原生采数；Linux/macOS 请保留为 pending-reproduction，WSL/容器不计入验收"
                .to_owned(),
        );
    }
    if cfg!(debug_assertions) {
        return Err("09R3 测量必须使用 release 配置；请运行 cargo run --release".to_owned());
    }
    let rustc_version = Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|error| format!("读取 rustc 版本失败：{error}"))?;
    let rustc_version = String::from_utf8_lossy(&rustc_version.stdout);
    if !rustc_version.starts_with("rustc 1.96.0 ") {
        return Err(format!(
            "09R3 要求 Rust 1.96.0，当前为 {}",
            rustc_version.trim()
        ));
    }
    let root = repository_root();
    let manifest_path = root.join("tests/benchmarks/manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("读取清单 {} 失败：{error}", manifest_path.display()))?;
    let manifest: Manifest =
        serde_json::from_str(&manifest_text).map_err(|error| format!("解析清单失败：{error}"))?;
    validate_manifest(&manifest)?;
    let output_dir = root.join("tests/benchmarks/reports");
    fs::create_dir_all(&output_dir).map_err(|error| format!("创建报告目录失败：{error}"))?;

    let compiled = compile_all(&root, &manifest)?;
    let differential = differential_report(&compiled, &manifest)?;
    write_report(
        &output_dir.join("windows-native-semantic-differential.json"),
        &differential,
    )?;
    let performance = performance_report(&compiled, &manifest)?;
    write_report(
        &output_dir.join("windows-native-performance.json"),
        &performance,
    )?;
    let memory = memory_report(&compiled, &manifest)?;
    write_report(&output_dir.join("windows-native-memory.json"), &memory)?;
    let encoding = encoding_report(&compiled, &manifest)?;
    write_report(
        &output_dir.join("windows-native-encoding-size.json"),
        &encoding,
    )?;
    let freeze = freeze_record(&performance, &manifest);
    write_report(&output_dir.join("09r3-freeze.json"), &freeze)?;

    if !differential.passed {
        return Err("语义差分或期望结果不一致，未写入可通过的冻结结论".to_owned());
    }
    println!(
        "09R3 Windows 原生报告已写入 {}（{} 个基准）",
        output_dir.display(),
        compiled.len()
    );
    Ok(())
}

/// 从基准 crate 位置推导仓库根目录。
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("基准 crate 应位于仓库 tests/benchmarks")
        .to_path_buf()
}

/// 校验布局版本、测量协议、基准族和每项期望结果。
fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.format_version != FORMAT_VERSION
        || manifest.opcode_min != OPCODE_MIN
        || manifest.opcode_max != OPCODE_MAX
    {
        return Err(format!(
            "清单编码范围必须是 FORMAT_VERSION={FORMAT_VERSION}、opcode {OPCODE_MIN}..{OPCODE_MAX}"
        ));
    }
    if manifest.warmup_iterations == 0 || manifest.measurement_iterations < 5 {
        return Err("预热必须大于 0，测量重复次数至少为 5，才能计算四分位区间".to_owned());
    }
    if manifest.max_call_depth == 0 {
        return Err("最大调用深度必须大于 0".to_owned());
    }
    let mut ids = BTreeMap::new();
    for benchmark in &manifest.benchmarks {
        if benchmark.id.is_empty()
            || benchmark.family.is_empty()
            || benchmark.entry.is_empty()
            || benchmark.arguments.is_empty()
        {
            return Err(format!(
                "基准 {:?} 缺少 id/family/entry/arguments",
                benchmark.id
            ));
        }
        if benchmark.expected_outcome == "success" && benchmark.expected_value.is_none() {
            return Err(format!("成功基准 {} 必须登记 expected_value", benchmark.id));
        }
        if benchmark.expected_outcome != "success" && benchmark.expected_error_code.is_none() {
            return Err(format!(
                "失败基准 {} 必须登记 expected_error_code",
                benchmark.id
            ));
        }
        if ids.insert(&benchmark.id, ()).is_some() {
            return Err(format!("基准 id 重复：{}", benchmark.id));
        }
    }
    let families = manifest
        .benchmarks
        .iter()
        .map(|item| item.family.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for required in [
        "deep-expression-arithmetic",
        "named-local-loop",
        "deep-call-recursion",
        "container-dense",
    ] {
        if !families.contains(required) {
            return Err(format!("缺少 R1-G 基准族：{required}"));
        }
    }
    Ok(())
}

/// 用真实 Xiao 源码编译并降低所有基准，且预先验证版本 3 编码。
fn compile_all(root: &Path, manifest: &Manifest) -> Result<Vec<CompiledBenchmark>, String> {
    let compiler = FrontendCompiler::new();
    manifest
        .benchmarks
        .iter()
        .map(|spec| {
            let source_path = root.join("tests/benchmarks").join(&spec.source);
            let source = fs::read_to_string(&source_path)
                .map_err(|error| format!("读取基准 {} 失败：{error}", source_path.display()))?;
            let artifact = compiler
                .compile(&FrontendRequest::from_text_at(source, source_path.clone()))
                .map_err(|error| {
                    format!("基准 {} 前端编译失败：{:?}", spec.id, error.diagnostics())
                })?;
            let program = lower_program(&artifact.ir);
            if !program.unsupported.is_empty() {
                return Err(format!(
                    "基准 {} 含未降低构造：{:?}；R3 编码体积不可测",
                    spec.id, program.unsupported
                ));
            }
            let Some(function) = program
                .functions
                .iter()
                .find(|function| function.name == spec.entry)
                .cloned()
            else {
                return Err(format!("基准 {} 找不到入口函数 {}", spec.id, spec.entry));
            };
            let mut program = program;
            program.functions[0] = function;
            for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
                let encoded = encode(&program, width)
                    .map_err(|error| format!("基准 {} 编码失败：{error}", spec.id))?;
                if encoded.bytes.get(4).copied() != Some(FORMAT_VERSION) {
                    return Err(format!("基准 {} 编码未使用布局版本 3", spec.id));
                }
            }
            Ok(CompiledBenchmark {
                spec: spec.clone(),
                program,
            })
        })
        .collect()
}

/// 将清单中的最大调用深度转换为 VM 运行选项。
fn options(manifest: &Manifest) -> VmOptions {
    VmOptions {
        max_call_depth: manifest.max_call_depth,
    }
}

/// 在指定字节码机载体上执行一个已编译基准。
fn run_machine(machine: Machine, benchmark: &CompiledBenchmark, manifest: &Manifest) -> RunOutcome {
    let options = options(manifest);
    if benchmark.spec.arguments.is_empty() {
        return match machine {
            Machine::Stack => run_with::<StackCarrier>(&benchmark.program, options),
            Machine::Register => run_with::<RegisterCarrier>(&benchmark.program, options),
            Machine::Hybrid => run_with::<HybridCarrier>(&benchmark.program, options),
        };
    }
    let arguments = benchmark
        .spec
        .arguments
        .iter()
        .copied()
        .map(RuntimeValue::Int)
        .collect::<Vec<_>>();
    match machine {
        Machine::Stack => run_with_values::<StackCarrier>(&benchmark.program, options, &arguments),
        Machine::Register => {
            run_with_values::<RegisterCarrier>(&benchmark.program, options, &arguments)
        }
        Machine::Hybrid => {
            run_with_values::<HybridCarrier>(&benchmark.program, options, &arguments)
        }
    }
}

/// 将 VM 结果转换为可比较、可序列化的观测值。
fn observe(outcome: &RunOutcome) -> Observation {
    let (outcome_name, error_code, error) = match &outcome.result {
        RunResult::Success => ("success".to_owned(), None, None),
        RunResult::Error(error) => (
            "error".to_owned(),
            Some(error.code().to_owned()),
            Some(error_record(error)),
        ),
        RunResult::Fatal(error) => (
            "fatal".to_owned(),
            Some(error.code().to_owned()),
            Some(fatal_record(error)),
        ),
    };
    let releases = outcome
        .events
        .iter()
        .filter_map(|event| match event {
            VmEvent::ValueReleased {
                scope,
                exit,
                value,
                kind,
            } => Some(ReleaseRecord {
                scope: *scope,
                exit: exit.clone(),
                value: *value,
                kind: kind.clone(),
            }),
            _ => None,
        })
        .collect();
    Observation {
        outcome: outcome_name,
        error_code,
        error,
        value: outcome.value.as_ref().map(value_record),
        releases,
        max_call_depth: outcome.metrics.max_call_depth,
    }
}

/// 递归复制可恢复 Xiao 错误的稳定字段。
fn error_record(error: &xiao_diagnostics::XiaoError) -> ErrorRecord {
    ErrorRecord {
        code: error.code().to_owned(),
        message_id: error.message_id().to_owned(),
        cause: error.cause().map(|cause| Box::new(error_record(cause))),
        suppressed: error.suppressed().iter().map(error_record).collect(),
    }
}

/// 递归复制不可恢复致命错误的稳定字段。
fn fatal_record(error: &xiao_diagnostics::FatalError) -> ErrorRecord {
    ErrorRecord {
        code: error.code().to_owned(),
        message_id: error.message_id().to_owned(),
        cause: error.cause().map(|cause| Box::new(fatal_record(cause))),
        suppressed: error.suppressed().iter().map(fatal_record).collect(),
    }
}

/// 将运行时值转换为报告中的递归快照。
fn value_record(value: &RuntimeValue) -> ValueRecord {
    match value {
        RuntimeValue::Int(value) => ValueRecord::Int(*value),
        RuntimeValue::Sint(value) => ValueRecord::Sint(*value),
        RuntimeValue::Lint(value) => ValueRecord::Lint(value.clone()),
        RuntimeValue::Float(value) => ValueRecord::Float(value.to_bits()),
        RuntimeValue::Sfloat(value) => ValueRecord::Sfloat(value.to_bits()),
        RuntimeValue::Lfloat(value) => ValueRecord::Lfloat(value.clone()),
        RuntimeValue::Bool(value) => ValueRecord::Bool(*value),
        RuntimeValue::Str(value) => ValueRecord::Str(
            value
                .to_string()
                .unwrap_or_else(|error| format!("<string-error:{error}>")),
        ),
        RuntimeValue::Array(value) => ValueRecord::Array(
            value
                .with_elements(|values| values.iter().map(value_record).collect())
                .unwrap_or_default(),
        ),
        RuntimeValue::Tuple(value) => ValueRecord::Tuple(
            value
                .with_elements(|values| values.iter().map(value_record).collect())
                .unwrap_or_default(),
        ),
        RuntimeValue::DictTable(value) => ValueRecord::DictTable(
            value
                .with_entries(|entries| {
                    entries
                        .iter()
                        .map(|(key, value)| (key.clone(), value_record(value)))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        RuntimeValue::DictColumn(value) => ValueRecord::DictColumn(
            value
                .with_entries(|entries| {
                    entries
                        .iter()
                        .map(|(key, value)| (key.clone(), value_record(value)))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        RuntimeValue::Set(value) => ValueRecord::Set(
            value
                .with_elements(|values| values.iter().map(value_record).collect())
                .unwrap_or_default(),
        ),
        RuntimeValue::None => ValueRecord::None,
        other => ValueRecord::Other(other.type_name()),
    }
}

/// 检查一个观测是否同时满足清单期望和错误契约。
fn expected_matches(benchmark: &CompiledBenchmark, observation: &Observation) -> bool {
    let value_matches = match benchmark.spec.expected_value {
        Some(expected) => observation
            .value
            .as_ref()
            .is_some_and(|value| matches!(value, ValueRecord::Int(actual) if *actual == expected)),
        None => observation.value.is_none(),
    };
    observation.outcome == benchmark.spec.expected_outcome
        && observation.error_code == benchmark.spec.expected_error_code
        && value_matches
}

/// 运行三种载体并生成语义差分报告。
fn differential_report(
    benchmarks: &[CompiledBenchmark],
    manifest: &Manifest,
) -> Result<DifferentialReport, String> {
    let mut cases = Vec::with_capacity(benchmarks.len());
    let mut passed = true;
    for benchmark in benchmarks {
        let mut machine_reports = Vec::with_capacity(Machine::ALL.len());
        let mut observations = Vec::with_capacity(Machine::ALL.len());
        for machine in Machine::ALL {
            let observation = observe(&run_machine(machine, benchmark, manifest));
            let matches_expected = expected_matches(benchmark, &observation);
            passed &= matches_expected;
            observations.push(observation.clone());
            machine_reports.push(MachineSemantic {
                machine: machine.name().to_owned(),
                observation,
                matches_expected,
            });
        }
        let all_machines_equal = observations.windows(2).all(|pair| {
            pair[0].outcome == pair[1].outcome
                && pair[0].error_code == pair[1].error_code
                && pair[0].error == pair[1].error
                && pair[0].value == pair[1].value
                && pair[0].releases == pair[1].releases
                && pair[0].max_call_depth == pair[1].max_call_depth
        });
        passed &= all_machines_equal;
        cases.push(DifferentialCase {
            id: benchmark.spec.id.clone(),
            family: benchmark.spec.family.clone(),
            source: format!("tests/benchmarks/{}", benchmark.spec.source),
            entry: benchmark.spec.entry.clone(),
            arguments: benchmark.spec.arguments.clone(),
            expected_outcome: benchmark.spec.expected_outcome.clone(),
            expected_error_code: benchmark.spec.expected_error_code.clone(),
            expected_value: benchmark.spec.expected_value,
            machines: machine_reports,
            all_machines_equal,
        });
    }
    Ok(DifferentialReport {
        report_version: REPORT_VERSION,
        stage: "09R3",
        platform: "windows-native",
        format_version: manifest.format_version,
        opcode_range: [manifest.opcode_min, manifest.opcode_max],
        protocol: protocol(manifest),
        cases,
        passed,
    })
}

/// 按固定预热和采样协议生成性能报告。
fn performance_report(
    benchmarks: &[CompiledBenchmark],
    manifest: &Manifest,
) -> Result<PerformanceReport, String> {
    let mut cases = Vec::with_capacity(benchmarks.len());
    for benchmark in benchmarks {
        let mut timings = BTreeMap::new();
        for machine in Machine::ALL {
            for _ in 0..manifest.warmup_iterations {
                std::hint::black_box(run_machine(machine, benchmark, manifest));
            }
            let mut samples = Vec::with_capacity(manifest.measurement_iterations);
            for _ in 0..manifest.measurement_iterations {
                let start = Instant::now();
                let outcome = run_machine(machine, benchmark, manifest);
                std::hint::black_box(outcome);
                samples.push(start.elapsed().as_nanos());
            }
            samples.sort_unstable();
            timings.insert(machine.name().to_owned(), timing_summary(samples));
        }
        let stack = timings
            .get("stack")
            .map(|summary| summary.median_ns)
            .unwrap_or(1) as f64;
        let ratios_to_stack = timings
            .iter()
            .map(|(machine, summary)| (machine.clone(), summary.median_ns as f64 / stack))
            .collect();
        cases.push(PerformanceCase {
            id: benchmark.spec.id.clone(),
            family: benchmark.spec.family.clone(),
            timings,
            ratios_to_stack,
        });
    }
    let mut family_medians: BTreeMap<String, BTreeMap<String, u128>> = BTreeMap::new();
    let mut family_ratios_to_stack: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    for case in &cases {
        for machine in Machine::ALL {
            let median = case.timings[machine.name()].median_ns;
            family_medians
                .entry(case.family.clone())
                .or_default()
                .entry(machine.name().to_owned())
                .and_modify(|current| *current = current.saturating_add(median))
                .or_insert(median);
        }
    }
    let family_counts = cases
        .iter()
        .fold(BTreeMap::<String, usize>::new(), |mut counts, case| {
            *counts.entry(case.family.clone()).or_default() += 1;
            counts
        });
    for (family, values) in &mut family_medians {
        let count = family_counts[family] as u128;
        for value in values.values_mut() {
            *value /= count;
        }
        let stack = values["stack"] as f64;
        family_ratios_to_stack.insert(
            family.clone(),
            values
                .iter()
                .map(|(machine, value)| (machine.clone(), *value as f64 / stack))
                .collect(),
        );
    }
    let global = global_medians(&family_medians);
    let stack = global["stack"] as f64;
    Ok(PerformanceReport {
        report_version: REPORT_VERSION,
        stage: "09R3",
        platform: "windows-native",
        format_version: manifest.format_version,
        opcode_range: [manifest.opcode_min, manifest.opcode_max],
        protocol: protocol(manifest),
        cases,
        family_medians,
        family_ratios_to_stack,
        global_register_ratio_to_stack: global["register"] as f64 / stack,
        global_hybrid_ratio_to_stack: global["hybrid"] as f64 / stack,
        threshold: ThresholdRecord {
            throughput_improvement_required: 0.10,
            cold_start_and_memory_and_size_regression_allowed: 0.10,
            interpretation: "时间比值低于 0.90 才表示相对栈式吞吐至少提升 10%；四族必须分别查看",
        },
    })
}

/// 从排序后的纳秒样本计算中位数和四分位数。
fn timing_summary(samples_ns: Vec<u128>) -> TimingSummary {
    let median_ns = percentile(&samples_ns, 50);
    let q1_ns = percentile(&samples_ns, 25);
    let q3_ns = percentile(&samples_ns, 75);
    TimingSummary {
        samples_ns,
        median_ns,
        q1_ns,
        q3_ns,
    }
}

/// 计算一个排序样本切片的离散百分位位置。
fn percentile(samples: &[u128], percentile: usize) -> u128 {
    if samples.is_empty() {
        return 0;
    }
    let index = ((samples.len() - 1) * percentile) / 100;
    samples[index]
}

/// 对各基准族的中位数再次取全局中位数。
fn global_medians(
    family_medians: &BTreeMap<String, BTreeMap<String, u128>>,
) -> BTreeMap<String, u128> {
    let mut result = BTreeMap::new();
    for machine in Machine::ALL {
        let mut values = family_medians
            .values()
            .filter_map(|family| family.get(machine.name()).copied())
            .collect::<Vec<_>>();
        values.sort_unstable();
        result.insert(machine.name().to_owned(), percentile(&values, 50));
    }
    result
}

/// 采集峰值工作集和载体逻辑栈指标。
fn memory_report(
    benchmarks: &[CompiledBenchmark],
    manifest: &Manifest,
) -> Result<MemoryReport, String> {
    let mut cases = Vec::with_capacity(benchmarks.len());
    for benchmark in benchmarks {
        let mut machines = BTreeMap::new();
        for machine in Machine::ALL {
            for _ in 0..manifest.warmup_iterations {
                std::hint::black_box(run_machine(machine, benchmark, manifest));
            }
            let before = peak_working_set_bytes();
            let outcome = run_machine(machine, benchmark, manifest);
            let after = peak_working_set_bytes();
            machines.insert(
                machine.name().to_owned(),
                MemoryMachineRecord {
                    peak_working_set_bytes: before.into_iter().chain(after).max(),
                    peak_stack_depth: outcome.metrics.max_stack_depth,
                    stack_map_entries: outcome.metrics.stack_map_entries,
                    spill_count: outcome.metrics.spill_count,
                    call_save_count: outcome.metrics.call_save_count,
                },
            );
        }
        cases.push(MemoryCase {
            id: benchmark.spec.id.clone(),
            family: benchmark.spec.family.clone(),
            machines,
        });
    }
    Ok(MemoryReport {
        report_version: REPORT_VERSION,
        stage: "09R3",
        platform: "windows-native",
        format_version: manifest.format_version,
        opcode_range: [manifest.opcode_min, manifest.opcode_max],
        peak_memory_measurement: "Windows GetProcessMemoryInfo PeakWorkingSetSize；同一 harness 进程内记录，逻辑载体峰值另列",
        cases,
    })
}

/// 计算 LEB128 与定宽操作数布局的编码体积。
fn encoding_report(
    benchmarks: &[CompiledBenchmark],
    manifest: &Manifest,
) -> Result<EncodingReport, String> {
    let mut cases = Vec::with_capacity(benchmarks.len());
    for benchmark in benchmarks {
        let mut widths = BTreeMap::new();
        for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
            let encoded = encode(&benchmark.program, width)
                .map_err(|error| format!("编码体积报告失败 {}：{error}", benchmark.spec.id))?;
            if encoded.bytes.get(4).copied() != Some(FORMAT_VERSION) {
                return Err(format!("{} 的编码格式不是版本 3", benchmark.spec.id));
            }
            widths.insert(width_name(width).to_owned(), encoded.bytes.len());
        }
        cases.push(EncodingCase {
            id: benchmark.spec.id.clone(),
            family: benchmark.spec.family.clone(),
            widths,
        });
    }
    Ok(EncodingReport {
        report_version: REPORT_VERSION,
        stage: "09R3",
        platform: "windows-native",
        format_version: manifest.format_version,
        opcode_range: [manifest.opcode_min, manifest.opcode_max],
        cases,
    })
}

/// 根据性能门槛选择最终冻结的字节码机载体。
fn freeze_record(performance: &PerformanceReport, manifest: &Manifest) -> FreezeRecord {
    let selected_machine = if performance.global_register_ratio_to_stack <= 0.90 {
        "register".to_owned()
    } else if performance.global_hybrid_ratio_to_stack <= 0.90 {
        "hybrid".to_owned()
    } else {
        "stack".to_owned()
    };
    FreezeRecord {
        report_version: REPORT_VERSION,
        stage: "09R3",
        platform_status: PlatformStatus {
            windows_native: "completed",
            linux: "pending-reproduction",
            macos: "pending-reproduction",
            wsl_and_containers: "development-only; excluded from acceptance",
        },
        selected_machine,
        selection_basis: "按 Windows 原生性能报告的全局中位数比值选择；四族明细保留在性能报告",
        family_direction_explanation: family_direction_explanation(performance),
        format_version: manifest.format_version,
        opcode_range: [manifest.opcode_min, manifest.opcode_max],
        frozen_items: vec![
            "最终字节码机型",
            "寄存器类别与分配策略",
            "函数调用 ABI",
            "异常与清理转移 ABI",
            "指令编码与版本字段",
            "源码映射格式",
            "性能阈值与基准协议",
        ],
        invalidation_rule: "指令集、ABI 或编码任何一项事后改动，本轮全部基准数字作废并需重新冻结",
    }
}

/// 把四个基准族相对栈式的方向和比值写入冻结记录。
fn family_direction_explanation(performance: &PerformanceReport) -> String {
    let details = performance
        .family_ratios_to_stack
        .iter()
        .map(|(family, ratios)| {
            let register = ratios.get("register").copied().unwrap_or(f64::NAN);
            let hybrid = ratios.get("hybrid").copied().unwrap_or(f64::NAN);
            let register_direction = if register < 1.0 {
                "寄存器式快于栈式"
            } else {
                "寄存器式慢于栈式"
            };
            let hybrid_direction = if hybrid < 1.0 {
                "混合式快于栈式"
            } else {
                "混合式慢于栈式"
            };
            format!(
                "{family}：{register_direction}（比值 {register:.3}），{hybrid_direction}（比值 {hybrid:.3}）"
            )
        })
        .collect::<Vec<_>>();
    format!(
        "按族分别观察：{}。族间方向相反时以这些明细解释，不以全局中位数掩盖。",
        details.join("；")
    )
}

/// 返回所有报告复用的固定测量协议。
fn protocol(manifest: &Manifest) -> ProtocolRecord {
    ProtocolRecord {
        rust_toolchain: "1.96.0",
        profile: "release",
        opt_level: 3,
        codegen_units: 1,
        lto: false,
        warmup_iterations: manifest.warmup_iterations,
        measurement_iterations: manifest.measurement_iterations,
        statistic: "median plus q1/q3; samples sorted by elapsed nanoseconds",
        baseline: "same-build Windows native stack carrier",
        input_scale: "manifest arguments are fixed and shared by all three machines",
    }
}

/// 返回操作数布局在报告中的稳定名称。
fn width_name(width: OperandWidth) -> &'static str {
    match width {
        OperandWidth::Leb128 => "leb128",
        OperandWidth::FixedU16 => "fixed-u16",
    }
}

/// 以 UTF-8 格式写入缩进后的 JSON 报告。
fn write_report<T: Serialize>(path: &Path, report: &T) -> Result<(), String> {
    let text =
        serde_json::to_string_pretty(report).map_err(|error| format!("序列化报告失败：{error}"))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("写入 {} 失败：{error}", path.display()))
}

#[cfg(windows)]
#[repr(C)]
/// Windows 工作集 API 使用的进程内存计数器布局。
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

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
}

#[cfg(windows)]
#[link(name = "psapi")]
unsafe extern "system" {
    fn GetProcessMemoryInfo(
        process: *mut std::ffi::c_void,
        counters: *mut ProcessMemoryCounters,
        size: u32,
    ) -> i32;
}

#[cfg(windows)]
/// 读取当前进程的峰值工作集大小。
fn peak_working_set_bytes() -> Option<u64> {
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
    // SAFETY: Windows API receives a valid current-process handle and a pointer to a
    // correctly sized, writable PROCESS_MEMORY_COUNTERS-compatible structure.
    let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    (ok != 0).then_some(counters.peak_working_set_size as u64)
}

#[cfg(not(windows))]
/// 非 Windows 平台不把工作集数字计入本轮验收。
fn peak_working_set_bytes() -> Option<u64> {
    None
}
