//! 项目环境布局、稳定指纹和最小元数据物化。
//!
//! 本模块只消费已经规范化的 [`xiao_config::ConfigDocument`]，不会重新扫描配置源码、
//! 执行项目代码、解析远程来源或生成锁文件。环境目录保存轻量元数据，依赖源码和缓存
//! 由后续 E1/E2 负责。

use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use xiao_codegen_llvm::{CODEGEN_VERSION, TargetDescription, Toolchain, stable_hash};
use xiao_config::ConfigDocument;

use crate::diagnostics::{
    ENVIRONMENT_ALREADY_EXISTS_CODE, ENVIRONMENT_INVALID_NAME_CODE, ENVIRONMENT_WRITE_CODE,
};

/// 环境目录内的元数据文件名。
pub const ENVIRONMENT_METADATA_FILE: &str = ".xiao-environment.json";
/// 当前环境元数据的版本号。
pub const ENVIRONMENT_METADATA_VERSION: u32 = 1;

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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
}

impl EnvironmentMetadata {
    /// 将元数据编码为稳定、无绝对路径的 JSON 文本。
    #[must_use]
    pub fn to_json(&self) -> String {
        let value = serde_json::json!({
            "metadata_version": self.metadata_version,
            "logical_name": self.logical_name,
            "directory_name": self.directory_name,
            "config_fingerprint": self.config_fingerprint,
            "toolchain_fingerprint": self.toolchain_fingerprint,
            "target_fingerprint": self.target_fingerprint,
            "environment_fingerprint": self.environment_fingerprint,
            "lockfile_summary": self.lockfile_summary,
        });
        format!(
            "{}\n",
            serde_json::to_string_pretty(&value).expect("环境元数据必须可编码为 JSON")
        )
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
}

impl EnvironmentError {
    /// 返回稳定诊断编号。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidName { .. } => ENVIRONMENT_INVALID_NAME_CODE,
            Self::AlreadyExists { .. } => ENVIRONMENT_ALREADY_EXISTS_CODE,
            Self::Write { .. } => ENVIRONMENT_WRITE_CODE,
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
        }
    }
}

impl std::error::Error for EnvironmentError {}

/// 环境创建错误的语义别名。
pub type EnvironmentCreationError = EnvironmentError;

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
    let layout = match logical_name {
        Some(name) => EnvironmentLayout::named_for_project(project_root, name)?,
        None => EnvironmentLayout::default_for_project(project_root),
    };
    let metadata = build_environment_metadata(&layout, document, toolchain, target);
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
    if let Err(error) = fs::write(&metadata_path, metadata.to_json()) {
        let _ = fs::remove_file(&metadata_path);
        let _ = fs::remove_dir(&layout.path);
        return Err(EnvironmentError::Write {
            path: metadata_path,
            operation: "写入元数据",
            message: error.to_string(),
        });
    }
    Ok(metadata)
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
