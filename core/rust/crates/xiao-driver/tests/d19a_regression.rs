//! 19A（19.4）回归集：溢出、动态值、容器路径、drop、模块初始化、数据竞争诊断。
//!
//! 每个样本在 `-O0`..`-O3` 下各跑一遍，错误码、退出码和释放序列必须逐级一致。
//! 本批只留样本，不修实现；样本暴露的缺陷回到对应阶段处理。

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use xiao_diagnostics::{CROSS_THREAD_CODE, XiaoError, XiaoErrorKind};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendContext, FrontendRequest, FrontendVmDriver, run,
};
use xiao_optimizer::OptimizationLevel;
use xiao_vm::VmEvent;

const LEVELS: [OptimizationLevel; 4] = [
    OptimizationLevel::O0,
    OptimizationLevel::O1,
    OptimizationLevel::O2,
    OptimizationLevel::O3,
];

#[derive(Debug, PartialEq, Eq)]
struct Seen {
    code: Option<String>,
    exit: u8,
    drops: Vec<String>,
}

fn observe(source: &str, level: OptimizationLevel) -> Seen {
    let outcome =
        run(&DriverRequest::new(FrontendRequest::from_text(source)).with_optimization_level(level));
    seen_from(&outcome)
}

fn seen_from(outcome: &DriverOutcome) -> Seen {
    let drops = outcome
        .as_executed()
        .map(|execution| {
            execution
                .events()
                .iter()
                .filter_map(|event| match event {
                    VmEvent::ValueReleased {
                        scope, exit, kind, ..
                    } => Some(format!("{scope}:{exit}:{kind}")),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    Seen {
        code: outcome.code().map(ToOwned::to_owned),
        exit: outcome.exit_code().as_process_code(),
        drops,
    }
}

/// 四个级别必须给出同一观察结果，返回该结果。
fn across_levels(label: &str, source: &str) -> Seen {
    let baseline = observe(source, OptimizationLevel::O0);
    for level in &LEVELS[1..] {
        let other = observe(source, *level);
        assert_eq!(baseline, other, "{label}: O0 与 {level:?} 不一致");
    }
    baseline
}

#[test]
fn numeric_overflow_is_reported_identically_at_every_level() {
    let overflow =
        "def square(int value) -> int\n    return value * value\nresult = square(4000000000)\n";
    let seen = across_levels("overflow", overflow);
    assert_eq!(seen.code.as_deref(), Some("X06-RUNTIME-009"));
    assert_ne!(seen.exit, 0);

    let in_range =
        "def square(int value) -> int\n    return value * value\nresult = square(3000000000)\n";
    let seen = across_levels("in-range", in_range);
    assert_eq!(seen.code, None);
    assert_eq!(seen.exit, 0);
}

#[test]
fn dynamic_string_to_bool_boundary_is_stable() {
    for literal in ["true", "True", "false", "False"] {
        let source = format!(
            "def parse(str raw) -> bool\n    return raw as bool\nresult = parse(\"{literal}\")\n"
        );
        let seen = across_levels(literal, &source);
        assert_eq!(seen.code, None, "{literal} 应成功转换");
    }
    let invalid = "def parse(str raw) -> bool\n    return raw as bool\nresult = parse(\"yes\")\n";
    let seen = across_levels("yes", invalid);
    assert_eq!(seen.code.as_deref(), Some("X06-RUNTIME-002"));
}

#[test]
fn container_path_errors_keep_distinct_codes() {
    let valid = "items = [1, 2]\ntable = {name = \"x\"}\na = items[1]\nb = table[name]\n";
    assert_eq!(across_levels("valid", valid).code, None);
    let cases = [
        ("items = [1, 2]\nb = items[3]\n", "X03-TYPE-004"),
        (
            "table = {name = \"x\"}\na = table[missing]\n",
            "X03-TYPE-005",
        ),
        ("items = [1]\na = items[name]\n", "X03-TYPE-003"),
    ];
    for (source, expected) in cases {
        let seen = across_levels(expected, source);
        assert_eq!(seen.code.as_deref(), Some(expected), "{source}");
        assert_ne!(seen.exit, 0);
    }
}

#[test]
fn drop_sequences_are_identical_and_non_empty() {
    let plain = "def f() -> str\n    try\n        return \"result\"\n    finally\n        cleanup = \"cleanup\"\nresult = f()\n";
    let nested = "def f() -> int\n    try\n        outer_try_payload = \"outer-try-payload\"\n        try\n            payload = \"inner-payload\"\n        finally\n            return 2\n    finally\n        outer_payload = \"outer-payload\"\nresult = f()\n";
    for (label, source) in [("plain", plain), ("nested", nested)] {
        let seen = across_levels(label, source);
        assert_eq!(seen.code, None, "{label}");
        assert!(!seen.drops.is_empty(), "{label}: 应至少有一次释放");
    }
}

/// `finally` 内无条件 `return`/`raise` 必须沿生产路径执行，并保持释放顺序。
#[test]
fn finally_with_unconditional_exit_runs_in_production() {
    let returning = "def f() -> int\n    try\n        payload = \"try-payload\"\n    finally\n        return 2\nresult = f()\n";
    let seen = across_levels("return", returning);
    assert_eq!(seen.code, None);
    assert!(!seen.drops.is_empty());

    let raising = "try\n    payload = \"try-payload\"\nfinally\n    cleanup = \"cleanup\"\n    raise TypeError(code = \"cleanup-error\")\n";
    let seen = across_levels("raise", raising);
    assert_eq!(seen.code.as_deref(), Some("cleanup-error"));
    assert!(seen.drops.len() >= 2);
}

static WORKSPACE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let sequence = WORKSPACE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "xiao-d19a-regression-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("创建回归工作区");
        Self(path)
    }

    fn write(&self, relative: &str, text: &str) {
        fs::write(self.0.join(relative), text).expect("写入回归源码");
    }

    fn request(&self, source: &str, level: OptimizationLevel) -> DriverRequest {
        let path = self.0.join("main.xiao");
        self.write("main.xiao", source);
        let mut context = FrontendContext::host();
        context.project_root = Some(self.0.clone());
        DriverRequest::new(FrontendRequest::from_text_at(source, path).with_context(context))
            .with_optimization_level(level)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn module_loads(outcome: &DriverOutcome, module: &str) -> usize {
    outcome
        .as_executed()
        .expect("应已进入 VM")
        .events()
        .iter()
        .filter(
            |event| matches!(event, VmEvent::ModuleLoaded { module: loaded } if loaded == module),
        )
        .count()
}

#[test]
fn module_initialization_runs_once_per_session_at_every_level() {
    let source = "[Local]\n    count = 3\nimport helper\nif helper.State.count != 7\n    raise ArithmeticError(code = \"MODULE_TABLE\")\n";
    for level in LEVELS {
        let workspace = Workspace::new();
        workspace.write("helper.xiao", "[State]\n    count = 7\n");
        let mut driver = FrontendVmDriver::new();
        let first = driver.run(&workspace.request(source, level));
        assert!(first.is_success(), "{level:?}: {first:?}");
        assert_eq!(module_loads(&first, "project:helper"), 1, "{level:?}");
        let second = driver.run(&workspace.request(source, level));
        assert!(second.is_success(), "{level:?}: {second:?}");
        assert_eq!(module_loads(&second, "project:helper"), 0, "{level:?}");
    }
}

#[test]
fn failing_module_initialization_is_not_cached_at_any_level() {
    let mut codes = Vec::new();
    for level in LEVELS {
        let workspace = Workspace::new();
        workspace.write("helper.xiao", "raise ArithmeticError(code = \"BROKEN\")\n");
        let mut driver = FrontendVmDriver::new();
        let failed = driver.run(&workspace.request("import helper\n", level));
        assert!(!failed.is_success(), "{level:?}: {failed:?}");
        codes.push(failed.code().map(ToOwned::to_owned));
        workspace.write("helper.xiao", "answer = 42\n");
        let retried =
            driver.run(&workspace.request("import helper\nvalue = helper.answer\n", level));
        assert!(retried.is_success(), "{level:?}: {retried:?}");
        assert_eq!(module_loads(&retried, "project:helper"), 1, "{level:?}");
    }
    assert!(codes[0].is_some(), "模块初始化失败必须带稳定错误码");
    assert!(
        codes.iter().all(|code| *code == codes[0]),
        "各级别的模块初始化失败错误码必须一致: {codes:?}"
    );
}

/// 源码层目前没有能触发跨线程传递的语法，只能固定运行时诊断本身的身份。
#[test]
fn cross_thread_diagnostic_identity_is_stable() {
    let error = XiaoError::cross_thread();
    assert_eq!(error.code(), CROSS_THREAD_CODE);
    assert_eq!(error.code(), "X06-RUNTIME-010");
    assert_eq!(error.message_id(), "runtime.cross_thread");
    assert_eq!(error.kind(), XiaoErrorKind::Concurrency);
}
