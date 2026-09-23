//! 09-B0-E 检查点开关的附加性能对照；不参与 09R3 冻结报告。

use std::time::Instant;

use xiao_bytecode::lower_program;
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_vm::{RunRequest, RunResult, VmOptions, run_request};

/// 每组测量前用于稳定运行状态的预热次数。
const WARMUP_ITERATIONS: usize = 3;
/// 每种检查点配置的正式测量次数。
const MEASUREMENT_ITERATIONS: usize = 11;

/// 编译性能夹具源码并生成对应的 TAC 程序。
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

/// 使用指定检查点配置执行一次性能夹具。
fn run_once(ir: &xiao_ir::IrProgram, program: &xiao_bytecode::TacProgram, enabled: bool) {
    let options = VmOptions {
        checkpoints_enabled: enabled,
        checkpoint_interval: 1024,
        ..VmOptions::default()
    };
    let outcome = run_request(&RunRequest::new(ir, program).with_options(options));
    assert!(matches!(outcome.result, RunResult::Success));
}

/// 返回无序样本的中位数。
fn median(values: &mut [u128]) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

/// 交替测量启用与关闭检查点时的执行耗时。
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
        let first = index % 2 == 0;
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
/// 对照检查点启用与关闭时的 release 构建执行耗时。
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
