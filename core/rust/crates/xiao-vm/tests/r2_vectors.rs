//! 09R2 共享语义向量。
//!
//! 向量与机型无关：栈式、分类型寄存器式和混合式载体直接复用同一组期望值，
//! 向量里不得出现任何机型专属字段，也禁止为某一型改期望。
//!
//! 期望值锁定四件事：结构化结果、稳定错误码、`(作用域, 退出边, 值, 类别)` 的
//! **完整释放序列**、以及最大调用深度。释放序列来自冻结计划，是后端「没有重排、
//! 去重或漏放」的可观察证据。

use serde::Deserialize;
use xiao_bytecode::research::lower_program;
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_vm::research::{
    Carrier, HybridCarrier, RegisterCarrier, RunResult, StackCarrier, VmEvent, VmOptions, run_with,
};

/// 一份语义向量文件。
#[derive(Debug, Deserialize)]
struct VectorFile {
    /// 对应工程期。
    stage: String,
    /// 快照状态。
    status: String,
    /// 用例列表。
    cases: Vec<VectorCase>,
}

/// 一条语义向量。
#[derive(Debug, Deserialize)]
struct VectorCase {
    /// 用例名。
    name: String,
    /// Xiao 源码。
    source: String,
    /// 调用深度上限；省略时用默认值。
    #[serde(default)]
    max_call_depth: Option<usize>,
    /// 期望结果。
    expect: Expectation,
}

/// 一条用例的期望结果。
#[derive(Debug, Deserialize, PartialEq)]
struct Expectation {
    /// `success`、`error` 或 `fatal`。
    outcome: String,
    /// 期望的稳定错误码或故障码。
    #[serde(default)]
    error_code: Option<String>,
    /// 期望的完整释放序列。
    #[serde(default)]
    releases: Vec<ReleaseRecord>,
    /// 期望达到的最大调用深度。
    max_call_depth: usize,
}

/// 一条释放记录。
#[derive(Debug, Deserialize, PartialEq)]
struct ReleaseRecord {
    /// 触发释放的作用域。
    scope: u32,
    /// 退出边稳定名称。
    exit: String,
    /// `IrValue.id`。
    value: u32,
    /// 强释放或弱释放。
    kind: String,
}

/// 从一次运行结果提取可观察事实。
fn observe<C: Carrier>(case: &VectorCase) -> Expectation {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(case.source.clone()))
        .unwrap_or_else(|error| panic!("用例 {} 前端应成功: {:?}", case.name, error.diagnostics()));
    let tac = lower_program(&artifact.ir);
    let options = VmOptions {
        max_call_depth: case.max_call_depth.unwrap_or(1024),
    };
    let outcome = run_with::<C>(&tac, options);
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
    Expectation {
        outcome: match outcome.result {
            RunResult::Success => "success".to_owned(),
            RunResult::Error(_) => "error".to_owned(),
            RunResult::Fatal(_) => "fatal".to_owned(),
        },
        error_code: outcome.result.error_code().map(str::to_owned),
        releases,
        max_call_depth: outcome.metrics.max_call_depth,
    }
}

/// 断言一份向量文件的全部用例。
fn assert_vectors<C: Carrier>(raw: &str, expected_stage: &str, machine: &str) {
    let file: VectorFile = serde_json::from_str(raw).expect("向量文件应可解析");
    assert_eq!(file.stage, expected_stage);
    assert_eq!(file.status, "verified-runtime");
    assert!(!file.cases.is_empty(), "向量文件不应为空");
    let mut mismatches = Vec::new();
    for case in &file.cases {
        let actual = observe::<C>(case);
        if actual != case.expect {
            mismatches.push(format!(
                "{machine}/{}: 期望 {:?}，实际 {:?}",
                case.name, case.expect, actual
            ));
        }
    }
    assert!(mismatches.is_empty(), "期望不一致的用例：{mismatches:?}");
}

/// 用同一解析与观察路径依次验证三种载体。
fn assert_all_machines(raw: &str, expected_stage: &str) {
    assert_vectors::<StackCarrier>(raw, expected_stage, "stack");
    assert_vectors::<RegisterCarrier>(raw, expected_stage, "register");
    assert_vectors::<HybridCarrier>(raw, expected_stage, "hybrid");
}

#[test]
/// 标量、字符串与显式转换的语义向量。
fn scalar_vectors_are_stable() {
    assert_all_machines(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../tests/spec/09-bytecode/scalar.json"
        )),
        "09R2",
    );
}

#[test]
/// 分支、循环、函数调用与递归的语义向量。
fn control_vectors_are_stable() {
    assert_all_machines(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../tests/spec/09-bytecode/control.json"
        )),
        "09R2",
    );
}

#[test]
/// 可恢复错误与致命故障的语义向量。
fn error_vectors_are_stable() {
    assert_all_machines(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../tests/spec/09-bytecode/errors.json"
        )),
        "09R2",
    );
}

#[test]
/// 容器构造、精确索引与临时值释放的语义向量。
fn container_vectors_are_stable() {
    assert_all_machines(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../tests/spec/09-bytecode/containers.json"
        )),
        "09R2",
    );
}

#[test]
/// 高级选择、嵌套范围、固定种子和广播在三种载体上复用同一向量。
fn selector_vectors_are_stable() {
    assert_all_machines(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../tests/spec/09-bytecode/selectors.json"
        )),
        "09R2",
    );
}
