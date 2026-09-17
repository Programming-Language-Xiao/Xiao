//! 08-U0 统一前端流水线。
//!
//! 本模块只编排已经实现的前序分析器，并把它们的结果交给 `xiao-ir`。它不
//! 复制解析、类型或生命周期语义，也不执行配置和 Xiao 用户代码。前端错误
//! 会完整累积；只有无错误结果才会产生经过验证的 IR。

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use xiao_config::NormalizedConfig;
use xiao_diagnostics::{Diagnostic, DiagnosticParam, Severity};
use xiao_ir::{IrProgram, IrValidationError, IrValidator, lower_program};
use xiao_lifetime::analyze as analyze_lifetime;
use xiao_modules::{ProjectModuleResult, analyze_project};
use xiao_source::SourceFile;
use xiao_syntax::{ParseResult, Program, parse};
use xiao_types::{TypeCheckResult, check};

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
    /// 可选项目根；提供时会运行本地模块发现和依赖解析。
    pub project_root: Option<PathBuf>,
    /// 已规范化的 `config.xiao`；前端只读取声明树，不执行配置值。
    pub config: Option<NormalizedConfig>,
    /// 是否提供了配置树的兼容性标记；新调用方优先使用 [`Self::config`]。
    pub config_present: bool,
    /// 可选外部包图摘要。
    pub external_modules: Option<ExternalModuleGraph>,
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

        let project = request.context.project_root.as_deref().map(analyze_project);
        if let Some(project) = &project {
            diagnostics.extend(
                project
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.diagnostic.clone()),
            );
        }

        let Some(program) = parsed.program.as_ref() else {
            return Err(FrontendError { diagnostics });
        };
        let type_result = check(&request.source, program);
        diagnostics.extend(type_result.diagnostics.clone());

        let lifetime = analyze_lifetime(&request.source, program, &type_result);
        diagnostics.extend(lifetime.diagnostics.clone());

        if diagnostics.iter().any(Diagnostic::is_error) {
            return Err(FrontendError { diagnostics });
        }

        let mut ir = lower_program(
            &request.source,
            program,
            &type_result,
            &lifetime,
            project.as_ref(),
        );
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

        let validation = IrValidator::new().validate(&ir);
        if !validation.is_success() {
            diagnostics.extend(validation.errors.iter().map(validation_diagnostic));
            return Err(FrontendError { diagnostics });
        }
        Ok(FrontendArtifact { ir, diagnostics })
    }
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
