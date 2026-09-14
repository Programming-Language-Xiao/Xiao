//! C2-A 集合静态类型、可哈希和成员判断规格测试。

use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::{
    RuntimeCheckKind, SET_CONSTRUCTOR_ARITY_CODE, SET_DUPLICATE_ELEMENT_CODE,
    SET_ELEMENT_TYPE_MISMATCH_CODE, SET_INDEX_UNSUPPORTED_CODE, SET_MEMBERSHIP_TYPE_CODE,
    SET_UNHASHABLE_ELEMENT_CODE, ScalarType, SetType, Type, TypeChecker,
};

/// 解析并检查一段 Xiao 源码。
fn check(source_text: &str) -> xiao_types::TypeCheckResult {
    let source = SourceFile::from_text(source_text);
    let parsed = Parser::new(&source).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    TypeChecker::check(&source, parsed.program.as_ref().expect("program"))
}

#[test]
/// 非空集合推断单一元素类型，`none` 可作为独立元素类型，`bool` 不参与数值合并。
fn infers_scalar_set_types() {
    let result =
        check("ids = {1, 2, 3}\nflags = {true, false}\nempty = set()\nnone_values = {none}\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    assert_eq!(
        result.binding("ids").expect("ids").scheme.ty,
        Type::Set(SetType::homogeneous(Type::scalar(ScalarType::Int)))
    );
    assert_eq!(
        result.binding("flags").expect("flags").scheme.ty,
        Type::Set(SetType::homogeneous(Type::scalar(ScalarType::Bool)))
    );
    assert_eq!(
        result
            .binding("none_values")
            .expect("none_values")
            .scheme
            .ty,
        Type::Set(SetType::homogeneous(Type::None))
    );
    assert_eq!(
        result.binding("empty").expect("empty").scheme.ty,
        Type::Set(SetType::Unknown)
    );
}

#[test]
/// 未显式集合拒绝异构元素、静态重复元素和不可哈希容器元素。
fn rejects_invalid_set_elements() {
    let result = check(
        "mixed = {1, true}\nduplicate = {1, 1}\narray_value = {[1, 2]}\ntuple_value = {(1, 2)}\n",
    );
    let codes = result
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&SET_ELEMENT_TYPE_MISMATCH_CODE));
    assert!(codes.contains(&SET_DUPLICATE_ELEMENT_CODE));
    assert_eq!(
        codes
            .iter()
            .filter(|code| **code == SET_UNHASHABLE_ELEMENT_CODE)
            .count(),
        2
    );
}

#[test]
/// 显式元素类型约束每个成员，且 `set()` 可在声明上下文中获得元素类型。
fn checks_explicit_set_type() {
    let valid = check("int ids = {1, 2}\nstr names = set()\n");
    assert!(valid.is_success(), "diagnostics: {:?}", valid.diagnostics);
    assert_eq!(
        valid.binding("ids").expect("ids").scheme.ty,
        Type::Set(SetType::homogeneous(Type::scalar(ScalarType::Int)))
    );
    assert_eq!(
        valid.binding("names").expect("names").scheme.ty,
        Type::Set(SetType::homogeneous(Type::scalar(ScalarType::Str)))
    );

    let invalid = check("int ids = {1, \"bad\"}\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SET_ELEMENT_TYPE_MISMATCH_CODE)
    );
}

#[test]
/// `set()` 仅接受零参数，并将错误保留为集合类型以减少级联诊断。
fn checks_set_constructor_arity() {
    let result = check("bad = set(1)\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SET_CONSTRUCTOR_ARITY_CODE)
    );
    assert!(matches!(
        result.binding("bad").expect("bad").scheme.ty,
        Type::Set(SetType::Unknown)
    ));
}

#[test]
/// 集合成员判断返回 `bool`，并检查左值元素类型与可哈希性。
fn checks_membership() {
    let valid = check("ids = {1, 2}\nhas = 2 in ids\nmissing = 3 not in ids\n");
    assert!(valid.is_success(), "diagnostics: {:?}", valid.diagnostics);
    assert_eq!(
        valid.binding("has").expect("has").scheme.ty,
        Type::scalar(ScalarType::Bool)
    );
    assert_eq!(
        valid.binding("missing").expect("missing").scheme.ty,
        Type::scalar(ScalarType::Bool)
    );

    let invalid = check("ids = {1, 2}\nbad = \"x\" in ids\narray = [1]\nbad_hash = array in ids\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SET_MEMBERSHIP_TYPE_CODE)
    );
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SET_UNHASHABLE_ELEMENT_CODE)
    );
}

#[test]
/// 动态成员值登记集合成员和可哈希运行时检查，不伪造静态结果。
fn records_dynamic_set_checks() {
    let result = check("value = unknown()\nvalues = {value}\nfound = value in values\n");
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
}

#[test]
/// 集合不支持数字或高级选择器索引。
fn rejects_set_indexing() {
    let result = check("values = {1, 2}\nfirst = values[0]\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SET_INDEX_UNSUPPORTED_CODE)
    );
}
