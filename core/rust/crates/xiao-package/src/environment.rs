//! 项目环境布局、稳定指纹和最小元数据物化。
//!
//! 本模块只消费已经规范化的 [`xiao_config::ConfigDocument`]，不会重新扫描配置源码、
//! 执行项目代码、解析远程来源或生成锁文件。环境目录保存轻量元数据，依赖源码和缓存
//! 由 E1/E2A 的独立模块负责。

use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use xiao_codegen_llvm::{CODEGEN_VERSION, TargetDescription, Toolchain, stable_hash};
use xiao_config::ConfigDocument;

use crate::cache::CacheStore;
use crate::diagnostics::{
    ENVIRONMENT_ALREADY_EXISTS_CODE, ENVIRONMENT_INVALID_NAME_CODE,
    ENVIRONMENT_METADATA_VERSION_CODE, ENVIRONMENT_WRITE_CODE,
};
use crate::lockfile::atomic_write_file;
use crate::mapping::{
    MappingError, PackageObjectMapping, materialize_package_mappings, validate_package_mappings,
};
use crate::model::PackageGraph;

/// 环境目录内的元数据文件名。
pub const ENVIRONMENT_METADATA_FILE: &str = ".xiao-environment.json";
/// 当前环境元数据的版本号。
pub const ENVIRONMENT_METADATA_VERSION: u32 = 2;

/// 环境逻辑名称、目录名称和绝对落点。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentLayout {
    /// 用户引用环境时使用的逻辑名称。
    pub logical_name: String,
    /// 相对于项目根的目录名称。
    pub directory_name: String,
    /// 环境目录路径。
    pub path: PathBuf,
}

impl EnvironmentLayout {
    /// 根据项目根和可选逻辑名称构造环境布局。
    pub fn for_project(
        project_root: impl AsRef<Path>,
        logical_name: Option<&str>,
    ) -> Result<Self, EnvironmentError> {
        match logical_name {
            None => Ok(Self::default_for_project(project_root)),
            Some(name) => Self::named_for_project(project_root, name),
        }
    }

    /// 构造默认的 `venv`/`.venv` 环境。
    pub fn default_for_project(project_root: impl AsRef<Path>) -> Self {
        let root = project_root.as_ref();
        Self {
            logical_name: "venv".to_owned(),
            directory_name: ".venv".to_owned(),
            path: root.join(".venv"),
        }
    }

    /// 构造显式名称环境；逻辑名称和目录名称相同。
    pub fn named_for_project(
        project_root: impl AsRef<Path>,
        logical_name: &str,
    ) -> Result<Self, EnvironmentError> {
        validate_environment_name(logical_name)?;
        Ok(Self {
            logical_name: logical_name.to_owned(),
            directory_name: logical_name.to_owned(),
            path: project_root.as_ref().join(logical_name),
        })
    }
}

/// 配置、工具链、目标和汇总环境指纹。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentFingerprint {
    /// 规范化配置树指纹。
    pub config: String,
    /// LLVM 工具链和 ABI 指纹。
    pub toolchain: String,
    /// 目标平台字段指纹。
    pub target: String,
    /// 三个维度与环境名称组合后的汇总指纹。
    pub environment: String,
}

/// 环境目录内保存的最小元数据。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnvironmentMetadata {
    /// 元数据格式版本。
    pub metadata_version: u32,
    /// 环境逻辑名称。
    pub logical_name: String,
    /// 环境目录名称。
    pub directory_name: String,
    /// 规范化配置树指纹。
    pub config_fingerprint: String,
    /// 工具链指纹。
    pub toolchain_fingerprint: String,
    /// 目标指纹。
    pub target_fingerprint: String,
    /// 汇总环境指纹。
    pub environment_fingerprint: String,
    /// 后续锁文件摘要预留字段。
    pub lockfile_summary: Option<String>,
    /// 按包逻辑身份排序的全局源码对象映射。
    pub package_mappings: Vec<PackageObjectMapping>,
}

/// 用于兼容读取 v1/v2 元数据的内部反序列化形状。
#[derive(Debug, Deserialize)]
struct EnvironmentMetadataFields {
    metadata_version: u32,
    logical_name: String,
    directory_name: String,
    config_fingerprint: String,
    toolchain_fingerprint: String,
    target_fingerprint: String,
    environment_fingerprint: String,
    lockfile_summary: Option<String>,
    #[serde(default)]
    package_mappings: Vec<PackageObjectMapping>,
}

impl<'de> Deserialize<'de> for EnvironmentMetadata {
    /// 读取 v1/v2 元数据，并拒绝高于当前读取器的版本。
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = EnvironmentMetadataFields::deserialize(deserializer)?;
        if fields.metadata_version > ENVIRONMENT_METADATA_VERSION {
            return Err(de::Error::custom(format!(
                "不支持的环境元数据版本：{}",
                fields.metadata_version
            )));
        }
        Ok(Self {
            metadata_version: fields.metadata_version,
            logical_name: fields.logical_name,
            directory_name: fields.directory_name,
            config_fingerprint: fields.config_fingerprint,
            toolchain_fingerprint: fields.toolchain_fingerprint,
            target_fingerprint: fields.target_fingerprint,
            environment_fingerprint: fields.environment_fingerprint,
            lockfile_summary: fields.lockfile_summary,
            package_mappings: fields.package_mappings,
        })
    }
}

impl EnvironmentMetadata {
    /// 将元数据编码为稳定、无绝对路径的 JSON 文本。
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            "{}\n",
            serde_json::to_string_pretty(self).expect("环境元数据必须可编码为 JSON")
        )
    }

    /// 从 JSON 文本读取 v1/v2 环境元数据并拒绝未来版本。
    pub fn from_json(text: &str) -> Result<Self, EnvironmentError> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|error| EnvironmentError::MetadataRead {
                path: PathBuf::from("<json>"),
                operation: "解析环境元数据",
                message: error.to_string(),
            })?;
        let version = value
            .get("metadata_version")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| EnvironmentError::MetadataRead {
                path: PathBuf::from("<json>"),
                operation: "读取环境元数据版本",
                message: "metadata_version 必须是无符号整数".to_owned(),
            })?;
        if version > ENVIRONMENT_METADATA_VERSION {
            return Err(EnvironmentError::UnsupportedMetadataVersion {
                path: PathBuf::from("<json>"),
                version,
            });
        }
        serde_json::from_value(value).map_err(|error| EnvironmentError::MetadataRead {
            path: PathBuf::from("<json>"),
            operation: "读取环境元数据",
            message: error.to_string(),
        })
    }
}

/// 环境创建、命名或元数据写入失败。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvironmentError {
    /// 环境逻辑名称非法。
    InvalidName {
        /// 被拒绝的环境名称。
        name: String,
    },
    /// 目标环境目录已存在。
    AlreadyExists {
        /// 已存在的环境目录。
        path: PathBuf,
    },
    /// 环境目录或元数据写入失败。
    Write {
        /// 失败操作涉及的路径。
        path: PathBuf,
        /// 失败操作名称。
        operation: &'static str,
        /// 主机错误文本。
        message: String,
    },
    /// 元数据版本高于当前读取器。
    UnsupportedMetadataVersion {
        /// 元数据文件路径。
        path: PathBuf,
        /// 不支持的版本号。
        version: u32,
    },
    /// 元数据读取或解析失败。
    MetadataRead {
        /// 元数据文件路径。
        path: PathBuf,
        /// 失败操作名称。
        operation: &'static str,
        /// 主机错误文本。
        message: String,
    },
}

impl EnvironmentError {
    /// 返回稳定诊断编号。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidName { .. } => ENVIRONMENT_INVALID_NAME_CODE,
            Self::AlreadyExists { .. } => ENVIRONMENT_ALREADY_EXISTS_CODE,
            Self::Write { .. } => ENVIRONMENT_WRITE_CODE,
            Self::UnsupportedMetadataVersion { .. } => ENVIRONMENT_METADATA_VERSION_CODE,
            Self::MetadataRead { .. } => ENVIRONMENT_WRITE_CODE,
        }
    }
}

impl Display for EnvironmentError {
    /// 格式化稳定环境诊断。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName { name } => {
                write!(formatter, "{}: 非法环境名称：{name}", self.code())
            }
            Self::AlreadyExists { path } => {
                write!(formatter, "{}: 环境已存在：{}", self.code(), path.display())
            }
            Self::Write {
                path,
                operation,
                message,
            } => write!(
                formatter,
                "{}: {operation} 环境路径 {} 失败：{message}",
                self.code(),
                path.display()
            ),
            Self::UnsupportedMetadataVersion { path, version } => write!(
                formatter,
                "{}: 环境元数据 {} 的版本 {version} 高于当前读取器",
                self.code(),
                path.display()
            ),
            Self::MetadataRead {
                path,
                operation,
                message,
            } => write!(
                formatter,
                "{}: {operation} 环境元数据 {} 失败：{message}",
                self.code(),
                path.display()
            ),
        }
    }
}

impl std::error::Error for EnvironmentError {}

/// 环境创建错误的语义别名。
pub type EnvironmentCreationError = EnvironmentError;

/// 环境包图物化失败，保留环境和缓存两个粒度的诊断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvironmentPackageError {
    /// 环境目录或元数据物化失败。
    Environment(EnvironmentError),
    /// 包图到缓存对象的映射失败。
    Mapping(MappingError),
}

impl Display for EnvironmentPackageError {
    /// 输出底层环境或映射诊断。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Environment(error) => Display::fmt(error, formatter),
            Self::Mapping(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for EnvironmentPackageError {}

impl From<EnvironmentError> for EnvironmentPackageError {
    /// 将环境错误包装为包图物化错误。
    fn from(error: EnvironmentError) -> Self {
        Self::Environment(error)
    }
}

impl From<MappingError> for EnvironmentPackageError {
    /// 将映射错误包装为包图物化错误。
    fn from(error: MappingError) -> Self {
        Self::Mapping(error)
    }
}

/// 为配置文档生成稳定指纹。
#[must_use]
pub fn fingerprint_config(document: &ConfigDocument) -> String {
    format!(
        "xiao-config-fingerprint-v1-{}",
        stable_hash(&document.canonical_fingerprint_input())
    )
}

/// 为目标平台字段生成稳定指纹。
#[must_use]
pub fn fingerprint_target(target: &TargetDescription) -> String {
    format!(
        "xiao-target-fingerprint-v1-{}",
        stable_hash(target.fingerprint_fields().as_bytes())
    )
}

/// 组合配置、工具链、目标和名称得到环境汇总指纹。
#[must_use]
pub fn fingerprint_environment(
    layout: &EnvironmentLayout,
    config: &str,
    toolchain: &str,
    target: &str,
) -> String {
    let mut input = Vec::new();
    input.extend_from_slice(b"xiao-environment-fingerprint-v1\0");
    append_text(&mut input, &layout.logical_name);
    append_text(&mut input, &layout.directory_name);
    append_text(&mut input, config);
    append_text(&mut input, toolchain);
    append_text(&mut input, target);
    format!("xiao-environment-fingerprint-v1-{}", stable_hash(&input))
}

/// 从已经规范化的配置文档和显式工具链/目标描述生成环境元数据。
#[must_use]
pub fn build_environment_metadata(
    layout: &EnvironmentLayout,
    document: &ConfigDocument,
    toolchain: &Toolchain,
    target: &TargetDescription,
) -> EnvironmentMetadata {
    build_environment_metadata_with_mappings(layout, document, toolchain, target, Vec::new())
}

/// 从配置、工具链、目标和已准备的包映射生成 v2 环境元数据。
#[must_use]
pub fn build_environment_metadata_with_mappings(
    layout: &EnvironmentLayout,
    document: &ConfigDocument,
    toolchain: &Toolchain,
    target: &TargetDescription,
    mut package_mappings: Vec<PackageObjectMapping>,
) -> EnvironmentMetadata {
    package_mappings.sort_by(|left, right| left.package.cmp(&right.package));
    let config_fingerprint = fingerprint_config(document);
    let target_fingerprint = fingerprint_target(target);
    let toolchain_fingerprint = toolchain.fingerprint(target, CODEGEN_VERSION).to_string();
    let environment_fingerprint = fingerprint_environment(
        layout,
        &config_fingerprint,
        &toolchain_fingerprint,
        &target_fingerprint,
    );
    EnvironmentMetadata {
        metadata_version: ENVIRONMENT_METADATA_VERSION,
        logical_name: layout.logical_name.clone(),
        directory_name: layout.directory_name.clone(),
        config_fingerprint,
        toolchain_fingerprint,
        target_fingerprint,
        environment_fingerprint,
        lockfile_summary: None,
        package_mappings,
    }
}

/// 创建环境目录并写入最小元数据；已存在目录不会被覆盖。
pub fn materialize_environment(
    project_root: impl AsRef<Path>,
    logical_name: Option<&str>,
    document: &ConfigDocument,
    toolchain: &Toolchain,
    target: &TargetDescription,
) -> Result<EnvironmentMetadata, EnvironmentError> {
    materialize_environment_with_mappings(
        project_root,
        logical_name,
        document,
        toolchain,
        target,
        Vec::new(),
    )
}

/// 创建环境目录并写入包含包映射的 v2 元数据。
pub fn materialize_environment_with_mappings(
    project_root: impl AsRef<Path>,
    logical_name: Option<&str>,
    document: &ConfigDocument,
    toolchain: &Toolchain,
    target: &TargetDescription,
    package_mappings: Vec<PackageObjectMapping>,
) -> Result<EnvironmentMetadata, EnvironmentError> {
    let layout = match logical_name {
        Some(name) => EnvironmentLayout::named_for_project(project_root, name)?,
        None => EnvironmentLayout::default_for_project(project_root),
    };
    materialize_environment_at_layout(&layout, document, toolchain, target, package_mappings)
}

/// 从 D1 包图导入源码对象并物化项目环境及其逻辑映射。
#[allow(clippy::result_large_err)]
pub fn materialize_environment_from_graph(
    project_root: impl AsRef<Path>,
    logical_name: Option<&str>,
    document: &ConfigDocument,
    toolchain: &Toolchain,
    target: &TargetDescription,
    graph: &PackageGraph,
    cache: &CacheStore,
) -> Result<EnvironmentMetadata, EnvironmentPackageError> {
    let mappings = materialize_package_mappings(graph, cache)?;
    Ok(materialize_environment_with_mappings(
        project_root,
        logical_name,
        document,
        toolchain,
        target,
        mappings,
    )?)
}

/// 从 D1 包图导入源码对象并物化用户域下的全局环境。
#[allow(clippy::result_large_err)]
pub fn materialize_global_environment_from_graph(
    logical_name: &str,
    document: &ConfigDocument,
    toolchain: &Toolchain,
    target: &TargetDescription,
    graph: &PackageGraph,
    cache: &CacheStore,
) -> Result<EnvironmentMetadata, EnvironmentPackageError> {
    let mappings = materialize_package_mappings(graph, cache)?;
    let layout =
        EnvironmentLayout::named_for_project(cache.layout().environments_root(), logical_name)?;
    Ok(materialize_environment_at_layout(
        &layout, document, toolchain, target, mappings,
    )?)
}

/// 在已计算好的环境布局中创建目录并写入元数据。
fn materialize_environment_at_layout(
    layout: &EnvironmentLayout,
    document: &ConfigDocument,
    toolchain: &Toolchain,
    target: &TargetDescription,
    package_mappings: Vec<PackageObjectMapping>,
) -> Result<EnvironmentMetadata, EnvironmentError> {
    let metadata = build_environment_metadata_with_mappings(
        layout,
        document,
        toolchain,
        target,
        package_mappings,
    );
    fs::create_dir(&layout.path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            EnvironmentError::AlreadyExists {
                path: layout.path.clone(),
            }
        } else {
            EnvironmentError::Write {
                path: layout.path.clone(),
                operation: "创建目录",
                message: error.to_string(),
            }
        }
    })?;
    let metadata_path = layout.path.join(ENVIRONMENT_METADATA_FILE);
    if let Err(error) = atomic_write_file(&metadata_path, metadata.to_json().as_bytes()) {
        let _ = fs::remove_dir(&layout.path);
        return Err(EnvironmentError::Write {
            path: metadata_path,
            operation: "写入元数据",
            message: error.to_string(),
        });
    }
    Ok(metadata)
}

/// 原子更新既有环境的元数据映射。
///
/// 该入口不重新创建环境目录；写入先进入同目录暂存文件，再使用平台替换语义提交，
/// 因而中断只会留下旧元数据或完整的新元数据，不会留下半截 JSON。
#[allow(clippy::result_large_err)]
pub fn update_environment_metadata(
    metadata_path: impl AsRef<Path>,
    metadata: &EnvironmentMetadata,
) -> Result<(), EnvironmentError> {
    let path = metadata_path.as_ref();
    read_environment_metadata(path)?;
    atomic_write_file(path, metadata.to_json().as_bytes()).map_err(|error| {
        EnvironmentError::Write {
            path: path.to_path_buf(),
            operation: "原子更新元数据",
            message: error.to_string(),
        }
    })
}

/// 将已存在项目或全局环境的包映射原子替换为完整的新映射。
#[allow(clippy::result_large_err)]
pub fn update_environment_mappings(
    metadata_path: impl AsRef<Path>,
    package_mappings: Vec<PackageObjectMapping>,
) -> Result<EnvironmentMetadata, EnvironmentPackageError> {
    validate_package_mappings(&package_mappings)?;
    let path = metadata_path.as_ref();
    let mut metadata = read_environment_metadata(path)?;
    metadata.metadata_version = ENVIRONMENT_METADATA_VERSION;
    metadata.package_mappings = package_mappings;
    update_environment_metadata(path, &metadata)?;
    Ok(metadata)
}

/// 读取环境目录内的元数据并对未来版本给出稳定诊断。
pub fn read_environment_metadata(
    metadata_path: impl AsRef<Path>,
) -> Result<EnvironmentMetadata, EnvironmentError> {
    let path = metadata_path.as_ref();
    let text = fs::read_to_string(path).map_err(|error| EnvironmentError::MetadataRead {
        path: path.to_path_buf(),
        operation: "读取环境元数据",
        message: error.to_string(),
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| EnvironmentError::MetadataRead {
            path: path.to_path_buf(),
            operation: "解析环境元数据",
            message: error.to_string(),
        })?;
    let version = value
        .get("metadata_version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| EnvironmentError::MetadataRead {
            path: path.to_path_buf(),
            operation: "读取环境元数据版本",
            message: "metadata_version 必须是无符号整数".to_owned(),
        })?;
    if version > ENVIRONMENT_METADATA_VERSION {
        return Err(EnvironmentError::UnsupportedMetadataVersion {
            path: path.to_path_buf(),
            version,
        });
    }
    serde_json::from_value(value).map_err(|error| EnvironmentError::MetadataRead {
        path: path.to_path_buf(),
        operation: "读取环境元数据",
        message: error.to_string(),
    })
}

/// 校验环境名，拒绝路径穿越、分隔符和控制字符。
fn validate_environment_name(name: &str) -> Result<(), EnvironmentError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name
            .chars()
            .any(|character| character.is_control() || character == '/' || character == '\\')
    {
        return Err(EnvironmentError::InvalidName {
            name: name.to_owned(),
        });
    }
    Ok(())
}

/// 追加带长度边界的环境指纹文本字段。
fn append_text(output: &mut Vec<u8>, text: &str) {
    output.extend_from_slice(
        &u64::try_from(text.len())
            .expect("环境指纹字段长度必须能表示为 64 位无符号整数")
            .to_be_bytes(),
    );
    output.extend_from_slice(text.as_bytes());
}
