//! 类型化 IR 不变量验证。
//!
//! 验证器在后端之前运行，发现结构问题时返回可供编排器和工具消费的稳定
//! 诊断。它不尝试修复 IR，也不执行任何用户代码。

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use xiao_lifetime::ExitKind;

use crate::model::*;

/// IR 结构验证错误的稳定编号。
pub const IR_INVALID_CODE: &str = "X08-IR-001";
/// IR 版本不受支持时使用的稳定编号。
pub const IR_VERSION_CODE: &str = "X08-IR-002";
/// 后端实际发出的释放序列与冻结释放计划不一致时的稳定编号。
pub const IR_RELEASE_MISMATCH_CODE: &str = "X08-IR-003";

/// 一次执行实际发出的释放序列观测。
///
/// 后端在降低过程中记录每个 `(作用域, 退出边)` 真正发出的释放动作，再交给
/// [`reconcile_release_plans`] 与冻结计划比对。这里使用纯数据而不是借用后端
/// 类型，因此 `xiao-ir` 不需要反向依赖任何后端 crate。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedRelease {
    /// 触发释放的作用域编号。
    pub scope: u32,
    /// 退出边的稳定名称，取值集合与 [`ExitKind::as_name`] 一致。
    pub exit: String,
    /// 按该次退出实际发出的释放动作。
    pub actions: Vec<IrReleaseAction>,
}

impl ObservedRelease {
    /// 创建一条释放序列观测。
    #[must_use]
    pub fn new(scope: u32, exit: impl Into<String>, actions: Vec<IrReleaseAction>) -> Self {
        Self {
            scope,
            exit: exit.into(),
            actions,
        }
    }
}

/// 一条 IR 验证错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrValidationError {
    /// 稳定诊断编号。
    pub code: &'static str,
    /// 机器可读的路径，例如 `body[0].kind`。
    pub path: String,
    /// 简短原因；不作为身份字段。
    pub message: String,
    /// 可选相关源码区间。
    pub span: Option<IrSpan>,
}

impl Display for IrValidationError {
    /// 输出适合开发者查看的验证错误。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}: {}",
            self.code, self.path, self.message
        )
    }
}

impl std::error::Error for IrValidationError {}

/// IR 验证结果。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IrValidationResult {
    /// 按确定性遍历顺序保存的错误。
    pub errors: Vec<IrValidationError>,
}

impl IrValidationResult {
    /// 判断验证是否成功。
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// 判断验证是否失败。
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.is_success()
    }

    /// 返回错误只读视图。
    #[must_use]
    pub fn errors(&self) -> &[IrValidationError] {
        &self.errors
    }
}

/// 类型化 IR 验证器。
#[derive(Clone, Copy, Debug, Default)]
pub struct IrValidator;

impl IrValidator {
    /// 创建验证器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// 验证一份完整 IR；失败时不得交给后端消费。
    #[must_use]
    pub fn validate(&self, program: &IrProgram) -> IrValidationResult {
        let mut result = IrValidationResult::default();
        if program.version != IR_VERSION {
            result.errors.push(IrValidationError {
                code: IR_VERSION_CODE,
                path: "version".to_owned(),
                message: format!("不支持的 IR 版本 {}", program.version),
                span: Some(program.span),
            });
        }
        if program.span.start > program.span.end {
            result.errors.push(error(
                "span",
                "源码区间起点不能大于终点",
                Some(program.span),
            ));
        }
        validate_statements(&program.body, "body", &mut result);
        let mut table_names = BTreeSet::new();
        for (index, table) in program.table_signatures.iter().enumerate() {
            if !table_names.insert(&table.name) || table.runtime_signature().is_none() {
                result.errors.push(error(
                    &format!("table_signatures[{index}]"),
                    "表签名重复或包含非法成员、类型、源码区间",
                    Some(table.span),
                ));
            }
        }
        for (index, module) in program.modules.iter().enumerate() {
            validate_statements(&module.body, &format!("modules[{index}].body"), &mut result);
            for (symbol_index, symbol) in module.symbols.iter().enumerate() {
                validate_span(
                    symbol.span,
                    &format!("modules[{index}].symbols[{symbol_index}].span"),
                    &mut result,
                );
            }
        }
        validate_control_flow(&program.control_flow, &mut result);
        validate_ownership(&program.ownership, &mut result);
        for (index, check) in program.runtime_checks.iter().enumerate() {
            validate_span(
                check.span,
                &format!("runtime_checks[{index}].span"),
                &mut result,
            );
        }
        result
    }
}

/// 便捷验证函数。
#[must_use]
pub fn validate(program: &IrProgram) -> IrValidationResult {
    IrValidator::new().validate(program)
}

/// 创建一条内部验证错误。
fn error(path: &str, message: &str, span: Option<IrSpan>) -> IrValidationError {
    IrValidationError {
        code: IR_INVALID_CODE,
        path: path.to_owned(),
        message: message.to_owned(),
        span,
    }
}

/// 创建一条释放序列对账错误。
fn release_error(path: &str, message: &str) -> IrValidationError {
    IrValidationError {
        code: IR_RELEASE_MISMATCH_CODE,
        path: path.to_owned(),
        message: message.to_owned(),
        span: None,
    }
}

/// 验证一个源码区间的半开边界。
fn validate_span(span: IrSpan, path: &str, result: &mut IrValidationResult) {
    if span.start > span.end {
        result
            .errors
            .push(error(path, "源码区间起点不能大于终点", Some(span)));
    }
}

/// 递归验证语句列表及其文档区间。
fn validate_statements(statements: &[IrStatement], path: &str, result: &mut IrValidationResult) {
    for (index, statement) in statements.iter().enumerate() {
        let statement_path = format!("{path}[{index}]");
        validate_span(statement.span, &format!("{statement_path}.span"), result);
        for (doc_index, doc) in statement.leading_docs.iter().enumerate() {
            validate_span(
                *doc,
                &format!("{statement_path}.leading_docs[{doc_index}]"),
                result,
            );
        }
        validate_statement_kind(&statement.kind, &statement_path, result);
    }
}

/// 验证单个语句的递归子节点和控制流边界。
fn validate_statement_kind(kind: &IrStatementKind, path: &str, result: &mut IrValidationResult) {
    match kind {
        IrStatementKind::Expression { value } | IrStatementKind::Raise { value } => {
            validate_expression(value, &format!("{path}.value"), result)
        }
        IrStatementKind::Assignment { target, value } => {
            validate_name(target, &format!("{path}.target"), result);
            validate_type(&value.ty, &format!("{path}.value.ty"), result);
            validate_expression(value, &format!("{path}.value"), result);
        }
        IrStatementKind::ExtendedAssignment { target, value, .. } => {
            validate_expression(target, &format!("{path}.target"), result);
            validate_expression(value, &format!("{path}.value"), result);
        }
        IrStatementKind::Declaration {
            target,
            declared_type,
            constraint_path,
            value,
            ..
        } => {
            validate_name(target, &format!("{path}.target"), result);
            validate_type(declared_type, &format!("{path}.declared_type"), result);
            if let Some(path_value) = constraint_path {
                validate_path(path_value, &format!("{path}.constraint_path"), result);
            }
            if let Some(value) = value {
                validate_expression(value, &format!("{path}.value"), result);
            }
        }
        IrStatementKind::ConstDeclaration { target, value, .. } => {
            validate_name(target, &format!("{path}.target"), result);
            validate_expression(value, &format!("{path}.value"), result);
        }
        IrStatementKind::Import { .. } | IrStatementKind::Break | IrStatementKind::Continue => {}
        IrStatementKind::Table { name, body, .. } => {
            validate_name(name, &format!("{path}.name"), result);
            validate_statements(body, &format!("{path}.body"), result);
        }
        IrStatementKind::Function {
            name,
            parameters,
            return_type,
            body,
            ..
        } => {
            validate_name(name, &format!("{path}.name"), result);
            validate_type(return_type, &format!("{path}.return_type"), result);
            for (index, parameter) in parameters.iter().enumerate() {
                validate_name(
                    &parameter.name,
                    &format!("{path}.parameters[{index}].name"),
                    result,
                );
                validate_type(
                    &parameter.ty,
                    &format!("{path}.parameters[{index}].ty"),
                    result,
                );
                if let Some(default) = &parameter.default {
                    validate_expression(
                        default,
                        &format!("{path}.parameters[{index}].default"),
                        result,
                    );
                }
            }
            validate_statements(body, &format!("{path}.body"), result);
        }
        IrStatementKind::If {
            condition,
            body,
            elif_branches,
            else_body,
        } => {
            validate_expression(condition, &format!("{path}.condition"), result);
            validate_statements(body, &format!("{path}.body"), result);
            for (index, branch) in elif_branches.iter().enumerate() {
                validate_span(
                    branch.span,
                    &format!("{path}.elif_branches[{index}].span"),
                    result,
                );
                validate_expression(
                    &branch.condition,
                    &format!("{path}.elif_branches[{index}].condition"),
                    result,
                );
                validate_statements(
                    &branch.body,
                    &format!("{path}.elif_branches[{index}].body"),
                    result,
                );
            }
            if let Some(body) = else_body {
                validate_statements(body, &format!("{path}.else_body"), result);
            }
        }
        IrStatementKind::For {
            target,
            iterable,
            body,
        } => {
            validate_name(target, &format!("{path}.target"), result);
            validate_expression(iterable, &format!("{path}.iterable"), result);
            validate_statements(body, &format!("{path}.body"), result);
        }
        IrStatementKind::While { condition, body } => {
            validate_expression(condition, &format!("{path}.condition"), result);
            validate_statements(body, &format!("{path}.body"), result);
        }
        IrStatementKind::Return { value } => {
            if let Some(value) = value {
                validate_expression(value, &format!("{path}.value"), result);
            }
        }
        IrStatementKind::Try {
            body,
            catches,
            finally_body,
        } => {
            if catches.is_empty() && finally_body.is_none() {
                result
                    .errors
                    .push(error(path, "try 必须包含 catch 或 finally", None));
            }
            validate_statements(body, &format!("{path}.body"), result);
            for (index, clause) in catches.iter().enumerate() {
                validate_name(
                    &clause.binding,
                    &format!("{path}.catches[{index}].binding"),
                    result,
                );
                validate_name(
                    &clause.error_type,
                    &format!("{path}.catches[{index}].error_type"),
                    result,
                );
                validate_span(
                    clause.span,
                    &format!("{path}.catches[{index}].span"),
                    result,
                );
                validate_statements(
                    &clause.body,
                    &format!("{path}.catches[{index}].body"),
                    result,
                );
            }
            if let Some(body) = finally_body {
                validate_statements(body, &format!("{path}.finally_body"), result);
            }
        }
    }
}

/// 验证名称非空且源码区间有效。
fn validate_name(name: &IrName, path: &str, result: &mut IrValidationResult) {
    validate_span(name.span, &format!("{path}.span"), result);
    if name.text.is_empty() {
        result
            .errors
            .push(error(path, "名称不能为空", Some(name.span)));
    }
}

/// 验证路径和每个路径段。
fn validate_path(path: &IrPath, path_name: &str, result: &mut IrValidationResult) {
    validate_span(path.span, &format!("{path_name}.span"), result);
    for (index, segment) in path.segments.iter().enumerate() {
        validate_span(
            segment.span,
            &format!("{path_name}.segments[{index}].span"),
            result,
        );
        if let IrPathSegmentKind::Name { name } = &segment.kind {
            validate_name(name, &format!("{path_name}.segments[{index}].name"), result);
        }
    }
}

/// 递归验证表达式、类型和选择器。
fn validate_expression(expression: &IrExpression, path: &str, result: &mut IrValidationResult) {
    validate_span(expression.span, &format!("{path}.span"), result);
    validate_type(&expression.ty, &format!("{path}.ty"), result);
    match &expression.kind {
        IrExpressionKind::Literal { .. } | IrExpressionKind::Name { .. } => {
            if let IrExpressionKind::Name { name } = &expression.kind {
                validate_name(name, &format!("{path}.name"), result);
            }
        }
        IrExpressionKind::Array { elements }
        | IrExpressionKind::Tuple { elements }
        | IrExpressionKind::Set { elements } => {
            for (index, element) in elements.iter().enumerate() {
                validate_expression(element, &format!("{path}.elements[{index}]"), result);
            }
        }
        IrExpressionKind::DictTable { entries } | IrExpressionKind::DictColumn { entries } => {
            for (index, entry) in entries.iter().enumerate() {
                validate_expression(
                    &entry.value,
                    &format!("{path}.entries[{index}].value"),
                    result,
                );
                validate_span(entry.span, &format!("{path}.entries[{index}].span"), result);
            }
        }
        IrExpressionKind::Group { expression: inner }
        | IrExpressionKind::Cast {
            expression: inner, ..
        } => {
            validate_expression(inner, &format!("{path}.expression"), result);
        }
        IrExpressionKind::Unary { operand, .. } => {
            validate_expression(operand, &format!("{path}.operand"), result);
        }
        IrExpressionKind::Binary { left, right, .. } => {
            validate_expression(left, &format!("{path}.left"), result);
            validate_expression(right, &format!("{path}.right"), result);
        }
        IrExpressionKind::Call { callee, arguments }
        | IrExpressionKind::NewCall { callee, arguments } => {
            validate_expression(callee, &format!("{path}.callee"), result);
            for (index, argument) in arguments.iter().enumerate() {
                if let Some(name) = &argument.name {
                    validate_name(name, &format!("{path}.arguments[{index}].name"), result);
                }
                validate_expression(
                    &argument.value,
                    &format!("{path}.arguments[{index}].value"),
                    result,
                );
                validate_span(
                    argument.span,
                    &format!("{path}.arguments[{index}].span"),
                    result,
                );
            }
        }
        IrExpressionKind::Member { object, member } => {
            validate_expression(object, &format!("{path}.object"), result);
            validate_name(member, &format!("{path}.member"), result);
        }
        IrExpressionKind::Selector {
            source,
            selector,
            step,
            selection_plan: _,
        } => {
            if matches!(source.ty, IrType::Set { .. }) {
                result.errors.push(error(
                    path,
                    "集合保持无序，不能进入索引选择器",
                    Some(expression.span),
                ));
            }
            validate_expression(source, &format!("{path}.source"), result);
            validate_selector(selector, &format!("{path}.selector"), result);
            if let Some(step) = step {
                validate_expression(step, &format!("{path}.step"), result);
            }
        }
    }
}

/// 递归验证类型结构和集合状态互斥性。
fn validate_type(ty: &IrType, path: &str, result: &mut IrValidationResult) {
    match ty {
        IrType::Scalar { name } => {
            if name.is_empty() {
                result
                    .errors
                    .push(error(path, "标量类型名称不能为空", None));
            }
        }
        IrType::Function {
            parameters,
            return_type,
        } => {
            for (index, parameter) in parameters.iter().enumerate() {
                validate_type(parameter, &format!("{path}.parameters[{index}]"), result);
            }
            validate_type(return_type, &format!("{path}.return_type"), result);
        }
        IrType::Array { shape } => match shape {
            IrArrayShape::Homogeneous { element, .. } => {
                validate_type(element, &format!("{path}.element"), result);
            }
            IrArrayShape::Heterogeneous { elements } => {
                for (index, element) in elements.iter().enumerate() {
                    validate_type(element, &format!("{path}.elements[{index}]"), result);
                }
            }
            IrArrayShape::Unknown => {}
        },
        IrType::Tuple { elements } => {
            for (index, element) in elements.iter().enumerate() {
                validate_type(element, &format!("{path}.elements[{index}]"), result);
            }
        }
        IrType::DictTable { entries } | IrType::DictColumn { entries } => {
            for (index, entry) in entries.iter().enumerate() {
                if entry.key.is_empty() {
                    result.errors.push(error(
                        &format!("{path}.entries[{index}].key"),
                        "字典键不能为空",
                        None,
                    ));
                }
                validate_type(
                    &entry.value,
                    &format!("{path}.entries[{index}].value"),
                    result,
                );
            }
        }
        IrType::Set {
            members,
            empty,
            unknown,
            ..
        } => {
            if *empty && *unknown {
                result
                    .errors
                    .push(error(path, "集合不能同时标记为空和未知", None));
            }
            for (index, member) in members.iter().enumerate() {
                validate_type(member, &format!("{path}.members[{index}]"), result);
            }
        }
        IrType::Table { name, kind } => {
            if name.is_empty() || kind.is_empty() {
                result.errors.push(error(path, "表类型身份不完整", None));
            }
        }
        IrType::None | IrType::Variable { .. } | IrType::Dynamic => {}
    }
}

/// 验证选择器项、路径和随机数量表达式。
fn validate_selector(selector: &IrSelector, path: &str, result: &mut IrValidationResult) {
    validate_span(selector.span, &format!("{path}.span"), result);
    for (index, item) in selector.items.iter().enumerate() {
        let item_path = format!("{path}.items[{index}]");
        match item {
            IrSelectorItem::Exact { path, span } => {
                validate_path(path, &format!("{item_path}.path"), result);
                validate_span(*span, &format!("{item_path}.span"), result);
            }
            IrSelectorItem::Range { start, end, span } => {
                validate_path(start, &format!("{item_path}.start"), result);
                validate_path(end, &format!("{item_path}.end"), result);
                validate_span(*span, &format!("{item_path}.span"), result);
            }
            IrSelectorItem::OpenRange {
                start, end, span, ..
            } => {
                if let Some(start) = start {
                    validate_path(start, &format!("{item_path}.start"), result);
                }
                if let Some(end) = end {
                    validate_path(end, &format!("{item_path}.end"), result);
                }
                validate_span(*span, &format!("{item_path}.span"), result);
            }
            IrSelectorItem::All { span } => {
                validate_span(*span, &format!("{item_path}.span"), result)
            }
            IrSelectorItem::Random { count, span, .. } => {
                validate_expression(count, &format!("{item_path}.count"), result);
                validate_span(*span, &format!("{item_path}.span"), result);
            }
        }
    }
}

/// 验证基本块编号、入口和后继目标。
fn validate_control_flow(flow: &IrControlFlow, result: &mut IrValidationResult) {
    let ids = flow.blocks.iter().map(|block| block.id).collect::<Vec<_>>();
    let unique = ids.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != ids.len() {
        result
            .errors
            .push(error("control_flow.blocks", "基本块编号必须唯一", None));
    }
    if let Some(entry) = flow.entry {
        if !unique.contains(&entry) {
            result
                .errors
                .push(error("control_flow.entry", "入口块不存在", None));
        }
    }
    for (index, block) in flow.blocks.iter().enumerate() {
        if let Some(span) = block.span {
            validate_span(span, &format!("control_flow.blocks[{index}].span"), result);
        }
        for (successor_index, successor) in block.successors.iter().enumerate() {
            if !unique.contains(&successor.target) {
                result.errors.push(error(
                    &format!("control_flow.blocks[{index}].successors[{successor_index}]"),
                    "后继目标块不存在",
                    None,
                ));
            }
        }
    }
}

/// 验证作用域、值、所有权边和释放计划引用。
fn validate_ownership(ownership: &IrOwnership, result: &mut IrValidationResult) {
    let value_ids = ownership
        .values
        .iter()
        .map(|value| value.id)
        .collect::<BTreeSet<_>>();
    let scope_ids = ownership
        .scopes
        .iter()
        .map(|scope| scope.id)
        .collect::<BTreeSet<_>>();
    if value_ids.len() != ownership.values.len() {
        result
            .errors
            .push(error("ownership.values", "值编号必须唯一", None));
    }
    if scope_ids.len() != ownership.scopes.len() {
        result
            .errors
            .push(error("ownership.scopes", "作用域编号必须唯一", None));
    }
    for (index, scope) in ownership.scopes.iter().enumerate() {
        if let Some(parent) = scope.parent {
            if !scope_ids.contains(&parent) {
                result.errors.push(error(
                    &format!("ownership.scopes[{index}].parent"),
                    "父作用域不存在",
                    Some(scope.span),
                ));
            }
        }
        validate_span(
            scope.span,
            &format!("ownership.scopes[{index}].span"),
            result,
        );
    }
    for (index, value) in ownership.values.iter().enumerate() {
        if !scope_ids.contains(&value.scope) {
            result.errors.push(error(
                &format!("ownership.values[{index}].scope"),
                "值所属作用域不存在",
                Some(value.span),
            ));
        }
        validate_span(
            value.span,
            &format!("ownership.values[{index}].span"),
            result,
        );
    }
    for (index, edge) in ownership
        .strong_edges
        .iter()
        .chain(ownership.weak_edges.iter())
        .enumerate()
    {
        if !value_ids.contains(&edge.from) || !value_ids.contains(&edge.to) {
            result.errors.push(error(
                &format!("ownership.edges[{index}]"),
                "所有权边引用了不存在的值",
                edge.span,
            ));
        }
    }
    for (index, plan) in ownership.release_plans.iter().enumerate() {
        let path = format!("ownership.release_plans[{index}]");
        if !scope_ids.contains(&plan.scope) {
            result.errors.push(error(
                &format!("{path}.scope"),
                "释放计划所属作用域不存在",
                None,
            ));
        }
        if ExitKind::from_name(&plan.exit).is_none() {
            result.errors.push(error(
                &format!("{path}.exit"),
                "退出边名称不是冻结的稳定拼写",
                None,
            ));
        }
        let mut orders = BTreeSet::new();
        let mut released = BTreeSet::new();
        for action in &plan.actions {
            if !value_ids.contains(&action.value) {
                result.errors.push(error(
                    &format!("{path}.actions"),
                    "释放动作引用了不存在的值",
                    None,
                ));
            }
            if !orders.insert(action.order) {
                result.errors.push(error(
                    &format!("{path}.actions"),
                    "同一释放计划内的顺序编号必须唯一",
                    None,
                ));
            }
            if !released.insert(action.value) {
                result.errors.push(error(
                    &format!("{path}.actions"),
                    "同一释放计划内不得重复释放同一个值",
                    None,
                ));
            }
        }
        let expected = (0..plan.actions.len()).collect::<Vec<_>>();
        let mut actual = plan
            .actions
            .iter()
            .map(|action| action.order)
            .collect::<Vec<_>>();
        actual.sort_unstable();
        if actual != expected {
            result.errors.push(error(
                &format!("{path}.actions"),
                "释放顺序必须从零开始且连续",
                None,
            ));
        }
        for value in &plan.transferred {
            if !value_ids.contains(value) {
                result.errors.push(error(
                    &format!("{path}.transferred"),
                    "转移动作引用了不存在的值",
                    None,
                ));
            }
            if released.contains(value) {
                result.errors.push(error(
                    &format!("{path}.transferred"),
                    "转移出去的值不得同时出现在同一计划的释放动作里",
                    None,
                ));
            }
        }
    }
}

/// 比对后端实际发出的释放序列与冻结释放计划。
///
/// 这是注册阶段之外唯一能验证「后端没有重排、去重或漏放」的检查：验证器只
/// 能看计划本身，而计划是每个作用域乘以全部退出边的无条件笛卡尔积，覆盖检查
/// 恒真、区分不了任何东西。只有逐条比对实际执行序列才有校验价值。
///
/// 比较按 `order` 排序后进行，不依赖两侧 `Vec` 的下标顺序：IR 里的释放计划按
/// `exit` 字符串排序，与上游枚举顺序不同。
#[must_use]
pub fn reconcile_release_plans(
    program: &IrProgram,
    observed: &[ObservedRelease],
) -> IrValidationResult {
    let mut result = IrValidationResult::default();
    for (index, entry) in observed.iter().enumerate() {
        let path = format!("observed[{index}]");
        if ExitKind::from_name(&entry.exit).is_none() {
            result.errors.push(release_error(
                &format!("{path}.exit"),
                "退出边名称不是冻结的稳定拼写",
            ));
            continue;
        }
        let plan = program
            .ownership
            .release_plans
            .iter()
            .find(|plan| plan.scope == entry.scope && plan.exit == entry.exit);
        let Some(plan) = plan else {
            result.errors.push(release_error(
                &path,
                &format!(
                    "作用域 {} 的 {} 退出边没有对应的释放计划",
                    entry.scope, entry.exit
                ),
            ));
            continue;
        };
        if !actions_match(&entry.actions, &plan.actions) {
            result.errors.push(release_error(
                &path,
                &format!(
                    "作用域 {} 的 {} 退出边实际发出的释放序列与冻结计划不一致",
                    entry.scope, entry.exit
                ),
            ));
        }
    }
    result
}

/// 判断两侧释放动作在按顺序排列后是否逐条相等。
fn actions_match(observed: &[IrReleaseAction], planned: &[IrReleaseAction]) -> bool {
    let sorted = |actions: &[IrReleaseAction]| {
        let mut sorted = actions.to_vec();
        sorted.sort_by_key(|action| action.order);
        sorted
    };
    let observed = sorted(observed);
    let planned = sorted(planned);
    observed.len() == planned.len()
        && observed.iter().zip(&planned).all(|(observed, planned)| {
            observed.value == planned.value
                && observed.order == planned.order
                && observed.kind == planned.kind
        })
}
