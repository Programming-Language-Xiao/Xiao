//! C1 有序容器选择、随机种子和标量广播的规格测试。

use xiao_source::SourceFile;
use xiao_syntax::{Parser, Program, ScalarType};
use xiao_types::{
    ArrayType, RandomSeedPlan, RuntimeCheckKind, SelectionPathSegment, Type, TypeChecker,
};
use xiao_types::{
    RANDOM_SEED_CODE, SELECTOR_ASSIGNMENT_CODE, SELECTOR_INVALID_RANDOM_COUNT_CODE,
    SELECTOR_INVALID_STEP_CODE, SELECTOR_RANDOM_EXHAUSTED_CODE, SELECTOR_UNORDERED_CONTAINER_CODE,
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
/// 多选、闭区间和单边范围应保留来源根容器及源码顺序。
fn checks_ordered_ranges_and_multiple_selection() {
    let result = check(
        "items = [1, \"x\", true, 4]\nselected = items[2, 0]\nranged = items[1~3]\nhead = items[<2]\nall = items[=]\n",
    );
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert!(matches!(
        result.binding("selected").expect("selected").scheme.ty,
        Type::Array(ArrayType::Heterogeneous { ref elements }) if elements.len() == 2
            && elements[0] == Type::scalar(ScalarType::Bool)
            && elements[1] == Type::scalar(ScalarType::Int)
    ));
    assert!(matches!(
        result.binding("ranged").expect("ranged").scheme.ty,
        Type::Array(ArrayType::Heterogeneous { ref elements }) if elements.len() == 3
    ));
    assert!(
        matches!(
            result.binding("head").expect("head").scheme.ty,
            Type::Array(ArrayType::Heterogeneous { ref elements }) if elements.len() == 2
        ),
        "head type: {:?}",
        result.binding("head").expect("head").scheme.ty
    );
    assert!(matches!(
        result.binding("all").expect("all").scheme.ty,
        Type::Array(ArrayType::Heterogeneous { ref elements }) if elements.len() == 4
    ));
}

#[test]
/// 单边范围的边界包含规则和正负步长必须稳定。
fn checks_open_ranges_and_steps() {
    let result = check(
        "items = [0, 1, 2, 3, 4]\nempty = items[<0]\ninclusive = items[<=2]\nreverse = items{ -2 }[0~4]\nmultiple = items{2}[0~4, 1~3]\n",
    );
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert!(matches!(
        result.binding("empty").expect("empty").scheme.ty,
        Type::Array(ArrayType::Heterogeneous { ref elements }) if elements.is_empty()
    ));
    assert!(matches!(
        result.binding("inclusive").expect("inclusive").scheme.ty,
        Type::Array(ArrayType::Heterogeneous { ref elements }) if elements.len() == 3
    ));
    assert_eq!(result.selection_plans()[2].selected_paths.len(), 3);
    assert_eq!(result.selection_plans()[3].selected_paths.len(), 5);
}

#[test]
/// 负索引按 Python 规则规范化，字符串索引保留 Runtime 边界检查。
fn supports_negative_and_string_indices() {
    let result = check("items = [1, 2, 3]\nlast = items[-1]\ntext = \"你好\"\nchar = text[-1]\n");
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert_eq!(
        result.binding("last").expect("last").scheme.ty,
        Type::scalar(ScalarType::Int)
    );
    assert_eq!(
        result.binding("char").expect("char").scheme.ty,
        Type::scalar(ScalarType::Str)
    );
    assert!(
        result
            .runtime_checks()
            .iter()
            .any(|check| check.kind == RuntimeCheckKind::SelectorBounds)
    );
}

#[test]
/// 零步长、随机负数和无放回超量必须产生稳定诊断。
fn rejects_invalid_step_and_random_counts() {
    let result = check(
        "items = [1, 2]\nbad_step = items{0}[=]\nbad_count = items[?-1]\ntoo_many = items[?3]\n",
    );
    assert!(result.has_errors());
    let codes = result
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&SELECTOR_INVALID_STEP_CODE));
    assert!(codes.contains(&SELECTOR_INVALID_RANDOM_COUNT_CODE));
    assert!(codes.contains(&SELECTOR_RANDOM_EXHAUSTED_CODE));
}

#[test]
/// 放回随机计划允许超量；测试随机源应能按同一道路复现结果。
fn records_random_modes_and_seed() {
    let result = check(
        "items = [1, 2]\nrandom.seed(42)\nwithout = items[?2]\nwith = items[!?5]\nzero = items[?0]\n",
    );
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert_eq!(result.random_seed_plans().len(), 1);
    assert_eq!(
        result.random_seed_plans()[0],
        RandomSeedPlan {
            span: result.random_seed_plans()[0].span,
            value: Some(42),
            dynamic: false,
        }
    );
    assert!(matches!(
        result.binding("zero").expect("zero").scheme.ty,
        Type::Array(ArrayType::Heterogeneous { ref elements }) if elements.is_empty()
    ));
    assert!(
        result
            .selection_plans()
            .iter()
            .any(|plan| plan.with_replacement)
    );
}

#[test]
/// 字典列放回重复键时，结果类型必须转为保序元组。
fn dictionary_column_repetition_becomes_tuple() {
    let result = check("column = <name = \"x\", level = 1>\nvalue = column[!?3]\n");
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert!(matches!(
        result.binding("value").expect("value").scheme.ty,
        Type::Tuple(_)
    ));
}

#[test]
/// 字典表只能进行单个精确键读取，高级选择必须静态拒绝。
fn rejects_advanced_dictionary_table_selection() {
    let result = check("table = {name = \"x\", level = 1}\nvalue = table[0, 1]\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SELECTOR_UNORDERED_CONTAINER_CODE)
    );
}

#[test]
/// `random.seed` 只接受非负整数，负数和浮点值不得静默转换。
fn rejects_invalid_random_seed() {
    let negative = check("random.seed(-1)\n");
    assert!(
        negative
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == RANDOM_SEED_CODE)
    );
    let float = check("random.seed(1.5)\n");
    assert!(
        float
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == RANDOM_SEED_CODE)
    );
}

#[test]
/// 选择器左值只接受标量广播，并生成事务性写入计划。
fn checks_scalar_broadcast_assignment() {
    let valid = check("items = [1, 2, 3]\nitems[0, 2] = 0\n");
    assert!(!valid.has_errors(), "{:?}", valid.diagnostics());
    assert_eq!(valid.broadcast_assignment_plans().len(), 1);
    assert!(valid.broadcast_assignment_plans()[0].transactional);
    assert_eq!(valid.broadcast_assignment_plans()[0].target_paths.len(), 2);

    let invalid = check("items = [1, 2]\nitems[0, 1] = [3, 4]\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SELECTOR_ASSIGNMENT_CODE)
    );
    assert!(invalid.broadcast_assignment_plans().is_empty());
}

#[test]
/// 动态步长和动态随机数量只登记 Runtime 检查，不被静态阶段误报。
fn defers_dynamic_selector_operands() {
    let result = check("items = [1, 2, 3]\nint step\nint count\npicked = items{step}[?count]\n");
    assert!(result.has_errors());
    assert!(
        result
            .runtime_checks()
            .iter()
            .any(|check| check.kind == RuntimeCheckKind::SelectorStep)
    );
    assert!(
        result
            .runtime_checks()
            .iter()
            .any(|check| check.kind == RuntimeCheckKind::RandomCount)
    );
}

#[test]
/// 选择器路径段应保留原始索引和静态规范化位置。
fn exposes_normalized_selection_paths() {
    let result = check("items = [1, 2, 3]\nvalue = items[-1]\n");
    let path = &result.selection_plans()[0].selected_paths[0];
    assert!(matches!(
        path[0],
        SelectionPathSegment::Index {
            raw: -1,
            resolved: Some(2)
        }
    ));
}

#[test]
/// 跨嵌套闭区间应保留边界分支并在类型投影时合并同一父容器。
fn expands_nested_ranges_and_projects_shape() {
    let result = check(
        "items = (\"a\", [9, 8], [1, 2])\nexact = items[1, 2/1]\nranged = items[1~2/1]\nprefix = items[<2]\n",
    );
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert_eq!(
        result.binding("exact").expect("exact").scheme.ty,
        Type::Tuple(vec![
            Type::Array(ArrayType::heterogeneous(vec![
                Type::scalar(ScalarType::Int),
                Type::scalar(ScalarType::Int),
            ])),
            Type::Array(ArrayType::heterogeneous(vec![Type::scalar(
                ScalarType::Int
            )])),
        ])
    );
    assert_eq!(
        result.binding("ranged").expect("ranged").scheme.ty,
        Type::Tuple(vec![
            Type::Array(ArrayType::heterogeneous(vec![
                Type::scalar(ScalarType::Int),
                Type::scalar(ScalarType::Int),
            ])),
            Type::Array(ArrayType::heterogeneous(vec![
                Type::scalar(ScalarType::Int),
                Type::scalar(ScalarType::Int),
            ])),
        ])
    );
    assert_eq!(
        result.binding("prefix").expect("prefix").scheme.ty,
        Type::Tuple(vec![
            Type::scalar(ScalarType::Str),
            Type::Array(ArrayType::heterogeneous(vec![
                Type::scalar(ScalarType::Int),
                Type::scalar(ScalarType::Int),
            ])),
        ])
    );
    assert_eq!(result.selection_plans()[1].selected_paths.len(), 3);
}

#[test]
/// 静态无放回随机选择的结果不能错误地变成空容器。
fn random_without_replacement_keeps_source_shape() {
    let result = check("items = [1, 2, 3]\npicked = items[?2]\n");
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert_eq!(
        result.binding("picked").expect("picked").scheme.ty,
        Type::Array(ArrayType::heterogeneous(vec![
            Type::scalar(ScalarType::Int),
            Type::scalar(ScalarType::Int),
            Type::scalar(ScalarType::Int),
        ]))
    );
}

#[test]
/// 字典列同一条目被数字和键名重复选中时转为元组。
fn dictionary_column_nonrandom_repetition_becomes_tuple() {
    let result = check("column = <name = \"x\", level = 1>\nvalue = column[0, name]\n");
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert!(matches!(
        &result.binding("value").expect("value").scheme.ty,
        Type::Tuple(items) if items.len() == 2
    ));
}

#[test]
/// 静态目标越界时不得生成广播计划或提前标记根绑定已写入。
fn invalid_broadcast_target_is_transactional() {
    let result = check("items = [1]\nitems[3] = 0\n");
    assert!(result.has_errors());
    assert!(result.broadcast_assignment_plans().is_empty());
    assert!(result.binding("items").expect("items").initialized);
}

#[test]
/// 范围端点穿过无序字典表时必须拒绝，而不是依赖存储顺序。
fn rejects_range_through_dictionary_table() {
    let result =
        check("items = [{nested = <a = 1, b = 2>}, 1]\nvalue = items[0/nested/0~0/nested/1]\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SELECTOR_UNORDERED_CONTAINER_CODE)
    );
}

#[test]
/// 排他下界位于嵌套容器节点时，不能把该节点的后代误选进结果。
fn excludes_descendants_of_exclusive_lower_boundary() {
    let result = check("items = [[1, 2], [3, 4]]\nvalue = items[>0]\n");
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert_eq!(
        result.binding("value").expect("value").scheme.ty,
        Type::Array(ArrayType::heterogeneous(vec![Type::Array(
            ArrayType::heterogeneous(vec![
                Type::scalar(ScalarType::Int),
                Type::scalar(ScalarType::Int),
            ]),
        )]))
    );
}

#[test]
/// 超出 usize 的静态步长应退化为只取首项，而不能发生整数截断。
fn saturates_extremely_large_step() {
    let result =
        check("items = [0, 1, 2]\nvalue = items{170141183460469231731687303715884105727}[=]\n");
    assert!(!result.has_errors(), "{:?}", result.diagnostics());
    assert_eq!(
        result.selection_plans()[0].selected_paths.len(),
        1,
        "巨大步长不应截断成较小的步长"
    );
}
