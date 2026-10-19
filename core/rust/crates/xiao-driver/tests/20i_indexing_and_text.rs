//! 20I 非字面量下标、数组注解、数组追加与文本接口回归。

use xiao_codegen_llvm::{CodegenOptions, NativeBuild};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver,
};

fn run(source: &str) -> DriverOutcome {
    let request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("20I 前端通过");
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
fn expression_indices_use_codepoints_and_array_positions() {
    let outcome = run(
        "text = \"小雪\"\ni = 0\nprint(text[i + 1])\nvalues = [10, 20, 30]\nprint(values[i + 2])\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "雪\n30\n");
}

#[test]
fn expression_indices_can_call_index_producers() {
    let outcome =
        run("def pick() -> int\n    return 1\nvalues = [10, 20]\nprint(values[pick()])\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "20\n");
}

#[test]
fn set_methods_inside_index_expressions_are_linked() {
    let outcome = run("values = [10, 20]\nprint(values[{1}.size()])\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "20\n");
}

#[test]
fn expression_indices_cover_tuple_and_dict_column() {
    let outcome = run(
        "tuple = (10, 20)\ncolumn = <first = \"甲\", second = \"乙\">\ni = 1\nprint(tuple[i])\nprint(column[i + 0])\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "20\n乙\n");
}

#[test]
fn array_annotations_and_push_preserve_shared_order() {
    let outcome = run(
        "def first([int] values) -> int\n    return values[0]\ndef append([int] values, int value) -> none\n    array_push(values, value)\n    return none\nvalues = [1, 2]\nalias = values\nappend(alias, 3)\nprint(first(values))\nfor value in values\n    print(value)\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "1\n1\n2\n3\n");

    let nested = run("values = []\nalias = values\narray_push(alias, [7])\nprint(values[0][0])\n");
    assert!(nested.is_success(), "{nested:?}");
    assert_eq!(output(&nested), "7\n");
}

#[test]
fn generic_array_annotations_infer_element_types() {
    let outcome = run(
        "def head<T>([T] values) -> T\n    return values[0]\nprint(head([7]))\nprint(head([\"雪\"]))\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "7\n雪\n");
}

#[test]
fn text_find_slice_and_split_keep_frozen_boundaries() {
    let outcome = run(
        "from stdlib.text import find, slice, split\nprint(find(\"a小雪a\", \"小雪\"))\nprint(find(\"abc\", \"x\"))\nprint(find(\"abc\", \"\"))\nprint(slice(\"小雪\", 0, 1))\nfor part in split(\"a,,b,\", \",\")\n    print(part)\nfor part in split(\"\", \",\")\n    print(part)\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(&outcome), "1\n-1\n0\n小\na\n\nb\n\n\n");
}

#[test]
fn indexing_boundaries_keep_static_and_runtime_diagnostics() {
    for (source, code) in [
        ("text = \"x\"\nprint(text[true])\n", "X03-TYPE-003"),
        ("values = [1]\nprint(values[2])\n", "X03-TYPE-004"),
        ("values = [1]\nprint(values[1 + 1])\n", "X03-TYPE-004"),
        ("values = [1]\nprint(values[(-1)])\n", "X03-TYPE-004"),
        (
            "table = {name = \"x\"}\ni = 0\nprint(table[i + 1])\n",
            "X03-TYPE-008",
        ),
        (
            "values = [1, 2]\ni = 0\nprint(values[i, 1])\n",
            "X03-TYPE-003",
        ),
    ] {
        let error = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .expect_err("非法下标应被前端拒绝");
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == code),
            "{code}: {error:?}"
        );
    }
    let outcome = run("text = \"x\"\ni = -1\nprint(text[i])\n");
    assert_eq!(outcome.code(), Some("X06-RUNTIME-017"), "{outcome:?}");
}

#[test]
fn native_lowering_accepts_expression_indices_and_array_push() {
    let request = FrontendRequest::from_text(
        "from stdlib.text import split\nparts = split(\"a,b\", \",\")\ni = 1\nprint(parts[i])\n",
    );
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("20I 前端通过");
    let module = NativeBuild::new()
        .lower(artifact.ir(), &CodegenOptions::default())
        .expect("20I 原生降低应通过");
    assert!(module.uses_runtime);
}

#[test]
fn text_error_codes_are_stable() {
    let slice = run("from stdlib.text import slice\nslice(\"x\", 0, 2)\n");
    assert_eq!(slice.code(), Some("TEXT_SLICE_OUT_OF_RANGE"), "{slice:?}");
    let split = run("from stdlib.text import split\nsplit(\"x\", \"\")\n");
    assert_eq!(
        split.code(),
        Some("TEXT_SPLIT_EMPTY_SEPARATOR"),
        "{split:?}"
    );
}
