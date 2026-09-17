//! C0 容器表达式、声明和精确选择器检查。
//!
//! 该文件是 `checker.rs` 的语义边界：父模块负责语句分派和标量规则，
//! 本模块只负责递归容器形状、字典键唯一性、声明路径以及 C0 允许的单项
//! 精确索引。这样后续集合、范围选择和 Runtime 写操作可以在独立阶段接入。

use std::collections::BTreeSet;

use xiao_source::SourceSpan;
use xiao_syntax::{DictEntry, DictKey, Expression, IndexPath, PathSegment, ScalarType, Selector};

use crate::containers::{
    ArrayType, ContainerPathSegment, DictEntryType, DictType, PathConstraintTree,
};
use crate::conversion::can_assign;
use crate::diagnostics::{
    CONTAINER_INDEX_OUT_OF_BOUNDS_CODE, CONTAINER_KEY_NOT_FOUND_CODE, CONTAINER_TYPE_MISMATCH_CODE,
    DUPLICATE_CONTAINER_KEY_CODE, INVALID_CONTAINER_PATH_CODE, INVALID_DECLARATION_PATH_CODE,
};
use crate::materialization::{build_plan, merge_constraints};
use crate::path_constraints::{
    PathConversionError, PathResolutionError, PathResolutionErrorKind, lower_index_path,
    resolve_exact_path,
};
use crate::types::Type;

use super::{RuntimeCheckKind, TypeChecker};

impl<'source> TypeChecker<'source> {
    /// 验证一次整体容器赋值是否满足绑定上已经登记的路径约束。
    pub(super) fn check_container_assignment_constraints(
        &mut self,
        root: &Type,
        constraints: &PathConstraintTree,
        span: SourceSpan,
    ) {
        self.validate_existing_container_constraints(root, constraints, span);
    }

    /// 检查 C0 容器字面量，并将 C2-A 集合交给独立集合模块。
    pub(super) fn check_container_expression(&mut self, expression: &Expression) -> Type {
        match expression {
            Expression::ArrayLiteral { elements, .. } => {
                let types = elements
                    .iter()
                    .map(|element| self.check_expression(element))
                    .collect::<Vec<_>>();
                if types.is_empty() {
                    Type::Array(ArrayType::Unknown)
                } else {
                    Type::Array(ArrayType::heterogeneous(types))
                }
            }
            Expression::TupleLiteral { elements, .. } => Type::Tuple(
                elements
                    .iter()
                    .map(|element| self.check_expression(element))
                    .collect(),
            ),
            Expression::DictTableLiteral { entries, .. } => {
                Type::DictTable(self.check_dictionary_entries(entries))
            }
            Expression::DictColumnLiteral { entries, .. } => {
                Type::DictColumn(self.check_dictionary_entries(entries))
            }
            Expression::SetLiteral { elements, span } => self.check_set_literal(elements, *span),
            _ => Type::Dynamic,
        }
    }

    /// 尝试检查带标量前缀的容器声明；返回 `false` 表示应由标量路径处理。
    pub(super) fn try_check_container_declaration(
        &mut self,
        target: xiao_syntax::Name,
        declared_type: ScalarType,
        constraint_path: Option<&IndexPath>,
        value: Option<&Expression>,
    ) -> bool {
        let has_container_initializer = value.is_some_and(|expression| {
            is_container_expression(expression) || self.is_set_constructor(expression)
        });
        if constraint_path.is_none() && !has_container_initializer {
            return false;
        }

        let key = self.name_key(target);
        let value_type = value.map(|expression| self.check_expression(expression));
        let expected = Type::scalar(declared_type);
        let mut constraints = PathConstraintTree::new();

        let lowered_path = if let Some(path) = constraint_path {
            match lower_index_path(self.source(), path) {
                Ok(path) => Some(path),
                Err(error) => {
                    self.report_path_conversion_error(path, error);
                    None
                }
            }
        } else {
            None
        };

        let root_type = if let Some(path) = lowered_path.as_ref() {
            constraints.insert(path.clone(), expected.clone());
            self.check_path_constrained_declaration(
                constraint_path.expect("lowered path has source path"),
                value_type.as_ref(),
                path,
            )
        } else if constraint_path.is_some() {
            Some(Type::Array(ArrayType::Unknown))
        } else {
            self.check_homogeneous_array_declaration(
                target,
                declared_type,
                value_type.as_ref(),
                &mut constraints,
            )
        };

        let Some(root_type) = root_type else {
            // 即使初始化器不是容器，也保留一个动态绑定，避免后续名称读取
            // 产生一串与首个容器诊断无关的未定义错误。
            self.declare_container_binding(
                target,
                key,
                Type::Dynamic,
                value.is_some(),
                constraints,
            );
            return true;
        };

        let binding_name = target.unquoted_text(self.source()).to_owned();
        if let Some(existing) = self.environment.lookup_current(&key).cloned() {
            if constraint_path.is_none() {
                self.type_error(
                    crate::diagnostics::DUPLICATE_DECLARATION_CODE,
                    "x03.type.duplicate_container_declaration",
                    target.span,
                    format!("名称 {} 在当前作用域中已经声明", self.display_name(target)),
                );
                return true;
            }
            if !existing.scheme.ty.is_container() && !existing.scheme.ty.is_dynamic() {
                self.type_error(
                    CONTAINER_TYPE_MISMATCH_CODE,
                    "x03.type.container_binding_expected",
                    target.span,
                    format!("{} 不是可追加路径约束的容器", self.display_name(target)),
                );
                return true;
            }
            let existing_type = existing.scheme.ty.clone();
            if value.is_some() && existing.initialized && !can_assign(&root_type, &existing_type) {
                self.type_error(
                    CONTAINER_TYPE_MISMATCH_CODE,
                    "x03.type.container_redeclaration_mismatch",
                    target.span,
                    format!(
                        "新的容器类型 {} 不符合已锁定的 {}",
                        root_type, existing_type
                    ),
                );
                return true;
            }
            let merged = merge_constraints(&existing.container_constraints, &constraints);
            if let Some(binding) = self.environment.lookup_mut(&key) {
                binding.container_constraints = merged.clone();
                if value.is_some() {
                    binding.initialized = true;
                    binding.scheme.ty = root_type.clone();
                }
            }
            if value.is_some() {
                self.validate_constraint_tree(&root_type, &merged, target.span);
            } else if existing.initialized {
                // 约束声明也可能出现在容器初始化之后。已知结构必须在
                // 当前类型阶段立即验证；未知长度/动态值则继续留给后续
                // Runtime，不应被伪装成已经通过的静态检查。
                self.validate_existing_container_constraints(&existing_type, &merged, target.span);
            }
            self.record_materialization_if_needed(
                &binding_name,
                &merged,
                value_type.as_ref(),
                &existing_type,
            );
            return true;
        }

        self.declare_container_binding(
            target,
            key,
            root_type.clone(),
            value.is_some(),
            constraints.clone(),
        );
        if value.is_some() {
            self.validate_constraint_tree(&root_type, &constraints, target.span);
        }
        self.record_materialization_if_needed(
            &binding_name,
            &constraints,
            value_type.as_ref(),
            &root_type,
        );
        true
    }

    /// 检查容器选择器表达式；C1 语义实现位于独立选择器模块。
    pub(super) fn check_container_selector(
        &mut self,
        source: &Expression,
        step: Option<&Expression>,
        selector: &Selector,
        span: SourceSpan,
    ) -> Type {
        self.check_advanced_selector(source, step, selector, span)
    }

    /// 检查字典条目、推导值类型并拒绝重复键。
    fn check_dictionary_entries(&mut self, entries: &[DictEntry]) -> DictType {
        let mut seen = BTreeSet::new();
        let mut typed = Vec::with_capacity(entries.len());
        for entry in entries {
            let key = self.dictionary_key(entry);
            if !seen.insert(key.clone()) {
                self.type_error(
                    DUPLICATE_CONTAINER_KEY_CODE,
                    "x03.type.duplicate_dictionary_key",
                    entry.key.span(),
                    format!("字典键 {} 重复", key),
                );
            }
            let value = self.check_expression(&entry.value);
            typed.push(DictEntryType {
                key,
                value: Box::new(value),
            });
        }
        DictType::new(typed)
    }

    /// 检查声明路径约束下的初始化器并返回根容器类型。
    fn check_path_constrained_declaration(
        &mut self,
        source_path: &IndexPath,
        value_type: Option<&Type>,
        path: &[ContainerPathSegment],
    ) -> Option<Type> {
        let Some(value_type) = value_type else {
            return Some(Type::Array(ArrayType::Unknown));
        };
        if !value_type.is_container() && !value_type.is_dynamic() {
            self.type_error(
                CONTAINER_TYPE_MISMATCH_CODE,
                "x03.type.path_requires_container",
                source_path.span(),
                format!("路径约束需要容器初始化器，实际为 {}", value_type),
            );
            return None;
        }
        match resolve_exact_path(value_type, path) {
            Ok(_) => {}
            Err(error) => self.report_path_resolution_error(source_path, error),
        }
        Some(value_type.clone())
    }

    /// 对已知初始化器重新检查一组父/子路径约束。
    fn validate_constraint_tree(
        &mut self,
        root: &Type,
        constraints: &PathConstraintTree,
        span: SourceSpan,
    ) {
        let roots = constraints
            .constraints()
            .iter()
            .filter(|constraint| {
                !constraints.constraints().iter().any(|ancestor| {
                    ancestor.path.len() < constraint.path.len()
                        && ancestor
                            .path
                            .iter()
                            .zip(&constraint.path)
                            .all(|(left, right)| left == right)
                })
            })
            .map(|constraint| constraint.path.clone())
            .collect::<Vec<_>>();
        for path in roots {
            self.validate_constraint_node(root, &path, constraints, span);
        }
    }

    /// 验证追加到已初始化绑定上的约束，并报告可静态证明的路径错误。
    fn validate_existing_container_constraints(
        &mut self,
        root: &Type,
        constraints: &PathConstraintTree,
        span: SourceSpan,
    ) {
        for constraint in constraints.constraints() {
            if let Err(error) = resolve_exact_path(root, &constraint.path) {
                self.report_path_resolution_error_at_span(span, error);
            }
        }
        self.validate_constraint_tree(root, constraints, span);
    }

    /// 递归验证一条父路径及其更具体子路径，确保子路径覆盖只影响自身位置。
    fn validate_constraint_node(
        &mut self,
        root: &Type,
        path: &[ContainerPathSegment],
        constraints: &PathConstraintTree,
        span: SourceSpan,
    ) {
        let Ok(actual) = resolve_exact_path(root, path) else {
            return;
        };
        if actual.is_dynamic() {
            return;
        }
        let Some(expected) = constraints.effective_for(path).cloned() else {
            return;
        };
        // 路径可以定位到集合这个容器本身，但不能定位到某个集合成员。
        // 非空路径落在集合上时，显式类型约束应作用于所有成员。
        if !path.is_empty()
            && let Type::Set(set) = &actual
        {
            for element in set.member_types() {
                self.check_set_element_assignment(element, &expected, span);
            }
            if set.allows_dynamic() || set.is_unknown() {
                self.push_runtime_check(span, RuntimeCheckKind::SetMembership);
            }
            return;
        }
        let children = direct_children(&actual, path);
        if children.is_empty() {
            if !actual.is_container() {
                self.check_container_element_assignment(&actual, &expected, span);
            }
            return;
        }
        for (child_path, child_type) in children {
            let has_specific = constraints.constraints().iter().any(|constraint| {
                constraint.path.len() >= child_path.len()
                    && constraint
                        .path
                        .iter()
                        .zip(&child_path)
                        .all(|(left, right)| left == right)
            });
            if has_specific {
                self.validate_constraint_node(root, &child_path, constraints, span);
            } else {
                self.check_container_element_assignment(&child_type, &expected, span);
            }
        }
    }

    /// 检查没有显式路径的显式容器类型声明。
    fn check_homogeneous_array_declaration(
        &mut self,
        target: xiao_syntax::Name,
        declared_type: ScalarType,
        value_type: Option<&Type>,
        constraints: &mut PathConstraintTree,
    ) -> Option<Type> {
        let expected = Type::scalar(declared_type);
        let value_type = value_type?;
        constraints.insert(Vec::new(), expected.clone());
        match value_type {
            Type::Array(array) => {
                let valid = match array {
                    ArrayType::Unknown => true,
                    ArrayType::Homogeneous { element, .. } => {
                        self.check_container_element_assignment(element, &expected, target.span)
                    }
                    ArrayType::Heterogeneous { elements } => elements.iter().all(|element| {
                        self.check_container_element_assignment(element, &expected, target.span)
                    }),
                };
                let length = array.length();
                if valid {
                    Some(Type::Array(match length {
                        Some(length) => ArrayType::homogeneous_with_length(expected, length),
                        None => ArrayType::homogeneous(Type::scalar(declared_type)),
                    }))
                } else {
                    Some(value_type.clone())
                }
            }
            Type::DictTable(dictionary) | Type::DictColumn(dictionary) => {
                for entry in &dictionary.entries {
                    self.check_container_element_assignment(&entry.value, &expected, target.span);
                }
                Some(value_type.clone())
            }
            Type::Set(set) => {
                if set.is_unknown() {
                    Some(Type::Set(crate::SetType::homogeneous(expected)))
                } else {
                    let valid = set.member_types().all(|element| {
                        self.check_set_element_assignment(element, &expected, target.span)
                    });
                    if set.allows_dynamic() {
                        self.push_runtime_check(target.span, RuntimeCheckKind::SetMembership);
                    }
                    if valid {
                        Some(Type::Set(crate::SetType::homogeneous(expected)))
                    } else {
                        Some(value_type.clone())
                    }
                }
            }
            _ => {
                self.type_error(
                    CONTAINER_TYPE_MISMATCH_CODE,
                    "x03.type.typed_prefix_requires_container",
                    target.span,
                    format!(
                        "显式元素类型 {} 需要受支持的容器初始化器，实际为 {}",
                        expected, value_type
                    ),
                );
                None
            }
        }
    }

    /// 检查一个已知数组元素是否可写入显式类型槽。
    fn check_container_element_assignment(
        &mut self,
        actual: &Type,
        expected: &Type,
        span: SourceSpan,
    ) -> bool {
        if can_assign(actual, expected) {
            true
        } else {
            self.type_error(
                CONTAINER_TYPE_MISMATCH_CODE,
                "x03.type.array_element_mismatch",
                span,
                format!("容器元素 {} 不符合显式类型 {}", actual, expected),
            );
            false
        }
    }

    /// 向类型环境声明容器绑定。
    fn declare_container_binding(
        &mut self,
        target: xiao_syntax::Name,
        key: String,
        ty: Type,
        initialized: bool,
        constraints: PathConstraintTree,
    ) {
        if let Err(error) =
            self.environment
                .declare_mutable_with_constraints(key, ty, initialized, constraints)
        {
            self.environment_error(target.span, error);
        }
    }

    /// 只在空数组或未知路径声明时登记静态形状计划。
    fn record_materialization_if_needed(
        &mut self,
        binding: &str,
        constraints: &PathConstraintTree,
        value_type: Option<&Type>,
        current_type: &Type,
    ) {
        let candidate = value_type.unwrap_or(current_type);
        let needs_plan = !constraints.is_empty() && is_unknown_or_empty_array(candidate);
        if !needs_plan {
            return;
        }
        self.materialization_plans
            .retain(|plan| plan.binding != binding);
        self.materialization_plans
            .push(build_plan(binding.to_owned(), constraints));
    }

    /// 将字典键规范化为类型层文本。
    fn dictionary_key(&self, entry: &DictEntry) -> String {
        match entry.key {
            DictKey::Name(name) => name.unquoted_text(self.source()).to_owned(),
            DictKey::String(span) => decode_string_literal(self.source().slice(span)),
        }
    }

    /// 把路径转换错误映射为稳定类型诊断。
    fn report_path_conversion_error(&mut self, path: &IndexPath, error: PathConversionError) {
        let span = path
            .segments
            .get(error.segment)
            .map_or(path.span(), PathSegment::span);
        self.type_error(
            INVALID_DECLARATION_PATH_CODE,
            "x03.type.invalid_path",
            span,
            error.to_string(),
        );
    }

    /// 把精确路径解析错误映射为稳定类型诊断。
    fn report_path_resolution_error(&mut self, path: &IndexPath, error: PathResolutionError) {
        let span = path
            .segments
            .get(error.segment)
            .map_or(path.span(), PathSegment::span);
        self.report_path_resolution_error_at_span(span, error);
    }

    /// 在没有原始语法路径时，以声明源码区间承载路径解析诊断。
    fn report_path_resolution_error_at_span(
        &mut self,
        span: SourceSpan,
        error: PathResolutionError,
    ) {
        let (code, message_id) = match error.kind {
            PathResolutionErrorKind::SetIndexUnsupported => {
                self.set_index_error(span);
                return;
            }
            PathResolutionErrorKind::OutOfBounds { .. } => (
                CONTAINER_INDEX_OUT_OF_BOUNDS_CODE,
                "x03.type.index_out_of_bounds",
            ),
            PathResolutionErrorKind::MissingKey => {
                (CONTAINER_KEY_NOT_FOUND_CODE, "x03.type.key_not_found")
            }
            PathResolutionErrorKind::WrongSegment => {
                (INVALID_CONTAINER_PATH_CODE, "x03.type.invalid_path_segment")
            }
        };
        self.type_error(code, message_id, span, error.to_string());
    }
}

/// 判断表达式是否是 C0 容器字面量。
fn is_container_expression(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::ArrayLiteral { .. }
            | Expression::TupleLiteral { .. }
            | Expression::DictTableLiteral { .. }
            | Expression::DictColumnLiteral { .. }
            | Expression::SetLiteral { .. }
    )
}

/// 判断类型是否为空数组或未知数组，供物化计划判断使用。
fn is_unknown_or_empty_array(ty: &Type) -> bool {
    matches!(ty, Type::Array(ArrayType::Unknown))
        || matches!(
            ty,
            Type::Array(ArrayType::Heterogeneous { elements }) if elements.is_empty()
        )
}

/// 返回一个已知容器节点的直接子路径和子类型。
fn direct_children(
    root: &Type,
    path: &[ContainerPathSegment],
) -> Vec<(Vec<ContainerPathSegment>, Type)> {
    let mut children = Vec::new();
    match root {
        Type::Array(ArrayType::Heterogeneous { elements }) => {
            for (index, element) in elements.iter().enumerate() {
                let mut child = path.to_vec();
                child.push(ContainerPathSegment::Index(index));
                children.push((child, element.clone()));
            }
        }
        Type::Array(ArrayType::Homogeneous {
            element,
            length: Some(length),
        }) => {
            for index in 0..*length {
                let mut child = path.to_vec();
                child.push(ContainerPathSegment::Index(index));
                children.push((child, element.as_ref().clone()));
            }
        }
        Type::Tuple(elements) => {
            for (index, element) in elements.iter().enumerate() {
                let mut child = path.to_vec();
                child.push(ContainerPathSegment::Index(index));
                children.push((child, element.clone()));
            }
        }
        Type::DictTable(dictionary) => {
            for entry in &dictionary.entries {
                let mut child = path.to_vec();
                child.push(ContainerPathSegment::Key(entry.key.clone()));
                children.push((child, entry.value.as_ref().clone()));
            }
        }
        Type::DictColumn(dictionary) => {
            for (index, entry) in dictionary.entries.iter().enumerate() {
                let mut index_path = path.to_vec();
                index_path.push(ContainerPathSegment::Index(index));
                children.push((index_path, entry.value.as_ref().clone()));
                let mut key_path = path.to_vec();
                key_path.push(ContainerPathSegment::Key(entry.key.clone()));
                children.push((key_path, entry.value.as_ref().clone()));
            }
        }
        Type::Array(ArrayType::Homogeneous { length: None, .. })
        | Type::Array(ArrayType::Unknown)
        | Type::Dynamic
        | Type::Variable(_)
        | Type::Function { .. }
        | Type::Scalar(_)
        | Type::Set(_)
        | Type::Table(_)
        | Type::None => {}
    }
    children
}

/// 解析字符串字面量的稳定文本：去掉外围引号并处理转义。
///
/// 这是字符串字面量的**唯一**解码实现。类型层用它规范化字典键，IR 降低必须
/// 消费同一个函数——两侧各写一份会让静态能通过的键在运行时查不到。
/// 词法阶段已经验证引号配对，未知转义保留字符本身。
#[must_use]
pub fn decode_string_literal(text: &str) -> String {
    if text.len() < 2 {
        return text.to_owned();
    }
    let inner = &text[1..text.len() - 1];
    let mut output = String::with_capacity(inner.len());
    let mut escaped = false;
    for character in inner.chars() {
        if escaped {
            output.push(match character {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }
    if escaped {
        output.push('\\');
    }
    output
}
