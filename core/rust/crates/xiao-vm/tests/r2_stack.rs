//! 09R2 栈式载体解释器规格。

use xiao_bytecode::research::lower_program;
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_vm::research::{RunResult, VmEvent, VmOptions, run};

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
/// 函数调用与循环应真的被执行，并记录调用深度。
fn executes_calls_and_loops() {
    let source = "def count(int limit) -> int\n    total = 0\n    while total != limit\n        total = total + 1\n    return total\nresult = count(3)\n";
    let outcome = run(&load(source), VmOptions::new());
    assert!(outcome.result.is_success(), "结果: {:?}", outcome.result);
    assert!(outcome.metrics.max_call_depth >= 2, "应进入被调函数");
}

#[test]
/// 递归深度超过上限时产生不可恢复的栈溢出故障，而不是普通错误。
fn stack_overflow_is_fatal() {
    let outcome = run(&load(RUNAWAY), VmOptions { max_call_depth: 8 });
    assert!(
        matches!(outcome.result, RunResult::Fatal(_)),
        "结果: {:?}",
        outcome.result
    );
    assert_eq!(outcome.result.error_code(), Some("X07-FATAL-004"));
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
    assert_eq!(outcome.result.error_code(), Some("X06-RUNTIME-009"));
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

    let fatal = run(&load(RUNAWAY), VmOptions { max_call_depth: 8 });
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
