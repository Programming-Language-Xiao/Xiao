//! 20K 字典动态写入与开放字典注解回归。

use xiao_codegen_llvm::{CodegenOptions, NativeBuild};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver,
};

fn run(source: &str) -> DriverOutcome {
    let request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("20K 前端通过");
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
fn dynamic_dict_table_and_column_writes_update_aliases() {
    let outcome = run(
        "values = {name = 1}\nalias = values\nkey = \"name\"\nalias[key] = 2\ncolumn = <first = 3, second = 4>\ni = 1\ncolumn[i] = 5\nprint(values[\"name\"])\nprint(column[\"second\"])\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "2\n5\n");
}

#[test]
fn dynamic_dict_write_errors_keep_runtime_codes() {
    assert_eq!(
        run("values = {name = 1}\nkey = \"missing\"\nvalues[key] = 2\n").code(),
        Some("X06-RUNTIME-015")
    );
    assert_eq!(
        run("column = <first = 1>\ni = 1\ncolumn[i] = 2\n").code(),
        Some("X06-RUNTIME-017")
    );
}

#[test]
fn static_missing_dict_write_remains_type_error() {
    let error = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "values = {name = 1}\nvalues[\"missing\"] = 2\n",
        ))
        .expect_err("静态缺失键写入应在类型层拒绝");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-005"),
        "{error:?}"
    );
}

#[test]
fn open_dict_annotation_accepts_fixed_shape_values() {
    let outcome = run(
        "def get(dict<str, int> values, str key) -> int\n    return values[key]\nvalues = {name = 7}\nprint(get(values, \"name\"))\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "7\n");
}

#[test]
fn generic_open_dict_annotation_infers_value_type() {
    let outcome = run(
        "def head<T>(dict<str, T> values) -> T\n    return values[\"name\"]\nprint(head({name = 9}))\nprint(head({name = \"雪\"}))\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "9\n雪\n");
}

#[test]
fn native_lowering_emits_dynamic_dict_write_and_queries() {
    let request = FrontendRequest::from_text(
        "from stdlib.dict import set as dict_set, keys\ndict_data = {name = 1}\nkey = \"name\"\ndict_set(dict_data, key, 2)\nprint(keys(dict_data)[0])\n",
    );
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("20K 原生前端通过");
    let module = NativeBuild::new()
        .lower(artifact.ir(), &CodegenOptions::default())
        .expect("20K 原生降低通过");
    assert!(module.uses_runtime);
    assert!(module.text.contains("xiao_runtime_value_index_set_dynamic"));
    assert!(module.text.contains("xiao_runtime_value_dict_keys"));
}

#[test]
fn stdlib_dict_queries_share_mutable_handles_and_order() {
    let outcome = run(
        "from stdlib.dict import get, set, remove, contains, keys, values as dict_values\nvalues = {name = 1, other = 2}\nset(values, \"name\", 7)\nprint(get(values, \"name\"))\nprint(contains(values, \"other\"))\nprint(contains(values, \"missing\"))\nfor key in keys(values)\n    print(key)\nfor value in dict_values(values)\n    print(value)\nremove(values, \"name\")\nprint(contains(values, \"name\"))\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(
        output(&outcome),
        "7\ntrue\nfalse\nname\nother\n7\n2\nfalse\n"
    );
}

#[test]
fn stdlib_dict_remove_missing_key_is_015() {
    let outcome = run(
        "from stdlib.dict import remove\ndict_data = {name = 1}\nremove(dict_data, \"missing\")\n",
    );
    assert_eq!(outcome.code(), Some("X06-RUNTIME-015"), "{outcome:?}");
}

#[test]
fn dict_remove_invalidates_fixed_key_shape() {
    let outcome = run(
        "from stdlib.dict import remove\ndict_data = {name = 1}\nremove(dict_data, \"name\")\nprint(dict_data[\"name\"])\n",
    );
    assert_eq!(outcome.code(), Some("X06-RUNTIME-015"), "{outcome:?}");
}
