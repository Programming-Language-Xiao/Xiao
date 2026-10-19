//! 20H 集合方法式 API 的类型、VM 与静态链接回归。

use xiao_codegen_llvm::{CodegenOptions, NativeBuild};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendContext, FrontendRequest,
    FrontendVmDriver,
};

fn run(source: &str) -> DriverOutcome {
    let request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("20H 前端通过");
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
/// 八个方法与自由函数共享结果顺序、别名可见性和集合变更语义。
fn methods_match_set_module_semantics() {
    let outcome = run(
        "values = {3, 1, 2}\nalias = values\nalias.add(2)\nvalues.add(4)\nprint(values.size())\nprint(values.contains(4))\nfor value in values\n    print(value)\nalias.remove(3)\nprint(values.contains(3))\nfor value in values.union({4, 5})\n    print(value)\nfor value in values.intersection({2, 5, 4})\n    print(value)\nfor value in values.difference({1})\n    print(value)\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(
        output(&outcome),
        "4\ntrue\n3\n1\n2\n4\nfalse\n1\n2\n4\n5\n2\n4\n2\n4\n"
    );
}

#[test]
/// 接收者表达式只求值一次，链式方法继续复用同一模块函数体。
fn method_receiver_is_evaluated_once() {
    let outcome = run(
        "def make() -> set<int>\n    print(\"once\")\n    return {1}\nprint(make().union({2}).size())\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "once\n2\n");
}

#[test]
/// 用户函数与集合方法同名时，方法仍绑定官方 `stdlib.set`，不会触发重复定义或误调用。
fn method_linking_isolated_from_user_function_names() {
    let outcome = run(
        "def add(set<int> values, int value) -> none\n    print(\"local\")\nvalues = {1}\nvalues.add(2)\nprint(values.size())\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "2\n");
}

#[test]
/// 集合模块的显式导入与同名用户函数并存时，方法静态副本仍保持隔离。
fn explicit_set_import_and_user_method_name_remain_isolated() {
    let outcome = run(
        "from stdlib.set import size\ndef add(set<int> values, int value) -> none\n    return none\nvalues = {1}\nvalues.add(2)\nprint(size(values))\nprint(values.size())\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "2\n2\n");
}

#[test]
/// 归档使用宿主标准库时，方法调用走模块函数路径并保持同一套语义。
fn external_standard_library_methods_use_module_loader() {
    let context = FrontendContext::host();
    let context = FrontendContext {
        external_standard_library: true,
        ..context
    };
    let request = FrontendRequest::from_text(
        "values = {1}\nvalues.add(2)\nprint(values.union({3}).size())\n",
    )
    .with_context(context);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("外部标准库方法路径前端应通过");
    let outcome = FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(request));
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "3\n");
}

#[test]
/// 表方法同名不应隐式暴露未导入的 `stdlib.set` 自由函数。
fn table_methods_do_not_expose_set_free_functions() {
    let source = "[[Counter]]\n    def add(self, int amount) -> none\n        return none\nitem = new Counter()\nitem.add(1)\nadd({1}, 2)\n";
    let error = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect_err("未导入的 set.add 不应因表方法语法而可见");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X02-TYPE-001"),
        "{error:?}"
    );
}

#[test]
/// 表上的 `add` 方法保持原有表调用路径，不会被集合静态链接接管。
fn table_method_named_add_remains_executable() {
    let outcome = run(
        "[[Counter]]\n    def add(self, int amount) -> none\n        print(amount)\n        return none\nitem = new Counter()\nitem.add(1)\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "1\n");
}

#[test]
/// LLVM 动态降低器复用自由函数实现，方法链与接收者求值均能生成可校验 IR。
fn native_lowering_supports_set_methods_and_chains() {
    let source = "def add(set<int> values, int value) -> none\n    return none\nvalues = {1}\nprint(values.union({2}).add(3))\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("集合方法 LLVM 夹具前端应通过");
    let module = NativeBuild::new()
        .lower(artifact.ir(), &CodegenOptions::default())
        .expect("集合方法应完成 LLVM 降低");
    assert!(module.uses_runtime);
    assert!(module.text.contains("define void @xiao.fn."));
}

#[test]
/// 动态接收者、方法值和静态集合未知方法分别使用冻结诊断身份。
fn method_diagnostics_are_stable() {
    let cases = [
        (
            "def f(value) -> none\n    value.add(1)\nf(1)\n",
            "X03-TYPE-024",
        ),
        ("values = {1}\nmethod = values.add\n", "X03-TYPE-025"),
        ("values = {1}\nvalues.missing()\n", "X03-TYPE-026"),
    ];
    for (source, code) in cases {
        let error = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .expect_err("方法负例应被前端拒绝");
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == code),
            "{code}: {error:?}"
        );
    }
}
