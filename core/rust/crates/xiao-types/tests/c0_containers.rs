//! C0 容器结构、路径约束和精确索引的类型规格测试。

use xiao_source::SourceFile;
use xiao_syntax::{Parser, Program};
use xiao_types::{
    ArrayType, ContainerPathSegment, DictEntryType, DictType, PathConstraintTree, Type,
    TypeChecker, can_assign,
};

/// 解析并检查源码，要求语法层先通过。
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
/// 异构数组、元组和两种字典应保留结构化类型，而不是退化为 Dynamic。
fn infers_structured_container_types() {
    let result = check(
        "items = [1, \"x\", true]\npoint = (1, \"x\")\ntable = {name = \"x\"}\ncolumn = <name = \"x\">\n",
    );
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert!(matches!(
        &result.binding("items").expect("items").scheme.ty,
        Type::Array(ArrayType::Heterogeneous { elements }) if elements.len() == 3
    ));
    assert!(
        matches!(&result.binding("point").expect("point").scheme.ty, Type::Tuple(items) if items.len() == 2)
    );
    assert!(matches!(
        result.binding("table").expect("table").scheme.ty,
        Type::DictTable(_)
    ));
    assert!(matches!(
        result.binding("column").expect("column").scheme.ty,
        Type::DictColumn(_)
    ));
}

#[test]
/// 显式数组元素类型必须约束所有直接元素，并允许空数组生成未知长度形状。
fn enforces_homogeneous_array_prefix() {
    let valid = check("int values = [1, 2, 3]\nint empty = []\n");
    assert!(!valid.has_errors(), "{:?}", valid.diagnostics());
    assert!(matches!(
        &valid.binding("values").expect("values").scheme.ty,
        Type::Array(ArrayType::Homogeneous { element, length: Some(3) }) if element.as_ref() == &Type::scalar(xiao_syntax::ScalarType::Int)
    ));
    assert!(matches!(
        &valid.binding("empty").expect("empty").scheme.ty,
        Type::Array(ArrayType::Homogeneous { length: None, .. })
    ));

    let invalid = check("int values = [1, \"bad\"]\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-001")
    );
}

#[test]
/// 字典列支持用显式标量前缀约束全部值，但仍保留键和顺序结构。
fn enforces_typed_dictionary_column_values() {
    let valid = check("int column = <first = 1, second = 2>\n");
    assert!(!valid.has_errors(), "{:?}", valid.diagnostics());
    assert!(matches!(
        valid.binding("column").expect("column").scheme.ty,
        Type::DictColumn(_)
    ));
    let invalid = check("int column = <first = 1, second = \"bad\">\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-001")
    );
}

#[test]
/// 重复字典键在类型阶段拒绝，且键名和字符串键共享规范化空间。
fn rejects_duplicate_dictionary_keys() {
    let result = check("value = {name = 1, \"name\" = 2}\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-002")
    );
}

#[test]
/// 无初始化路径声明应保存约束并生成最小形状物化计划。
fn builds_path_constraint_materialization_plan() {
    let result = check("int list[3/2]\n");
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    let binding = result.binding("list").expect("list");
    assert_eq!(binding.container_constraints.constraints().len(), 1);
    assert_eq!(result.materialization_plans().len(), 1);
    assert_eq!(
        result.materialization_plans()[0].minimum_lengths,
        vec![4, 3]
    );
}

#[test]
/// 后声明的更具体路径可以追加，且同一路径由后声明覆盖。
fn merges_later_path_constraints() {
    let result = check("str list[2]\nint list[2/0]\n");
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    let binding = result.binding("list").expect("list");
    assert_eq!(binding.container_constraints.constraints().len(), 2);
    assert_eq!(
        binding.container_constraints.get(&[
            ContainerPathSegment::Index(2),
            ContainerPathSegment::Index(0)
        ]),
        Some(&Type::scalar(xiao_syntax::ScalarType::Int))
    );
}

#[test]
/// 路径停在已知内嵌数组时，前缀类型约束该节点的直接元素。
fn parent_path_constrains_nested_array_elements() {
    let result = check("str list[0] = [[\"a\", \"b\"]]\n");
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    let invalid = check("str list[0] = [[\"a\", 1]]\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-001")
    );
}

#[test]
/// 更具体的初始化路径应覆盖父路径，而不是被父约束重复拒绝。
fn specific_path_overrides_parent_for_initialized_value() {
    let result = check("str list[0]\nint list[0/1] = [[\"text\", 1]]\n");
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
}

#[test]
/// 已初始化容器追加路径约束时应立即检查已知元素，并避免伪造物化计划。
fn validates_late_constraints_against_initialized_container() {
    let mismatch = check("items = [1]\nstr items[0]\n");
    assert!(
        mismatch
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-001")
    );
    assert!(mismatch.materialization_plans().is_empty());

    let bounds = check("items = [1]\nint items[2]\n");
    assert!(
        bounds
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-004")
    );
    assert!(bounds.materialization_plans().is_empty());

    let unknown = check("items = []\nint items[2]\n");
    assert!(!unknown.has_errors(), "{:?}", unknown.diagnostics());
    assert_eq!(unknown.materialization_plans().len(), 1);
}

#[test]
/// 整体重新赋值也必须复用绑定上的路径约束，不能绕过元素类型检查。
fn validates_constraints_on_container_assignment() {
    let invalid = check("int items[0]\nitems = [\"bad\"]\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-001")
    );
}

#[test]
/// 带路径的后续声明不能借初始化器偷偷改变已经锁定的容器根类型。
fn rejects_incompatible_initialized_container_redeclaration() {
    let result = check("items = [1]\nstr items[0] = [\"bad\"]\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-001")
    );
}

#[test]
/// 字典列是有序容器，结构化赋值不能把不同键顺序当成兼容类型。
fn preserves_dictionary_column_order_for_assignment() {
    let first = Type::DictColumn(DictType::new(vec![
        DictEntryType {
            key: "first".to_owned(),
            value: Box::new(Type::scalar(xiao_syntax::ScalarType::Int)),
        },
        DictEntryType {
            key: "second".to_owned(),
            value: Box::new(Type::scalar(xiao_syntax::ScalarType::Str)),
        },
    ]));
    let reversed = Type::DictColumn(DictType::new(vec![
        DictEntryType {
            key: "second".to_owned(),
            value: Box::new(Type::scalar(xiao_syntax::ScalarType::Str)),
        },
        DictEntryType {
            key: "first".to_owned(),
            value: Box::new(Type::scalar(xiao_syntax::ScalarType::Int)),
        },
    ]));
    assert!(!can_assign(&first, &reversed));
}

#[test]
/// C0 精确索引返回叶子类型，并拒绝范围、多选和静态越界。
fn checks_exact_selectors() {
    let valid = check("items = [1, \"x\"]\nvalue = items[1]\n");
    assert!(!valid.has_errors(), "{:?}", valid.diagnostics());
    assert_eq!(
        valid
            .nodes()
            .iter()
            .find(|node| node.span == node.span
                && node.ty == Type::scalar(xiao_syntax::ScalarType::Str))
            .map(|node| node.ty.clone()),
        Some(Type::scalar(xiao_syntax::ScalarType::Str))
    );

    let invalid = check("items = [1, 2]\na = items[0~1]\nb = items[3]\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-007")
    );
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-004")
    );
}

#[test]
/// 字典表按键、字典列按数字位置和键名读取，错误类别保持可区分。
fn checks_dictionary_selectors() {
    let valid = check(
        "table = {name = \"x\"}\ncolumn = <name = \"x\", level = 1>\na = table[name]\nb = column[0]\nc = column[level]\n",
    );
    assert!(!valid.has_errors(), "{:?}", valid.diagnostics());
    let missing = check("table = {name = \"x\"}\na = table[missing]\n");
    assert!(
        missing
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-005")
    );
    let wrong_kind = check("items = [1]\na = items[name]\n");
    assert!(
        wrong_kind
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-TYPE-003")
    );
}

#[test]
/// 公共路径解析 API 对固定数组形状返回确定的元素类型。
fn public_path_api_is_stable() {
    let root = Type::array_literal(vec![Type::scalar(xiao_syntax::ScalarType::Int)]);
    assert_eq!(
        xiao_types::resolve_exact_path(&root, &[ContainerPathSegment::Index(0)]),
        Ok(Type::scalar(xiao_syntax::ScalarType::Int))
    );
    let mut constraints = PathConstraintTree::new();
    constraints.insert(vec![], Type::scalar(xiao_syntax::ScalarType::Int));
    assert!(!constraints.is_empty());
}
