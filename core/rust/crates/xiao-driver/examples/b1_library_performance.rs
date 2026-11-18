//! B1 F10 显式测量工具：真实完整产品源码、独立黄金、一次预热和五次中位数。

#[path = "perf/calibration.rs"]
/// 对零读数和量化时钟保持有界增长的批次校准。
mod calibration;
#[path = "perf/cases.rs"]
/// 固定产品负载与独立黄金。
mod cases;
#[path = "perf/memory.rs"]
/// 各平台进程驻留内存上界。
mod memory;
#[path = "perf/native.rs"]
/// 原生受控编译、时钟窗口和结果校验。
mod native;
#[path = "perf/run_directory.rs"]
/// 原子创建本次运行独占的产物目录。
mod run_directory;
#[path = "perf/source_snapshot.rs"]
/// 固定实际标准库加载根与报告使用的源码摘要。
mod source_snapshot;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_vm::{RunResult, StackCarrier, Vm, VmEvent, VmEventSink, VmOptions};

/// 单次固定批次的窗口上界；内存为 OS 历史高水位减当前基线的保守上界。
#[derive(Clone, Debug, serde::Serialize)]
struct Sample {
    seconds: f64,
    resident_increment_upper_bytes: u64,
}

/// 两个整数 print 是窗口标记；不写终端，验证和输出在窗口外。
struct Sink {
    start: Option<(Instant, u64)>,
    samples: Rc<RefCell<Vec<Sample>>>,
    validated: usize,
    events: usize,
}

impl VmEventSink for Sink {
    /// 只处理窗口标记与窗口外黄金结果，并在窗口内保留独立看门狗。
    fn record(&mut self, event: VmEvent) {
        self.events += 1;
        if self.events & 8191 == 0
            && self
                .start
                .as_ref()
                .is_some_and(|(start, _)| start.elapsed() > Duration::from_secs(60))
        {
            panic!("测量窗口超过 60 秒看门狗");
        }
        if let VmEvent::IntrinsicOutput { text } = event {
            match text.as_str() {
                "0\n" => {
                    assert!(self.start.is_none());
                    let baseline = memory::resident().0;
                    self.start = Some((Instant::now(), baseline));
                }
                "1\n" => {
                    let (start, baseline) = self.start.take().unwrap();
                    let seconds = start.elapsed().as_secs_f64();
                    let peak = memory::resident().1;
                    self.samples.borrow_mut().push(Sample {
                        seconds,
                        resident_increment_upper_bytes: peak.saturating_sub(baseline),
                    });
                }
                "true\n" => self.validated += 1,
                other => panic!("黄金校验失败或出现非预期输出：{other:?}"),
            }
        }
    }
}

/// 计时包围已编译调用，包含标记的少量格式化/清理上界，不含编译、输入生成、验证或终端 I/O。
fn vm_samples(
    root: &Path,
    case: &cases::Workload,
    repeats: usize,
    sample_count: usize,
) -> Vec<Sample> {
    let source = cases::source(root, case, repeats, sample_count, false);
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap();
    let program = xiao_bytecode::lower_program(&artifact.ir);
    let samples = Rc::new(RefCell::new(Vec::with_capacity(sample_count)));
    let sink = Sink {
        start: None,
        samples: samples.clone(),
        validated: 0,
        events: 0,
    };
    let mut vm = Vm::<StackCarrier, _>::new(&program, VmOptions::default(), sink);
    let result = vm.run();
    assert!(matches!(result, RunResult::Success), "{result:?}");
    let sink = vm.into_sink();
    assert_eq!(sink.validated, sample_count);
    assert!(sink.start.is_none());
    let result = samples.borrow().clone();
    assert_eq!(result.len(), sample_count);
    result
}

/// 每条完成后保存检查点；超时/构建失败也不能把不完整队列当成全部通过。
fn save_report(
    output: Option<&str>,
    backend: &str,
    selected: &str,
    phase: &str,
    records: &[serde_json::Value],
    complete: bool,
    failed: bool,
) {
    let report = serde_json::json!({"scope":"controlled B1 measurement, not acceptance of lifecycle","target":xiao_codegen_llvm::TargetDescription::host().triple,"codegen_version":xiao_codegen_llvm::CODEGEN_VERSION,"runtime_abi_encoded":xiao_runtime_abi::ABI_ENCODED_VERSION,"backend":backend,"selected":selected,"phase":phase,"rust_debug_assertions":cfg!(debug_assertions),"protocol":"bounded geometric calibration to 100ms, then one warmup and five measured samples; each batch must reach 50ms; compile/input/validation/terminal I/O excluded; marker formatting/cleanup included as a conservative time upper bound","memory_method":"OS resident high-water mark minus current resident baseline, conservative upper bound that may include previous high-water mark; not allocator-exact","records":records,"complete":complete,"failed":failed});
    if let Some(path) = output {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

/// --case 接受单项或 all；--phase 为 maximum/scaling/all，输出保存完整样本而非只保存成绩。
fn main() {
    let mut backend = "vm".to_owned();
    let mut selected = "all".to_owned();
    let mut phase = "all".to_owned();
    let mut output = None;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--backend" => backend = args.next().expect("缺少 backend"),
            "--case" => selected = args.next().expect("缺少 case"),
            "--phase" => phase = args.next().expect("缺少 phase"),
            "--output" => output = Some(args.next().expect("缺少 output")),
            "--list" => {
                for id in cases::IDS {
                    println!("{id}");
                }
                return;
            }
            _ => panic!("未知参数 {argument}"),
        }
    }
    assert!(matches!(backend.as_str(), "vm" | "native"));
    assert!(matches!(phase.as_str(), "maximum" | "scaling" | "all"));
    assert!(selected == "all" || cases::IDS.contains(&selected.as_str()));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let sources = source_snapshot::Sources::capture(root).expect("固定F10源码失败");
    let directory = run_directory::create("xiao-b1-performance").expect("创建独占产物目录失败");
    println!("B1-PERF-ARTIFACTS {}", directory.display());
    let mut records = Vec::new();
    let mut failed = false;
    save_report(
        output.as_deref(),
        &backend,
        &selected,
        &phase,
        &records,
        false,
        false,
    );
    'cases: for &id in cases::IDS {
        if selected != "all" && selected != id {
            continue;
        }
        let scalar = id.starts_with("math.") || id == "fmt.format_int";
        let mut sizes = Vec::new();
        if phase != "scaling" || scalar {
            sizes.push(None);
        }
        if phase != "maximum" && !scalar {
            sizes.extend([Some(4096), Some(8192), Some(16384)]);
        }
        for level in if backend == "native" {
            vec![0, 3]
        } else {
            vec![0]
        } {
            let mut previous = None;
            for &size in &sizes {
                sources.check_current().expect("F10源码与加载根发生变化");
                let case = cases::workload(id, size);
                let measure = |repeats, count| {
                    if backend == "native" {
                        native::samples(root, &directory, &case, repeats, count, level)
                    } else {
                        vm_samples(root, &case, repeats, count)
                    }
                };
                // 单调用的标记成本会放大校准值；迭代校准真实批次，不能只按一次除法估算。
                let mut repeats = 1;
                let mut calibration = Vec::new();
                loop {
                    let sample = measure(repeats, 1).pop().unwrap();
                    let elapsed = sample.seconds;
                    println!(
                        "B1-PERF-CALIBRATION case={id} optimization={level} scale={size:?} repeats={repeats} seconds={elapsed}"
                    );
                    calibration.push(serde_json::json!({"repeats":repeats,"sample":sample}));
                    if elapsed >= 0.1 {
                        break;
                    }
                    repeats = calibration::next_batch(repeats, elapsed);
                }
                let samples = measure(repeats, 6);
                sources.check_current().expect("F10源码与加载根发生变化");
                let mut measured: Vec<_> = samples[1..]
                    .iter()
                    .map(|sample| sample.seconds / repeats as f64)
                    .collect();
                measured.sort_by(f64::total_cmp);
                let median = measured[2];
                let memory = samples[1..]
                    .iter()
                    .map(|sample| sample.resident_increment_upper_bytes)
                    .max()
                    .unwrap();
                let ratio = if size.is_some() {
                    previous.map(|last: f64| median / last)
                } else {
                    None
                };
                if size.is_some() {
                    previous = Some(median);
                }
                let batch_duration_passed =
                    samples[1..].iter().all(|sample| sample.seconds >= 0.05);
                let passed = batch_duration_passed
                    && median <= if backend == "native" { 2.0 } else { 10.0 }
                    && memory <= 64 * 1024 * 1024
                    && ratio.is_none_or(|value| value <= 3.0);
                let record = serde_json::json!({"case":case.id,"module":case.module,"function":case.function,"product_sha256":sources.product_digest(case.module),"backend":backend,"optimization":if backend=="native"{Some(level)}else{None},"rust_debug_assertions":cfg!(debug_assertions),"scale":size,"input_bytes_or_items":case.input_bytes_or_items,"output_bytes_or_items":case.output_bytes_or_items,"repeats":repeats,"calibration":calibration,"warmup":samples[0],"samples":&samples[1..],"median_seconds":median,"doubling_ratio":ratio,"resident_increment_upper_bytes":memory,"batch_duration_passed":batch_duration_passed,"passed":passed});
                println!("{record}");
                records.push(record);
                save_report(
                    output.as_deref(),
                    &backend,
                    &selected,
                    &phase,
                    &records,
                    false,
                    !passed,
                );
                if !passed {
                    failed = true;
                    break 'cases;
                }
            }
        }
    }
    save_report(
        output.as_deref(),
        &backend,
        &selected,
        &phase,
        &records,
        !failed,
        failed,
    );
    if failed {
        std::process::exit(1);
    }
}
