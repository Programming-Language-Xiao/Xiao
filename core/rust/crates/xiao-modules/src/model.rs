//! 05-B 模块、命名空间和解析结果的数据模型。

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use xiao_diagnostics::Diagnostic;
use xiao_source::{SourceFile, SourceSpan};
use xiao_syntax::Program;

/// 规范化的点号模块名称。
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModuleName {
    segments: Vec<String>,
}

impl ModuleName {
    /// 从非空名称段创建模块名称。
    #[must_use]
    pub fn new(segments: impl IntoIterator<Item = String>) -> Self {
        Self {
            segments: segments.into_iter().collect(),
        }
    }

    /// 创建根命名空间名称。
    #[must_use]
    pub const fn root() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// 返回名称段。
    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// 判断名称是否为空根名称。
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// 追加一个名称段。
    #[must_use]
    pub fn child(&self, segment: impl Into<String>) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment.into());
        Self { segments }
    }

    /// 返回父名称；根名称没有父名称。
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        (!self.segments.is_empty()).then(|| Self {
            segments: self.segments[..self.segments.len() - 1].to_vec(),
        })
    }

    /// 判断当前名称是否是另一个名称的前缀。
    #[must_use]
    pub fn is_prefix_of(&self, other: &Self) -> bool {
        other.segments.starts_with(&self.segments)
    }
}

impl Display for ModuleName {
    /// 以点号连接名称段。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.segments.join("."))
    }
}

/// 导入路径在模块解析后的拥有版本。
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ImportPathName(pub ModuleName);

impl ImportPathName {
    /// 返回内部模块名称。
    #[must_use]
    pub const fn as_module(&self) -> &ModuleName {
        &self.0
    }
}

/// 文件模块或纯目录命名空间。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModuleKind {
    /// 对应一个 `.xiao` 文件，可能拥有初始化代码。
    File,
    /// 对应一个目录，不拥有初始化代码。
    Namespace,
}

/// 一个已发现的模块记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleRecord {
    /// 模块逻辑名称。
    pub name: ModuleName,
    /// 模块类别。
    pub kind: ModuleKind,
    /// 对应文件或目录的规范化路径。
    pub path: PathBuf,
    /// 文件模块的 UTF-8 源码；命名空间为空。
    pub source: Option<SourceFile>,
    /// 文件模块的解析程序；命名空间为空。
    pub program: Option<Program>,
    /// 当前模块可见的符号接口。
    pub symbols: BTreeMap<String, ModuleSymbol>,
}

/// 模块中的一个可导入名称。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleSymbol {
    /// 规范化名称。
    pub name: String,
    /// 符号类别。
    pub kind: ModuleSymbolKind,
    /// 本地声明或导入的来源。
    pub origin: ExportOrigin,
    /// 声明/导入源码区间。
    pub span: SourceSpan,
}

/// 模块符号类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModuleSymbolKind {
    /// 普通值、常量、参数以外的顶层绑定。
    Value,
    /// 顶层函数。
    Function,
    /// 可用于限定访问的文件模块。
    Module,
    /// 可用于限定访问的目录命名空间。
    Namespace,
}

/// 符号最终来源；支持包内顶层再导出。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExportOrigin {
    /// 当前模块的本地声明。
    Local,
    /// 从其他模块再导入的符号。
    Reexport {
        /// 原始模块。
        module: ModuleName,
        /// 原始名称。
        name: String,
    },
}

/// 一个纯目录命名空间记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamespaceRecord {
    /// 命名空间逻辑名称。
    pub name: ModuleName,
    /// 对应目录路径。
    pub path: PathBuf,
}

/// 词法作用域的稳定编号。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LexicalScopeId(pub u32);

/// 导入绑定的类别。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingKind {
    /// 模块/命名空间限定符；不是一等 Xiao 值。
    Qualifier {
        /// 被限定的模块或命名空间。
        target: ModuleName,
        /// 目标类别。
        kind: ModuleKind,
    },
    /// 从模块选择出的可赋值值符号。
    Value {
        /// 符号来源。
        origin: ExportOrigin,
    },
    /// 普通本地绑定，仅用于冲突和遮蔽分析。
    Local,
}

/// 一个已解析的导入绑定。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedBinding {
    /// 所在模块。
    pub module: ModuleName,
    /// 所在词法作用域。
    pub scope: LexicalScopeId,
    /// 当前作用域中的名称。
    pub local_name: String,
    /// 绑定类别。
    pub kind: BindingKind,
    /// 导入或声明区间。
    pub span: SourceSpan,
}

/// 依赖边的来源类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImportEdgeKind {
    /// 导入语句直接产生的边。
    Import,
    /// 通过命名空间限定访问具体子模块产生的边。
    QualifiedUse,
}

/// 一条聚合后的模块依赖边。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportEdge {
    /// 目标模块或命名空间。
    pub target: ModuleName,
    /// 边的来源类别。
    pub kind: ImportEdgeKind,
    /// 所有对应源码区间，按遍历顺序保存。
    pub spans: Vec<SourceSpan>,
}

/// 文件模块依赖图和确定性初始化计划。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleGraph {
    /// 按来源模块保存的聚合边。
    pub edges: BTreeMap<ModuleName, Vec<ImportEdge>>,
    /// 无循环时的依赖优先顺序；有循环时为空。
    pub initialization_order: Vec<ModuleName>,
}

impl Default for ModuleGraph {
    /// 创建空依赖图。
    fn default() -> Self {
        Self {
            edges: BTreeMap::new(),
            initialization_order: Vec::new(),
        }
    }
}

/// 带模块上下文的诊断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleDiagnostic {
    /// 相关模块；文件读取失败时可能为空。
    pub module: Option<ModuleName>,
    /// 相关文件系统路径。
    pub path: Option<PathBuf>,
    /// 统一结构化诊断。
    pub diagnostic: Diagnostic,
}

/// 一次项目模块分析的完整结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectModuleResult {
    /// 使用的项目根路径。
    pub project_root: PathBuf,
    /// 文件模块和命名空间，按逻辑名排序。
    pub modules: BTreeMap<ModuleName, ModuleRecord>,
    /// 目录命名空间记录。
    pub namespaces: BTreeMap<ModuleName, NamespaceRecord>,
    /// 静态依赖图。
    pub graph: ModuleGraph,
    /// 所有解析出的绑定。
    pub bindings: Vec<ResolvedBinding>,
    /// 解析期间收集的诊断。
    pub diagnostics: Vec<ModuleDiagnostic>,
}

impl ProjectModuleResult {
    /// 判断分析结果是否没有错误诊断。
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.diagnostic.is_error())
    }

    /// 返回所有文件模块名称。
    #[must_use = "iterate over the discovered file module names"]
    pub fn file_modules(&self) -> impl Iterator<Item = &ModuleName> {
        self.modules
            .iter()
            .filter_map(|(name, record)| (record.kind == ModuleKind::File).then_some(name))
    }
}

/// 用于检测大小写折叠冲突的规范键。
#[must_use]
pub(crate) fn case_fold_name(name: &ModuleName) -> String {
    name.to_string().to_ascii_lowercase()
}
