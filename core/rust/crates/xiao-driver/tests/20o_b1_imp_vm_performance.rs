//! B1-IMP 最大输入 VM 初测：计时仅覆盖已编译产品调用，输入准备和输出在区间之外。

use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_vm::{RunResult, Vm, VmEvent, VmEventSink, VmOptions};

/// 只记录测试 wrapper 的起止和在计时外输出的黄金结果。
struct TimedSink {
    start: Option<Instant>,
    events: usize,
    samples: Rc<RefCell<Vec<Duration>>>,
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
                self.start = Some(Instant::now())
            }
            VmEvent::FunctionReturned { function, .. } if function == "__b1_measure" => {
                self.samples
                    .borrow_mut()
                    .push(self.start.take().unwrap().elapsed());
            }
            VmEvent::IntrinsicOutput { text } => self.output.borrow_mut().push_str(&text),
            _ => {}
        }
    }
}

/// 读取原样产品正文，重复次数只在测试 wrapper 中变化，输入与比较始终在计时外。
fn measure_fmt(text: &str, repeats: usize) -> Vec<Duration> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let library = fs::read_to_string(root.join("stdlib/fmt.xiao")).unwrap();
    let input = serde_json::to_string(text).unwrap();
    let source = format!(
        "{library}\ndef __b1_measure(str text) -> str\n    index = 0\n    actual = text\n    while index < {repeats}\n        actual = pad_left(text, 0, \".\")\n        index = index + 1\n    return actual\nprepared = {input}\nsample = 0\nwhile sample < 6\n    actual = __b1_measure(prepared)\n    print(actual == prepared)\n    sample = sample + 1\n"
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
        output: output.clone(),
    };
    let mut vm =
        Vm::<xiao_vm::research::StackCarrier, _>::new(&program, VmOptions::default(), sink);
    let result = vm.run();
    assert!(matches!(result, RunResult::Success), "{result:?}");
    assert_eq!(*output.borrow(), "true\n".repeat(6));
    let samples = samples.borrow().clone();
    assert_eq!(samples.len(), 6);
    samples
}

#[test]
/// 已有生产栈载体实测最大合法 ASCII 原文；预热一次、五次样本取中位数，黄金比较在外。
fn fmt_maximum_input_has_bounded_vm_call_time() {
    let samples = measure_fmt(&"x".repeat(65_536), 1);
    let mut measured = samples[1..].to_vec();
    measured.sort();
    let median = measured[2];
    println!(
        "B1-IMP-PERF backend=vm carrier=production-stack rust_debug_assertions={} case=fmt.pad_left.max-input bytes=65536 warmup={:?} samples={:?} median={median:?}",
        cfg!(debug_assertions),
        samples[0],
        &samples[1..]
    );
    assert!(
        median <= Duration::from_secs(10),
        "F10 最大合法输入的 VM 单次调用预算未达标：{median:?}"
    );
}

#[test]
/// 4K/8K/16K 各固定批次至少约 100ms，排除过短计时噪声后核对冻结的翻倍比。
fn fmt_input_scaling_respects_the_frozen_vm_budget() {
    let mut previous = None;
    for size in [4096, 8192, 16384] {
        let text = "x".repeat(size);
        let calibration = measure_fmt(&text, 1);
        let repeats = (0.1 / calibration[0].as_secs_f64()).ceil().max(1.0) as usize;
        let samples = measure_fmt(&text, repeats);
        let mut measured: Vec<_> = samples[1..]
            .iter()
            .map(|time| time.as_secs_f64() / repeats as f64)
            .collect();
        measured.sort_by(f64::total_cmp);
        let median = measured[2];
        println!(
            "B1-IMP-PERF backend=vm carrier=production-stack rust_debug_assertions={} case=fmt.pad_left.scaling bytes={size} repeats={repeats} batch_samples={:?} median_seconds={median}",
            cfg!(debug_assertions),
            &samples[1..]
        );
        if let Some(last) = previous {
            let ratio = median / last;
            println!("B1-IMP-PERF bytes={size} doubling_ratio={ratio}");
            assert!(ratio <= 3.0, "F10 翻倍比未达标：{ratio}");
        }
        previous = Some(median);
    }
}
