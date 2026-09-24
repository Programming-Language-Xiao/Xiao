//! 本地路径包配置读取与确定性依赖图解析。
//!
//! 解析器只读取每个包根目录下的 `config.xiao`，并把它交给 `xiao-config` 的静态入口。
//! 它不执行 Xiao 代码，不创建缓存或锁文件，也不访问远程来源。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use xiao_config::{ConfigDocument, dependency_declarations, parse_config_project};
use xiao_diagnostics::{Diagnostic, DiagnosticParam, Severity};
use xiao_source::{SourceFile, SourceSpan};

use crate::diagnostics::{
    PACKAGE_CONFIG_READ_CODE, PACKAGE_DEPENDENCY_CYCLE_CODE, PACKAGE_IDENTITY_CONFLICT_CODE,
    PACKAGE_INVALID_METADATA_CODE, PACKAGE_MISSING_DEPENDENCY_CODE,
};
use crate::model::{
    PackageDependency, PackageDiagnostic, PackageEdge, PackageGraph, PackageIdentity, PackageNode,
    PackageResolution, PackageSource,
};

/// 只解析本地路径包的 D1 解析器。
#[derive(Clone, Copy, Debug, Default)]
pub struct PackageResolver;

impl PackageResolver {
    /// 创建一个无状态包解析器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// 从项目根目录或其 `config.xiao` 文件解析可达本地包图。
    #[must_use]
    pub fn resolve_project(&self, project_or_config: impl AsRef<Path>) -> PackageResolution {
        let config_path = project_config_path(project_or_config.as_ref());
        let mut state = ResolverState::default();
        let Some((root, is_new)) = state.load_package(config_path, None, false) else {
            return state.finish();
        };
        state.graph.root = Some(root.clone());
        if is_new {
            state.expand_package(root);
        }
        state.finish()
    }

    /// 从指定 `config.xiao` 文件解析可达本地包图。
    #[must_use]
    pub fn resolve_config(&self, config_path: impl AsRef<Path>) -> PackageResolution {
        self.resolve_project(config_path)
    }
}

/// 从项目根目录或配置文件解析本地包图。
#[must_use]
pub fn resolve_project(project_or_config: impl AsRef<Path>) -> PackageResolution {
    PackageResolver::new().resolve_project(project_or_config)
}

/// `resolve_project` 的 D1 语义别名，便于调用方明确表达“路径依赖”边界。
#[must_use]
pub fn resolve_path_dependencies(project_or_config: impl AsRef<Path>) -> PackageResolution {
    resolve_project(project_or_config)
}

/// 递归解析期间保存图、缓存身份和 DFS 状态的内部状态。
#[derive(Default)]
struct ResolverState {
    graph: PackageGraph,
    diagnostics: Vec<PackageDiagnostic>,
    path_identities: BTreeMap<PathBuf, PackageIdentity>,
    name_identities: BTreeMap<String, (PackageIdentity, PathBuf)>,
    expanded: BTreeSet<PackageIdentity>,
    active: Vec<PackageIdentity>,
    reported_cycles: BTreeSet<String>,
}

impl ResolverState {
    /// 根据解析状态生成最终图，并在无错误时计算依赖优先顺序。
    fn finish(mut self) -> PackageResolution {
        if self.diagnostics.is_empty() {
            if let Some(root) = self.graph.root.clone() {
                let mut visited = BTreeSet::new();
                let mut order = Vec::new();
                append_resolution_order(&self.graph, &root, &mut visited, &mut order);
                self.graph.resolution_order = order;
            }
        }
        PackageResolution {
            graph: self.graph,
            diagnostics: self.diagnostics,
        }
    }

    /// 读取一个包配置、建立节点并检查包身份冲突。
    fn load_package(
        &mut self,
        requested_config: PathBuf,
        requested_name: Option<&str>,
        is_dependency: bool,
    ) -> Option<(PackageIdentity, bool)> {
        let canonical_config = match fs::canonicalize(&requested_config) {
            Ok(path) => path,
            Err(error) => {
                let code = if is_dependency {
                    PACKAGE_MISSING_DEPENDENCY_CODE
                } else {
                    PACKAGE_CONFIG_READ_CODE
                };
                let message_id = if is_dependency {
                    "x05.package.missing_dependency"
                } else {
                    "x05.package.config_read"
                };
                self.push_path_diagnostic(
                    code,
                    message_id,
                    None,
                    requested_config.clone(),
                    format!(
                        "无法读取{}包配置 {}：{error}",
                        if is_dependency { "依赖" } else { "根" },
                        requested_config.display()
                    ),
                    [
                        (
                            "path",
                            DiagnosticParam::Text(requested_config.display().to_string()),
                        ),
                        ("reason", DiagnosticParam::Text(error.to_string())),
                    ],
                );
                return None;
            }
        };
        let package_root = canonical_config
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| canonical_config.clone());
        if let Some(identity) = self.path_identities.get(&package_root).cloned() {
            if let Some(requested_name) = requested_name {
                self.check_requested_name(requested_name, &identity, &package_root, None);
            }
            return Some((identity, false));
        }

        let text = match fs::read_to_string(&canonical_config) {
            Ok(text) => text,
            Err(error) => {
                self.push_path_diagnostic(
                    PACKAGE_CONFIG_READ_CODE,
                    "x05.package.config_read",
                    None,
                    package_root.clone(),
                    format!("无法读取包配置 {}：{error}", canonical_config.display()),
                    [
                        (
                            "path",
                            DiagnosticParam::Text(canonical_config.display().to_string()),
                        ),
                        ("reason", DiagnosticParam::Text(error.to_string())),
                    ],
                );
                return None;
            }
        };
        let source = SourceFile::from_text(&text);
        let document = match parse_config_project(&source) {
            Ok(document) => document,
            Err(errors) => {
                for diagnostic in errors {
                    self.diagnostics.push(PackageDiagnostic {
                        package: None,
                        path: Some(package_root.clone()),
                        diagnostic,
                    });
                }
                return None;
            }
        };
        let Some(identity) = package_identity(&document, &package_root) else {
            self.push_path_diagnostic(
                PACKAGE_INVALID_METADATA_CODE,
                "x05.package.invalid_metadata",
                None,
                package_root.clone(),
                format!(
                    "包配置 {} 缺少有效的 [project].name/version",
                    canonical_config.display()
                ),
                [(
                    "path",
                    DiagnosticParam::Text(canonical_config.display().to_string()),
                )],
            );
            return None;
        };
        self.check_requested_name(
            requested_name.unwrap_or(&identity.name),
            &identity,
            &package_root,
            None,
        );
        self.check_identity_conflict(&identity, &package_root, None);

        let dependencies = dependency_declarations(&document)
            .into_iter()
            .map(|declaration| {
                let path = dependency_config_path(&package_root, &declaration.path);
                (
                    declaration.name.clone(),
                    PackageDependency::from_declaration(declaration, path),
                )
            })
            .collect::<BTreeMap<_, _>>();
        self.path_identities
            .insert(package_root.clone(), identity.clone());
        self.graph.nodes.insert(
            identity.clone(),
            PackageNode {
                identity: identity.clone(),
                root: package_root.clone(),
                dependencies,
            },
        );
        self.graph.edges.entry(identity.clone()).or_default();
        Some((identity, true))
    }

    /// 深度优先展开一个已加载包的全部直接依赖。
    fn expand_package(&mut self, identity: PackageIdentity) {
        if !self.expanded.insert(identity.clone()) {
            return;
        }
        self.active.push(identity.clone());
        let Some(node) = self.graph.nodes.get(&identity).cloned() else {
            self.active.pop();
            return;
        };
        for (name, dependency) in node.dependencies {
            let Some((target, is_new)) =
                self.load_package(dependency.path.clone(), Some(&name), true)
            else {
                continue;
            };
            if let Some(current) = self.graph.nodes.get_mut(&identity) {
                if let Some(declaration) = current.dependencies.get_mut(&name) {
                    declaration.target = Some(target.clone());
                }
            }
            self.graph
                .edges
                .entry(identity.clone())
                .or_default()
                .push(PackageEdge {
                    dependency: name,
                    kind: dependency.kind,
                    target: target.clone(),
                });
            if self.active.contains(&target) {
                self.report_cycle(&target, dependency.span, &identity);
            } else if is_new {
                self.expand_package(target);
            }
        }
        self.active.pop();
    }

    /// 检查依赖表名称是否与目标包的声明名称一致。
    fn check_requested_name(
        &mut self,
        requested_name: &str,
        identity: &PackageIdentity,
        package_root: &Path,
        span: Option<SourceSpan>,
    ) {
        if requested_name != identity.name {
            self.push_package_diagnostic(
                PACKAGE_IDENTITY_CONFLICT_CODE,
                "x05.package.identity_conflict",
                identity.clone(),
                package_root.to_path_buf(),
                span,
                format!(
                    "依赖名称 {requested_name:?} 指向了包 {:?}，包名必须一致",
                    identity.name
                ),
                [
                    (
                        "requested",
                        DiagnosticParam::Text(requested_name.to_owned()),
                    ),
                    ("actual", DiagnosticParam::Text(identity.name.clone())),
                ],
            );
        }
    }

    /// 检查同名包是否已经绑定到另一个身份或本地路径。
    fn check_identity_conflict(
        &mut self,
        identity: &PackageIdentity,
        package_root: &Path,
        span: Option<SourceSpan>,
    ) {
        if let Some((known, known_root)) = self.name_identities.get(&identity.name) {
            if known != identity || known_root != package_root {
                self.push_package_diagnostic(
                    PACKAGE_IDENTITY_CONFLICT_CODE,
                    "x05.package.identity_conflict",
                    identity.clone(),
                    package_root.to_path_buf(),
                    span,
                    format!(
                        "包名 {:?} 同时绑定到 {} 和 {}",
                        identity.name, known, identity
                    ),
                    [
                        ("package", DiagnosticParam::Text(identity.name.clone())),
                        (
                            "first",
                            DiagnosticParam::Text(known_root.display().to_string()),
                        ),
                        (
                            "second",
                            DiagnosticParam::Text(package_root.display().to_string()),
                        ),
                    ],
                );
            }
        } else {
            self.name_identities.insert(
                identity.name.clone(),
                (identity.clone(), package_root.to_path_buf()),
            );
        }
    }

    /// 将当前 DFS 栈中的回边转换为一次稳定的依赖环诊断。
    fn report_cycle(
        &mut self,
        target: &PackageIdentity,
        span: SourceSpan,
        current: &PackageIdentity,
    ) {
        let Some(start) = self.active.iter().position(|item| item == target) else {
            return;
        };
        let mut cycle = self.active[start..]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        cycle.push(target.to_string());
        let signature = cycle.join(" -> ");
        if !self.reported_cycles.insert(signature.clone()) {
            return;
        }
        let path = self
            .graph
            .nodes
            .get(current)
            .map(|node| node.root.clone())
            .unwrap_or_default();
        self.push_package_diagnostic(
            PACKAGE_DEPENDENCY_CYCLE_CODE,
            "x05.package.dependency_cycle",
            current.clone(),
            path,
            Some(span),
            format!("包依赖环：{signature}"),
            [("cycle", DiagnosticParam::Text(signature))],
        );
    }

    /// 添加不属于某个已构造包身份的路径级诊断。
    fn push_path_diagnostic<I, K>(
        &mut self,
        code: &str,
        message_id: &str,
        span: Option<SourceSpan>,
        path: PathBuf,
        message: String,
        params: I,
    ) where
        I: IntoIterator<Item = (K, DiagnosticParam)>,
        K: Into<String>,
    {
        let diagnostic = make_diagnostic(code, message_id, span, message, params);
        self.diagnostics.push(PackageDiagnostic {
            package: None,
            path: Some(path),
            diagnostic,
        });
    }

    /// 添加带包身份和路径上下文的结构化诊断。
    fn push_package_diagnostic<I, K>(
        &mut self,
        code: &str,
        message_id: &str,
        package: PackageIdentity,
        path: PathBuf,
        span: Option<SourceSpan>,
        message: String,
        params: I,
    ) where
        I: IntoIterator<Item = (K, DiagnosticParam)>,
        K: Into<String>,
    {
        let diagnostic = make_diagnostic(code, message_id, span, message, params);
        self.diagnostics.push(PackageDiagnostic {
            package: Some(package),
            path: Some(path),
            diagnostic,
        });
    }
}

/// 从已经通过配置校验的文档提取包身份。
fn package_identity(document: &ConfigDocument, package_root: &Path) -> Option<PackageIdentity> {
    let project = document.table("project")?;
    let name = project.get("name")?.value.as_str()?.to_owned();
    let version = project.get("version")?.value.as_str()?.to_owned();
    Some(PackageIdentity {
        name,
        version,
        source: PackageSource::local_path(package_root),
    })
}

/// 将项目目录或配置路径统一转换为配置文件路径。
fn project_config_path(path: &Path) -> PathBuf {
    if path
        .extension()
        .is_some_and(|extension| extension == "xiao")
    {
        path.to_path_buf()
    } else {
        path.join("config.xiao")
    }
}

/// 将依赖声明的相对路径转换为目标包的 `config.xiao` 路径。
fn dependency_config_path(package_root: &Path, path: &str) -> PathBuf {
    let candidate = package_root.join(path);
    if candidate
        .extension()
        .is_some_and(|extension| extension == "xiao")
    {
        candidate
    } else {
        candidate.join("config.xiao")
    }
}

/// 创建带可选源码区间和参数的包诊断。
fn make_diagnostic<I, K>(
    code: &str,
    message_id: &str,
    span: Option<SourceSpan>,
    message: String,
    params: I,
) -> Diagnostic
where
    I: IntoIterator<Item = (K, DiagnosticParam)>,
    K: Into<String>,
{
    let diagnostic = match span {
        Some(span) => Diagnostic::error_at(code, message_id, span, message),
        None => Diagnostic::new(code, message_id, Severity::Error, None, message),
    };
    diagnostic.with_params(params.into_iter().map(|(key, value)| (key.into(), value)))
}

/// 从根节点递归追加依赖优先的确定性顺序。
fn append_resolution_order(
    graph: &PackageGraph,
    identity: &PackageIdentity,
    visited: &mut BTreeSet<PackageIdentity>,
    order: &mut Vec<PackageIdentity>,
) {
    if !visited.insert(identity.clone()) {
        return;
    }
    if let Some(edges) = graph.edges.get(identity) {
        for edge in edges {
            append_resolution_order(graph, &edge.target, visited, order);
        }
    }
    order.push(identity.clone());
}
