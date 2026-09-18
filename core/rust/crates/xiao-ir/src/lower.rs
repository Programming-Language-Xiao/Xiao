//! AST、类型结果和生命周期结果到类型化 IR 的降低。
//!
//! 降低器是单向适配层：它只读取前序阶段的公开结果，不重新执行类型推断、
//! 模块解析或生命周期算法。无法在类型结果中找到的表达式保留为 `dynamic`，
//! 以便诊断输入仍可被快照，但正式前端会在有错误时拒绝输出该 IR。

use std::collections::BTreeMap;
use xiao_lifetime::{
    ControlFlowEdgeKind, EscapeReason, ExitKind, LifetimeResult, OwnershipEdgeReason,
    OwnershipKind, ReleaseActionKind, ScopeKind, StorageClass,
};
use xiao_modules::{ExportOrigin, ModuleKind, ModuleSymbolKind, ProjectModuleResult};
use xiao_source::{SourceFile, SourceSpan};
use xiao_syntax::{
    AssignmentOperator, BinaryOperator, CallArgument, CallArgumentKind, CatchClause, DeclaredType,
    DictEntry, DictKey, ElifBranch, EntryMode, Expression, FunctionParameter,
    FunctionParameterKind, FunctionTypeAnnotation, IndexPath, LiteralKind, Name, PathSegment,
    Program, RandomMode, Selector, SelectorItem, Statement, TableKind, TypeTerm, UnaryOperator,
};
use xiao_types::{ArrayType, DictType, SetType, TableValueKind, Type, TypeCheckResult};

use crate::model::*;

/// 将完整的前序分析结果降低为类型化 IR。
#[must_use]
pub fn lower_program(
    source: &SourceFile,
    program: &Program,
    type_result: &TypeCheckResult,
    lifetime: &LifetimeResult,
    modules: Option<&ProjectModuleResult>,
) -> IrProgram {
    let mut lowerer = Lowerer {
        source,
        type_result,
        selection_plan_cursor: BTreeMap::new(),
    };
    let body = lowerer.statements(&program.statements);
    let entry_mode = match program.entry_mode {
        EntryMode::Script => IrEntryMode::Script,
        EntryMode::Project { span } => IrEntryMode::Project {
            span: ir_span(span),
        },
    };
    let mut result = IrProgram::new(entry_mode, body, ir_span(program.span));
    result.modules = modules
        .map(|project| lower_modules(project, source))
        .unwrap_or_default();
    result.control_flow = lower_control_flow(lifetime);
    result.ownership = lower_ownership(lifetime);
    result.runtime_checks = type_result
        .runtime_checks()
        .iter()
        .map(|check| IrRuntimeCheck {
            kind: runtime_check_kind_name(check.kind).to_owned(),
            span: ir_span(check.span),
        })
        .collect();
    result.selection_plans = type_result
        .selection_plans()
        .iter()
        .map(lower_selection_plan)
        .collect();
    result.broadcast_assignment_plans = type_result
        .broadcast_assignment_plans()
        .iter()
        .map(lower_broadcast_assignment_plan)
        .collect();
    result.random_seed_plans = type_result
        .random_seed_plans()
        .iter()
        .map(lower_random_seed_plan)
        .collect();
    result
}

/// AST/类型结果降低器。
struct Lowerer<'a> {
    source: &'a SourceFile,
    type_result: &'a TypeCheckResult,
    /// 同一源码区间可能出现多个计划；按 lowering 访问顺序逐个分配稳定 ID。
    selection_plan_cursor: BTreeMap<(usize, usize), usize>,
}

impl<'a> Lowerer<'a> {
    /// 返回源码区间对应的类型，找不到时使用动态类型。
    fn ty(&self, span: SourceSpan) -> IrType {
        self.type_result
            .type_at(span)
            .map(lower_type)
            .unwrap_or(IrType::Dynamic)
    }

    /// 为一个选择表达式取得类型层计划 ID。
    fn selection_plan_id(&mut self, span: SourceSpan) -> Option<u32> {
        let candidates = self
            .type_result
            .selection_plans()
            .iter()
            .enumerate()
            .filter(|(_, plan)| plan.span == span)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let key = (span.start(), span.end());
        let cursor = self.selection_plan_cursor.entry(key).or_default();
        let index = candidates.get(*cursor).copied()?;
        *cursor += 1;
        u32::try_from(index).ok()
    }

    /// 降低名称并保留反引号信息。
    fn name(&self, name: Name) -> IrName {
        IrName {
            text: name.unquoted_text(self.source).to_owned(),
            backticked: name.backticked,
            span: ir_span(name.span),
        }
    }

    /// 降低一组语句。
    fn statements(&mut self, statements: &[Statement]) -> Vec<IrStatement> {
        statements
            .iter()
            .map(|statement| self.statement(statement))
            .collect()
    }

    /// 降低一个语句。
    fn statement(&mut self, statement: &Statement) -> IrStatement {
        let span = ir_span(statement.span());
        let leading_docs = statement
            .leading_docs()
            .iter()
            .copied()
            .map(ir_span)
            .collect();
        let kind = match statement {
            Statement::Expression { expression, .. } => IrStatementKind::Expression {
                value: self.expression(expression),
            },
            Statement::Assignment { target, value, .. } => IrStatementKind::Assignment {
                target: self.name(*target),
                value: self.expression(value),
            },
            Statement::ExtendedAssignment {
                target,
                operator,
                value,
                ..
            } => IrStatementKind::ExtendedAssignment {
                target: self.expression(target),
                operator: assignment_name(*operator).to_owned(),
                value: self.expression(value),
            },
            Statement::Declaration {
                target,
                declared_type,
                constraint_path,
                value,
                ..
            } => IrStatementKind::Declaration {
                target: self.name(*target),
                declared_type: lower_declared_type(declared_type),
                constraint_path: constraint_path.as_ref().map(|path| self.path(path)),
                value: value.as_ref().map(|expression| self.expression(expression)),
            },
            Statement::ConstDeclaration {
                target,
                declared_type,
                value,
                ..
            } => IrStatementKind::ConstDeclaration {
                target: self.name(*target),
                declared_type: declared_type.map(|ty| ty.as_str().to_owned()),
                value: self.expression(value),
            },
            Statement::Import { .. } => IrStatementKind::Import {
                description: self.source.slice(statement.span()).to_owned(),
            },
            Statement::Table {
                name, kind, body, ..
            } => IrStatementKind::Table {
                name: self.name(*name),
                table_kind: table_kind_name(*kind).to_owned(),
                body: self.statements(body),
            },
            Statement::Function {
                name,
                parameters,
                return_type,
                body,
                ..
            } => IrStatementKind::Function {
                name: self.name(*name),
                parameters: parameters
                    .iter()
                    .map(|parameter| self.parameter(parameter))
                    .collect(),
                return_type: return_type
                    .map(lower_function_annotation)
                    .unwrap_or(IrType::None),
                body: self.statements(body),
            },
            Statement::If {
                condition,
                body,
                elif_branches,
                else_body,
                ..
            } => IrStatementKind::If {
                condition: self.expression(condition),
                body: self.statements(body),
                elif_branches: elif_branches
                    .iter()
                    .map(|branch| self.elif_branch(branch))
                    .collect(),
                else_body: else_body.as_ref().map(|body| self.statements(body)),
            },
            Statement::For {
                target,
                iterable,
                body,
                ..
            } => IrStatementKind::For {
                target: self.name(*target),
                iterable: self.expression(iterable),
                body: self.statements(body),
            },
            Statement::While {
                condition, body, ..
            } => IrStatementKind::While {
                condition: self.expression(condition),
                body: self.statements(body),
            },
            Statement::Return { value, .. } => IrStatementKind::Return {
                value: value.as_ref().map(|expression| self.expression(expression)),
            },
            Statement::Break { .. } => IrStatementKind::Break,
            Statement::Continue { .. } => IrStatementKind::Continue,
            Statement::Try {
                body,
                catches,
                finally_body,
                ..
            } => IrStatementKind::Try {
                body: self.statements(body),
                catches: catches
                    .iter()
                    .map(|catch| self.catch_clause(catch))
                    .collect(),
                finally_body: finally_body.as_ref().map(|body| self.statements(body)),
            },
            Statement::Raise { value, .. } => IrStatementKind::Raise {
                value: self.expression(value),
            },
        };
        IrStatement {
            kind,
            span,
            leading_docs,
        }
    }

    /// 降低函数参数。
    fn parameter(&mut self, parameter: &FunctionParameter) -> IrParameter {
        IrParameter {
            name: self.name(parameter.name),
            kind: parameter_kind_name(parameter.kind).to_owned(),
            ty: parameter
                .annotation
                .map(lower_function_annotation)
                .unwrap_or_else(|| self.ty(parameter.span)),
            default: parameter
                .default
                .as_ref()
                .map(|value| self.expression(value)),
            span: ir_span(parameter.span),
        }
    }

    /// 降低 `elif` 分支。
    fn elif_branch(&mut self, branch: &ElifBranch) -> IrElifBranch {
        IrElifBranch {
            condition: self.expression(&branch.condition),
            body: self.statements(&branch.body),
            span: ir_span(branch.span),
        }
    }

    /// 降低 `catch` 处理器。
    fn catch_clause(&mut self, clause: &CatchClause) -> IrCatchClause {
        IrCatchClause {
            binding: self.name(clause.binding),
            error_type: self.name(clause.error_type),
            body: self.statements(&clause.body),
            span: ir_span(clause.span),
        }
    }

    /// 降低表达式并附加类型。
    fn expression(&mut self, expression: &Expression) -> IrExpression {
        let span = ir_span(expression.span());
        let ty = self.ty(expression.span());
        let kind = match expression {
            Expression::Literal { kind, .. } => IrExpressionKind::Literal {
                literal: literal_name(*kind).to_owned(),
                text: expression.text(self.source).to_owned(),
            },
            Expression::ArrayLiteral { elements, .. } => IrExpressionKind::Array {
                elements: elements.iter().map(|item| self.expression(item)).collect(),
            },
            Expression::TupleLiteral { elements, .. } => IrExpressionKind::Tuple {
                elements: elements.iter().map(|item| self.expression(item)).collect(),
            },
            Expression::DictTableLiteral { entries, .. } => IrExpressionKind::DictTable {
                entries: entries.iter().map(|entry| self.dict_entry(entry)).collect(),
            },
            Expression::SetLiteral { elements, .. } => IrExpressionKind::Set {
                elements: elements.iter().map(|item| self.expression(item)).collect(),
            },
            Expression::DictColumnLiteral { entries, .. } => IrExpressionKind::DictColumn {
                entries: entries.iter().map(|entry| self.dict_entry(entry)).collect(),
            },
            Expression::Name(name) => IrExpressionKind::Name {
                name: self.name(*name),
            },
            Expression::Group { expression, .. } => IrExpressionKind::Group {
                expression: Box::new(self.expression(expression)),
            },
            Expression::Unary {
                operator, operand, ..
            } => IrExpressionKind::Unary {
                operator: unary_name(*operator).to_owned(),
                operand: Box::new(self.expression(operand)),
            },
            Expression::Binary {
                operator,
                left,
                right,
                ..
            } => IrExpressionKind::Binary {
                operator: binary_name(*operator).to_owned(),
                left: Box::new(self.expression(left)),
                right: Box::new(self.expression(right)),
            },
            Expression::Call {
                callee, arguments, ..
            } => IrExpressionKind::Call {
                callee: Box::new(self.expression(callee)),
                arguments: arguments
                    .iter()
                    .map(|argument| self.call_argument(argument))
                    .collect(),
            },
            Expression::NewCall {
                callee, arguments, ..
            } => IrExpressionKind::NewCall {
                callee: Box::new(self.expression(callee)),
                arguments: arguments
                    .iter()
                    .map(|argument| self.call_argument(argument))
                    .collect(),
            },
            Expression::Member { object, member, .. } => IrExpressionKind::Member {
                object: Box::new(self.expression(object)),
                member: self.name(*member),
            },
            Expression::Cast {
                expression, target, ..
            } => IrExpressionKind::Cast {
                expression: Box::new(self.expression(expression)),
                target: target.as_str().to_owned(),
            },
            Expression::Selector {
                source,
                step,
                selector,
                ..
            } => IrExpressionKind::Selector {
                source: Box::new(self.expression(source)),
                selector: self.selector(selector),
                step: step.as_ref().map(|value| Box::new(self.expression(value))),
                selection_plan: self.selection_plan_id(expression.span()),
            },
        };
        IrExpression { kind, ty, span }
    }

    /// 降低字典条目。
    ///
    /// 键必须与类型层用同一套规范化：字符串键经 `decode_string_literal` 去掉
    /// 外围引号并处理转义。直接存源码切片会让静态能通过的键在运行时查不到。
    fn dict_entry(&mut self, entry: &DictEntry) -> IrDictEntry {
        let key = match entry.key {
            DictKey::Name(name) => self.name(name).text,
            DictKey::String(span) => xiao_types::decode_string_literal(self.source.slice(span)),
        };
        IrDictEntry {
            key,
            value: self.expression(&entry.value),
            span: ir_span(entry.span),
        }
    }

    /// 降低调用参数。
    fn call_argument(&mut self, argument: &CallArgument) -> IrCallArgument {
        IrCallArgument {
            kind: call_argument_kind_name(argument.kind).to_owned(),
            name: argument.name.map(|name| self.name(name)),
            value: self.expression(&argument.value),
            span: ir_span(argument.span),
        }
    }

    /// 降低路径。
    fn path(&self, path: &IndexPath) -> IrPath {
        IrPath {
            segments: path
                .segments
                .iter()
                .map(|segment| match segment {
                    PathSegment::Integer { span, negative } => IrPathSegment {
                        kind: IrPathSegmentKind::Index {
                            text: self.source.slice(*span).to_owned(),
                            negative: *negative,
                        },
                        span: ir_span(*span),
                    },
                    PathSegment::Name(name) => IrPathSegment {
                        kind: IrPathSegmentKind::Name {
                            name: self.name(*name),
                        },
                        span: ir_span(name.span),
                    },
                })
                .collect(),
            span: ir_span(path.span),
        }
    }

    /// 降低选择器。
    fn selector(&mut self, selector: &Selector) -> IrSelector {
        IrSelector {
            items: selector
                .items
                .iter()
                .map(|item| match item {
                    SelectorItem::Exact { path, span } => IrSelectorItem::Exact {
                        path: self.path(path),
                        span: ir_span(*span),
                    },
                    SelectorItem::Range { start, end, span } => IrSelectorItem::Range {
                        start: self.path(start),
                        end: self.path(end),
                        span: ir_span(*span),
                    },
                    SelectorItem::OpenRange {
                        start,
                        end,
                        include_start,
                        include_end,
                        span,
                    } => IrSelectorItem::OpenRange {
                        start: start.as_ref().map(|path| self.path(path)),
                        end: end.as_ref().map(|path| self.path(path)),
                        include_start: *include_start,
                        include_end: *include_end,
                        span: ir_span(*span),
                    },
                    SelectorItem::All { span } => IrSelectorItem::All {
                        span: ir_span(*span),
                    },
                    SelectorItem::Random { mode, count, span } => IrSelectorItem::Random {
                        mode: random_mode_name(*mode).to_owned(),
                        count: Box::new(self.expression(count)),
                        span: ir_span(*span),
                    },
                })
                .collect(),
            span: ir_span(selector.span),
        }
    }
}

/// 将源码区间转换为 IR 区间。
#[must_use]
pub const fn ir_span(span: SourceSpan) -> IrSpan {
    IrSpan::new(span.start(), span.end())
}

/// 将 Xiao 类型转换为 IR 类型。
#[must_use]
pub fn lower_type(ty: &Type) -> IrType {
    match ty {
        Type::Scalar(scalar) => IrType::Scalar {
            name: scalar.as_str().to_owned(),
        },
        Type::None => IrType::None,
        Type::Variable(id) => IrType::Variable { id: id.get() },
        Type::Dynamic => IrType::Dynamic,
        Type::Function {
            parameters,
            return_type,
        } => IrType::Function {
            parameters: parameters.iter().map(lower_type).collect(),
            return_type: Box::new(lower_type(return_type)),
        },
        Type::Array(array) => IrType::Array {
            shape: lower_array_type(array),
        },
        Type::Tuple(elements) => IrType::Tuple {
            elements: elements.iter().map(lower_type).collect(),
        },
        Type::DictTable(dictionary) => IrType::DictTable {
            entries: lower_dict_type(dictionary),
        },
        Type::DictColumn(dictionary) => IrType::DictColumn {
            entries: lower_dict_type(dictionary),
        },
        Type::Set(set) => lower_set_type(set),
        Type::Table(table) => IrType::Table {
            name: table.name.clone(),
            kind: table_value_kind_name(table.kind).to_owned(),
        },
    }
}

/// 转换类型层选择路径为可序列化镜像。
fn lower_selection_path(path: &xiao_types::SelectionPath) -> IrSelectionPath {
    path.iter()
        .map(|segment| match segment {
            xiao_types::SelectionPathSegment::Index { raw, resolved } => {
                IrSelectionPathSegment::Index {
                    raw: *raw,
                    resolved: *resolved,
                }
            }
            xiao_types::SelectionPathSegment::Key(key) => IrSelectionPathSegment::Key(key.clone()),
        })
        .collect()
}

/// 转换一个类型层选择项。
fn lower_selection_item(item: &xiao_types::SelectionItemPlan) -> IrSelectionItemPlan {
    match item {
        xiao_types::SelectionItemPlan::Exact { path } => IrSelectionItemPlan::Exact {
            path: lower_selection_path(path),
        },
        xiao_types::SelectionItemPlan::Range {
            start,
            end,
            include_start,
            include_end,
        } => IrSelectionItemPlan::Range {
            start: lower_selection_path(start),
            end: lower_selection_path(end),
            include_start: *include_start,
            include_end: *include_end,
        },
        xiao_types::SelectionItemPlan::All => IrSelectionItemPlan::All,
        xiao_types::SelectionItemPlan::Random {
            mode,
            count,
            dynamic_count,
        } => IrSelectionItemPlan::Random {
            mode: random_mode_name(*mode).to_owned(),
            count: *count,
            dynamic_count: *dynamic_count,
        },
    }
}

/// 转换类型层选择计划。
fn lower_selection_plan(plan: &xiao_types::SelectionPlan) -> IrSelectionPlan {
    IrSelectionPlan {
        span: ir_span(plan.span),
        source_type: lower_type(&plan.source_type),
        result_type: lower_type(&plan.result_type),
        items: plan.items.iter().map(lower_selection_item).collect(),
        selected_paths: plan
            .selected_paths
            .iter()
            .map(lower_selection_path)
            .collect(),
        target_types: plan.target_types.iter().map(lower_type).collect(),
        step: plan.step.as_ref().map(|step| IrStepPlan {
            value: step.value,
            dynamic: step.dynamic,
        }),
        requires_runtime_check: plan.requires_runtime_check,
        with_replacement: plan.with_replacement,
        has_duplicates: plan.has_duplicates,
    }
}

/// 转换广播计划。
fn lower_broadcast_assignment_plan(
    plan: &xiao_types::BroadcastAssignmentPlan,
) -> IrBroadcastAssignmentPlan {
    IrBroadcastAssignmentPlan {
        span: ir_span(plan.span),
        root_name: plan.root_name.clone(),
        target_paths: plan.target_paths.iter().map(lower_selection_path).collect(),
        value_type: lower_type(&plan.value_type),
        dynamic: plan.dynamic,
        transactional: plan.transactional,
    }
}

/// 转换随机种子计划。
fn lower_random_seed_plan(plan: &xiao_types::RandomSeedPlan) -> IrRandomSeedPlan {
    IrRandomSeedPlan {
        span: ir_span(plan.span),
        value: plan.value,
        dynamic: plan.dynamic,
    }
}

/// 转换数组形状。
fn lower_array_type(array: &ArrayType) -> IrArrayShape {
    match array {
        ArrayType::Homogeneous { element, length } => IrArrayShape::Homogeneous {
            element: Box::new(lower_type(element)),
            length: *length,
        },
        ArrayType::Heterogeneous { elements } => IrArrayShape::Heterogeneous {
            elements: elements.iter().map(lower_type).collect(),
        },
        ArrayType::Unknown => IrArrayShape::Unknown,
    }
}

/// 转换字典类型。
fn lower_dict_type(dictionary: &DictType) -> Vec<IrDictTypeEntry> {
    dictionary
        .entries
        .iter()
        .map(|entry| IrDictTypeEntry {
            key: entry.key.clone(),
            value: Box::new(lower_type(&entry.value)),
        })
        .collect()
}

/// 转换集合类型。
fn lower_set_type(set: &SetType) -> IrType {
    IrType::Set {
        members: set.member_types().map(lower_type).collect(),
        allows_dynamic: set.allows_dynamic(),
        empty: set.is_empty(),
        unknown: set.is_unknown(),
    }
}

/// 转换声明类型。
fn lower_declared_type(declared: &DeclaredType) -> IrType {
    match declared {
        DeclaredType::Scalar(scalar) => IrType::Scalar {
            name: scalar.as_str().to_owned(),
        },
        DeclaredType::Set(annotation) => IrType::Set {
            members: annotation
                .members
                .iter()
                .map(|term| match term {
                    TypeTerm::Scalar(scalar) => IrType::Scalar {
                        name: scalar.as_str().to_owned(),
                    },
                    TypeTerm::None => IrType::None,
                })
                .collect(),
            allows_dynamic: false,
            empty: false,
            unknown: annotation.members.is_empty(),
        },
    }
}

/// 转换函数类型注解。
fn lower_function_annotation(annotation: FunctionTypeAnnotation) -> IrType {
    match annotation {
        FunctionTypeAnnotation::Scalar(scalar) => IrType::Scalar {
            name: scalar.as_str().to_owned(),
        },
        FunctionTypeAnnotation::None => IrType::None,
    }
}

/// 转换模块摘要。
fn lower_modules(project: &ProjectModuleResult, source: &SourceFile) -> Vec<IrModule> {
    let mut modules = Vec::new();
    for (name, record) in &project.modules {
        let body = record
            .program
            .as_ref()
            .zip(record.source.as_ref())
            .map(|(program, module_source)| {
                let empty_types = TypeCheckResult {
                    nodes: Vec::new(),
                    diagnostics: Vec::new(),
                    runtime_checks: Vec::new(),
                    environment: xiao_types::TypeEnvironment::new(),
                    materialization_plans: Vec::new(),
                    selection_plans: Vec::new(),
                    broadcast_assignment_plans: Vec::new(),
                    random_seed_plans: Vec::new(),
                    entry_mode: program.entry_mode,
                    function_signatures: BTreeMap::new(),
                    table_signatures: BTreeMap::new(),
                };
                let mut lowerer = Lowerer {
                    source: module_source,
                    type_result: &empty_types,
                    selection_plan_cursor: BTreeMap::new(),
                };
                lowerer.statements(&program.statements)
            })
            .unwrap_or_default();
        let symbols = record
            .symbols
            .values()
            .map(|symbol| IrSymbol {
                name: symbol.name.clone(),
                kind: module_symbol_kind_name(symbol.kind).to_owned(),
                origin: export_origin_name(&symbol.origin),
                span: ir_span(symbol.span),
            })
            .collect();
        modules.push(IrModule {
            name: name.to_string(),
            kind: module_kind_name(record.kind).to_owned(),
            path: record.path.display().to_string(),
            body,
            symbols,
        });
    }
    // `source` is intentionally accepted to make the entry point's source context
    // explicit; module records already carry their own source and path.
    let _ = source;
    modules
}

/// 转换生命周期控制流。
fn lower_control_flow(lifetime: &LifetimeResult) -> IrControlFlow {
    let mut blocks = lifetime
        .control_flow()
        .blocks
        .values()
        .map(|block| IrBasicBlock {
            id: block.id.get(),
            scope: block.scope.get(),
            span: block.span.map(ir_span),
            statements: block.statements.iter().copied().map(ir_span).collect(),
            successors: block
                .successors
                .iter()
                .map(|(target, kind)| IrControlFlowSuccessor {
                    target: target.get(),
                    kind: control_flow_kind_name(*kind).to_owned(),
                })
                .collect(),
            exits: block
                .exits
                .iter()
                .map(|exit| exit_kind_name(*exit).to_owned())
                .collect(),
        })
        .collect::<Vec<_>>();
    blocks.sort_by_key(|block| block.id);
    IrControlFlow {
        entry: lifetime.control_flow().entry.map(|id| id.get()),
        blocks,
    }
}

/// 转换生命周期和所有权摘要。
fn lower_ownership(lifetime: &LifetimeResult) -> IrOwnership {
    let mut scopes = lifetime
        .scopes()
        .values()
        .map(|scope| IrScope {
            id: scope.id.get(),
            parent: scope.parent.map(|id| id.get()),
            kind: scope_kind_name(scope.kind).to_owned(),
            span: ir_span(scope.span),
            depth: scope.depth,
            values: scope.values.iter().map(|id| id.get()).collect(),
        })
        .collect::<Vec<_>>();
    scopes.sort_by_key(|scope| scope.id);
    let mut values = lifetime
        .values()
        .values()
        .map(|value| IrValue {
            id: value.id.get(),
            name: value.name.clone(),
            scope: value.scope.get(),
            span: ir_span(value.span),
            ty: value.ty.as_ref().map(lower_type),
            storage: storage_class_name(value.storage).to_owned(),
            declaration_order: value.declaration_order,
            parameter: value.parameter,
            constant: value.constant,
            temporary: value.temporary,
            escapes: value
                .escapes
                .iter()
                .map(|reason| escape_reason_name(*reason).to_owned())
                .collect(),
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|value| value.id);
    let mut strong_edges = lifetime
        .strong_edges()
        .iter()
        .map(lower_ownership_edge)
        .collect::<Vec<_>>();
    strong_edges.sort_by_key(|edge| (edge.from, edge.to));
    let mut weak_edges = lifetime
        .weak_edges()
        .iter()
        .map(lower_ownership_edge)
        .collect::<Vec<_>>();
    weak_edges.sort_by_key(|edge| (edge.from, edge.to));
    let mut release_plans = lifetime
        .release_plans()
        .values()
        .map(|plan| IrReleasePlan {
            scope: plan.scope.get(),
            exit: exit_kind_name(plan.exit).to_owned(),
            actions: plan
                .actions
                .iter()
                .map(|action| IrReleaseAction {
                    value: action.value.get(),
                    order: action.order,
                    kind: release_action_kind_name(action.kind).to_owned(),
                })
                .collect(),
            transferred: plan.transferred.iter().map(|id| id.get()).collect(),
        })
        .collect::<Vec<_>>();
    release_plans.sort_by_key(|plan| (plan.scope, plan.exit.clone()));
    let dynamic_checks = lifetime
        .dynamic_checks()
        .iter()
        .map(|check| IrDynamicLifetimeCheck {
            span: ir_span(check.span),
            value: check.value.map(|id| id.get()),
            reason: escape_reason_name(check.reason).to_owned(),
        })
        .collect();
    IrOwnership {
        scopes,
        values,
        strong_edges,
        weak_edges,
        release_plans,
        dynamic_checks,
    }
}

/// 转换所有权边。
fn lower_ownership_edge(edge: &xiao_lifetime::OwnershipEdge) -> IrOwnershipEdge {
    IrOwnershipEdge {
        from: edge.from.get(),
        to: edge.to.get(),
        kind: ownership_kind_name(edge.kind).to_owned(),
        reason: ownership_reason_name(edge.reason).to_owned(),
        span: edge.span.map(ir_span),
    }
}

/// 返回字面量类别的稳定 IR 名称。
fn literal_name(kind: LiteralKind) -> &'static str {
    match kind {
        LiteralKind::Integer => "integer",
        LiteralKind::Float => "float",
        LiteralKind::String => "str",
        LiteralKind::Boolean => "bool",
        LiteralKind::None => "none",
    }
}

/// 返回赋值运算符的稳定源码拼写。
fn assignment_name(operator: AssignmentOperator) -> &'static str {
    match operator {
        AssignmentOperator::Assign => "=",
        AssignmentOperator::AddAssign => "+=",
        AssignmentOperator::SubtractAssign => "-=",
        AssignmentOperator::IntersectAssign => "&=",
        AssignmentOperator::SymmetricDifferenceAssign => "^=",
        AssignmentOperator::MultiplyAssign => "*=",
        AssignmentOperator::DivideAssign => "/=",
        AssignmentOperator::FloorDivideAssign => "//=",
        AssignmentOperator::RemainderAssign => "%=",
        AssignmentOperator::PowerAssign => "**=",
    }
}

/// 返回一元运算符的稳定源码拼写。
fn unary_name(operator: UnaryOperator) -> &'static str {
    operator.as_str()
}

/// 返回二元运算符的稳定源码拼写。
fn binary_name(operator: BinaryOperator) -> &'static str {
    operator.as_str()
}

/// 返回调用参数类别的稳定名称。
fn call_argument_kind_name(kind: CallArgumentKind) -> &'static str {
    match kind {
        CallArgumentKind::Positional => "positional",
        CallArgumentKind::Keyword => "keyword",
        CallArgumentKind::Star => "star",
        CallArgumentKind::DoubleStar => "double_star",
    }
}

/// 返回函数参数类别的稳定名称。
fn parameter_kind_name(kind: FunctionParameterKind) -> &'static str {
    match kind {
        FunctionParameterKind::PositionalOnly => "positional_only",
        FunctionParameterKind::PositionalOrKeyword => "positional_or_keyword",
        FunctionParameterKind::KeywordOnly => "keyword_only",
        FunctionParameterKind::VarArgs => "var_args",
        FunctionParameterKind::VarKeywords => "var_keywords",
    }
}

/// 返回表声明形态的稳定名称。
fn table_kind_name(kind: TableKind) -> &'static str {
    match kind {
        TableKind::Singleton => "singleton",
        TableKind::Instance => "instance",
    }
}

/// 返回随机选择模式的稳定名称。
fn random_mode_name(mode: RandomMode) -> &'static str {
    match mode {
        RandomMode::WithoutReplacement => "without_replacement",
        RandomMode::WithReplacement => "with_replacement",
    }
}

/// 返回运行时检查类别的稳定名称。
fn runtime_check_kind_name(kind: xiao_types::RuntimeCheckKind) -> &'static str {
    match kind {
        xiao_types::RuntimeCheckKind::NumericRange => "numeric_range",
        xiao_types::RuntimeCheckKind::StringBoolean => "string_boolean",
        xiao_types::RuntimeCheckKind::DynamicConversion => "dynamic_conversion",
        xiao_types::RuntimeCheckKind::Arithmetic => "arithmetic",
        xiao_types::RuntimeCheckKind::SelectorBounds => "selector_bounds",
        xiao_types::RuntimeCheckKind::SelectorStep => "selector_step",
        xiao_types::RuntimeCheckKind::RandomCount => "random_count",
        xiao_types::RuntimeCheckKind::RandomSeed => "random_seed",
        xiao_types::RuntimeCheckKind::SetHashability => "set_hashability",
        xiao_types::RuntimeCheckKind::SetMembership => "set_membership",
        xiao_types::RuntimeCheckKind::SetOperation => "set_operation",
        xiao_types::RuntimeCheckKind::SetComparison => "set_comparison",
        xiao_types::RuntimeCheckKind::BooleanCondition => "boolean_condition",
        xiao_types::RuntimeCheckKind::Iterable => "iterable",
    }
}

/// 返回模块形态的稳定名称。
fn module_kind_name(kind: ModuleKind) -> &'static str {
    match kind {
        ModuleKind::File => "file",
        ModuleKind::Namespace => "namespace",
    }
}

/// 返回模块符号类别的稳定名称。
fn module_symbol_kind_name(kind: ModuleSymbolKind) -> &'static str {
    match kind {
        ModuleSymbolKind::Value => "value",
        ModuleSymbolKind::Function => "function",
        ModuleSymbolKind::Table => "table",
        ModuleSymbolKind::Module => "module",
        ModuleSymbolKind::Namespace => "namespace",
    }
}

/// 返回导出来源的稳定摘要。
fn export_origin_name(origin: &ExportOrigin) -> String {
    match origin {
        ExportOrigin::Local => "local".to_owned(),
        ExportOrigin::Reexport { module, name } => format!("reexport:{module}:{name}"),
    }
}

/// 返回表值形态的稳定名称。
fn table_value_kind_name(kind: TableValueKind) -> &'static str {
    match kind {
        TableValueKind::Singleton => "singleton",
        TableValueKind::Constructor => "constructor",
        TableValueKind::Instance => "instance",
    }
}

/// 返回生命周期作用域类别的稳定名称。
fn scope_kind_name(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Program => "program",
        ScopeKind::Function => "function",
        ScopeKind::Branch => "branch",
        ScopeKind::Loop => "loop",
        ScopeKind::Table => "table",
        ScopeKind::Block => "block",
        ScopeKind::Try => "try",
        ScopeKind::Catch => "catch",
        ScopeKind::Finally => "finally",
    }
}

/// 返回静态存储类别的稳定名称。
fn storage_class_name(class: StorageClass) -> &'static str {
    match class {
        StorageClass::Stack => "stack",
        StorageClass::HeapStrong => "heap_strong",
        StorageClass::HeapWeak => "heap_weak",
    }
}

/// 返回逃逸原因的稳定名称。
fn escape_reason_name(reason: EscapeReason) -> &'static str {
    match reason {
        EscapeReason::Returned => "returned",
        EscapeReason::CapturedByClosure => "captured_by_closure",
        EscapeReason::StoredInLongerLivedContainer => "stored_in_longer_lived_container",
        EscapeReason::CrossThread => "cross_thread",
        EscapeReason::DynamicValue => "dynamic_value",
        EscapeReason::Unknown => "unknown",
    }
}

/// 返回退出边类别的稳定名称。
///
/// 拼写由 [`ExitKind::as_name`] 单点维护；后端反查同一张表，不得另存一份。
fn exit_kind_name(kind: ExitKind) -> &'static str {
    kind.as_name()
}

/// 返回所有权边类别的稳定名称。
fn ownership_kind_name(kind: OwnershipKind) -> &'static str {
    match kind {
        OwnershipKind::Strong => "strong",
        OwnershipKind::Weak => "weak",
    }
}

/// 返回所有权边来源的稳定名称。
fn ownership_reason_name(reason: OwnershipEdgeReason) -> &'static str {
    match reason {
        OwnershipEdgeReason::Alias => "alias",
        OwnershipEdgeReason::ContainerElement => "container_element",
        OwnershipEdgeReason::ClosureCapture => "closure_capture",
        OwnershipEdgeReason::TableMember => "table_member",
        OwnershipEdgeReason::Explicit => "explicit",
    }
}

/// 返回释放动作类别的稳定名称。
///
/// 拼写由 [`ReleaseActionKind::as_name`] 单点维护。
fn release_action_kind_name(kind: ReleaseActionKind) -> &'static str {
    kind.as_name()
}

/// 返回控制流边类别的稳定名称。
fn control_flow_kind_name(kind: ControlFlowEdgeKind) -> &'static str {
    match kind {
        ControlFlowEdgeKind::Next => "next",
        ControlFlowEdgeKind::BranchTrue => "branch_true",
        ControlFlowEdgeKind::BranchFalse => "branch_false",
        ControlFlowEdgeKind::LoopBack => "loop_back",
        ControlFlowEdgeKind::Break => "break",
        ControlFlowEdgeKind::Continue => "continue",
        ControlFlowEdgeKind::Return => "return",
        ControlFlowEdgeKind::Error => "error",
    }
}
