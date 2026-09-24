//! 动态降低器子模块依赖方向的源码级回归测试。

/// 移除行注释，避免依赖断言被说明文字误触发。
fn code_without_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 断言源码不包含指定的反向依赖。
fn assert_no_dependency(source_name: &str, source: &str, dependency: &str) {
    let code = code_without_line_comments(source);
    assert!(
        !code.contains(dependency),
        "{source_name} 不得依赖 {dependency}"
    );
}

#[test]
/// 验证动态降低器门面、静态路径和职责子模块保持单向依赖。
fn module_dependency_direction_is_acyclic() {
    let facade = include_str!("dynamic.rs");
    for declaration in [
        "#[path = \"dynamic/container.rs\"]",
        "#[path = \"dynamic/control.rs\"]",
        "#[path = \"dynamic/entry.rs\"]",
        "#[path = \"dynamic/expression.rs\"]",
        "#[path = \"dynamic/predicate.rs\"]",
        "#[path = \"dynamic/release.rs\"]",
        "#[path = \"dynamic/runtime_abi.rs\"]",
        "#[path = \"dynamic/slot.rs\"]",
        "#[path = \"dynamic/text.rs\"]",
    ] {
        assert!(facade.contains(declaration), "门面缺少 {declaration}");
    }
    for implementation in [
        "fn emit_statement",
        "fn emit_expression",
        "fn emit_sequence",
        "fn emit_table_descriptor",
        "fn declare_runtime",
        "fn release_for_exit",
        "fn collect_slots",
    ] {
        assert!(
            !facade.contains(implementation),
            "门面不应保留实现 {implementation}"
        );
    }
    for primitive in [
        "fn generate",
        "fn emit(",
        "fn next_temp",
        "fn next_label",
        "fn check_status",
    ] {
        assert!(facade.contains(primitive), "门面缺少生成原语 {primitive}");
    }

    let ir = include_str!("ir.rs");
    let toolchain = include_str!("toolchain.rs");
    assert_no_dependency("ir.rs", ir, "crate::dynamic");
    assert_no_dependency("ir.rs", ir, "dynamic::");
    assert!(ir.contains("crate::text::{escape_llvm, stable_hash}"));
    assert!(facade.contains("crate::text::{escape_llvm, stable_hash}"));
    assert!(!ir.contains("fn escape_llvm"));
    assert!(!ir.contains("fn stable_hash"));
    assert!(!facade.contains("fn escape_llvm"));
    assert!(!facade.contains("fn stable_hash"));
    assert!(!toolchain.contains("fn fnv1a64"));
    assert!(!toolchain.contains("fn stable_hash"));

    let predicate = include_str!("dynamic/predicate.rs");
    let text = include_str!("dynamic/text.rs");
    assert_no_dependency("predicate.rs", predicate, "super::");
    assert_no_dependency("predicate.rs", predicate, "crate::dynamic");
    assert_no_dependency("text.rs", text, "super::");
    assert_no_dependency("text.rs", text, "crate::dynamic");

    let runtime_abi = include_str!("dynamic/runtime_abi.rs");
    let entry = include_str!("dynamic/entry.rs");
    let slot = include_str!("dynamic/slot.rs");
    let release = include_str!("dynamic/release.rs");
    let control = include_str!("dynamic/control.rs");
    let expression = include_str!("dynamic/expression.rs");
    let container = include_str!("dynamic/container.rs");
    let modules = [
        ("runtime_abi.rs", runtime_abi),
        ("entry.rs", entry),
        ("slot.rs", slot),
        ("release.rs", release),
        ("control.rs", control),
        ("expression.rs", expression),
        ("container.rs", container),
    ];
    for (name, source) in modules {
        assert_no_dependency(name, source, "crate::dynamic");
        assert_no_dependency(name, source, "super::entry");
        assert_no_dependency(name, source, "super::control");
        assert_no_dependency(name, source, "super::expression");
        assert_no_dependency(name, source, "super::container");
        assert_no_dependency(name, source, "super::release");
        assert_no_dependency(name, source, "super::runtime_abi");
        assert_no_dependency(name, source, "super::slot");
    }
    assert!(runtime_abi.contains("super::predicate"));
    assert!(slot.contains("super::predicate"));
    assert!(expression.contains("super::predicate"));
    assert!(expression.contains("super::text"));
    assert!(container.contains("super::predicate"));
    assert!(container.contains("super::text"));
}
