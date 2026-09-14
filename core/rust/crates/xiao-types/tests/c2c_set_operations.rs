//! C2-C 集合运算的静态类型、比较和原地赋值规格测试。

use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::{
    RuntimeCheckKind, SET_COMPARISON_TYPE_CODE, SET_OPERATION_TYPE_CODE, SetType, Type, TypeChecker,
};

/// 解析并检查一段 Xiao 源码。
fn check(source: &str) -> xiao_types::TypeCheckResult {
    let file = SourceFile::from_text(source);
    let parsed = Parser::new(&file).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    TypeChecker::check(&file, &parsed.program.expect("program"))
}

/// 读取绑定的静态类型。
fn binding_type(result: &xiao_types::TypeCheckResult, name: &str) -> Type {
    result
        .binding(name)
        .unwrap_or_else(|| panic!("missing binding {name}"))
        .scheme
        .ty
        .clone()
}

#[test]
/// 并集、交集、差集和对称差分别遵守 C2-C 的成员类型规则。
fn infers_set_operation_result_types() {
    let result = check(
        "left = {1, \"x\"}\nright = {1, true}\nunion = left + right\nintersection = left & right\ndifference = left - right\nsymmetric = left ^ right\n",
    );
    assert!(result.is_success(), "{:?}", result.diagnostics());
    assert_eq!(
        binding_type(&result, "union").to_string(),
        "set<bool | int | str>"
    );
    assert_eq!(
        binding_type(&result, "intersection").to_string(),
        "set<int>"
    );
    assert_eq!(
        binding_type(&result, "difference").to_string(),
        "set<int | str>"
    );
    assert_eq!(
        binding_type(&result, "symmetric").to_string(),
        "set<bool | int | str>"
    );
}

#[test]
/// 静态无共同成员的交集使用 `Empty`，不伪装成未知集合。
fn represents_static_empty_intersection() {
    let result = check("numbers = {1}\nwords = {\"x\"}\nempty = numbers & words\n");
    assert!(result.is_success(), "{:?}", result.diagnostics());
    assert_eq!(binding_type(&result, "empty"), Type::Set(SetType::empty()));
    assert_eq!(binding_type(&result, "empty").to_string(), "set<never>");
}

#[test]
/// 未知集合参与运算时保留可证明成员并登记集合运算检查。
fn propagates_dynamic_set_boundary() {
    let result = check(
        "unknown_set = set()\nknown = {1}\nunion = unknown_set + known\nintersection = unknown_set & known\nvalue = unknown()\ndynamic_union = known + value\n",
    );
    assert!(
        result
            .runtime_checks()
            .iter()
            .filter(|check| check.kind == RuntimeCheckKind::SetOperation)
            .count()
            >= 3
    );
    assert_eq!(
        binding_type(&result, "union").to_string(),
        "set<int | dynamic>"
    );
    assert_eq!(
        binding_type(&result, "intersection").to_string(),
        "set<int | dynamic>"
    );
    assert_eq!(binding_type(&result, "dynamic_union"), Type::Dynamic);
}

#[test]
/// 所有集合比较都返回 `bool`，动态边界登记独立的比较检查。
fn checks_set_comparisons() {
    let result = check(
        "left = {1}\nright = {1, 2}\nequal = left == right\nnot_equal = left != right\nsubset = left < right\nsubset_or_equal = left <= right\nsuperset = right > left\nsuperset_or_equal = right >= left\nunknown_set = set()\ndynamic_compare = left == unknown_set\n",
    );
    assert!(result.is_success(), "{:?}", result.diagnostics());
    for name in [
        "equal",
        "not_equal",
        "subset",
        "subset_or_equal",
        "superset",
        "superset_or_equal",
        "dynamic_compare",
    ] {
        assert_eq!(
            binding_type(&result, name),
            Type::scalar(xiao_syntax::ScalarType::Bool)
        );
    }
    assert!(
        result
            .runtime_checks()
            .iter()
            .any(|check| check.kind == RuntimeCheckKind::SetComparison)
    );
}

#[test]
/// 四种原地运算按左值锁定类型检查，并允许空结果写回集合。
fn checks_set_compound_assignments() {
    let valid = check(
        "left = {1, \"x\"}\nright = {1}\nleft += right\nleft -= right\nleft &= right\nleft ^= right\n",
    );
    assert!(valid.is_success(), "{:?}", valid.diagnostics());

    let empty = check("left = {1}\nright = {\"x\"}\nleft &= right\n");
    assert!(empty.is_success(), "{:?}", empty.diagnostics());

    let invalid = check("set<int> target = set()\nsource = {\"x\"}\ntarget += source\n");
    assert!(invalid.has_errors());
    assert!(invalid.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == "X02-TYPE-004"
            && diagnostic.message_id() == "x02.type.compound_result_mismatch"
    }));
}

#[test]
/// 集合与标量混用生成稳定集合诊断；普通标量加法仍由旧规则处理。
fn rejects_mixed_set_operations() {
    let result = check("bad_union = {1} + 2\nbad_intersection = 1 & 2\nbad_compare = {1} < 2\n");
    let codes = result
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&SET_OPERATION_TYPE_CODE));
    assert!(codes.contains(&SET_COMPARISON_TYPE_CODE));
    assert_eq!(binding_type(&result, "bad_union"), Type::Dynamic);
}

#[test]
/// 动态值与已知标量不能把标量伪装成集合操作数。
fn rejects_dynamic_set_operation_with_known_scalar() {
    let result = check("value = unknown()\nbad = value & 1\n");
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == SET_OPERATION_TYPE_CODE
            && diagnostic.message_id() == "x03.type.set_operation_requires_sets"
    }));
    assert!(
        !result
            .runtime_checks()
            .iter()
            .any(|check| { check.kind == RuntimeCheckKind::SetOperation })
    );
}
