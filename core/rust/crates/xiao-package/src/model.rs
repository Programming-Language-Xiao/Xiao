//! 包身份、来源预留和内存依赖图模型。
//!
//! 这里的图与 `xiao-modules::ModuleGraph` 有意分开：前者以包名、版本和来源身份为节点，
//! 后者以源码文件/命名空间为节点。D1 只建立确定性的内存模型，不保存锁文件或缓存。

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use xiao_config::{DependencyDeclaration, DependencyKind};
use xiao_diagnostics::Diagnostic;
use xiao_source::SourceSpan;

/// 一个包源的身份、配置别名和展示名称。
///
/// `source_id` 是唯一身份键；`alias` 只供配置引用；`display_name` 只供人类展示。
/// 三者不能合并为一个 `name` 字段，以便后续 E3A 接入多源协议时保持契约稳定。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PackageSource {
    /// 稳定的来源身份键，例如 `path:/workspace/lib`。
    pub source_id: String,
    /// 配置内可引用的别名；本地路径 D1 不自动生成别名。
    pub alias: Option<String>,
    /// 纯展示用名称，不参与身份比较或依赖解析。
    pub display_name: String,
}

impl PackageSource {
    /// 从规范化本地包根目录创建来源身份。
    #[must_use]
    pub fn local_path(path: &Path) -> Self {
        let normalized = normalize_source_path(path);
        Self {
            source_id: format!("path:{normalized}"),
            alias: None,
            display_name: path.display().to_string(),
        }
    }
}

impl PartialEq for PackageSource {
    /// 只按 `source_id` 比较；别名和展示名不改变来源身份。
    fn eq(&self, other: &Self) -> bool {
        self.source_id == other.source_id
    }
}

impl Eq for PackageSource {}

impl Hash for PackageSource {
    /// 只哈希稳定来源身份。
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_id.hash(state);
    }
}

impl Ord for PackageSource {
    /// 按稳定来源身份排序。
    fn cmp(&self, other: &Self) -> Ordering {
        self.source_id.cmp(&other.source_id)
    }
}

impl PartialOrd for PackageSource {
    /// 按稳定来源身份执行部分排序。
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 兼容文档用语的来源身份别名。
pub type SourceIdentity = PackageSource;

/// 一个包的逻辑身份。
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PackageIdentity {
    /// 规范化包名。
    pub name: String,
    /// 包配置声明的版本文本。
    pub version: String,
    /// 来源身份及其独立的 alias/display 字段。
    pub source: PackageSource,
}

impl Display for PackageIdentity {
    /// 以稳定的诊断键格式展示包身份。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}@{}[{}]",
            self.name, self.version, self.source.source_id
        )
    }
}

/// 一个已经读取到内存中的包依赖。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageDependency {
    /// 依赖表中的包名。
    pub name: String,
    /// 运行时或开发期依赖分类。
    pub kind: DependencyKind,
    /// 相对于声明包根目录的规范化路径。
    pub path: PathBuf,
    /// 可选版本约束；D1 保留但不求解。
    pub version: Option<String>,
    /// 可选来源引用；D1 保留 alias/source_id 形状。
    pub source: Option<String>,
    /// 依赖声明的源码区间。
    pub span: SourceSpan,
    /// 解析后的目标身份；未找到目标时为空。
    pub target: Option<PackageIdentity>,
}

impl PackageDependency {
    /// 从配置层声明创建尚未解析目标的包依赖。
    #[must_use]
    pub fn from_declaration(declaration: DependencyDeclaration, path: PathBuf) -> Self {
        Self {
            name: declaration.name,
            kind: declaration.kind,
            path,
            version: declaration.version,
            source: declaration.source,
            span: declaration.span,
            target: None,
        }
    }
}

/// 包图中的一个节点。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageNode {
    /// 包的逻辑身份。
    pub identity: PackageIdentity,
    /// 已规范化的包根目录。
    pub root: PathBuf,
    /// 按声明名称排序的直接依赖。
    pub dependencies: BTreeMap<String, PackageDependency>,
}

/// 包图中的一条依赖边。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageEdge {
    /// 发起依赖的配置名称。
    pub dependency: String,
    /// 运行时或开发期依赖分类。
    pub kind: DependencyKind,
    /// 目标包身份。
    pub target: PackageIdentity,
}

/// 包身份粒度的确定性依赖图。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageGraph {
    /// 根项目包身份。
    pub root: Option<PackageIdentity>,
    /// 按包身份排序的全部可达节点。
    pub nodes: BTreeMap<PackageIdentity, PackageNode>,
    /// 按来源包身份保存的依赖边。
    pub edges: BTreeMap<PackageIdentity, Vec<PackageEdge>>,
    /// 无错误、无环时的依赖优先顺序。
    pub resolution_order: Vec<PackageIdentity>,
}

impl Default for PackageGraph {
    /// 创建空包图。
    fn default() -> Self {
        Self {
            root: None,
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            resolution_order: Vec::new(),
        }
    }
}

/// 带包路径上下文的包解析诊断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageDiagnostic {
    /// 相关包；根配置读取失败时可能为空。
    pub package: Option<PackageIdentity>,
    /// 相关配置或包根路径。
    pub path: Option<PathBuf>,
    /// 统一结构化诊断。
    pub diagnostic: Diagnostic,
}

/// 一次本地包图解析的完整结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageResolution {
    /// 解析出的确定性包图；即使存在诊断也可能包含部分节点。
    pub graph: PackageGraph,
    /// 按稳定遍历顺序收集的诊断。
    pub diagnostics: Vec<PackageDiagnostic>,
}

impl PackageResolution {
    /// 判断解析是否成功且没有任何错误诊断。
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|entry| !entry.diagnostic.is_error())
    }
}

/// 生成本地路径源的规范化身份片段。
#[must_use]
pub fn local_source_id(path: &Path) -> String {
    PackageSource::local_path(path).source_id
}

/// 将本地路径转成稳定的正斜杠身份片段并去除尾部斜杠。
fn normalize_source_path(path: &Path) -> String {
    let mut normalized = path.to_string_lossy().replace('\\', "/");
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}
