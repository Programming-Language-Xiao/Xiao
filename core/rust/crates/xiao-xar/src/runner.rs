//! `.xar` 入口校验与统一 VM 运行器。

use std::fmt::{Display, Formatter};

use xiao_artifacts::ArtifactError;
use xiao_i18n::LocaleContext;

use super::{
    ARCHIVE_DEPENDENCY_UNSATISFIED_CODE, ARCHIVE_MISSING_OBJECT_CODE,
    ARCHIVE_VALIDATION_FAILED_CODE, ARCHIVE_VERSION_INCOMPATIBLE_CODE, ObjectKind, XarError,
    decode_xar,
};

/// 归档执行前的宿主约束；入口和对象仍只来自已验证的唯一索引。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XarRunOptions {
    /// 当前宿主支持的 Runtime ABI 编码。
    pub runtime_abi: u64,
    /// 当前宿主目标三元组；空字符串表示调用方暂不施加平台筛选。
    pub platform: String,
    /// 调用方显式指定的语言；为空时采用归档索引默认值。
    pub language_locale: Option<String>,
    /// 传给统一 VM 入口的执行参数。
    pub vm_options: xiao_vm::VmOptions,
}

impl Default for XarRunOptions {
    fn default() -> Self {
        Self {
            runtime_abi: xiao_runtime_abi::ABI_ENCODED_VERSION,
            platform: String::new(),
            language_locale: None,
            vm_options: xiao_vm::VmOptions::default(),
        }
    }
}

/// 归档运行时解析出的语言上下文。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XarLanguageResolution {
    /// 请求或归档携带的原始语言标签。
    pub requested: String,
    /// 实际用于内置目录渲染的规范标签。
    pub effective: String,
    /// 是否因为缺少内置目录而发生回落。
    pub fallback: bool,
}

/// 按显式参数、归档默认值、`zh-CN` 缺省顺序解析语言。
#[must_use]
pub fn resolve_language_locale(
    archive_locale: &str,
    explicit_locale: Option<&str>,
) -> XarLanguageResolution {
    let requested = explicit_locale
        .filter(|value| !value.trim().is_empty())
        .or_else(|| (!archive_locale.trim().is_empty()).then_some(archive_locale))
        .unwrap_or("zh-CN")
        .to_owned();
    match LocaleContext::from_config(&requested) {
        Ok(context) => XarLanguageResolution {
            requested,
            effective: context.tag().to_owned(),
            fallback: false,
        },
        Err(_) => XarLanguageResolution {
            requested,
            effective: "zh-CN".to_owned(),
            fallback: true,
        },
    }
}

/// 归档运行器在 VM 建立前报告的稳定错误类别。
#[derive(Debug)]
pub enum XarRunError {
    /// 入口或索引对象缺失。
    MissingObject(String),
    /// 归档格式、索引或 Runtime ABI 版本不兼容。
    VersionIncompatible(String),
    /// 依赖锁、平台或其他运行前提未满足。
    DependencyUnsatisfied(String),
    /// 归档、对象或 `.xiaoc` 校验失败。
    Validation(String),
}

impl XarRunError {
    /// 返回 17C 冻结的稳定错误码。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::MissingObject(_) => ARCHIVE_MISSING_OBJECT_CODE,
            Self::VersionIncompatible(_) => ARCHIVE_VERSION_INCOMPATIBLE_CODE,
            Self::DependencyUnsatisfied(_) => ARCHIVE_DEPENDENCY_UNSATISFIED_CODE,
            Self::Validation(_) => ARCHIVE_VALIDATION_FAILED_CODE,
        }
    }

    /// 返回稳定消息目录键。
    #[must_use]
    pub const fn message_id(&self) -> &'static str {
        match self {
            Self::MissingObject(_) => "x17.xar.archive_missing_object",
            Self::VersionIncompatible(_) => "x17.xar.archive_version_incompatible",
            Self::DependencyUnsatisfied(_) => "x17.xar.archive_dependency_unsatisfied",
            Self::Validation(_) => "x17.xar.archive_validation_failed",
        }
    }

    /// 返回开发者可读原因。
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::MissingObject(message)
            | Self::VersionIncompatible(message)
            | Self::DependencyUnsatisfied(message)
            | Self::Validation(message) => message,
        }
    }
}

impl Display for XarRunError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for XarRunError {}

/// 打开、完整验证并运行归档入口。
pub fn run_archive(
    bytes: &[u8],
    options: XarRunOptions,
) -> Result<xiao_vm::RunOutcome, XarRunError> {
    let archive = decode_xar(bytes).map_err(classify_run_error)?;
    let _language = resolve_language_locale(
        &archive.index().language_locale,
        options.language_locale.as_deref(),
    );
    let index = archive.index();
    if options.runtime_abi < index.runtime_abi_min || options.runtime_abi > index.runtime_abi_max {
        return Err(XarRunError::VersionIncompatible(format!(
            "Runtime ABI {} 不在归档要求的 {}..={} 范围内",
            options.runtime_abi, index.runtime_abi_min, index.runtime_abi_max
        )));
    }
    if !options.platform.is_empty()
        && !index.platform.is_empty()
        && index.platform != "portable"
        && index.platform != options.platform
    {
        return Err(XarRunError::DependencyUnsatisfied(format!(
            "归档平台 {} 与当前平台 {} 不兼容",
            index.platform, options.platform
        )));
    }
    for entry in &index.entries {
        if !options.platform.is_empty()
            && entry.target != "portable"
            && entry.target != options.platform
        {
            return Err(XarRunError::DependencyUnsatisfied(format!(
                "模块 {} 要求平台 {}",
                entry.logical_path, entry.target
            )));
        }
    }
    let entry = index
        .entries
        .iter()
        .find(|entry| entry.logical_path == index.entry)
        .ok_or_else(|| XarRunError::MissingObject(index.entry.clone()))?;
    if entry.object_kind != ObjectKind::Xiaoc {
        return Err(XarRunError::Validation(format!(
            "入口 {} 不是 `.xiaoc` 对象",
            entry.logical_path
        )));
    }
    let entry_bytes = archive
        .read_object(entry.object_kind, entry.digest)
        .map_err(classify_run_error)?;
    xiao_vm::run_xiaoc(&entry_bytes, options.vm_options)
        .map_err(|error| XarRunError::Validation(format!("入口 `.xiaoc` 执行前校验失败：{error}")))
}

fn classify_run_error(error: XarError) -> XarRunError {
    match error {
        XarError::MissingMember(path) => XarRunError::MissingObject(path),
        XarError::Artifact(ArtifactError::UnsupportedIndexVersion { major, .. }) => {
            XarRunError::VersionIncompatible(format!("不支持的归档索引主版本 {major}"))
        }
        XarError::Unsupported(message) if message.contains("版本") => {
            XarRunError::VersionIncompatible(message)
        }
        XarError::InvalidIndex(message)
            if message.contains("对象缺失") || message.contains("入口没有") =>
        {
            XarRunError::MissingObject(message)
        }
        other => XarRunError::Validation(other.to_string()),
    }
}
