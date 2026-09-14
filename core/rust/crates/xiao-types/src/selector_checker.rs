//! C1 有序容器选择器的静态检查与计划生成。
//!
//! 这里把语法层的选择器降低为 [`crate::SelectionPlan`]，并将无法在
//! 编译期确定的边界、步长和随机数量标记为 Runtime 检查。模块不创建
//! 容器值；实际读取、写入和随机抽样由后续 Runtime/IR 消费计划完成。

use xiao_source::SourceSpan;
use xiao_syntax::{
    AssignmentOperator, Expression, PathSegment, RandomMode, Selector, SelectorItem,
};

use crate::containers::{ArrayType, ContainerPathSegment};
use crate::diagnostics::{
    CONTAINER_INDEX_OUT_OF_BOUNDS_CODE, CONTAINER_KEY_NOT_FOUND_CODE, INVALID_CONTAINER_PATH_CODE,
    RANDOM_SEED_ARITY_CODE, RANDOM_SEED_CODE, SELECTOR_ASSIGNMENT_CODE,
    SELECTOR_INVALID_RANDOM_COUNT_CODE, SELECTOR_INVALID_STEP_CODE, SELECTOR_RANDOM_EXHAUSTED_CODE,
    SELECTOR_UNORDERED_CONTAINER_CODE,
};
use crate::numeric::ConstantValue;
use crate::selection_model::{
    BroadcastAssignmentPlan, RandomSeedPlan, SelectionItemPlan, SelectionPath,
    SelectionPathSegment, SelectionPlan, StepPlan,
};
use crate::selection_shape::{direct_selection_children, project_selection_type};
use crate::types::Type;

use super::{RuntimeCheckKind, TypeChecker};

/// 一条已经根据来源类型检查过的路径。
struct ResolvedPath {
    /// 保留原始索引和静态解析位置的路径。
    path: SelectionPath,
    /// 路径叶子类型。
    ty: Type,
    /// 是否仍有段需要 Runtime 检查。
    dynamic: bool,
}

/// 一个选择项展开出的静态路径和状态。
struct ExpandedItem {
    /// 规范化选择项。
    plan: SelectionItemPlan,
    /// 已知可展开的路径。
    paths: Vec<SelectionPath>,
    /// 是否需要 Runtime 才能确定候选路径。
    dynamic: bool,
    /// 是否为随机选择。
    random: bool,
    /// 是否为放回随机选择。
    with_replacement: bool,
    /// 是否明确是零数量随机选择。
    zero_random: bool,
    /// 静态目标路径对应的叶子类型。
    target_types: Vec<Type>,
}

impl<'source> TypeChecker<'source> {
    /// 检查一个选择器表达式并生成共享选择计划。
    pub(super) fn check_advanced_selector(
        &mut self,
        source: &Expression,
        step: Option<&Expression>,
        selector: &Selector,
        span: SourceSpan,
    ) -> Type {
        let source_type = self.check_expression(source);
        let mut requires_runtime_check = false;
        let step_plan = self.check_step_expression(step, &mut requires_runtime_check);

        if !is_selector_source(&source_type) {
            self.selector_error(
                SELECTOR_UNORDERED_CONTAINER_CODE,
                "x03.type.selector_source_not_ordered",
                span,
                format!("类型 {} 不能使用有序选择器", source_type),
            );
            let result_type = Type::Dynamic;
            self.selection_plans.push(SelectionPlan {
                span,
                source_type,
                result_type: result_type.clone(),
                items: Vec::new(),
                selected_paths: Vec::new(),
                target_types: Vec::new(),
                step: step_plan,
                requires_runtime_check: false,
                with_replacement: false,
                has_duplicates: false,
            });
            return result_type;
        }
        if matches!(source_type, Type::DictTable(_))
            && (step.is_some()
                || selector.items.len() != 1
                || !matches!(selector.items.first(), Some(SelectorItem::Exact { .. })))
        {
            self.selector_error(
                SELECTOR_UNORDERED_CONTAINER_CODE,
                "x03.type.selector_unordered_advanced",
                span,
                "字典表只支持单个精确键路径，不能使用范围、多选或随机选择".to_string(),
            );
            let result_type = Type::Dynamic;
            self.selection_plans.push(SelectionPlan {
                span,
                source_type,
                result_type: result_type.clone(),
                items: Vec::new(),
                selected_paths: Vec::new(),
                target_types: Vec::new(),
                step: step_plan,
                requires_runtime_check: false,
                with_replacement: false,
                has_duplicates: false,
            });
            return result_type;
        }

        let mut expanded = Vec::with_capacity(selector.items.len());
        for item in &selector.items {
            expanded.push(self.expand_selector_item(
                &source_type,
                item,
                &mut requires_runtime_check,
            ));
        }

        let mut selected_paths = Vec::new();
        let mut target_types = Vec::new();
        let mut item_plans = Vec::with_capacity(expanded.len());
        let mut has_random = false;
        let mut with_replacement = false;
        let mut dynamic_item = false;
        let mut only_zero_random = true;
        let mut has_nonzero_random = false;
        for item in expanded {
            has_random |= item.random;
            with_replacement |= item.with_replacement;
            dynamic_item |= item.dynamic;
            has_nonzero_random |= item.random && !item.zero_random;
            if !item.random || !item.zero_random {
                only_zero_random = false;
            }
            let (item_paths, item_types) = if let Some(step) = step_plan.as_ref()
                && !step.dynamic
                && let Some(value) = step.value
                && value != 1
            {
                (
                    apply_global_step(item.paths, value),
                    apply_global_step(item.target_types, value),
                )
            } else {
                (item.paths, item.target_types)
            };
            selected_paths.extend(item_paths);
            target_types.extend(item_types);
            item_plans.push(item.plan);
        }
        requires_runtime_check |= dynamic_item;

        let all_static = !requires_runtime_check
            && selected_paths.iter().all(|path| {
                path.iter().all(|segment| {
                    segment.resolved_index_value().is_some() || segment.key_value().is_some()
                })
            });
        let result_type = self.selector_result_type(
            &source_type,
            &selected_paths,
            &target_types,
            selector,
            has_random,
            with_replacement,
            only_zero_random,
            has_nonzero_random,
            all_static,
        );
        let has_duplicates = has_duplicate_paths(&selected_paths)
            || (with_replacement
                && item_plans.iter().any(|item| {
                    matches!(
                        item,
                        SelectionItemPlan::Random {
                            count: Some(count), ..
                        } if *count > 1
                    )
                }));
        self.selection_plans.push(SelectionPlan {
            span,
            source_type,
            result_type: result_type.clone(),
            items: item_plans,
            selected_paths,
            target_types,
            step: step_plan,
            requires_runtime_check,
            with_replacement,
            has_duplicates,
        });
        result_type
    }

    /// 检查选择器出现在赋值左侧时的事务性标量广播规则。
    pub(super) fn check_selector_assignment(
        &mut self,
        target: &Expression,
        operator: AssignmentOperator,
        value: &Expression,
    ) {
        let target_diagnostic_start = self.diagnostics.len();
        self.check_expression(target);
        let value_type = self.check_expression(value);
        let target_has_errors = self.has_errors_since(target_diagnostic_start);
        // 目标读取阶段已经报告了未定义、未初始化、越界或路径错误。
        // 右值即使合法，也不能为失败目标创建写入计划。
        if target_has_errors {
            return;
        }
        if operator != AssignmentOperator::Assign {
            self.selector_error(
                SELECTOR_ASSIGNMENT_CODE,
                "x03.type.selector_assignment_operator",
                target.span(),
                "选择器左值当前只支持标量广播赋值".to_string(),
            );
            return;
        }
        if value_type.is_container() || matches!(value_type, Type::None) {
            self.selector_error(
                SELECTOR_ASSIGNMENT_CODE,
                "x03.type.selector_assignment_scalar_required",
                value.span(),
                format!("选择器广播右值必须是标量，实际为 {}", value_type),
            );
            return;
        }
        let Expression::Selector { source, .. } = target else {
            self.selector_error(
                SELECTOR_ASSIGNMENT_CODE,
                "x03.type.selector_assignment_target",
                target.span(),
                "选择器广播目标结构无效".to_string(),
            );
            return;
        };
        let Expression::Name(name) = source.as_ref() else {
            self.selector_error(
                SELECTOR_ASSIGNMENT_CODE,
                "x03.type.selector_assignment_root",
                source.span(),
                "选择器写入目前只支持直接名称作为根容器".to_string(),
            );
            return;
        };
        let key = self.name_key(*name);
        let Some(binding) = self.environment.lookup(&key).cloned() else {
            return;
        };
        let Some(plan) = self
            .selection_plans
            .iter()
            .rev()
            .find(|plan| plan.span == target.span())
            .cloned()
        else {
            self.selector_error(
                SELECTOR_ASSIGNMENT_CODE,
                "x03.type.selector_assignment_plan",
                target.span(),
                "无法建立选择器写入计划".to_string(),
            );
            return;
        };
        if plan.has_random() {
            self.selector_error(
                SELECTOR_ASSIGNMENT_CODE,
                "x03.type.selector_assignment_random",
                target.span(),
                "随机选择不能作为确定性赋值目标".to_string(),
            );
            return;
        }
        let mut valid = binding.initialized && binding.mutable;
        if !binding.initialized {
            valid = false;
        }
        if !binding.mutable {
            self.selector_error(
                SELECTOR_ASSIGNMENT_CODE,
                "x03.type.selector_assignment_immutable",
                source.span(),
                "不能写入不可变绑定".to_string(),
            );
            valid = false;
        }
        for target_element in plan.target_types() {
            if !crate::conversion::can_assign(&value_type, target_element) {
                self.selector_error(
                    crate::diagnostics::CONTAINER_TYPE_MISMATCH_CODE,
                    "x03.type.selector_assignment_type",
                    value.span(),
                    format!("广播值 {} 不符合目标元素 {}", value_type, target_element),
                );
                valid = false;
            }
        }
        if !valid {
            return;
        }
        if plan.requires_runtime_check {
            self.push_runtime_check(target.span(), RuntimeCheckKind::SelectorBounds);
        }
        if let Err(error) = self.environment.assign(&key) {
            self.environment_error(source.span(), error);
            return;
        }
        self.broadcast_assignment_plans
            .push(BroadcastAssignmentPlan {
                span: target.span(),
                root_name: Some(name.unquoted_text(self.source()).to_owned()),
                target_paths: plan.selected_paths,
                value_type,
                dynamic: plan.requires_runtime_check,
                transactional: true,
            });
    }

    /// 检查 `random.seed(value)` 并记录当前执行上下文的种子计划。
    pub(super) fn check_random_seed_call(
        &mut self,
        arguments: &[Expression],
        span: SourceSpan,
    ) -> Type {
        if arguments.len() != 1 {
            for argument in arguments {
                self.check_expression(argument);
            }
            self.selector_error(
                RANDOM_SEED_ARITY_CODE,
                "x03.type.random_seed_arity",
                span,
                "random.seed 必须接收一个参数".to_string(),
            );
            return Type::None;
        }
        let argument = &arguments[0];
        let argument_type = self.check_expression(argument);
        let mut dynamic = false;
        let value = if argument_type.is_dynamic() {
            dynamic = true;
            self.push_runtime_check(argument.span(), RuntimeCheckKind::RandomSeed);
            None
        } else if !argument_type.is_integer() {
            self.selector_error(
                RANDOM_SEED_CODE,
                "x03.type.random_seed_integer",
                argument.span(),
                "random.seed 的参数必须是非负 lint 整数".to_string(),
            );
            None
        } else {
            match self.eval_const(argument) {
                Some(ConstantValue::Integer(value)) if value >= 0 => {
                    u128::try_from(value).ok().or_else(|| {
                        self.selector_error(
                            RANDOM_SEED_CODE,
                            "x03.type.random_seed_range",
                            argument.span(),
                            "random.seed 的值超出 lint 种子范围".to_string(),
                        );
                        None
                    })
                }
                Some(ConstantValue::BigInteger(value)) => match value.parse::<u128>() {
                    Ok(value) => Some(value),
                    Err(_) => {
                        self.selector_error(
                            RANDOM_SEED_CODE,
                            "x03.type.random_seed_range",
                            argument.span(),
                            "random.seed 的值超出可用种子范围".to_string(),
                        );
                        None
                    }
                },
                Some(ConstantValue::Integer(_)) => {
                    self.selector_error(
                        RANDOM_SEED_CODE,
                        "x03.type.random_seed_non_negative",
                        argument.span(),
                        "random.seed 不接受负数".to_string(),
                    );
                    None
                }
                Some(_) => {
                    self.selector_error(
                        RANDOM_SEED_CODE,
                        "x03.type.random_seed_integer",
                        argument.span(),
                        "random.seed 的参数必须是整数".to_string(),
                    );
                    None
                }
                None => {
                    dynamic = true;
                    self.push_runtime_check(argument.span(), RuntimeCheckKind::RandomSeed);
                    None
                }
            }
        };
        self.random_seed_plans.push(RandomSeedPlan {
            span,
            value,
            dynamic,
        });
        Type::None
    }

    /// 检查可选步长，并区分静态错误和 Runtime 检查。
    fn check_step_expression(
        &mut self,
        step: Option<&Expression>,
        requires_runtime_check: &mut bool,
    ) -> Option<StepPlan> {
        let step = step?;
        let step_type = self.check_expression(step);
        if step_type.is_dynamic() {
            *requires_runtime_check = true;
            self.push_runtime_check(step.span(), RuntimeCheckKind::SelectorStep);
            return Some(StepPlan::dynamic());
        }
        if !step_type.is_integer() {
            self.selector_error(
                SELECTOR_INVALID_STEP_CODE,
                "x03.type.selector_step_integer",
                step.span(),
                "选择器步长必须是整数".to_string(),
            );
            return None;
        }
        let Some(value) = self.eval_const(step).and_then(constant_integer) else {
            *requires_runtime_check = true;
            self.push_runtime_check(step.span(), RuntimeCheckKind::SelectorStep);
            return Some(StepPlan::dynamic());
        };
        if value == 0 {
            self.selector_error(
                SELECTOR_INVALID_STEP_CODE,
                "x03.type.selector_step_zero",
                step.span(),
                "选择器步长不能为 0".to_string(),
            );
            return None;
        }
        Some(StepPlan::known(value))
    }

    /// 展开一个语法选择项并执行静态边界检查。
    fn expand_selector_item(
        &mut self,
        source_type: &Type,
        item: &SelectorItem,
        requires_runtime_check: &mut bool,
    ) -> ExpandedItem {
        match item {
            SelectorItem::Exact { path, .. } => {
                let lowered = self.lower_selector_path(path);
                let resolved = lowered.as_ref().and_then(|lowered| {
                    self.resolve_selector_path(source_type, lowered, path.span())
                });
                let (paths, dynamic, target_types) =
                    resolved.map_or((Vec::new(), false, Vec::new()), |resolved| {
                        if resolved.dynamic {
                            *requires_runtime_check = true;
                            self.push_runtime_check(path.span(), RuntimeCheckKind::SelectorBounds);
                            (Vec::new(), true, Vec::new())
                        } else {
                            (vec![resolved.path], false, vec![resolved.ty])
                        }
                    });
                ExpandedItem {
                    plan: SelectionItemPlan::Exact {
                        path: lowered.unwrap_or_default(),
                    },
                    paths,
                    dynamic,
                    random: false,
                    with_replacement: false,
                    zero_random: false,
                    target_types,
                }
            }
            SelectorItem::Range { start, end, .. } => self.expand_range_item(
                source_type,
                Some(start),
                Some(end),
                true,
                true,
                requires_runtime_check,
            ),
            SelectorItem::OpenRange {
                start,
                end,
                include_start,
                include_end,
                ..
            } => self.expand_range_item(
                source_type,
                start.as_ref(),
                end.as_ref(),
                *include_start,
                *include_end,
                requires_runtime_check,
            ),
            SelectorItem::All { .. } => {
                let children = direct_selection_children(source_type);
                if children.is_none() {
                    self.push_runtime_check(item.span(), RuntimeCheckKind::SelectorBounds);
                }
                let paths = children.as_ref().map_or_else(
                    || {
                        *requires_runtime_check = true;
                        Vec::new()
                    },
                    |children| {
                        children
                            .iter()
                            .map(|(segment, _)| match segment {
                                ContainerPathSegment::Index(index) => {
                                    vec![SelectionPathSegment::resolved_index(
                                        *index as i128,
                                        *index,
                                    )]
                                }
                                ContainerPathSegment::Key(key) => {
                                    vec![SelectionPathSegment::key(key.clone())]
                                }
                            })
                            .collect()
                    },
                );
                ExpandedItem {
                    plan: SelectionItemPlan::All,
                    paths,
                    dynamic: direct_selection_children(source_type).is_none(),
                    random: false,
                    with_replacement: false,
                    zero_random: false,
                    target_types: children
                        .map(|children| children.into_iter().map(|(_, ty)| ty).collect())
                        .unwrap_or_default(),
                }
            }
            SelectorItem::Random { mode, count, .. } => {
                let count_type = self.check_expression(count);
                let mut dynamic_count = false;
                let mut static_count = None;
                if count_type.is_dynamic() {
                    dynamic_count = true;
                } else if !count_type.is_integer() {
                    self.selector_error(
                        SELECTOR_INVALID_RANDOM_COUNT_CODE,
                        "x03.type.random_count_integer",
                        count.span(),
                        "随机抽取数量必须是整数".to_string(),
                    );
                } else if let Some(value) = self.eval_const(count).and_then(constant_integer) {
                    if value < 0 {
                        self.selector_error(
                            SELECTOR_INVALID_RANDOM_COUNT_CODE,
                            "x03.type.random_count_negative",
                            count.span(),
                            "随机抽取数量不能为负数".to_string(),
                        );
                    } else if let Ok(value) = usize::try_from(value as u128) {
                        static_count = Some(value);
                        if let Some(available) =
                            direct_selection_children(source_type).map(|v| v.len())
                        {
                            if *mode == RandomMode::WithoutReplacement && value > available {
                                self.selector_error(
                                    SELECTOR_RANDOM_EXHAUSTED_CODE,
                                    "x03.type.random_without_replacement_exhausted",
                                    count.span(),
                                    format!("无放回抽取数量 {} 超过候选数 {}", value, available),
                                );
                            }
                            if available == 0 && value > 0 {
                                self.selector_error(
                                    SELECTOR_RANDOM_EXHAUSTED_CODE,
                                    "x03.type.random_empty_source",
                                    count.span(),
                                    "不能从空容器中抽取元素".to_string(),
                                );
                            }
                        } else if value > 0 {
                            dynamic_count = true;
                        }
                    } else {
                        self.selector_error(
                            SELECTOR_INVALID_RANDOM_COUNT_CODE,
                            "x03.type.random_count_range",
                            count.span(),
                            "随机抽取数量超出平台可表示范围".to_string(),
                        );
                    }
                } else {
                    dynamic_count = true;
                }
                if dynamic_count {
                    *requires_runtime_check = true;
                    self.push_runtime_check(count.span(), RuntimeCheckKind::RandomCount);
                }
                let zero_random = static_count == Some(0);
                ExpandedItem {
                    plan: SelectionItemPlan::Random {
                        mode: *mode,
                        count: static_count,
                        dynamic_count,
                    },
                    paths: Vec::new(),
                    dynamic: dynamic_count || static_count.is_none(),
                    random: true,
                    with_replacement: *mode == RandomMode::WithReplacement,
                    zero_random,
                    target_types: Vec::new(),
                }
            }
        }
    }

    /// 展开闭区间或单边范围；未知长度时保留 Runtime 边界标记。
    fn expand_range_item(
        &mut self,
        source_type: &Type,
        start: Option<&xiao_syntax::IndexPath>,
        end: Option<&xiao_syntax::IndexPath>,
        include_start: bool,
        include_end: bool,
        requires_runtime_check: &mut bool,
    ) -> ExpandedItem {
        let lowered_start = start.and_then(|path| self.lower_selector_path(path));
        let lowered_end = end.and_then(|path| self.lower_selector_path(path));
        let start_conversion_failed = start.is_some() && lowered_start.is_none();
        let end_conversion_failed = end.is_some() && lowered_end.is_none();
        let start_resolved = lowered_start.as_ref().and_then(|lowered| {
            self.resolve_selector_path(
                source_type,
                lowered,
                start.map_or_else(|| end.expect("范围端点存在").span(), |path| path.span()),
            )
        });
        let end_resolved = lowered_end.as_ref().and_then(|lowered| {
            self.resolve_selector_path(
                source_type,
                lowered,
                end.map_or_else(|| start.expect("范围端点存在").span(), |path| path.span()),
            )
        });
        let mut invalid_endpoint = start_conversion_failed
            || end_conversion_failed
            || (lowered_start.is_some() && start_resolved.is_none())
            || (lowered_end.is_some() && end_resolved.is_none());
        if !invalid_endpoint
            && [start_resolved.as_ref(), end_resolved.as_ref()]
                .into_iter()
                .flatten()
                .any(|resolved| path_crosses_unordered_container(source_type, &resolved.path))
        {
            self.selector_error(
                SELECTOR_UNORDERED_CONTAINER_CODE,
                "x03.type.selector_range_unordered_path",
                start.or(end).map_or_else(
                    || SourceSpan::new(0, 0).expect("零宽诊断区间"),
                    |path| path.span(),
                ),
                "范围端点不能穿过无序字典表".to_string(),
            );
            invalid_endpoint = true;
        }
        let dynamic = !invalid_endpoint
            && (start_resolved.as_ref().is_some_and(|path| path.dynamic)
                || end_resolved.as_ref().is_some_and(|path| path.dynamic)
                || direct_selection_children(source_type).is_none());
        if dynamic {
            *requires_runtime_check = true;
            if let Some(path) = start.or(end) {
                self.push_runtime_check(path.span(), RuntimeCheckKind::SelectorBounds);
            }
        }
        let paths = if dynamic || invalid_endpoint {
            Vec::new()
        } else {
            let (resolved_start, resolved_end) = (
                start_resolved.as_ref().map(|path| &path.path),
                end_resolved.as_ref().map(|path| &path.path),
            );
            match expand_direct_range(
                source_type,
                resolved_start,
                resolved_end,
                include_start,
                include_end,
            ) {
                Some(paths) => paths,
                None => {
                    *requires_runtime_check = true;
                    self.push_runtime_check(
                        start.or(end).map_or_else(
                            || SourceSpan::new(0, 0).expect("零宽诊断区间"),
                            |path| path.span(),
                        ),
                        RuntimeCheckKind::SelectorBounds,
                    );
                    Vec::new()
                }
            }
        };
        let target_types = paths
            .iter()
            .filter_map(|path| {
                self.resolve_selector_path(
                    source_type,
                    path,
                    SourceSpan::new(0, 0).expect("零宽诊断区间"),
                )
                .filter(|resolved| !resolved.dynamic)
                .map(|resolved| resolved.ty)
            })
            .collect();
        let plan = match (lowered_start, lowered_end) {
            (Some(start), Some(end)) => SelectionItemPlan::Range {
                start,
                end,
                include_start,
                include_end,
            },
            (start, end) => SelectionItemPlan::Range {
                start: start.unwrap_or_default(),
                end: end.unwrap_or_default(),
                include_start,
                include_end,
            },
        };
        ExpandedItem {
            plan,
            paths,
            dynamic,
            random: false,
            with_replacement: false,
            zero_random: false,
            target_types,
        }
    }

    /// 降低选择器路径，同时保留负索引的有符号值。
    fn lower_selector_path(&mut self, path: &xiao_syntax::IndexPath) -> Option<SelectionPath> {
        let mut output = Vec::with_capacity(path.segments.len());
        for segment in &path.segments {
            match segment {
                PathSegment::Integer { span, negative } => {
                    let text = self.source().slice(*span);
                    let digits = text.strip_prefix('-').unwrap_or(text);
                    let magnitude = match digits.parse::<i128>() {
                        Ok(value) => value,
                        Err(_) => {
                            self.selector_error(
                                INVALID_CONTAINER_PATH_CODE,
                                "x03.type.selector_invalid_index",
                                *span,
                                "选择器索引不是可表示的整数".to_string(),
                            );
                            return None;
                        }
                    };
                    let raw = if *negative {
                        match magnitude.checked_neg() {
                            Some(value) => value,
                            None => {
                                self.selector_error(
                                    INVALID_CONTAINER_PATH_CODE,
                                    "x03.type.selector_invalid_index",
                                    *span,
                                    "选择器负索引超出范围".to_string(),
                                );
                                return None;
                            }
                        }
                    } else {
                        magnitude
                    };
                    output.push(SelectionPathSegment::index(raw));
                }
                PathSegment::Name(name) => output.push(SelectionPathSegment::key(
                    name.unquoted_text(self.source()).to_owned(),
                )),
            }
        }
        Some(output)
    }

    /// 在来源类型上解析一条选择路径，并报告静态错误。
    fn resolve_selector_path(
        &mut self,
        root: &Type,
        path: &SelectionPath,
        diagnostic_span: SourceSpan,
    ) -> Option<ResolvedPath> {
        let mut current = root.clone();
        let mut output = Vec::with_capacity(path.len());
        let mut dynamic = false;
        for (segment, part) in path.iter().enumerate() {
            match &current {
                Type::Dynamic | Type::Variable(_) => {
                    dynamic = true;
                    output.push(part.clone());
                    current = Type::Dynamic;
                }
                Type::Array(array) => {
                    let SelectionPathSegment::Index { raw, .. } = part else {
                        self.path_kind_error(part, segment, diagnostic_span);
                        return None;
                    };
                    let (normalized, ty) = match resolve_array_index(array, *raw) {
                        Ok(Some((index, ty))) => (Some(index), ty),
                        Ok(None) => {
                            dynamic = true;
                            (None, array_element_type(array))
                        }
                        Err(length) => {
                            self.selector_error(
                                CONTAINER_INDEX_OUT_OF_BOUNDS_CODE,
                                "x03.type.index_out_of_bounds",
                                diagnostic_span,
                                format!("选择器索引超出容器长度 {}", length),
                            );
                            return None;
                        }
                    };
                    output.push(match normalized {
                        Some(index) => SelectionPathSegment::resolved_index(*raw, index),
                        None => part.clone(),
                    });
                    current = ty;
                }
                Type::Tuple(elements) => {
                    let SelectionPathSegment::Index { raw, .. } = part else {
                        self.path_kind_error(part, segment, diagnostic_span);
                        return None;
                    };
                    let Some(index) = normalize_index(*raw, elements.len()) else {
                        self.selector_error(
                            CONTAINER_INDEX_OUT_OF_BOUNDS_CODE,
                            "x03.type.index_out_of_bounds",
                            diagnostic_span,
                            format!("选择器索引 {} 超出元组长度 {}", raw, elements.len()),
                        );
                        return None;
                    };
                    output.push(SelectionPathSegment::resolved_index(*raw, index));
                    current = elements[index].clone();
                }
                Type::Scalar(xiao_syntax::ScalarType::Str) => {
                    let SelectionPathSegment::Index { raw, .. } = part else {
                        self.path_kind_error(part, segment, diagnostic_span);
                        return None;
                    };
                    dynamic = true;
                    output.push(part.clone());
                    let _ = raw;
                    current = Type::scalar(xiao_syntax::ScalarType::Str);
                }
                Type::DictTable(dictionary) => {
                    let SelectionPathSegment::Key(key) = part else {
                        self.path_kind_error(part, segment, diagnostic_span);
                        return None;
                    };
                    let Some(value) = dictionary.value_type(key) else {
                        self.selector_error(
                            CONTAINER_KEY_NOT_FOUND_CODE,
                            "x03.type.key_not_found",
                            diagnostic_span,
                            format!("字典表中不存在键 {}", key),
                        );
                        return None;
                    };
                    output.push(part.clone());
                    current = value.clone();
                }
                Type::DictColumn(dictionary) => {
                    let (index, value) = match part {
                        SelectionPathSegment::Key(key) => {
                            let Some(index) = dictionary
                                .entries
                                .iter()
                                .position(|entry| entry.key == *key)
                            else {
                                self.selector_error(
                                    CONTAINER_KEY_NOT_FOUND_CODE,
                                    "x03.type.key_not_found",
                                    diagnostic_span,
                                    format!("字典列中不存在键 {}", key),
                                );
                                return None;
                            };
                            (index, dictionary.entries[index].value.as_ref().clone())
                        }
                        SelectionPathSegment::Index { raw, .. } => {
                            let Some(index) = normalize_index(*raw, dictionary.entries.len())
                            else {
                                self.selector_error(
                                    CONTAINER_INDEX_OUT_OF_BOUNDS_CODE,
                                    "x03.type.index_out_of_bounds",
                                    diagnostic_span,
                                    format!(
                                        "字典列索引 {} 超出长度 {}",
                                        raw,
                                        dictionary.entries.len()
                                    ),
                                );
                                return None;
                            };
                            (index, dictionary.entries[index].value.as_ref().clone())
                        }
                    };
                    if matches!(part, SelectionPathSegment::Key(_)) {
                        output.push(part.clone());
                    } else if let SelectionPathSegment::Index { raw, .. } = part {
                        output.push(SelectionPathSegment::resolved_index(*raw, index));
                    }
                    current = value;
                }
                Type::Scalar(_) | Type::None | Type::Function { .. } => {
                    self.path_kind_error(part, segment, diagnostic_span);
                    return None;
                }
            }
        }
        Some(ResolvedPath {
            path: output,
            ty: current,
            dynamic,
        })
    }

    /// 根据静态展开情况决定选择表达式结果类型。
    #[allow(clippy::too_many_arguments)]
    fn selector_result_type(
        &self,
        source: &Type,
        paths: &[SelectionPath],
        target_types: &[Type],
        selector: &Selector,
        has_random: bool,
        with_replacement: bool,
        only_zero_random: bool,
        has_nonzero_random: bool,
        all_static: bool,
    ) -> Type {
        if selector.items.len() == 1
            && matches!(selector.items.first(), Some(SelectorItem::Exact { .. }))
            && paths.len() == 1
            && all_static
        {
            return target_types.first().cloned().unwrap_or(Type::Dynamic);
        }
        if has_random && only_zero_random {
            return crate::selection_shape::empty_selection_type(source);
        }
        // 非零随机项的具体位置只有 Runtime 才能知道。不能因为当前
        // 计划没有静态路径就把结果投影成空容器；来源类型是保守的
        // 静态上界。字典列的放回模式需要避免伪造重复键。
        if has_nonzero_random {
            if with_replacement && matches!(source, Type::DictColumn(_)) {
                let mut tuple_types = target_types.to_vec();
                for item in selector.items.iter().filter_map(|item| {
                    if let SelectorItem::Random {
                        mode: RandomMode::WithReplacement,
                        count,
                        ..
                    } = item
                    {
                        Some(count)
                    } else {
                        None
                    }
                }) {
                    let count = self
                        .eval_const(item)
                        .and_then(constant_integer)
                        .and_then(|value| usize::try_from(value).ok())
                        .unwrap_or(1);
                    tuple_types.extend(std::iter::repeat_n(Type::Dynamic, count));
                }
                return Type::Tuple(tuple_types);
            }
            return source.clone();
        }
        if all_static {
            return project_selection_type(source, paths, false);
        }
        if is_selector_source(source) {
            source.clone()
        } else {
            Type::Dynamic
        }
    }

    /// 将路径段种类错误映射为稳定诊断。
    fn path_kind_error(&mut self, part: &SelectionPathSegment, segment: usize, span: SourceSpan) {
        self.selector_error(
            INVALID_CONTAINER_PATH_CODE,
            "x03.type.invalid_path_segment",
            span,
            format!("第 {} 段选择路径 {:?} 与容器种类不匹配", segment, part),
        );
    }

    /// 写入一条选择器诊断。
    fn selector_error(
        &mut self,
        code: &'static str,
        message_id: &'static str,
        span: SourceSpan,
        message: String,
    ) {
        self.type_error(code, message_id, span, message);
    }
}

/// 判断类型是否允许使用选择器。
fn is_selector_source(source: &Type) -> bool {
    matches!(
        source,
        Type::Array(_)
            | Type::Tuple(_)
            | Type::DictColumn(_)
            | Type::Scalar(xiao_syntax::ScalarType::Str)
            | Type::Dynamic
            | Type::Variable(_)
            | Type::DictTable(_)
    )
}

/// 将常量值转换为有符号整数。
fn constant_integer(value: ConstantValue) -> Option<i128> {
    match value {
        ConstantValue::Integer(value) => Some(value),
        ConstantValue::BigInteger(value) => value.parse::<i128>().ok(),
        _ => None,
    }
}

/// 规范化 Python 风格负索引。
fn normalize_index(raw: i128, length: usize) -> Option<usize> {
    let index = if raw < 0 {
        (length as i128).checked_add(raw)?
    } else {
        raw
    };
    usize::try_from(index).ok().filter(|index| *index < length)
}

/// 读取数组索引的静态结果；未知长度返回 `Ok(None)`。
fn resolve_array_index(array: &ArrayType, raw: i128) -> Result<Option<(usize, Type)>, usize> {
    match array {
        ArrayType::Homogeneous { element, length } => {
            let Some(length) = length else {
                return Ok(None);
            };
            let Some(index) = normalize_index(raw, *length) else {
                return Err(*length);
            };
            Ok(Some((index, element.as_ref().clone())))
        }
        ArrayType::Heterogeneous { elements } => {
            let Some(index) = normalize_index(raw, elements.len()) else {
                return Err(elements.len());
            };
            Ok(Some((index, elements[index].clone())))
        }
        ArrayType::Unknown => Ok(None),
    }
}

/// 返回未知长度数组的元素类型。
fn array_element_type(array: &ArrayType) -> Type {
    match array {
        ArrayType::Homogeneous { element, .. } => element.as_ref().clone(),
        ArrayType::Heterogeneous { .. } | ArrayType::Unknown => Type::Dynamic,
    }
}

/// 按有序路径的深度优先顺序展开范围。
///
/// 范围端点可以落在嵌套容器中。先枚举可静态知道的节点，再按端点
/// 的规范化路径过滤，并移除同时命中的父节点；这样 `1~2/1` 会被
/// 表示为 `1`、`2/0`、`2/1`，结果投影阶段可以恢复嵌套形状。
fn expand_direct_range(
    source: &Type,
    start: Option<&SelectionPath>,
    end: Option<&SelectionPath>,
    include_start: bool,
    include_end: bool,
) -> Option<Vec<SelectionPath>> {
    let nodes = enumerate_ordered_nodes(source)?;
    let start_order = start.and_then(|path| canonical_order_key(source, path));
    let end_order = end.and_then(|path| canonical_order_key(source, path));
    let descending = match (&start_order, &end_order) {
        (Some(start), Some(end)) => start > end,
        _ => false,
    };
    let (lower, lower_inclusive, upper, upper_inclusive) = if descending {
        (
            end_order.as_deref(),
            include_end,
            start_order.as_deref(),
            include_start,
        )
    } else {
        (
            start_order.as_deref(),
            include_start,
            end_order.as_deref(),
            include_end,
        )
    };
    let mut selected = nodes
        .iter()
        .filter(|(order, _)| {
            let after_lower = lower.is_none_or(|bound| {
                order.as_slice() > bound || (lower_inclusive && order.as_slice() == bound)
            });
            let before_upper = upper.is_none_or(|bound| {
                order.as_slice() < bound || (upper_inclusive && order.as_slice() == bound)
            });
            after_lower && before_upper
        })
        .map(|(_, path)| path.clone())
        .collect::<Vec<_>>();
    if descending {
        selected.reverse();
    }
    // 只有被范围端点真正切入的分支才展开到子节点。中间分支和
    // 端点恰好落在父节点上的分支应作为完整容器保留；否则 `<2`
    // 会错误地把索引 1 的嵌套数组拆成若干元素。
    let selected_snapshot = selected.clone();
    let boundary_orders = [start_order.as_deref(), end_order.as_deref()];
    selected.retain(|path| {
        let path_order = path
            .iter()
            .filter_map(SelectionPathSegment::resolved_index_value)
            .collect::<Vec<_>>();
        // 当前节点本身是被端点切入的容器时，只保留其命中的
        // 子节点；端点恰好等于当前节点不属于“切入”。
        // 排他下界的整个子树都位于“下界之后”。如果下界落在一个
        // 容器节点上，简单的字典序过滤会错误地把该节点的子项纳入
        // `>boundary`，所以必须把这些后代一起排除。
        if lower.is_some_and(|bound| !lower_inclusive && is_strict_order_prefix(bound, &path_order))
        {
            return false;
        }
        let boundary_inside = boundary_orders.iter().flatten().any(|boundary| {
            is_order_prefix(&path_order, boundary) && path_order.len() < boundary.len()
        });
        if boundary_inside {
            return false;
        }
        !selected_snapshot.iter().any(|ancestor| {
            if !is_strict_path_prefix(ancestor, path) {
                return false;
            }
            let ancestor_order = ancestor
                .iter()
                .filter_map(SelectionPathSegment::resolved_index_value)
                .collect::<Vec<_>>();
            !boundary_orders.iter().flatten().any(|boundary| {
                is_order_prefix(&ancestor_order, boundary) && ancestor_order.len() < boundary.len()
            })
        })
    });
    Some(selected)
}

/// 一个可静态枚举的有序节点及其规范化顺序键。
type OrderedNode = (Vec<usize>, SelectionPath);

/// 枚举来源根的所有可静态知道的节点。
fn enumerate_ordered_nodes(source: &Type) -> Option<Vec<OrderedNode>> {
    direct_selection_children(source)?;
    let mut output = Vec::new();
    enumerate_ordered_children(source, &[], &mut output);
    Some(output)
}

/// 递归枚举一个已知长度的有序容器；未知子容器作为整体节点保留。
fn enumerate_ordered_children(
    source: &Type,
    prefix: &[SelectionPathSegment],
    output: &mut Vec<OrderedNode>,
) {
    let Some(children) = direct_selection_children(source) else {
        return;
    };
    for (index, (_, child_type)) in children.iter().enumerate() {
        let mut path = prefix.to_vec();
        path.push(SelectionPathSegment::resolved_index(index as i128, index));
        let mut order = prefix
            .iter()
            .filter_map(SelectionPathSegment::resolved_index_value)
            .collect::<Vec<_>>();
        order.push(index);
        output.push((order, path.clone()));
        if is_recursively_ordered(child_type) {
            enumerate_ordered_children(child_type, &path, output);
        }
    }
}

/// 判断一个节点是否值得继续枚举其子项。
fn is_recursively_ordered(source: &Type) -> bool {
    matches!(
        source,
        Type::Array(ArrayType::Heterogeneous { .. })
            | Type::Array(ArrayType::Homogeneous {
                length: Some(_),
                ..
            })
            | Type::Tuple(_)
            | Type::DictColumn(_)
    )
}

/// 把一条已解析路径转换成按容器位置排列的顺序键。
fn canonical_order_key(source: &Type, path: &SelectionPath) -> Option<Vec<usize>> {
    let mut current = source;
    let mut order = Vec::with_capacity(path.len());
    for segment in path {
        match current {
            Type::Array(array) => {
                if array.length().is_none()
                    && matches!(array, ArrayType::Homogeneous { .. } | ArrayType::Unknown)
                {
                    return None;
                }
                let SelectionPathSegment::Index {
                    resolved: Some(index),
                    ..
                } = segment
                else {
                    return None;
                };
                let child = array.element_at(*index)?;
                order.push(*index);
                current = child;
            }
            Type::Tuple(elements) => {
                let SelectionPathSegment::Index {
                    resolved: Some(index),
                    ..
                } = segment
                else {
                    return None;
                };
                current = elements.get(*index)?;
                order.push(*index);
            }
            Type::DictColumn(dictionary) => {
                let index = match segment {
                    SelectionPathSegment::Index {
                        resolved: Some(index),
                        ..
                    } => *index,
                    SelectionPathSegment::Key(key) => dictionary
                        .entries
                        .iter()
                        .position(|entry| entry.key == *key)?,
                    SelectionPathSegment::Index { resolved: None, .. } => return None,
                };
                current = dictionary.entries.get(index)?.value.as_ref();
                order.push(index);
            }
            // 字符串字符数在类型阶段未知；动态路径不会进入静态范围展开。
            Type::Dynamic
            | Type::Variable(_)
            | Type::DictTable(_)
            | Type::Scalar(_)
            | Type::None
            | Type::Function { .. } => return None,
        }
    }
    Some(order)
}

/// 判断路径是否真正进入了无序字典表。
fn path_crosses_unordered_container(source: &Type, path: &SelectionPath) -> bool {
    let mut current = source;
    for segment in path {
        if matches!(current, Type::DictTable(_)) {
            return true;
        }
        current = match (current, segment) {
            (
                Type::Array(array),
                SelectionPathSegment::Index {
                    resolved: Some(index),
                    ..
                },
            ) => match array.element_at(*index) {
                Some(child) => child,
                None => return false,
            },
            (
                Type::Tuple(elements),
                SelectionPathSegment::Index {
                    resolved: Some(index),
                    ..
                },
            ) => match elements.get(*index) {
                Some(child) => child,
                None => return false,
            },
            (Type::DictColumn(dictionary), segment) => {
                let index = match segment {
                    SelectionPathSegment::Index {
                        resolved: Some(index),
                        ..
                    } => *index,
                    SelectionPathSegment::Key(key) => match dictionary
                        .entries
                        .iter()
                        .position(|entry| entry.key == *key)
                    {
                        Some(index) => index,
                        None => return false,
                    },
                    SelectionPathSegment::Index { resolved: None, .. } => return false,
                };
                match dictionary.entries.get(index) {
                    Some(entry) => entry.value.as_ref(),
                    None => return false,
                }
            }
            _ => return false,
        };
    }
    false
}

/// 判断一个路径是否为另一路径的严格前缀。
fn is_strict_path_prefix(prefix: &SelectionPath, path: &SelectionPath) -> bool {
    prefix.len() < path.len()
        && prefix.iter().zip(path).all(|(left, right)| {
            left.resolved_index_value() == right.resolved_index_value()
                && left.key_value() == right.key_value()
        })
}

/// 判断一个顺序键是否是另一个顺序键的前缀。
fn is_order_prefix(prefix: &[usize], order: &[usize]) -> bool {
    prefix.len() <= order.len() && prefix.iter().zip(order).all(|(left, right)| left == right)
}

/// 判断一个顺序键是否是另一个顺序键的严格前缀。
fn is_strict_order_prefix(prefix: &[usize], order: &[usize]) -> bool {
    prefix.len() < order.len() && is_order_prefix(prefix, order)
}

/// 对一组已经合并的路径应用静态步长。
fn apply_global_step<T>(items: Vec<T>, step: i128) -> Vec<T> {
    if step == 1 || items.is_empty() {
        return items;
    }
    let distance = usize::try_from(step.unsigned_abs()).unwrap_or(usize::MAX);
    if distance == 0 {
        return items;
    }
    if step > 0 {
        items.into_iter().step_by(distance).collect()
    } else {
        items.into_iter().rev().step_by(distance).collect()
    }
}

/// 判断路径序列中是否保留了重复选择。
fn has_duplicate_paths(paths: &[SelectionPath]) -> bool {
    paths
        .iter()
        .enumerate()
        .any(|(index, path)| paths[..index].contains(path))
}
