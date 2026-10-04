//! `.xar` 入口校验与统一 VM 运行器。

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use xiao_artifacts::{ArtifactError, Digest256};
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
    /// 生产事件接收器容量；协议入口与源码运行保持同一范围。
    pub event_capacity: usize,
    /// 调用方是否显式请求独立诊断窗口。
    pub debug: bool,
}

impl Default for XarRunOptions {
    fn default() -> Self {
        Self {
            runtime_abi: xiao_runtime_abi::ABI_ENCODED_VERSION,
            platform: String::new(),
            language_locale: None,
            vm_options: xiao_vm::VmOptions::default(),
            event_capacity: xiao_vm::DEFAULT_EVENT_CAPACITY,
            debug: false,
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

/// 一次归档校验的机器可读审计记录，不包含凭据、环境变量值或用户源码。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArchiveAuditRecord {
    /// 完整归档字节摘要。
    pub archive_digest: String,
    /// 索引 Schema 版本。
    pub index_schema_major: u32,
    /// 索引 Schema 次版本。
    pub index_schema_minor: u32,
    /// 已验证的物理成员数量。
    pub verified_member_count: usize,
    /// 归档声明的平台。
    pub archive_platform: String,
    /// 本次校验使用的宿主平台。
    pub host_platform: String,
    /// 当前 Runtime ABI 是否满足归档范围。
    pub runtime_abi_compatible: bool,
    /// 当前平台是否满足归档约束。
    pub platform_compatible: bool,
    /// 归档调试激活位。
    pub debug_activation: bool,
    /// 请求或归档默认语言。
    pub language_requested: String,
    /// 实际使用的语言。
    pub language_effective: String,
    /// 是否发生语言回落。
    pub language_fallback: bool,
}

impl ArchiveAuditRecord {
    /// 以确定性 JSON 字节输出审计记录。
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

/// 只做归档校验并生成机器可读审计记录，不建立 VM。
pub fn audit_archive(
    bytes: &[u8],
    options: &XarRunOptions,
) -> Result<ArchiveAuditRecord, XarRunError> {
    let archive = decode_xar(bytes).map_err(classify_run_error)?;
    let index = archive.index();
    let language =
        resolve_language_locale(&index.language_locale, options.language_locale.as_deref());
    let runtime_abi_compatible = options.runtime_abi >= index.runtime_abi_min
        && options.runtime_abi <= index.runtime_abi_max;
    let platform_compatible = options.platform.is_empty()
        || index.platform.is_empty()
        || index.platform == "portable"
        || index.platform == options.platform;
    Ok(ArchiveAuditRecord {
        archive_digest: Digest256::of_bytes(bytes).as_hex(),
        index_schema_major: index.schema_major,
        index_schema_minor: index.schema_minor,
        verified_member_count: archive.members().len(),
        archive_platform: index.platform.clone(),
        host_platform: options.platform.clone(),
        runtime_abi_compatible,
        platform_compatible,
        debug_activation: index.debug_activation || options.debug,
        language_requested: language.requested,
        language_effective: language.effective,
        language_fallback: language.fallback,
    })
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
    run_archive_with_control(bytes, options, None)
}

/// 打开、验证并运行归档，同时接入生产 VM 的取消与截止时间来源。
pub fn run_archive_with_control(
    bytes: &[u8],
    options: XarRunOptions,
    cancellation: Option<xiao_vm::CancellationSource>,
) -> Result<xiao_vm::RunOutcome, XarRunError> {
    let archive = decode_xar(bytes).map_err(classify_run_error)?;
    let _language = resolve_language_locale(
        &archive.index().language_locale,
        options.language_locale.as_deref(),
    );
    let index = archive.index();
    if options.event_capacity == 0 || options.event_capacity > xiao_vm::MAX_EVENT_CAPACITY {
        return Err(XarRunError::Validation(format!(
            "事件容量必须位于 1..={}（收到 {}）",
            xiao_vm::MAX_EVENT_CAPACITY,
            options.event_capacity
        )));
    }
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
    xiao_vm::run_xiaoc_production(
        &entry_bytes,
        options.vm_options,
        options.event_capacity,
        cancellation,
    )
    .map_err(|error| XarRunError::Validation(format!("入口 `.xiaoc` 执行前校验失败：{error}")))
}

/// 运行归档并把已产生的 VM 事件交给调用方观察；校验失败时回调不会被调用。
pub fn run_archive_with_event_observer(
    bytes: &[u8],
    options: XarRunOptions,
    mut observe: impl FnMut(&xiao_vm::VmEvent),
) -> Result<xiao_vm::RunOutcome, XarRunError> {
    let outcome = run_archive(bytes, options)?;
    for event in &outcome.events {
        observe(event);
    }
    Ok(outcome)
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
