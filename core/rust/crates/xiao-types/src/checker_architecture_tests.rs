//! 检查器子模块依赖方向的源码级回归测试。

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
/// 验证检查器门面、规则模块和结果模型保持单向依赖。
fn module_dependency_direction_is_acyclic() {
    let facade = include_str!("checker.rs");
    for declaration in [
        "#[path = \"checker/constant.rs\"]",
        "#[path = \"checker/conversion.rs\"]",
        "#[path = \"checker/diagnostic.rs\"]",
        "#[path = \"checker/expression.rs\"]",
        "#[path = \"checker/result.rs\"]",
        "#[path = \"checker/statement.rs\"]",
    ] {
        assert!(facade.contains(declaration), "门面缺少 {declaration}");
    }
    for implementation in [
        "fn check_expression",
        "fn check_statement",
        "fn check_call",
        "fn eval_const",
        "fn check_constant_target",
        "fn type_error",
        "fn push_runtime_check",
    ] {
        assert!(
            !facade.contains(implementation),
            "门面不应保留实现 {implementation}"
        );
    }
    assert!(facade.contains("pub use self::result::"));

    let result = include_str!("checker/result.rs");
    assert_no_dependency("result.rs", result, "super::");
    assert_no_dependency("result.rs", result, "crate::checker");

    let constant = include_str!("checker/constant.rs");
    assert_no_dependency("constant.rs", constant, "super::TypeChecker");
    assert_no_dependency("constant.rs", constant, "super::expression");
    assert_no_dependency("constant.rs", constant, "super::statement");
    assert_no_dependency("constant.rs", constant, "crate::checker");

    let conversion = include_str!("checker/conversion.rs");
    assert!(conversion.contains("super::constant"));
    assert_no_dependency("conversion.rs", conversion, "super::expression");
    assert_no_dependency("conversion.rs", conversion, "super::statement");
    assert_no_dependency("conversion.rs", conversion, "crate::checker");

    let diagnostic = include_str!("checker/diagnostic.rs");
    assert_no_dependency("diagnostic.rs", diagnostic, "super::expression");
    assert_no_dependency("diagnostic.rs", diagnostic, "super::statement");
    assert_no_dependency("diagnostic.rs", diagnostic, "crate::checker");

    let statement = include_str!("checker/statement.rs");
    assert!(statement.contains("super::constant"));
    assert_no_dependency("statement.rs", statement, "super::expression");
    assert_no_dependency("statement.rs", statement, "super::conversion");
    assert_no_dependency("statement.rs", statement, "crate::checker");

    let expression = include_str!("checker/expression.rs");
    assert!(expression.contains("super::constant"));
    assert_no_dependency("expression.rs", expression, "super::statement");
    assert_no_dependency("expression.rs", expression, "super::conversion");
    assert_no_dependency("expression.rs", expression, "crate::checker");
}
