//! `.xar` 入口校验与统一 VM 运行器。

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use xiao_artifacts::{ArtifactError, Digest256};
use xiao_i18n::LocaleContext;
use xiao_intrinsics::{KNOWN_CAPABILITY_MASK, validate_capability_mask};

use super::{
    ARCHIVE_DEPENDENCY_UNSATISFIED_CODE, ARCHIVE_INVALID_REQUIREMENTS_CODE,
    ARCHIVE_MISSING_OBJECT_CODE, ARCHIVE_VALIDATION_FAILED_CODE, ARCHIVE_VERSION_INCOMPATIBLE_CODE,
    ObjectKind, XarError, open_indexed_archive,
};

/// 归档执行前的宿主约束；入口和对象仍只来自已验证的唯一索引。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XarRunOptions {
    /// 当前宿主支持的 Runtime ABI 编码。
    pub runtime_abi: u64,
    /// 当前宿主支持的归档必需功能位；bit 0 保留给索引要求身份。
    pub runtime_features: u64,
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
    /// 本次请求允许的不可变能力上下文。
    pub capabilities: u64,
}

impl Default for XarRunOptions {
    fn default() -> Self {
        Self {
            runtime_abi: xiao_runtime_abi::ABI_ENCODED_VERSION,
            runtime_features: xiao_artifacts::ARCHIVE_REQUIRED_FEATURES
                | xiao_runtime_abi::SUPPORTED_CONCURRENCY_FEATURES,
            platform: String::new(),
            language_locale: None,
            vm_options: xiao_vm::VmOptions::default(),
            event_capacity: xiao_vm::DEFAULT_EVENT_CAPACITY,
            debug: false,
            capabilities: KNOWN_CAPABILITY_MASK,
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
    let indexed = open_indexed_archive(bytes).map_err(classify_run_error)?;
    validate_requirements(indexed.index(), options)?;
    let archive = indexed.verify().map_err(classify_run_error)?;
    Ok(audit_record(
        bytes,
        archive.index(),
        archive.members().len(),
        options,
    ))
}

/// 只读取并验证索引，不解码对象；返回的审计仅计入已经校验 CRC 的一个索引成员。
///
/// 供协议语言选择和启动预检使用；对象仍须经完整归档校验后才能读取或执行。
pub fn inspect_archive_index(
    bytes: &[u8],
    options: &XarRunOptions,
) -> Result<(xiao_artifacts::ArchiveIndex, ArchiveAuditRecord), XarRunError> {
    let indexed = open_indexed_archive(bytes).map_err(classify_run_error)?;
    let record = audit_record(bytes, indexed.index(), 1, options);
    Ok((indexed.index().clone(), record))
}

/// 成功启动和索引层拒绝复用同一审计构造；已验证数量由调用方如实提供。
fn audit_record(
    bytes: &[u8],
    index: &xiao_artifacts::ArchiveIndex,
    verified_member_count: usize,
    options: &XarRunOptions,
) -> ArchiveAuditRecord {
    let language =
        resolve_language_locale(&index.language_locale, options.language_locale.as_deref());
    let runtime_abi_compatible = options.runtime_abi >= index.runtime_abi_min
        && options.runtime_abi <= index.runtime_abi_max;
    let platform_compatible = options.platform.is_empty()
        || ((index.platform.is_empty()
            || index.platform == "portable"
            || index.platform == options.platform)
            && index
                .entries
                .iter()
                .all(|entry| entry.target == "portable" || entry.target == options.platform));
    ArchiveAuditRecord {
        archive_digest: Digest256::of_bytes(bytes).as_hex(),
        index_schema_major: index.schema_major,
        index_schema_minor: index.schema_minor,
        verified_member_count,
        archive_platform: index.platform.clone(),
        host_platform: options.platform.clone(),
        runtime_abi_compatible,
        platform_compatible,
        debug_activation: index.debug_activation || options.debug,
        language_requested: language.requested,
        language_effective: language.effective,
        language_fallback: language.fallback,
    }
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
    /// 索引要求含未知能力位或无效元数据。
    InvalidRequirements(String),
    /// 本次请求缺少归档运行必需能力。
    CapabilityUnavailable(String),
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
            Self::InvalidRequirements(_) => ARCHIVE_INVALID_REQUIREMENTS_CODE,
            Self::CapabilityUnavailable(_) => xiao_diagnostics::CAPABILITY_UNAVAILABLE_CODE,
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
            Self::InvalidRequirements(_) => "x17.xar.archive_invalid_requirements",
            Self::CapabilityUnavailable(_) => "x17.xar.archive_capability_unavailable",
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
            Self::InvalidRequirements(message) | Self::CapabilityUnavailable(message) => message,
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
    run_archive_with_module_loader(bytes, options, cancellation, None)
}

/// 从归档加载本地对象，只将官方标准库请求转给宿主加载器。
pub fn run_archive_with_module_loader(
    bytes: &[u8],
    options: XarRunOptions,
    cancellation: Option<xiao_vm::CancellationSource>,
    official: Option<&dyn xiao_vm::ModuleLoader>,
) -> Result<xiao_vm::RunOutcome, XarRunError> {
    let indexed = open_indexed_archive(bytes).map_err(classify_run_error)?;
    validate_archive_context(indexed.index(), &options)?;
    let archive = indexed.verify().map_err(classify_run_error)?;
    let _language = resolve_language_locale(
        &archive.index().language_locale,
        options.language_locale.as_deref(),
    );
    let index = archive.index();
    let entry = executable_entry(index)?;
    let entry_bytes = archive
        .read_object(entry.object_kind, entry.digest)
        .map_err(classify_run_error)?;
    let loader = ArchiveModuleLoader {
        archive: &archive,
        official,
    };
    xiao_vm::run_xiaoc_production_with_features(
        &entry_bytes,
        options.vm_options,
        options.event_capacity,
        cancellation,
        options.capabilities,
        Some(&loader),
        options.runtime_features >> 1,
    )
    .map_err(|error| XarRunError::Validation(format!("入口 `.xiaoc` 执行前校验失败：{error}")))
}

/// 归档索引是项目模块的唯一来源，不读取构建机缓存或原项目目录。
#[derive(Debug)]
struct ArchiveModuleLoader<'a> {
    archive: &'a super::XarArchive,
    official: Option<&'a dyn xiao_vm::ModuleLoader>,
}

impl xiao_vm::ModuleLoader for ArchiveModuleLoader<'_> {
    fn contains(&self, identity: &str) -> bool {
        if identity == "stdlib" || identity.starts_with("stdlib.") {
            return self
                .official
                .is_some_and(|loader| loader.contains(identity));
        }
        self.archive.index().entries.iter().any(|entry| {
            entry.object_kind == ObjectKind::Xiaoc
                && (entry.module == identity || entry.module.starts_with(&format!("{identity}.")))
        })
    }

    fn compile(&self, _identity: &str) -> Result<Option<xiao_vm::CompiledModule>, String> {
        Err("归档模块通过已验证字节码加载".to_owned())
    }

    fn load(&self, identity: &str) -> Result<Option<xiao_vm::ExecutableModule>, String> {
        if identity == "stdlib" || identity.starts_with("stdlib.") {
            return self
                .official
                .ok_or("未注册官方标准库加载器")?
                .load(identity);
        }
        let Some(entry) = self
            .archive
            .index()
            .entries
            .iter()
            .find(|entry| entry.object_kind == ObjectKind::Xiaoc && entry.module == identity)
        else {
            return if self.contains(identity) {
                Ok(None)
            } else {
                Err(format!("归档缺少模块 {identity}"))
            };
        };
        let bytes = self
            .archive
            .read_object(entry.object_kind, entry.digest)
            .map_err(|error| error.to_string())?;
        let file = xiao_bytecode::decode_xiaoc(&bytes).map_err(|error| error.to_string())?;
        let instance_tables = file
            .program
            .table_definitions
            .iter()
            .filter(|table| table.signature.kind == "instance")
            .flat_map(|table| {
                [
                    format!("ascii:{}", table.signature.name),
                    format!("backtick:{}", table.signature.name),
                ]
            })
            .collect();
        Ok(Some(xiao_vm::ExecutableModule {
            program: file.program,
            source_name: entry.logical_path.clone(),
            instance_tables,
        }))
    }

    fn environment(&self, _identity: &str) -> Option<&str> {
        None
    }
}

/// 校验已解析索引与本次运行上下文；在载荷读取和诊断进程启动前使用。
pub fn validate_archive_context(
    index: &xiao_artifacts::ArchiveIndex,
    options: &XarRunOptions,
) -> Result<(), XarRunError> {
    if options.event_capacity == 0 || options.event_capacity > xiao_vm::MAX_EVENT_CAPACITY {
        return Err(XarRunError::Validation(format!(
            "事件容量必须位于 1..={}（收到 {}）",
            xiao_vm::MAX_EVENT_CAPACITY,
            options.event_capacity
        )));
    }
    validate_requirements(index, options)?;
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
    executable_entry(index)?;
    Ok(())
}

/// 在诊断启动和执行前使用同一份入口映射/种类检查。
fn executable_entry(
    index: &xiao_artifacts::ArchiveIndex,
) -> Result<&xiao_artifacts::ArchiveEntry, XarRunError> {
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
    Ok(entry)
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

impl From<XarError> for XarRunError {
    fn from(error: XarError) -> Self {
        match error {
            XarError::Artifact(ArtifactError::ArchiveVersionIncompatible(message)) => {
                Self::VersionIncompatible(message)
            }
            XarError::Artifact(ArtifactError::InvalidArchiveRequirements(message)) => {
                Self::InvalidRequirements(message)
            }
            XarError::MissingMember(path) => XarRunError::MissingObject(path),
            XarError::Artifact(ArtifactError::UnsupportedIndexVersion { major, .. }) => {
                XarRunError::VersionIncompatible(format!("不支持的归档索引主版本 {major}"))
            }
            XarError::Unsupported(message) if message.contains("版本") => {
                XarRunError::VersionIncompatible(message)
            }
            XarError::InvalidIndex(message) if message.contains("版本") => {
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
}

/// 在定位或读取入口对象前验证本次启动要求；标准库只解析，不初始化。
fn validate_requirements(
    index: &xiao_artifacts::ArchiveIndex,
    options: &XarRunOptions,
) -> Result<(), XarRunError> {
    if index.required_features & !xiao_runtime_abi::SUPPORTED_CONCURRENCY_FEATURES != 0 {
        return Err(XarRunError::InvalidRequirements(format!(
            "归档含未知 required-feature 位 {:#x}",
            index.required_features & !xiao_runtime_abi::SUPPORTED_CONCURRENCY_FEATURES
        )));
    }
    let supported_features = xiao_artifacts::ARCHIVE_REQUIRED_FEATURES
        | xiao_runtime_abi::SUPPORTED_CONCURRENCY_FEATURES;
    if options.runtime_features & !supported_features != 0 {
        return Err(XarRunError::InvalidRequirements(format!(
            "Runtime 声明未知 feature 位 {:#x}",
            options.runtime_features & !supported_features
        )));
    }
    let missing_features = index.required_features & !options.runtime_features;
    if missing_features != 0 {
        return Err(XarRunError::VersionIncompatible(format!(
            "Runtime 缺少归档要求的 feature 位 {missing_features:#x}"
        )));
    }
    validate_capability_mask(index.required_capabilities)
        .map_err(|error| XarRunError::InvalidRequirements(error.to_string()))?;
    validate_capability_mask(options.capabilities)
        .map_err(|error| XarRunError::Validation(format!("请求 capabilities 无效：{error}")))?;
    let identity = xiao_standard_library::current_standard_library_identity();
    if index.stdlib_interface_version != identity.stdlib_interface_version {
        return Err(XarRunError::VersionIncompatible(format!(
            "标准库接口版本：期望 {}，实际 {}",
            index.stdlib_interface_version, identity.stdlib_interface_version
        )));
    }
    if index.language_version != identity.language_version {
        return Err(XarRunError::VersionIncompatible(format!(
            "语言版本：期望 {}，实际 {}",
            index.language_version, identity.language_version
        )));
    }
    if !index.stdlib_modules.is_empty() {
        let installed = xiao_modules::analyze_project(xiao_standard_library::source_root());
        for module in &index.stdlib_modules {
            let exists = module
                .strip_prefix(xiao_standard_library::STANDARD_LIBRARY_PREFIX)
                .filter(|name| !name.is_empty())
                .map(|name| xiao_modules::ModuleName::new(name.split('.').map(str::to_owned)))
                .is_some_and(|name| {
                    installed.is_success() && installed.modules.contains_key(&name)
                });
            if !exists {
                return Err(XarRunError::DependencyUnsatisfied(format!(
                    "缺少可解析的标准库模块 {module}"
                )));
            }
        }
    }
    let missing = index.required_capabilities & !options.capabilities;
    if missing != 0 {
        let names = [
            (0, "filesystem"),
            (1, "clock"),
            (2, "process"),
            (3, "network"),
            (4, "native_extension"),
            (5, "stdout"),
            (6, "stdin"),
        ]
        .into_iter()
        .filter_map(|(bit, name)| (missing & (1 << bit) != 0).then_some(name))
        .collect::<Vec<_>>()
        .join(", ");
        return Err(XarRunError::CapabilityUnavailable(format!(
            "归档缺少能力：{names}"
        )));
    }
    Ok(())
}

fn classify_run_error(error: XarError) -> XarRunError {
    error.into()
}
