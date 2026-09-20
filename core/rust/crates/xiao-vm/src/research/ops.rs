//! 值运算的薄包装。
//!
//! 语义核经这里调用 Runtime 的算子表，不散落裸 `RuntimeValue` 调用。这样算子
//! 矩阵只有一处引用点：将来 Runtime 换实现、或某个机型需要不同的快速路径，
//! 都只改这一个文件。这里不做任何隐式宽度提升——那由后端在降低时插入显式转换。

use xiao_bytecode::research::{ArithOp, CompareOp, PathStep, SetCompareOp, SetOpKind};
use xiao_ir::{
    IrSelectionItemPlan, IrSelectionPath, IrSelectionPathSegment, IrSelectionPlan, IrType,
};
use xiao_runtime::{
    ArrayHandle, DictHandle, DictKind, RuntimeError, RuntimeResult, RuntimeValue, SetHandle,
    TupleHandle, is_hashable,
};
use xiao_syntax::RandomMode;
use xiao_types::{RandomSelectionError, RandomSource, sample_indices};

/// 执行一条三地址算术指令。
pub fn apply_arith(
    op: ArithOp,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    match op {
        ArithOp::Add => left.add(right),
        ArithOp::Subtract => left.subtract(right),
        ArithOp::Multiply => left.multiply(right),
        ArithOp::Divide => left.divide(right),
        ArithOp::FloorDivide => left.floor_divide(right),
        ArithOp::Remainder => left.remainder(right),
        ArithOp::Power => left.power(right),
    }
}

/// 取出两个集合操作数；任一不是集合就返回它的类型名供调用方构造稳定错误。
///
/// 走到这里说明静态检查与运行时检查都没拦住——正常路径上动态边界会先被
/// `Check` 指令拒绝，所以这是兜底而不是主路径。
fn set_operands(
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> Result<(SetHandle, SetHandle), String> {
    match (left, right) {
        (RuntimeValue::Set(left_set), RuntimeValue::Set(right_set)) => {
            Ok((left_set.clone(), right_set.clone()))
        }
        (RuntimeValue::Set(_), other) | (other, _) => Err(other.type_name().to_owned()),
    }
}

/// 执行集合代数运算，结果仍是集合。
pub fn apply_set_op(
    op: SetOpKind,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    let (left_set, right_set) =
        set_operands(left, right).map_err(RuntimeError::set_operation_requires_sets)?;
    let result = match op {
        SetOpKind::Union => left_set.union(&right_set)?,
        SetOpKind::Intersection => left_set.intersection(&right_set)?,
        SetOpKind::Difference => left_set.difference(&right_set)?,
        SetOpKind::SymmetricDifference => left_set.symmetric_difference(&right_set)?,
    };
    Ok(RuntimeValue::Set(result))
}

/// 执行集合关系比较，结果恒为布尔。
pub fn apply_set_compare(
    op: SetCompareOp,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    if matches!(op, SetCompareOp::Member | SetCompareOp::NotMember) {
        let RuntimeValue::Set(right_set) = right else {
            return Err(RuntimeError::set_membership_requires_hashable(
                right.type_name(),
            ));
        };
        if !is_hashable(left) {
            return Err(RuntimeError::set_membership_requires_hashable(
                left.type_name(),
            ));
        }
        let member = right_set.contains(left)?;
        return Ok(RuntimeValue::Bool(if op == SetCompareOp::Member {
            member
        } else {
            !member
        }));
    }
    let (left_set, right_set) =
        set_operands(left, right).map_err(RuntimeError::set_comparison_requires_sets)?;
    let value = match op {
        SetCompareOp::Equal => left_set.equals(&right_set)?,
        SetCompareOp::NotEqual => !left_set.equals(&right_set)?,
        SetCompareOp::ProperSubset => left_set.is_proper_subset(&right_set)?,
        SetCompareOp::Subset => left_set.is_subset(&right_set)?,
        SetCompareOp::ProperSuperset => left_set.is_proper_superset(&right_set)?,
        SetCompareOp::Superset => left_set.is_superset(&right_set)?,
        SetCompareOp::Member | SetCompareOp::NotMember => unreachable!("成员分支已提前返回"),
    };
    Ok(RuntimeValue::Bool(value))
}

/// 构造数组。
pub fn new_array(elements: Vec<RuntimeValue>) -> RuntimeResult<RuntimeValue> {
    Ok(RuntimeValue::Array(ArrayHandle::new(elements)?))
}

/// 构造元组。
pub fn new_tuple(elements: Vec<RuntimeValue>) -> RuntimeResult<RuntimeValue> {
    Ok(RuntimeValue::Tuple(TupleHandle::new(elements)?))
}

/// 构造无序字典表。
pub fn new_dict_table(entries: Vec<(String, RuntimeValue)>) -> RuntimeResult<RuntimeValue> {
    Ok(RuntimeValue::DictTable(DictHandle::new(
        DictKind::Table,
        entries,
    )?))
}

/// 构造字典列。
pub fn new_dict_column(entries: Vec<(String, RuntimeValue)>) -> RuntimeResult<RuntimeValue> {
    Ok(RuntimeValue::DictColumn(DictHandle::new(
        DictKind::Column,
        entries,
    )?))
}

/// 构造集合；不可哈希元素由 Runtime 拒绝。
pub fn new_set(elements: Vec<RuntimeValue>) -> RuntimeResult<RuntimeValue> {
    Ok(RuntimeValue::Set(SetHandle::new(elements)?))
}

/// 返回可迭代值的逻辑长度。
///
/// 字符串长度按 Unicode 标量（码点）计算，与 `StringHandle::len` 和统一索引
/// 语义一致；集合沿用 `SetHandle` 的物理顺序契约，但这里只读取元素数量。
pub fn len(source: &RuntimeValue) -> RuntimeResult<RuntimeValue> {
    let length = match source {
        RuntimeValue::Array(handle) => handle.len(),
        RuntimeValue::Tuple(handle) => handle.len(),
        RuntimeValue::Str(handle) => handle.len(),
        RuntimeValue::Set(handle) => handle.len(),
        RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle) => handle.len(),
        _ => {
            return Err(RuntimeError::type_mismatch(
                "可迭代容器",
                source.type_name(),
            ));
        }
    };
    let length = i64::try_from(length)
        .map_err(|_| RuntimeError::numeric_overflow("可迭代值长度超出 int 范围"))?;
    Ok(RuntimeValue::Int(length))
}

/// 按运行时整数索引读取一个可迭代值的元素。
///
/// 动态索引先还原为 [`PathStep::Index`]，再复用内部 `index_step`，确保数组、元组、
/// 字符串和字典列的负索引及边界行为与静态 `IndexGet` 完全一致。
pub fn index_get_dynamic(
    source: &RuntimeValue,
    index: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    let raw = integer_index(index)?;
    index_step(source, &PathStep::Index(raw))
}

/// 从 Runtime 值读取动态索引整数。
fn integer_index(value: &RuntimeValue) -> RuntimeResult<i128> {
    match value {
        RuntimeValue::Int(value) => Ok(i128::from(*value)),
        RuntimeValue::Sint(value) => Ok(i128::from(*value)),
        RuntimeValue::Lint(value) => value
            .parse::<i128>()
            .map_err(|_| RuntimeError::type_mismatch("int", "lint")),
        _ => Err(RuntimeError::type_mismatch("int", value.type_name())),
    }
}

/// 按精确路径读取容器元素。
///
/// 负索引一律经 [`xiao_types::normalize_index`] 归一化——与类型检查器共用同一
/// 套语义，不在这里重写一份。越界与键缺失使用稳定错误身份。
pub fn index_get(source: &RuntimeValue, path: &[PathStep]) -> RuntimeResult<RuntimeValue> {
    if path.is_empty() {
        return Err(RuntimeError::invalid_value("精确索引路径不能为空"));
    }
    let mut current = source.clone();
    for step in path {
        current = index_step(&current, step)?;
    }
    Ok(current)
}

/// 按一个路径段读取值；高级选择与精确索引共用此读取规则。
fn index_step(source: &RuntimeValue, step: &PathStep) -> RuntimeResult<RuntimeValue> {
    match (source, step) {
        (RuntimeValue::Array(handle), PathStep::Index(raw)) => {
            let index = resolve_index(*raw, handle.len(), "array")?;
            handle.element(index)?.ok_or_else(missing_element)
        }
        (RuntimeValue::Tuple(handle), PathStep::Index(raw)) => {
            let index = resolve_index(*raw, handle.len(), "tuple")?;
            handle.element(index)?.ok_or_else(missing_element)
        }
        (RuntimeValue::Str(handle), PathStep::Index(raw)) => {
            let length = handle.len();
            let index = resolve_index(*raw, length, "str")?;
            let text = handle.with_str(|text| text.chars().nth(index))?;
            match text {
                Some(character) => RuntimeValue::new_string(character.to_string()),
                None => Err(missing_element()),
            }
        }
        (RuntimeValue::Set(handle), PathStep::Index(raw)) => {
            let index = resolve_index(*raw, handle.len(), "set")?;
            handle
                .with_elements(|elements| elements.get(index).cloned())?
                .ok_or_else(missing_element)
        }
        (
            RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle),
            PathStep::Key(key),
        ) => handle
            .value(key)?
            .ok_or_else(|| RuntimeError::key_not_found(handle.kind().as_str(), key.clone())),
        (
            RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle),
            PathStep::Index(raw),
        ) => {
            let length = handle.len();
            let index = resolve_index(*raw, length, handle.kind().as_str())?;
            let entry = handle
                .with_entries(|entries| entries.get(index).map(|(_, value)| value.clone()))?;
            entry.ok_or_else(missing_element)
        }
        _ => Err(RuntimeError::type_mismatch(
            "可精确索引的容器",
            source.type_name(),
        )),
    }
}

/// 执行一条高级选择计划。
///
/// 计划中的静态路径已经由类型阶段展开；只有标记为动态的边界、步长和随机
/// 数量在这里读取寄存器。结果形状完全由 `result_type` 决定，运行时值变体只
/// 负责提供元素，不参与形状推断。
pub fn selector_apply<R: RandomSource>(
    source: &RuntimeValue,
    plan: &IrSelectionPlan,
    dynamic_step: Option<&RuntimeValue>,
    dynamic_counts: &[Option<RuntimeValue>],
    random: &mut R,
) -> RuntimeResult<RuntimeValue> {
    let step = selector_step(plan, dynamic_step)?;
    // 随机项即使数量是静态值也没有 `selected_paths`：候选位置和抽样顺序
    // 必须在执行期经注入的随机源产生。只有完全没有随机项且所有边界均已
    // 静态展开时，才能直接消费计划里的路径快照。
    let has_runtime_items = plan.requires_runtime_check
        || plan
            .items
            .iter()
            .any(|item| matches!(item, IrSelectionItemPlan::Random { .. }));
    let paths = if !has_runtime_items && plan.step.as_ref().is_none_or(|item| !item.dynamic) {
        plan.selected_paths.clone()
    } else {
        expand_runtime_items(source, plan, step, dynamic_counts, random)?
    };
    let values = paths
        .iter()
        .map(|path| read_ir_path(source, path))
        .collect::<RuntimeResult<Vec<_>>>()?;
    build_selection_result(source, plan, &paths, &values)
}

/// 按事务性广播计划把一个标量写入全部目标。
///
/// 目标解析与写入分成两个阶段：第一阶段完整验证并锁定所有位置，任何失败
/// 都不会触碰容器；第二阶段才提交克隆后的标量值，因此不会留下半次修改。
pub fn broadcast_assign(
    root: &RuntimeValue,
    value: &RuntimeValue,
    plan: &xiao_ir::IrBroadcastAssignmentPlan,
) -> RuntimeResult<()> {
    if !plan.transactional {
        return Err(RuntimeError::selector_bounds("广播计划不是事务性的"));
    }
    if is_runtime_container(value) || matches!(value, RuntimeValue::None) {
        return Err(RuntimeError::type_mismatch("标量", value.type_name()));
    }
    let mut targets = Vec::with_capacity(plan.target_paths.len());
    for path in &plan.target_paths {
        targets.push(resolve_broadcast_target(root, path)?);
    }
    for target in targets {
        target.write(value.clone())?;
    }
    Ok(())
}

/// 先解析一条完整广播路径，再把最后一段固定为可写目标。
///
/// 中间段只读取句柄，不修改任何载荷；因此所有目标都解析成功后才会进入提交
/// 阶段，嵌套路径与直接路径遵守同一事务边界。
fn resolve_broadcast_target(
    root: &RuntimeValue,
    path: &IrSelectionPath,
) -> RuntimeResult<BroadcastTarget> {
    let Some((last, prefix)) = path.split_last() else {
        return Err(RuntimeError::selector_bounds("广播目标路径不能为空"));
    };
    let mut current = root.clone();
    for segment in prefix {
        current = read_broadcast_segment(&current, segment)?;
    }
    match (&current, last) {
        (RuntimeValue::Array(handle), IrSelectionPathSegment::Index { raw, .. }) => {
            let index = xiao_types::normalize_index(*raw, handle.len())
                .ok_or_else(|| RuntimeError::selector_bounds("广播目标索引越界"))?;
            Ok(BroadcastTarget::Array(handle.clone(), index))
        }
        (RuntimeValue::Tuple(handle), IrSelectionPathSegment::Index { raw, .. }) => {
            let index = xiao_types::normalize_index(*raw, handle.len())
                .ok_or_else(|| RuntimeError::selector_bounds("广播目标索引越界"))?;
            Ok(BroadcastTarget::Tuple(handle.clone(), index))
        }
        (RuntimeValue::DictTable(handle), IrSelectionPathSegment::Key(key))
        | (RuntimeValue::DictColumn(handle), IrSelectionPathSegment::Key(key)) => {
            if !handle.contains_key(key)? {
                return Err(RuntimeError::selector_bounds("广播目标键不存在"));
            }
            Ok(BroadcastTarget::DictKey(handle.clone(), key.clone()))
        }
        (RuntimeValue::DictColumn(handle), IrSelectionPathSegment::Index { raw, .. }) => {
            let index = xiao_types::normalize_index(*raw, handle.len())
                .ok_or_else(|| RuntimeError::selector_bounds("广播目标索引越界"))?;
            Ok(BroadcastTarget::DictIndex(handle.clone(), index))
        }
        (RuntimeValue::Set(_), _) => Err(RuntimeError::selector_bounds("集合不可索引")),
        _ => Err(RuntimeError::selector_bounds("广播目标路径与容器不匹配")),
    }
}

/// 读取广播路径的中间段；任意读失败都归一到选择器边界错误。
fn read_broadcast_segment(
    source: &RuntimeValue,
    segment: &IrSelectionPathSegment,
) -> RuntimeResult<RuntimeValue> {
    let step = match segment {
        IrSelectionPathSegment::Index { raw, .. } => PathStep::Index(*raw),
        IrSelectionPathSegment::Key(key) => PathStep::Key(key.clone()),
    };
    index_step(source, &step).map_err(|_| RuntimeError::selector_bounds("广播目标路径不可读取"))
}

/// 一个已完成验证的广播目标。
enum BroadcastTarget {
    /// 数组位置。
    Array(ArrayHandle, usize),
    /// 元组位置。
    Tuple(TupleHandle, usize),
    /// 字典键。
    DictKey(DictHandle, String),
    /// 字典列位置。
    DictIndex(DictHandle, usize),
}

impl BroadcastTarget {
    /// 提交一个目标写入。
    fn write(self, value: RuntimeValue) -> RuntimeResult<()> {
        match self {
            Self::Array(handle, index) => handle.with_elements_mut(|elements| {
                if let Some(slot) = elements.get_mut(index) {
                    *slot = value;
                    Ok(())
                } else {
                    Err(RuntimeError::selector_bounds("广播目标索引失效"))
                }
            })?,
            Self::Tuple(handle, index) => handle.with_elements_mut(|elements| {
                if let Some(slot) = elements.get_mut(index) {
                    *slot = value;
                    Ok(())
                } else {
                    Err(RuntimeError::selector_bounds("广播目标索引失效"))
                }
            })?,
            Self::DictKey(handle, key) => handle.with_entries_mut(|entries| {
                if let Some((_, slot)) = entries.iter_mut().find(|(name, _)| *name == key) {
                    *slot = value;
                    Ok(())
                } else {
                    Err(RuntimeError::selector_bounds("广播目标键失效"))
                }
            })?,
            Self::DictIndex(handle, index) => handle.with_entries_mut(|entries| {
                if let Some((_, slot)) = entries.get_mut(index) {
                    *slot = value;
                    Ok(())
                } else {
                    Err(RuntimeError::selector_bounds("广播目标索引失效"))
                }
            })?,
        }
    }
}

/// 读取计划步长；静态步长已在计划路径中生效，动态步长在运行时校验。
fn selector_step(
    plan: &IrSelectionPlan,
    dynamic_step: Option<&RuntimeValue>,
) -> RuntimeResult<i128> {
    let Some(step) = &plan.step else {
        return Ok(1);
    };
    let value = if step.dynamic {
        let Some(value) = dynamic_step.and_then(integer_value) else {
            return Err(RuntimeError::selector_step("动态步长不是整数"));
        };
        value
    } else {
        step.value.unwrap_or(1)
    };
    if value == 0 {
        return Err(RuntimeError::selector_step("选择器步长不能为 0"));
    }
    Ok(value)
}

/// 动态计划项的运行时展开。静态展开结果不经过这里，避免重复应用步长。
fn expand_runtime_items<R: RandomSource>(
    source: &RuntimeValue,
    plan: &IrSelectionPlan,
    step: i128,
    dynamic_counts: &[Option<RuntimeValue>],
    random: &mut R,
) -> RuntimeResult<Vec<IrSelectionPath>> {
    let mut output = Vec::new();
    for (item_index, item) in plan.items.iter().enumerate() {
        let mut item_paths = match item {
            IrSelectionItemPlan::Exact { path } => vec![resolve_runtime_path(source, path)?],
            IrSelectionItemPlan::All => direct_runtime_paths(source)?,
            IrSelectionItemPlan::Range {
                start,
                end,
                include_start,
                include_end,
            } => expand_runtime_range(source, start, end, *include_start, *include_end)?,
            IrSelectionItemPlan::Random {
                mode,
                count,
                dynamic_count,
            } => {
                let count = if *dynamic_count {
                    let value = dynamic_counts
                        .get(item_index)
                        .and_then(Option::as_ref)
                        .and_then(integer_value)
                        .ok_or_else(|| RuntimeError::random_count("动态随机数量不是整数"))?;
                    if value < 0 {
                        return Err(RuntimeError::random_count("随机抽取数量不能为负数"));
                    }
                    usize::try_from(value)
                        .map_err(|_| RuntimeError::random_count("随机抽取数量超出范围"))?
                } else {
                    count.unwrap_or(0)
                };
                let candidates = direct_runtime_paths(source)?;
                let mode = match mode.as_str() {
                    "without_replacement" => RandomMode::WithoutReplacement,
                    "with_replacement" => RandomMode::WithReplacement,
                    _ => return Err(RuntimeError::random_count("未知随机选择模式")),
                };
                let indices =
                    sample_indices(candidates.len(), count, mode, random).map_err(random_error)?;
                indices
                    .into_iter()
                    .map(|index| candidates[index].clone())
                    .collect()
            }
        };
        // 类型层只会对有静态路径的选择项应用步长；运行时展开动态范围
        // 时也必须复用同一“每个项目独立”规则。随机项没有类型层路径，
        // 因而保持抽样数量和顺序，不额外套步长。
        if plan.step.is_some() && !matches!(item, IrSelectionItemPlan::Random { .. }) {
            item_paths = apply_step(item_paths, step);
        }
        output.extend(item_paths);
    }
    Ok(output)
}

/// 对单个选择项应用正/负步长。
fn apply_step<T>(items: Vec<T>, step: i128) -> Vec<T> {
    let distance = usize::try_from(step.unsigned_abs()).unwrap_or(usize::MAX);
    if step > 0 {
        items.into_iter().step_by(distance.max(1)).collect()
    } else {
        items.into_iter().rev().step_by(distance.max(1)).collect()
    }
}

/// 枚举运行时来源的一层有序路径。
fn direct_runtime_paths(source: &RuntimeValue) -> RuntimeResult<Vec<IrSelectionPath>> {
    match source {
        RuntimeValue::Array(handle) => Ok((0..handle.len())
            .map(|index| {
                vec![IrSelectionPathSegment::Index {
                    raw: index as i128,
                    resolved: Some(index),
                }]
            })
            .collect()),
        RuntimeValue::Tuple(handle) => Ok((0..handle.len())
            .map(|index| {
                vec![IrSelectionPathSegment::Index {
                    raw: index as i128,
                    resolved: Some(index),
                }]
            })
            .collect()),
        RuntimeValue::DictColumn(handle) => Ok((0..handle.len())
            .map(|index| {
                vec![IrSelectionPathSegment::Index {
                    raw: index as i128,
                    resolved: Some(index),
                }]
            })
            .collect()),
        RuntimeValue::Str(handle) => Ok((0..handle.len())
            .map(|index| {
                vec![IrSelectionPathSegment::Index {
                    raw: index as i128,
                    resolved: Some(index),
                }]
            })
            .collect()),
        RuntimeValue::DictTable(_) => {
            Err(RuntimeError::selector_bounds("无序字典表不支持高级选择"))
        }
        RuntimeValue::Set(_) => Err(RuntimeError::selector_bounds("集合不可索引")),
        _ => Err(RuntimeError::type_mismatch("有序容器", source.type_name())),
    }
}

/// 解析一个计划路径中的动态索引。
fn resolve_runtime_path(
    source: &RuntimeValue,
    path: &IrSelectionPath,
) -> RuntimeResult<IrSelectionPath> {
    let mut current = source.clone();
    let mut resolved = Vec::with_capacity(path.len());
    for segment in path {
        match segment {
            IrSelectionPathSegment::Index { raw, .. } => {
                let length = match &current {
                    RuntimeValue::Array(handle) => handle.len(),
                    RuntimeValue::Tuple(handle) => handle.len(),
                    RuntimeValue::DictColumn(handle) => handle.len(),
                    RuntimeValue::Str(handle) => handle.len(),
                    RuntimeValue::Set(_) => {
                        return Err(RuntimeError::selector_bounds("集合不可索引"));
                    }
                    _ => {
                        return Err(RuntimeError::selector_bounds("路径穿过不可索引值"));
                    }
                };
                let index = xiao_types::normalize_index(*raw, length).ok_or_else(|| {
                    RuntimeError::selector_bounds(format!(
                        "选择器索引 {raw} 超出运行时长度 {length}"
                    ))
                })?;
                current = index_step(&current, &PathStep::Index(index as i128))
                    .map_err(|_| RuntimeError::selector_bounds("选择器路径不可读取"))?;
                resolved.push(IrSelectionPathSegment::Index {
                    raw: *raw,
                    resolved: Some(index),
                });
            }
            IrSelectionPathSegment::Key(key) => {
                current = index_step(&current, &PathStep::Key(key.clone()))
                    .map_err(|_| RuntimeError::selector_bounds("选择器键路径不可读取"))?;
                resolved.push(IrSelectionPathSegment::Key(key.clone()));
            }
        }
    }
    Ok(resolved)
}

/// 展开动态范围。范围按运行时有序节点的深度优先顺序工作；字典表和集合稳定失败。
fn expand_runtime_range(
    source: &RuntimeValue,
    start: &IrSelectionPath,
    end: &IrSelectionPath,
    include_start: bool,
    include_end: bool,
) -> RuntimeResult<Vec<IrSelectionPath>> {
    let start_boundary = resolve_range_boundary(source, start)?;
    let end_boundary = resolve_range_boundary(source, end)?;
    let mut nodes = Vec::new();
    enumerate_runtime_nodes(source, &[], &mut nodes)?;
    let start_order = start_boundary.as_ref().map(|(order, _)| order.as_slice());
    let end_order = end_boundary.as_ref().map(|(order, _)| order.as_slice());
    let descending = start_order
        .zip(end_order)
        .is_some_and(|(left, right)| left > right);
    let (lower, lower_inclusive, upper, upper_inclusive) = if descending {
        (end_order, include_end, start_order, include_start)
    } else {
        (start_order, include_start, end_order, include_end)
    };
    let mut selected = nodes
        .iter()
        .filter(|(order, _)| {
            let after = lower.is_none_or(|bound| {
                order.as_slice() > bound || (lower_inclusive && order.as_slice() == bound)
            });
            let before = upper.is_none_or(|bound| {
                order.as_slice() < bound || (upper_inclusive && order.as_slice() == bound)
            });
            after && before
        })
        .cloned()
        .collect::<Vec<_>>();
    if descending {
        selected.reverse();
    }
    // 与类型层的范围裁剪保持同一规则：端点切入某个容器时只保留命中的
    // 后缀；中间容器作为整体节点保留，避免把 `<2` 展平成子元素。
    let selected_snapshot = selected.clone();
    let boundaries = [start_order, end_order];
    selected.retain(|(order, path)| {
        if lower.is_some_and(|bound| !lower_inclusive && is_strict_order_prefix(bound, order)) {
            return false;
        }
        if boundaries
            .iter()
            .flatten()
            .any(|bound| is_order_prefix(order, bound) && order.len() < bound.len())
        {
            return false;
        }
        !selected_snapshot
            .iter()
            .any(|(ancestor_order, ancestor_path)| {
                is_strict_runtime_path_prefix(ancestor_path, path)
                    && !boundaries.iter().flatten().any(|bound| {
                        is_order_prefix(ancestor_order, bound) && ancestor_order.len() < bound.len()
                    })
            })
    });
    Ok(selected.into_iter().map(|(_, path)| path).collect())
}

/// 解析一个范围端点并返回其深度优先顺序键；空路径代表单边范围。
fn resolve_range_boundary(
    source: &RuntimeValue,
    boundary: &IrSelectionPath,
) -> RuntimeResult<Option<(Vec<usize>, IrSelectionPath)>> {
    if boundary.is_empty() {
        return Ok(None);
    }
    let mut current = source.clone();
    let mut path = Vec::with_capacity(boundary.len());
    let mut order = Vec::with_capacity(boundary.len());
    for segment in boundary {
        if matches!(current, RuntimeValue::DictTable(_)) {
            return Err(RuntimeError::selector_bounds("范围不能穿过无序字典表"));
        }
        let (canonical, position) = match (&current, segment) {
            (RuntimeValue::Array(handle), IrSelectionPathSegment::Index { raw, .. }) => {
                let index = xiao_types::normalize_index(*raw, handle.len())
                    .ok_or_else(|| RuntimeError::selector_bounds("范围端点索引越界"))?;
                (
                    IrSelectionPathSegment::Index {
                        raw: *raw,
                        resolved: Some(index),
                    },
                    index,
                )
            }
            (RuntimeValue::Tuple(handle), IrSelectionPathSegment::Index { raw, .. }) => {
                let index = xiao_types::normalize_index(*raw, handle.len())
                    .ok_or_else(|| RuntimeError::selector_bounds("范围端点索引越界"))?;
                (
                    IrSelectionPathSegment::Index {
                        raw: *raw,
                        resolved: Some(index),
                    },
                    index,
                )
            }
            (RuntimeValue::DictColumn(handle), IrSelectionPathSegment::Key(key)) => {
                let index = handle
                    .with_entries(|entries| entries.iter().position(|(name, _)| name == key))?
                    .ok_or_else(|| RuntimeError::selector_bounds("范围端点键不存在"))?;
                (IrSelectionPathSegment::Key(key.clone()), index)
            }
            (RuntimeValue::DictColumn(handle), IrSelectionPathSegment::Index { raw, .. }) => {
                let index = xiao_types::normalize_index(*raw, handle.len())
                    .ok_or_else(|| RuntimeError::selector_bounds("范围端点索引越界"))?;
                (
                    IrSelectionPathSegment::Index {
                        raw: *raw,
                        resolved: Some(index),
                    },
                    index,
                )
            }
            (RuntimeValue::Str(handle), IrSelectionPathSegment::Index { raw, .. }) => {
                let index = xiao_types::normalize_index(*raw, handle.len())
                    .ok_or_else(|| RuntimeError::selector_bounds("范围端点索引越界"))?;
                (
                    IrSelectionPathSegment::Index {
                        raw: *raw,
                        resolved: Some(index),
                    },
                    index,
                )
            }
            (RuntimeValue::Set(_), _) => {
                return Err(RuntimeError::selector_bounds("集合不可索引"));
            }
            _ => {
                return Err(RuntimeError::selector_bounds("范围端点路径与容器不匹配"));
            }
        };
        current = index_step(
            &current,
            &match &canonical {
                IrSelectionPathSegment::Index { raw, .. } => PathStep::Index(*raw),
                IrSelectionPathSegment::Key(key) => PathStep::Key(key.clone()),
            },
        )
        .map_err(|_| RuntimeError::selector_bounds("范围端点不可读取"))?;
        path.push(canonical);
        order.push(position);
    }
    Ok(Some((order, path)))
}

/// 枚举运行时来源的全部有序节点（深度优先）。
fn enumerate_runtime_nodes(
    source: &RuntimeValue,
    prefix: &[IrSelectionPathSegment],
    output: &mut Vec<(Vec<usize>, IrSelectionPath)>,
) -> RuntimeResult<()> {
    match source {
        RuntimeValue::Array(handle) => {
            for index in 0..handle.len() {
                let child = handle
                    .element(index)?
                    .ok_or_else(|| RuntimeError::selector_bounds("范围节点不可读取"))?;
                let path = append_index(prefix, index);
                let mut order = path_order(&path);
                output.push((std::mem::take(&mut order), path.clone()));
                if is_runtime_ordered(&child) {
                    enumerate_runtime_nodes(&child, &path, output)?;
                }
            }
            Ok(())
        }
        RuntimeValue::Tuple(handle) => {
            for index in 0..handle.len() {
                let child = handle
                    .element(index)?
                    .ok_or_else(|| RuntimeError::selector_bounds("范围节点不可读取"))?;
                let path = append_index(prefix, index);
                output.push((path_order(&path), path.clone()));
                if is_runtime_ordered(&child) {
                    enumerate_runtime_nodes(&child, &path, output)?;
                }
            }
            Ok(())
        }
        RuntimeValue::DictColumn(handle) => {
            for index in 0..handle.len() {
                let child = handle
                    .with_entries(|entries| entries.get(index).map(|(_, value)| value.clone()))?
                    .ok_or_else(|| RuntimeError::selector_bounds("范围节点不可读取"))?;
                let path = append_index(prefix, index);
                output.push((path_order(&path), path.clone()));
                if is_runtime_ordered(&child) {
                    enumerate_runtime_nodes(&child, &path, output)?;
                }
            }
            Ok(())
        }
        RuntimeValue::Str(handle) => {
            for index in 0..handle.len() {
                let path = append_index(prefix, index);
                output.push((path_order(&path), path));
            }
            Ok(())
        }
        RuntimeValue::DictTable(_) => {
            Err(RuntimeError::selector_bounds("无序字典表不支持范围选择"))
        }
        RuntimeValue::Set(_) => Err(RuntimeError::selector_bounds("集合不可索引")),
        _ => Err(RuntimeError::selector_bounds("范围来源不是有序容器")),
    }
}

/// 判断一个运行时值是否可继续按有序节点展开。
fn is_runtime_ordered(value: &RuntimeValue) -> bool {
    matches!(
        value,
        RuntimeValue::Array(_)
            | RuntimeValue::Tuple(_)
            | RuntimeValue::DictColumn(_)
            | RuntimeValue::Str(_)
    )
}

/// 追加一个规范化数字路径段。
fn append_index(prefix: &[IrSelectionPathSegment], index: usize) -> IrSelectionPath {
    let mut path = prefix.to_vec();
    path.push(IrSelectionPathSegment::Index {
        raw: index as i128,
        resolved: Some(index),
    });
    path
}

/// 从规范化路径提取深度优先顺序键。
fn path_order(path: &IrSelectionPath) -> Vec<usize> {
    path.iter()
        .filter_map(|segment| match segment {
            IrSelectionPathSegment::Index {
                resolved: Some(index),
                ..
            } => Some(*index),
            IrSelectionPathSegment::Key(_) => None,
            IrSelectionPathSegment::Index { resolved: None, .. } => None,
        })
        .collect()
}

/// 判断一条运行时路径是否是另一条路径的严格前缀。
fn is_strict_runtime_path_prefix(prefix: &IrSelectionPath, path: &IrSelectionPath) -> bool {
    prefix.len() < path.len()
        && prefix
            .iter()
            .zip(path)
            .all(|(left, right)| match (left, right) {
                (
                    IrSelectionPathSegment::Index {
                        resolved: Some(left),
                        ..
                    },
                    IrSelectionPathSegment::Index {
                        resolved: Some(right),
                        ..
                    },
                ) => left == right,
                (IrSelectionPathSegment::Key(left), IrSelectionPathSegment::Key(right)) => {
                    left == right
                }
                _ => false,
            })
}

/// 判断一个顺序键是否是另一个键的严格前缀。
fn is_strict_order_prefix(prefix: &[usize], order: &[usize]) -> bool {
    prefix.len() < order.len() && is_order_prefix(prefix, order)
}

/// 判断一个顺序键是否为另一个键的前缀。
fn is_order_prefix(prefix: &[usize], order: &[usize]) -> bool {
    prefix.len() <= order.len() && prefix.iter().zip(order).all(|(left, right)| left == right)
}

/// 从计划路径读取值。
fn read_ir_path(source: &RuntimeValue, path: &IrSelectionPath) -> RuntimeResult<RuntimeValue> {
    let path = path
        .iter()
        .map(|segment| match segment {
            IrSelectionPathSegment::Index { raw, .. } => PathStep::Index(*raw),
            IrSelectionPathSegment::Key(key) => PathStep::Key(key.clone()),
        })
        .collect::<Vec<_>>();
    index_get(source, &path)
}

/// 按类型计划构造结果值。
fn build_selection_result(
    source: &RuntimeValue,
    plan: &IrSelectionPlan,
    paths: &[IrSelectionPath],
    values: &[RuntimeValue],
) -> RuntimeResult<RuntimeValue> {
    if matches!(plan.result_type, IrType::Scalar { ref name } if name == "str")
        && matches!(source, RuntimeValue::Str(_))
    {
        let mut text = String::new();
        for value in values {
            let RuntimeValue::Str(handle) = value else {
                return Err(RuntimeError::selector_bounds("字符串选择结果不是字符"));
            };
            text.push_str(&handle.to_string()?);
        }
        return RuntimeValue::new_string(text);
    }
    match &plan.result_type {
        IrType::Tuple { .. } if matches!(source, RuntimeValue::DictColumn(_)) => {
            new_tuple(values.to_vec())
        }
        IrType::Array { .. } | IrType::Tuple { .. } | IrType::DictColumn { .. } => {
            project_ordered_result(source, paths, values)
        }
        _ => new_array(values.to_vec()),
    }
}

/// 按来源容器递归重建选择结果，保留直接节点分组和嵌套形状。
fn project_ordered_result(
    source: &RuntimeValue,
    paths: &[IrSelectionPath],
    values: &[RuntimeValue],
) -> RuntimeResult<RuntimeValue> {
    if paths.is_empty() {
        return empty_runtime_container(source);
    }
    if matches!(source, RuntimeValue::Str(_)) {
        let mut text = String::new();
        for value in values {
            let RuntimeValue::Str(handle) = value else {
                return Err(RuntimeError::selector_bounds("字符串选择结果不是字符"));
            };
            text.push_str(&handle.to_string()?);
        }
        return RuntimeValue::new_string(text);
    }
    let groups = group_runtime_paths(source, paths)?;
    if groups.is_empty() {
        return new_array(values.to_vec());
    }
    let mut projected = Vec::with_capacity(groups.len());
    let mut keys = Vec::with_capacity(groups.len());
    for group in groups {
        let first_position = group.positions[0];
        if group.direct {
            projected.push(values[first_position].clone());
        } else {
            let nested_paths = group
                .positions
                .iter()
                .map(|position| paths[*position][1..].to_vec())
                .collect::<Vec<_>>();
            let nested_values = group
                .positions
                .iter()
                .map(|position| values[*position].clone())
                .collect::<Vec<_>>();
            let child = read_ir_path(source, &paths[first_position][..1].to_vec())?;
            projected.push(project_ordered_result(
                &child,
                &nested_paths,
                &nested_values,
            )?);
        }
        if let RuntimeValue::DictColumn(handle) = source {
            let key = handle
                .with_entries(|entries| entries.get(group.index).map(|(key, _)| key.clone()))?
                .ok_or_else(|| RuntimeError::selector_bounds("字典列结果键不存在"))?;
            keys.push(key);
        }
    }
    match source {
        RuntimeValue::Array(_) => new_array(projected),
        RuntimeValue::Tuple(_) => new_tuple(projected),
        RuntimeValue::DictColumn(_) => new_dict_column(keys.into_iter().zip(projected).collect()),
        _ => new_array(projected),
    }
}

/// 一个直接路径分组；`direct` 表示该组选择了父节点本身。
struct RuntimePathGroup {
    /// 直接子节点的零基位置。
    index: usize,
    /// 对应原始路径/值的位置。
    positions: Vec<usize>,
    /// 是否为父节点直接选择。
    direct: bool,
}

/// 按类型层 `group_direct_paths` 的规则分组运行时路径。
fn group_runtime_paths(
    source: &RuntimeValue,
    paths: &[IrSelectionPath],
) -> RuntimeResult<Vec<RuntimePathGroup>> {
    let mut groups = Vec::new();
    for (position, path) in paths.iter().enumerate() {
        let Some(segment) = path.first() else {
            continue;
        };
        let index = runtime_segment_index(source, segment)?;
        let direct = path.len() == 1;
        if direct {
            groups.push(RuntimePathGroup {
                index,
                positions: vec![position],
                direct: true,
            });
        } else if let Some(group) = groups
            .iter_mut()
            .find(|group: &&mut RuntimePathGroup| group.index == index && !group.direct)
        {
            group.positions.push(position);
        } else {
            groups.push(RuntimePathGroup {
                index,
                positions: vec![position],
                direct: false,
            });
        }
    }
    Ok(groups)
}

/// 把直接路径段解析成来源容器中的位置。
fn runtime_segment_index(
    source: &RuntimeValue,
    segment: &IrSelectionPathSegment,
) -> RuntimeResult<usize> {
    match (source, segment) {
        (RuntimeValue::Array(handle), IrSelectionPathSegment::Index { raw, .. }) => {
            xiao_types::normalize_index(*raw, handle.len())
                .ok_or_else(|| RuntimeError::selector_bounds("结果路径索引越界"))
        }
        (RuntimeValue::Tuple(handle), IrSelectionPathSegment::Index { raw, .. }) => {
            xiao_types::normalize_index(*raw, handle.len())
                .ok_or_else(|| RuntimeError::selector_bounds("结果路径索引越界"))
        }
        (RuntimeValue::DictColumn(handle), IrSelectionPathSegment::Index { raw, .. }) => {
            xiao_types::normalize_index(*raw, handle.len())
                .ok_or_else(|| RuntimeError::selector_bounds("结果路径索引越界"))
        }
        (RuntimeValue::DictColumn(handle), IrSelectionPathSegment::Key(key)) => handle
            .with_entries(|entries| entries.iter().position(|(name, _)| name == key))?
            .ok_or_else(|| RuntimeError::selector_bounds("结果路径键不存在")),
        _ => Err(RuntimeError::selector_bounds("结果路径与来源容器不匹配")),
    }
}

/// 构造与来源容器同根的空运行时值。
fn empty_runtime_container(source: &RuntimeValue) -> RuntimeResult<RuntimeValue> {
    match source {
        RuntimeValue::Array(_) => new_array(Vec::new()),
        RuntimeValue::Tuple(_) => new_tuple(Vec::new()),
        RuntimeValue::DictColumn(_) => new_dict_column(Vec::new()),
        RuntimeValue::Str(_) => RuntimeValue::new_string(String::new()),
        _ => new_array(Vec::new()),
    }
}

/// 从运行时标量读取整数。
fn integer_value(value: &RuntimeValue) -> Option<i128> {
    match value {
        RuntimeValue::Int(value) => Some(i128::from(*value)),
        RuntimeValue::Sint(value) => Some(i128::from(*value)),
        RuntimeValue::Lint(value) => value.parse().ok(),
        _ => None,
    }
}

/// 判断运行时值是否为容器。
fn is_runtime_container(value: &RuntimeValue) -> bool {
    matches!(
        value,
        RuntimeValue::Array(_)
            | RuntimeValue::Tuple(_)
            | RuntimeValue::DictTable(_)
            | RuntimeValue::DictColumn(_)
            | RuntimeValue::Set(_)
    )
}

/// 将共享随机算法的边界错误映射为稳定 Runtime 身份。
fn random_error(error: RandomSelectionError) -> RuntimeError {
    match error {
        RandomSelectionError::InvalidCount => RuntimeError::random_count("随机数量无效"),
        RandomSelectionError::WithoutReplacementTooMany { count, available } => {
            RuntimeError::random_count(format!("无放回抽取 {count} 项超过候选数 {available}"))
        }
        RandomSelectionError::EmptySource => RuntimeError::random_count("不能从空来源抽取元素"),
    }
}

/// 按容器长度归一化一个有符号索引。
fn resolve_index(raw: i128, length: usize, container: &str) -> RuntimeResult<usize> {
    xiao_types::normalize_index(raw, length)
        .ok_or_else(|| RuntimeError::index_out_of_bounds(container, length, raw))
}

/// 归一化通过、但元素仍取不到时使用的错误。
///
/// 只有容器在读取过程中变得不可读才会走到这里，因此按「对象已释放」报告，
/// 而不是伪造一个越界位置。
fn missing_element() -> RuntimeError {
    RuntimeError::use_after_release()
}

/// 执行一条三地址比较指令，结果恒为布尔。
pub fn apply_compare(
    op: CompareOp,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    let result = match op {
        CompareOp::Less => left.less(right)?,
        CompareOp::LessEqual => left.less_equal(right)?,
        CompareOp::Greater => left.greater(right)?,
        CompareOp::GreaterEqual => left.greater_equal(right)?,
        CompareOp::Equal => left.equals(right),
        CompareOp::NotEqual => left.not_equals(right),
    };
    Ok(RuntimeValue::Bool(result))
}

#[cfg(test)]
/// 容器构造与精确索引的运行时语义。
///
/// 这些用例直接调用算子而非从源码构造：静态检查器会提前拒绝常量越界与缺失键，
/// 因此运行时容器错误在字面量程序里不可达，只能在这一层验证。
mod tests {
    use super::{
        apply_set_compare, apply_set_op, broadcast_assign, index_get, index_get_dynamic, len,
        new_array, new_dict_column, new_dict_table, new_set, new_tuple, selector_apply,
    };
    use xiao_bytecode::research::PathStep;
    use xiao_bytecode::research::{SetCompareOp, SetOpKind};
    use xiao_ir::{
        IrArrayShape, IrBroadcastAssignmentPlan, IrSelectionItemPlan, IrSelectionPath,
        IrSelectionPathSegment, IrSelectionPlan, IrSpan, IrStepPlan, IrType,
    };
    use xiao_runtime::{
        CONTAINER_HASHABILITY_CODE, CONTAINER_INDEX_CODE, CONTAINER_KEY_CODE, RANDOM_COUNT_CODE,
        RuntimeValue, SELECTOR_BOUNDS_CODE, SELECTOR_STEP_CODE, SET_COMPARISON_CODE,
        SET_MEMBERSHIP_CODE, SET_OPERATION_CODE,
    };
    use xiao_types::SeededRandom;

    /// 返回测试计划使用的零宽源码区间。
    fn span() -> IrSpan {
        IrSpan::new(0, 0)
    }

    /// 构造一个未解析的单段数字路径。
    fn index_path(index: i128) -> IrSelectionPath {
        vec![IrSelectionPathSegment::Index {
            raw: index,
            resolved: None,
        }]
    }

    /// 构造一条已解析的数字路径。
    fn resolved_path(indices: &[usize]) -> IrSelectionPath {
        indices
            .iter()
            .map(|index| IrSelectionPathSegment::Index {
                raw: *index as i128,
                resolved: Some(*index),
            })
            .collect()
    }

    /// 返回测试用的整数标量类型。
    fn int_type() -> IrType {
        IrType::Scalar {
            name: "int".to_owned(),
        }
    }

    /// 构造指定长度的异构整数数组结果类型。
    fn array_result_type(element_count: usize) -> IrType {
        IrType::Array {
            shape: IrArrayShape::Heterogeneous {
                elements: vec![int_type(); element_count],
            },
        }
    }

    /// 构造不含动态边界的最小选择计划。
    fn selection_plan(
        items: Vec<IrSelectionItemPlan>,
        selected_paths: Vec<IrSelectionPath>,
        result_type: IrType,
    ) -> IrSelectionPlan {
        IrSelectionPlan {
            span: span(),
            source_type: IrType::Dynamic,
            result_type,
            items,
            selected_paths,
            target_types: Vec::new(),
            step: None,
            requires_runtime_check: false,
            with_replacement: false,
            has_duplicates: false,
        }
    }

    /// 提取数组或元组中的整数元素，供形状断言使用。
    fn ints(value: &RuntimeValue) -> Vec<i64> {
        match value {
            RuntimeValue::Array(handle) => handle
                .with_elements(|elements| {
                    elements
                        .iter()
                        .map(|element| match element {
                            RuntimeValue::Int(value) => *value,
                            other => panic!("应为整数元素，实际为 {other:?}"),
                        })
                        .collect()
                })
                .expect("数组应可读"),
            RuntimeValue::Tuple(handle) => handle
                .with_elements(|elements| {
                    elements
                        .iter()
                        .map(|element| match element {
                            RuntimeValue::Int(value) => *value,
                            other => panic!("应为整数元素，实际为 {other:?}"),
                        })
                        .collect()
                })
                .expect("元组应可读"),
            other => panic!("应为数组或元组，实际为 {other:?}"),
        }
    }

    /// 提取一层嵌套数组或元组中的整数元素。
    fn nested_ints(value: &RuntimeValue) -> Vec<Vec<i64>> {
        let elements = match value {
            RuntimeValue::Array(handle) => handle
                .with_elements(|elements| elements.to_vec())
                .expect("数组应可读"),
            RuntimeValue::Tuple(handle) => handle
                .with_elements(|elements| elements.to_vec())
                .expect("元组应可读"),
            other => panic!("应为嵌套容器，实际为 {other:?}"),
        };
        elements.iter().map(ints).collect()
    }

    /// 构造一个两元素整数数组。
    fn pair() -> RuntimeValue {
        new_array(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]).expect("数组应分配")
    }

    #[test]
    /// 正索引与负索引都按同一套归一化语义取到元素。
    fn array_index_supports_negative_positions() {
        let array = pair();
        assert_eq!(
            index_get(&array, &[PathStep::Index(0)]),
            Ok(RuntimeValue::Int(1))
        );
        assert_eq!(
            index_get(&array, &[PathStep::Index(-1)]),
            Ok(RuntimeValue::Int(2))
        );
        assert_eq!(
            index_get(&array, &[PathStep::Index(-2)]),
            Ok(RuntimeValue::Int(1))
        );
    }

    #[test]
    /// 越界使用稳定错误身份，不返回空值也不截断。
    fn array_index_out_of_bounds_is_stable() {
        let array = pair();
        let error = index_get(&array, &[PathStep::Index(2)]).expect_err("应越界");
        assert_eq!(error.code(), CONTAINER_INDEX_CODE);
        let error = index_get(&array, &[PathStep::Index(-3)]).expect_err("负索引也应越界");
        assert_eq!(error.code(), CONTAINER_INDEX_CODE);
    }

    #[test]
    /// 字典表按键读取，键缺失使用稳定错误身份。
    fn dict_key_lookup_is_stable() {
        let dict = new_dict_table(vec![("a".to_owned(), RuntimeValue::Int(7))]).expect("应分配");
        assert_eq!(
            index_get(&dict, &[PathStep::Key("a".to_owned())]),
            Ok(RuntimeValue::Int(7))
        );
        let error = index_get(&dict, &[PathStep::Key("b".to_owned())]).expect_err("键应缺失");
        assert_eq!(error.code(), CONTAINER_KEY_CODE);
    }

    #[test]
    /// 字典列同时支持键与数字索引。
    fn dict_column_supports_key_and_position() {
        let column = new_dict_column(vec![
            ("a".to_owned(), RuntimeValue::Int(7)),
            ("b".to_owned(), RuntimeValue::Int(8)),
        ])
        .expect("应分配");
        assert_eq!(
            index_get(&column, &[PathStep::Key("b".to_owned())]),
            Ok(RuntimeValue::Int(8))
        );
        assert_eq!(
            index_get(&column, &[PathStep::Index(-1)]),
            Ok(RuntimeValue::Int(8))
        );
    }

    #[test]
    /// 字符串按字符位置索引，返回单字符字符串。
    fn string_index_returns_single_character() {
        let text = RuntimeValue::new_string("小雪").expect("应分配");
        let indexed = index_get(&text, &[PathStep::Index(-1)]).expect("应取到字符");
        assert_eq!(indexed.type_name(), "str");
        assert!(index_get(&text, &[PathStep::Index(2)]).is_err());
    }

    #[test]
    /// 元组和集合支持运行时位置索引；标量仍给出类型错误。
    fn tuple_and_set_index_and_unsupported_source() {
        let tuple = new_tuple(vec![RuntimeValue::Int(1)]).expect("应分配");
        assert_eq!(
            index_get(&tuple, &[PathStep::Index(0)]),
            Ok(RuntimeValue::Int(1))
        );
        let set = new_set(vec![RuntimeValue::Int(1)]).expect("应分配");
        assert_eq!(
            index_get(&set, &[PathStep::Index(0)]),
            Ok(RuntimeValue::Int(1)),
            "迭代的内部动态索引必须沿用集合物理顺序"
        );
        assert!(index_get(&RuntimeValue::Int(1), &[PathStep::Index(0)]).is_err());
    }

    #[test]
    /// `Len`/动态索引的运行时算子必须覆盖所有可迭代形态，并以码点计字符串长度。
    fn len_and_dynamic_index_cover_iterables() {
        let array = new_array(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]).expect("应分配");
        let tuple = new_tuple(vec![RuntimeValue::Int(3)]).expect("应分配");
        let set = new_set(vec![RuntimeValue::Int(4), RuntimeValue::Int(5)]).expect("应分配");
        let table = new_dict_table(vec![
            ("first".to_owned(), RuntimeValue::Int(6)),
            ("second".to_owned(), RuntimeValue::Int(7)),
        ])
        .expect("应分配");
        let column = new_dict_column(vec![
            ("first".to_owned(), RuntimeValue::Int(8)),
            ("second".to_owned(), RuntimeValue::Int(9)),
        ])
        .expect("应分配");
        let text = RuntimeValue::new_string("小雪A").expect("应分配");
        for (value, expected) in [
            (&array, 2),
            (&tuple, 1),
            (&set, 2),
            (&table, 2),
            (&column, 2),
            (&text, 3),
        ] {
            assert_eq!(len(value), Ok(RuntimeValue::Int(expected)));
        }
        let indexed = index_get_dynamic(&text, &RuntimeValue::Int(1)).expect("码点索引应成功");
        let RuntimeValue::Str(indexed) = indexed else {
            panic!("字符串动态索引应返回 str");
        };
        assert_eq!(indexed.to_string().expect("字符串应可读取"), "雪");
        assert_eq!(
            index_get_dynamic(&table, &RuntimeValue::Int(-1)),
            Ok(RuntimeValue::Int(7))
        );
        assert_eq!(
            index_get_dynamic(&column, &RuntimeValue::Int(0)),
            Ok(RuntimeValue::Int(8))
        );
    }

    #[test]
    /// 路径会继续下降到嵌套容器；若叶子不是容器，必须明确报错而不是静默取第一段。
    fn nested_path_rejects_non_container_leaf() {
        let array = pair();
        assert!(index_get(&array, &[PathStep::Index(0), PathStep::Index(1)]).is_err());
    }

    #[test]
    /// 不可哈希元素进入集合的稳定错误身份。
    fn set_hashability_error_identity() {
        let nested = pair();
        let error = new_set(vec![nested]).expect_err("数组不可作为集合元素");
        assert_eq!(error.code(), CONTAINER_HASHABILITY_CODE);
    }

    #[test]
    /// 集合算子层必须报告稳定的操作数错误，而不是落入数值宽度诊断。
    fn set_operation_and_comparison_error_identities_are_stable() {
        let scalar = RuntimeValue::Int(1);
        let set = new_set(vec![RuntimeValue::Int(1)]).expect("集合应分配");
        let operation = apply_set_op(SetOpKind::Union, &scalar, &set).expect_err("应拒绝标量");
        assert_eq!(operation.code(), SET_OPERATION_CODE);
        let comparison =
            apply_set_compare(SetCompareOp::Equal, &scalar, &set).expect_err("应拒绝标量");
        assert_eq!(comparison.code(), SET_COMPARISON_CODE);
    }

    #[test]
    /// `x in s` 的左值是被包含成员，不能把两个集合比较的方向套过来。
    fn set_membership_uses_left_member_and_right_set_direction() {
        let set = new_set(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]).expect("集合应分配");
        assert_eq!(
            apply_set_compare(SetCompareOp::Member, &RuntimeValue::Int(1), &set),
            Ok(RuntimeValue::Bool(true))
        );
        assert_eq!(
            apply_set_compare(SetCompareOp::Member, &RuntimeValue::Int(3), &set),
            Ok(RuntimeValue::Bool(false))
        );
        assert_eq!(
            apply_set_compare(SetCompareOp::NotMember, &RuntimeValue::Int(3), &set),
            Ok(RuntimeValue::Bool(true))
        );
        let error = apply_set_compare(
            SetCompareOp::Member,
            &RuntimeValue::Int(1),
            &RuntimeValue::Int(2),
        )
        .expect_err("右侧非集合应拒绝");
        assert_eq!(error.code(), SET_MEMBERSHIP_CODE);
    }

    #[test]
    /// 字典列的键名路径必须投影出对应键，而不能被当作无效的数组路径丢弃。
    fn selector_projects_dictionary_key_path() {
        let source = new_dict_column(vec![
            ("a".to_owned(), RuntimeValue::Int(1)),
            ("b".to_owned(), RuntimeValue::Int(2)),
        ])
        .expect("字典列应分配");
        let path = vec![IrSelectionPathSegment::Key("b".to_owned())];
        let plan = selection_plan(
            vec![IrSelectionItemPlan::Exact { path: path.clone() }],
            vec![path],
            IrType::DictColumn {
                entries: Vec::new(),
            },
        );
        let mut random = SeededRandom::new(7);
        let result = selector_apply(&source, &plan, None, &[], &mut random).expect("选择应成功");
        let RuntimeValue::DictColumn(handle) = result else {
            panic!("结果应保持字典列形状");
        };
        let entries = handle
            .with_entries(|entries| entries.to_vec())
            .expect("条目应可读");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "b");
        assert_eq!(entries[0].1, RuntimeValue::Int(2));
    }

    #[test]
    /// 字典列同一直接键被索引和键名重复命中时必须转为有序元组。
    fn selector_keeps_dictionary_duplicate_order() {
        let source = new_dict_column(vec![
            ("a".to_owned(), RuntimeValue::Int(1)),
            ("b".to_owned(), RuntimeValue::Int(2)),
        ])
        .expect("字典列应分配");
        let first = resolved_path(&[0]);
        let second = vec![IrSelectionPathSegment::Key("a".to_owned())];
        let plan = selection_plan(
            vec![
                IrSelectionItemPlan::Exact {
                    path: first.clone(),
                },
                IrSelectionItemPlan::Exact {
                    path: second.clone(),
                },
            ],
            vec![first, second],
            IrType::Tuple {
                elements: vec![int_type(), int_type()],
            },
        );
        let mut random = SeededRandom::new(7);
        let result = selector_apply(&source, &plan, None, &[], &mut random).expect("选择应成功");
        assert_eq!(ints(&result), vec![1, 1]);
    }

    #[test]
    /// 多段嵌套路径应按直接父节点分组，保留来源容器的嵌套形状。
    fn selector_projects_nested_arrays() {
        let source = new_array(vec![
            new_array(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]).expect("内层数组"),
            new_array(vec![RuntimeValue::Int(3), RuntimeValue::Int(4)]).expect("内层数组"),
        ])
        .expect("外层数组");
        let first = resolved_path(&[0, 1]);
        let second = resolved_path(&[1, 0]);
        let plan = selection_plan(
            vec![
                IrSelectionItemPlan::Exact {
                    path: first.clone(),
                },
                IrSelectionItemPlan::Exact {
                    path: second.clone(),
                },
            ],
            vec![first, second],
            array_result_type(2),
        );
        let mut random = SeededRandom::new(7);
        let result = selector_apply(&source, &plan, None, &[], &mut random).expect("选择应成功");
        assert_eq!(nested_ints(&result), vec![vec![2], vec![3]]);
    }

    #[test]
    /// 动态范围必须在运行时枚举嵌套节点，并继续应用静态步长。
    fn dynamic_range_applies_static_step() {
        let source =
            new_array((0..6).map(RuntimeValue::Int).collect::<Vec<_>>()).expect("数组应分配");
        let start = index_path(0);
        let end = index_path(5);
        let mut plan = selection_plan(
            vec![IrSelectionItemPlan::Range {
                start,
                end,
                include_start: true,
                include_end: true,
            }],
            Vec::new(),
            array_result_type(3),
        );
        plan.step = Some(IrStepPlan {
            value: Some(2),
            dynamic: false,
        });
        plan.requires_runtime_check = true;
        let mut random = SeededRandom::new(7);
        let result = selector_apply(&source, &plan, None, &[], &mut random).expect("范围应成功");
        assert_eq!(ints(&result), vec![0, 2, 4]);
    }

    #[test]
    /// 动态嵌套范围要与类型层的深度优先边界裁剪保持一致。
    fn dynamic_nested_range_preserves_boundary_shape() {
        let source = new_array(vec![
            new_array(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]).expect("内层数组"),
            new_array(vec![RuntimeValue::Int(3), RuntimeValue::Int(4)]).expect("内层数组"),
        ])
        .expect("外层数组");
        let mut plan = selection_plan(
            vec![IrSelectionItemPlan::Range {
                start: resolved_path(&[0]),
                end: resolved_path(&[1, 0]),
                include_start: true,
                include_end: true,
            }],
            Vec::new(),
            array_result_type(2),
        );
        plan.requires_runtime_check = true;
        let mut random = SeededRandom::new(1);
        let result = selector_apply(&source, &plan, None, &[], &mut random).expect("范围应成功");
        assert_eq!(nested_ints(&result), vec![vec![1, 2], vec![3]]);
    }

    #[test]
    /// 相同种子必须驱动相同的抽样序列；静态随机项不能退化为空路径。
    fn static_random_is_reproducible() {
        let source =
            new_array((1..=4).map(RuntimeValue::Int).collect::<Vec<_>>()).expect("数组应分配");
        let plan = selection_plan(
            vec![IrSelectionItemPlan::Random {
                mode: "without_replacement".to_owned(),
                count: Some(3),
                dynamic_count: false,
            }],
            Vec::new(),
            array_result_type(4),
        );
        let mut first_random = SeededRandom::new(42);
        let mut second_random = SeededRandom::new(42);
        let first =
            selector_apply(&source, &plan, None, &[], &mut first_random).expect("抽样应成功");
        let second =
            selector_apply(&source, &plan, None, &[], &mut second_random).expect("抽样应成功");
        assert_eq!(ints(&first), ints(&second));
        assert_eq!(ints(&first).len(), 3);
    }

    #[test]
    /// 动态随机数量的负值、无放回超量和空来源都必须保留随机数量错误身份。
    fn dynamic_random_boundaries_are_stable() {
        let source = pair();
        let plan = selection_plan(
            vec![IrSelectionItemPlan::Random {
                mode: "without_replacement".to_owned(),
                count: None,
                dynamic_count: true,
            }],
            Vec::new(),
            array_result_type(0),
        );
        let mut random = SeededRandom::new(1);
        let error = selector_apply(
            &source,
            &plan,
            None,
            &[Some(RuntimeValue::Int(-1))],
            &mut random,
        )
        .expect_err("负数量应失败");
        assert_eq!(error.code(), RANDOM_COUNT_CODE);

        let mut random = SeededRandom::new(1);
        let error = selector_apply(
            &source,
            &plan,
            None,
            &[Some(RuntimeValue::Int(3))],
            &mut random,
        )
        .expect_err("无放回超量应失败");
        assert_eq!(error.code(), RANDOM_COUNT_CODE);

        let empty = new_array(Vec::new()).expect("空数组应分配");
        let mut random = SeededRandom::new(1);
        let error = selector_apply(
            &empty,
            &plan,
            None,
            &[Some(RuntimeValue::Int(1))],
            &mut random,
        )
        .expect_err("空来源正抽样应失败");
        assert_eq!(error.code(), RANDOM_COUNT_CODE);
    }

    #[test]
    /// 动态步长零值必须报选择器步长错误，合法负步长则反向取值。
    fn dynamic_step_boundaries_are_stable() {
        let source =
            new_array((0..5).map(RuntimeValue::Int).collect::<Vec<_>>()).expect("数组应分配");
        let mut plan = selection_plan(
            vec![IrSelectionItemPlan::All],
            Vec::new(),
            array_result_type(0),
        );
        plan.step = Some(IrStepPlan {
            value: None,
            dynamic: true,
        });
        plan.requires_runtime_check = true;
        let mut random = SeededRandom::new(1);
        let error = selector_apply(
            &source,
            &plan,
            Some(&RuntimeValue::Int(0)),
            &[],
            &mut random,
        )
        .expect_err("零步长应失败");
        assert_eq!(error.code(), SELECTOR_STEP_CODE);

        let mut random = SeededRandom::new(1);
        let result = selector_apply(
            &source,
            &plan,
            Some(&RuntimeValue::Int(-2)),
            &[],
            &mut random,
        )
        .expect("负步长应成功");
        assert_eq!(ints(&result), vec![4, 2, 0]);
    }

    #[test]
    /// 广播必须先验证全部目标；嵌套数组的合法目标则一次性写入。
    fn broadcast_is_transactional_and_supports_nested_paths() {
        let root = new_array(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]).expect("数组");
        let plan = IrBroadcastAssignmentPlan {
            span: span(),
            root_name: Some("root".to_owned()),
            target_paths: vec![resolved_path(&[0]), resolved_path(&[2])],
            value_type: int_type(),
            dynamic: false,
            transactional: true,
        };
        let error = broadcast_assign(&root, &RuntimeValue::Int(9), &plan)
            .expect_err("第二个目标越界应回滚");
        assert_eq!(error.code(), SELECTOR_BOUNDS_CODE);
        assert_eq!(ints(&root), vec![1, 2]);

        let nested = new_array(vec![
            new_array(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]).expect("内层数组"),
            new_array(vec![RuntimeValue::Int(3), RuntimeValue::Int(4)]).expect("内层数组"),
        ])
        .expect("外层数组");
        let nested_plan = IrBroadcastAssignmentPlan {
            span: span(),
            root_name: Some("nested".to_owned()),
            target_paths: vec![resolved_path(&[0, 1]), resolved_path(&[1, 0])],
            value_type: int_type(),
            dynamic: false,
            transactional: true,
        };
        broadcast_assign(&nested, &RuntimeValue::Int(7), &nested_plan).expect("嵌套广播应成功");
        assert_eq!(nested_ints(&nested), vec![vec![1, 7], vec![7, 4]]);
    }
}
