//! C2-B 异构集合和动态成员的静态闭环规格测试。

use xiao_source::SourceFile;
use xiao_syntax::{Parser, Program};
use xiao_types::{
    RuntimeCheckKind, SET_ELEMENT_TYPE_MISMATCH_CODE, SET_MEMBERSHIP_TYPE_CODE, SetType, Type,
    TypeChecker,
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
    let program: Program = parsed.program.expect("program");
    TypeChecker::check(&file, &program)
}

#[test]
/// 未显式声明的集合应推导静态成员类型并集，而不是因异构元素失败。
fn infers_heterogeneous_member_union() {
    let result = check("values = {1, \"x\", true, 1.0}\n");
    assert!(result.is_success(), "{:?}", result.diagnostics());
    let ty = &result.binding("values").expect("values").scheme.ty;
    let Type::Set(set) = ty else {
        panic!("expected set type, got {ty}");
    };
    assert_eq!(
        set.to_string(),
        "set<bool | float | int | str>",
        "成员类型采用稳定规范化顺序"
    );
    assert!(!set.allows_dynamic());
    assert_eq!(set.to_member_types().len(), 4);
}

#[test]
/// 显式并集保留完整声明，即使初始化器只出现其中一个成员类型。
fn preserves_explicit_union_on_binding() {
    let result = check("set<int | str> values = {1}\nother = set()\n");
    assert!(result.is_success(), "{:?}", result.diagnostics());
    assert_eq!(
        result.binding("values").expect("values").scheme.ty,
        Type::Set(SetType::heterogeneous(vec![
            Type::scalar(xiao_syntax::ScalarType::Int),
            Type::scalar(xiao_syntax::ScalarType::Str),
        ]))
    );
    assert!(matches!(
        result.binding("other").expect("other").scheme.ty,
        Type::Set(SetType::Unknown)
    ));
}

#[test]
/// 显式并集拒绝不在成员集合中的静态类型，`bool` 不等同于 `int`。
fn rejects_static_member_outside_union() {
    let result = check("set<int | str> values = {1, true}\nbad = true in values\n");
    let codes = result
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&SET_ELEMENT_TYPE_MISMATCH_CODE));
    assert!(codes.contains(&SET_MEMBERSHIP_TYPE_CODE));
}

#[test]
/// 动态集合成员保留已知并集并登记哈希和成员判断 Runtime 检查。
fn keeps_dynamic_tail_and_runtime_marks() {
    let result = check(
        "dynamic_value = unknown()\nvalues = {1, dynamic_value}\nfound = dynamic_value in values\n",
    );
    assert!(
        result
            .runtime_checks()
            .iter()
            .any(|check| check.kind == RuntimeCheckKind::SetHashability)
    );
    assert!(
        result
            .runtime_checks()
            .iter()
            .any(|check| check.kind == RuntimeCheckKind::SetMembership)
    );
    let Type::Set(set) = &result.binding("values").expect("values").scheme.ty else {
        panic!("expected set");
    };
    assert!(set.allows_dynamic());
    assert!(set.contains_type(&Type::scalar(xiao_syntax::ScalarType::Int)));
}

#[test]
/// 同类型静态常量重复仍报错；不同静态类型的相同数值不视为重复。
fn isolates_static_equality_by_type() {
    let result = check("same = {1, 1}\ndifferent = {1, 1.0, true}\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-017")
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == "X03-TYPE-017")
            .count(),
        1
    );
    assert!(result.binding("different").is_some());
}

#[test]
/// 旧式 `int name = {...}` 仍作为同构容器约束，不能借异构 C2-B 放宽。
fn keeps_legacy_homogeneous_prefix_strict() {
    let result = check("ids = {1, \"bad\"}\nint strict_ids = {1, \"bad\"}\n");
    assert!(result.binding("ids").is_some());
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SET_ELEMENT_TYPE_MISMATCH_CODE)
    );
}

#[test]
/// 集合赋值按静态成员并集检查；动态尾标转为 Runtime 检查而非静默放行。
fn checks_set_assignment_compatibility() {
    let valid = check("set<int | str> target = set()\nsource = {1, \"x\"}\ntarget = source\n");
    assert!(valid.is_success(), "{:?}", valid.diagnostics());

    let invalid = check("set<int> target = set()\nsource = {1, \"x\"}\ntarget = source\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X02-TYPE-004")
    );

    let dynamic =
        check("set<int> target = set()\nvalue = unknown()\nsource = {value}\ntarget = source\n");
    assert!(
        dynamic
            .runtime_checks()
            .iter()
            .filter(|check| check.kind == RuntimeCheckKind::SetMembership)
            .count()
            >= 1
    );
}
