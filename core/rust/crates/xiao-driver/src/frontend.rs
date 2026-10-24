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
use xiao_syntax::{
    Expression, FunctionParameterKind, FunctionTypeAnnotation, ImportStatement, ParseResult,
    Program, Statement, TypeTerm, parse,
};
use xiao_types::{
    ExternalNamespaces, FunctionParameterSignature, FunctionSignature, SetType, Type,
    TypeCheckResult, TypeChecker, TypeVarId,
};
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
    /// 归档编译保留官方导入，只记录宿主要求，不静态复制标准库函数。
    pub external_standard_library: bool,
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

/// 集合方法的规范名称；方法调用会隐式引入 `stdlib.set` 的纯 Xiao 函数体。
const SET_METHOD_NAMES: [&str; 8] = [
    "size",
    "is_empty",
    "add",
    "remove",
    "contains",
    "union",
    "intersection",
    "difference",
];

/// 方法静态链接发生顶层同名函数冲突时使用的内部标准库函数前缀。
const SET_METHOD_FUNCTION_PREFIX: &str = "__xiao_stdlib_set_";

/// 字典方法静态链接使用的内部标准库函数前缀。
const DICT_METHOD_FUNCTION_PREFIX: &str = "__xiao_stdlib_dict_";

/// 字典方法的规范名称。
const DICT_METHOD_NAMES: [&str; 8] = [
    "get", "set", "remove", "contains", "keys", "values", "size", "is_empty",
];

/// 只重命名标准库源码中的标识符，不触碰字符串、注释和反引号名称。
///
/// 方法调用没有显式导入绑定，因此需要把 `stdlib.set` 的函数体接入入口 IR。
/// 若用户同时声明了同名顶层函数，直接拼接会触发重复函数诊断；把受控标准库
/// 源码的八个函数及其相互调用改成内部名称，可以保留用户函数的正常可见性。
fn rename_set_module_functions(source: &str) -> String {
    fn is_identifier_start(character: char) -> bool {
        character == '_' || character.is_ascii_alphabetic()
    }

    fn is_identifier_continue(character: char) -> bool {
        character == '_' || character.is_ascii_alphanumeric()
    }

    let mut output = String::with_capacity(source.len() + 8 * SET_METHOD_FUNCTION_PREFIX.len());
    let mut characters = source.char_indices().peekable();
    while let Some((_, character)) = characters.next() {
        if character == '"' || character == '\'' {
            let quote = character;
            output.push(character);
            let mut escaped = false;
            for (_, next) in characters.by_ref() {
                output.push(next);
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == quote {
                    break;
                }
            }
            continue;
        }
        if character == '#' {
            output.push(character);
            for (_, next) in characters.by_ref() {
                output.push(next);
                if next == '\n' {
                    break;
                }
            }
            continue;
        }
        if character == '`' {
            output.push(character);
            for (_, next) in characters.by_ref() {
                output.push(next);
                if next == '`' {
                    break;
                }
            }
            continue;
        }
        if !is_identifier_start(character) {
            output.push(character);
            continue;
        }
        let mut identifier = String::from(character);
        while let Some((_, next)) = characters.peek().copied() {
            if !is_identifier_continue(next) {
                break;
            }
            characters.next();
            identifier.push(next);
        }
        if SET_METHOD_NAMES.contains(&identifier.as_str()) {
            output.push_str(SET_METHOD_FUNCTION_PREFIX);
        }
        output.push_str(&identifier);
    }
    output
}

/// 为静态链接隔离 `stdlib.dict` 的自由函数名称。
/// 为静态链接隔离 `stdlib.dict` 的公开函数名称。
fn rename_dict_module_functions(source: &str) -> String {
    fn is_identifier_start(character: char) -> bool {
        character == '_' || character.is_ascii_alphabetic()
    }
    fn is_identifier_continue(character: char) -> bool {
        character == '_' || character.is_ascii_alphanumeric()
    }
    let mut output = String::with_capacity(source.len() + 8 * DICT_METHOD_FUNCTION_PREFIX.len());
    let mut characters = source.char_indices().peekable();
    while let Some((_, character)) = characters.next() {
        if character == '"' || character == '\'' {
            let quote = character;
            output.push(character);
            let mut escaped = false;
            for (_, next) in characters.by_ref() {
                output.push(next);
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == quote {
                    break;
                }
            }
            continue;
        }
        if character == '#' {
            output.push(character);
            for (_, next) in characters.by_ref() {
                output.push(next);
                if next == '\n' {
                    break;
                }
            }
            continue;
        }
        if character == '`' {
            output.push(character);
            for (_, next) in characters.by_ref() {
                output.push(next);
                if next == '`' {
                    break;
                }
            }
            continue;
        }
        if !is_identifier_start(character) {
            output.push(character);
            continue;
        }
        let mut identifier = String::from(character);
        while let Some((_, next)) = characters.peek().copied() {
            if !is_identifier_continue(next) {
                break;
            }
            characters.next();
            identifier.push(next);
        }
        if DICT_METHOD_NAMES.contains(&identifier.as_str()) {
            output.push_str(DICT_METHOD_FUNCTION_PREFIX);
        }
        output.push_str(&identifier);
    }
    output
}

/// 判断表达式树中是否出现集合方法形状；接收者的静态集合类型留给类型检查器确认。
fn expression_uses_set_method(
    expression: &Expression,
    source: &SourceFile,
    types: Option<&TypeCheckResult>,
) -> bool {
    match expression {
        Expression::Call {
            callee, arguments, ..
        } => {
            let direct = match callee.as_ref() {
                Expression::Member { object, member, .. }
                    if !member.backticked
                        && SET_METHOD_NAMES.contains(&member.unquoted_text(source)) =>
                {
                    types.is_none_or(|types| {
                        types
                            .type_at(object.span())
                            .is_some_and(|ty| matches!(ty, Type::Set(_)))
                    })
                }
                _ => false,
            };
            direct
                || expression_uses_set_method(callee, source, types)
                || arguments
                    .iter()
                    .any(|argument| expression_uses_set_method(&argument.value, source, types))
        }
        Expression::ArrayLiteral { elements, .. }
        | Expression::TupleLiteral { elements, .. }
        | Expression::SetLiteral { elements, .. } => elements
            .iter()
            .any(|element| expression_uses_set_method(element, source, types)),
        Expression::DictTableLiteral { entries, .. }
        | Expression::DictColumnLiteral { entries, .. } => entries
            .iter()
            .any(|entry| expression_uses_set_method(&entry.value, source, types)),
        Expression::Group { expression, .. }
        | Expression::Cast { expression, .. }
        | Expression::Unary {
            operand: expression,
            ..
        } => expression_uses_set_method(expression, source, types),
        Expression::Binary { left, right, .. } => {
            expression_uses_set_method(left, source, types)
                || expression_uses_set_method(right, source, types)
        }
        Expression::NewCall {
            callee, arguments, ..
        } => {
            expression_uses_set_method(callee, source, types)
                || arguments
                    .iter()
                    .any(|argument| expression_uses_set_method(&argument.value, source, types))
        }
        Expression::Member { object, .. } => expression_uses_set_method(object, source, types),
        Expression::Selector {
            source: object,
            step,
            selector,
            ..
        } => {
            expression_uses_set_method(object, source, types)
                || step
                    .as_deref()
                    .is_some_and(|step| expression_uses_set_method(step, source, types))
                || selector.items.iter().any(|item| {
                    matches!(
                        item,
                        xiao_syntax::SelectorItem::ExactExpression { expression, .. }
                            if expression_uses_set_method(expression, source, types)
                    )
                })
        }
        Expression::Literal { .. } | Expression::Name(_) => false,
    }
}

/// 递归检查语句体，使函数、表和错误控制流中的集合方法都进入标准库链接计划。
fn statements_use_set_method(
    statements: &[Statement],
    source: &SourceFile,
    types: Option<&TypeCheckResult>,
) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Expression { expression, .. }
        | Statement::Assignment {
            value: expression, ..
        }
        | Statement::ConstDeclaration {
            value: expression, ..
        }
        | Statement::Raise {
            value: expression, ..
        } => expression_uses_set_method(expression, source, types),
        Statement::ExtendedAssignment { target, value, .. } => {
            expression_uses_set_method(target, source, types)
                || expression_uses_set_method(value, source, types)
        }
        Statement::Declaration { value, .. } => value
            .as_ref()
            .is_some_and(|value| expression_uses_set_method(value, source, types)),
        Statement::Table { body, .. } => statements_use_set_method(body, source, types),
        Statement::Function {
            parameters, body, ..
        } => {
            parameters.iter().any(|parameter| {
                parameter
                    .default
                    .as_ref()
                    .is_some_and(|default| expression_uses_set_method(default, source, types))
            }) || statements_use_set_method(body, source, types)
        }
        Statement::If {
            condition,
            body,
            elif_branches,
            else_body,
            ..
        } => {
            expression_uses_set_method(condition, source, types)
                || statements_use_set_method(body, source, types)
                || elif_branches.iter().any(|branch| {
                    expression_uses_set_method(&branch.condition, source, types)
                        || statements_use_set_method(&branch.body, source, types)
                })
                || else_body
                    .as_deref()
                    .is_some_and(|body| statements_use_set_method(body, source, types))
        }
        Statement::For { iterable, body, .. }
        | Statement::While {
            condition: iterable,
            body,
            ..
        } => {
            expression_uses_set_method(iterable, source, types)
                || statements_use_set_method(body, source, types)
        }
        Statement::Return { value, .. } => value
            .as_ref()
            .is_some_and(|value| expression_uses_set_method(value, source, types)),
        Statement::Try {
            body,
            catches,
            finally_body,
            ..
        } => {
            statements_use_set_method(body, source, types)
                || catches
                    .iter()
                    .any(|catch| statements_use_set_method(&catch.body, source, types))
                || finally_body
                    .as_deref()
                    .is_some_and(|body| statements_use_set_method(body, source, types))
        }
        Statement::Import { .. } | Statement::Break { .. } | Statement::Continue { .. } => false,
    })
}

/// 判断入口是否使用集合方法；该事实驱动标准库源码静态链接和类型接口注入。
fn program_uses_set_method(program: &Program, source: &SourceFile) -> bool {
    statements_use_set_method(&program.statements, source, None)
}

/// 类型检查完成后，仅把真正作用于静态集合的调用纳入标准库静态链接。
fn program_uses_typed_set_method(
    program: &Program,
    source: &SourceFile,
    types: &TypeCheckResult,
) -> bool {
    statements_use_set_method(&program.statements, source, Some(types))
}

/// 判断表达式树中是否出现静态字典方法调用。
/// 判断表达式树是否包含字典方法调用。
fn expression_uses_dict_method(
    expression: &Expression,
    source: &SourceFile,
    types: Option<&TypeCheckResult>,
) -> bool {
    match expression {
        Expression::Call {
            callee, arguments, ..
        } => {
            let direct = match callee.as_ref() {
                Expression::Member { object, member, .. }
                    if !member.backticked
                        && DICT_METHOD_NAMES.contains(&member.unquoted_text(source)) =>
                {
                    types.is_none_or(|types| {
                        types.type_at(object.span()).is_some_and(|ty| {
                            matches!(ty, Type::DictTable(_) | Type::DictColumn(_))
                        })
                    })
                }
                _ => false,
            };
            direct
                || expression_uses_dict_method(callee, source, types)
                || arguments
                    .iter()
                    .any(|argument| expression_uses_dict_method(&argument.value, source, types))
        }
        Expression::ArrayLiteral { elements, .. }
        | Expression::TupleLiteral { elements, .. }
        | Expression::SetLiteral { elements, .. } => elements
            .iter()
            .any(|element| expression_uses_dict_method(element, source, types)),
        Expression::DictTableLiteral { entries, .. }
        | Expression::DictColumnLiteral { entries, .. } => entries
            .iter()
            .any(|entry| expression_uses_dict_method(&entry.value, source, types)),
        Expression::Group { expression, .. }
        | Expression::Cast { expression, .. }
        | Expression::Unary {
            operand: expression,
            ..
        } => expression_uses_dict_method(expression, source, types),
        Expression::Binary { left, right, .. } => {
            expression_uses_dict_method(left, source, types)
                || expression_uses_dict_method(right, source, types)
        }
        Expression::Member { object, .. } => expression_uses_dict_method(object, source, types),
        Expression::Selector {
            source: source_expression,
            step,
            ..
        } => {
            expression_uses_dict_method(source_expression, source, types)
                || step
                    .as_deref()
                    .is_some_and(|step| expression_uses_dict_method(step, source, types))
        }
        Expression::Literal { .. } | Expression::Name(_) => false,
        Expression::NewCall { arguments, .. } => arguments
            .iter()
            .any(|argument| expression_uses_dict_method(&argument.value, source, types)),
    }
}

/// 递归判断语句列表中是否出现静态字典方法调用。
/// 递归判断语句列表是否包含字典方法调用。
fn statements_use_dict_method(
    statements: &[Statement],
    source: &SourceFile,
    types: Option<&TypeCheckResult>,
) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Expression { expression, .. }
        | Statement::Assignment {
            value: expression, ..
        }
        | Statement::ConstDeclaration {
            value: expression, ..
        } => expression_uses_dict_method(expression, source, types),
        Statement::Declaration { value, .. } => value
            .as_ref()
            .is_some_and(|value| expression_uses_dict_method(value, source, types)),
        Statement::ExtendedAssignment { target, value, .. } => {
            expression_uses_dict_method(target, source, types)
                || expression_uses_dict_method(value, source, types)
        }
        Statement::Table { body, .. } => statements_use_dict_method(body, source, types),
        Statement::Function {
            body, parameters, ..
        } => {
            parameters.iter().any(|parameter| {
                parameter
                    .default
                    .as_ref()
                    .is_some_and(|default| expression_uses_dict_method(default, source, types))
            }) || statements_use_dict_method(body, source, types)
        }
        Statement::If {
            condition,
            body,
            elif_branches,
            else_body,
            ..
        } => {
            expression_uses_dict_method(condition, source, types)
                || statements_use_dict_method(body, source, types)
                || elif_branches.iter().any(|branch| {
                    expression_uses_dict_method(&branch.condition, source, types)
                        || statements_use_dict_method(&branch.body, source, types)
                })
                || else_body
                    .as_deref()
                    .is_some_and(|body| statements_use_dict_method(body, source, types))
        }
        Statement::For { iterable, body, .. }
        | Statement::While {
            condition: iterable,
            body,
            ..
        } => {
            expression_uses_dict_method(iterable, source, types)
                || statements_use_dict_method(body, source, types)
        }
        Statement::Return { value, .. } => value
            .as_ref()
            .is_some_and(|value| expression_uses_dict_method(value, source, types)),
        Statement::Raise { value, .. } => expression_uses_dict_method(value, source, types),
        Statement::Try {
            body,
            catches,
            finally_body,
            ..
        } => {
            statements_use_dict_method(body, source, types)
                || catches
                    .iter()
                    .any(|catch| statements_use_dict_method(&catch.body, source, types))
                || finally_body
                    .as_deref()
                    .is_some_and(|body| statements_use_dict_method(body, source, types))
        }
        Statement::Import { .. } | Statement::Break { .. } | Statement::Continue { .. } => false,
    })
}

/// 类型检查完成后判断是否需要静态链接 `stdlib.dict` 方法实现。
/// 判断类型检查结果中的字典方法是否需要静态链接。
fn program_uses_typed_dict_method(
    program: &Program,
    source: &SourceFile,
    types: &TypeCheckResult,
) -> bool {
    statements_use_dict_method(&program.statements, source, Some(types))
}

/// 从规范标准库源码提取完整注解的函数接口；不执行模块，跨模块类型变量使用
/// 签名内的占位身份保存，调用点再由类型检查器实例化。
fn standard_module_function_signatures(
    standard: &ProjectModuleResult,
) -> BTreeMap<String, BTreeMap<String, FunctionSignature>> {
    /// 将标准库函数注解映射成带有本地占位变量的接口类型。
    fn annotation_type(
        annotation: &FunctionTypeAnnotation,
        generic_types: &BTreeMap<String, Type>,
    ) -> Type {
        match annotation {
            FunctionTypeAnnotation::Scalar(scalar) => Type::Scalar(*scalar),
            FunctionTypeAnnotation::Resource(resource) => Type::Resource(*resource),
            FunctionTypeAnnotation::None => Type::None,
            FunctionTypeAnnotation::Set(annotation) => Type::Set(SetType::heterogeneous(
                annotation
                    .members
                    .iter()
                    .map(|term| match term {
                        TypeTerm::Scalar(scalar) => Type::scalar(*scalar),
                        TypeTerm::None => Type::None,
                        TypeTerm::Resource(resource) => Type::Resource(*resource),
                        TypeTerm::TypeParameter(name) => {
                            generic_types.get(name).cloned().unwrap_or(Type::Dynamic)
                        }
                    })
                    .collect::<Vec<_>>(),
            )),
            FunctionTypeAnnotation::Array(element) => Type::Array(
                xiao_types::ArrayType::homogeneous(annotation_type(element, generic_types)),
            ),
            FunctionTypeAnnotation::Dict(annotation) => Type::DictTable(
                xiao_types::DictType::open(annotation_type(&annotation.value, generic_types)),
            ),
            FunctionTypeAnnotation::Thread(value) => {
                Type::thread(annotation_type(value, generic_types))
            }
            FunctionTypeAnnotation::Task(value) => Type::task(annotation_type(value, generic_types)),
            FunctionTypeAnnotation::Runtime => Type::Runtime,
            FunctionTypeAnnotation::Pool => Type::Pool,
            FunctionTypeAnnotation::Shared(value) => {
                Type::shared(annotation_type(value, generic_types))
            }
            FunctionTypeAnnotation::SharedMut(value) => {
                Type::shared_mut(annotation_type(value, generic_types))
            }
            FunctionTypeAnnotation::LockGuard(value) => {
                Type::lock_guard(annotation_type(value, generic_types))
            }
            FunctionTypeAnnotation::TypeParameter(name) => {
                generic_types.get(name).cloned().unwrap_or(Type::Dynamic)
            }
        }
    }
    let mut modules = BTreeMap::new();
    for (module, record) in &standard.modules {
        let (Some(source), Some(program)) = (&record.source, &record.program) else {
            continue;
        };
        let key = |name: xiao_syntax::Name| {
            format!(
                "{}:{}",
                if name.backticked { "backtick" } else { "ascii" },
                name.unquoted_text(source)
            )
        };
        let mut signatures = BTreeMap::new();
        for statement in &program.statements {
            let Statement::Function {
                name,
                type_parameters,
                parameters,
                return_type: Some(return_type),
                span,
                ..
            } = statement
            else {
                continue;
            };
            let generic_ids = type_parameters
                .iter()
                .enumerate()
                .map(|(index, _)| TypeVarId::new(index as u32))
                .collect::<Vec<_>>();
            let generic_types = type_parameters
                .iter()
                .zip(&generic_ids)
                .map(|(name, id)| (name.unquoted_text(source).to_owned(), Type::variable(*id)))
                .collect::<BTreeMap<_, _>>();
            let parameters = parameters
                .iter()
                .map(|parameter| {
                    if matches!(
                        parameter.kind,
                        FunctionParameterKind::VarArgs | FunctionParameterKind::VarKeywords
                    ) {
                        return None;
                    }
                    Some(FunctionParameterSignature {
                        name: key(parameter.name),
                        kind: parameter.kind,
                        ty: annotation_type(parameter.annotation.as_ref()?, &generic_types),
                        has_default: parameter.default.is_some(),
                    })
                })
                .collect::<Option<Vec<_>>>();
            let Some(parameters) = parameters else {
                continue;
            };
            signatures.insert(
                key(*name),
                FunctionSignature::new(
                    key(*name),
                    parameters,
                    annotation_type(return_type, &generic_types),
                    *span,
                )
                .with_type_parameters(
                    type_parameters
                        .iter()
                        .map(|name| name.unquoted_text(source).to_owned())
                        .collect::<Vec<_>>(),
                )
                .with_type_parameter_ids(generic_ids),
            );
        }
        modules.insert(format!("{STANDARD_LIBRARY_NAMESPACE}.{module}"), signatures);
    }
    modules
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
        let isolate_set_method_functions = module_name == ModuleName::new(vec!["set".to_owned()])
            && local_functions
                .iter()
                .any(|name| SET_METHOD_NAMES.contains(&name.as_str()));
        let module_source = if isolate_set_method_functions {
            rename_set_module_functions(module_source.text())
        } else {
            module_source.text().to_owned()
        };
        result.sources.entry(identity).or_insert(module_source);
        // 若入口定义了与集合方法同名的函数，静态副本已经改用内部前缀；
        // 保留显式导入，让它继续从宿主标准库模块解析，避免二次检查时
        // 把 `size` 等绑定误当成入口顶层函数。
        if !isolate_set_method_functions {
            result.import_spans.insert((span.start(), span.end()));
        }
    }
    result
}

/// 在类型结果确认接收者为集合后，补入隐式 `stdlib.set` 静态链接。
///
/// 语法扫描只负责加载标准库签名，不能直接决定链接：表方法也可能叫 `add`/`size`，
/// 把它们提前拼接会让未导入的自由函数意外进入当前程序的顶层作用域。
fn plan_typed_set_method_link(
    plan: &mut StandardLinkPlan,
    program: &Program,
    source: &SourceFile,
    standard: &ProjectModuleResult,
    types: &TypeCheckResult,
) {
    if !program_uses_typed_set_method(program, source, types) {
        return;
    }
    let Some(record) = standard
        .modules
        .get(&ModuleName::new(vec!["set".to_owned()]))
    else {
        return;
    };
    let Some(module_source) = &record.source else {
        return;
    };
    let identity = ModuleName::new(vec!["set".to_owned()]).to_string();
    let local_functions = program
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Function { name, .. } => Some(name.unquoted_text(source).to_owned()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let module_source = if local_functions
        .iter()
        .any(|name| SET_METHOD_NAMES.contains(&name.as_str()))
    {
        rename_set_module_functions(module_source.text())
    } else {
        module_source.text().to_owned()
    };
    plan.sources.insert(identity, module_source);
}

/// 在类型结果确认接收者为字典后，补入隐式 `stdlib.dict` 静态链接。
/// 将类型确认过的字典方法实现加入标准库静态链接计划。
fn plan_typed_dict_method_link(
    plan: &mut StandardLinkPlan,
    program: &Program,
    source: &SourceFile,
    standard: &ProjectModuleResult,
    types: &TypeCheckResult,
) {
    if !program_uses_typed_dict_method(program, source, types) {
        return;
    }
    let Some(record) = standard
        .modules
        .get(&ModuleName::new(vec!["dict".to_owned()]))
    else {
        return;
    };
    let Some(module_source) = &record.source else {
        return;
    };
    let identity = ModuleName::new(vec!["dict".to_owned()]).to_string();
    plan.sources
        .insert(identity, rename_dict_module_functions(module_source.text()));
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
            || parsed
                .program
                .as_ref()
                .is_some_and(|program| program_uses_set_method(program, &request.source))
            || parsed.program.as_ref().is_some_and(|program| {
                statements_use_dict_method(&program.statements, &request.source, None)
            })
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
                if !request.context.external_standard_library {
                    standard_link = plan_standard_link(program, &request.source, &standard);
                }
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
        let (mut effective_source, mut effective_program) = if standard_link.sources.is_empty() {
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
        let mut checker = TypeChecker::new(&effective_source);
        if let Some(namespaces) = namespaces {
            checker = checker.with_external_namespaces(namespaces.clone());
        }
        if let Some(standard) = &standard_library {
            checker = checker
                .with_module_function_signatures(standard_module_function_signatures(standard));
        }
        let mut type_result = checker.check_program(&effective_program);
        // 语法扫描只负责让标准库签名可用；只有第一次类型检查确认接收者确实是
        // `set<T>` 后，才把纯 Xiao 函数体静态拼入入口。这样 `table.add()` 不会
        // 使未导入的 `stdlib.set.add` 变成当前程序的自由函数。
        let can_link_typed_set_method = !diagnostics.iter().any(Diagnostic::is_error)
            && !type_result.diagnostics.iter().any(Diagnostic::is_error)
            && !request.context.external_standard_library
            && standard_library.is_some();
        if can_link_typed_set_method {
            if let Some(standard) = &standard_library {
                let source_count = standard_link.sources.len();
                plan_typed_set_method_link(
                    &mut standard_link,
                    program,
                    &request.source,
                    standard,
                    &type_result,
                );
                if standard_link.sources.len() != source_count {
                    (effective_source, effective_program) =
                        link_standard_sources(&request.source, &standard_link);
                    let mut checker = TypeChecker::new(&effective_source);
                    if let Some(namespaces) = namespaces {
                        checker = checker.with_external_namespaces(namespaces.clone());
                    }
                    checker = checker.with_module_function_signatures(
                        standard_module_function_signatures(standard),
                    );
                    type_result = checker.check_program(&effective_program);
                }
            }
        }
        let can_link_typed_dict_method = !diagnostics.iter().any(Diagnostic::is_error)
            && !type_result.diagnostics.iter().any(Diagnostic::is_error)
            && !request.context.external_standard_library
            && standard_library.is_some();
        if can_link_typed_dict_method {
            if let Some(standard) = &standard_library {
                let source_count = standard_link.sources.len();
                plan_typed_dict_method_link(
                    &mut standard_link,
                    program,
                    &request.source,
                    standard,
                    &type_result,
                );
                if standard_link.sources.len() != source_count {
                    (effective_source, effective_program) =
                        link_standard_sources(&request.source, &standard_link);
                    let mut checker = TypeChecker::new(&effective_source);
                    if let Some(namespaces) = namespaces {
                        checker = checker.with_external_namespaces(namespaces.clone());
                    }
                    checker = checker.with_module_function_signatures(
                        standard_module_function_signatures(standard),
                    );
                    type_result = checker.check_program(&effective_program);
                }
            }
        }
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
