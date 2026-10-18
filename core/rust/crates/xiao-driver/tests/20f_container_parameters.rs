//! 20F 通用集合参数、官方 `stdlib.set` 和双实例运行回归。

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver,
};

static ARCHIVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn run(source: &str) -> DriverOutcome {
    let request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("20F 前端通过");
    FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(request))
}

#[test]
/// 同一泛型函数可以在一个程序中处理整数和字符串集合。
fn local_generic_set_function_has_multiple_instances() {
    let outcome = run(
        "def size<T>(set<T> values) -> int\n    count = 0\n    for value in values\n        count = count + 1\n    return count\nprint(size({1, 2}))\nprint(size({\"a\", \"b\"}))\n",
    );
    let output = outcome
        .as_executed()
        .into_iter()
        .flat_map(|execution| execution.events())
        .filter_map(|event| match event {
            xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output, "2\n2\n");
}

#[test]
/// 裸类型参数也按调用点分别实例化，避免只在集合路径上产生多态。
fn local_generic_scalar_function_has_multiple_instances() {
    let outcome = run(
        "def identity<T>(T value) -> T\n    return value\nprint(identity(1))\nprint(identity(\"x\"))\n",
    );
    let output = outcome
        .as_executed()
        .into_iter()
        .flat_map(|execution| execution.events())
        .filter_map(|event| match event {
            xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output, "1\nx\n");
}

#[test]
/// 官方模块的两个泛型入口与本地实现一致，并进入保留标准库命名空间。
fn official_set_module_supports_alias_and_empty_set() {
    let outcome = run(
        "from stdlib.set import size as count, is_empty\nprint(count({1, 2}))\nprint(is_empty(set()))\n",
    );
    let output = outcome
        .as_executed()
        .into_iter()
        .flat_map(|execution| execution.events())
        .filter_map(|event| match event {
            xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output, "2\ntrue\n");
    assert!(outcome.as_executed().is_some_and(|execution| execution.events().iter().any(|event| matches!(event, xiao_vm::VmEvent::ModuleLoaded { module } if module == "stdlib.set"))));
}

#[test]
/// 显式类型参数在函数体内不能被静默特化为具体标量。
fn rigid_type_parameter_use_has_stable_diagnostic() {
    let request = FrontendRequest::from_text("def bad<T>(T value) -> int\n    return value + 1\n");
    let error = FrontendCompiler::new()
        .compile(&request)
        .expect_err("刚性类型参数应拒绝具体运算");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X04-TYPE-008"),
        "{error:?}"
    );
}

#[test]
/// 刚性类型参数出现在返回统一中时也使用 20F 稳定诊断。
fn rigid_type_parameter_return_has_stable_diagnostic() {
    let request = FrontendRequest::from_text("def bad<T>(T value) -> T\n    return 1\n");
    let error = FrontendCompiler::new()
        .compile(&request)
        .expect_err("返回值不应把刚性类型参数特化为 int");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X04-TYPE-008"),
        "{error:?}"
    );
}

#[test]
/// 直接递归调用复用当前刚性签名，禁止把递归函数特化成具体类型。
fn generic_recursive_call_keeps_rigid_signature() {
    let request = FrontendRequest::from_text("def loop<T>(T value) -> int\n    return loop(1)\n");
    let error = FrontendCompiler::new()
        .compile(&request)
        .expect_err("递归调用不应把 T 特化为 int");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X04-TYPE-008"),
        "{error:?}"
    );
}

#[test]
/// 20E 归档闭包会记录 `stdlib.set`，但不把官方模块对象复制入归档。
fn archive_closure_records_official_set_module() {
    let root = std::env::temp_dir().join(format!(
        "xiao-20f-archive-{}-{}",
        std::process::id(),
        ARCHIVE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    let input = root.join("main.xiao");
    fs::write(&input, "from stdlib.set import size\nprint(size({1}))\n").unwrap();
    let mut request = xiao_driver::ArchiveBuildRequest::new(&input);
    request.cache_root = Some(root.join("cache"));
    let result =
        xiao_driver::build_archive(&request, &xiao_driver::CancellationToken::new()).unwrap();
    assert!(
        result
            .index
            .stdlib_modules
            .contains(&"stdlib.set".to_owned())
    );
    assert!(
        result
            .index
            .entries
            .iter()
            .all(|entry| !entry.module.starts_with("stdlib"))
    );
    let _ = fs::remove_dir_all(root);
}
