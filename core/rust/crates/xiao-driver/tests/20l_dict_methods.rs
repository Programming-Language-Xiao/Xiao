//! 20L 字典方法式 API、20K 收口与门禁回归。

use xiao_codegen_llvm::{CodegenOptions, NativeBuild};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver,
};

/// 编译并在 VM 中执行一段测试源码。
fn run(source: &str) -> DriverOutcome {
    let request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("20L 前端通过");
    FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(request))
}

/// 收集测试执行产生的标准输出事件。
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
/// 方法与自由函数在表和列上的结果、顺序和错误语义一致。
fn dictionary_methods_match_free_functions_on_table_and_column() {
    let outcome = run(
        "from stdlib.dict import get, set, remove, contains, keys, values, size, is_empty\ntable = {name = 1, other = 2}\ncolumn = <name = 3, other = 4>\ntable.set(\"name\", 7)\ncolumn.set(\"name\", 8)\nprint(table.get(\"name\"))\nprint(get(table, \"name\"))\nprint(column.get(\"name\"))\nprint(contains(table, \"other\"))\nprint(column.contains(\"other\"))\nprint(table.size())\nprint(column.is_empty())\nremove(table, \"other\")\nprint(table.contains(\"other\"))\nfor key in column.keys()\n    print(key)\nfor value in table.values()\n    print(value)\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(
        output(&outcome),
        "7\n7\n8\ntrue\ntrue\n2\nfalse\nfalse\nname\nother\n7\n"
    );
}

#[test]
/// 方法调用无需显式导入即可触发标准库静态链接。
fn dictionary_methods_link_without_explicit_import() {
    let outcome = run(
        "values = {name = 1}\nvalues.set(\"name\", 2)\nprint(values.get(\"name\"))\nprint(values.size())\nprint(values.is_empty())\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "2\n1\nfalse\n");
}

#[test]
/// 方法更新和删除通过别名共享句柄并保留 015 缺失键身份。
fn dictionary_methods_preserve_aliases_and_missing_key_015() {
    let outcome = run(
        "values = {name = 1}\nalias = values\nalias.set(\"name\", 2)\nprint(values.get(\"name\"))\nalias.remove(\"name\")\nprint(values.get(\"name\"))\n",
    );
    assert_eq!(outcome.code(), Some("X06-RUNTIME-015"), "{outcome:?}");
}

#[test]
/// 动态接收者、方法值和未知方法使用冻结诊断编号。
fn dictionary_method_boundary_diagnostics_are_stable() {
    for (source, code) in [
        (
            "def call<T>(T value) -> none\n    value.get(\"name\")\n",
            "X03-TYPE-027",
        ),
        ("values = {name = 1}\nmethod = values.get\n", "X03-TYPE-028"),
        ("values = {name = 1}\nvalues.unknown()\n", "X03-TYPE-029"),
    ] {
        let error = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .expect_err("字典方法边界应被前端拒绝");
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == code),
            "{code}: {error:?}"
        );
    }
}

#[test]
/// 传递调用链中的数组变更会擦除调用方固定长度。
fn transitive_array_mutation_invalidates_fixed_length() {
    let outcome = run(
        "def inner([int] values) -> none\n    array_push(values, 30)\ndef outer([int] values) -> none\n    inner(values)\nvalues = [10, 20]\nouter(values)\nprint(values[2])\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "30\n");
}

#[test]
/// 修改后的未知长度数组字面量越界走 017。
fn modified_array_literal_bounds_use_runtime_017() {
    let outcome =
        run("values = [10, 20]\nalias = values\narray_push(alias, 30)\nprint(values[5])\n");
    assert_eq!(outcome.code(), Some("X06-RUNTIME-017"), "{outcome:?}");
}

#[test]
/// 表字段作为选择器根可以执行动态字典写入。
fn member_array_root_write_is_supported() {
    let outcome = run(
        "[[Holder]]\n    values = {name = 1}\nitem = new Holder()\nkey = \"name\"\nitem.values[key] = 7\nprint(item.values[key])\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "7\n");
}

#[test]
/// LLVM 文本降低包含静态链接的字典方法函数体。
fn native_lowering_links_dictionary_method_functions() {
    let request = FrontendRequest::from_text(
        "values = {name = 1}\nvalues.set(\"name\", 2)\nprint(values.get(\"name\"))\n",
    );
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("字典方法原生前端通过");
    let module = NativeBuild::new()
        .lower(artifact.ir(), &CodegenOptions::default())
        .expect("字典方法原生降低通过");
    assert!(module.uses_runtime);
    assert!(module.text.matches("define void @xiao.fn.").count() >= 8);
}
