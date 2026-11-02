//! B1-IMP 最大输入 VM 初测：计时仅覆盖已编译产品调用，输入准备和输出在区间之外。

#[path = "../examples/perf/calibration.rs"]
/// 显式F10工具的量化时钟校准回归也随默认性能目标执行。
mod calibration;

use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_vm::{RunResult, Vm, VmEvent, VmEventSink, VmOptions};

/// 本测试二进制内串行采样，不修改工作区全局测试并发设置。
static MEASUREMENT_LOCK: Mutex<()> = Mutex::new(());

/// Runtime 对象峰值包含拥有的 Unicode 索引缓冲，不代表完整进程 RSS 或全部 VM 内存。
#[derive(Clone, Copy, Debug)]
struct Sample {
    elapsed: Duration,
    runtime_object_peak_bytes: u64,
}

/// 只记录测试 wrapper 的起止和在计时外输出的黄金结果。
struct TimedSink {
    start: Option<Instant>,
    events: usize,
    samples: Rc<RefCell<Vec<Sample>>>,
    memory: Option<xiao_runtime::MemoryMeasurement>,
    output: Rc<RefCell<String>>,
}

impl VmEventSink for TimedSink {
    /// wrapper 包含真实产品调用及其参数绑定，不包含 VM/输入初始化或黄金比较。
    fn record(&mut self, event: VmEvent) {
        self.events += 1;
        if self.events & 8191 == 0
            && self
                .start
                .is_some_and(|start| start.elapsed() > Duration::from_secs(60))
        {
            panic!("B1-IMP 产品调用超过 60 秒测量看门狗");
        }
        match event {
            VmEvent::FunctionEntered { function, .. } if function == "__b1_measure" => {
                self.memory = Some(xiao_runtime::start_memory_measurement());
                self.start = Some(Instant::now());
            }
            VmEvent::FunctionReturned { function, .. } if function == "__b1_measure" => {
                let elapsed = self.start.take().unwrap().elapsed();
                let runtime_object_peak_bytes = self.memory.take().unwrap().peak_live_bytes();
                self.samples.borrow_mut().push(Sample {
                    elapsed,
                    runtime_object_peak_bytes,
                });
            }
            VmEvent::IntrinsicOutput { text } => self.output.borrow_mut().push_str(&text),
            _ => {}
        }
    }
}

/// 读取原样产品正文，重复次数只在测试 wrapper 中变化，输入与比较始终在计时外。
fn measure_fmt(
    function: &str,
    text: &str,
    width: usize,
    fill: &str,
    expected: &str,
    repeats: usize,
    sample_count: usize,
) -> Vec<Sample> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let library = fs::read_to_string(root.join("stdlib/fmt.xiao")).unwrap();
    let input = serde_json::to_string(text).unwrap();
    let expected = serde_json::to_string(expected).unwrap();
    let fill = serde_json::to_string(fill).unwrap();
    let source = format!(
        "{library}\ndef __b1_measure(str text) -> str\n    index = 0\n    actual = text\n    while index < {repeats}\n        actual = {function}(text, {width}, {fill})\n        index = index + 1\n    return actual\nprepared = {input}\nexpected = {expected}\nsample = 0\nwhile sample < {sample_count}\n    actual = __b1_measure(prepared)\n    print(actual == expected)\n    sample = sample + 1\n"
    );
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap();
    let program = xiao_bytecode::lower_program(&artifact.ir);
    let samples = Rc::new(RefCell::new(Vec::with_capacity(6)));
    let output = Rc::new(RefCell::new(String::new()));
    let sink = TimedSink {
        start: None,
        events: 0,
        samples: samples.clone(),
        memory: None,
        output: output.clone(),
    };
    let mut vm =
        Vm::<xiao_vm::research::StackCarrier, _>::new(&program, VmOptions::default(), sink);
    let result = vm.run();
    assert!(matches!(result, RunResult::Success), "{result:?}");
    assert_eq!(*output.borrow(), "true\n".repeat(sample_count));
    let samples = samples.borrow().clone();
    assert_eq!(samples.len(), sample_count);
    samples
}

#[test]
/// 最大输入扫描与左右最大输出均真实执行；预热一次、五次样本取中位数，黄金比较在外。
fn fmt_maximum_input_has_bounded_vm_call_time() {
    let _lock = MEASUREMENT_LOCK.lock().unwrap();
    let ascii = "x".repeat(65_536);
    let text = "🚀".repeat(16_384);
    let padding = "😀".repeat(49_152);
    let left = format!("{padding}{text}");
    let right = format!("{text}{padding}");
    assert_eq!(text.len(), 65_536);
    assert_eq!(left.len(), 262_144);
    assert_eq!(right.len(), 262_144);
    for (case, function, text, width, fill, expected) in [
        ("scan", "pad_left", ascii.as_str(), 0, ".", ascii.as_str()),
        (
            "max-output-left",
            "pad_left",
            text.as_str(),
            65_536,
            "😀",
            left.as_str(),
        ),
        (
            "max-output-right",
            "pad_right",
            text.as_str(),
            65_536,
            "😀",
            right.as_str(),
        ),
    ] {
        let samples = measure_fmt(function, text, width, fill, expected, 1, 6);
        let mut measured: Vec<_> = samples[1..].iter().map(|sample| sample.elapsed).collect();
        measured.sort();
        let median = measured[2];
        let object_peak = samples
            .iter()
            .map(|sample| sample.runtime_object_peak_bytes)
            .max()
            .unwrap();
        println!(
            "B1-IMP-PERF backend=vm carrier=production-stack rust_debug_assertions={} case=fmt.{case} input_bytes={} output_bytes={} warmup={:?} samples={:?} median={median:?} runtime_object_peak_bytes={object_peak}",
            cfg!(debug_assertions),
            text.len(),
            expected.len(),
            samples[0].elapsed,
            &samples[1..]
        );
        assert!(
            median <= Duration::from_secs(10),
            "F10 {case} 的 VM 单次调用预算未达标：{median:?}"
        );
        assert!(
            object_peak <= 64 * 1024 * 1024,
            "{case} 的 Runtime 对象峰值超限：{object_peak}"
        );
    }
}

#[test]
/// 4K/8K/16K 各固定批次至少约 100ms，排除过短计时噪声后核对冻结的翻倍比。
fn fmt_input_scaling_respects_the_frozen_vm_budget() {
    let _lock = MEASUREMENT_LOCK.lock().unwrap();
    for padding in [false, true] {
        let mut previous = None;
        for size in [4096, 8192, 16384] {
            let (text, width, fill, expected) = if padding {
                let text = "🚀".repeat(size / 4);
                let expected = format!("{}{text}", "😀".repeat(size * 3 / 4));
                (text, size, "😀", expected)
            } else {
                let text = "x".repeat(size);
                (text.clone(), 0, ".", text)
            };
            let calibration = measure_fmt("pad_left", &text, width, fill, &expected, 1, 1);
            let repeats = (0.1 / calibration[0].elapsed.as_secs_f64()).ceil().max(1.0) as usize;
            let samples = measure_fmt("pad_left", &text, width, fill, &expected, repeats, 6);
            let mut measured: Vec<_> = samples[1..]
                .iter()
                .map(|sample| sample.elapsed.as_secs_f64() / repeats as f64)
                .collect();
            measured.sort_by(f64::total_cmp);
            let median = measured[2];
            println!(
                "B1-IMP-PERF backend=vm carrier=production-stack rust_debug_assertions={} case=fmt.pad_left.scaling padding={padding} input_bytes={size} output_bytes={} repeats={repeats} batch_samples={:?} median_seconds={median}",
                cfg!(debug_assertions),
                expected.len(),
                &samples[1..]
            );
            if let Some(last) = previous {
                let ratio = median / last;
                println!("B1-IMP-PERF padding={padding} input_bytes={size} doubling_ratio={ratio}");
                assert!(ratio <= 3.0, "F10 翻倍比未达标：{ratio}");
            }
            previous = Some(median);
        }
    }
}
