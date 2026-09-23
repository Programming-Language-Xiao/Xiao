//! 09-B0-E 检查点开关的附加性能对照；不参与 09R3 冻结报告。

use std::time::Instant;

use xiao_bytecode::lower_program;
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_vm::{RunRequest, RunResult, VmOptions, run_request};

const WARMUP_ITERATIONS: usize = 3;
const MEASUREMENT_ITERATIONS: usize = 11;

fn compile() -> (xiao_ir::IrProgram, xiao_bytecode::TacProgram) {
    let source = "total = 0\nindex = 0\nwhile index != 50000\n    total = total + index\n    index = index + 1\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("性能夹具源码应成功编译");
    let ir = artifact.ir;
    let program = lower_program(&ir);
    assert!(program.unsupported.is_empty(), "性能夹具不得含未降低构造");
    (ir, program)
}

fn run_once(ir: &xiao_ir::IrProgram, program: &xiao_bytecode::TacProgram, enabled: bool) {
    let options = VmOptions {
        checkpoints_enabled: enabled,
        checkpoint_interval: 1024,
        ..VmOptions::default()
    };
    let outcome = run_request(&RunRequest::new(ir, program).with_options(options));
    assert!(matches!(outcome.result, RunResult::Success));
}

fn median(values: &mut [u128]) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn measure_pair(
    ir: &xiao_ir::IrProgram,
    program: &xiao_bytecode::TacProgram,
) -> (Vec<u128>, Vec<u128>) {
    for _ in 0..WARMUP_ITERATIONS {
        run_once(ir, program, true);
        run_once(ir, program, false);
    }
    let mut enabled = Vec::with_capacity(MEASUREMENT_ITERATIONS);
    let mut disabled = Vec::with_capacity(MEASUREMENT_ITERATIONS);
    for index in 0..MEASUREMENT_ITERATIONS {
        let first = index.is_multiple_of(2);
        for enabled_first in [first, !first] {
            let start = Instant::now();
            run_once(ir, program, enabled_first);
            let elapsed = start.elapsed().as_nanos();
            if enabled_first {
                enabled.push(elapsed);
            } else {
                disabled.push(elapsed);
            }
        }
    }
    (enabled, disabled)
}

#[test]
fn checkpoint_toggle_release_comparison() {
    let (ir, program) = compile();
    let (enabled, disabled) = measure_pair(&ir, &program);
    let mut enabled_median = enabled.clone();
    let mut disabled_median = disabled.clone();
    println!(
        "checkpoint_comparison={{\"profile\":\"release\",\"warmup_iterations\":{},\"measurement_iterations\":{},\"checkpoint_interval\":1024,\"enabled_ns\":{:?},\"disabled_ns\":{:?},\"enabled_median_ns\":{},\"disabled_median_ns\":{}}}",
        WARMUP_ITERATIONS,
        MEASUREMENT_ITERATIONS,
        enabled,
        disabled,
        median(&mut enabled_median),
        median(&mut disabled_median),
    );
}
