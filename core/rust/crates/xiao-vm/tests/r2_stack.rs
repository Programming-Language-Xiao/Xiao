//! 09R2 栈式载体解释器规格。

use xiao_bytecode::research::lower_program;
use xiao_bytecode::research::{
    BlockId, CategoryMap, ConstPool, FuncId, OperandWidth, RegisterClass, TacAbi, TacBlock,
    TacConstant, TacFunction, TacInstr, TacOp, TacProgram, VReg, build_pc_map,
};
use xiao_diagnostics::{
    FATAL_STACK_OVERFLOW_CODE, NUMERIC_OVERFLOW_CODE, RANDOM_COUNT_CODE, RANDOM_SEED_CODE,
    SELECTOR_STEP_CODE, TYPE_MISMATCH_CODE,
};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_ir::IrSpan;
use xiao_vm::research::{RunResult, VmEvent, VmOptions, run, run_hybrid, run_register};

/// 编译并降低一份源码。
fn load(source_text: &str) -> xiao_bytecode::research::TacProgram {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source_text))
        .unwrap_or_else(|error| panic!("前端应成功: {:?}", error.diagnostics()));
    lower_program(&artifact.ir)
}

/// 形参参与运算，编译期无法折叠，因此溢出只能在运行时发现。
///
/// 注意不能用字面量构造运行时错误：`value = 7 % 0` 会在类型阶段就被常量折叠
/// 拒绝（`X02-TYPE-007`），那是正确行为，不是缺陷。
const OVERFLOWS: &str =
    "def square(int value) -> int\n    return value * value\nresult = square(4000000000)\n";

/// 无终止递归，用于触发不可恢复的栈溢出。
const RUNAWAY: &str = "def down(int value) -> int\n    return down(value)\nresult = down(1)\n";

#[test]
/// 标量算术应真的被执行。
fn executes_scalar_arithmetic() {
    let outcome = run(&load("value = 1 + 2\n"), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert!(outcome.metrics.instructions > 0);
}

#[test]
/// 高级多选应经过共享选择算子在栈式机型完成执行。
fn executes_advanced_selector() {
    let program = load("values = [1, 2, 3, 4]\nselected = values[0, 2]\n");
    assert!(
        program.unsupported.is_empty(),
        "未支持项: {:?}",
        program.unsupported
    );
    let outcome = run(&program, VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert!(run_register(&program, VmOptions::new()).result.is_success());
    assert!(run_hybrid(&program, VmOptions::new()).result.is_success());
}

#[test]
/// 范围、全选、正负步长和随机选择都应共用同一高级指令。
fn executes_selector_ranges_steps_and_random() {
    let source = "values = [1, 2, 3, 4]\na = values[1~3]\nb = values[<2]\nc = values{2}[=]\nd = values{ -1 }[=]\ne = values[?2]\n";
    let program = load(source);
    assert!(
        program.unsupported.is_empty(),
        "未支持项: {:?}",
        program.unsupported
    );
    assert!(run(&program, VmOptions::new()).result.is_success());
    assert!(run_register(&program, VmOptions::new()).result.is_success());
    assert!(run_hybrid(&program, VmOptions::new()).result.is_success());
}

#[test]
/// 选择器左值广播应在全部目标验证后一次性提交。
fn executes_selector_broadcast_assignment() {
    let program = load("values = [1, 2, 3]\nvalues[0, 2] = 9\n");
    assert!(
        program.unsupported.is_empty(),
        "未支持项: {:?}",
        program.unsupported
    );
    assert!(run(&program, VmOptions::new()).result.is_success());
    assert!(run_register(&program, VmOptions::new()).result.is_success());
    assert!(run_hybrid(&program, VmOptions::new()).result.is_success());
}

#[test]
/// `random.seed` 应降低为显式种子操作并保持可执行。
fn executes_random_seed_plan() {
    let program = load("values = [1, 2, 3]\nrandom.seed(42)\npicked = values[?1]\n");
    assert!(
        program.unsupported.is_empty(),
        "未支持项: {:?}",
        program.unsupported
    );
    assert!(run(&program, VmOptions::new()).result.is_success());
    assert!(run_register(&program, VmOptions::new()).result.is_success());
    assert!(run_hybrid(&program, VmOptions::new()).result.is_success());
}

#[test]
/// 动态零步长应在执行器产生选择器专用可恢复错误。
fn dynamic_zero_step_is_recoverable() {
    let program = load(
        "def choose(int step) -> int\n    return step\nvalues = [1, 2, 3]\nstep = choose(0)\nresult = values{step}[=]\n",
    );
    for outcome in [
        run(&program, VmOptions::new()),
        run_register(&program, VmOptions::new()),
        run_hybrid(&program, VmOptions::new()),
    ] {
        assert_eq!(outcome.result.error_code(), Some(SELECTOR_STEP_CODE));
    }
}

#[test]
/// 动态合法步长和动态范围端点应在三种载体上继续执行，而不是被通用检查提前拒绝。
fn dynamic_selector_inputs_are_executed() {
    let program = load(
        "def choose(int value) -> int\n    return value\nvalues = [1, 2, 3, 4, 5]\nstep = choose(2)\nselected = values{step}[=]\n",
    );
    assert!(
        program.unsupported.is_empty(),
        "动态选择不应留下未支持项: {:?}",
        program.unsupported
    );
    for outcome in [
        run(&program, VmOptions::new()),
        run_register(&program, VmOptions::new()),
        run_hybrid(&program, VmOptions::new()),
    ] {
        assert!(
            outcome.result.is_success(),
            "动态选择应成功: {:?}",
            outcome.result
        );
    }
}

#[test]
/// 放回随机可以超过候选数，零数量则返回成功的空结果。
fn replacement_random_and_empty_selection_are_executed() {
    let replacement = load("values = [1, 2]\npicked = values[!?5]\n");
    assert!(replacement.unsupported.is_empty());
    for outcome in [
        run(&replacement, VmOptions::new()),
        run_register(&replacement, VmOptions::new()),
        run_hybrid(&replacement, VmOptions::new()),
    ] {
        assert!(
            outcome.result.is_success(),
            "放回随机应成功: {:?}",
            outcome.result
        );
    }

    let empty = load("values = [1, 2]\npicked = values[?0]\n");
    assert!(empty.unsupported.is_empty());
    for outcome in [
        run(&empty, VmOptions::new()),
        run_register(&empty, VmOptions::new()),
        run_hybrid(&empty, VmOptions::new()),
    ] {
        assert!(
            outcome.result.is_success(),
            "零数量随机应成功: {:?}",
            outcome.result
        );
    }
}

#[test]
/// 字典列的键名选择应保留键和值，而不是退化成数字路径。
fn dictionary_column_key_selector_is_executed() {
    let program = load("column = <first = 1, second = 2>\nselected = column[second]\n");
    assert!(
        program.unsupported.is_empty(),
        "字典列选择不应留下未支持项: {:?}",
        program.unsupported
    );
    for outcome in [
        run(&program, VmOptions::new()),
        run_register(&program, VmOptions::new()),
        run_hybrid(&program, VmOptions::new()),
    ] {
        assert!(
            outcome.result.is_success(),
            "字典列键名选择应成功: {:?}",
            outcome.result
        );
    }
}

#[test]
/// 动态随机数量的负值和无放回超量必须在三种载体上使用同一错误身份。
fn dynamic_random_count_is_recoverable() {
    for count in [-1, 3] {
        let source = format!(
            "def choose(int count) -> int\n    return count\nvalues = [1, 2]\ncount = choose({count})\nresult = values[?count]\n"
        );
        let program = load(&source);
        for outcome in [
            run(&program, VmOptions::new()),
            run_register(&program, VmOptions::new()),
            run_hybrid(&program, VmOptions::new()),
        ] {
            assert_eq!(outcome.result.error_code(), Some(RANDOM_COUNT_CODE));
        }
    }
}

#[test]
/// 动态随机种子必须检查非负性，合法种子仍可在三种载体执行。
fn dynamic_random_seed_is_recoverable() {
    let invalid = load(
        "def choose(int seed) -> int\n    return seed\nseed = choose(-1)\nrandom.seed(seed)\nvalues = [1, 2]\npicked = values[?1]\n",
    );
    for outcome in [
        run(&invalid, VmOptions::new()),
        run_register(&invalid, VmOptions::new()),
        run_hybrid(&invalid, VmOptions::new()),
    ] {
        assert_eq!(outcome.result.error_code(), Some(RANDOM_SEED_CODE));
    }

    let valid = load(
        "def choose(int seed) -> int\n    return seed\nseed = choose(42)\nrandom.seed(seed)\nvalues = [1, 2]\npicked = values[?1]\n",
    );
    assert!(run(&valid, VmOptions::new()).result.is_success());
    assert!(run_register(&valid, VmOptions::new()).result.is_success());
    assert!(run_hybrid(&valid, VmOptions::new()).result.is_success());
}

#[test]
/// 函数调用与循环应真的被执行，并记录调用深度。
fn executes_calls_and_loops() {
    let source = "def count(int limit) -> int\n    total = 0\n    while total != limit\n        total = total + 1\n    return total\nresult = count(3)\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert!(outcome.metrics.max_call_depth >= 2, "应进入被调函数");
}

#[test]
/// 三种载体分别上报溢出、映射点和调用保存，不共用一个总成本计数。
fn machine_metrics_keep_independent_cost_axes() {
    let program = load(
        "def identity(int value) -> int\n    return value\npayload = \"heap\"\ntotal = 0\nwhile total != 2\n    total = total + 1\nresult = identity(total)\n",
    );
    let stack = run(&program, VmOptions::new());
    let registers = run_register(&program, VmOptions::new());
    let hybrid = run_hybrid(&program, VmOptions::new());

    assert!(stack.result.is_success());
    assert!(registers.result.is_success());
    assert!(hybrid.result.is_success());
    assert_eq!(stack.metrics.spill_count, 0, "栈式不写独立帧槽");
    assert!(
        stack.metrics.stack_map_entries > 0,
        "栈式在跳转目标需要映射"
    );
    assert_eq!(
        registers.metrics.stack_map_entries, 0,
        "分类型寄存器式按 R1-F 完全不需要映射"
    );
    assert!(registers.metrics.spill_count > 0);
    assert!(registers.metrics.call_save_count > 0);
    assert!(
        hybrid.metrics.stack_map_entries > 0,
        "混合式在调用点与帧尾需要映射"
    );
    assert!(hybrid.metrics.spill_count > 0);
    assert!(hybrid.metrics.call_save_count > 0);
}

#[test]
/// R1-F 冻结的映射分界：栈式数跳转目标，混合式数调用点与帧尾，寄存器式恒为 0。
///
/// 三种机型必须**同量纲**，否则 09R3 无法把这一轴当作机型差异来对比。
fn stack_map_points_follow_the_frozen_machine_division() {
    // 差分一：给同一条直线代码加一个循环（只增加跳转目标）。
    // 栈式应当增长，混合式**一点都不该变**——跳转目标不是它的映射点。
    let straight = load("total = 2\n");
    let looping = load("total = 0\nwhile total != 2\n    total = total + 1\n");
    assert_eq!(
        run(&straight, VmOptions::new()).metrics.stack_map_entries,
        0,
        "直线代码没有跳转目标"
    );
    assert!(
        run(&looping, VmOptions::new()).metrics.stack_map_entries > 0,
        "循环带来跳转目标，栈式必须计数"
    );
    assert_eq!(
        run_hybrid(&looping, VmOptions::new())
            .metrics
            .stack_map_entries,
        run_hybrid(&straight, VmOptions::new())
            .metrics
            .stack_map_entries,
        "跳转目标不得进入混合式的映射点数"
    );

    // 差分二：加一次函数调用（只增加调用点与被调帧尾）。
    // 混合式应当增长，栈式**一点都不该变**——调用点不是它的映射点。
    let calling = load("def identity(int value) -> int\n    return value\nresult = identity(1)\n");
    assert!(
        run_hybrid(&calling, VmOptions::new())
            .metrics
            .stack_map_entries
            > run_hybrid(&straight, VmOptions::new())
                .metrics
                .stack_map_entries,
        "调用点与帧尾是混合式的映射点"
    );
    assert_eq!(
        run(&calling, VmOptions::new()).metrics.stack_map_entries,
        run(&straight, VmOptions::new()).metrics.stack_map_entries,
        "调用点不得进入栈式的映射点数"
    );

    // 不论程序形态，分类型寄存器式都恒为 0。
    for program in [&straight, &looping, &calling] {
        assert_eq!(
            run_register(program, VmOptions::new())
                .metrics
                .stack_map_entries,
            0,
            "寄存器式按 R1-F 完全不需要映射"
        );
    }
}

#[test]
/// 递归深度超过上限时产生不可恢复的栈溢出故障，而不是普通错误。
fn stack_overflow_is_fatal() {
    let outcome = run(
        &load(RUNAWAY),
        VmOptions {
            max_call_depth: 8,
            ..VmOptions::default()
        },
    );
    assert!(
        matches!(outcome.result, RunResult::Fatal(_)),
        "结果: {:?}",
        outcome.result
    );
    assert_eq!(outcome.result.error_code(), Some(FATAL_STACK_OVERFLOW_CODE));
}

#[test]
/// 整数溢出产生稳定的可恢复错误身份，与致命故障分属两条通道。
fn overflow_is_recoverable() {
    let outcome = run(&load(OVERFLOWS), VmOptions::new());
    assert!(
        matches!(outcome.result, RunResult::Error(_)),
        "结果: {:?}",
        outcome.result
    );
    assert_eq!(outcome.result.error_code(), Some(NUMERIC_OVERFLOW_CODE));
}

#[test]
/// 堆字符串在作用域退出时按冻结计划释放。
fn releases_heap_values_on_scope_exit() {
    let outcome = run(&load("value = \"x\"\n"), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert!(outcome.metrics.releases > 0, "应执行释放动作");
}

#[test]
/// 事件流应覆盖模块、函数、作用域、释放与错误五类。
fn records_all_event_categories() {
    let outcome = run(&load("value = \"x\"\n"), VmOptions::new());
    let has = |predicate: fn(&VmEvent) -> bool| outcome.events.iter().any(predicate);
    assert!(has(|event| matches!(event, VmEvent::ModuleLoaded { .. })));
    assert!(has(|event| matches!(
        event,
        VmEvent::FunctionEntered { .. }
    )));
    assert!(has(|event| matches!(event, VmEvent::ScopeEntered { .. })));
    assert!(has(|event| matches!(event, VmEvent::ValueReleased { .. })));

    let recovered = run(&load(OVERFLOWS), VmOptions::new());
    assert!(
        recovered
            .events
            .iter()
            .any(|event| matches!(event, VmEvent::ErrorRaised { .. }))
    );

    let fatal = run(
        &load(RUNAWAY),
        VmOptions {
            max_call_depth: 8,
            ..VmOptions::default()
        },
    );
    assert!(
        fatal
            .events
            .iter()
            .any(|event| matches!(event, VmEvent::FatalRaised { .. }))
    );
}

#[test]
/// 研究代码必须是叶子：生产 `src/` 不得引用 `crate::research`。
///
/// 这条约束是可执行的，任何把它们耦合回去的改动都会让本用例失败。
fn research_module_stays_a_leaf() {
    let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    let mut stack = vec![source_root.clone()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).expect("应能读取源码目录") {
            let path = entry.expect("目录项").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "research") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("应能读取源码文件");
            checked += 1;
            assert!(
                !text.contains("crate::research"),
                "生产模块引用了研究子模块：{}",
                path.display()
            );
        }
    }
    assert!(checked > 0, "应至少检查一个生产源文件");
}

#[test]
/// 容器构造与精确索引应端到端跑通，负索引按长度归一化。
fn executes_containers_and_exact_index() {
    let source = "values = [1, 2, 3]\nfirst = values[0]\nlast = values[-1]\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert!(outcome.metrics.releases > 0, "容器临时值应被释放");
}

#[test]
/// 具名绑定之间的赋值不得清空源绑定。
///
/// 一律用 `Move` 会让 `second = first` 把 `first` 的寄存器搬空，之后再用
/// `first` 就读到空寄存器报无效句柄。
fn copying_between_bindings_keeps_the_source() {
    let source = "first = \"x\"\nsecond = first\nvalue = first\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert_eq!(outcome.result.error_code(), None);
}

#[test]
/// 临时值写进绑定仍然走移动，不产生额外引用。
fn moving_temporaries_into_bindings_still_works() {
    let outcome = run(&load("text = \"x\"\n"), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
}

#[test]
/// 显式构造错误应命中匹配处理器并继续执行 finally。
fn catches_constructed_error_and_runs_finally() {
    let source = "try\n    raise ArithmeticError(code = \"X\", message = \"bad\")\ncatch err as ArithmeticError\n    handled = true\nfinally\n    cleaned = true\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert!(
        outcome
            .events
            .iter()
            .any(|event| matches!(event, VmEvent::HandlerMatched { .. }))
    );
}

#[test]
/// catch 绑定重抛的仍是同一个错误对象，而不是按类型重新构造一条错误。
fn catch_binding_rethrow_preserves_error_identity() {
    let source = "try\n    try\n        raise ArithmeticError(code = \"identity\")\n    catch first as ArithmeticError\n        raise first\n    finally\n        inner_cleanup = true\ncatch second as ArithmeticError\n    handled = true\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerMatched { .. }))
            .count(),
        2,
        "重抛应先命中内层再命中外层: {:?}",
        outcome.events
    );
}

#[test]
/// 被调函数未匹配的错误应回到调用方继续查找处理器，错误身份和清理顺序不变。
fn propagates_error_to_caller_handler() {
    let source = "def fail() -> int\n    raise ArithmeticError(code = \"from-call\")\n    return 0\ntry\n    result = fail()\ncatch err as ArithmeticError\n    handled = true\nfinally\n    cleaned = true\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerMatched { .. }))
            .count(),
        1
    );
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1
    );
}

#[test]
/// catch 体再次抛错时，本层 finally 不得重复执行，但外层清理仍要运行。
fn catch_rethrow_does_not_repeat_own_finally() {
    let source = "try\n    raise ArithmeticError(code = \"main\")\ncatch err as ArithmeticError\n    raise TypeError(code = \"rethrow\")\nfinally\n    cleaned = true\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(
        matches!(outcome.result, RunResult::Error(_)),
        "结果: {:?}",
        outcome.result
    );
    assert_eq!(outcome.result.error_code(), Some("rethrow"));
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1,
        "本层 finally 应只执行一次: {:?}",
        outcome.events
    );
}

#[test]
/// 内层未匹配时应先清理内层 finally，再由外层 catch 接住。
fn nested_unmatched_reaches_outer_catch_once() {
    let source = "try\n    try\n        raise ArithmeticError(code = \"nested\")\n    catch inner as TypeError\n        inner_ok = true\n    finally\n        inner_clean = true\ncatch outer as ArithmeticError\n    outer_ok = true\nfinally\n    outer_clean = true\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerMatched { .. }))
            .count(),
        1
    );
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        2,
        "内外层 finally 各执行一次: {:?}",
        outcome.events
    );
}

#[test]
/// 正常退出与 return 退出都只调用一次 finally 子程序。
fn finally_runs_once_on_normal_and_return() {
    for source in [
        "try\n    value = 1\nfinally\n    cleaned = true\n",
        "def f() -> int\n    try\n        return 1\n    finally\n        cleaned = true\nresult = f()\n",
    ] {
        let outcome = run(&load(source), VmOptions::new());
        assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
                .count(),
            1,
            "finally 应只执行一次: {:?}",
            outcome.events
        );
    }
}

#[test]
/// finally 子程序中的嵌套 try 仍应在子程序边界内完成自己的 catch/finally。
fn nested_try_inside_finally_is_routed_locally() {
    let source = "try\n    value = 1\nfinally\n    try\n        raise TypeError(code = \"inner\")\n    catch err as TypeError\n        handled = true\n    finally\n        nested_clean = true\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerMatched { .. }))
            .count(),
        1,
        "嵌套 catch 应在 finally 子程序内命中: {:?}",
        outcome.events
    );
}

#[test]
/// 同一个 try 位于循环体内时，每一轮都必须重新执行 finally。
fn nested_try_finally_runs_once_per_loop_iteration() {
    let source = "count = 0\nwhile count < 2\n    try\n        count = count + 1\n    finally\n        cleanup = \"round\"\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        2,
        "两轮循环各应进入一次 finally: {:?}",
        outcome.events
    );
}

#[test]
/// 嵌套 return 必须按内层 finally 到外层 finally 的顺序展开。
fn nested_return_runs_finally_from_inner_to_outer() {
    let source = "def f() -> int\n    try\n        try\n            return 1\n        finally\n            inner = \"inner\"\n    finally\n        outer = \"outer\"\nresult = f()\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    let scopes = outcome
        .events
        .iter()
        .filter_map(|event| match event {
            VmEvent::HandlerEntered { scope, .. } => Some(*scope),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(scopes.len(), 2, "应进入两层 finally: {:?}", outcome.events);
    assert!(scopes[0] > scopes[1], "应先内层后外层: {scopes:?}");
}

#[test]
/// 内层 finally 的故障可以由外围 catch 接住，且不会把故障重新送回内层。
fn finally_failure_reaches_outer_catch() {
    let source = "try\n    try\n        value = 1\n    finally\n        raise TypeError(code = \"cleanup\")\ncatch err as TypeError\n    handled = true\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerMatched { .. }))
            .count(),
        1,
        "外围 catch 应接住 finally 故障: {:?}",
        outcome.events
    );
}

#[test]
/// 主错误经过 finally 时，清理体的新错误只能进入 suppressed。
fn finally_failure_is_suppressed_on_primary_error() {
    let source = "try\n    raise ArithmeticError(code = \"main\")\nfinally\n    raise TypeError(code = \"cleanup\")\n";
    let outcome = run(&load(source), VmOptions::new());
    let RunResult::Error(error) = outcome.result else {
        panic!("应保留主错误: {:?}", outcome.result);
    };
    assert_eq!(error.code(), "main");
    assert_eq!(error.suppressed().len(), 1);
    assert_eq!(error.suppressed()[0].code(), "cleanup");
}

#[test]
/// Fatal 故障绕过普通 catch、finally 和所有释放计划。
fn fatal_bypasses_handlers_and_cleanup() {
    let source = "def down(int value) -> int\n    return down(value)\ntry\n    result = down(1)\ncatch err as Error\n    handled = true\nfinally\n    cleaned = true\n";
    let outcome = run(
        &load(source),
        VmOptions {
            max_call_depth: 6,
            ..VmOptions::default()
        },
    );
    assert!(
        matches!(outcome.result, RunResult::Fatal(_)),
        "结果: {:?}",
        outcome.result
    );
    assert_eq!(outcome.metrics.releases, 0);
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        VmEvent::HandlerMatched { .. } | VmEvent::HandlerEntered { .. }
    )));
}

#[test]
/// 含堆字符串的 Fatal 路径仍然跳过全部释放计划和 finally。
fn fatal_with_heap_values_has_no_releases() {
    let source = "def down(int value) -> int\n    return down(value)\ntry\n    payload = \"heap\"\n    result = down(1)\ncatch err as Error\n    handled = true\nfinally\n    cleanup = \"cleanup\"\n";
    let outcome = run(
        &load(source),
        VmOptions {
            max_call_depth: 6,
            ..VmOptions::default()
        },
    );
    assert!(
        matches!(outcome.result, RunResult::Fatal(_)),
        "结果: {:?}",
        outcome.result
    );
    assert_eq!(outcome.metrics.releases, 0, "Fatal 不得释放堆值");
    assert!(!outcome.events.iter().any(|event| matches!(
        event,
        VmEvent::ValueReleased { .. } | VmEvent::HandlerEntered { .. }
    )));
}

#[test]
/// break/continue 离开受保护体时也必须先执行 finally。
fn loop_exits_run_finally_before_jump() {
    let break_source = "while true\n    try\n        break\n    finally\n        cleaned = true\n";
    let break_outcome = run(&load(break_source), VmOptions::new());
    assert!(
        break_outcome.result.is_success(),
        "结果: {:?}",
        break_outcome.result
    );
    assert_eq!(
        break_outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1,
        "break 路径 finally 应执行一次: {:?}",
        break_outcome.events
    );

    let continue_source = "value = 0\nwhile value < 1\n    value = value + 1\n    try\n        continue\n    finally\n        cleaned = true\n";
    let continue_outcome = run(&load(continue_source), VmOptions::new());
    assert!(
        continue_outcome.result.is_success(),
        "结果: {:?}",
        continue_outcome.result
    );
    assert_eq!(
        continue_outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1,
        "continue 路径 finally 应执行一次: {:?}",
        continue_outcome.events
    );
}

#[test]
/// 离开带 `finally` 的循环后，后续错误不得重新触发已经完成的清理。
fn loop_exit_does_not_reenter_finally_on_later_error() {
    let source = "while true\n    try\n        break\n    finally\n        cleaned = \"round\"\nraise ArithmeticError(code = \"after\")\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(matches!(outcome.result, RunResult::Error(_)));
    assert_eq!(outcome.result.error_code(), Some("after"));
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1,
        "后续错误不应再次进入循环 finally: {:?}",
        outcome.events
    );
}

#[test]
/// `finally` 自身的 return/break 应覆盖挂起退出，并继续完成外围清理。
fn finally_control_exit_is_dispatched_after_subroutine() {
    let return_source =
        "def f() -> int\n    try\n        return 1\n    finally\n        return 2\nresult = f()\n";
    let return_outcome = run(&load(return_source), VmOptions::new());
    assert!(
        return_outcome.result.is_success(),
        "结果: {:?}",
        return_outcome.result
    );
    assert_eq!(
        return_outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1,
        "finally return 只能进入一次: {:?}",
        return_outcome.events
    );

    let nested_return_source = "def f() -> int\n    try\n        try\n            return 1\n        finally\n            return 2\n    finally\n        outer = 3\nresult = f()\n";
    let nested_return = run(&load(nested_return_source), VmOptions::new());
    assert!(
        nested_return.result.is_success(),
        "结果: {:?}",
        nested_return.result
    );
    assert_eq!(
        nested_return
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        2,
        "内层覆盖性 return 后仍应执行内层和外层清理: {:?}",
        nested_return.events
    );

    let local_break_source = "value = 0\ntry\n    value = 1\nfinally\n    while value < 2\n        value = value + 1\n        break\nouter = true\n";
    let local_break = run(&load(local_break_source), VmOptions::new());
    assert!(
        local_break.result.is_success(),
        "结果: {:?}",
        local_break.result
    );
    assert_eq!(
        local_break
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1,
        "finally 内部循环的 break 不得额外触发外层 finally: {:?}",
        local_break.events
    );
}

#[test]
/// `catch` 体的非局部退出同样必须先经过所属 `finally`。
fn catch_control_exit_runs_own_finally() {
    let return_source = "def f() -> int\n    try\n        raise TypeError(code = \"caught\")\n    catch err as TypeError\n        return 2\n    finally\n        cleaned = \"return\"\nresult = f()\n";
    let return_outcome = run(&load(return_source), VmOptions::new());
    assert!(
        return_outcome.result.is_success(),
        "结果: {:?}",
        return_outcome.result
    );
    assert_eq!(
        return_outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1,
        "catch return 应执行一次 finally: {:?}",
        return_outcome.events
    );

    let break_source = "value = 0\nwhile value < 1\n    try\n        raise TypeError(code = \"caught\")\n    catch err as TypeError\n        break\n    finally\n        cleaned = \"break\"\n";
    let break_outcome = run(&load(break_source), VmOptions::new());
    assert!(
        break_outcome.result.is_success(),
        "结果: {:?}",
        break_outcome.result
    );
    assert_eq!(
        break_outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1,
        "catch break 应执行一次 finally: {:?}",
        break_outcome.events
    );

    let continue_source = "count = 0\nwhile count < 2\n    count = count + 1\n    try\n        raise TypeError(code = \"caught\")\n    catch err as TypeError\n        continue\n    finally\n        cleaned = \"continue\"\n";
    let continue_outcome = run(&load(continue_source), VmOptions::new());
    assert!(
        continue_outcome.result.is_success(),
        "结果: {:?}",
        continue_outcome.result
    );
    assert_eq!(
        continue_outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        2,
        "catch continue 每轮应执行一次 finally: {:?}",
        continue_outcome.events
    );
}

#[test]
/// 正常路径的 finally 故障成为主错误；已有主错误时清理故障进入 suppressed。
fn finally_failure_on_normal_and_catch_paths_is_routed_once() {
    let normal_source =
        "try\n    value = 1\nfinally\n    raise TypeError(code = \"normal-cleanup\")\n";
    let normal_outcome = run(&load(normal_source), VmOptions::new());
    assert!(matches!(normal_outcome.result, RunResult::Error(_)));
    assert_eq!(normal_outcome.result.error_code(), Some("normal-cleanup"));
    assert_eq!(
        normal_outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1
    );

    let catch_source = "try\n    raise ArithmeticError(code = \"primary\")\ncatch err as ArithmeticError\n    handled = true\nfinally\n    raise TypeError(code = \"catch-cleanup\")\n";
    let catch_outcome = run(&load(catch_source), VmOptions::new());
    assert!(
        catch_outcome.result.is_success(),
        "主错误应继续交给 catch，清理错误进入 suppressed: {:?}, 事件: {:?}",
        catch_outcome.result,
        catch_outcome.events
    );
    assert_eq!(
        catch_outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
            .count(),
        1
    );
}

#[test]
/// catch 中的非局部退出穿过外层 finally 时，内层与外层各执行一次且顺序不反转。
fn catch_exit_runs_outer_finally_after_inner_finally() {
    let source = "def f() -> int\n    try\n        try\n            raise TypeError(code = \"caught\")\n        catch err as TypeError\n            return 2\n        finally\n            inner = \"inner\"\n    finally\n        outer = \"outer\"\nresult = f()\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    let scopes = outcome
        .events
        .iter()
        .filter_map(|event| match event {
            VmEvent::HandlerEntered { scope, .. } => Some(*scope),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(scopes.len(), 2, "应进入两层 finally: {:?}", outcome.events);
    assert!(
        scopes[0] > scopes[1],
        "catch 所属内层 finally 应先于外层: {scopes:?}"
    );
}

#[test]
/// finally 内覆盖性 return 必须执行原 try 作用域的释放计划。
fn finally_return_releases_try_scope_value() {
    let source = "def f() -> int\n    try\n        payload = \"try-payload\"\n    finally\n        return 2\nresult = f()\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success());
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(
                |event| matches!(event, VmEvent::ValueReleased { exit, .. } if exit == "return")
            )
            .count(),
        1,
        "finally return 必须释放 try 作用域值: {:?}",
        outcome.events
    );
}

#[test]
/// finally 内的 break 也必须执行所属 try 作用域的释放计划。
fn finally_break_cleanup_with_heap_values() {
    let source = "value = 0\nwhile value < 1\n    try\n        payload = \"try-payload\"\n    finally\n        break\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert!(
        outcome
            .events
            .iter()
            .any(|event| matches!(event, VmEvent::ValueReleased { exit, .. } if exit == "break")),
        "finally break 必须释放 try 作用域值: {:?}",
        outcome.events
    );
}

#[test]
/// 内外层 finally 都覆盖 return 时，释放顺序仍应从内层到外层且不重复。
fn nested_finally_return_releases_each_scope_once() {
    let source = "def f() -> int\n    try\n        outer_try_payload = \"outer-try-payload\"\n        try\n            payload = \"inner-payload\"\n        finally\n            return 2\n    finally\n        outer_payload = \"outer-payload\"\nresult = f()\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    let returns = outcome
        .events
        .iter()
        .filter_map(|event| match event {
            VmEvent::ValueReleased { scope, exit, .. } if exit == "return" => Some(*scope),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        returns.len(),
        2,
        "内外层 try 值各释放一次: {:?}",
        outcome.events
    );
    assert_eq!(
        returns.len(),
        returns
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "每个作用域只释放一次: {:?}",
        returns
    );
}

#[test]
/// finally 的覆盖性 return 应替换挂起错误，并仍释放原 try 作用域。
fn finally_return_overrides_error_without_leaking_try_values() {
    let source = "def f() -> int\n    try\n        payload = \"try-payload\"\n        raise TypeError(code = \"primary\")\n    finally\n        return 2\nresult = f()\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(
        outcome.result.is_success(),
        "finally return 应覆盖错误: {:?}",
        outcome.result
    );
    assert!(
        outcome
            .events
            .iter()
            .any(|event| matches!(event, VmEvent::ValueReleased { exit, .. } if exit == "return")),
        "覆盖性 return 必须释放 try 值: {:?}",
        outcome.events
    );
}

#[test]
/// 返回堆字符串时，释放计划不得先消费返回寄存器。
fn returning_heap_string_survives_finally_cleanup() {
    let source = "def f() -> str\n    try\n        return \"result\"\n    finally\n        cleanup = \"cleanup\"\nresult = f()\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert!(
        outcome
            .events
            .iter()
            .any(|event| matches!(event, VmEvent::ValueReleased { .. })),
        "finally 或函数作用域应执行释放: {:?}",
        outcome.events
    );
}

#[test]
/// finally 抛出主错误时，try 与 finally 两侧的堆值都按未匹配错误边释放。
fn finally_raise_releases_active_scopes() {
    let source = "try\n    payload = \"try-payload\"\nfinally\n    cleanup = \"cleanup\"\n    raise TypeError(code = \"cleanup-error\")\n";
    let outcome = run(&load(source), VmOptions::new());
    assert_eq!(outcome.result.error_code(), Some("cleanup-error"));
    let releases = outcome
        .events
        .iter()
        .filter(|event| matches!(event, VmEvent::ValueReleased { .. }))
        .count();
    assert!(
        releases >= 2,
        "两个活动作用域都应释放: {:?}",
        outcome.events
    );
}

#[test]
/// finally 内嵌套 finally 的未捕获故障不应重新进入已执行的外层子程序。
fn nested_finally_failure_does_not_repeat_outer_finally() {
    let source = "try\n    payload = \"outer-try\"\nfinally\n    try\n        inner = \"inner-try\"\n    finally\n        raise TypeError(code = \"nested-cleanup\")\n";
    let outcome = run(&load(source), VmOptions::new());
    assert_eq!(outcome.result.error_code(), Some("nested-cleanup"));
    let entered = outcome
        .events
        .iter()
        .filter(|event| matches!(event, VmEvent::HandlerEntered { .. }))
        .count();
    assert_eq!(
        entered, 2,
        "内外层 finally 各执行一次: {:?}",
        outcome.events
    );
}

#[test]
/// Fatal 终止事件应只在最终运行边界记录一次。
fn fatal_event_is_emitted_once_at_run_boundary() {
    let outcome = run(
        &load(RUNAWAY),
        VmOptions {
            max_call_depth: 4,
            ..VmOptions::default()
        },
    );
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::FatalRaised { .. }))
            .count(),
        1,
        "Fatal 事件不应按传播帧重复记录: {:?}",
        outcome.events
    );
}

#[test]
/// 多个具体 `catch` 按书写顺序选择第一个匹配处理器。
fn selects_first_matching_catch() {
    let source = "try\n    raise ArithmeticError(code = \"multi\")\ncatch wrong as TypeError\n    wrong_seen = true\ncatch right as ArithmeticError\n    right_seen = true\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    let matches = outcome
        .events
        .iter()
        .filter_map(|event| match event {
            VmEvent::HandlerMatched { catch_type, .. } => catch_type.as_deref(),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(matches, vec!["ArithmeticError"]);
}

#[test]
/// 手工 TAC 的动态错误边界必须拒绝非错误值，并给出稳定类型错误码；错误值通过。
fn dynamic_conversion_check_has_stable_failure_code() {
    let failing = run(&manual_check_program(false), VmOptions::new());
    assert!(
        matches!(failing.result, RunResult::Error(_)),
        "结果: {:?}",
        failing.result
    );
    assert_eq!(failing.result.error_code(), Some(TYPE_MISMATCH_CODE));

    let passing = run(&manual_check_program(true), VmOptions::new());
    assert!(
        passing.result.is_success(),
        "错误值应通过 dynamic_conversion: {:?}",
        passing.result
    );
}

#[test]
/// 字符串布尔转换复用 Runtime 的四个冻结拼写，并把非法拼写映射为类型错误。
fn string_boolean_check_has_stable_failure_code() {
    for conversion in ["raw as bool", "bool(raw)"] {
        for literal in ["true", "True", "false", "False"] {
            let source = format!(
                "def parse(str raw) -> bool\n    return {conversion}\nresult = parse(\"{literal}\")\n"
            );
            let program = load(&source);
            assert!(
                program.unsupported.is_empty(),
                "合法字符串转换不应留下未支持项: {:?}",
                program.unsupported
            );
            for outcome in [
                run(&program, VmOptions::new()),
                run_register(&program, VmOptions::new()),
                run_hybrid(&program, VmOptions::new()),
            ] {
                assert!(
                    outcome.result.is_success(),
                    "{conversion} / {literal} 应成功转换: {:?}",
                    outcome.result
                );
            }
        }
    }

    for conversion in ["raw as bool", "bool(raw)"] {
        let source = format!(
            "def parse(str raw) -> bool\n    return {conversion}\nresult = parse(\"yes\")\n"
        );
        let invalid = load(&source);
        for outcome in [
            run(&invalid, VmOptions::new()),
            run_register(&invalid, VmOptions::new()),
            run_hybrid(&invalid, VmOptions::new()),
        ] {
            assert_eq!(outcome.result.error_code(), Some(TYPE_MISMATCH_CODE));
        }
    }
}

#[test]
/// 未捕获错误应保留产生点和逐层调用帧的后端位置。
fn errors_retain_vm_stack_and_bytecode_offset() {
    let source = "def fail() -> int\n    raise ArithmeticError(code = \"boom\")\n    return 0\nresult = fail()\n";
    let outcome = run(&load(source), VmOptions::new());
    let RunResult::Error(error) = outcome.result else {
        panic!("应得到未捕获错误: {:?}", outcome.result);
    };
    assert!(error.stack().iter().any(|frame| frame.function() == "fail"));
    assert!(
        error
            .stack()
            .iter()
            .any(|frame| frame.function().is_empty())
    );
    assert!(
        error
            .stack()
            .iter()
            .all(|frame| frame.backend().bytecode_offset.is_some())
    );
}

#[test]
/// 错误堆栈使用编码器的物理 pc，而不是源码区间起点。
fn errors_use_encoded_pc_mapping() {
    let source = "def fail() -> int\n    raise ArithmeticError(code = \"boom\")\n    return 0\nresult = fail()\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("前端应成功");
    let tac = lower_program(&artifact.ir);
    let fail_index = tac
        .functions
        .iter()
        .position(|function| function.name == "fail")
        .expect("fail 函数应存在");
    let (block, instruction_index, source_start) = tac.functions[fail_index]
        .blocks
        .iter()
        .find_map(|block| {
            block
                .instructions
                .iter()
                .enumerate()
                .find_map(|(index, instruction)| {
                    matches!(instruction.op, TacOp::Raise { .. }).then_some((
                        block.id,
                        index,
                        instruction.span.start,
                    ))
                })
        })
        .expect("fail 函数应有 Raise 指令");
    let map = build_pc_map(&tac, OperandWidth::Leb128).expect("TAC 应可建立 pc 映射");
    let expected_pc = map
        .pc_at(FuncId::new(fail_index as u32), block, instruction_index)
        .expect("Raise 应有物理 pc");
    assert_ne!(
        expected_pc as usize, source_start,
        "测试必须区分源码偏移与物理 pc"
    );

    let outcome = run(&tac, VmOptions::new());
    let RunResult::Error(error) = outcome.result else {
        panic!("应得到未捕获错误: {:?}", outcome.result);
    };
    let frame = error
        .stack()
        .iter()
        .find(|frame| frame.function() == "fail")
        .expect("错误堆栈应包含 fail 帧");
    assert_eq!(frame.backend().bytecode_offset, Some(expected_pc as u64));
}

/// 构造只包含 `LoadConst`/`MakeError`/`Check` 的最小 TAC 程序，隔离检查执行语义。
fn manual_check_program(error_value: bool) -> TacProgram {
    let span = IrSpan::new(0, 1);
    let mut constants = ConstPool::new();
    let integer = constants.intern(TacConstant::Int(1));
    let value = VReg::new(0);
    let error = VReg::new(1);
    let mut categories = CategoryMap::new();
    categories.insert(
        value,
        if error_value {
            RegisterClass::Dynamic
        } else {
            RegisterClass::Int
        },
    );
    categories.insert(error, RegisterClass::Dynamic);

    let mut entry = vec![];
    if error_value {
        entry.push(TacInstr::with_dst(
            TacOp::MakeError {
                type_name: "TypeError".to_owned(),
                code: None,
                message: None,
            },
            value,
            span,
        ));
    } else {
        entry.push(TacInstr::with_dst(TacOp::LoadConst(integer), value, span));
    }
    entry.push(TacInstr::new(
        TacOp::Check {
            kind: "dynamic_conversion".to_owned(),
            value,
            on_failure: BlockId::new(1),
            expected: None,
        },
        span,
    ));
    entry.push(TacInstr::new(TacOp::Return { value: None }, span));
    let failure = vec![
        TacInstr::with_dst(
            TacOp::MakeError {
                type_name: "TypeError".to_owned(),
                code: None,
                message: None,
            },
            error,
            span,
        ),
        TacInstr::new(TacOp::Raise { value: error }, span),
    ];
    TacProgram {
        version: 1,
        abi: TacAbi {
            bytecode_abi_version: 1,
            runtime_abi_version: 1,
            ir_version: 1,
            language_version: "0.1.0".to_owned(),
            target: "test".to_owned(),
        },
        constants,
        signatures: xiao_bytecode::research::CallSigTable::new(),
        functions: vec![TacFunction {
            name: String::new(),
            signature: None,
            entry: BlockId::new(0),
            blocks: vec![
                TacBlock {
                    id: BlockId::new(0),
                    scope: 0,
                    instructions: entry,
                },
                TacBlock {
                    id: BlockId::new(1),
                    scope: 0,
                    instructions: failure,
                },
            ],
            parameters: Vec::new(),
            locals: Vec::new(),
            categories: categories.clone(),
            scopes: vec![0],
            handlers: Vec::new(),
            value_registers: std::collections::BTreeMap::new(),
            span,
        }],
        categories,
        plans: Vec::new(),
        selection_plans: Vec::new(),
        broadcast_assignment_plans: Vec::new(),
        random_seed_plans: Vec::new(),
        table_definitions: Vec::new(),
        unsupported: Vec::new(),
    }
}
