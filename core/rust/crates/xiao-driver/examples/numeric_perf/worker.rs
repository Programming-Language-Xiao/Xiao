//! 独立release VM子进程；只在完整语言调用的标记之间计时。

use super::memory;
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use xiao_vm::{RunResult, StackCarrier, Vm, VmEvent, VmEventSink, VmOptions};

/// 两后端共用的逐行测量协议。
pub fn emit(value: serde_json::Value) {
    println!("{value}");
    std::io::stdout().flush().unwrap();
}

struct Sink {
    started: Option<Instant>,
    baseline: bool,
    warmed: bool,
}

impl VmEventSink for Sink {
    /// 所有输入/预热/黄金检查均在计时区间外；OS高水位覆盖整个进程。
    fn record(&mut self, event: VmEvent) {
        if let VmEvent::IntrinsicOutput { text } = event {
            match text.as_str() {
                "0\n" => {
                    assert!(!self.baseline);
                    self.baseline = true;
                    emit(
                        serde_json::json!({"event":"baseline","pid":std::process::id(),"memory":memory::resident().ok()}),
                    );
                }
                "1\n" => {
                    assert!(self.baseline && !self.warmed);
                    self.warmed = true;
                    emit(serde_json::json!({"event":"ready","pid":std::process::id()}));
                }
                "2\n" => {
                    assert!(self.warmed && self.started.is_none());
                    self.started = Some(Instant::now());
                }
                "3\n" => {
                    let elapsed = self.started.take().unwrap().elapsed().as_secs_f64();
                    emit(
                        serde_json::json!({"event":"sample","seconds":elapsed,"memory":memory::resident().ok()}),
                    );
                }
                "4\n" => {
                    assert!(self.started.is_none());
                    emit(serde_json::json!({"event":"done"}));
                }
                "true\n" => {
                    assert!(self.started.is_none());
                    emit(serde_json::json!({"event":"validated"}));
                }
                other => panic!("A6黄金失败或非预期输出：{other:?}"),
            }
        }
    }
}

/// VM已构造后，语言程序先发baseline再构造输入；每个进程只运行一个规模。
pub fn vm(path: &Path) {
    if cfg!(debug_assertions) {
        panic!("A6测量必须使用release VM/Runtime");
    }
    let file = xiao_bytecode::decode_xiaoc(&std::fs::read(path).unwrap()).unwrap();
    let mut vm = Vm::<StackCarrier, _>::new(
        &file.program,
        VmOptions::default(),
        Sink {
            started: None,
            baseline: false,
            warmed: false,
        },
    );
    let result = vm.run();
    assert!(
        matches!(result, RunResult::Success),
        "A6 VM执行失败：{result:?}"
    );
}
