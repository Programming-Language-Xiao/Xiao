//! 20G 集合增删、只读包装与稳定缺失错误回归。

use xiao_codegen_llvm::{CodegenOptions, NativeBuild};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver,
};

fn run(source: &str) -> DriverOutcome {
    let request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("20G 前端通过");
    FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(request))
}

fn output(outcome: &DriverOutcome) -> String {
    outcome
        .as_executed()
        .into_iter()
        .flat_map(|execution| execution.events())
        .filter_map(|event| match event {
            xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
/// 插入、删除、别名可见性和四个只读包装共享同一集合语义。
fn mutation_and_readonly_wrappers_observe_shared_ordered_set() {
    let outcome = run(
        "from stdlib.set import size, add, remove, contains, union, intersection, difference\nvalues = {1, 2}\nalias = values\nadd(values, 3)\nprint(size(alias))\nprint(contains(alias, 3))\nprint(size(union(values, {3, 4})))\nprint(size(intersection(values, {2, 4})))\nprint(size(difference(values, {1})))\nremove(alias, 2)\nprint(contains(values, 2))\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "3\ntrue\n4\n1\n2\nfalse\n");
}

#[test]
/// 删除缺失元素生成可恢复的库级错误，并能被普通 Error 捕获。
fn remove_missing_element_is_catchable() {
    let outcome = run(
        "from stdlib.set import remove\nvalues = {1}\ntry\n    remove(values, 2)\n    print(\"bad\")\ncatch err as Error\n    print(\"missing\")\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "missing\n");
}

#[test]
/// 可变入口沿用 `set<T>` 调用点的泛型不可哈希诊断。
fn add_rejects_unhashable_element() {
    let request = FrontendRequest::from_text(
        "from stdlib.set import add\nvalues = {1}\narray = [2]\nadd(values, array)\n",
    );
    let error = FrontendCompiler::new()
        .compile(&request)
        .expect_err("数组不能作为集合元素");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-023"),
        "{error:?}"
    );
}

#[test]
/// 原生降低器必须把标准库别名调用映射回规范函数符号，不能读取不存在的动态槽。
fn native_alias_calls_lower_to_canonical_functions() {
    for source in [
        "from stdlib.set import size as count\nprint(count({\"a\", \"b\"}))\n",
        "from stdlib.text import length as len\nprint(len(\"小雪\"))\n",
    ] {
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .expect("标准库别名源码应通过前端");
        let module = NativeBuild::new()
            .lower(artifact.ir(), &CodegenOptions::default())
            .expect("标准库别名应完成 LLVM 降低");
        assert!(module.uses_runtime, "别名调用应使用 Runtime 动态路径");
        assert!(module.text.contains("define void @xiao.fn."), "{source}");
    }
}

#[test]
/// 集合快照在循环体提前返回时仍由函数帧统一清理，原生 LLVM 降低不得生成悬空释放边。
fn native_set_snapshot_nonlocal_exit_is_lowerable() {
    let source = "def first(set<int> values) -> int\n    for value in values\n        return value\n    return 0\nprint(first({1, 2}))\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("带集合返回的源码应通过前端");
    let module = NativeBuild::new()
        .lower(artifact.ir(), &CodegenOptions::default())
        .expect("集合快照非局部退出应完成 LLVM 降低");
    assert!(module.text.contains("xiao.generated.site"));
}

#[test]
/// 集合循环在开始时冻结快照，覆盖删除当前元素、添加新元素和删除后续元素；
/// 下一轮循环只能观察到上一轮已经提交的集合状态。
fn set_iteration_uses_snapshot() {
    let outcome = run(
        "from stdlib.set import add, remove, size\ncurrent = {1, 2, 3}\nfor value in current\n    remove(current, value)\n    print(value)\nprint(size(current))\nfor value in current\n    print(value)\nadded = {1, 2}\nfor value in added\n    add(added, value + 10)\n    print(value)\nprint(size(added))\nfor value in added\n    print(value)\nfollowing = {1, 2, 3}\nfor value in following\n    if value == 1\n        remove(following, 2)\n    print(value)\nprint(size(following))\nfor value in following\n    print(value)\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(
        output(&outcome),
        "1\n2\n3\n0\n1\n2\n4\n1\n2\n11\n12\n1\n2\n3\n2\n1\n3\n"
    );
}
