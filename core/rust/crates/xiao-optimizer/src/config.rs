//! 优化级别、规范化配置和可复现指纹。

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// 将优化配置编码为指纹时使用的实现版本。
pub const CONFIG_VERSION: u32 = 1;

/// 本阶段冻结的优化级别集合。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum OptimizationLevel {
    /// 基线：执行规范化和验证，不执行语义优化 Pass。
    O0,
    /// 第一档优化占位；当前没有语义 Pass。
    O1,
    /// 第二档优化占位；当前没有语义 Pass。
    O2,
    /// 第三档优化占位；当前没有语义 Pass。
    O3,
}

impl OptimizationLevel {
    /// 返回稳定的数值表示。
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::O0 => 0,
            Self::O1 => 1,
            Self::O2 => 2,
            Self::O3 => 3,
        }
    }

    /// 返回稳定的配置名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::O0 => "O0",
            Self::O1 => "O1",
            Self::O2 => "O2",
            Self::O3 => "O3",
        }
    }
}

impl TryFrom<u8> for OptimizationLevel {
    type Error = OptimizationConfigError;

    /// 将协议或内部数值转换为冻结的优化级别。
    fn try_from(level: u8) -> Result<Self, Self::Error> {
        match level {
            0 => Ok(Self::O0),
            1 => Ok(Self::O1),
            2 => Ok(Self::O2),
            3 => Ok(Self::O3),
            other => Err(OptimizationConfigError::InvalidLevel(other)),
        }
    }
}

/// 只有产物实际嵌入语言目录时才进入优化指纹的语言信息。
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct EmbeddedLocale {
    /// 规范化语言标签。
    pub tag: String,
    /// 语言目录内容摘要，而不是宿主目录路径。
    pub catalog_digest: String,
}

/// 优化配置的规范化输入对象。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OptimizationConfig {
    /// 请求的优化级别。
    pub level: OptimizationLevel,
    /// 请求的 Pass 名称；顺序由规范化过程稳定排序。
    pub pass_set: Vec<String>,
    /// 是否保留调试信息。
    pub debug_info: bool,
    /// 是否保留源码映射。
    pub source_map: bool,
    /// 是否保留 Runtime 诊断事件。
    pub diagnostic_events: bool,
    /// 可选的嵌入式语言目录摘要。
    pub embedded_locale: Option<EmbeddedLocale>,
    /// 规范化目标描述。
    pub target: String,
    /// Runtime ABI 编码版本。
    pub runtime_abi: Option<u64>,
    /// 工具链版本摘要；不包含绝对路径。
    pub toolchain: String,
    /// 项目模块图摘要。
    pub module_graph: Vec<String>,
    /// 依赖锁定图摘要。
    pub dependency_lock: Vec<String>,
    /// 已验证静态配置摘要。
    pub static_config_digest: Option<String>,
    /// 是否允许目标 CPU 特化；本阶段只记录，不执行。
    pub allow_cpu_specialization: bool,
    /// 是否允许链接时优化；本阶段只记录，不执行。
    pub allow_lto: bool,
    /// 实验性 Pass 名称；本阶段只记录，不执行。
    pub experimental_passes: Vec<String>,
}

impl OptimizationConfig {
    /// 创建默认的 O0 配置。
    #[must_use]
    pub fn baseline(target: impl Into<String>) -> Self {
        Self {
            level: OptimizationLevel::O0,
            pass_set: Vec::new(),
            debug_info: false,
            source_map: true,
            diagnostic_events: false,
            embedded_locale: None,
            target: target.into(),
            runtime_abi: None,
            toolchain: String::new(),
            module_graph: Vec::new(),
            dependency_lock: Vec::new(),
            static_config_digest: None,
            allow_cpu_specialization: false,
            allow_lto: false,
            experimental_passes: Vec::new(),
        }
    }

    /// 设置优化级别。
    #[must_use]
    pub const fn with_level(mut self, level: OptimizationLevel) -> Self {
        self.level = level;
        self
    }

    /// 设置工具链摘要。
    #[must_use]
    pub fn with_toolchain(mut self, toolchain: impl Into<String>) -> Self {
        self.toolchain = toolchain.into();
        self
    }

    /// 设置 Runtime ABI 版本。
    #[must_use]
    pub const fn with_runtime_abi(mut self, runtime_abi: u64) -> Self {
        self.runtime_abi = Some(runtime_abi);
        self
    }

    /// 设置项目模块图摘要，并由规范化过程排序去重。
    #[must_use]
    pub fn with_module_graph<I, S>(mut self, modules: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.module_graph = modules.into_iter().map(Into::into).collect();
        self
    }

    /// 设置依赖锁定图摘要，并由规范化过程排序去重。
    #[must_use]
    pub fn with_dependency_lock<I, S>(mut self, dependencies: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.dependency_lock = dependencies.into_iter().map(Into::into).collect();
        self
    }

    /// 设置已经计算好的静态配置摘要。
    #[must_use]
    pub fn with_static_config_digest(mut self, digest: impl Into<String>) -> Self {
        self.static_config_digest = Some(digest.into());
        self
    }

    /// 设置嵌入式语言目录摘要。
    #[must_use]
    pub fn with_embedded_locale(mut self, locale: EmbeddedLocale) -> Self {
        self.embedded_locale = Some(locale);
        self
    }

    /// 规范化配置中的顺序、空白和重复项。
    pub fn normalize(mut self) -> Result<Self, OptimizationConfigError> {
        if self.target.trim().is_empty() {
            return Err(OptimizationConfigError::EmptyField("target"));
        }
        self.target = self.target.trim().to_owned();
        self.toolchain = self.toolchain.trim().to_owned();
        self.static_config_digest = normalize_optional(self.static_config_digest);
        self.pass_set = normalize_list(self.pass_set);
        self.module_graph = normalize_list(self.module_graph);
        self.dependency_lock = normalize_list(self.dependency_lock);
        self.experimental_passes = normalize_list(self.experimental_passes);
        if let Some(locale) = &mut self.embedded_locale {
            locale.tag = locale.tag.trim().to_owned();
            locale.catalog_digest = locale.catalog_digest.trim().to_owned();
            if locale.tag.is_empty() || locale.catalog_digest.is_empty() {
                return Err(OptimizationConfigError::InvalidEmbeddedLocale);
            }
        }
        Ok(self)
    }

    /// 返回规范化后的可复现配置指纹。
    pub fn fingerprint(&self) -> Result<OptimizationFingerprint, OptimizationConfigError> {
        let normalized = self.clone().normalize()?;
        let encoded = serde_json::to_vec(&normalized)
            .map_err(|error| OptimizationConfigError::Serialization(error.to_string()))?;
        Ok(OptimizationFingerprint(format!(
            "xiao-opt-fnv1a64-{}",
            stable_hash(&[CONFIG_VERSION.to_string().as_bytes(), b";", &encoded])
        )))
    }
}

/// 优化配置验证错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OptimizationConfigError {
    /// 级别不在 O0–O3 范围内。
    InvalidLevel(u8),
    /// 必填字段为空。
    EmptyField(&'static str),
    /// 嵌入语言目录信息不完整。
    InvalidEmbeddedLocale,
    /// 配置无法序列化。
    Serialization(String),
}

impl Display for OptimizationConfigError {
    /// 输出开发者可读的稳定错误。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLevel(level) => {
                write!(formatter, "不支持的优化级别 {level}，必须是 0..=3")
            }
            Self::EmptyField(field) => write!(formatter, "优化配置字段 {field} 不能为空"),
            Self::InvalidEmbeddedLocale => {
                formatter.write_str("嵌入式语言目录必须同时提供标签和摘要")
            }
            Self::Serialization(message) => write!(formatter, "优化配置序列化失败：{message}"),
        }
    }
}

impl std::error::Error for OptimizationConfigError {}

/// 优化配置的稳定指纹。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OptimizationFingerprint(String);

impl OptimizationFingerprint {
    /// 返回固定格式的指纹文本。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for OptimizationFingerprint {
    /// 输出指纹文本。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    })
}

fn normalize_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn stable_hash(parts: &[&[u8]]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for part in parts {
        for byte in *part {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    format!("{hash:016x}")
}
