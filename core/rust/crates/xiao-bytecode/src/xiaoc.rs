//! 正式 `.xiaoc` 单模块产物格式。
//!
//! 这里的格式与 [`crate::encode`] 的 09R 内存编码完全分离。文件使用显式的小端
//! 字段、长度前缀和分区目录，加载器会在返回可执行程序前验证全部边界、重叠、版本、
//! 完整性和必需能力。所有表的写入顺序都由源码顺序或字典序决定，因此相同的规范
//! 输入会产生相同的文件字节。

use std::fmt::{Display, Formatter};

use crate::TAC_RUNTIME_ABI_VERSION;
use crate::encode::{EncodeOptions, EncodedProgram, OperandWidth, decode_encoded, encode};
use crate::tac::{TacConstant, TacProgram};

/// `.xiaoc` 文件魔数：`XIAOC`、回车、换行和 DOS 文件结束标记。
pub const XIAOC_MAGIC: [u8; 8] = *b"XIAOC\r\n\x1a";
/// 当前支持的格式主版本。
pub const XIAOC_FORMAT_MAJOR: u16 = 1;
/// 当前支持的格式次版本。
pub const XIAOC_FORMAT_MINOR: u16 = 0;
/// 文件头的最小长度；扩展头会使 `header_size` 大于该值。
pub const XIAOC_HEADER_MIN_SIZE: usize = 72;
/// 当前目录头的长度。
pub const XIAOC_DIRECTORY_HEADER_SIZE: usize = 8;
/// 当前目录项的最小长度。该值是当前实现的编码尺寸，不是格式契约上限。
pub const XIAOC_DIRECTORY_ENTRY_MIN_SIZE: usize = 56;
/// 当前目录编码版本。
pub const XIAOC_DIRECTORY_VERSION: u16 = 1;
/// 必需分区标志。
pub const SECTION_FLAG_REQUIRED: u32 = 1;
/// 已定义的文件标志：调试激活、目标受限和嵌入文案目录。
pub const FILE_FLAG_DEBUG_ACTIVE: u32 = 1 << 0;
/// 文件使用了显式目标约束。
pub const FILE_FLAG_PLATFORM_CONSTRAINED: u32 = 1 << 1;
/// 文件嵌入了文案目录信息。
pub const FILE_FLAG_EMBEDDED_LOCALE: u32 = 1 << 2;
/// 当前实现支持的文件标志集合。
pub const SUPPORTED_FILE_FLAGS: u32 =
    FILE_FLAG_DEBUG_ACTIVE | FILE_FLAG_PLATFORM_CONSTRAINED | FILE_FLAG_EMBEDDED_LOCALE;
/// 当前实现支持的必需功能位集合。首版不需要额外能力位。
pub const SUPPORTED_REQUIRED_FEATURES: u64 = 0;
/// 目录和正文允许的最大分区数量。
pub const MAX_SECTION_COUNT: u32 = 4096;
/// 单个分区允许的最大正文长度。
pub const MAX_SECTION_SIZE: u64 = 1 << 34;
/// 元数据和表项中长度前缀允许的最大值。
pub const MAX_TABLE_COUNT: u32 = 1 << 20;

/// 首版正式分区编号。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u32)]
pub enum XiaocSectionKind {
    /// 模块元数据。
    Metadata = 1,
    /// UTF-8 字符串表。
    Strings = 2,
    /// 类型和调用签名表。
    Types = 3,
    /// 常量池。
    Constants = 4,
    /// 外部导入表；首版可以为空。
    Imports = 5,
    /// 函数与入口表。
    Functions = 6,
    /// 经 09R 验证的指令流。
    Instructions = 7,
    /// 紧凑源码位置映射。
    SourceMap = 8,
    /// 可选的额外调试符号。
    DebugSymbols = 9,
    /// 可选的源码正文。
    Source = 10,
}

impl XiaocSectionKind {
    fn from_raw(raw: u32) -> Option<Self> {
        Some(match raw {
            1 => Self::Metadata,
            2 => Self::Strings,
            3 => Self::Types,
            4 => Self::Constants,
            5 => Self::Imports,
            6 => Self::Functions,
            7 => Self::Instructions,
            8 => Self::SourceMap,
            9 => Self::DebugSymbols,
            10 => Self::Source,
            _ => return None,
        })
    }

    /// 返回稳定的分区编号。
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    /// 返回稳定的分区名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Metadata => "metadata",
            Self::Strings => "strings",
            Self::Types => "types",
            Self::Constants => "constants",
            Self::Imports => "imports",
            Self::Functions => "functions",
            Self::Instructions => "instructions",
            Self::SourceMap => "source-map",
            Self::DebugSymbols => "debug-symbols",
            Self::Source => "source",
        }
    }
}

/// 文件中的平台约束。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum XiaocPlatform {
    /// 不含 FFI、目标 CPU 指令或平台资源的平台无关模块。
    #[default]
    Independent,
    /// 明确声明目标三元组和所需功能集合的模块。
    Constrained {
        /// 目标三元组或等价架构描述。
        target: String,
        /// 目标所需的功能集合，写入时按字典序规范化。
        features: Vec<String>,
    },
}

/// `.xiaoc` 的模块元数据。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XiaocMetadata {
    /// 稳定的逻辑模块身份，不得是绝对路径。
    pub module_id: String,
    /// 规范源码摘要；可以为空，表示源码由外部索引提供。
    pub source_digest: String,
    /// 13A 产生的规范优化配置指纹。
    pub optimization_fingerprint: String,
    /// 依赖锁摘要；为空表示无依赖。
    pub dependency_lock_digest: String,
    /// Xiao 语言版本。
    pub language_version: String,
    /// IR 版本。
    pub ir_version: u32,
    /// 虚拟机/字节码版本描述。
    pub vm_version: String,
    /// `-debug` 是否激活独立诊断窗口。
    pub debug_active: bool,
    /// 调试组件版本；调试激活时必须非空。
    pub diagnostic_component_version: String,
    /// 平台约束。
    pub platform: XiaocPlatform,
    /// 构建时嵌入的语言目录标签和摘要。
    pub embedded_locale: Option<(String, String)>,
    /// 规范化的依赖摘要列表，按字典序保存。
    pub dependencies: Vec<String>,
}

impl Default for XiaocMetadata {
    fn default() -> Self {
        Self {
            module_id: "main".to_owned(),
            source_digest: String::new(),
            optimization_fingerprint: "xiao-opt-unset".to_owned(),
            dependency_lock_digest: String::new(),
            language_version: "0.1.0".to_owned(),
            ir_version: 1,
            vm_version: "09R3".to_owned(),
            debug_active: false,
            diagnostic_component_version: String::new(),
            platform: XiaocPlatform::Independent,
            embedded_locale: None,
            dependencies: Vec::new(),
        }
    }
}

impl XiaocMetadata {
    /// 创建指定模块身份的元数据。
    #[must_use]
    pub fn new(module_id: impl Into<String>) -> Self {
        Self {
            module_id: module_id.into(),
            ..Self::default()
        }
    }

    /// 设置优化指纹。
    #[must_use]
    pub fn with_optimization_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.optimization_fingerprint = fingerprint.into();
        self
    }

    /// 设置规范源码摘要。
    #[must_use]
    pub fn with_source_digest(mut self, digest: impl Into<String>) -> Self {
        self.source_digest = digest.into();
        self
    }

    /// 设置依赖锁摘要。
    #[must_use]
    pub fn with_dependency_lock_digest(mut self, digest: impl Into<String>) -> Self {
        self.dependency_lock_digest = digest.into();
        self
    }

    /// 启用调试激活位并记录诊断组件版本。
    #[must_use]
    pub fn with_debug(mut self, component_version: impl Into<String>) -> Self {
        self.debug_active = true;
        self.diagnostic_component_version = component_version.into();
        self
    }

    /// 设置明确的目标约束。
    #[must_use]
    pub fn with_platform(mut self, target: impl Into<String>, features: Vec<String>) -> Self {
        self.platform = XiaocPlatform::Constrained {
            target: target.into(),
            features,
        };
        self
    }

    fn normalize(mut self) -> Result<Self, XiaocError> {
        self.module_id = self.module_id.trim().to_owned();
        self.source_digest = self.source_digest.trim().to_owned();
        self.optimization_fingerprint = self.optimization_fingerprint.trim().to_owned();
        self.dependency_lock_digest = self.dependency_lock_digest.trim().to_owned();
        self.language_version = self.language_version.trim().to_owned();
        self.vm_version = self.vm_version.trim().to_owned();
        if self.module_id.is_empty() {
            return Err(XiaocError::InvalidMetadata("module_id 不能为空".to_owned()));
        }
        if self.optimization_fingerprint.is_empty() {
            return Err(XiaocError::InvalidMetadata(
                "optimization_fingerprint 不能为空".to_owned(),
            ));
        }
        if self.debug_active && self.diagnostic_component_version.trim().is_empty() {
            return Err(XiaocError::InvalidMetadata(
                "debug 激活时 diagnostic_component_version 不能为空".to_owned(),
            ));
        }
        if contains_absolute_path(&self.module_id)
            || contains_absolute_path(&self.source_digest)
            || contains_absolute_path(&self.dependency_lock_digest)
            || contains_absolute_path(&self.diagnostic_component_version)
        {
            return Err(XiaocError::InvalidMetadata(
                "元数据不得包含绝对路径".to_owned(),
            ));
        }
        self.dependencies = normalize_strings(self.dependencies);
        if self
            .dependencies
            .iter()
            .any(|value| contains_absolute_path(value))
        {
            return Err(XiaocError::InvalidMetadata(
                "依赖摘要不得包含绝对路径".to_owned(),
            ));
        }
        if let Some((locale, digest)) = &mut self.embedded_locale {
            *locale = locale.trim().to_owned();
            *digest = digest.trim().to_owned();
            if locale.is_empty() || digest.is_empty() {
                return Err(XiaocError::InvalidMetadata(
                    "嵌入文案目录必须同时提供标签和摘要".to_owned(),
                ));
            }
        }
        if let XiaocPlatform::Constrained { target, features } = &mut self.platform {
            *target = target.trim().to_owned();
            if target.is_empty() || contains_absolute_path(target) {
                return Err(XiaocError::InvalidMetadata("目标三元组不能为空".to_owned()));
            }
            *features = normalize_strings(std::mem::take(features));
        }
        Ok(self)
    }
}

/// 编码选项。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XiaocOptions {
    /// 09R 内存指令流使用的操作数宽度。
    pub operand_width: OperandWidth,
    /// 允许的头部扩展字节；扩展会计入 `header_size`。
    pub header_extension: Vec<u8>,
    /// 可选的完整调试符号分区正文。
    pub debug_symbols: Option<Vec<u8>>,
    /// 可选的源码正文分区正文。
    pub source: Option<Vec<u8>>,
}

impl Default for XiaocOptions {
    fn default() -> Self {
        Self {
            operand_width: OperandWidth::Leb128,
            header_extension: Vec::new(),
            debug_symbols: None,
            source: None,
        }
    }
}

/// 文件头的结构化视图。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XiaocHeader {
    /// 格式主版本。
    pub format_major: u16,
    /// 格式次版本。
    pub format_minor: u16,
    /// 当前头部（含扩展）长度。
    pub header_size: u32,
    /// 文件标志。
    pub flags: u32,
    /// 分区数量。
    pub section_count: u32,
    /// 分区目录偏移。
    pub directory_offset: u64,
    /// 分区目录长度。
    pub directory_size: u64,
    /// 文件总长度。
    pub file_size: u64,
    /// 必需功能位集合。
    pub required_features: u64,
    /// Runtime ABI 最小版本。
    pub runtime_abi_min: u32,
    /// Runtime ABI 最大版本。
    pub runtime_abi_max: u32,
    /// 头部扩展字节。
    pub extension: Vec<u8>,
}

/// 一个分区目录项的结构化视图。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XiaocSection {
    /// 已知分区种类；未知可选分区解码时会跳过。
    pub kind: Option<XiaocSectionKind>,
    /// 原始分区编号。
    pub kind_id: u32,
    /// 必需或可选标志。
    pub flags: u32,
    /// 文件偏移。
    pub offset: u64,
    /// 文件中存储的长度。
    pub stored_size: u64,
    /// 解压后的逻辑长度；首版必须等于 `stored_size`。
    pub logical_size: u64,
    /// 正文对齐要求。
    pub alignment: u32,
    /// FNV-1a 64 位完整性摘要。
    pub checksum: u64,
    /// 分区正文版本。
    pub version: u32,
    /// 已验证的正文；未知可选分区也保留，便于检查器展示。
    pub data: Vec<u8>,
}

/// 解码后、已经通过全部格式检查的 `.xiaoc`。
#[derive(Clone, Debug)]
pub struct XiaocFile {
    /// 结构化文件头。
    pub header: XiaocHeader,
    /// 模块元数据。
    pub metadata: XiaocMetadata,
    /// 已知和未知可选分区目录。
    pub sections: Vec<XiaocSection>,
    /// 已验证的 09R 内存编码。
    pub encoded: EncodedProgram,
    /// 已验证可交给 VM 的 TAC 程序。
    pub program: TacProgram,
}

/// 格式检查器返回的轻量摘要。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XiaocInspection {
    /// 文件头。
    pub header: XiaocHeader,
    /// 模块身份。
    pub module_id: String,
    /// 优化配置指纹。
    pub optimization_fingerprint: String,
    /// 依赖锁摘要。
    pub dependency_lock_digest: String,
    /// 分区名称和长度。
    pub sections: Vec<(String, u64)>,
}

/// `.xiaoc` 编解码和验证错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XiaocError {
    /// 输入不是 `.xiaoc` 魔数。
    InvalidMagic,
    /// 主版本不受支持。
    UnsupportedMajorVersion(u16),
    /// 次版本需要尚未实现的必需扩展。
    UnsupportedMinorVersion(u16),
    /// 输入在指定字段处截断。
    UnexpectedEof(String),
    /// 字段包含非法值。
    InvalidField {
        /// 出错字段。
        field: String,
        /// 稳定的开发者原因。
        message: String,
    },
    /// 元数据不满足确定性或必填约束。
    InvalidMetadata(String),
    /// 未知必需分区。
    UnknownRequiredSection(u32),
    /// 未知必需功能位。
    UnknownRequiredFeature(u8),
    /// 分区边界、对齐或重叠错误。
    InvalidBounds(String),
    /// 分区正文摘要不匹配。
    ChecksumMismatch {
        /// 出错分区名称或编号。
        section: String,
    },
    /// 正式容器中的指令流无法通过 09R 解码验证。
    InvalidInstructionStream(String),
    /// UTF-8 正文无效。
    InvalidUtf8(String),
}

impl Display for XiaocError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic => formatter.write_str("XIAOC-001: `.xiaoc` 魔数错误"),
            Self::UnsupportedMajorVersion(version) => {
                write!(formatter, "XIAOC-002: 不支持的 `.xiaoc` 主版本 {version}")
            }
            Self::UnsupportedMinorVersion(version) => {
                write!(formatter, "XIAOC-003: 不支持的 `.xiaoc` 次版本 {version}")
            }
            Self::UnexpectedEof(field) => write!(formatter, "XIAOC-004: {field} 处截断"),
            Self::InvalidField { field, message } => {
                write!(formatter, "XIAOC-005: {field} 非法：{message}")
            }
            Self::InvalidMetadata(message) => write!(formatter, "XIAOC-006: 元数据非法：{message}"),
            Self::UnknownRequiredSection(kind) => {
                write!(formatter, "XIAOC-007: 未知必需分区 {kind}")
            }
            Self::UnknownRequiredFeature(bit) => {
                write!(formatter, "XIAOC-008: 未知必需功能位 {bit}")
            }
            Self::InvalidBounds(message) => write!(formatter, "XIAOC-009: 分区边界非法：{message}"),
            Self::ChecksumMismatch { section } => {
                write!(formatter, "XIAOC-010: 分区 {section} 完整性校验失败")
            }
            Self::InvalidInstructionStream(message) => {
                write!(formatter, "XIAOC-011: 指令流验证失败：{message}")
            }
            Self::InvalidUtf8(field) => write!(formatter, "XIAOC-012: {field} 不是 UTF-8"),
        }
    }
}

impl std::error::Error for XiaocError {}

/// 将 TAC 程序编码为默认 `.xiaoc` 文件。
pub fn encode_xiaoc(program: &TacProgram, metadata: XiaocMetadata) -> Result<Vec<u8>, XiaocError> {
    encode_xiaoc_with_options(program, metadata, XiaocOptions::default())
}

/// 使用指定选项将 TAC 程序编码为 `.xiaoc` 文件。
pub fn encode_xiaoc_with_options(
    program: &TacProgram,
    metadata: XiaocMetadata,
    options: XiaocOptions,
) -> Result<Vec<u8>, XiaocError> {
    let metadata = metadata.normalize()?;
    if contains_absolute_path(&program.abi.target) {
        return Err(XiaocError::InvalidMetadata(
            "程序目标描述不得包含绝对路径".to_owned(),
        ));
    }
    if !options.header_extension.is_empty() && options.header_extension.len() > u32::MAX as usize {
        return Err(XiaocError::InvalidField {
            field: "header_extension".to_owned(),
            message: "长度超出 u32".to_owned(),
        });
    }
    let encoded = encode(
        program,
        EncodeOptions {
            operand_width: options.operand_width,
        },
    )
    .map_err(|error| XiaocError::InvalidInstructionStream(error.to_string()))?;

    let mut sections = vec![
        (
            XiaocSectionKind::Metadata,
            SECTION_FLAG_REQUIRED,
            metadata_bytes(&metadata)?,
        ),
        (
            XiaocSectionKind::Strings,
            SECTION_FLAG_REQUIRED,
            strings_bytes(program, &metadata)?,
        ),
        (
            XiaocSectionKind::Types,
            SECTION_FLAG_REQUIRED,
            types_bytes(program)?,
        ),
        (
            XiaocSectionKind::Constants,
            SECTION_FLAG_REQUIRED,
            constants_bytes(program)?,
        ),
        (
            XiaocSectionKind::Imports,
            SECTION_FLAG_REQUIRED,
            imports_bytes()?,
        ),
        (
            XiaocSectionKind::Functions,
            SECTION_FLAG_REQUIRED,
            functions_bytes(&encoded)?,
        ),
        (
            XiaocSectionKind::Instructions,
            SECTION_FLAG_REQUIRED,
            encoded.bytes.clone(),
        ),
        (
            XiaocSectionKind::SourceMap,
            SECTION_FLAG_REQUIRED,
            source_map_bytes(&encoded)?,
        ),
    ];
    if let Some(data) = options.debug_symbols {
        sections.push((XiaocSectionKind::DebugSymbols, 0, data));
    }
    if let Some(data) = options.source {
        sections.push((XiaocSectionKind::Source, 0, data));
    }
    if sections.len() > MAX_SECTION_COUNT as usize {
        return Err(XiaocError::InvalidBounds("分区数量超限".to_owned()));
    }

    let mut flags = 0_u32;
    if metadata.debug_active {
        flags |= FILE_FLAG_DEBUG_ACTIVE;
    }
    if matches!(metadata.platform, XiaocPlatform::Constrained { .. }) {
        flags |= FILE_FLAG_PLATFORM_CONSTRAINED;
    }
    if metadata.embedded_locale.is_some() {
        flags |= FILE_FLAG_EMBEDDED_LOCALE;
    }
    let header_size = XIAOC_HEADER_MIN_SIZE
        .checked_add(options.header_extension.len())
        .ok_or_else(|| XiaocError::InvalidBounds("头部长度溢出".to_owned()))?;
    let directory_offset = align_up(header_size as u64, 8)?;
    let directory_size = (XIAOC_DIRECTORY_HEADER_SIZE as u64)
        .checked_add(
            (sections.len() as u64)
                .checked_mul(XIAOC_DIRECTORY_ENTRY_MIN_SIZE as u64)
                .ok_or_else(|| XiaocError::InvalidBounds("目录长度溢出".to_owned()))?,
        )
        .ok_or_else(|| XiaocError::InvalidBounds("目录长度溢出".to_owned()))?;
    let mut cursor = align_up(directory_offset + directory_size, 8)?;
    let mut entries = Vec::with_capacity(sections.len());
    let mut payloads = Vec::with_capacity(sections.len());
    for (kind, section_flags, data) in sections {
        let offset = align_up(cursor, 8)?;
        let size = u64::try_from(data.len())
            .map_err(|_| XiaocError::InvalidBounds("分区长度超出 u64".to_owned()))?;
        if size > MAX_SECTION_SIZE {
            return Err(XiaocError::InvalidBounds(format!(
                "分区 {} 过大",
                kind.as_str()
            )));
        }
        entries.push((kind, section_flags, offset, size, checksum(&data)));
        payloads.push((offset, data));
        cursor = offset
            .checked_add(size)
            .ok_or_else(|| XiaocError::InvalidBounds("文件长度溢出".to_owned()))?;
    }
    let file_size = cursor;
    let mut header = Vec::with_capacity(header_size);
    header.extend_from_slice(&XIAOC_MAGIC);
    put_u16(&mut header, XIAOC_FORMAT_MAJOR);
    put_u16(&mut header, XIAOC_FORMAT_MINOR);
    put_u32(&mut header, header_size as u32);
    put_u32(&mut header, flags);
    put_u32(&mut header, entries.len() as u32);
    put_u64(&mut header, directory_offset);
    put_u64(&mut header, directory_size);
    put_u64(&mut header, file_size);
    put_u64(&mut header, SUPPORTED_REQUIRED_FEATURES);
    put_u32(&mut header, TAC_RUNTIME_ABI_VERSION);
    put_u32(&mut header, TAC_RUNTIME_ABI_VERSION);
    put_u32(&mut header, 0);
    put_u32(&mut header, options.header_extension.len() as u32);
    header.extend_from_slice(&options.header_extension);

    let output_size = usize::try_from(file_size)
        .map_err(|_| XiaocError::InvalidBounds("文件长度超出宿主可寻址范围".to_owned()))?;
    let mut output = vec![0; output_size];
    output[..header.len()].copy_from_slice(&header);
    let directory_start = directory_offset as usize;
    let mut directory = Vec::with_capacity(directory_size as usize);
    put_u32(&mut directory, XIAOC_DIRECTORY_ENTRY_MIN_SIZE as u32);
    put_u16(&mut directory, XIAOC_DIRECTORY_VERSION);
    put_u16(&mut directory, 0);
    for (kind, section_flags, offset, size, digest) in &entries {
        put_u32(&mut directory, kind.as_u32());
        put_u32(&mut directory, *section_flags);
        put_u64(&mut directory, *offset);
        put_u64(&mut directory, *size);
        put_u64(&mut directory, *size);
        put_u32(&mut directory, 8);
        put_u64(&mut directory, *digest);
        put_u32(&mut directory, 1);
        put_u32(&mut directory, 0);
        put_u32(&mut directory, 0);
    }
    output[directory_start..directory_start + directory.len()].copy_from_slice(&directory);
    for (offset, data) in payloads {
        let start = offset as usize;
        output[start..start + data.len()].copy_from_slice(&data);
    }
    Ok(output)
}

/// 解码并验证 `.xiaoc`；成功返回的文件可以安全交给 VM。
pub fn decode_xiaoc(bytes: &[u8]) -> Result<XiaocFile, XiaocError> {
    let (header, entries) = parse_container(bytes)?;
    let metadata_entry = required_entry(&entries, XiaocSectionKind::Metadata)?;
    let metadata = parse_metadata(&metadata_entry.data)?;
    let instruction_entry = required_entry(&entries, XiaocSectionKind::Instructions)?;
    let encoded = decode_encoded(&instruction_entry.data)
        .map_err(|error| XiaocError::InvalidInstructionStream(error.to_string()))?;
    let program = encoded
        .decode()
        .map_err(|error| XiaocError::InvalidInstructionStream(error.to_string()))?;
    validate_table_sections(&entries, &program, &encoded)?;
    if program.abi.runtime_abi_version < header.runtime_abi_min
        || program.abi.runtime_abi_version > header.runtime_abi_max
    {
        return Err(XiaocError::InvalidField {
            field: "runtime_abi".to_owned(),
            message: format!(
                "程序 ABI {} 不在 {}..={} 区间",
                program.abi.runtime_abi_version, header.runtime_abi_min, header.runtime_abi_max
            ),
        });
    }
    if metadata.debug_active != (header.flags & FILE_FLAG_DEBUG_ACTIVE != 0) {
        return Err(XiaocError::InvalidMetadata(
            "调试激活位与文件标志不一致".to_owned(),
        ));
    }
    Ok(XiaocFile {
        header,
        metadata,
        sections: entries,
        encoded,
        program,
    })
}

/// 执行格式、目录、摘要、元数据和 09R 指令正文检查。
pub fn validate_xiaoc(bytes: &[u8]) -> Result<XiaocHeader, XiaocError> {
    Ok(decode_xiaoc(bytes)?.header)
}

/// 读取检查工具需要的来源、指纹、依赖和分区摘要。
pub fn inspect_xiaoc(bytes: &[u8]) -> Result<XiaocInspection, XiaocError> {
    let file = decode_xiaoc(bytes)?;
    let metadata = &file.metadata;
    Ok(XiaocInspection {
        header: file.header,
        module_id: metadata.module_id.clone(),
        optimization_fingerprint: metadata.optimization_fingerprint.clone(),
        dependency_lock_digest: metadata.dependency_lock_digest.clone(),
        sections: file
            .sections
            .iter()
            .map(|section| {
                (
                    section.kind.map_or_else(
                        || format!("unknown-{}", section.kind_id),
                        |kind| kind.as_str().to_owned(),
                    ),
                    section.logical_size,
                )
            })
            .collect(),
    })
}

/// 默认编码入口的别名，方便产物层按“文件编码”命名调用。
pub fn encode_file(program: &TacProgram, metadata: XiaocMetadata) -> Result<Vec<u8>, XiaocError> {
    encode_xiaoc(program, metadata)
}

/// 默认解码入口的别名。
pub fn decode_file(bytes: &[u8]) -> Result<XiaocFile, XiaocError> {
    decode_xiaoc(bytes)
}

fn parse_container(bytes: &[u8]) -> Result<(XiaocHeader, Vec<XiaocSection>), XiaocError> {
    if bytes.len() < XIAOC_HEADER_MIN_SIZE || bytes[..8] != XIAOC_MAGIC {
        return Err(XiaocError::InvalidMagic);
    }
    let mut reader = Reader::new(bytes);
    reader.skip(8, "magic")?;
    let major = reader.u16("format_major")?;
    let minor = reader.u16("format_minor")?;
    if major != XIAOC_FORMAT_MAJOR {
        return Err(XiaocError::UnsupportedMajorVersion(major));
    }
    if minor > XIAOC_FORMAT_MINOR {
        return Err(XiaocError::UnsupportedMinorVersion(minor));
    }
    let header_size = reader.u32("header_size")?;
    let flags = reader.u32("flags")?;
    let section_count = reader.u32("section_count")?;
    let directory_offset = reader.u64("directory_offset")?;
    let directory_size = reader.u64("directory_size")?;
    let file_size = reader.u64("file_size")?;
    let required_features = reader.u64("required_features")?;
    let runtime_abi_min = reader.u32("runtime_abi_min")?;
    let runtime_abi_max = reader.u32("runtime_abi_max")?;
    let reserved = reader.u32("reserved")?;
    let extension_len = reader.u32("extension_len")? as usize;
    if reserved != 0 {
        return Err(XiaocError::InvalidField {
            field: "reserved".to_owned(),
            message: "必须为零".to_owned(),
        });
    }
    if flags & !SUPPORTED_FILE_FLAGS != 0 {
        return Err(XiaocError::InvalidField {
            field: "flags".to_owned(),
            message: "含未定义位".to_owned(),
        });
    }
    if required_features != SUPPORTED_REQUIRED_FEATURES {
        let bit = required_features.trailing_zeros() as u8;
        return Err(XiaocError::UnknownRequiredFeature(bit));
    }
    if section_count > MAX_SECTION_COUNT {
        return Err(XiaocError::InvalidBounds("section_count 超限".to_owned()));
    }
    if header_size < XIAOC_HEADER_MIN_SIZE as u32
        || extension_len != header_size as usize - XIAOC_HEADER_MIN_SIZE
    {
        return Err(XiaocError::InvalidBounds(
            "header_size 与扩展长度不一致".to_owned(),
        ));
    }
    if file_size != bytes.len() as u64 {
        return Err(XiaocError::InvalidBounds(
            "file_size 与输入长度不一致".to_owned(),
        ));
    }
    if runtime_abi_min > runtime_abi_max {
        return Err(XiaocError::InvalidField {
            field: "runtime_abi".to_owned(),
            message: "最小版本大于最大版本".to_owned(),
        });
    }
    let extension = reader.bytes(extension_len, "header_extension")?.to_vec();
    if reader.position() != header_size as usize {
        return Err(XiaocError::InvalidBounds("头部边界错误".to_owned()));
    }
    let header = XiaocHeader {
        format_major: major,
        format_minor: minor,
        header_size,
        flags,
        section_count,
        directory_offset,
        directory_size,
        file_size,
        required_features,
        runtime_abi_min,
        runtime_abi_max,
        extension,
    };
    let directory_end = directory_offset
        .checked_add(directory_size)
        .ok_or_else(|| XiaocError::InvalidBounds("目录长度溢出".to_owned()))?;
    if directory_offset < header_size as u64 || directory_end > file_size {
        return Err(XiaocError::InvalidBounds(
            "目录越过文件边界或头部".to_owned(),
        ));
    }
    if directory_offset % 8 != 0 || directory_size < XIAOC_DIRECTORY_HEADER_SIZE as u64 {
        return Err(XiaocError::InvalidBounds(
            "目录未按 8 字节对齐或过短".to_owned(),
        ));
    }
    let directory_start = usize::try_from(directory_offset)
        .map_err(|_| XiaocError::InvalidBounds("目录偏移超出宿主可寻址范围".to_owned()))?;
    let directory_end_usize = usize::try_from(directory_end)
        .map_err(|_| XiaocError::InvalidBounds("目录末端超出宿主可寻址范围".to_owned()))?;
    let mut directory_reader = Reader::bounded(bytes, directory_start, directory_end_usize)?;
    let entry_size = directory_reader.u32("directory_entry_size")? as usize;
    let directory_version = directory_reader.u16("directory_version")?;
    let directory_reserved = directory_reader.u16("directory_reserved")?;
    if directory_version != XIAOC_DIRECTORY_VERSION {
        return Err(XiaocError::InvalidField {
            field: "directory_version".to_owned(),
            message: format!("不支持 {directory_version}"),
        });
    }
    if directory_reserved != 0 || entry_size < XIAOC_DIRECTORY_ENTRY_MIN_SIZE {
        return Err(XiaocError::InvalidField {
            field: "directory_entry_size".to_owned(),
            message: "目录项过短或保留字段非零".to_owned(),
        });
    }
    let expected_directory = XIAOC_DIRECTORY_HEADER_SIZE
        .checked_add(
            entry_size
                .checked_mul(section_count as usize)
                .ok_or_else(|| XiaocError::InvalidBounds("目录长度溢出".to_owned()))?,
        )
        .ok_or_else(|| XiaocError::InvalidBounds("目录长度溢出".to_owned()))?;
    if expected_directory != directory_size as usize {
        return Err(XiaocError::InvalidBounds(
            "directory_size 与项长度不一致".to_owned(),
        ));
    }
    let mut sections = Vec::with_capacity(section_count as usize);
    let mut previous_end = directory_end;
    let mut known_kinds = Vec::new();
    for index in 0..section_count {
        let kind_id = directory_reader.u32("section_kind")?;
        let section_flags = directory_reader.u32("section_flags")?;
        let offset = directory_reader.u64("section_offset")?;
        let stored_size = directory_reader.u64("stored_size")?;
        let logical_size = directory_reader.u64("logical_size")?;
        let alignment = directory_reader.u32("alignment")?;
        let digest = directory_reader.u64("checksum")?;
        let version = directory_reader.u32("section_version")?;
        let reserved = directory_reader.u32("section_reserved")?;
        let entry_reserved = directory_reader.u32("directory_entry_reserved")?;
        if entry_size > XIAOC_DIRECTORY_ENTRY_MIN_SIZE {
            directory_reader.skip(
                entry_size - XIAOC_DIRECTORY_ENTRY_MIN_SIZE,
                "directory_extension",
            )?;
        }
        if section_flags & !SECTION_FLAG_REQUIRED != 0 {
            return Err(XiaocError::InvalidField {
                field: format!("section[{index}].flags"),
                message: "含未定义位".to_owned(),
            });
        }
        if reserved != 0 || entry_reserved != 0 || stored_size != logical_size {
            return Err(XiaocError::InvalidField {
                field: format!("section[{index}]"),
                message: "保留字段非零或 stored_size != logical_size".to_owned(),
            });
        }
        if alignment == 0
            || !alignment.is_power_of_two()
            || alignment > 4096
            || offset % u64::from(alignment) != 0
        {
            return Err(XiaocError::InvalidBounds(format!(
                "section[{index}] 对齐非法"
            )));
        }
        let end = offset
            .checked_add(stored_size)
            .ok_or_else(|| XiaocError::InvalidBounds(format!("section[{index}] 长度溢出")))?;
        if offset < header_size as u64 || offset < directory_end || end > file_size {
            return Err(XiaocError::InvalidBounds(format!("section[{index}] 越界")));
        }
        if offset < previous_end {
            return Err(XiaocError::InvalidBounds(
                "分区重叠或未按 offset 递增".to_owned(),
            ));
        }
        if stored_size > MAX_SECTION_SIZE {
            return Err(XiaocError::InvalidBounds(format!("section[{index}] 过大")));
        }
        let section_start = usize::try_from(offset).map_err(|_| {
            XiaocError::InvalidBounds(format!("section[{index}] 偏移超出宿主可寻址范围"))
        })?;
        let section_end = usize::try_from(end).map_err(|_| {
            XiaocError::InvalidBounds(format!("section[{index}] 末端超出宿主可寻址范围"))
        })?;
        let data = bytes[section_start..section_end].to_vec();
        if checksum(&data) != digest {
            return Err(XiaocError::ChecksumMismatch {
                section: kind_id.to_string(),
            });
        }
        let kind = XiaocSectionKind::from_raw(kind_id);
        if kind.is_none() && section_flags & SECTION_FLAG_REQUIRED != 0 {
            return Err(XiaocError::UnknownRequiredSection(kind_id));
        }
        if let Some(kind) = kind {
            if section_flags & SECTION_FLAG_REQUIRED != 0 && version != 1 {
                return Err(XiaocError::InvalidField {
                    field: format!("section[{index}].version"),
                    message: format!("{} 分区版本 {version} 不受支持", kind.as_str()),
                });
            }
            if known_kinds.contains(&kind) {
                return Err(XiaocError::InvalidField {
                    field: format!("section[{index}].kind"),
                    message: "同一种已知分区重复出现".to_owned(),
                });
            }
            known_kinds.push(kind);
        }
        sections.push(XiaocSection {
            kind,
            kind_id,
            flags: section_flags,
            offset,
            stored_size,
            logical_size,
            alignment,
            checksum: digest,
            version,
            data,
        });
        previous_end = end;
    }
    if directory_reader.position() != directory_end as usize {
        return Err(XiaocError::InvalidBounds("目录没有完整消费".to_owned()));
    }
    for required in [
        XiaocSectionKind::Metadata,
        XiaocSectionKind::Strings,
        XiaocSectionKind::Types,
        XiaocSectionKind::Constants,
        XiaocSectionKind::Imports,
        XiaocSectionKind::Functions,
        XiaocSectionKind::Instructions,
        XiaocSectionKind::SourceMap,
    ] {
        required_entry(&sections, required)?;
    }
    Ok((header, sections))
}

fn required_entry(
    entries: &[XiaocSection],
    kind: XiaocSectionKind,
) -> Result<&XiaocSection, XiaocError> {
    entries
        .iter()
        .find(|entry| entry.kind == Some(kind) && entry.flags & SECTION_FLAG_REQUIRED != 0)
        .ok_or_else(|| XiaocError::InvalidBounds(format!("缺少必需分区 {}", kind.as_str())))
}

fn validate_table_sections(
    entries: &[XiaocSection],
    program: &TacProgram,
    encoded: &EncodedProgram,
) -> Result<(), XiaocError> {
    validate_strings(required_entry(entries, XiaocSectionKind::Strings)?)?;
    validate_types(required_entry(entries, XiaocSectionKind::Types)?)?;
    let functions = required_entry(entries, XiaocSectionKind::Functions)?;
    validate_functions(functions, program, encoded)?;
    let constants = required_entry(entries, XiaocSectionKind::Constants)?;
    validate_constants(constants, program)?;
    let imports = required_entry(entries, XiaocSectionKind::Imports)?;
    validate_zero_table(imports, "imports")?;
    if encoded.functions.len() != program.functions.len() {
        return Err(XiaocError::InvalidInstructionStream(
            "函数目录数量不一致".to_owned(),
        ));
    }
    let source_map = required_entry(entries, XiaocSectionKind::SourceMap)?;
    validate_source_map(source_map, encoded)?;
    Ok(())
}

fn validate_strings(section: &XiaocSection) -> Result<(), XiaocError> {
    let mut reader = Reader::new(&section.data);
    let values = reader.strings("strings")?;
    if reader.remaining() != 0 || values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(XiaocError::InvalidField {
            field: "strings".to_owned(),
            message: "字符串表未规范排序或有尾部".to_owned(),
        });
    }
    Ok(())
}

fn validate_types(section: &XiaocSection) -> Result<(), XiaocError> {
    let mut reader = Reader::new(&section.data);
    let count = reader.u32("types")?;
    if count > MAX_TABLE_COUNT {
        return Err(XiaocError::InvalidBounds("types 数量超限".to_owned()));
    }
    for _ in 0..count {
        let _ = reader.u32("type.parameter_count")?;
        let _ = reader.u32("type.var_args")?;
        let _ = reader.u32("type.kw_args")?;
    }
    if reader.remaining() != 0 {
        return Err(XiaocError::InvalidField {
            field: "types".to_owned(),
            message: "尾部有未消费字节".to_owned(),
        });
    }
    Ok(())
}

fn validate_zero_table(section: &XiaocSection, field: &str) -> Result<(), XiaocError> {
    let mut reader = Reader::new(&section.data);
    let count = reader.u32(field)?;
    if count != 0 || reader.remaining() != 0 {
        return Err(XiaocError::InvalidField {
            field: field.to_owned(),
            message: "首版必须为空".to_owned(),
        });
    }
    Ok(())
}

fn validate_constants(section: &XiaocSection, program: &TacProgram) -> Result<(), XiaocError> {
    let mut reader = Reader::new(&section.data);
    let count = reader.u32("constants")?;
    if count as usize != program.constants.len() {
        return Err(XiaocError::InvalidField {
            field: "constants".to_owned(),
            message: "常量表数量不一致".to_owned(),
        });
    }
    for _ in 0..count {
        match reader.byte("constant.tag")? {
            0 => {
                reader.skip(8, "constant.int")?;
            }
            1 => {
                reader.skip(4, "constant.sint")?;
            }
            2 | 5 | 7 => {
                let _ = reader.string("constant.text")?;
            }
            3 => {
                reader.skip(8, "constant.float")?;
            }
            4 => {
                reader.skip(4, "constant.sfloat")?;
            }
            6 => {
                let _ = reader.bool("constant.bool")?;
            }
            tag => {
                return Err(XiaocError::InvalidField {
                    field: "constant.tag".to_owned(),
                    message: format!("未知标签 {tag}"),
                });
            }
        }
    }
    if reader.remaining() != 0 {
        return Err(XiaocError::InvalidField {
            field: "constants".to_owned(),
            message: "尾部有未消费字节".to_owned(),
        });
    }
    Ok(())
}

fn validate_functions(
    section: &XiaocSection,
    program: &TacProgram,
    encoded: &EncodedProgram,
) -> Result<(), XiaocError> {
    let mut reader = Reader::new(&section.data);
    let count = reader.u32("functions")?;
    if count as usize != program.functions.len() || count as usize != encoded.functions.len() {
        return Err(XiaocError::InvalidField {
            field: "functions".to_owned(),
            message: "函数表数量不一致".to_owned(),
        });
    }
    for (index, expected) in encoded.functions.iter().enumerate() {
        if reader.string("function.name")? != expected.name {
            return Err(XiaocError::InvalidField {
                field: format!("functions[{index}].name"),
                message: "函数名不一致".to_owned(),
            });
        }
        let block_count = reader.u32("function.blocks")?;
        let code_len = reader.u32("function.code_len")?;
        if block_count as usize != expected.blocks.len() || code_len != expected.code_len {
            return Err(XiaocError::InvalidField {
                field: format!("functions[{index}]"),
                message: "函数目录不一致".to_owned(),
            });
        }
        for block in &expected.blocks {
            if reader.u32("block.id")? != block.id.get()
                || reader.u32("block.pc")? != block.pc
                || reader.u32("block.size")? != block.bytes.len() as u32
                || reader.u32("block.instructions")? != block.instruction_pcs.len() as u32
            {
                return Err(XiaocError::InvalidField {
                    field: format!("functions[{index}].block"),
                    message: "基本块目录不一致".to_owned(),
                });
            }
        }
    }
    if reader.remaining() != 0 {
        return Err(XiaocError::InvalidField {
            field: "functions".to_owned(),
            message: "尾部有未消费字节".to_owned(),
        });
    }
    Ok(())
}

fn validate_source_map(section: &XiaocSection, encoded: &EncodedProgram) -> Result<(), XiaocError> {
    let mut reader = Reader::new(&section.data);
    let mut expected_count = 0_u32;
    for function in &encoded.functions {
        for block in &function.blocks {
            expected_count = expected_count.saturating_add(block.spans.len() as u32);
        }
    }
    let count = reader.u32("source_map.count")?;
    if count != expected_count {
        return Err(XiaocError::InvalidField {
            field: "source_map.count".to_owned(),
            message: "映射数量不一致".to_owned(),
        });
    }
    for _ in 0..count {
        reader.skip(4 + 4 + 4 + 8 + 8, "source_map.entry")?;
    }
    if reader.remaining() != 0 {
        return Err(XiaocError::InvalidField {
            field: "source_map".to_owned(),
            message: "尾部有未消费字节".to_owned(),
        });
    }
    Ok(())
}

fn metadata_bytes(metadata: &XiaocMetadata) -> Result<Vec<u8>, XiaocError> {
    let mut out = Vec::new();
    put_u16(&mut out, 1);
    put_u16(&mut out, 0);
    put_string(&mut out, &metadata.module_id)?;
    put_string(&mut out, &metadata.source_digest)?;
    put_string(&mut out, &metadata.optimization_fingerprint)?;
    put_string(&mut out, &metadata.dependency_lock_digest)?;
    put_string(&mut out, &metadata.language_version)?;
    put_u32(&mut out, metadata.ir_version);
    put_string(&mut out, &metadata.vm_version)?;
    out.push(u8::from(metadata.debug_active));
    put_string(&mut out, &metadata.diagnostic_component_version)?;
    match &metadata.platform {
        XiaocPlatform::Independent => out.push(0),
        XiaocPlatform::Constrained { target, features } => {
            out.push(1);
            put_string(&mut out, target)?;
            put_strings(&mut out, features)?;
        }
    }
    match &metadata.embedded_locale {
        None => out.push(0),
        Some((locale, digest)) => {
            out.push(1);
            put_string(&mut out, locale)?;
            put_string(&mut out, digest)?;
        }
    }
    put_strings(&mut out, &metadata.dependencies)?;
    Ok(out)
}

fn parse_metadata(bytes: &[u8]) -> Result<XiaocMetadata, XiaocError> {
    let mut reader = Reader::new(bytes);
    let version = reader.u16("metadata_version")?;
    let reserved = reader.u16("metadata_reserved")?;
    if version != 1 || reserved != 0 {
        return Err(XiaocError::InvalidMetadata(
            "元数据版本或保留字段错误".to_owned(),
        ));
    }
    let module_id = reader.string("module_id")?;
    let source_digest = reader.string("source_digest")?;
    let optimization_fingerprint = reader.string("optimization_fingerprint")?;
    let dependency_lock_digest = reader.string("dependency_lock_digest")?;
    let language_version = reader.string("language_version")?;
    let ir_version = reader.u32("ir_version")?;
    let vm_version = reader.string("vm_version")?;
    let debug_active = reader.bool("debug_active")?;
    let diagnostic_component_version = reader.string("diagnostic_component_version")?;
    let platform_tag = reader.byte("platform")?;
    let platform = match platform_tag {
        0 => XiaocPlatform::Independent,
        1 => XiaocPlatform::Constrained {
            target: reader.string("target")?,
            features: reader.strings("features")?,
        },
        _ => return Err(XiaocError::InvalidMetadata("未知 platform 标签".to_owned())),
    };
    let embedded_locale = match reader.byte("embedded_locale")? {
        0 => None,
        1 => Some((reader.string("locale")?, reader.string("catalog_digest")?)),
        _ => {
            return Err(XiaocError::InvalidMetadata(
                "未知 embedded_locale 标签".to_owned(),
            ));
        }
    };
    let dependencies = reader.strings("dependencies")?;
    if reader.remaining() != 0 {
        return Err(XiaocError::InvalidMetadata(
            "元数据尾部有未消费字节".to_owned(),
        ));
    }
    XiaocMetadata {
        module_id,
        source_digest,
        optimization_fingerprint,
        dependency_lock_digest,
        language_version,
        ir_version,
        vm_version,
        debug_active,
        diagnostic_component_version,
        platform,
        embedded_locale,
        dependencies,
    }
    .normalize()
}

fn strings_bytes(program: &TacProgram, metadata: &XiaocMetadata) -> Result<Vec<u8>, XiaocError> {
    let mut strings = vec![
        metadata.module_id.clone(),
        metadata.optimization_fingerprint.clone(),
    ];
    strings.extend(
        program
            .functions
            .iter()
            .map(|function| function.name.clone()),
    );
    for constant in program.constants.iter() {
        match constant {
            TacConstant::Lint(value) | TacConstant::Lfloat(value) | TacConstant::Str(value) => {
                strings.push(value.clone())
            }
            _ => {}
        }
    }
    strings = normalize_strings(strings);
    let mut out = Vec::new();
    put_strings(&mut out, &strings)?;
    Ok(out)
}

fn types_bytes(program: &TacProgram) -> Result<Vec<u8>, XiaocError> {
    let mut out = Vec::new();
    put_u32(&mut out, program.signatures.len() as u32);
    for signature in program.signatures.iter() {
        put_u32(&mut out, signature.parameter_types.len() as u32);
        put_u32(&mut out, u32::from(signature.var_args_slot.is_some()));
        put_u32(&mut out, u32::from(signature.kw_args_slot.is_some()));
    }
    Ok(out)
}

fn constants_bytes(program: &TacProgram) -> Result<Vec<u8>, XiaocError> {
    let mut out = Vec::new();
    put_u32(&mut out, program.constants.len() as u32);
    for constant in program.constants.iter() {
        match constant {
            TacConstant::Int(value) => {
                out.push(0);
                out.extend_from_slice(&value.to_le_bytes());
            }
            TacConstant::Sint(value) => {
                out.push(1);
                out.extend_from_slice(&value.to_le_bytes());
            }
            TacConstant::Lint(value) => {
                out.push(2);
                put_string(&mut out, value)?;
            }
            TacConstant::Float(value) => {
                out.push(3);
                out.extend_from_slice(&value.to_bits().to_le_bytes());
            }
            TacConstant::Sfloat(value) => {
                out.push(4);
                out.extend_from_slice(&value.to_bits().to_le_bytes());
            }
            TacConstant::Lfloat(value) => {
                out.push(5);
                put_string(&mut out, value)?;
            }
            TacConstant::Bool(value) => {
                out.push(6);
                out.push(u8::from(*value));
            }
            TacConstant::Str(value) => {
                out.push(7);
                put_string(&mut out, value)?;
            }
        }
    }
    Ok(out)
}

fn imports_bytes() -> Result<Vec<u8>, XiaocError> {
    Ok(0_u32.to_le_bytes().to_vec())
}

fn functions_bytes(encoded: &EncodedProgram) -> Result<Vec<u8>, XiaocError> {
    let mut out = Vec::new();
    put_u32(&mut out, encoded.functions.len() as u32);
    for function in &encoded.functions {
        put_string(&mut out, &function.name)?;
        put_u32(&mut out, function.blocks.len() as u32);
        put_u32(&mut out, function.code_len);
        for block in &function.blocks {
            put_u32(&mut out, block.id.get());
            put_u32(&mut out, block.pc);
            put_u32(&mut out, block.bytes.len() as u32);
            put_u32(&mut out, block.instruction_pcs.len() as u32);
        }
    }
    Ok(out)
}

fn source_map_bytes(encoded: &EncodedProgram) -> Result<Vec<u8>, XiaocError> {
    let mut out = Vec::new();
    let mut count = 0_u32;
    for function in &encoded.functions {
        for block in &function.blocks {
            count = count.saturating_add(block.spans.len() as u32);
        }
    }
    put_u32(&mut out, count);
    for (function_id, function) in encoded.functions.iter().enumerate() {
        for block in &function.blocks {
            for (index, span) in block.spans.iter().enumerate() {
                put_u32(&mut out, function_id as u32);
                put_u32(&mut out, block.id.get());
                put_u32(&mut out, block.instruction_pcs[index]);
                put_u64(&mut out, span.start as u64);
                put_u64(&mut out, span.end as u64);
            }
        }
    }
    Ok(out)
}

fn put_string(out: &mut Vec<u8>, value: &str) -> Result<(), XiaocError> {
    let bytes = value.as_bytes();
    if bytes.len() > u32::MAX as usize {
        return Err(XiaocError::InvalidBounds("字符串长度超出 u32".to_owned()));
    }
    put_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
    Ok(())
}

fn put_strings(out: &mut Vec<u8>, values: &[String]) -> Result<(), XiaocError> {
    if values.len() > MAX_TABLE_COUNT as usize {
        return Err(XiaocError::InvalidBounds("字符串表过大".to_owned()));
    }
    put_u32(out, values.len() as u32);
    for value in values {
        put_string(out, value)?;
    }
    Ok(())
}

fn normalize_strings(mut values: Vec<String>) -> Vec<String> {
    values
        .iter_mut()
        .for_each(|value| *value = value.trim().to_owned());
    values.retain(|value| !value.is_empty());
    values.sort();
    values.dedup();
    values
}

fn contains_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.starts_with('/') || value.starts_with('\\') || (bytes.len() >= 2 && bytes[1] == b':')
}

fn align_up(value: u64, alignment: u64) -> Result<u64, XiaocError> {
    let mask = alignment - 1;
    value
        .checked_add(mask)
        .map(|value| value & !mask)
        .ok_or_else(|| XiaocError::InvalidBounds("对齐溢出".to_owned()))
}

fn checksum(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
    limit: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            limit: bytes.len(),
        }
    }
    fn bounded(bytes: &'a [u8], start: usize, end: usize) -> Result<Self, XiaocError> {
        if start > end || end > bytes.len() {
            return Err(XiaocError::InvalidBounds("读取范围越界".to_owned()));
        }
        Ok(Self {
            bytes,
            position: start,
            limit: end,
        })
    }
    fn position(&self) -> usize {
        self.position
    }
    fn remaining(&self) -> usize {
        self.limit.saturating_sub(self.position)
    }
    fn skip(&mut self, length: usize, field: &str) -> Result<(), XiaocError> {
        self.take(length, field).map(|_| ())
    }
    fn take(&mut self, length: usize, field: &str) -> Result<&'a [u8], XiaocError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| XiaocError::InvalidBounds(format!("{field} 长度溢出")))?;
        if end > self.limit {
            return Err(XiaocError::UnexpectedEof(field.to_owned()));
        }
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }
    fn bytes(&mut self, length: usize, field: &str) -> Result<&'a [u8], XiaocError> {
        self.take(length, field)
    }
    fn byte(&mut self, field: &str) -> Result<u8, XiaocError> {
        Ok(self.take(1, field)?[0])
    }
    fn u16(&mut self, field: &str) -> Result<u16, XiaocError> {
        Ok(u16::from_le_bytes(self.take(2, field)?.try_into().unwrap()))
    }
    fn u32(&mut self, field: &str) -> Result<u32, XiaocError> {
        Ok(u32::from_le_bytes(self.take(4, field)?.try_into().unwrap()))
    }
    fn u64(&mut self, field: &str) -> Result<u64, XiaocError> {
        Ok(u64::from_le_bytes(self.take(8, field)?.try_into().unwrap()))
    }
    fn bool(&mut self, field: &str) -> Result<bool, XiaocError> {
        match self.byte(field)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(XiaocError::InvalidField {
                field: field.to_owned(),
                message: "布尔值必须为 0 或 1".to_owned(),
            }),
        }
    }
    fn string(&mut self, field: &str) -> Result<String, XiaocError> {
        let length = self.u32(field)? as usize;
        let bytes = self.take(length, field)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| XiaocError::InvalidUtf8(field.to_owned()))
    }
    fn strings(&mut self, field: &str) -> Result<Vec<String>, XiaocError> {
        let count = self.u32(field)?;
        if count > MAX_TABLE_COUNT {
            return Err(XiaocError::InvalidBounds(format!("{field} 数量超限")));
        }
        (0..count)
            .map(|index| self.string(&format!("{field}[{index}]")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TAC_BYTECODE_ABI_VERSION;
    use crate::tac::{TacAbi, TacProgram};

    fn empty_program() -> TacProgram {
        TacProgram {
            version: crate::TAC_VERSION,
            abi: TacAbi {
                bytecode_abi_version: TAC_BYTECODE_ABI_VERSION,
                runtime_abi_version: TAC_RUNTIME_ABI_VERSION,
                ir_version: 1,
                language_version: "0.1.0".to_owned(),
                target: "portable".to_owned(),
            },
            constants: Default::default(),
            signatures: Default::default(),
            functions: Vec::new(),
            categories: Default::default(),
            plans: Vec::new(),
            selection_plans: Vec::new(),
            broadcast_assignment_plans: Vec::new(),
            random_seed_plans: Vec::new(),
            table_definitions: Vec::new(),
            unsupported: Vec::new(),
        }
    }

    #[test]
    fn round_trip_is_deterministic_and_variable_length_header_is_supported() {
        let program = empty_program();
        let metadata = XiaocMetadata::new("demo").with_debug("diag-1");
        let options = XiaocOptions {
            header_extension: vec![1, 2, 3],
            ..Default::default()
        };
        let first = encode_xiaoc_with_options(&program, metadata.clone(), options.clone()).unwrap();
        let second = encode_xiaoc_with_options(&program, metadata, options).unwrap();
        assert_eq!(first, second);
        let file = decode_xiaoc(&first).unwrap();
        assert_eq!(file.header.header_size as usize, XIAOC_HEADER_MIN_SIZE + 3);
        assert!(file.metadata.debug_active);
    }

    #[test]
    fn rejects_truncation_overlap_unknown_required_and_compression() {
        let program = empty_program();
        let bytes = encode_xiaoc(&program, XiaocMetadata::default()).unwrap();
        for length in 0..bytes.len() {
            assert!(decode_xiaoc(&bytes[..length]).is_err());
        }
        let mut changed = bytes.clone();
        changed[68] = 1;
        assert!(decode_xiaoc(&changed).is_err());
        let mut compressed = bytes;
        let directory = u64::from_le_bytes(compressed[32..40].try_into().unwrap()) as usize;
        compressed[directory + 8 + 16..directory + 8 + 24].copy_from_slice(&1_u64.to_le_bytes());
        assert!(decode_xiaoc(&compressed).is_err());
    }

    #[test]
    fn malformed_random_bytes_never_panic() {
        let program = empty_program();
        let bytes = encode_xiaoc(&program, XiaocMetadata::default()).unwrap();
        for seed in 0..256_u16 {
            let mut candidate = bytes.clone();
            let index = usize::from(seed) % candidate.len();
            candidate[index] ^= (seed as u8).wrapping_mul(31).wrapping_add(1);
            let _ = decode_xiaoc(&candidate);
        }
    }
}
