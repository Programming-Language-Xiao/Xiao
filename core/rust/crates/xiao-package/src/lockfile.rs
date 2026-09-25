//! 11A-E2A 的本地依赖锁文件与原子文件提交。
//!
//! 锁文件只消费 D1 已解析的 [`PackageGraph`]，不重新扫描配置、不执行包代码，也不实现
//! 版本求解。源码摘要继续由 E1 的 [`crate::cache::source_directory_digest`] 提供；本模块
//! 只负责把完整图、逻辑包身份和内容对象摘要保存为可审计的 `xiao.lock.json`。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use xiao_config::ConfigDocument;

use crate::cache::{CacheError, CacheStore, source_directory_digest};
use crate::diagnostics::{
    LOCKFILE_CONFIG_MISMATCH_CODE, LOCKFILE_CONTENT_MISMATCH_CODE, LOCKFILE_INVALID_CODE,
    LOCKFILE_IO_CODE, LOCKFILE_SOURCE_MISMATCH_CODE, LOCKFILE_UNSUPPORTED_VERSION_CODE,
};
use crate::environment::fingerprint_config;
use crate::model::{PackageGraph, PackageIdentity, PackageNode};

/// 项目锁文件的固定文件名。
pub const LOCKFILE_NAME: &str = "xiao.lock.json";
/// 当前支持的锁文件格式版本。
pub const LOCKFILE_VERSION: u32 = 1;

/// 生成原子暂存文件名的进程内计数器。
static NEXT_ATOMIC_FILE: AtomicU64 = AtomicU64::new(0);

/// 记录完整本地依赖图的确定性 JSON 锁文件。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LockFile {
    /// 锁文件格式版本。
    pub lock_version: u32,
    /// 根项目规范化配置的 E0 指纹。
    pub config_fingerprint: String,
    /// 项目根包的完整逻辑身份。
    pub root: PackageIdentity,
    /// 按稳定身份键排序的全部可达包。
    pub packages: BTreeMap<String, LockedPackage>,
}

/// 锁文件中的一个完整包条目。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LockedPackage {
    /// 规范化包名。
    pub name: String,
    /// 包配置声明的版本文本。
    pub version: String,
    /// 包的源身份、别名和展示字段。
    pub source: crate::model::PackageSource,
    /// E1 源码对象的 SHA-256 摘要。
    pub content_digest: String,
    /// 该包的全部直接依赖，按配置名称排序。
    pub dependencies: BTreeMap<String, LockedDependency>,
    /// 预编译产物变体预留，E2A 首版固定为空。
    #[serde(default)]
    pub precompiled_variants: Vec<String>,
    /// 目标条件维度预留，E2A 首版固定为空。
    #[serde(default)]
    pub target_conditions: Vec<String>,
}

impl LockedPackage {
    /// 返回锁文件条目对应的完整逻辑身份。
    #[must_use]
    pub fn identity(&self) -> PackageIdentity {
        PackageIdentity {
            name: self.name.clone(),
            version: self.version.clone(),
            source: self.source.clone(),
        }
    }
}

/// 锁文件中的一条直接依赖边。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LockedDependency {
    /// 依赖所属的配置表名称。
    pub kind: String,
    /// 原声明的版本约束占位文本。
    pub version_constraint: Option<String>,
    /// 原声明的来源引用占位文本。
    pub source_reference: Option<String>,
    /// 本地依赖的配置路径。
    pub config_path: String,
    /// 解析后的目标完整逻辑身份。
    pub target: PackageIdentity,
}

/// 锁文件写入结果，用于区分首次生成、内容复用和实际更新。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockFileWriteStatus {
    /// 目标文件此前不存在，本次创建。
    Created,
    /// 目标文件字节完全相同，本次没有重写。
    Reused,
    /// 目标文件存在但内容已变化，本次原子替换。
    Updated,
}

/// 配置或本地源码与锁文件不一致的稳定诊断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LockfileMismatch {
    /// 配置解析出的包名、版本或依赖边发生变化。
    ConfigurationChanged {
        /// 受影响的包名。
        package: String,
    },
    /// 本地包源码摘要发生变化。
    ContentChanged {
        /// 受影响的包名。
        package: String,
        /// 锁文件中的摘要。
        expected: String,
        /// 当前源码摘要。
        actual: String,
    },
    /// 本地包源身份发生变化，通常意味着路径变化。
    SourceChanged {
        /// 受影响的包名。
        package: String,
        /// 锁文件中的来源身份。
        expected: String,
        /// 当前来源身份。
        actual: String,
    },
}

impl LockfileMismatch {
    /// 返回配置与锁文件不一致的稳定诊断编号。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ConfigurationChanged { .. } => LOCKFILE_CONFIG_MISMATCH_CODE,
            Self::ContentChanged { .. } => LOCKFILE_CONTENT_MISMATCH_CODE,
            Self::SourceChanged { .. } => LOCKFILE_SOURCE_MISMATCH_CODE,
        }
    }
}

impl Display for LockfileMismatch {
    /// 输出带稳定编号的锁文件过期诊断。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigurationChanged { package } => {
                write!(
                    formatter,
                    "{}: 包 {package:?} 的配置或依赖图已变化",
                    self.code()
                )
            }
            Self::ContentChanged {
                package,
                expected,
                actual,
            } => write!(
                formatter,
                "{}: 包 {package:?} 的源码摘要已变化：锁定 {expected}，当前 {actual}",
                self.code()
            ),
            Self::SourceChanged {
                package,
                expected,
                actual,
            } => write!(
                formatter,
                "{}: 包 {package:?} 的来源身份已变化：锁定 {expected}，当前 {actual}",
                self.code()
            ),
        }
    }
}

impl std::error::Error for LockfileMismatch {}

/// 锁文件读取、校验、生成或原子写入错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LockfileError {
    /// 文件系统读写失败。
    Io {
        /// 相关路径。
        path: PathBuf,
        /// 失败操作。
        operation: &'static str,
        /// 主机错误文本。
        message: String,
    },
    /// JSON 或锁文件结构不合法。
    Invalid {
        /// 相关路径。
        path: PathBuf,
        /// 稳定错误说明。
        message: String,
    },
    /// 锁文件版本高于当前读取器。
    UnsupportedVersion {
        /// 相关路径。
        path: PathBuf,
        /// 不支持的版本号。
        version: u32,
    },
    /// 图没有根包，无法生成项目锁文件。
    EmptyGraph,
    /// E1 源码摘要计算失败。
    Cache {
        /// 底层缓存/源码错误。
        error: CacheError,
    },
    /// 当前配置或源码与锁文件不一致。
    Mismatch {
        /// 具体的不一致类型。
        mismatch: LockfileMismatch,
    },
}

impl LockfileError {
    /// 返回错误对应的稳定诊断编号。
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => LOCKFILE_IO_CODE,
            Self::Invalid { .. } | Self::EmptyGraph => LOCKFILE_INVALID_CODE,
            Self::UnsupportedVersion { .. } => LOCKFILE_UNSUPPORTED_VERSION_CODE,
            Self::Cache { error } => error.code(),
            Self::Mismatch { mismatch } => mismatch.code(),
        }
    }
}

impl Display for LockfileError {
    /// 输出带稳定编号的锁文件错误。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                path,
                operation,
                message,
            } => write!(
                formatter,
                "{}: {operation} {}：{message}",
                self.code(),
                path.display()
            ),
            Self::Invalid { path, message } => {
                write!(
                    formatter,
                    "{}: 锁文件 {} 无效：{message}",
                    self.code(),
                    path.display()
                )
            }
            Self::UnsupportedVersion { path, version } => write!(
                formatter,
                "{}: 锁文件 {} 使用不支持的版本 {version}",
                self.code(),
                path.display()
            ),
            Self::EmptyGraph => write!(formatter, "{}: 依赖图没有根包", self.code()),
            Self::Cache { error } => Display::fmt(error, formatter),
            Self::Mismatch { mismatch } => Display::fmt(mismatch, formatter),
        }
    }
}

impl std::error::Error for LockfileError {}

impl From<CacheError> for LockfileError {
    /// 将 E1 源码摘要错误保留在锁文件错误中。
    fn from(error: CacheError) -> Self {
        Self::Cache { error }
    }
}

/// 返回项目根下的锁文件路径。
#[must_use]
pub fn lockfile_path(project_root: impl AsRef<Path>) -> PathBuf {
    project_root.as_ref().join(LOCKFILE_NAME)
}

/// 从完整 D1 包图和 E1 缓存生成确定性的锁文件模型。
#[allow(clippy::result_large_err)]
pub fn build_lockfile(
    graph: &PackageGraph,
    document: &ConfigDocument,
    cache: &CacheStore,
) -> Result<LockFile, LockfileError> {
    let root = graph.root.clone().ok_or(LockfileError::EmptyGraph)?;
    validate_graph(graph)?;
    let mut packages = BTreeMap::new();
    for (identity, node) in &graph.nodes {
        let object = cache.import_source_directory(&node.root)?;
        packages.insert(
            identity_key(identity),
            locked_package(node, object.reference.digest, graph),
        );
    }
    let lockfile = LockFile {
        lock_version: LOCKFILE_VERSION,
        config_fingerprint: fingerprint_config(document),
        root,
        packages,
    };
    lockfile.validate(Path::new("<memory>"))?;
    Ok(lockfile)
}

/// 将锁文件编码为稳定、可审计的 JSON 文本。
impl LockFile {
    /// 验证版本和结构，并编码为带尾换行的格式化 JSON。
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            "{}\n",
            serde_json::to_string_pretty(self).expect("锁文件模型必须可编码为 JSON")
        )
    }

    /// 从 JSON 文本读取当前版本锁文件，拒绝不支持的版本。
    #[allow(clippy::result_large_err)]
    pub fn from_json(text: &str) -> Result<Self, LockfileError> {
        Self::from_json_at(text, Path::new("<json>"))
    }

    /// 检查锁文件版本、键一致性、摘要格式和根包存在性。
    #[allow(clippy::result_large_err)]
    pub fn validate(&self, path: &Path) -> Result<(), LockfileError> {
        if self.lock_version > LOCKFILE_VERSION {
            return Err(LockfileError::UnsupportedVersion {
                path: path.to_path_buf(),
                version: self.lock_version,
            });
        }
        if self.lock_version != LOCKFILE_VERSION {
            return Err(LockfileError::Invalid {
                path: path.to_path_buf(),
                message: format!("lock_version 必须为 {LOCKFILE_VERSION}"),
            });
        }
        if self.config_fingerprint.is_empty() {
            return Err(LockfileError::Invalid {
                path: path.to_path_buf(),
                message: "config_fingerprint 不得为空".to_owned(),
            });
        }
        if !self.packages.contains_key(&identity_key(&self.root)) {
            return Err(LockfileError::Invalid {
                path: path.to_path_buf(),
                message: "packages 缺少 root 条目".to_owned(),
            });
        }
        for (key, package) in &self.packages {
            if key != &identity_key(&package.identity()) {
                return Err(LockfileError::Invalid {
                    path: path.to_path_buf(),
                    message: format!("包键与包身份不一致：{key}"),
                });
            }
            if !is_digest(&package.content_digest) {
                return Err(LockfileError::Invalid {
                    path: path.to_path_buf(),
                    message: format!("包 {} 的 content_digest 不是 SHA-256", package.name),
                });
            }
            if !package.precompiled_variants.is_empty() || !package.target_conditions.is_empty() {
                return Err(LockfileError::Invalid {
                    path: path.to_path_buf(),
                    message: format!("包 {} 的预编译变体/目标条件在 E2A 不可用", package.name),
                });
            }
            for (name, dependency) in &package.dependencies {
                if dependency.kind != "dependencies" && dependency.kind != "devdependencies" {
                    return Err(LockfileError::Invalid {
                        path: path.to_path_buf(),
                        message: format!("包 {} 的依赖分类无效", package.name),
                    });
                }
                if name != &dependency.target.name
                    || !self
                        .packages
                        .contains_key(&identity_key(&dependency.target))
                {
                    return Err(LockfileError::Invalid {
                        path: path.to_path_buf(),
                        message: format!("包 {} 的依赖 {name} 不在完整依赖图中", package.name),
                    });
                }
            }
        }
        let mut reachable = BTreeSet::new();
        let mut active = BTreeSet::new();
        let mut pending = vec![(identity_key(&self.root), false)];
        while let Some((identity, exiting)) = pending.pop() {
            if exiting {
                active.remove(&identity);
                reachable.insert(identity);
                continue;
            }
            if reachable.contains(&identity) {
                continue;
            }
            if !active.insert(identity.clone()) {
                return Err(LockfileError::Invalid {
                    path: path.to_path_buf(),
                    message: "锁文件依赖图存在环".to_owned(),
                });
            }
            pending.push((identity.clone(), true));
            pending.extend(
                self.packages[&identity]
                    .dependencies
                    .values()
                    .map(|dependency| (identity_key(&dependency.target), false)),
            );
        }
        if reachable.len() != self.packages.len() {
            return Err(LockfileError::Invalid {
                path: path.to_path_buf(),
                message: "锁文件含有不可达的包条目".to_owned(),
            });
        }
        Ok(())
    }

    /// 从带路径上下文的 JSON 文本读取锁文件。
    #[allow(clippy::result_large_err)]
    fn from_json_at(text: &str, path: &Path) -> Result<Self, LockfileError> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|error| LockfileError::Invalid {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        let version = value
            .get("lock_version")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| LockfileError::Invalid {
                path: path.to_path_buf(),
                message: "lock_version 必须是无符号整数".to_owned(),
            })?;
        if version > LOCKFILE_VERSION {
            return Err(LockfileError::UnsupportedVersion {
                path: path.to_path_buf(),
                version,
            });
        }
        let lockfile: Self =
            serde_json::from_value(value).map_err(|error| LockfileError::Invalid {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        lockfile.validate(path)?;
        Ok(lockfile)
    }
}

/// 读取并校验一个项目锁文件。
#[allow(clippy::result_large_err)]
pub fn read_lockfile(path: impl AsRef<Path>) -> Result<LockFile, LockfileError> {
    let path = path.as_ref();
    let text = fs::read_to_string(path).map_err(|error| LockfileError::Io {
        path: path.to_path_buf(),
        operation: "读取锁文件",
        message: error.to_string(),
    })?;
    LockFile::from_json_at(&text, path)
}

/// 原子写入锁文件；内容未变化时返回 `Reused` 而不重写。
#[allow(clippy::result_large_err)]
pub fn write_lockfile(
    path: impl AsRef<Path>,
    lockfile: &LockFile,
) -> Result<LockFileWriteStatus, LockfileError> {
    let path = path.as_ref();
    lockfile.validate(path)?;
    let contents = lockfile.to_json();
    let status = match fs::read(path) {
        Ok(existing) => {
            let text = std::str::from_utf8(&existing).map_err(|error| LockfileError::Invalid {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
            LockFile::from_json_at(text, path)?;
            if existing == contents.as_bytes() {
                LockFileWriteStatus::Reused
            } else {
                LockFileWriteStatus::Updated
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => LockFileWriteStatus::Created,
        Err(error) => {
            return Err(LockfileError::Io {
                path: path.to_path_buf(),
                operation: "读取待更新锁文件",
                message: error.to_string(),
            });
        }
    };
    if status != LockFileWriteStatus::Reused {
        atomic_write_file(path, contents.as_bytes()).map_err(|error| LockfileError::Io {
            path: path.to_path_buf(),
            operation: "原子写入锁文件",
            message: error.to_string(),
        })?;
    }
    Ok(status)
}

/// 生成项目锁文件，或在内容一致时复用已有文件。
#[allow(clippy::result_large_err)]
pub fn generate_or_reuse_lockfile(
    project_root: impl AsRef<Path>,
    graph: &PackageGraph,
    document: &ConfigDocument,
    cache: &CacheStore,
) -> Result<LockFileWriteStatus, LockfileError> {
    let lockfile = build_lockfile(graph, document, cache)?;
    write_lockfile(lockfile_path(project_root), &lockfile)
}

/// 比较当前完整依赖图和本地源码与锁文件是否一致。
#[allow(clippy::result_large_err)]
pub fn compare_lockfile(
    lockfile: &LockFile,
    graph: &PackageGraph,
    document: &ConfigDocument,
) -> Result<(), LockfileError> {
    lockfile.validate(Path::new("<memory>"))?;
    validate_graph(graph)?;
    let mut current_by_name = BTreeMap::new();
    for (identity, node) in &graph.nodes {
        current_by_name.insert(identity.name.clone(), (identity, node));
    }
    let mut locked_by_name = BTreeMap::new();
    for package in lockfile.packages.values() {
        locked_by_name.insert(package.name.clone(), package);
    }
    if current_by_name.len() != locked_by_name.len() {
        return Err(configuration_mismatch("<graph>"));
    }
    for (name, (identity, _)) in &current_by_name {
        let Some(locked) = locked_by_name.get(name) else {
            return Err(configuration_mismatch(name));
        };
        if identity.source.source_id != locked.source.source_id {
            return Err(LockfileError::Mismatch {
                mismatch: LockfileMismatch::SourceChanged {
                    package: name.clone(),
                    expected: locked.source.source_id.clone(),
                    actual: identity.source.source_id.clone(),
                },
            });
        }
    }
    if graph.root != Some(lockfile.root.clone()) {
        return Err(configuration_mismatch("<root>"));
    }
    if lockfile.config_fingerprint != fingerprint_config(document) {
        return Err(configuration_mismatch("<config.xiao>"));
    }
    for (name, (identity, node)) in current_by_name {
        let locked = locked_by_name[&name];
        if identity.version != locked.version || !dependencies_match(node, locked, graph) {
            return Err(configuration_mismatch(&name));
        }
        let actual = source_directory_digest(&node.root)?;
        if actual != locked.content_digest {
            return Err(LockfileError::Mismatch {
                mismatch: LockfileMismatch::ContentChanged {
                    package: name,
                    expected: locked.content_digest.clone(),
                    actual,
                },
            });
        }
    }
    Ok(())
}

/// 读取并比较项目锁文件，供后续 `sync`/`install` 复用。
#[allow(clippy::result_large_err)]
pub fn validate_lockfile(
    path: impl AsRef<Path>,
    graph: &PackageGraph,
    document: &ConfigDocument,
) -> Result<LockFile, LockfileError> {
    let lockfile = read_lockfile(path)?;
    compare_lockfile(&lockfile, graph, document)?;
    Ok(lockfile)
}

/// 供环境模块复用的同文件系统原子文件写入。
pub(crate) fn atomic_write_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    atomic_write_file_with(path, |file| file.write_all(contents))
}

/// 在暂存写入阶段注入写入器，提交前的失败必须保留原目标。
fn atomic_write_file_with(
    path: &Path,
    write_contents: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "目标文件没有父目录"))?;
    fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("xiao-file");
    for _attempt in 0..10_000 {
        let id = NEXT_ATOMIC_FILE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(".{file_name}.tmp-{}-{id}", std::process::id()));
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let written = write_contents(&mut file).and_then(|()| file.sync_all());
        drop(file);
        if let Err(error) = written.and_then(|()| fs::rename(&temporary, path)) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "原子提交暂存文件名称耗尽",
    ))
}

/// 从已解析图节点及其依赖边创建确定性包条目。
fn locked_package(
    node: &PackageNode,
    content_digest: String,
    graph: &PackageGraph,
) -> LockedPackage {
    let mut dependencies = BTreeMap::new();
    if let Some(edges) = graph.edges.get(&node.identity) {
        for edge in edges {
            dependencies.insert(
                edge.dependency.clone(),
                LockedDependency {
                    kind: edge.kind.table_name().to_owned(),
                    target: edge.target.clone(),
                    version_constraint: node.dependencies[&edge.dependency].version.clone(),
                    source_reference: node.dependencies[&edge.dependency].source.clone(),
                    config_path: node.dependencies[&edge.dependency]
                        .path
                        .to_string_lossy()
                        .replace('\\', "/"),
                },
            );
        }
    }
    LockedPackage {
        name: node.identity.name.clone(),
        version: node.identity.version.clone(),
        source: node.identity.source.clone(),
        content_digest,
        dependencies,
        precompiled_variants: Vec::new(),
        target_conditions: Vec::new(),
    }
}

/// 比较声明文本及解析后的全部直接依赖边。
fn dependencies_match(node: &PackageNode, locked: &LockedPackage, graph: &PackageGraph) -> bool {
    let Some(edges) = graph.edges.get(&node.identity) else {
        return locked.dependencies.is_empty();
    };
    if edges.len() != locked.dependencies.len() {
        return false;
    }
    edges.iter().all(|edge| {
        locked
            .dependencies
            .get(&edge.dependency)
            .is_some_and(|dependency| {
                let declared = &node.dependencies[&edge.dependency];
                dependency.kind == edge.kind.table_name()
                    && dependency.target == edge.target
                    && dependency.version_constraint == declared.version
                    && dependency.source_reference == declared.source
                    && dependency.config_path == declared.path.to_string_lossy().replace('\\', "/")
            })
    })
}

/// 拒绝解析失败后的部分图、缺失边和重复的直接依赖边。
fn validate_graph(graph: &PackageGraph) -> Result<(), LockfileError> {
    let root = graph.root.as_ref().ok_or(LockfileError::EmptyGraph)?;
    if !graph.nodes.contains_key(root) || graph.resolution_order.len() != graph.nodes.len() {
        return Err(LockfileError::Invalid {
            path: PathBuf::from("<graph>"),
            message: "包图不完整或存在依赖环".to_owned(),
        });
    }
    for (identity, node) in &graph.nodes {
        if node.identity != *identity
            || graph.edges.get(identity).is_none_or(|edges| {
                edges.len() != node.dependencies.len()
                    || edges
                        .iter()
                        .map(|edge| &edge.dependency)
                        .collect::<BTreeSet<_>>()
                        .len()
                        != node.dependencies.len()
                    || edges.iter().any(|edge| {
                        edge.target.name != edge.dependency
                            || !graph.nodes.contains_key(&edge.target)
                            || node
                                .dependencies
                                .get(&edge.dependency)
                                .is_none_or(|dependency| {
                                    dependency.target.as_ref() != Some(&edge.target)
                                        || dependency.kind != edge.kind
                                })
                    })
            })
        {
            return Err(LockfileError::Invalid {
                path: PathBuf::from("<graph>"),
                message: format!("包 {identity} 的依赖图不完整"),
            });
        }
    }
    Ok(())
}

/// 构造配置与完整依赖图失配的独立诊断。
fn configuration_mismatch(package: &str) -> LockfileError {
    LockfileError::Mismatch {
        mismatch: LockfileMismatch::ConfigurationChanged {
            package: package.to_owned(),
        },
    }
}

/// 将完整包身份转为锁文件的确定性排序键。
fn identity_key(identity: &PackageIdentity) -> String {
    identity.to_string()
}

/// 检查源码对象摘要为 64 位小写十六进制 SHA-256。
fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// 覆盖原子写入阶段失败后的旧文件完整性与后续替换。
#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{self, Write};
    use std::sync::atomic::Ordering;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{NEXT_ATOMIC_FILE, atomic_write_file, atomic_write_file_with};

    #[test]
    /// 暂存文件写入一半即失败时旧文件不损坏，后续写入仍能覆盖。
    fn interrupted_temporary_write_preserves_existing_target() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_ATOMIC_FILE.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "xiao-atomic-write-{stamp}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("create isolated directory");
        let target = directory.join("metadata.json");
        let original = b"{\"ready\":true}\n";
        fs::write(&target, original).expect("write existing metadata");

        let failure = atomic_write_file_with(&target, |file| {
            file.write_all(b"{\"ready\":")?;
            Err(io::Error::other("injected partial write"))
        })
        .expect_err("temporary write must fail");
        assert_eq!(failure.to_string(), "injected partial write");
        assert_eq!(fs::read(&target).expect("old metadata intact"), original);
        assert_eq!(fs::read_dir(&directory).expect("read directory").count(), 1);

        let replacement = b"{\"ready\":false}\n";
        atomic_write_file(&target, replacement).expect("replace after interrupted write");
        assert_eq!(
            fs::read(&target).expect("new metadata complete"),
            replacement
        );
        fs::remove_file(&target).expect("remove test file");
        fs::remove_dir(&directory).expect("remove isolated directory");
    }
}
