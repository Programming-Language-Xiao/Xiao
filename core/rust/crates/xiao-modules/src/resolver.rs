//! 05-B 导入目标、词法绑定和本地模块依赖图解析。
//!
//! 解析器建立的是静态旁路结果：它不执行导入语句，不创建模块 Runtime
//! 值，也不调用类型检查器。所有分支中的导入都会进入候选图，实际模块
//! 初始化时机留给后续 Runtime。

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

use xiao_diagnostics::{Diagnostic, DiagnosticParam, Severity};
use xiao_source::{SourceFile, SourceSpan};
use xiao_syntax::{Expression, ImportPath, ImportStatement, Name, SelectedImport, Statement};

use crate::diagnostics::{
    IMPORT_BINDING_CONFLICT_CODE, INVALID_QUALIFIER_USE_CODE, MISSING_IMPORT_SYMBOL_CODE,
    MISSING_IMPORT_TARGET_CODE, MODULE_CYCLE_CODE,
};
use crate::model::{
    BindingKind, ExportOrigin, ImportEdge, ImportEdgeKind, LexicalScopeId, ModuleGraph, ModuleKind,
    ModuleName, ModuleRecord, ModuleSymbol, ModuleSymbolKind, ProjectModuleResult, ResolvedBinding,
};

/// 解析一个已经发现的项目模块集合。
pub(crate) fn resolve_project(result: &mut ProjectModuleResult) {
    let mut state = AnalysisState::from_result(result);
    state.collect_imports();
    state.resolve_direct_targets();
    state.compute_initialization_order();
    state.build_export_interfaces();
    state.resolve_bindings_and_qualified_uses();
    state.write_back(result);
}

/// 一份不借用公开结果的解析工作集，避免在遍历 AST 时发生跨字段可变借用。
#[derive(Debug)]
struct AnalysisState {
    project_root: PathBuf,
    modules: BTreeMap<ModuleName, WorkModule>,
    namespaces: BTreeSet<ModuleName>,
    imports: Vec<RawImport>,
    graph: ModuleGraph,
    bindings: Vec<ResolvedBinding>,
    diagnostics: Vec<crate::model::ModuleDiagnostic>,
    next_scope: u32,
}

/// 一个文件模块的解析副本。
#[derive(Clone, Debug)]
struct WorkModule {
    record: ModuleRecord,
    local_symbols: BTreeMap<String, ModuleSymbol>,
}

/// 一个带所属模块和作用域的导入语句。
#[derive(Clone, Debug)]
struct RawImport {
    module: ModuleName,
    top_level: bool,
    statement: ImportStatement,
}

/// 已解析的导入目标。
#[derive(Clone, Debug)]
enum Target {
    /// 一个文件模块。
    File(ModuleName),
    /// 一个纯目录命名空间。
    Namespace(ModuleName),
}

impl Target {
    /// 返回目标逻辑名称。
    fn name(&self) -> &ModuleName {
        match self {
            Self::File(name) | Self::Namespace(name) => name,
        }
    }

    /// 返回目标类别。
    fn kind(&self) -> ModuleKind {
        match self {
            Self::File(_) => ModuleKind::File,
            Self::Namespace(_) => ModuleKind::Namespace,
        }
    }
}

/// 一个当前作用域中的静态绑定。
#[derive(Clone, Debug)]
struct ScopeBinding {
    kind: BindingKind,
    span: SourceSpan,
}

/// 解析作用域栈。
struct BindingResolver<'a> {
    state: &'a mut AnalysisState,
    module: ModuleName,
    source: &'a SourceFile,
    scopes: Vec<HashMap<String, ScopeBinding>>,
    scope_ids: Vec<LexicalScopeId>,
}

impl AnalysisState {
    /// 从发现阶段的公开结果建立工作集。
    fn from_result(result: &ProjectModuleResult) -> Self {
        let modules = result
            .modules
            .iter()
            .map(|(name, record)| {
                (
                    name.clone(),
                    WorkModule {
                        local_symbols: record.symbols.clone(),
                        record: record.clone(),
                    },
                )
            })
            .collect();
        Self {
            project_root: result.project_root.clone(),
            modules,
            namespaces: result.namespaces.keys().cloned().collect(),
            imports: Vec::new(),
            graph: ModuleGraph::default(),
            bindings: Vec::new(),
            diagnostics: Vec::new(),
            next_scope: 0,
        }
    }

    /// 递归收集全部模块中的导入语句和稳定作用域编号。
    fn collect_imports(&mut self) {
        let module_names = self.modules.keys().cloned().collect::<Vec<_>>();
        for module in module_names {
            let Some(program) = self
                .modules
                .get(&module)
                .and_then(|work| work.record.program.clone())
            else {
                continue;
            };
            self.collect_statements(&module, &program.statements, true);
        }
    }

    /// 为一组语句递归分配作用域并保存导入节点。
    fn collect_statements(
        &mut self,
        module: &ModuleName,
        statements: &[Statement],
        top_level: bool,
    ) {
        for statement in statements {
            match statement {
                Statement::Import { import, .. } => self.imports.push(RawImport {
                    module: module.clone(),
                    top_level,
                    statement: import.clone(),
                }),
                Statement::Function { body, .. } => {
                    self.collect_statements(module, body, false);
                }
                Statement::Table { body, .. } => {
                    self.collect_statements(module, body, false);
                }
                Statement::If {
                    body,
                    elif_branches,
                    else_body,
                    ..
                } => {
                    self.collect_statements(module, body, false);
                    for branch in elif_branches {
                        self.collect_statements(module, &branch.body, false);
                    }
                    if let Some(body) = else_body {
                        self.collect_statements(module, body, false);
                    }
                }
                Statement::For { body, .. } | Statement::While { body, .. } => {
                    self.collect_statements(module, body, false);
                }
                _ => {}
            }
        }
    }

    /// 分配一个工作集范围内唯一的作用域编号。
    fn allocate_scope(&mut self) -> LexicalScopeId {
        let scope = LexicalScopeId(self.next_scope);
        self.next_scope = self.next_scope.saturating_add(1);
        scope
    }

    /// 解析所有直接导入目标并先建立导入边。
    fn resolve_direct_targets(&mut self) {
        let imports = self.imports.clone();
        for raw in imports {
            match &raw.statement {
                ImportStatement::Modules { imports, .. } => {
                    for import in imports {
                        let Some(target) = self.target_from_path(&import.path, &raw.module) else {
                            continue;
                        };
                        self.add_edge(
                            &raw.module,
                            target.name().clone(),
                            ImportEdgeKind::Import,
                            import.span,
                        );
                    }
                }
                ImportStatement::From {
                    module, imports, ..
                } => {
                    let Some(target) = self.target_from_path(module, &raw.module) else {
                        continue;
                    };
                    for import in imports {
                        if let Some(child) = self.target_for_selected(&target, import, &raw.module)
                        {
                            self.add_edge(
                                &raw.module,
                                child.name().clone(),
                                ImportEdgeKind::Import,
                                import.span,
                            );
                        } else if matches!(target, Target::File(_)) {
                            self.add_edge(
                                &raw.module,
                                target.name().clone(),
                                ImportEdgeKind::Import,
                                import.span,
                            );
                        }
                    }
                }
            }
        }
    }

    /// 计算当前名称是否指向文件模块或命名空间。
    fn target_from_path(&self, path: &ImportPath, source_module: &ModuleName) -> Option<Target> {
        let name = self.path_name(path, source_module)?;
        self.target_by_name(&name)
    }

    /// 从选择导入目标中解析子模块/命名空间。
    fn target_for_selected(
        &self,
        target: &Target,
        import: &SelectedImport,
        source_module: &ModuleName,
    ) -> Option<Target> {
        let name = self.name_text(import.name, source_module);
        match target {
            Target::Namespace(parent) => self.target_by_name(&parent.child(name)),
            Target::File(_) => None,
        }
    }

    /// 按逻辑名称查找文件模块或命名空间。
    fn target_by_name(&self, name: &ModuleName) -> Option<Target> {
        if self.modules.contains_key(name) {
            Some(Target::File(name.clone()))
        } else if self.namespaces.contains(name) {
            Some(Target::Namespace(name.clone()))
        } else {
            None
        }
    }

    /// 把 AST 名称转换为逻辑模块名称。
    fn path_name(&self, path: &ImportPath, source_module: &ModuleName) -> Option<ModuleName> {
        let mut segments = Vec::with_capacity(path.segments.len());
        for segment in &path.segments {
            let text = self.name_text(*segment, source_module);
            if text.is_empty() {
                return None;
            }
            segments.push(text);
        }
        Some(ModuleName::new(segments))
    }

    /// 从源码区间读取名称；模块段不允许反引号，但此处统一去壳。
    fn name_text(&self, name: Name, source_module: &ModuleName) -> String {
        self.modules
            .get(source_module)
            .and_then(|work| work.record.source.as_ref())
            .map(|source| name.unquoted_text(source))
            .unwrap_or_default()
            .to_owned()
    }

    /// 记录一条聚合依赖边。
    fn add_edge(
        &mut self,
        source: &ModuleName,
        target: ModuleName,
        kind: ImportEdgeKind,
        span: SourceSpan,
    ) {
        let edges = self.graph.edges.entry(source.clone()).or_default();
        if let Some(edge) = edges.iter_mut().find(|edge| edge.target == target) {
            if edge.kind == ImportEdgeKind::QualifiedUse && kind == ImportEdgeKind::Import {
                edge.kind = ImportEdgeKind::Import;
            }
            edge.spans.push(span);
        } else {
            edges.push(ImportEdge {
                target,
                kind,
                spans: vec![span],
            });
            edges.sort_by(|left, right| left.target.cmp(&right.target));
        }
    }

    /// 生成缺失目标诊断。
    fn missing_target(&mut self, module: &ModuleName, span: SourceSpan, path: &ImportPath) {
        self.missing_target_display(module, span, self.display_path(path, module));
    }

    /// 生成一个已经规范化逻辑名称对应的缺失目标诊断。
    fn missing_target_name(&mut self, module: &ModuleName, span: SourceSpan, target: ModuleName) {
        self.missing_target_display(module, span, target.to_string());
    }

    /// 根据展示文本生成缺失目标诊断，避免把名称文本重新解析成模块名称。
    fn missing_target_display(&mut self, module: &ModuleName, span: SourceSpan, display: String) {
        self.push_diagnostic(
            Some(module.clone()),
            span,
            MISSING_IMPORT_TARGET_CODE,
            "x05.module.missing_import_target",
            format!("导入目标不存在: {display}"),
            [("target".to_owned(), DiagnosticParam::Text(display))],
        );
    }

    /// 计算文件模块依赖的确定性拓扑序，并报告循环。
    fn compute_initialization_order(&mut self) {
        self.graph.initialization_order.clear();
        let names = self.modules.keys().cloned().collect::<Vec<_>>();
        let mut colors = BTreeMap::<ModuleName, VisitColor>::new();
        let mut stack = Vec::<ModuleName>::new();
        let mut cycle_keys = BTreeSet::new();
        let mut has_cycle = false;
        for name in names {
            if !matches!(colors.get(&name), Some(VisitColor::Done)) {
                self.visit_module_order(
                    &name,
                    &mut colors,
                    &mut stack,
                    &mut cycle_keys,
                    &mut has_cycle,
                );
            }
        }
        // 深度优先访问在依赖返回后写入当前模块，因此天然已经是依赖优先序。
        // 循环图没有可执行的确定初始化计划，不能把部分 DFS 结果暴露给后端。
        if has_cycle {
            self.graph.initialization_order.clear();
        }
    }

    /// 深度优先计算依赖优先顺序。
    fn visit_module_order(
        &mut self,
        name: &ModuleName,
        colors: &mut BTreeMap<ModuleName, VisitColor>,
        stack: &mut Vec<ModuleName>,
        cycle_keys: &mut BTreeSet<String>,
        has_cycle: &mut bool,
    ) {
        match colors.get(name) {
            Some(VisitColor::Done) => return,
            Some(VisitColor::Active) => {
                *has_cycle = true;
                let start = stack.iter().position(|item| item == name).unwrap_or(0);
                let cycle = stack[start..]
                    .iter()
                    .chain(std::iter::once(name))
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                let key = cycle.join("->");
                if cycle_keys.insert(key.clone()) {
                    self.push_system_diagnostic(
                        MODULE_CYCLE_CODE,
                        "x05.module.import_cycle",
                        format!("检测到模块循环导入: {key}"),
                        [("cycle".to_owned(), DiagnosticParam::Text(key))],
                    );
                }
                return;
            }
            None => {}
        }
        colors.insert(name.clone(), VisitColor::Active);
        stack.push(name.clone());
        let dependencies = self
            .graph
            .edges
            .get(name)
            .into_iter()
            .flatten()
            .filter(|edge| edge.kind == ImportEdgeKind::Import)
            .filter(|edge| self.modules.contains_key(&edge.target))
            .map(|edge| edge.target.clone())
            .collect::<Vec<_>>();
        for dependency in dependencies {
            self.visit_module_order(&dependency, colors, stack, cycle_keys, has_cycle);
        }
        stack.pop();
        colors.insert(name.clone(), VisitColor::Done);
        self.graph.initialization_order.push(name.clone());
    }

    /// 按依赖优先顺序传播每个文件模块的顶层导出接口。
    fn build_export_interfaces(&mut self) {
        let order = self.graph.initialization_order.clone();
        for module in order {
            let Some(work) = self.modules.get(&module).cloned() else {
                continue;
            };
            let mut symbols = work.local_symbols;
            let top_level = self
                .imports
                .iter()
                .filter(|raw| raw.module == module && raw.top_level)
                .cloned()
                .collect::<Vec<_>>();
            for raw in top_level {
                self.add_top_level_exports(&module, &mut symbols, &raw);
            }
            if let Some(record) = self.modules.get_mut(&module) {
                record.record.symbols = symbols;
            }
        }
    }

    /// 将一条顶层导入加入模块接口，并验证选择符号存在。
    fn add_top_level_exports(
        &mut self,
        module: &ModuleName,
        symbols: &mut BTreeMap<String, ModuleSymbol>,
        raw: &RawImport,
    ) {
        match &raw.statement {
            ImportStatement::Modules { imports, .. } => {
                for import in imports {
                    let Some(target) = self.target_from_path(&import.path, module) else {
                        continue;
                    };
                    let local = import.alias.unwrap_or_else(|| import.path.segments[0]);
                    let key = self.name_text_for_module(local, module);
                    let symbol = ModuleSymbol {
                        name: key.clone(),
                        kind: match target.kind() {
                            ModuleKind::File => ModuleSymbolKind::Module,
                            ModuleKind::Namespace => ModuleSymbolKind::Namespace,
                        },
                        origin: ExportOrigin::Local,
                        span: import.span,
                    };
                    symbols.entry(key).or_insert(symbol);
                }
            }
            ImportStatement::From {
                module: path,
                imports,
                ..
            } => {
                let Some(target) = self.target_from_path(path, module) else {
                    return;
                };
                for import in imports {
                    let local_name = import.alias.map_or(import.name, |alias| alias);
                    let local_key = self.name_text_for_module(local_name, module);
                    if let Some(child) = self.target_for_selected(&target, import, module) {
                        symbols.entry(local_key.clone()).or_insert(ModuleSymbol {
                            name: local_key,
                            kind: match child.kind() {
                                ModuleKind::File => ModuleSymbolKind::Module,
                                ModuleKind::Namespace => ModuleSymbolKind::Namespace,
                            },
                            origin: ExportOrigin::Local,
                            span: import.span,
                        });
                        continue;
                    }
                    let Target::File(target_module) = &target else {
                        continue;
                    };
                    let selected_name = self.name_text_for_module(import.name, module);
                    let Some(origin_symbol) = self
                        .modules
                        .get(target_module)
                        .and_then(|work| work.record.symbols.get(&selected_name))
                        .cloned()
                    else {
                        continue;
                    };
                    let origin = match origin_symbol.origin {
                        ExportOrigin::Local => ExportOrigin::Reexport {
                            module: target_module.clone(),
                            name: selected_name,
                        },
                        other => other,
                    };
                    symbols.entry(local_key.clone()).or_insert(ModuleSymbol {
                        name: local_key,
                        kind: origin_symbol.kind,
                        origin,
                        span: import.span,
                    });
                }
            }
        }
    }

    /// 对每个模块重新遍历 AST，建立词法绑定并解析限定成员访问。
    fn resolve_bindings_and_qualified_uses(&mut self) {
        let modules = self
            .modules
            .iter()
            .filter_map(|(name, work)| {
                work.record
                    .source
                    .as_ref()
                    .zip(work.record.program.as_ref())
                    .map(|(source, program)| (name.clone(), source.clone(), program.clone()))
            })
            .collect::<Vec<_>>();
        for (module, source, program) in modules {
            let root = self.allocate_scope();
            let mut resolver = BindingResolver::new(self, module, &source, root);
            resolver.check_statements(&program.statements);
        }
    }

    /// 将工作集写回公共结果。
    fn write_back(self, result: &mut ProjectModuleResult) {
        for (name, work) in self.modules {
            if let Some(record) = result.modules.get_mut(&name) {
                record.symbols = work.record.symbols;
            }
        }
        result.graph = self.graph;
        result.bindings = self.bindings;
        result.diagnostics.extend(self.diagnostics);
    }

    /// 将模块名转换为当前源码中的名称文本。
    fn name_text_for_module(&self, name: Name, module: &ModuleName) -> String {
        self.modules
            .get(module)
            .and_then(|work| work.record.source.as_ref())
            .map_or_else(String::new, |source| name.unquoted_text(source).to_owned())
    }

    /// 从任意模块源码读取路径名称。
    fn display_path(&self, path: &ImportPath, module: &ModuleName) -> String {
        path.segments
            .iter()
            .map(|segment| self.name_text_for_module(*segment, module))
            .collect::<Vec<_>>()
            .join(".")
    }

    /// 记录缺失符号诊断。
    fn missing_symbol(
        &mut self,
        module: &ModuleName,
        span: SourceSpan,
        target: &ModuleName,
        symbol: &str,
    ) {
        self.push_diagnostic(
            Some(module.clone()),
            span,
            MISSING_IMPORT_SYMBOL_CODE,
            "x05.module.missing_import_symbol",
            format!("模块 {} 没有可导入名称 {}", target, symbol),
            [
                (
                    "module".to_owned(),
                    DiagnosticParam::Text(target.to_string()),
                ),
                (
                    "symbol".to_owned(),
                    DiagnosticParam::Text(symbol.to_owned()),
                ),
            ],
        );
    }

    /// 追加带模块上下文的源码诊断。
    fn push_diagnostic(
        &mut self,
        module: Option<ModuleName>,
        span: SourceSpan,
        code: &'static str,
        message_id: &'static str,
        message: String,
        params: impl IntoIterator<Item = (String, DiagnosticParam)>,
    ) {
        let path = module
            .as_ref()
            .and_then(|name| self.modules.get(name))
            .map(|work| work.record.path.clone());
        let diagnostic = Diagnostic::error_at(code, message_id, span, message).with_params(params);
        self.diagnostics.push(crate::model::ModuleDiagnostic {
            module,
            path,
            diagnostic,
        });
    }

    /// 追加没有源码区间的系统诊断。
    fn push_system_diagnostic(
        &mut self,
        code: &'static str,
        message_id: &'static str,
        message: String,
        params: impl IntoIterator<Item = (String, DiagnosticParam)>,
    ) {
        let diagnostic =
            Diagnostic::new(code, message_id, Severity::Error, None, message).with_params(params);
        self.diagnostics.push(crate::model::ModuleDiagnostic {
            module: None,
            path: Some(self.project_root.clone()),
            diagnostic,
        });
    }
}

/// 访问状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitColor {
    /// 正在访问。
    Active,
    /// 已完成。
    Done,
}

impl<'a> BindingResolver<'a> {
    /// 创建一个模块作用域解析器。
    fn new(
        state: &'a mut AnalysisState,
        module: ModuleName,
        source: &'a SourceFile,
        root: LexicalScopeId,
    ) -> Self {
        Self {
            state,
            module,
            source,
            scopes: vec![HashMap::new()],
            scope_ids: vec![root],
        }
    }

    /// 检查一组语句。
    fn check_statements(&mut self, statements: &[Statement]) {
        self.predeclare_functions(statements);
        for statement in statements {
            self.check_statement(statement);
        }
    }

    /// 预登记当前块的函数名，使其与 04 阶段的函数检查顺序一致。
    fn predeclare_functions(&mut self, statements: &[Statement]) {
        for statement in statements {
            let (name, span) = match statement {
                Statement::Function { name, span, .. } | Statement::Table { name, span, .. } => {
                    (*name, *span)
                }
                _ => continue,
            };
            let key = self.name_text(name);
            if self.current_scope().contains_key(&key) {
                self.binding_conflict(span, &key);
            } else {
                self.current_scope_mut().insert(
                    key,
                    ScopeBinding {
                        kind: BindingKind::Local,
                        span,
                    },
                );
            }
        }
    }

    /// 检查单条语句。
    fn check_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Import { import, .. } => self.check_import(import),
            Statement::Expression { expression, .. } => self.check_expression(expression),
            Statement::Assignment { target, value, .. } => {
                self.check_expression(value);
                self.check_assignment_name(*target, target.span);
            }
            Statement::ExtendedAssignment { target, value, .. } => {
                self.check_expression(value);
                self.check_expression(target);
            }
            Statement::Declaration { target, value, .. } => {
                if let Some(value) = value {
                    self.check_expression(value);
                }
                self.declare_local(*target, target.span);
            }
            Statement::ConstDeclaration { target, value, .. } => {
                self.check_expression(value);
                self.declare_local(*target, target.span);
            }
            Statement::Function {
                parameters, body, ..
            } => {
                for parameter in parameters {
                    if let Some(default) = &parameter.default {
                        self.check_expression(default);
                    }
                }
                self.push_scope();
                for parameter in parameters {
                    self.declare_local_name(parameter.name, parameter.span);
                }
                self.check_statements(body);
                self.pop_scope();
            }
            Statement::Table { body, .. } => {
                self.push_scope();
                self.predeclare_functions(body);
                for member in body {
                    match member {
                        Statement::Assignment { target, value, .. } => {
                            self.check_expression(value);
                            self.declare_local(*target, target.span);
                        }
                        Statement::Declaration { target, value, .. } => {
                            if let Some(value) = value {
                                self.check_expression(value);
                            }
                            self.declare_local(*target, target.span);
                        }
                        Statement::ConstDeclaration { target, value, .. } => {
                            self.check_expression(value);
                            self.declare_local(*target, target.span);
                        }
                        Statement::Function {
                            parameters, body, ..
                        } => {
                            for parameter in parameters {
                                if let Some(default) = &parameter.default {
                                    self.check_expression(default);
                                }
                            }
                            self.push_scope();
                            for parameter in parameters {
                                self.declare_local_name(parameter.name, parameter.span);
                            }
                            self.check_statements(body);
                            self.pop_scope();
                        }
                        other => self.check_statement(other),
                    }
                }
                self.pop_scope();
            }
            Statement::If {
                condition,
                body,
                elif_branches,
                else_body,
                ..
            } => {
                self.check_expression(condition);
                self.check_scoped_body(body);
                for branch in elif_branches {
                    self.check_expression(&branch.condition);
                    self.check_scoped_body(&branch.body);
                }
                if let Some(body) = else_body {
                    self.check_scoped_body(body);
                }
            }
            Statement::For {
                target,
                iterable,
                body,
                ..
            } => {
                self.check_expression(iterable);
                self.push_scope();
                self.declare_local(*target, target.span);
                self.check_statements(body);
                self.pop_scope();
            }
            Statement::While {
                condition, body, ..
            } => {
                self.check_expression(condition);
                self.check_scoped_body(body);
            }
            Statement::Return { value, .. } => {
                if let Some(value) = value {
                    self.check_expression(value);
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
            Statement::Raise { value, .. } => self.check_expression(value),
            Statement::Try {
                body,
                catches,
                finally_body,
                ..
            } => {
                self.check_scoped_body(body);
                for catch in catches {
                    self.push_scope();
                    self.declare_local_name(catch.binding, catch.binding.span);
                    self.check_statements(&catch.body);
                    self.pop_scope();
                }
                if let Some(body) = finally_body {
                    self.check_scoped_body(body);
                }
            }
        }
    }

    /// 检查一条导入语句并声明绑定。
    fn check_import(&mut self, import: &ImportStatement) {
        match import {
            ImportStatement::Modules { imports, .. } => {
                for item in imports {
                    let Some(target) = self.state.target_from_path(&item.path, &self.module) else {
                        self.state
                            .missing_target(&self.module, item.path.span(), &item.path);
                        continue;
                    };
                    let Some(first) = item.path.segments.first().copied() else {
                        self.state
                            .missing_target(&self.module, item.path.span(), &item.path);
                        continue;
                    };
                    let local = item.alias.unwrap_or(first);
                    let key = self.name_text(local);
                    let (qualifier_target, qualifier_kind) = if item.alias.is_some() {
                        (target.name().clone(), target.kind())
                    } else {
                        let root = ModuleName::new(vec![key.clone()]);
                        let Some(root_target) = self.state.target_by_name(&root) else {
                            self.state
                                .missing_target_name(&self.module, item.path.span(), root);
                            continue;
                        };
                        (root, root_target.kind())
                    };
                    let binding = ScopeBinding {
                        kind: BindingKind::Qualifier {
                            target: qualifier_target,
                            kind: qualifier_kind,
                        },
                        span: item.span,
                    };
                    self.declare_import_binding(key, binding);
                }
            }
            ImportStatement::From {
                module, imports, ..
            } => {
                let Some(target) = self.state.target_from_path(module, &self.module) else {
                    self.state
                        .missing_target(&self.module, module.span(), module);
                    return;
                };
                for item in imports {
                    let local = item.alias.map_or(item.name, |alias| alias);
                    let key = self.name_text(local);
                    if let Some(child) = self.state.target_for_selected(&target, item, &self.module)
                    {
                        self.declare_import_binding(
                            key,
                            ScopeBinding {
                                kind: BindingKind::Qualifier {
                                    target: child.name().clone(),
                                    kind: child.kind(),
                                },
                                span: item.span,
                            },
                        );
                    } else {
                        match &target {
                            Target::File(target_module) => {
                                let selected = self.name_text(item.name);
                                let symbol = self
                                    .state
                                    .modules
                                    .get(target_module)
                                    .and_then(|work| work.record.symbols.get(&selected))
                                    .cloned();
                                if let Some(symbol) = symbol {
                                    self.declare_import_binding(
                                        key,
                                        ScopeBinding {
                                            kind: BindingKind::Value {
                                                origin: symbol.origin,
                                            },
                                            span: item.span,
                                        },
                                    );
                                } else {
                                    self.state.missing_symbol(
                                        &self.module,
                                        item.name.span,
                                        target_module,
                                        &selected,
                                    );
                                }
                            }
                            Target::Namespace(parent) => {
                                let selected = self.name_text(item.name);
                                self.state.missing_target_name(
                                    &self.module,
                                    item.name.span,
                                    parent.child(selected),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// 检查表达式中的名称和限定成员链。
    fn check_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Literal { .. } | Expression::Name(_) => {
                if let Expression::Name(name) = expression {
                    let key = self.name_text(*name);
                    if let Some(binding) = self.lookup(&key) {
                        if matches!(binding.kind, BindingKind::Qualifier { .. }) {
                            self.invalid_qualifier(*name, "限定符必须继续访问成员");
                        }
                    }
                }
            }
            Expression::ArrayLiteral { elements, .. }
            | Expression::TupleLiteral { elements, .. }
            | Expression::SetLiteral { elements, .. } => {
                for element in elements {
                    self.check_expression(element);
                }
            }
            Expression::DictTableLiteral { entries, .. }
            | Expression::DictColumnLiteral { entries, .. } => {
                for entry in entries {
                    self.check_expression(&entry.value);
                }
            }
            Expression::Group { expression, .. }
            | Expression::Unary {
                operand: expression,
                ..
            }
            | Expression::Cast { expression, .. } => self.check_expression(expression),
            Expression::Binary { left, right, .. } => {
                self.check_expression(left);
                self.check_expression(right);
            }
            Expression::Call {
                callee, arguments, ..
            }
            | Expression::NewCall {
                callee, arguments, ..
            } => {
                self.check_expression(callee);
                for argument in arguments {
                    self.check_expression(&argument.value);
                }
            }
            Expression::Member { .. } => {
                self.resolve_member_chain(expression);
            }
            Expression::Selector {
                source,
                step,
                selector,
                ..
            } => {
                self.check_expression(source);
                if let Some(step) = step {
                    self.check_expression(step);
                }
                for item in &selector.items {
                    if let xiao_syntax::SelectorItem::Random { count, .. } = item {
                        self.check_expression(count);
                    }
                }
            }
        }
    }

    /// 解析以导入限定符为根的最长成员链。
    fn resolve_member_chain(&mut self, expression: &Expression) {
        let Some((root, members)) = flatten_member_chain(expression) else {
            return;
        };
        let root_key = self.name_text(root);
        let Some(binding) = self.lookup(&root_key) else {
            self.check_expression_member_children(expression);
            return;
        };
        let BindingKind::Qualifier { target, kind } = binding.kind else {
            self.check_expression_member_children(expression);
            return;
        };
        let mut current = target;
        let mut current_kind = kind;
        let mut consumed = 0usize;
        while consumed < members.len() {
            let member = members[consumed];
            let member_name = self.name_text(member);
            let next = current.child(member_name.clone());
            if let Some(child) = self.state.target_by_name(&next) {
                current = child.name().clone();
                current_kind = child.kind();
                consumed += 1;
                if child.kind() == ModuleKind::File {
                    self.state.add_edge(
                        &self.module,
                        child.name().clone(),
                        ImportEdgeKind::QualifiedUse,
                        expression.span(),
                    );
                    break;
                }
            } else {
                match current_kind {
                    ModuleKind::Namespace => {
                        self.state
                            .missing_target_name(&self.module, member.span, next);
                        return;
                    }
                    // 文件模块的下一段是符号而非子模块；退出循环后由
                    // 统一符号接口检查路径处理。
                    ModuleKind::File => break,
                }
            }
        }
        if current_kind == ModuleKind::Namespace {
            self.invalid_qualifier(root, "限定路径没有解析到文件模块");
            return;
        }
        let Some(target_record) = self.state.modules.get(&current) else {
            self.invalid_qualifier(root, "限定路径没有解析到文件模块");
            return;
        };
        if consumed == members.len() {
            self.invalid_qualifier(root, "模块限定符不能作为普通值使用");
            return;
        }
        let symbol_name = self.name_text(members[consumed]);
        let has_symbol = target_record.record.symbols.contains_key(&symbol_name);
        if !has_symbol {
            self.state
                .missing_symbol(&self.module, members[consumed].span, &current, &symbol_name);
        }
        // 成员之后的对象属性由类型阶段处理；这里只检查模块前缀。
    }

    /// 对非限定成员表达式递归检查其对象。
    fn check_expression_member_children(&mut self, expression: &Expression) {
        if let Expression::Member { object, .. } = expression {
            self.check_expression(object);
        }
    }

    /// 检查一个简单赋值是否试图改写限定符。
    fn check_assignment_name(&mut self, name: Name, span: SourceSpan) {
        let key = self.name_text(name);
        if let Some(binding) = self.lookup(&key) {
            if matches!(binding.kind, BindingKind::Qualifier { .. }) {
                self.invalid_qualifier(name, "模块和命名空间限定符不可赋值");
            }
        } else {
            self.current_scope_mut().insert(
                key,
                ScopeBinding {
                    kind: BindingKind::Local,
                    span,
                },
            );
        }
    }

    /// 声明普通本地名称。
    fn declare_local(&mut self, name: Name, span: SourceSpan) {
        self.declare_local_name(name, span);
    }

    /// 声明名称并检查当前作用域冲突。
    fn declare_local_name(&mut self, name: Name, span: SourceSpan) {
        let key = self.name_text(name);
        if self.current_scope().contains_key(&key) {
            self.binding_conflict(span, &key);
            return;
        }
        self.current_scope_mut().insert(
            key,
            ScopeBinding {
                kind: BindingKind::Local,
                span,
            },
        );
    }

    /// 声明导入名称并记录公开旁路绑定。
    fn declare_import_binding(&mut self, key: String, binding: ScopeBinding) {
        if let Some(existing) = self.current_scope().get(&key).cloned() {
            // 多个 `import a.b`/`import a.c` 共享根命名空间时合并。
            let merge = matches!(
                (&existing.kind, &binding.kind),
                (
                    BindingKind::Qualifier {
                        target: left,
                        kind: ModuleKind::Namespace
                    },
                    BindingKind::Qualifier {
                        target: right,
                        kind: ModuleKind::Namespace
                    }
                ) if left == right
            );
            if !merge {
                self.binding_conflict(binding.span, &key);
                return;
            }
        } else {
            self.current_scope_mut()
                .insert(key.clone(), binding.clone());
        }
        self.state.bindings.push(ResolvedBinding {
            module: self.module.clone(),
            scope: *self.scope_ids.last().expect("作用域栈非空"),
            local_name: key,
            kind: binding.kind,
            span: binding.span,
        });
    }

    /// 在独立词法作用域中检查语句体。
    fn check_scoped_body(&mut self, body: &[Statement]) {
        self.push_scope();
        self.check_statements(body);
        self.pop_scope();
    }

    /// 压入作用域。
    fn push_scope(&mut self) {
        let scope = self.state.allocate_scope();
        self.scopes.push(HashMap::new());
        self.scope_ids.push(scope);
    }

    /// 弹出作用域。
    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
            self.scope_ids.pop();
        }
    }

    /// 从内向外查找绑定。
    fn lookup(&self, key: &str) -> Option<ScopeBinding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(key).cloned())
    }

    /// 当前作用域不可变借用。
    fn current_scope(&self) -> &HashMap<String, ScopeBinding> {
        self.scopes.last().expect("作用域栈非空")
    }

    /// 当前作用域可变借用。
    fn current_scope_mut(&mut self) -> &mut HashMap<String, ScopeBinding> {
        self.scopes.last_mut().expect("作用域栈非空")
    }

    /// 读取当前源码中的名称。
    fn name_text(&self, name: Name) -> String {
        name.unquoted_text(self.source).to_owned()
    }

    /// 报告绑定冲突。
    fn binding_conflict(&mut self, span: SourceSpan, key: &str) {
        self.state.push_diagnostic(
            Some(self.module.clone()),
            span,
            IMPORT_BINDING_CONFLICT_CODE,
            "x05.module.import_binding_conflict",
            format!("当前作用域中的名称已被占用: {key}"),
            [("name".to_owned(), DiagnosticParam::Text(key.to_owned()))],
        );
    }

    /// 报告限定符误用。
    fn invalid_qualifier(&mut self, name: Name, detail: &str) {
        self.state.push_diagnostic(
            Some(self.module.clone()),
            name.span,
            INVALID_QUALIFIER_USE_CODE,
            "x05.module.invalid_qualifier_use",
            detail.to_owned(),
            [(
                "name".to_owned(),
                DiagnosticParam::Text(self.name_text(name)),
            )],
        );
    }
}

/// 将嵌套成员表达式展平成根名称和成员列表。
fn flatten_member_chain(expression: &Expression) -> Option<(Name, Vec<Name>)> {
    /// 递归收集成员链中的成员名称，并返回最左侧根名称。
    fn collect(expression: &Expression, members: &mut Vec<Name>) -> Option<Name> {
        match expression {
            Expression::Member { object, member, .. } => {
                members.push(*member);
                collect(object, members)
            }
            Expression::Name(name) => Some(*name),
            _ => None,
        }
    }
    let mut members = Vec::new();
    let root = collect(expression, &mut members)?;
    members.reverse();
    Some((root, members))
}
