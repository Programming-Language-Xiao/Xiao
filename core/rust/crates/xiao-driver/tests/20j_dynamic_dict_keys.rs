//! 20J 字典表/字典列动态键与 20I 收口回归。

use xiao_codegen_llvm::{CodegenOptions, NativeBuild};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver,
};

fn run(source: &str) -> DriverOutcome {
    let request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("20J 前端通过");
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
fn dict_table_uses_static_key_priority_then_dynamic_string_keys() {
    let outcome = run(
        "values = {name = 10, other = 20}\nkey = \"other\"\nprint(values[name])\nprint(values[key])\nprint(values[key + \"\"])\nprint(values[\"name\"])\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "10\n20\n20\n10\n");
}

#[test]
fn dict_column_distinguishes_static_keys_dynamic_keys_and_positions() {
    let outcome = run(
        "column = <i = 10, j = 20>\ni = 1\nkey = \"j\"\nprint(column[i])\nprint(column[key])\nprint(column[i + 0])\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "10\n20\n20\n");
}

#[test]
fn dict_column_without_key_name_uses_integer_position() {
    let outcome = run("column = <first = 10, second = 20>\ni = 1\nprint(column[i])\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "20\n");
}

#[test]
fn missing_dynamic_key_is_runtime_015() {
    let outcome = run("values = {name = 10}\nkey = \"missing\"\nprint(values[key])\n");
    assert_eq!(outcome.code(), Some("X06-RUNTIME-015"), "{outcome:?}");
}

#[test]
fn dict_dynamic_key_type_errors_are_static() {
    let request = FrontendRequest::from_text("values = {name = 10}\ni = 1\nprint(values[i])\n");
    let error = FrontendCompiler::new()
        .compile(&request)
        .expect_err("字典表动态键必须是 str");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-003"),
        "{error:?}"
    );
}

#[test]
fn missing_static_string_key_remains_a_type_error() {
    let request = FrontendRequest::from_text("values = {name = 10}\nprint(values[\"missing\"])\n");
    let error = FrontendCompiler::new()
        .compile(&request)
        .expect_err("静态字符串键缺失应在类型层拒绝");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-005"),
        "{error:?}"
    );
}

#[test]
fn native_lowering_accepts_dynamic_dictionary_keys() {
    let request =
        FrontendRequest::from_text("values = {name = 10}\nkey = \"name\"\nprint(values[key])\n");
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("20J 前端通过");
    let module = NativeBuild::new()
        .lower(artifact.ir(), &CodegenOptions::default())
        .expect("20J 原生降低应通过");
    assert!(module.uses_runtime);
    assert!(
        module
            .text
            .contains("@xiao_runtime_value_index_get_dynamic")
    );
}
