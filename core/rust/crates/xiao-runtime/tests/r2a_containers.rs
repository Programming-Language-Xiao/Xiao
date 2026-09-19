//! 09R2 批次 2 容器运行时对象规格。
//!
//! 重点是**引用计数真的平衡**，而不是「没报错」：容器持有元素会让句柄计数上升，
//! 容器释放必须把每个元素逐个放开。

use xiao_runtime::{
    ArrayHandle, CONTAINER_HASHABILITY_CODE, DictHandle, DictKind, RuntimeValue, SetHandle,
    StringHandle, TupleHandle,
};

/// 构造一个字符串值并返回它与底层句柄，便于断言引用计数。
fn text_value(content: &str) -> (RuntimeValue, StringHandle) {
    let handle = StringHandle::new(content).expect("字符串应分配");
    let value = RuntimeValue::Str(handle.clone());
    (value, handle)
}

#[test]
/// 数组持有元素会让计数上升，数组释放后必须回到原值。
fn array_releases_every_element() {
    let (element, handle) = text_value("x");
    assert_eq!(handle.strong_count(), 2, "句柄 + 值各持一次");

    let array = ArrayHandle::new(vec![element.clone(), element.clone()]).expect("数组应分配");
    assert_eq!(handle.strong_count(), 4, "数组持有两个元素副本");
    assert_eq!(array.len(), 2);

    drop(element);
    assert_eq!(handle.strong_count(), 3);

    array.try_release().expect("数组应释放");
    assert_eq!(handle.strong_count(), 1, "元素必须被逐个放开");
}

#[test]
/// 元组与数组一样逐项持有、逐项释放。
fn tuple_releases_every_element() {
    let (element, handle) = text_value("y");
    let tuple = TupleHandle::new(vec![element.clone()]).expect("元组应分配");
    assert_eq!(handle.strong_count(), 3);
    drop(element);
    tuple.try_release().expect("元组应释放");
    assert_eq!(handle.strong_count(), 1);
}

#[test]
/// 字典按键读取，值副本会持有一份引用，字典释放后放开。
fn dict_holds_and_releases_values() {
    let (element, handle) = text_value("z");
    let dict = DictHandle::new(DictKind::Column, vec![("key".to_owned(), element.clone())])
        .expect("字典应分配");
    assert_eq!(handle.strong_count(), 3);
    assert_eq!(dict.kind(), DictKind::Column);
    let read = dict.value("key").expect("应可读取");
    assert_eq!(read, Some(element.clone()));
    assert_eq!(handle.strong_count(), 4, "读取返回副本再持一次");
    drop(read);
    assert_eq!(dict.value("missing").expect("缺失键返回空"), None);
    assert!(!dict.contains_key("missing").expect("应可查询"));
    drop(element);
    dict.try_release().expect("字典应释放");
    assert_eq!(handle.strong_count(), 1);
}

#[test]
/// 集合按相等去重，被丢弃的重复元素必须立刻放开引用。
fn set_deduplicates_and_releases_dropped_elements() {
    let (element, handle) = text_value("same");
    let set = SetHandle::new(vec![element.clone(), element.clone(), element.clone()])
        .expect("集合应分配");
    assert_eq!(set.len(), 1, "内容相同的元素去重为一个");
    assert!(set.contains(&element).expect("应可查询"));
    // 三个副本入参 → 计数 5；去重后只保留一个，另外两个必须已放开。
    assert_eq!(handle.strong_count(), 3);
    drop(element);
    set.try_release().expect("集合应释放");
    assert_eq!(handle.strong_count(), 1);
}

#[test]
/// 不可哈希元素进入集合被拒绝，并使用稳定错误身份。
fn set_rejects_unhashable_elements() {
    let nested = ArrayHandle::new(vec![RuntimeValue::Int(1)]).expect("数组应分配");
    let error = SetHandle::new(vec![RuntimeValue::Array(nested)]).expect_err("应拒绝数组元素");
    assert_eq!(error.code(), CONTAINER_HASHABILITY_CODE);
}

#[test]
/// 容器类型名必须各自稳定，不能退化成 `dynamic`。
fn container_type_names_are_stable() {
    let array = RuntimeValue::Array(ArrayHandle::new(Vec::new()).expect("数组应分配"));
    let tuple = RuntimeValue::Tuple(TupleHandle::new(Vec::new()).expect("元组应分配"));
    let table =
        RuntimeValue::DictTable(DictHandle::new(DictKind::Table, Vec::new()).expect("字典应分配"));
    let column = RuntimeValue::DictColumn(
        DictHandle::new(DictKind::Column, Vec::new()).expect("字典应分配"),
    );
    let set = RuntimeValue::Set(SetHandle::new(Vec::new()).expect("集合应分配"));
    assert_eq!(array.type_name(), "array");
    assert_eq!(tuple.type_name(), "tuple");
    assert_eq!(table.type_name(), "dict_table");
    assert_eq!(column.type_name(), "dict_column");
    assert_eq!(set.type_name(), "set");
}

#[test]
/// 容器按对象身份相等，且相等性与哈希自洽（相等的值哈希相同）。
fn containers_compare_by_identity() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let hash_of = |value: &RuntimeValue| {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    };

    let first = RuntimeValue::Array(ArrayHandle::new(vec![RuntimeValue::Int(1)]).expect("分配"));
    let second = RuntimeValue::Array(ArrayHandle::new(vec![RuntimeValue::Int(1)]).expect("分配"));
    let clone = first.clone();
    assert_eq!(first, clone, "同一对象头相等");
    assert_ne!(first, second, "不同对象头不相等");
    assert_eq!(hash_of(&first), hash_of(&clone), "相等必须同哈希");
    assert_eq!(first, first, "相等必须自反");
}

#[test]
/// 标量的相等与哈希同口径：浮点走位模式，`-0.0` 与 `0.0` 不相等。
fn scalar_hash_matches_equality() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let hash_of = |value: &RuntimeValue| {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    };

    let zero = RuntimeValue::Float(0.0);
    let negative_zero = RuntimeValue::Float(-0.0);
    assert_ne!(zero, negative_zero);
    assert_ne!(hash_of(&zero), hash_of(&negative_zero));
    assert_eq!(hash_of(&zero), hash_of(&zero.clone()));
    let nan = RuntimeValue::Float(f64::NAN);
    assert_eq!(nan, nan.clone(), "位模式相等使 NaN 自反");
}

/// 由整数构造集合，供代数与比较用例复用。
fn int_set(values: &[i64]) -> SetHandle {
    SetHandle::new(
        values
            .iter()
            .map(|value| RuntimeValue::Int(*value))
            .collect(),
    )
    .expect("集合应分配")
}

/// 取出集合元素并转成整数序列，供顺序断言使用。
fn int_elements(set: &SetHandle) -> Vec<i64> {
    set.with_elements(|elements| {
        elements
            .iter()
            .map(|value| match value {
                RuntimeValue::Int(value) => *value,
                other => panic!("应为整数元素，实际为 {other:?}"),
            })
            .collect()
    })
    .expect("应可读取元素")
}

#[test]
/// 四种集合代数按操作数顺序产出结果，不依赖哈希、不依赖排序。
fn set_algebra_follows_operand_order() {
    let left = int_set(&[1, 2, 3]);
    let right = int_set(&[3, 4, 2]);

    assert_eq!(
        int_elements(&left.union(&right).expect("并集")),
        vec![1, 2, 3, 4],
        "并集是左侧原序，随后右侧中不在左侧者按右侧原序"
    );
    assert_eq!(
        int_elements(&left.intersection(&right).expect("交集")),
        vec![2, 3],
        "交集按左侧原序过滤"
    );
    assert_eq!(
        int_elements(&left.difference(&right).expect("差集")),
        vec![1],
        "差集按左侧原序过滤"
    );
    assert_eq!(
        int_elements(&left.symmetric_difference(&right).expect("对称差")),
        vec![1, 4],
        "对称差先左侧独有，再右侧独有"
    );
}

#[test]
/// 代数结果沿用构造期去重：重复元素不得在结果里重现。
fn set_algebra_results_stay_deduplicated() {
    let left = int_set(&[1, 1, 2]);
    let right = int_set(&[2, 2, 3]);
    let union = left.union(&right).expect("并集");
    assert_eq!(union.len(), 3);
    assert_eq!(int_elements(&union), vec![1, 2, 3]);
}

#[test]
/// 集合相等是**无序双向包含**，既不是句柄身份，也不是元素序列逐位相等。
///
/// 反例的锚点：物理表示是有序 `Vec`，逐位比较会把 `{1, 2}` 与 `{2, 1}` 判成不等；
/// 两者是不同对象，身份比较也会判成不等。两条错法都会让这里失败。
fn set_equality_is_unordered_mutual_inclusion() {
    let left = int_set(&[1, 2]);
    let same = int_set(&[2, 1]);
    assert!(!left.same_object(&same), "前提：两者是不同对象");
    assert!(left.equals(&same).expect("相等判定"));
    assert!(!left.equals(&int_set(&[1, 2, 3])).expect("相等判定"));
    assert!(!left.equals(&int_set(&[1])).expect("相等判定"));
}

#[test]
/// 子集与真子集必须分开：真子集要求包含**且**不相等。
///
/// 两者若共用同一个判断，`{1, 2} ⊂ {1, 2}` 会被判成真。
fn set_subset_distinguishes_proper_from_equal() {
    let small = int_set(&[1, 2]);
    let same = int_set(&[2, 1]);
    let big = int_set(&[1, 2, 3]);

    assert!(small.is_subset(&big).expect("子集"));
    assert!(small.is_proper_subset(&big).expect("真子集"));
    assert!(small.is_subset(&same).expect("相等也是子集"));
    assert!(!small.is_proper_subset(&same).expect("真子集"));
    assert!(big.is_superset(&small).expect("超集"));
    assert!(big.is_proper_superset(&small).expect("真超集"));
    assert!(same.is_superset(&small).expect("相等也是超集"));
    assert!(!same.is_proper_superset(&small).expect("真超集"));
}

#[test]
/// 空集是任何集合的子集与真子集，但不是自己的真子集；方向不能反。
fn set_empty_is_subset_of_everything_but_not_proper_of_itself() {
    let empty = int_set(&[]);
    let one = int_set(&[1]);

    assert!(empty.is_subset(&one).expect("子集"));
    assert!(empty.is_proper_subset(&one).expect("真子集"));
    assert!(!one.is_subset(&empty).expect("子集"), "方向不得反");
    assert!(empty.is_subset(&empty).expect("子集"));
    assert!(!empty.is_proper_subset(&empty).expect("真子集"));

    // 「空集」与「相减得到的空集」必须是同一种值。
    let subtracted = one.difference(&one).expect("差集");
    assert_eq!(subtracted.len(), 0);
    assert!(empty.equals(&subtracted).expect("相等判定"));
}

#[test]
/// 异构集合按判别式隔离：`1`、`true`、`1.0` 与 `Sint(1)` 是四个不同元素。
fn heterogeneous_set_members_are_discriminated_by_variant() {
    let mixed = SetHandle::new(vec![
        RuntimeValue::Int(1),
        RuntimeValue::Bool(true),
        RuntimeValue::Float(1.0),
        RuntimeValue::Sint(1),
    ])
    .expect("集合应分配");
    assert_eq!(mixed.len(), 4, "判别式不同的值不得互相去重");

    let ints = int_set(&[1]);
    let bools = SetHandle::new(vec![RuntimeValue::Bool(true)]).expect("集合应分配");
    assert!(
        !ints.equals(&bools).expect("相等判定"),
        "1 与 true 不是同一元素"
    );
    assert_eq!(
        int_elements(&mixed.intersection(&ints).expect("交集")),
        vec![1],
        "交集只应命中 Int(1)"
    );
}
