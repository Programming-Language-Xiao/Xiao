//! 08-U0 统一前端流水线。
//!
//! 本模块只编排已经实现的前序分析器，并把它们的结果交给 `xiao-ir`。它不
//! 复制解析、类型或生命周期语义，也不执行配置和 Xiao 用户代码。前端错误
//! 会完整累积；只有无错误结果才会产生经过验证的 IR。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use crate::packages::PackageRegistry;
use xiao_config::NormalizedConfig;
use xiao_diagnostics::{Diagnostic, DiagnosticParam, Severity};
use xiao_ir::{IrProgram, IrValidationError, IrValidator, lower_program};
use xiao_lifetime::analyze as analyze_lifetime;
use xiao_modules::{ModuleName, ProjectModuleResult, analyze_project};
use xiao_source::SourceFile;
use xiao_syntax::{ImportStatement, ParseResult, Program, Statement, parse};
use xiao_types::{ExternalNamespaces, TypeCheckResult, TypeChecker, check};
use xiao_vm::STANDARD_LIBRARY_NAMESPACE;

/// 前端流水线版本；用于请求/结果协商和快照元数据。
pub const FRONTEND_VERSION: u32 = 1;

/// 可选的外部依赖图摘要。
///
/// 11A 包管理器尚未接入时，前端只保留身份字符串，不读取网络、不执行包
/// 配置。后续包解析器可以把同一结构替换为完整的已验证依赖图。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExternalModuleGraph {
    /// 已解析外部包的稳定身份列表。
    pub packages: Vec<String>,
}

impl ExternalModuleGraph {
    /// 创建一份外部包图摘要。
    #[must_use]
    pub fn new(packages: impl Into<Vec<String>>) -> Self {
        let mut packages = packages.into();
        packages.sort();
        packages.dedup();
        Self { packages }
    }
}

/// 前端项目上下文。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrontendContext {
    /// 一次执行固定使用的规范语言标签。
    pub locale: String,
    /// 可选项目根；提供时会运行本地模块发现和依赖解析。
    pub project_root: Option<PathBuf>,
    /// 已规范化的 `config.xiao`；前端只读取声明树，不执行配置值。
    pub config: Option<NormalizedConfig>,
    /// 是否提供了配置树的兼容性标记；新调用方优先使用 [`Self::config`]。
    pub config_present: bool,
    /// 可选外部包图摘要。
    pub external_modules: Option<ExternalModuleGraph>,
    /// 来自环境视图的包命名空间接口；仅用于静态类型检查。
    pub package_namespaces: Option<ExternalNamespaces>,
    /// 环境包的只读静态映射，不是 VM 初始化缓存。
    pub package_registry: Option<PackageRegistry>,
    /// 以模块身份编译时，需要在执行完顶层后导出的名称。
    pub module_exports: Vec<String>,
    /// 目标平台描述，默认使用 `host`。
    pub target: String,
    /// 语言版本，默认使用 `0.1.0`。
    pub language_version: String,
}

impl FrontendContext {
    /// 创建默认主机上下文。
    #[must_use]
    pub fn host() -> Self {
        Self {
            locale: "zh-CN".to_owned(),
            target: "host".to_owned(),
            language_version: "0.1.0".to_owned(),
            ..Self::default()
        }
    }

    /// 将规范化配置树附加到上下文。
    #[must_use]
    pub fn with_config(mut self, config: NormalizedConfig) -> Self {
        self.config = Some(config);
        self.config_present = true;
        self
    }

    /// 将外部包图摘要附加到上下文。
    #[must_use]
    pub fn with_external_modules(mut self, graph: ExternalModuleGraph) -> Self {
        self.external_modules = Some(graph);
        self
    }

    /// 设置一次执行固定使用的规范语言标签。
    #[must_use]
    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = locale.into();
        self
    }
}

/// 一次前端编译请求。
#[derive(Clone, Debug, PartialEq)]
pub struct FrontendRequest {
    /// 已验证 UTF-8 源码。
    pub source: SourceFile,
    /// 源文件路径；纯内存请求可以为空。
    pub source_path: Option<PathBuf>,
    /// 项目和目标上下文。
    pub context: FrontendContext,
}

impl FrontendRequest {
    /// 从源码文本创建内存请求。
    #[must_use]
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            source: SourceFile::new(text.into()),
            source_path: None,
            context: FrontendContext::host(),
        }
    }

    /// 从源码文本和路径创建请求。
    #[must_use]
    pub fn from_text_at(text: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            source: SourceFile::new(text.into()),
            source_path: Some(path.into()),
            context: FrontendContext::host(),
        }
    }

    /// 从磁盘读取 `.xiao` 源文件创建请求。
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, FrontendIoError> {
        let path = path.as_ref().to_path_buf();
        let bytes = fs::read(&path).map_err(|error| FrontendIoError {
            path: path.clone(),
            message: error.to_string(),
        })?;
        let source = SourceFile::from_bytes(bytes).map_err(|error| FrontendIoError {
            path: path.clone(),
            message: error.to_string(),
        })?;
        Ok(Self {
            source,
            source_path: Some(path),
            context: FrontendContext::host(),
        })
    }

    /// 设置项目上下文。
    #[must_use]
    pub fn with_context(mut self, context: FrontendContext) -> Self {
        self.context = context;
        self
    }
}

/// 前端源码读取错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontendIoError {
    /// 失败路径。
    pub path: PathBuf,
    /// 宿主错误摘要。
    pub message: String,
}

impl Display for FrontendIoError {
    /// 输出读取错误。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "无法读取源码 {}: {}",
            self.path.display(),
            self.message
        )
    }
}

impl std::error::Error for FrontendIoError {}

/// 首批纯 Xiao 函数的静态链接计划；运行时导入仍保留在入口 IR 中。
#[derive(Clone, Debug, Default)]
struct StandardLinkPlan {
    sources: BTreeMap<String, String>,
    import_spans: BTreeSet<(usize, usize)>,
}

/// 选择可安全静态链接的首批标准库函数导入。
fn plan_standard_link(
    program: &Program,
    source: &SourceFile,
    standard: &ProjectModuleResult,
) -> StandardLinkPlan {
    let local_functions = program
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Function { name, .. } => Some(name.unquoted_text(source).to_owned()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut result = StandardLinkPlan::default();
    for statement in &program.statements {
        let Statement::Import {
            import: ImportStatement::From {
                module, imports, ..
            },
            span,
            ..
        } = statement
        else {
            continue;
        };
        if module.segments.len() < 2
            || module.segments[0].unquoted_text(source) != STANDARD_LIBRARY_NAMESPACE
        {
            continue;
        }
        let module_name = module
            .segments
            .iter()
            .skip(1)
            .map(|name| name.unquoted_text(source).to_owned())
            .collect::<Vec<_>>();
        let module_name = ModuleName::new(module_name);
        let Some(record) = standard.modules.get(&module_name) else {
            continue;
        };
        let all_static_functions = imports.iter().all(|item| {
            if item.alias.is_some() {
                return false;
            }
            let name = item.name.unquoted_text(source);
            !local_functions.contains(name)
                && record
                    .symbols
                    .get(name)
                    .is_some_and(|symbol| symbol.kind == xiao_modules::ModuleSymbolKind::Function)
        });
        if !all_static_functions {
            continue;
        }
        let Some(module_source) = &record.source else {
            continue;
        };
        let identity = module_name.to_string();
        result
            .sources
            .entry(identity)
            .or_insert_with(|| module_source.text().to_owned());
        result.import_spans.insert((span.start(), span.end()));
    }
    result
}

/// 在调用源码后追加被静态导入的标准库函数，并移除仅供静态分析的导入节点。
fn link_standard_sources(source: &SourceFile, plan: &StandardLinkPlan) -> (SourceFile, Program) {
    let mut text = source.text().to_owned();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    for module_source in plan.sources.values() {
        text.push_str(module_source);
        if !module_source.ends_with('\n') {
            text.push('\n');
        }
    }
    let linked_source = SourceFile::new(text);
    let parsed = parse(&linked_source);
    let mut program = parsed
        .program
        .expect("standard library sources are validated before static linking");
    program.statements.retain(|statement| {
        !plan
            .import_spans
            .contains(&(statement.span().start(), statement.span().end()))
    });
    (linked_source, program)
}

/// 把静态检查时移除的官方导入语句还原到最终 IR，供 VM 加载专用命名空间。
fn restore_standard_imports(
    linked: &mut IrProgram,
    original: IrProgram,
    original_program: &Program,
    import_spans: &BTreeSet<(usize, usize)>,
) {
    let mut linked_statements = linked
        .body
        .drain(..)
        .map(|statement| ((statement.span.start, statement.span.end), statement))
        .collect::<BTreeMap<_, _>>();
    let original_imports = original
        .body
        .into_iter()
        .filter(|statement| import_spans.contains(&(statement.span.start, statement.span.end)))
        .map(|statement| ((statement.span.start, statement.span.end), statement))
        .collect::<BTreeMap<_, _>>();
    let mut body = Vec::with_capacity(linked_statements.len() + original_imports.len());
    for statement in &original_program.statements {
        let key = (statement.span().start(), statement.span().end());
        if import_spans.contains(&key) {
            if let Some(import) = original_imports.get(&key) {
                body.push(import.clone());
            }
        } else if let Some(statement) = linked_statements.remove(&key) {
            body.push(statement);
        }
    }
    body.extend(linked_statements.into_values());
    linked.body = body;
}

/// 前端成功产物。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontendArtifact {
    /// 经过验证的类型化 IR。
    pub ir: IrProgram,
    /// 非错误级别的前端诊断（例如警告）。
    pub diagnostics: Vec<Diagnostic>,
}

impl FrontendArtifact {
    /// 返回 IR 的只读视图。
    #[must_use]
    pub const fn ir(&self) -> &IrProgram {
        &self.ir
    }

    /// 返回保留的诊断。
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// 前端失败结果；包含所有已收集的结构化诊断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontendError {
    /// 按流水线阶段和源码顺序累积的诊断。
    pub diagnostics: Vec<Diagnostic>,
}

impl FrontendError {
    /// 判断是否包含错误诊断。
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// 返回诊断只读视图。
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl Display for FrontendError {
    /// 输出首条错误摘要。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(diagnostic) = self.diagnostics.iter().find(|item| item.is_error()) {
            write!(formatter, "{}: {}", diagnostic.code(), diagnostic.message())
        } else {
            formatter.write_str("前端流水线失败")
        }
    }
}

impl std::error::Error for FrontendError {}

/// 统一前端编译器。
#[derive(Clone, Copy, Debug, Default)]
pub struct FrontendCompiler;

impl FrontendCompiler {
    /// 创建无状态前端编译器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// 执行一次完整前端流水线。
    ///
    /// 所有阶段均只接收不可变输入；只要任意阶段产生错误，函数就返回完整
    /// 诊断而不降低 IR。成功时返回的 IR 已通过 `IrValidator`。
    pub fn compile(&self, request: &FrontendRequest) -> Result<FrontendArtifact, FrontendError> {
        let mut diagnostics = Vec::new();

        let parsed = parse(&request.source);
        diagnostics.extend(parsed.diagnostics.clone());
        let mut project = request.context.project_root.as_deref().map(analyze_project);
        let standard_library_requested = parsed
            .program
            .as_ref()
            .is_some_and(|program| imports_standard_library(program, &request.source))
            || project
                .as_ref()
                .is_some_and(project_imports_standard_library);
        let Some(program) = parsed.program.as_ref() else {
            if let Some(project) = &project {
                diagnostics.extend(
                    project
                        .diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.diagnostic.clone()),
                );
            }
            return Err(FrontendError { diagnostics });
        };
        if project
            .as_ref()
            .is_some_and(uses_reserved_standard_library_namespace)
        {
            diagnostics.push(standard_library_namespace_conflict_diagnostic(
                "项目不得声明",
            ));
            return Err(FrontendError { diagnostics });
        }
        let package_namespaces = request.context.package_namespaces.as_ref().or_else(|| {
            request
                .context
                .package_registry
                .as_ref()
                .map(|registry| &registry.namespaces)
        });
        if package_namespaces.is_some_and(uses_reserved_standard_library_namespace_external) {
            diagnostics.push(standard_library_namespace_conflict_diagnostic(
                "普通包不得声明",
            ));
            return Err(FrontendError { diagnostics });
        }
        if request.context.project_root.is_none()
            && standard_library_requested
            && is_project_import_shadowing_standard_library(program, &request.source)
        {
            diagnostics.push(Diagnostic::new(
                "X05-MODULE-003",
                "x05.module.file_namespace_conflict",
                Severity::Error,
                None,
                format!("入口不得把项目模块绑定覆盖为保留的官方标准库命名空间 {STANDARD_LIBRARY_NAMESPACE}"),
            ));
            return Err(FrontendError { diagnostics });
        }
        let mut standard_link = StandardLinkPlan::default();
        let mut standard_library = None;
        if standard_library_requested {
            let standard = analyze_project(crate::module_loader::standard_library_root());
            diagnostics.extend(
                standard
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.diagnostic.clone()),
            );
            if standard.is_success() {
                standard_library = Some(standard.clone());
                diagnostics.extend(standard_library_import_diagnostics(
                    program,
                    &request.source,
                    &standard,
                ));
                if let Some(project) = &project {
                    let entry_path = request
                        .source_path
                        .as_ref()
                        .and_then(|path| fs::canonicalize(path).ok());
                    for record in project.modules.values() {
                        let Some(module_program) = record.program.as_ref() else {
                            continue;
                        };
                        let Some(module_source) = record.source.as_ref() else {
                            continue;
                        };
                        let module_path = fs::canonicalize(&record.path).ok();
                        if entry_path.is_some() && entry_path == module_path {
                            continue;
                        }
                        diagnostics.extend(standard_library_import_diagnostics(
                            module_program,
                            module_source,
                            &standard,
                        ));
                    }
                }
                if diagnostics.iter().any(Diagnostic::is_error) {
                    return Err(FrontendError { diagnostics });
                }
                standard_link = plan_standard_link(program, &request.source, &standard);
                project = Some(merge_standard_library(project.take(), standard));
            }
        }
        if let Some(project) = &project {
            diagnostics.extend(
                project
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| {
                        if !standard_library_requested {
                            return true;
                        }
                        // 标准库索引在专用检查中校验入口和项目所有模块，只过滤
                        // 已存在目标的项目镜像；未知目标由专用检查返回稳定错误。
                        !standard_library.as_ref().is_some_and(|standard| {
                            is_unresolved_standard_library_import(&diagnostic.diagnostic)
                                && standard_import_target_exists(&diagnostic.diagnostic, standard)
                        })
                    })
                    .map(|diagnostic| diagnostic.diagnostic.clone()),
            );
        }
        let (effective_source, effective_program) = if standard_link.sources.is_empty() {
            (request.source.clone(), program.clone())
        } else {
            link_standard_sources(&request.source, &standard_link)
        };
        let namespaces = request.context.package_namespaces.as_ref().or_else(|| {
            request
                .context
                .package_registry
                .as_ref()
                .map(|registry| &registry.namespaces)
        });
        let type_result = if let Some(namespaces) = namespaces {
            TypeChecker::new(&effective_source)
                .with_external_namespaces(namespaces.clone())
                .check_program(&effective_program)
        } else {
            check(&effective_source, &effective_program)
        };
        diagnostics.extend(type_result.diagnostics.clone());

        let lifetime = analyze_lifetime(&effective_source, &effective_program, &type_result);
        diagnostics.extend(lifetime.diagnostics.clone());

        if diagnostics.iter().any(Diagnostic::is_error) {
            return Err(FrontendError { diagnostics });
        }

        let mut ir = lower_program(
            &effective_source,
            &effective_program,
            &type_result,
            &lifetime,
            project.as_ref(),
        );
        if !standard_link.import_spans.is_empty() {
            let original_ir = lower_program(
                &request.source,
                program,
                &type_result,
                &lifetime,
                project.as_ref(),
            );
            restore_standard_imports(&mut ir, original_ir, program, &standard_link.import_spans);
        }
        ir.language_version = if request.context.language_version.is_empty() {
            "0.1.0".to_owned()
        } else {
            request.context.language_version.clone()
        };
        ir.target = if request.context.target.is_empty() {
            "host".to_owned()
        } else {
            request.context.target.clone()
        };
        ir.config_present = request.context.config.is_some() || request.context.config_present;
        ir.external_packages = request
            .context
            .external_modules
            .as_ref()
            .map(|graph| graph.packages.clone())
            .unwrap_or_default();
        ir.package_roots = request.context.package_registry.as_ref().map_or_else(
            || {
                request
                    .context
                    .package_namespaces
                    .as_ref()
                    .map_or_else(Vec::new, |namespaces| {
                        namespaces
                            .members
                            .keys()
                            .filter(|path| !path.contains('.'))
                            .cloned()
                            .collect()
                    })
            },
            |registry| {
                registry
                    .namespaces
                    .members
                    .keys()
                    .filter(|path| !path.contains('.'))
                    .cloned()
                    .collect()
            },
        );
        ir.module_exports = request.context.module_exports.clone();

        let validation = IrValidator::new().validate(&ir);
        if !validation.is_success() {
            diagnostics.extend(validation.errors.iter().map(validation_diagnostic));
            return Err(FrontendError { diagnostics });
        }
        Ok(FrontendArtifact { ir, diagnostics })
    }
}

/// 仅识别项目模块分析阶段对保留标准库根产生的“目标不存在”诊断。
fn is_unresolved_standard_library_import(diagnostic: &Diagnostic) -> bool {
    diagnostic.code() == "X05-MODULE-004"
        && matches!(
            diagnostic.params().get("target"),
            Some(DiagnosticParam::Text(target))
                if target == STANDARD_LIBRARY_NAMESPACE
                    || target.starts_with("stdlib.")
        )
}

/// 判断项目索引中的任一源码模块是否引用保留的官方标准库。
fn project_imports_standard_library(project: &ProjectModuleResult) -> bool {
    project.modules.values().any(|record| {
        record
            .program
            .as_ref()
            .zip(record.source.as_ref())
            .is_some_and(|(program, source)| imports_standard_library(program, source))
    })
}

/// 判断项目模块分析的标准库目标是否真实存在于工具链源码索引。
fn standard_import_target_exists(diagnostic: &Diagnostic, standard: &ProjectModuleResult) -> bool {
    let Some(DiagnosticParam::Text(target)) = diagnostic.params().get("target") else {
        return false;
    };
    if target == STANDARD_LIBRARY_NAMESPACE {
        return true;
    }
    let Some(suffix) = target.strip_prefix("stdlib.") else {
        return false;
    };
    standard_target_exists(
        standard,
        &ModuleName::new(suffix.split('.').map(str::to_owned)),
    )
}

/// 判断标准库索引中是否存在一个模块或目录命名空间。
fn standard_target_exists(standard: &ProjectModuleResult, name: &ModuleName) -> bool {
    name.is_root() || standard.modules.contains_key(name) || standard.namespaces.contains_key(name)
}

/// 静态验证保留标准库命名空间中的模块与导出名称。
fn standard_library_import_diagnostics(
    program: &Program,
    source: &SourceFile,
    standard: &ProjectModuleResult,
) -> Vec<Diagnostic> {
    /// 构造缺少标准库模块的稳定诊断。
    fn target_diagnostic(name: &ModuleName, span: xiao_source::SourceSpan) -> Diagnostic {
        let target = format!("{STANDARD_LIBRARY_NAMESPACE}.{name}");
        Diagnostic::error_at(
            "X05-MODULE-004",
            "x05.module.missing_import_target",
            span,
            format!("导入目标不存在: {target}"),
        )
        .with_params([("target".to_owned(), DiagnosticParam::Text(target))])
    }

    /// 构造缺少标准库导出符号的稳定诊断。
    fn symbol_diagnostic(
        module: &ModuleName,
        symbol: &str,
        span: xiao_source::SourceSpan,
    ) -> Diagnostic {
        let target = format!("{STANDARD_LIBRARY_NAMESPACE}.{module}");
        Diagnostic::error_at(
            "X05-MODULE-005",
            "x05.module.missing_import_symbol",
            span,
            format!("标准库模块 {target} 未导出名称 {symbol}"),
        )
        .with_params([
            ("target".to_owned(), DiagnosticParam::Text(target)),
            (
                "symbol".to_owned(),
                DiagnosticParam::Text(symbol.to_owned()),
            ),
        ])
    }

    /// 递归检查所有控制流位置的标准库导入。
    fn walk(
        statements: &[Statement],
        source: &SourceFile,
        standard: &ProjectModuleResult,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for statement in statements {
            match statement {
                Statement::Import { import, .. } => match import {
                    ImportStatement::Modules { imports, .. } => {
                        for item in imports {
                            let Some(name) = standard_import_module(&item.path.segments, source)
                            else {
                                continue;
                            };
                            if !standard_target_exists(standard, &name) {
                                diagnostics.push(target_diagnostic(&name, item.path.span));
                            }
                        }
                    }
                    ImportStatement::From {
                        module, imports, ..
                    } => {
                        let Some(name) = standard_import_module(&module.segments, source) else {
                            continue;
                        };
                        if !standard_target_exists(standard, &name) {
                            diagnostics.push(target_diagnostic(&name, module.span));
                            continue;
                        }
                        if let Some(record) = standard.modules.get(&name) {
                            for item in imports {
                                let symbol = item.name.unquoted_text(source);
                                if !record.symbols.contains_key(symbol) {
                                    diagnostics.push(symbol_diagnostic(&name, symbol, item.span));
                                }
                            }
                        } else {
                            for item in imports {
                                let child = name.child(item.name.unquoted_text(source).to_owned());
                                if !standard_target_exists(standard, &child) {
                                    diagnostics.push(target_diagnostic(&child, item.span));
                                }
                            }
                        }
                    }
                },
                Statement::Function { body, .. }
                | Statement::Table { body, .. }
                | Statement::For { body, .. }
                | Statement::While { body, .. } => walk(body, source, standard, diagnostics),
                Statement::If {
                    body,
                    elif_branches,
                    else_body,
                    ..
                } => {
                    walk(body, source, standard, diagnostics);
                    for branch in elif_branches {
                        walk(&branch.body, source, standard, diagnostics);
                    }
                    if let Some(body) = else_body {
                        walk(body, source, standard, diagnostics);
                    }
                }
                Statement::Try {
                    body,
                    catches,
                    finally_body,
                    ..
                } => {
                    walk(body, source, standard, diagnostics);
                    for catch in catches {
                        walk(&catch.body, source, standard, diagnostics);
                    }
                    if let Some(body) = finally_body {
                        walk(body, source, standard, diagnostics);
                    }
                }
                _ => {}
            }
        }
    }

    let mut diagnostics = Vec::new();
    walk(&program.statements, source, standard, &mut diagnostics);
    diagnostics
}

/// 解析并去除模块路径中的保留标准库根名称。
fn standard_import_module(names: &[xiao_syntax::Name], source: &SourceFile) -> Option<ModuleName> {
    if names.first()?.unquoted_text(source) != STANDARD_LIBRARY_NAMESPACE {
        return None;
    }
    Some(ModuleName::new(
        names
            .iter()
            .skip(1)
            .map(|name| name.unquoted_text(source).to_owned()),
    ))
}

/// 判断项目是否自行占用了官方标准库保留根及其任一子模块。
fn uses_reserved_standard_library_namespace(project: &ProjectModuleResult) -> bool {
    let root = ModuleName::new([STANDARD_LIBRARY_NAMESPACE.to_owned()]);
    project.modules.keys().any(|name| root.is_prefix_of(name))
        || project
            .namespaces
            .keys()
            .any(|name| root.is_prefix_of(name))
}

/// 判断普通包或外部命名空间是否占用了官方标准库保留根。
fn uses_reserved_standard_library_namespace_external(namespaces: &ExternalNamespaces) -> bool {
    namespaces.members.keys().any(|name| {
        name == STANDARD_LIBRARY_NAMESPACE
            || name
                .strip_prefix(STANDARD_LIBRARY_NAMESPACE)
                .is_some_and(|suffix| suffix.starts_with('.'))
    })
}

/// 构造统一的标准库保留命名空间冲突诊断。
fn standard_library_namespace_conflict_diagnostic(subject: &str) -> Diagnostic {
    Diagnostic::new(
        "X05-MODULE-003",
        "x05.module.file_namespace_conflict",
        Severity::Error,
        None,
        format!("{subject}保留的官方标准库命名空间 {STANDARD_LIBRARY_NAMESPACE}"),
    )
}

/// 判断入口是否把常规项目/包导入误当成标准库模块来覆盖。
fn is_project_import_shadowing_standard_library(program: &Program, source: &SourceFile) -> bool {
    let mut aliases = BTreeSet::new();
    for statement in &program.statements {
        let Statement::Import { import, .. } = statement else {
            continue;
        };
        if let ImportStatement::Modules { imports, .. } = import {
            for item in imports {
                if item
                    .path
                    .segments
                    .first()
                    .is_some_and(|name| name.unquoted_text(source) != STANDARD_LIBRARY_NAMESPACE)
                    && item.alias.is_some_and(|alias| {
                        alias.unquoted_text(source) == STANDARD_LIBRARY_NAMESPACE
                    })
                {
                    aliases.insert(STANDARD_LIBRARY_NAMESPACE.to_owned());
                }
            }
        }
    }
    !aliases.is_empty()
}

/// 判断程序是否引用保留的官方标准库命名空间。
fn imports_standard_library(program: &Program, source: &SourceFile) -> bool {
    /// 递归查找一组语句中的保留标准库导入。
    fn statements_import_standard(statements: &[Statement], source: &SourceFile) -> bool {
        statements.iter().any(|statement| match statement {
            Statement::Import { import, .. } => match import {
                ImportStatement::Modules { imports, .. } => imports.iter().any(|item| {
                    item.path.segments.first().is_some_and(|name| {
                        name.unquoted_text(source) == STANDARD_LIBRARY_NAMESPACE
                    })
                }),
                ImportStatement::From { module, .. } => module
                    .segments
                    .first()
                    .is_some_and(|name| name.unquoted_text(source) == STANDARD_LIBRARY_NAMESPACE),
            },
            Statement::Function { body, .. }
            | Statement::Table { body, .. }
            | Statement::For { body, .. }
            | Statement::While { body, .. } => statements_import_standard(body, source),
            Statement::If {
                body,
                elif_branches,
                else_body,
                ..
            } => {
                statements_import_standard(body, source)
                    || elif_branches
                        .iter()
                        .any(|branch| statements_import_standard(&branch.body, source))
                    || else_body
                        .as_deref()
                        .is_some_and(|body| statements_import_standard(body, source))
            }
            Statement::Try {
                body,
                catches,
                finally_body,
                ..
            } => {
                statements_import_standard(body, source)
                    || catches
                        .iter()
                        .any(|catch| statements_import_standard(&catch.body, source))
                    || finally_body
                        .as_deref()
                        .is_some_and(|body| statements_import_standard(body, source))
            }
            _ => false,
        })
    }

    statements_import_standard(&program.statements, source)
}

/// 把标准库源码项目映射到保留命名空间下，并合入当前项目的模块索引。
fn merge_standard_library(
    project: Option<ProjectModuleResult>,
    standard: ProjectModuleResult,
) -> ProjectModuleResult {
    let prefix = ModuleName::new([STANDARD_LIBRARY_NAMESPACE.to_owned()]);
    let modules = standard
        .modules
        .into_iter()
        .map(|(name, mut record)| {
            let name = prefix_module_name(&prefix, &name);
            record.name = name.clone();
            (name, record)
        })
        .collect::<BTreeMap<_, _>>();
    let namespaces = standard
        .namespaces
        .into_iter()
        .map(|(name, mut record)| {
            let name = prefix_module_name(&prefix, &name);
            record.name = name.clone();
            (name, record)
        })
        .collect::<BTreeMap<_, _>>();
    let standard = ProjectModuleResult {
        project_root: standard.project_root,
        modules,
        namespaces,
        graph: Default::default(),
        bindings: Vec::new(),
        diagnostics: standard.diagnostics,
    };
    let Some(mut project) = project else {
        return standard;
    };
    project.modules.extend(standard.modules);
    project.namespaces.extend(standard.namespaces);
    project
}

/// 将一个标准库模块名放到保留根命名空间下。
fn prefix_module_name(prefix: &ModuleName, name: &ModuleName) -> ModuleName {
    ModuleName::new(
        prefix
            .segments()
            .iter()
            .cloned()
            .chain(name.segments().iter().cloned()),
    )
}

/// 便捷前端编译函数。
pub fn compile(request: &FrontendRequest) -> Result<FrontendArtifact, FrontendError> {
    FrontendCompiler::new().compile(request)
}

/// 将 IR 验证错误转为统一结构化诊断。
fn validation_diagnostic(error: &IrValidationError) -> Diagnostic {
    let span = error
        .span
        .and_then(|span| xiao_source::SourceSpan::new(span.start, span.end));
    Diagnostic::new(
        error.code,
        "x08.ir.invalid",
        Severity::Error,
        span,
        error.message.clone(),
    )
    .with_params([("path".to_owned(), DiagnosticParam::Text(error.path.clone()))])
}

// 保留这些类型名称作为文档和 IDE 的阶段边界锚点；实际结果由公开前序 crate
// 提供，不能在 driver 中复制一份。
#[allow(dead_code)]
/// 解析结果的前端阶段边界类型别名。
type _ParseBoundary = ParseResult;
#[allow(dead_code)]
/// 程序 AST 的前端阶段边界类型别名。
type _ProgramBoundary = Program;
#[allow(dead_code)]
/// 类型检查结果的前端阶段边界类型别名。
type _TypeBoundary = TypeCheckResult;
#[allow(dead_code)]
/// 模块分析结果的前端阶段边界类型别名。
type _ModuleBoundary = ProjectModuleResult;

/// 构建一个空的确定性类型结果映射；供测试夹具和后续多模块降低使用。
#[must_use]
pub fn empty_type_results() -> BTreeMap<String, TypeCheckResult> {
    BTreeMap::new()
}
