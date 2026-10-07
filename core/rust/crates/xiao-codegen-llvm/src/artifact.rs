//! 链接后原生产物的对象格式、符号和依赖检查。
//!
//! 验证器不调用 `nm`、`dumpbin` 或宿主工具链，也不搜索 `PATH`。它只消费调用方已经
//! 选定的目标描述，并读取最终文件的对象格式元数据，为 Runtime 裁剪和调试启动提供
//! 产物层事实。

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::error::{CodegenError, Result};
use crate::target::{Endian, ObjectFormat, TargetDescription};

/// ELF 文件头的魔数。
const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
/// 大端 Mach-O fat 文件的 32 位魔数。
const MACHO_FAT_MAGIC: u32 = 0xcafebabe;
/// 大端 Mach-O fat 文件的 64 位魔数。
const MACHO_FAT_MAGIC_64: u32 = 0xcafebabf;
/// 大端 Mach-O 32 位 thin 文件的魔数。
const MACHO_MAGIC_32: u32 = 0xfeedface;
/// 大端 Mach-O 64 位 thin 文件的魔数。
const MACHO_MAGIC_64: u32 = 0xfeedfacf;
/// 小端 Mach-O 32 位 thin 文件的魔数。
const MACHO_CIGAM_32: u32 = 0xcefaedfe;
/// 小端 Mach-O 64 位 thin 文件的魔数。
const MACHO_CIGAM_64: u32 = 0xcffaedfe;
/// PE 文件的签名。
const PE_SIGNATURE: &[u8; 4] = b"PE\0\0";

/// Runtime 导出或符号的统一前缀。
const RUNTIME_SYMBOL_PREFIX: &str = "xiao_runtime_";
/// 调试构建启动入口的符号名。
const DEBUG_START_SYMBOL: &str = "xiao_native_debug_start";
/// Runtime 诊断钩子的统一前缀。
const DIAGNOSTIC_SYMBOL_PREFIX: &str = "xiao_runtime_diagnostic_";
/// 只有这些符号表示调试启动激活位；普通错误报告也会链接 `diagnostic_event`。
const DEBUG_ACTIVATION_SYMBOLS: &[&str] = &[
    DEBUG_START_SYMBOL,
    "xiao_runtime_diagnostic_prepare",
    "xiao_runtime_diagnostic_ready",
    "xiao_runtime_diagnostic_finish",
];

/// 产物中可观察到的对象格式事实。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactInspection {
    /// 解析时使用的对象格式。
    pub object_format: ObjectFormat,
    /// 从符号表、导出表和导入表收集的符号名。
    pub symbols: Vec<String>,
    /// 按产物符号表原始顺序读取的符号；PE 导出/导入观测可能为空。
    pub symbol_order: Vec<String>,
    /// 是否读取到了真实对象格式符号表。
    pub symbol_table_readable: bool,
    /// 从动态依赖或 PE 导入表收集的库名。
    pub dependencies: Vec<String>,
}

impl ArtifactInspection {
    /// 判断对象格式中是否存在给定的规范符号名。
    #[must_use]
    pub fn has_symbol(&self, symbol: &str) -> bool {
        self.symbols.iter().any(|item| item == symbol)
    }

    /// 返回排序后的符号顺序，用于跨链接器的确定性比较。
    #[must_use]
    pub fn normalized_symbol_order(&self) -> Vec<String> {
        let mut symbols = self.symbol_order.clone();
        symbols.sort();
        symbols
    }
}

/// 链接器符号表的可验证状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolTableStatus {
    /// 读取到了真实对象格式符号表。
    Readable,
    /// 当前格式只能观察导出/导入表，不能证明内部符号。
    Unavailable,
}

/// 两份符号表报告的可验证比较结果。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolTableComparison {
    /// 两份可读符号表的规范化顺序一致。
    Identical,
    /// 两份可读符号表的对象格式或规范化顺序不同。
    Different,
    /// 至少一份符号表不可读，不能据此判定一致。
    Unavailable,
}

/// 独立产物验收模式。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactAcceptanceMode {
    /// 普通发布产物。
    Release,
    /// 带独立诊断启动能力的调试产物。
    Debug,
    /// 已请求剥离符号的产物。
    Stripped,
}

/// strip/调试/诊断的独立产物验收结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactModeReport {
    /// 验收模式。
    pub mode: ArtifactAcceptanceMode,
    /// 符号表状态。
    pub symbol_table: SymbolTableStatus,
    /// 是否观察到调试激活符号。
    pub diagnostic_symbols_present: bool,
    /// 是否通过当前模式的独立边界。
    pub accepted: bool,
    /// 未通过或明确不可验证时的稳定说明。
    pub diagnostic: Option<String>,
}

/// 符号表规范化报告。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolTableReport {
    /// 对象格式。
    pub object_format: ObjectFormat,
    /// 符号表状态。
    pub status: SymbolTableStatus,
    /// 原始符号顺序。
    pub original_order: Vec<String>,
    /// 规范化排序后的顺序。
    pub normalized_order: Vec<String>,
}

/// 读取链接后产物的符号表并建立规范化报告。
pub fn inspect_symbol_table(
    path: impl AsRef<Path>,
    target: &TargetDescription,
) -> Result<SymbolTableReport> {
    let inspection = inspect_artifact(path, target)?;
    Ok(SymbolTableReport {
        object_format: inspection.object_format,
        status: if inspection.symbol_table_readable {
            SymbolTableStatus::Readable
        } else {
            SymbolTableStatus::Unavailable
        },
        original_order: inspection.symbol_order.clone(),
        normalized_order: inspection.normalized_symbol_order(),
    })
}

/// 比较两份链接后符号表报告。
///
/// 只有两份报告都明确读取到了真实符号表时才会返回 `Identical` 或 `Different`；
/// 任一侧不可读都返回 `Unavailable`，避免把“看不见”当成“顺序一致”。
#[must_use]
pub fn compare_symbol_table_reports(
    first: &SymbolTableReport,
    second: &SymbolTableReport,
) -> SymbolTableComparison {
    if first.status != SymbolTableStatus::Readable || second.status != SymbolTableStatus::Readable {
        return SymbolTableComparison::Unavailable;
    }
    if first.object_format == second.object_format
        && first.normalized_order == second.normalized_order
    {
        SymbolTableComparison::Identical
    } else {
        SymbolTableComparison::Different
    }
}

/// 对真实产物执行独立的 release/debug/stripped 模式检查。
///
/// `Stripped` 模式遇到不可读符号表时返回 `accepted = true` 并保留明确诊断，
/// 表示“剥离状态符合请求，但符号层不可再验证”；任何其他解析失败仍返回错误。
pub fn verify_artifact_mode(
    path: impl AsRef<Path>,
    target: &TargetDescription,
    mode: ArtifactAcceptanceMode,
    declared_components: &[String],
) -> Result<ArtifactModeReport> {
    let path = path.as_ref();
    match mode {
        ArtifactAcceptanceMode::Release | ArtifactAcceptanceMode::Debug => {
            let composition = verify_artifact(
                path,
                target,
                declared_components,
                mode == ArtifactAcceptanceMode::Debug,
            )?;
            let symbol_table = if composition.verification == ArtifactVerification::Complete {
                SymbolTableStatus::Readable
            } else {
                SymbolTableStatus::Unavailable
            };
            Ok(ArtifactModeReport {
                mode,
                symbol_table,
                diagnostic_symbols_present: !composition.diagnostic_symbols.is_empty(),
                accepted: true,
                diagnostic: (symbol_table == SymbolTableStatus::Unavailable).then(|| {
                    "产物符号表不可读；Runtime 组成仅作不可验证观察，不能证明内部裁剪".to_owned()
                }),
            })
        }
        ArtifactAcceptanceMode::Stripped => match inspect_artifact(path, target) {
            Ok(inspection) => Ok(ArtifactModeReport {
                mode,
                symbol_table: if inspection.symbol_table_readable {
                    SymbolTableStatus::Readable
                } else {
                    SymbolTableStatus::Unavailable
                },
                diagnostic_symbols_present: inspection
                    .symbols
                    .iter()
                    .any(|symbol| DEBUG_ACTIVATION_SYMBOLS.contains(&symbol.as_str())),
                accepted: !inspection.symbol_table_readable,
                diagnostic: Some(if inspection.symbol_table_readable {
                    "请求 stripped，但产物仍保留可读符号表".to_owned()
                } else {
                    "产物符号不可读；strip 状态明确，但符号层不再可验证".to_owned()
                }),
            }),
            Err(error) => {
                let message = error.to_string();
                if message.contains("缺少可验证的符号表") {
                    Ok(ArtifactModeReport {
                        mode,
                        symbol_table: SymbolTableStatus::Unavailable,
                        diagnostic_symbols_present: false,
                        accepted: true,
                        diagnostic: Some(message),
                    })
                } else {
                    Err(error)
                }
            }
        },
    }
}

/// Runtime 裁剪验证后可供构建诊断消费的事实摘要。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactRuntimeComposition {
    /// 最终产物的对象格式。
    pub object_format: ObjectFormat,
    /// IR 层登记的 Runtime 组件。
    pub declared_components: Vec<String>,
    /// 产物符号表中实际观察到的 Runtime 组件。
    pub observed_components: Vec<String>,
    /// 产物符号表中观察到的 Runtime 符号。
    pub runtime_symbols: Vec<String>,
    /// 未能映射到已知 Runtime 组件的符号；用于保留未来 ABI 的事实。
    pub unclassified_runtime_symbols: Vec<String>,
    /// 产物声明的外部库依赖。
    pub dependencies: Vec<String>,
    /// 与调试启动或诊断钩子有关的实际符号。
    pub diagnostic_symbols: Vec<String>,
    /// 产物组件观察的可信度；没有可读 COFF 表时，导出表由链接参数主动写入，不能证明内部节已裁剪。
    pub verification: ArtifactVerification,
}

/// 产物 Runtime 组成观察的可信度。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactVerification {
    /// ELF/Mach-O 或实际存在的 COFF 符号表提供了链接后事实。
    Complete,
    /// COFF 没有可读的内部符号表，只能观察由链接参数主动写入的导出表。
    UnverifiedCoffExports,
}

impl ArtifactVerification {
    /// 返回协议和诊断使用的稳定标识。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::UnverifiedCoffExports => "unverified-coff-exports",
        }
    }
}

/// 读取并解析指定目标的链接后产物。
pub fn inspect_artifact(
    path: impl AsRef<Path>,
    target: &TargetDescription,
) -> Result<ArtifactInspection> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| CodegenError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    parse_artifact(path, &bytes, target)
}

/// 验证 Runtime 组件的链接后证据和调试启动符号。
pub fn verify_artifact(
    path: impl AsRef<Path>,
    target: &TargetDescription,
    declared_components: &[String],
    debug_enabled: bool,
) -> Result<ArtifactRuntimeComposition> {
    let path = path.as_ref();
    let inspection = inspect_artifact(path, target)?;
    let declared_components = normalized_components(declared_components);
    let runtime_symbols = inspection
        .symbols
        .iter()
        .filter(|symbol| symbol.starts_with(RUNTIME_SYMBOL_PREFIX))
        .cloned()
        .collect::<Vec<_>>();
    let observed_components = components_for_symbols(&runtime_symbols);
    let unclassified_symbols = unclassified_runtime_symbols(&runtime_symbols);
    let runtime_dependencies = inspection
        .dependencies
        .iter()
        .filter(|dependency| is_runtime_dependency(dependency))
        .cloned()
        .collect::<Vec<_>>();
    let verification =
        if inspection.object_format == ObjectFormat::Coff && !inspection.symbol_table_readable {
            ArtifactVerification::UnverifiedCoffExports
        } else {
            ArtifactVerification::Complete
        };

    if declared_components.is_empty() && !runtime_symbols.is_empty() {
        return Err(artifact_error(
            path,
            format!(
                "纯静态模块的产物包含 Runtime 符号：{}",
                runtime_symbols.join(", ")
            ),
        ));
    }
    if declared_components.is_empty() && !runtime_dependencies.is_empty() {
        return Err(artifact_error(
            path,
            format!(
                "纯静态模块的产物包含 Runtime 动态依赖：{}",
                runtime_dependencies.join(", ")
            ),
        ));
    }
    if verification == ArtifactVerification::Complete {
        let missing = declared_components
            .iter()
            .filter(|component| !observed_components.iter().any(|item| item == *component))
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(artifact_error(
                path,
                format!(
                    "产物未观察到 IR 层登记的 Runtime 组件：{}",
                    missing.join(", ")
                ),
            ));
        }
        let unexpected = observed_components
            .iter()
            .filter(|component| !declared_components.iter().any(|item| item == *component))
            .cloned()
            .collect::<Vec<_>>();
        if !unexpected.is_empty() {
            return Err(artifact_error(
                path,
                format!(
                    "产物观察到未由 IR 登记的 Runtime 组件：{}",
                    unexpected.join(", ")
                ),
            ));
        }
    }

    let diagnostic_symbols = inspection
        .symbols
        .iter()
        .filter(|symbol| DEBUG_ACTIVATION_SYMBOLS.contains(&symbol.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let has_debug_start = diagnostic_symbols
        .iter()
        .any(|symbol| symbol == DEBUG_START_SYMBOL);
    if debug_enabled && !has_debug_start {
        return Err(artifact_error(
            path,
            "调试构建产物缺少 xiao_native_debug_start 符号".to_owned(),
        ));
    }
    if !debug_enabled && !diagnostic_symbols.is_empty() {
        return Err(artifact_error(
            path,
            format!(
                "普通构建产物包含调试诊断符号：{}",
                diagnostic_symbols.join(", ")
            ),
        ));
    }

    Ok(ArtifactRuntimeComposition {
        object_format: inspection.object_format,
        declared_components,
        observed_components,
        runtime_symbols,
        unclassified_runtime_symbols: unclassified_symbols,
        dependencies: inspection.dependencies,
        diagnostic_symbols,
        verification,
    })
}

/// 按目标描述选择对象格式解析器并验证格式元数据。
fn parse_artifact(
    path: &Path,
    bytes: &[u8],
    target: &TargetDescription,
) -> Result<ArtifactInspection> {
    target.validate()?;
    match target.object_format {
        ObjectFormat::Coff => parse_pe(path, bytes, target),
        ObjectFormat::Elf => parse_elf(path, bytes, target),
        ObjectFormat::MachO => parse_macho(path, bytes, target),
    }
}

/// 解析 ELF 的符号表和动态依赖，并检查目标架构、位宽与字节序。
fn parse_elf(path: &Path, bytes: &[u8], target: &TargetDescription) -> Result<ArtifactInspection> {
    if bytes.get(..4) != Some(ELF_MAGIC) {
        return Err(artifact_error(path, "文件不是 ELF 产物".to_owned()));
    }
    let class = byte_at(bytes, 4, path, "ELF class")?;
    let data = byte_at(bytes, 5, path, "ELF data encoding")?;
    let is_64 = match class {
        1 => false,
        2 => true,
        value => return Err(artifact_error(path, format!("不支持的 ELF class：{value}"))),
    };
    if (is_64 && target.pointer_width != 64) || (!is_64 && target.pointer_width != 32) {
        return Err(artifact_error(
            path,
            format!(
                "ELF 位宽与目标描述不一致：class={class}、pointer_width={}",
                target.pointer_width
            ),
        ));
    }
    let endian = match data {
        1 => Endian::Little,
        2 => Endian::Big,
        value => {
            return Err(artifact_error(
                path,
                format!("不支持的 ELF 字节序：{value}"),
            ));
        }
    };
    if endian != target.endian {
        return Err(artifact_error(
            path,
            "ELF 字节序与目标描述不一致".to_owned(),
        ));
    }
    let machine = read_u16(bytes, 18, endian, path, "ELF machine")?;
    let expected_machine = match target.triple.split('-').next().unwrap_or_default() {
        "x86_64" => Some(0x003e),
        "aarch64" | "arm64" => Some(0x00b7),
        "i386" | "i686" => Some(0x0003),
        "arm" | "armv7" => Some(0x0028),
        _ => None,
    };
    if let Some(expected_machine) = expected_machine
        && machine != expected_machine
    {
        return Err(artifact_error(
            path,
            format!(
                "ELF machine 与目标描述不一致：machine=0x{machine:04x}、expected=0x{expected_machine:04x}"
            ),
        ));
    }

    let (section_offset, section_entry_size, section_count) = if is_64 {
        (
            read_u64(bytes, 40, endian, path, "ELF section offset")?,
            read_u16(bytes, 58, endian, path, "ELF section entry size")? as u64,
            read_u16(bytes, 60, endian, path, "ELF section count")? as u64,
        )
    } else {
        (
            read_u32(bytes, 32, endian, path, "ELF section offset")? as u64,
            read_u16(bytes, 46, endian, path, "ELF section entry size")? as u64,
            read_u16(bytes, 48, endian, path, "ELF section count")? as u64,
        )
    };
    let minimum_section_size = if is_64 { 64 } else { 40 };
    if section_entry_size < minimum_section_size as u64 {
        return Err(artifact_error(
            path,
            "ELF section entry size 无效".to_owned(),
        ));
    }
    let section_bytes = checked_range(
        bytes,
        section_offset,
        section_entry_size
            .checked_mul(section_count)
            .ok_or_else(|| artifact_error(path, "ELF section 表长度溢出".to_owned()))?,
        path,
        "ELF section 表",
    )?;
    let mut sections = Vec::new();
    for index in 0..section_count as usize {
        let start = index
            .checked_mul(section_entry_size as usize)
            .ok_or_else(|| artifact_error(path, "ELF section 索引溢出".to_owned()))?;
        let entry = &section_bytes[start..start + section_entry_size as usize];
        let (section_type, offset, size, link, entry_size) = if is_64 {
            (
                read_u32(entry, 4, endian, path, "ELF section type")?,
                read_u64(entry, 24, endian, path, "ELF section offset")?,
                read_u64(entry, 32, endian, path, "ELF section size")?,
                read_u32(entry, 40, endian, path, "ELF section link")? as u64,
                read_u64(entry, 56, endian, path, "ELF symbol entry size")?,
            )
        } else {
            (
                read_u32(entry, 4, endian, path, "ELF section type")?,
                read_u32(entry, 16, endian, path, "ELF section offset")? as u64,
                read_u32(entry, 20, endian, path, "ELF section size")? as u64,
                read_u32(entry, 24, endian, path, "ELF section link")? as u64,
                read_u32(entry, 36, endian, path, "ELF symbol entry size")? as u64,
            )
        };
        if size > 0 {
            let _ = checked_range(bytes, offset, size, path, "ELF section")?;
        }
        sections.push(ElfSection {
            section_type,
            offset,
            size,
            link,
            entry_size,
        });
    }

    let mut symbols = BTreeSet::new();
    let mut symbol_order = Vec::new();
    let mut dependencies = BTreeSet::new();
    let mut usable_symbol_table = false;
    for section in &sections {
        if section.section_type != 2 && section.section_type != 11 {
            continue;
        }
        let string_table = sections
            .get(section.link as usize)
            .ok_or_else(|| artifact_error(path, "ELF 符号表的字符串表索引无效".to_owned()))?;
        let strings = checked_range(
            bytes,
            string_table.offset,
            string_table.size,
            path,
            "ELF 字符串表",
        )?;
        if section.section_type == 2 && section.size > 0 && !strings.is_empty() {
            usable_symbol_table = true;
        }
        let default_entry_size = if is_64 { 24 } else { 16 };
        let entry_size = if section.entry_size == 0 {
            default_entry_size
        } else {
            section.entry_size
        };
        if entry_size < default_entry_size {
            return Err(artifact_error(
                path,
                "ELF 符号表 entry size 无效".to_owned(),
            ));
        }
        let count = section.size / entry_size;
        for index in 0..count {
            let entry_offset = section
                .offset
                .checked_add(
                    index
                        .checked_mul(entry_size)
                        .ok_or_else(|| artifact_error(path, "ELF 符号表索引溢出".to_owned()))?,
                )
                .ok_or_else(|| artifact_error(path, "ELF 符号表偏移溢出".to_owned()))?;
            let entry = checked_range(bytes, entry_offset, entry_size, path, "ELF 符号表项")?;
            let name_offset = read_u32(entry, 0, endian, path, "ELF symbol name")? as u64;
            if name_offset == 0 {
                continue;
            }
            let name = c_string(strings, name_offset, path, "ELF symbol name")?;
            if !name.is_empty() {
                let name = normalize_symbol(name);
                symbols.insert(name.clone());
                symbol_order.push(name);
            }
        }
    }
    if !usable_symbol_table {
        return Err(artifact_error(
            path,
            "ELF 产物缺少可验证的符号表；被剥离的产物不能静默通过".to_owned(),
        ));
    }
    for section in &sections {
        if section.section_type != 6 {
            continue;
        }
        let string_table = sections.get(section.link as usize).ok_or_else(|| {
            artifact_error(path, "ELF dynamic section 的字符串表索引无效".to_owned())
        })?;
        let strings = checked_range(
            bytes,
            string_table.offset,
            string_table.size,
            path,
            "ELF 动态字符串表",
        )?;
        let entry_size = if section.entry_size == 0 {
            if is_64 { 16 } else { 8 }
        } else {
            section.entry_size
        };
        let count = section.size / entry_size;
        for index in 0..count {
            let entry_offset = section
                .offset
                .checked_add(
                    index
                        .checked_mul(entry_size)
                        .ok_or_else(|| artifact_error(path, "ELF dynamic 索引溢出".to_owned()))?,
                )
                .ok_or_else(|| artifact_error(path, "ELF dynamic 偏移溢出".to_owned()))?;
            let entry = checked_range(bytes, entry_offset, entry_size, path, "ELF dynamic 项")?;
            let (tag, value) = if is_64 {
                (
                    read_u64(entry, 0, endian, path, "ELF dynamic tag")?,
                    read_u64(entry, 8, endian, path, "ELF dynamic value")?,
                )
            } else {
                (
                    read_u32(entry, 0, endian, path, "ELF dynamic tag")? as u64,
                    read_u32(entry, 4, endian, path, "ELF dynamic value")? as u64,
                )
            };
            if tag == 0 {
                break;
            }
            if tag == 1 {
                let name = c_string(strings, value, path, "ELF dependency")?;
                if !name.is_empty() {
                    dependencies.insert(name.to_owned());
                }
            }
        }
    }
    Ok(ArtifactInspection {
        object_format: ObjectFormat::Elf,
        symbols: symbols.into_iter().collect(),
        symbol_order,
        symbol_table_readable: true,
        dependencies: dependencies.into_iter().collect(),
    })
}

#[derive(Clone, Copy)]
/// ELF section 表中供符号和动态段解析使用的字段。
struct ElfSection {
    section_type: u32,
    offset: u64,
    size: u64,
    link: u64,
    entry_size: u64,
}

/// 解析 Mach-O thin 或 fat 文件，并提取符号表和动态库依赖。
fn parse_macho(
    path: &Path,
    bytes: &[u8],
    target: &TargetDescription,
) -> Result<ArtifactInspection> {
    let magic = read_be_u32(bytes, 0).or_else(|| read_le_u32(bytes, 0));
    let (slice, endian) = match magic {
        Some(MACHO_FAT_MAGIC) | Some(0xbebafeca) => macho_fat_slice(path, bytes, target, false)?,
        Some(MACHO_FAT_MAGIC_64) | Some(0xbfbafeca) => macho_fat_slice(path, bytes, target, true)?,
        Some(_) => {
            let endian = macho_thin_endian(path, bytes)?;
            (bytes, endian)
        }
        None => return Err(artifact_error(path, "文件不是 Mach-O 产物".to_owned())),
    };
    parse_macho_thin(path, slice, target, endian)
}

/// 从 Mach-O fat 文件中选择与目标架构匹配的 slice。
fn macho_fat_slice<'a>(
    path: &Path,
    bytes: &'a [u8],
    target: &TargetDescription,
    is_64: bool,
) -> Result<(&'a [u8], Endian)> {
    let endian = if read_be_u32(bytes, 0)
        == Some(if is_64 {
            MACHO_FAT_MAGIC_64
        } else {
            MACHO_FAT_MAGIC
        }) {
        Endian::Big
    } else {
        Endian::Little
    };
    let count = read_u32(bytes, 4, endian, path, "Mach-O fat 架构数量")?;
    let expected_cpu = macho_cpu_type(target);
    let entry_size = if is_64 { 32_u64 } else { 20_u64 };
    for index in 0..count as u64 {
        let offset = 8_u64
            .checked_add(
                index
                    .checked_mul(entry_size)
                    .ok_or_else(|| artifact_error(path, "Mach-O fat 索引溢出".to_owned()))?,
            )
            .ok_or_else(|| artifact_error(path, "Mach-O fat 偏移溢出".to_owned()))?;
        let entry = checked_range(bytes, offset, entry_size, path, "Mach-O fat 架构项")?;
        let cpu = read_u32(entry, 0, endian, path, "Mach-O CPU 类型")?;
        if let Some(expected_cpu) = expected_cpu
            && cpu != expected_cpu
        {
            continue;
        }
        let slice_offset = if is_64 {
            read_u64(entry, 8, endian, path, "Mach-O fat slice 偏移")?
        } else {
            read_u32(entry, 8, endian, path, "Mach-O fat slice 偏移")? as u64
        };
        let slice_size = if is_64 {
            read_u64(entry, 16, endian, path, "Mach-O fat slice 长度")?
        } else {
            read_u32(entry, 12, endian, path, "Mach-O fat slice 长度")? as u64
        };
        let slice = checked_range(bytes, slice_offset, slice_size, path, "Mach-O fat slice")?;
        return Ok((slice, macho_thin_endian(path, slice)?));
    }
    Err(artifact_error(
        path,
        "Mach-O fat 产物没有目标架构切片".to_owned(),
    ))
}

/// 根据 Mach-O thin 文件头判断其字节序。
fn macho_thin_endian(path: &Path, bytes: &[u8]) -> Result<Endian> {
    match read_be_u32(bytes, 0).or_else(|| read_le_u32(bytes, 0)) {
        Some(MACHO_MAGIC_32 | MACHO_MAGIC_64) => Ok(Endian::Big),
        Some(MACHO_CIGAM_32 | MACHO_CIGAM_64) => Ok(Endian::Little),
        _ => Err(artifact_error(path, "文件不是 Mach-O 产物".to_owned())),
    }
}

/// 解析一个 Mach-O thin slice 的 load commands 和符号表。
fn parse_macho_thin(
    path: &Path,
    bytes: &[u8],
    target: &TargetDescription,
    endian: Endian,
) -> Result<ArtifactInspection> {
    let magic = read_u32(bytes, 0, endian, path, "Mach-O magic")?;
    let cpu = read_u32(bytes, 4, endian, path, "Mach-O CPU 类型")?;
    let is_64 = matches!(magic, MACHO_MAGIC_64 | MACHO_CIGAM_64);
    let is_32 = matches!(magic, MACHO_MAGIC_32 | MACHO_CIGAM_32);
    if !is_64 && !is_32 {
        return Err(artifact_error(path, "文件不是 Mach-O 产物".to_owned()));
    }
    if (is_64 && target.pointer_width != 64) || (is_32 && target.pointer_width != 32) {
        return Err(artifact_error(
            path,
            "Mach-O 位宽与目标描述不一致".to_owned(),
        ));
    }
    if endian != target.endian {
        return Err(artifact_error(
            path,
            "Mach-O 字节序与目标描述不一致".to_owned(),
        ));
    }
    let expected_cpu = macho_cpu_type(target);
    if let Some(expected_cpu) = expected_cpu
        && cpu != expected_cpu
    {
        return Err(artifact_error(
            path,
            format!(
                "Mach-O CPU 类型与目标描述不一致：cpu=0x{cpu:08x}、expected=0x{expected_cpu:08x}"
            ),
        ));
    }
    let command_count = read_u32(bytes, 16, endian, path, "Mach-O load command 数量")?;
    let command_bytes = read_u32(bytes, 20, endian, path, "Mach-O load command 长度")? as u64;
    let commands_offset = if is_64 { 32_u64 } else { 28_u64 };
    let commands = checked_range(
        bytes,
        commands_offset,
        command_bytes,
        path,
        "Mach-O load commands",
    )?;
    let mut symbols = BTreeSet::new();
    let mut symbol_order = Vec::new();
    let mut dependencies = BTreeSet::new();
    let mut symbol_table = None;
    let mut cursor = 0_u64;
    for _ in 0..command_count {
        let command = checked_range(commands, cursor, 8, path, "Mach-O load command")?;
        let command_kind = read_u32(command, 0, endian, path, "Mach-O command")? & 0x7fff_ffff;
        let command_size = read_u32(command, 4, endian, path, "Mach-O command size")? as u64;
        if command_size < 8 {
            return Err(artifact_error(path, "Mach-O command size 无效".to_owned()));
        }
        let full_command = checked_range(commands, cursor, command_size, path, "Mach-O command")?;
        match command_kind {
            0x2 => {
                symbol_table = Some((
                    read_u32(full_command, 8, endian, path, "Mach-O symbol offset")? as u64,
                    read_u32(full_command, 12, endian, path, "Mach-O symbol count")? as u64,
                    read_u32(full_command, 16, endian, path, "Mach-O string offset")? as u64,
                    read_u32(full_command, 20, endian, path, "Mach-O string size")? as u64,
                ));
            }
            0xc | 0x18 | 0x1f | 0x23 => {
                let name_offset =
                    read_u32(full_command, 8, endian, path, "Mach-O dylib name")? as u64;
                let name = c_string(full_command, name_offset, path, "Mach-O dylib name")?;
                if !name.is_empty() {
                    dependencies.insert(name.to_owned());
                }
            }
            _ => {}
        }
        cursor = cursor
            .checked_add(command_size)
            .ok_or_else(|| artifact_error(path, "Mach-O command 偏移溢出".to_owned()))?;
    }
    let mut usable_symbol_table = false;
    if let Some((symbol_offset, symbol_count, string_offset, string_size)) = symbol_table {
        let strings = checked_range(bytes, string_offset, string_size, path, "Mach-O 字符串表")?;
        usable_symbol_table = symbol_count > 0 && !strings.is_empty();
        let entry_size = if is_64 { 16_u64 } else { 12_u64 };
        for index in 0..symbol_count {
            let offset = symbol_offset
                .checked_add(
                    index
                        .checked_mul(entry_size)
                        .ok_or_else(|| artifact_error(path, "Mach-O symbol 索引溢出".to_owned()))?,
                )
                .ok_or_else(|| artifact_error(path, "Mach-O symbol 偏移溢出".to_owned()))?;
            let entry = checked_range(bytes, offset, entry_size, path, "Mach-O symbol")?;
            let name_offset = read_u32(entry, 0, endian, path, "Mach-O symbol name")? as u64;
            if name_offset == 0 {
                continue;
            }
            let name = c_string(strings, name_offset, path, "Mach-O symbol name")?;
            if !name.is_empty() {
                let name = normalize_symbol(name);
                symbols.insert(name.clone());
                symbol_order.push(name);
            }
        }
    }
    if !usable_symbol_table {
        return Err(artifact_error(
            path,
            "Mach-O 产物缺少可验证的符号表；被剥离的产物不能静默通过".to_owned(),
        ));
    }
    Ok(ArtifactInspection {
        object_format: ObjectFormat::MachO,
        symbols: symbols.into_iter().collect(),
        symbol_order,
        symbol_table_readable: true,
        dependencies: dependencies.into_iter().collect(),
    })
}

/// 解析 PE/COFF 符号、导入和导出表，并检查目标架构与位宽。
fn parse_pe(path: &Path, bytes: &[u8], target: &TargetDescription) -> Result<ArtifactInspection> {
    if bytes.get(..2) != Some(b"MZ") {
        return Err(artifact_error(path, "文件不是 PE/COFF 产物".to_owned()));
    }
    if target.endian != Endian::Little {
        return Err(artifact_error(path, "PE/COFF 只支持小端目标".to_owned()));
    }
    let pe_offset = read_u32(bytes, 0x3c, Endian::Little, path, "PE header offset")? as u64;
    if checked_range(bytes, pe_offset, 4, path, "PE signature")? != PE_SIGNATURE {
        return Err(artifact_error(path, "PE signature 无效".to_owned()));
    }
    let coff_offset = pe_offset + 4;
    let machine = read_u16(bytes, coff_offset, Endian::Little, path, "COFF machine")?;
    let expected_machine = match target.triple.split('-').next().unwrap_or_default() {
        "x86_64" => Some(0x8664),
        "aarch64" | "arm64" => Some(0xaa64),
        "i386" | "i686" => Some(0x014c),
        _ => None,
    };
    if let Some(expected_machine) = expected_machine
        && machine != expected_machine
    {
        return Err(artifact_error(
            path,
            format!(
                "PE machine 与目标描述不一致：machine=0x{machine:04x}、expected=0x{expected_machine:04x}"
            ),
        ));
    }
    let section_count = read_u16(
        bytes,
        coff_offset + 2,
        Endian::Little,
        path,
        "COFF section count",
    )? as u64;
    let symbol_table_offset = read_u32(
        bytes,
        coff_offset + 8,
        Endian::Little,
        path,
        "COFF symbol table",
    )? as u64;
    let symbol_count = read_u32(
        bytes,
        coff_offset + 12,
        Endian::Little,
        path,
        "COFF symbol count",
    )? as u64;
    let optional_size = read_u16(
        bytes,
        coff_offset + 16,
        Endian::Little,
        path,
        "PE optional header size",
    )? as u64;
    let optional_offset = coff_offset + 20;
    let optional = checked_range(
        bytes,
        optional_offset,
        optional_size,
        path,
        "PE optional header",
    )?;
    let optional_magic = read_u16(
        optional,
        0,
        Endian::Little,
        path,
        "PE optional header magic",
    )?;
    let (is_64, directory_count_offset, directories_offset) = match optional_magic {
        0x10b => (false, 92_u64, 96_u64),
        0x20b => (true, 108_u64, 112_u64),
        value => {
            return Err(artifact_error(
                path,
                format!("不支持的 PE optional header：0x{value:x}"),
            ));
        }
    };
    if (is_64 && target.pointer_width != 64) || (!is_64 && target.pointer_width != 32) {
        return Err(artifact_error(path, "PE 位宽与目标描述不一致".to_owned()));
    }
    let directory_count = read_u32(
        optional,
        directory_count_offset,
        Endian::Little,
        path,
        "PE data directory count",
    )? as u64;
    let header_size = read_u32(optional, 60, Endian::Little, path, "PE size of headers")? as u64;
    let section_offset = optional_offset
        .checked_add(optional_size)
        .ok_or_else(|| artifact_error(path, "PE section 偏移溢出".to_owned()))?;
    let mut sections = Vec::new();
    for index in 0..section_count {
        let offset = section_offset
            .checked_add(
                index
                    .checked_mul(40)
                    .ok_or_else(|| artifact_error(path, "PE section 索引溢出".to_owned()))?,
            )
            .ok_or_else(|| artifact_error(path, "PE section 偏移溢出".to_owned()))?;
        let section = checked_range(bytes, offset, 40, path, "PE section")?;
        sections.push(PeSection {
            virtual_address: read_u32(section, 12, Endian::Little, path, "PE section RVA")?,
            raw_offset: read_u32(section, 20, Endian::Little, path, "PE section raw offset")?,
            raw_size: read_u32(section, 16, Endian::Little, path, "PE section raw size")?,
        });
    }
    let mut symbols = BTreeSet::new();
    let mut symbol_order = Vec::new();
    let mut dependencies = BTreeSet::new();
    let mut symbol_table_readable = false;
    if symbol_table_offset != 0 && symbol_count != 0 {
        let table = checked_range(
            bytes,
            symbol_table_offset,
            symbol_count
                .checked_mul(18)
                .ok_or_else(|| artifact_error(path, "COFF symbol 表长度溢出".to_owned()))?,
            path,
            "COFF symbol 表",
        )?;
        let string_offset = symbol_table_offset
            .checked_add(
                symbol_count
                    .checked_mul(18)
                    .ok_or_else(|| artifact_error(path, "COFF 字符串表偏移溢出".to_owned()))?,
            )
            .ok_or_else(|| artifact_error(path, "COFF 字符串表偏移溢出".to_owned()))?;
        let string_size = read_u32(
            bytes,
            string_offset,
            Endian::Little,
            path,
            "COFF 字符串表长度",
        )? as u64;
        let strings = checked_range(bytes, string_offset, string_size, path, "COFF 字符串表")?;
        symbol_table_readable = !strings.is_empty();
        let mut index = 0_u64;
        while index < symbol_count {
            let start = (index as usize) * 18;
            let entry = &table[start..start + 18];
            let name = coff_symbol_name(entry, strings, path)?;
            if !name.is_empty() {
                let name = normalize_symbol(name);
                symbols.insert(name.clone());
                symbol_order.push(name);
            }
            let auxiliary = entry[17] as u64;
            index = index
                .checked_add(1 + auxiliary)
                .ok_or_else(|| artifact_error(path, "COFF auxiliary symbol 索引溢出".to_owned()))?;
        }
    }
    let directory = |index: u64| -> Result<Option<(u64, u64)>> {
        if index >= directory_count {
            return Ok(None);
        }
        let offset = directories_offset
            .checked_add(
                index
                    .checked_mul(8)
                    .ok_or_else(|| artifact_error(path, "PE data directory 索引溢出".to_owned()))?,
            )
            .ok_or_else(|| artifact_error(path, "PE data directory 偏移溢出".to_owned()))?;
        if offset + 8 > optional.len() as u64 {
            return Err(artifact_error(
                path,
                "PE data directory 超出 optional header".to_owned(),
            ));
        }
        Ok(Some((
            read_u32(optional, offset, Endian::Little, path, "PE directory RVA")? as u64,
            read_u32(
                optional,
                offset + 4,
                Endian::Little,
                path,
                "PE directory size",
            )? as u64,
        )))
    };
    if let Some((rva, _size)) = directory(0)? {
        if rva != 0 {
            let export = checked_range(
                bytes,
                rva_to_offset(rva, &sections, header_size, path)?,
                40,
                path,
                "PE export directory",
            )?;
            let names_count =
                read_u32(export, 24, Endian::Little, path, "PE export name count")? as u64;
            let names_rva = read_u32(export, 32, Endian::Little, path, "PE export names")? as u64;
            let names = checked_range(
                bytes,
                rva_to_offset(names_rva, &sections, header_size, path)?,
                names_count
                    .checked_mul(4)
                    .ok_or_else(|| artifact_error(path, "PE export names 长度溢出".to_owned()))?,
                path,
                "PE export names",
            )?;
            for index in 0..names_count as usize {
                let name_rva = read_u32(
                    names,
                    (index * 4) as u64,
                    Endian::Little,
                    path,
                    "PE export name RVA",
                )? as u64;
                let name = c_string_at_rva(
                    bytes,
                    name_rva,
                    &sections,
                    header_size,
                    path,
                    "PE export name",
                )?;
                if !name.is_empty() {
                    let name = normalize_symbol(name);
                    symbols.insert(name.clone());
                }
            }
        }
    }
    if let Some((rva, _size)) = directory(1)? {
        if rva != 0 {
            let import_offset = rva_to_offset(rva, &sections, header_size, path)?;
            let mut descriptor_index = 0_u64;
            loop {
                let offset = import_offset
                    .checked_add(
                        descriptor_index
                            .checked_mul(20)
                            .ok_or_else(|| artifact_error(path, "PE import 索引溢出".to_owned()))?,
                    )
                    .ok_or_else(|| artifact_error(path, "PE import 偏移溢出".to_owned()))?;
                let descriptor = checked_range(bytes, offset, 20, path, "PE import descriptor")?;
                let original_thunk =
                    read_u32(descriptor, 0, Endian::Little, path, "PE import thunk")? as u64;
                let name_rva =
                    read_u32(descriptor, 12, Endian::Little, path, "PE import name")? as u64;
                let first_thunk = read_u32(
                    descriptor,
                    16,
                    Endian::Little,
                    path,
                    "PE import first thunk",
                )? as u64;
                if original_thunk == 0 && name_rva == 0 && first_thunk == 0 {
                    break;
                }
                let library = c_string_at_rva(
                    bytes,
                    name_rva,
                    &sections,
                    header_size,
                    path,
                    "PE import library",
                )?;
                if !library.is_empty() {
                    dependencies.insert(library.to_owned());
                }
                let thunk_rva = if original_thunk == 0 {
                    first_thunk
                } else {
                    original_thunk
                };
                let pointer_size = if is_64 { 8_u64 } else { 4_u64 };
                let ordinal_mask = if is_64 {
                    0x8000_0000_0000_0000_u64
                } else {
                    0x8000_0000_u64
                };
                if thunk_rva != 0 {
                    let mut thunk_index = 0_u64;
                    loop {
                        let thunk_entry_rva = thunk_rva
                            .checked_add(thunk_index.checked_mul(pointer_size).ok_or_else(
                                || artifact_error(path, "PE thunk 索引溢出".to_owned()),
                            )?)
                            .ok_or_else(|| artifact_error(path, "PE thunk 偏移溢出".to_owned()))?;
                        let thunk_offset =
                            rva_to_offset(thunk_entry_rva, &sections, header_size, path)?;
                        let value = if is_64 {
                            read_u64(bytes, thunk_offset, Endian::Little, path, "PE thunk value")?
                        } else {
                            read_u32(bytes, thunk_offset, Endian::Little, path, "PE thunk value")?
                                as u64
                        };
                        if value == 0 {
                            break;
                        }
                        if value & ordinal_mask == 0 {
                            let name_rva = value.checked_add(2).ok_or_else(|| {
                                artifact_error(path, "PE import symbol 偏移溢出".to_owned())
                            })?;
                            let name = c_string_at_rva(
                                bytes,
                                name_rva,
                                &sections,
                                header_size,
                                path,
                                "PE import symbol",
                            )?;
                            if !name.is_empty() {
                                let name = normalize_symbol(name);
                                symbols.insert(name.clone());
                            }
                        }
                        thunk_index = thunk_index
                            .checked_add(1)
                            .ok_or_else(|| artifact_error(path, "PE thunk 索引溢出".to_owned()))?;
                    }
                }
                descriptor_index = descriptor_index.checked_add(1).ok_or_else(|| {
                    artifact_error(path, "PE import descriptor 索引溢出".to_owned())
                })?;
            }
        }
    }
    Ok(ArtifactInspection {
        object_format: ObjectFormat::Coff,
        symbols: symbols.into_iter().collect(),
        symbol_order,
        symbol_table_readable,
        dependencies: dependencies.into_iter().collect(),
    })
}

#[derive(Clone, Copy)]
/// PE section 的 RVA 到文件偏移映射所需字段。
struct PeSection {
    virtual_address: u32,
    raw_offset: u32,
    raw_size: u32,
}

/// 将 PE 的相对虚拟地址映射到文件内偏移。
fn rva_to_offset(rva: u64, sections: &[PeSection], header_size: u64, path: &Path) -> Result<u64> {
    if rva < header_size {
        return Ok(rva);
    }
    for section in sections {
        let start = u64::from(section.virtual_address);
        let raw_size = u64::from(section.raw_size);
        if rva >= start && rva - start < raw_size {
            return Ok(u64::from(section.raw_offset) + (rva - start));
        }
    }
    Err(artifact_error(
        path,
        format!("PE RVA 0x{rva:x} 没有对应文件偏移"),
    ))
}

/// 从 PE 的相对虚拟地址读取以 NUL 结尾的字符串。
fn c_string_at_rva<'a>(
    bytes: &'a [u8],
    rva: u64,
    sections: &[PeSection],
    header_size: u64,
    path: &Path,
    context: &str,
) -> Result<&'a str> {
    let offset = rva_to_offset(rva, sections, header_size, path)?;
    c_string(bytes, offset, path, context)
}

/// 读取 COFF 符号项中的内嵌名称或字符串表名称。
fn coff_symbol_name<'a>(entry: &'a [u8], strings: &'a [u8], path: &Path) -> Result<&'a str> {
    let zeroes = read_u32(entry, 0, Endian::Little, path, "COFF symbol name prefix")?;
    if zeroes == 0 {
        let offset = read_u32(entry, 4, Endian::Little, path, "COFF symbol string offset")? as u64;
        if offset < 4 {
            return Ok("");
        }
        return c_string(strings, offset, path, "COFF symbol name");
    }
    let end = entry[..8].iter().position(|byte| *byte == 0).unwrap_or(8);
    std::str::from_utf8(&entry[..end])
        .map_err(|error| artifact_error(path, format!("COFF symbol 名称不是 UTF-8：{error}")))
}

/// 去重并稳定排序 IR 层登记的 Runtime 组件。
fn normalized_components(components: &[String]) -> Vec<String> {
    components
        .iter()
        .map(String::as_str)
        .filter(|component| !component.trim().is_empty())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// 根据产物中观察到的 Runtime 符号映射组件名称。
fn components_for_symbols(symbols: &[String]) -> Vec<String> {
    let mut components = BTreeSet::new();
    for symbol in symbols {
        add_components_for_symbol(symbol, &mut components);
    }
    components.into_iter().collect()
}

/// 将一个 Runtime 符号登记到它所属的组件集合。
fn add_components_for_symbol(symbol: &str, components: &mut BTreeSet<String>) {
    if symbol.starts_with("xiao_runtime_value_")
        || symbol.starts_with("xiao_runtime_string_")
        || symbol.starts_with("xiao_runtime_error_")
        || symbol.starts_with("xiao_runtime_abi_")
        || symbol.starts_with("xiao_runtime_language_context_")
        || symbol.starts_with("xiao_runtime_fatal_")
        || symbol == "xiao_runtime_write_i64"
        || symbol == "xiao_runtime_print_values"
        || symbol == "xiao_runtime_input"
    {
        components.insert("value".to_owned());
    }
    if symbol == "xiao_runtime_release"
        || symbol == "xiao_runtime_retain"
        || symbol.starts_with("xiao_runtime_release_trace_")
        || symbol == "xiao_runtime_value_copy"
        || symbol == "xiao_runtime_value_release"
        || symbol == "xiao_runtime_value_release_strong"
        || symbol == "xiao_runtime_value_release_weak"
    {
        components.insert("rc".to_owned());
    }
    if symbol.starts_with("xiao_runtime_weak_")
        || symbol == "xiao_runtime_value_weak"
        || symbol == "xiao_runtime_value_release_weak"
    {
        components.insert("weak".to_owned());
    }
    if symbol.starts_with("xiao_runtime_array_")
        || symbol.starts_with("xiao_runtime_tuple_")
        || symbol.starts_with("xiao_runtime_dict_")
        || symbol.starts_with("xiao_runtime_set_")
        || symbol.starts_with("xiao_runtime_value_array_")
        || symbol.starts_with("xiao_runtime_value_tuple_")
        || symbol.starts_with("xiao_runtime_value_dict_")
        || symbol.starts_with("xiao_runtime_value_set_")
        || symbol == "xiao_runtime_value_select"
    {
        components.insert("containers".to_owned());
    }
    if symbol.starts_with("xiao_runtime_table_") || symbol.starts_with("xiao_runtime_value_table_")
    {
        components.insert("tables".to_owned());
    }
}

/// 返回无法映射到已知组件且不属于调试诊断钩子的 Runtime 符号。
fn unclassified_runtime_symbols(symbols: &[String]) -> Vec<String> {
    symbols
        .iter()
        .filter(|symbol| {
            !symbol.starts_with(DIAGNOSTIC_SYMBOL_PREFIX) && {
                let mut components = BTreeSet::new();
                add_components_for_symbol(symbol, &mut components);
                components.is_empty()
            }
        })
        .cloned()
        .collect()
}

/// 判断外部依赖是否看起来属于 Xiao Runtime。
fn is_runtime_dependency(dependency: &str) -> bool {
    let dependency = dependency.to_ascii_lowercase();
    ["xiao-runtime", "xiao_runtime", "xiao.runtime"]
        .iter()
        .any(|marker| dependency.contains(marker))
}

/// 返回目标 Mach-O 使用的 CPU 类型；未知架构保留旧的首个 slice 兼容行为。
fn macho_cpu_type(target: &TargetDescription) -> Option<u32> {
    match target.triple.split('-').next().unwrap_or_default() {
        "x86_64" => Some(0x0100_0007),
        "aarch64" | "arm64" => Some(0x0100_000c),
        "i386" | "i686" => Some(0x0000_0007),
        _ => None,
    }
}

/// 统一 Mach-O 前导下划线，同时保留其他符号名称。
fn normalize_symbol(symbol: &str) -> String {
    let symbol = symbol.trim_end_matches('\0');
    if let Some(rest) = symbol.strip_prefix("_xiao_") {
        return format!("xiao_{rest}");
    }
    symbol.to_owned()
}

/// 创建带产物路径的统一验证错误。
fn artifact_error(path: &Path, message: String) -> CodegenError {
    CodegenError::ArtifactVerification {
        path: path.to_path_buf(),
        message,
    }
}

/// 安全读取产物中的单个字节。
fn byte_at(bytes: &[u8], offset: u64, path: &Path, context: &str) -> Result<u8> {
    let offset =
        usize::try_from(offset).map_err(|_| artifact_error(path, format!("{context} 偏移溢出")))?;
    bytes
        .get(offset)
        .copied()
        .ok_or_else(|| artifact_error(path, format!("{context} 超出产物边界")))
}

/// 安全取得产物中的连续字节范围，并报告溢出或越界。
fn checked_range<'a>(
    bytes: &'a [u8],
    offset: u64,
    size: u64,
    path: &Path,
    context: &str,
) -> Result<&'a [u8]> {
    let end = offset
        .checked_add(size)
        .ok_or_else(|| artifact_error(path, format!("{context} 范围溢出")))?;
    if end > bytes.len() as u64 {
        return Err(artifact_error(path, format!("{context} 超出产物边界")));
    }
    Ok(&bytes[offset as usize..end as usize])
}

/// 从字节表中读取以 NUL 结尾且为 UTF-8 的字符串。
fn c_string<'a>(bytes: &'a [u8], offset: u64, path: &Path, context: &str) -> Result<&'a str> {
    let start =
        usize::try_from(offset).map_err(|_| artifact_error(path, format!("{context} 偏移溢出")))?;
    if start >= bytes.len() {
        return Err(artifact_error(path, format!("{context} 超出字符串表")));
    }
    let end = bytes[start..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|length| start + length)
        .ok_or_else(|| artifact_error(path, format!("{context} 缺少 NUL 终止符")))?;
    std::str::from_utf8(&bytes[start..end])
        .map_err(|error| artifact_error(path, format!("{context} 不是 UTF-8：{error}")))
}

/// 按目标字节序读取 16 位无符号整数。
fn read_u16(bytes: &[u8], offset: u64, endian: Endian, path: &Path, context: &str) -> Result<u16> {
    let bytes = checked_range(bytes, offset, 2, path, context)?;
    Ok(match endian {
        Endian::Little => u16::from_le_bytes([bytes[0], bytes[1]]),
        Endian::Big => u16::from_be_bytes([bytes[0], bytes[1]]),
    })
}

/// 按目标字节序读取 32 位无符号整数。
fn read_u32(bytes: &[u8], offset: u64, endian: Endian, path: &Path, context: &str) -> Result<u32> {
    let bytes = checked_range(bytes, offset, 4, path, context)?;
    Ok(match endian {
        Endian::Little => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        Endian::Big => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
    })
}

/// 按目标字节序读取 64 位无符号整数。
fn read_u64(bytes: &[u8], offset: u64, endian: Endian, path: &Path, context: &str) -> Result<u64> {
    let bytes = checked_range(bytes, offset, 8, path, context)?;
    Ok(match endian {
        Endian::Little => u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]),
        Endian::Big => u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]),
    })
}

/// 尝试从指定偏移读取小端 32 位整数。
fn read_le_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let bytes = bytes.get(offset..end)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// 尝试从指定偏移读取大端 32 位整数。
fn read_be_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let bytes = bytes.get(offset..end)?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
/// 覆盖三种对象格式及 Runtime 产物级验证的受控夹具测试。
mod tests {
    use std::path::Path;

    use super::{
        ArtifactAcceptanceMode, ArtifactRuntimeComposition, ArtifactVerification,
        SymbolTableComparison, SymbolTableReport, SymbolTableStatus, byte_at,
        compare_symbol_table_reports, components_for_symbols, inspect_symbol_table,
        normalize_symbol, parse_artifact, unclassified_runtime_symbols, verify_artifact,
        verify_artifact_mode,
    };
    use crate::{ObjectFormat, TargetDescription};

    /// 只移除 Mach-O 的 Xiao 前导下划线。
    #[test]
    fn normalizes_macho_leading_underscore_without_touching_other_symbols() {
        assert_eq!(
            normalize_symbol("_xiao_runtime_value_int"),
            "xiao_runtime_value_int"
        );
        assert_eq!(
            normalize_symbol("xiao_runtime_value_int"),
            "xiao_runtime_value_int"
        );
        assert_eq!(normalize_symbol("_main"), "_main");
    }

    /// 将代表性 Runtime 符号映射为组件集合。
    #[test]
    fn maps_runtime_symbols_to_declared_component_names() {
        assert_eq!(
            components_for_symbols(&[
                "xiao_runtime_value_int".to_owned(),
                "xiao_runtime_value_release_weak".to_owned(),
                "xiao_runtime_array_new".to_owned(),
                "xiao_runtime_table_new".to_owned(),
            ]),
            vec!["containers", "rc", "tables", "value", "weak"]
        );
    }

    /// 未知 Runtime 符号不能静默丢失，诊断钩子则由独立字段记录。
    #[test]
    fn keeps_unclassified_runtime_symbols_visible() {
        assert_eq!(
            unclassified_runtime_symbols(&[
                "xiao_runtime_value_int".to_owned(),
                "xiao_runtime_diagnostic_prepare".to_owned(),
                "xiao_runtime_future_feature".to_owned(),
            ]),
            vec!["xiao_runtime_future_feature"]
        );
    }

    /// 不可窄化到当前地址宽度的偏移必须返回结构化错误。
    #[test]
    fn rejects_byte_offset_that_cannot_fit_usize() {
        let error = byte_at(&[1], u64::MAX, Path::new("byte-fixture"), "字节")
            .expect_err("超大偏移必须拒绝");
        if usize::BITS < 64 {
            assert!(error.to_string().contains("偏移溢出"));
        } else {
            assert!(error.to_string().contains("超出产物边界"));
        }
    }

    /// 确认声明组件和观察组件在诊断摘要中保持独立。
    #[test]
    fn composition_keeps_declared_and_observed_components_separate() {
        let composition = ArtifactRuntimeComposition {
            object_format: crate::ObjectFormat::Elf,
            declared_components: vec!["value".to_owned()],
            observed_components: vec!["value".to_owned()],
            runtime_symbols: vec!["xiao_runtime_value_int".to_owned()],
            unclassified_runtime_symbols: Vec::new(),
            dependencies: Vec::new(),
            diagnostic_symbols: Vec::new(),
            verification: ArtifactVerification::Complete,
        };
        assert_eq!(
            composition.declared_components,
            composition.observed_components
        );
    }

    /// 解析受控 ELF 夹具中的 Runtime 符号。
    #[test]
    fn parses_controlled_elf_symbols() {
        let bytes = elf_fixture("xiao_runtime_value_int");
        let inspection = parse_artifact(
            Path::new("elf-fixture"),
            &bytes,
            &TargetDescription::linux_x86_64(),
        )
        .expect("ELF 夹具应通过");
        assert_eq!(inspection.object_format, ObjectFormat::Elf);
        assert!(inspection.has_symbol("xiao_runtime_value_int"));
    }

    /// 解析受控 Mach-O 夹具并规范化符号前缀。
    #[test]
    fn parses_controlled_macho_symbols_and_normalizes_prefix() {
        let bytes = macho_fixture("_xiao_runtime_value_int");
        let inspection = parse_artifact(
            Path::new("macho-fixture"),
            &bytes,
            &TargetDescription::macos_x86_64(),
        )
        .expect("Mach-O 夹具应通过");
        assert_eq!(inspection.object_format, ObjectFormat::MachO);
        assert!(inspection.has_symbol("xiao_runtime_value_int"));
    }

    /// 解析受控 PE/COFF 夹具中的 Runtime 符号。
    #[test]
    fn parses_controlled_pe_coff_symbols() {
        let bytes = pe_fixture("xiao_runtime_value_int");
        let inspection = parse_artifact(
            Path::new("pe-fixture"),
            &bytes,
            &TargetDescription::windows_x86_64(),
        )
        .expect("PE/COFF 夹具应通过");
        assert_eq!(inspection.object_format, ObjectFormat::Coff);
        assert!(inspection.has_symbol("xiao_runtime_value_int"));
    }

    /// 拒绝纯静态产物中意外出现 Runtime 符号。
    #[test]
    fn static_artifact_with_runtime_symbol_is_rejected() {
        let path =
            std::env::temp_dir().join(format!("xiao-artifact-static-{}.elf", std::process::id()));
        std::fs::write(&path, elf_fixture("xiao_runtime_value_int")).expect("写入夹具");
        let result = verify_artifact(&path, &TargetDescription::linux_x86_64(), &[], false);
        let _ = std::fs::remove_file(&path);
        let error = result.expect_err("纯静态产物不能含 Runtime 符号");
        assert!(error.to_string().contains("包含 Runtime 符号"));
    }

    /// 纯静态产物即使没有 Runtime 符号，也不能携带 Runtime 动态依赖。
    #[test]
    fn static_artifact_with_runtime_dependency_is_rejected() {
        let error = verify_fixture(
            "static-runtime-dependency",
            &elf_fixture_with_dependency("main", "libxiao-runtime.so"),
            &TargetDescription::linux_x86_64(),
            &[],
            false,
        )
        .expect_err("纯静态产物不能依赖 Runtime 动态库");
        assert!(error.to_string().contains("Runtime 动态依赖"));
    }

    /// 动态产物保留 Runtime 依赖事实，但不因依赖名本身被拒绝。
    #[test]
    fn dynamic_artifact_reports_runtime_dependency_fact() {
        let composition = verify_fixture(
            "dynamic-runtime-dependency",
            &elf_fixture_with_dependency("xiao_runtime_value_int", "libxiao-runtime.so"),
            &TargetDescription::linux_x86_64(),
            &["value"],
            false,
        )
        .expect("动态产物的 Runtime 依赖应保留为事实");
        assert_eq!(composition.dependencies, vec!["libxiao-runtime.so"]);
    }

    /// 确认三个对象格式都能观察到已声明的 Runtime 组件。
    #[test]
    fn declared_runtime_component_is_observable_for_each_format() {
        let fixtures = [
            (
                "elf",
                elf_fixture("xiao_runtime_array_new"),
                TargetDescription::linux_x86_64(),
            ),
            (
                "macho",
                macho_fixture("_xiao_runtime_array_new"),
                TargetDescription::macos_x86_64(),
            ),
            (
                "pe",
                pe_fixture("xiao_runtime_array_new"),
                TargetDescription::windows_x86_64(),
            ),
        ];
        for (label, bytes, target) in fixtures {
            let composition = verify_fixture(label, &bytes, &target, &["containers"], false)
                .expect("容器组件必须来自链接后符号");
            assert_eq!(composition.observed_components, vec!["containers"]);
            assert_eq!(composition.runtime_symbols, vec!["xiao_runtime_array_new"]);
            assert_eq!(composition.verification, ArtifactVerification::Complete);
        }
    }

    /// 只对调试产物要求调试启动符号，并拒绝普通产物的诊断符号。
    #[test]
    fn debug_symbol_is_required_only_for_debug_artifacts() {
        let debug = elf_fixture("xiao_native_debug_start");
        let target = TargetDescription::linux_x86_64();
        let composition =
            verify_fixture("debug", &debug, &target, &[], true).expect("调试符号必须可见");
        assert_eq!(
            composition.diagnostic_symbols,
            vec!["xiao_native_debug_start"]
        );
        let error = verify_fixture("ordinary", &debug, &target, &[], false)
            .expect_err("普通构建不能包含调试符号");
        assert!(error.to_string().contains("普通构建产物包含"));
        let error = verify_fixture("missing-debug", &elf_fixture("main"), &target, &[], true)
            .expect_err("调试构建不能缺少启动符号");
        assert!(error.to_string().contains("缺少 xiao_native_debug_start"));
    }

    /// 三种产物模式必须分别返回独立的验收事实。
    #[test]
    fn verifies_release_debug_and_stripped_modes_independently() {
        let target = TargetDescription::linux_x86_64();
        let release = verify_fixture_mode(
            "release-mode",
            &elf_fixture("main"),
            &target,
            ArtifactAcceptanceMode::Release,
        )
        .expect("发布模式应通过");
        assert!(release.accepted);
        assert_eq!(release.symbol_table, SymbolTableStatus::Readable);
        assert!(!release.diagnostic_symbols_present);

        let debug = verify_fixture_mode(
            "debug-mode",
            &elf_fixture("xiao_native_debug_start"),
            &target,
            ArtifactAcceptanceMode::Debug,
        )
        .expect("调试模式应通过");
        assert!(debug.accepted);
        assert!(debug.diagnostic_symbols_present);

        let mut stripped_bytes = elf_fixture("main");
        put_u16(&mut stripped_bytes, 60, 0);
        let stripped = verify_fixture_mode(
            "stripped-mode",
            &stripped_bytes,
            &target,
            ArtifactAcceptanceMode::Stripped,
        )
        .expect("剥离模式应明确接受不可读符号表");
        assert!(stripped.accepted);
        assert_eq!(stripped.symbol_table, SymbolTableStatus::Unavailable);
        assert!(stripped.diagnostic.is_some());
    }

    /// 任一侧符号表不可读时，比较结果必须保持不可验证。
    #[test]
    fn symbol_table_comparison_never_equates_unavailable_reports() {
        let readable = SymbolTableReport {
            object_format: ObjectFormat::Elf,
            status: SymbolTableStatus::Readable,
            original_order: vec!["b".to_owned(), "a".to_owned()],
            normalized_order: vec!["a".to_owned(), "b".to_owned()],
        };
        let same = readable.clone();
        assert_eq!(
            compare_symbol_table_reports(&readable, &same),
            SymbolTableComparison::Identical
        );
        let unavailable = SymbolTableReport {
            object_format: ObjectFormat::Coff,
            status: SymbolTableStatus::Unavailable,
            original_order: Vec::new(),
            normalized_order: Vec::new(),
        };
        assert_eq!(
            compare_symbol_table_reports(&readable, &unavailable),
            SymbolTableComparison::Unavailable
        );
    }

    /// PE 有 COFF 表时可读；移除该表后只保留不可验证的导入/导出观测。
    #[test]
    fn pe_symbol_table_status_reflects_actual_coff_table() {
        let target = TargetDescription::windows_x86_64();
        let readable = inspect_fixture_symbol_table(
            "pe-readable-symbols",
            &pe_fixture("xiao_runtime_value_int"),
            &target,
        )
        .expect("带 COFF 表的 PE 应可读");
        assert_eq!(readable.status, SymbolTableStatus::Readable);

        let mut stripped = pe_fixture("xiao_runtime_value_int");
        put_u32(&mut stripped, 0x4c, 0);
        put_u32(&mut stripped, 0x50, 0);
        let unavailable = inspect_fixture_symbol_table("pe-no-coff-symbols", &stripped, &target)
            .expect("没有 COFF 表的 PE 应返回不可验证状态");
        assert_eq!(unavailable.status, SymbolTableStatus::Unavailable);
        assert_eq!(
            compare_symbol_table_reports(&readable, &unavailable),
            SymbolTableComparison::Unavailable
        );
    }

    /// 拒绝缺少可验证符号表的剥离产物。
    #[test]
    fn stripped_symbol_tables_are_not_accepted_as_trimming_evidence() {
        let mut elf = elf_fixture("xiao_runtime_array_new");
        put_u16(&mut elf, 60, 0);
        assert!(
            parse_artifact(
                Path::new("stripped-elf"),
                &elf,
                &TargetDescription::linux_x86_64()
            )
            .expect_err("剥离 ELF 符号表不得判定为裁剪成功")
            .to_string()
            .contains("缺少可验证的符号表")
        );
        let mut macho = macho_fixture("_xiao_runtime_array_new");
        put_u32(&mut macho, 16, 0);
        assert!(
            parse_artifact(
                Path::new("stripped-macho"),
                &macho,
                &TargetDescription::macos_x86_64()
            )
            .expect_err("剥离 Mach-O 符号表不得判定为裁剪成功")
            .to_string()
            .contains("缺少可验证的符号表")
        );
    }

    /// 解析 fat Mach-O 并验证 fat 头与 thin slice 的字节序可独立处理。
    #[test]
    fn parses_fat_macho_slice_with_independent_endianness() {
        let thin = macho_fixture("_xiao_runtime_array_new");
        let mut fat = vec![0_u8; 0x100 + thin.len()];
        fat[..4].copy_from_slice(&0xcafebabe_u32.to_be_bytes());
        fat[4..8].copy_from_slice(&1_u32.to_be_bytes());
        fat[8..12].copy_from_slice(&0x0100_0007_u32.to_be_bytes());
        fat[16..20].copy_from_slice(&0x100_u32.to_be_bytes());
        fat[20..24].copy_from_slice(&(thin.len() as u32).to_be_bytes());
        fat[0x100..].copy_from_slice(&thin);
        let inspection = parse_artifact(
            Path::new("fat-macho"),
            &fat,
            &TargetDescription::macos_x86_64(),
        )
        .expect("fat 表与 thin slice 各自使用正确的字节序");
        assert!(inspection.has_symbol("xiao_runtime_array_new"));
    }

    /// fat Mach-O 必须选择 arm64 slice，而不是因架构未匹配而误读第一个 slice。
    #[test]
    fn parses_arm64_slice_from_fat_macho() {
        let x86 = macho_fixture_with_cpu("_x86_marker", 0x0100_0007);
        let arm64 = macho_fixture_with_cpu("_xiao_runtime_array_new", 0x0100_000c);
        let x86_offset = 0x100_u32;
        let arm64_offset = 0x200_u32;
        let mut fat = vec![0_u8; arm64_offset as usize + arm64.len()];
        fat[..4].copy_from_slice(&0xcafebabe_u32.to_be_bytes());
        fat[4..8].copy_from_slice(&2_u32.to_be_bytes());
        put_be_u32(&mut fat, 8, 0x0100_0007);
        put_be_u32(&mut fat, 16, x86_offset);
        put_be_u32(&mut fat, 20, x86.len() as u32);
        put_be_u32(&mut fat, 28, 0x0100_000c);
        put_be_u32(&mut fat, 36, arm64_offset);
        put_be_u32(&mut fat, 40, arm64.len() as u32);
        fat[x86_offset as usize..x86_offset as usize + x86.len()].copy_from_slice(&x86);
        fat[arm64_offset as usize..arm64_offset as usize + arm64.len()].copy_from_slice(&arm64);

        let inspection = parse_artifact(
            Path::new("fat-macho-arm64"),
            &fat,
            &TargetDescription::macos_aarch64(),
        )
        .expect("应选择 arm64 fat slice");
        assert!(inspection.has_symbol("xiao_runtime_array_new"));
        assert!(!inspection.has_symbol("x86_marker"));
    }

    /// 将内存中的受控对象格式夹具写入临时文件并执行验证。
    fn verify_fixture(
        name: &str,
        bytes: &[u8],
        target: &TargetDescription,
        components: &[&str],
        debug: bool,
    ) -> crate::Result<ArtifactRuntimeComposition> {
        let path =
            std::env::temp_dir().join(format!("xiao-artifact-{name}-{}", std::process::id()));
        std::fs::write(&path, bytes).expect("写入受控产物夹具");
        let result = verify_artifact(
            &path,
            target,
            &components
                .iter()
                .map(|component| (*component).to_owned())
                .collect::<Vec<_>>(),
            debug,
        );
        let _ = std::fs::remove_file(path);
        result
    }

    /// 将内存中的夹具写入临时文件并执行指定模式验收。
    fn verify_fixture_mode(
        name: &str,
        bytes: &[u8],
        target: &TargetDescription,
        mode: ArtifactAcceptanceMode,
    ) -> crate::Result<super::ArtifactModeReport> {
        let path =
            std::env::temp_dir().join(format!("xiao-artifact-mode-{name}-{}", std::process::id()));
        std::fs::write(&path, bytes).expect("写入模式夹具");
        let result = verify_artifact_mode(&path, target, mode, &[]);
        let _ = std::fs::remove_file(path);
        result
    }

    /// 将夹具写入临时文件并读取符号表状态。
    fn inspect_fixture_symbol_table(
        name: &str,
        bytes: &[u8],
        target: &TargetDescription,
    ) -> crate::Result<SymbolTableReport> {
        let path =
            std::env::temp_dir().join(format!("xiao-symbol-table-{name}-{}", std::process::id()));
        std::fs::write(&path, bytes).expect("写入符号表夹具");
        let result = inspect_symbol_table(&path, target);
        let _ = std::fs::remove_file(path);
        result
    }

    /// 构造包含一个符号的最小 ELF 夹具。
    fn elf_fixture(symbol: &str) -> Vec<u8> {
        let strings = format!("\0{symbol}\0").into_bytes();
        let mut bytes = vec![0_u8; 0x280];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        put_u16(&mut bytes, 18, 0x003e);
        put_u64(&mut bytes, 40, 0x100);
        put_u16(&mut bytes, 58, 64);
        put_u16(&mut bytes, 60, 3);
        let symtab = 0x200;
        let strtab = 0x240;
        put_u32(&mut bytes, 0x140 + 4, 2);
        put_u64(&mut bytes, 0x140 + 24, symtab);
        put_u64(&mut bytes, 0x140 + 32, 48);
        put_u32(&mut bytes, 0x140 + 40, 2);
        put_u64(&mut bytes, 0x140 + 56, 24);
        put_u32(&mut bytes, 0x180 + 4, 3);
        put_u64(&mut bytes, 0x180 + 24, strtab);
        put_u64(&mut bytes, 0x180 + 32, strings.len() as u64);
        put_u32(&mut bytes, symtab as usize + 24, 1);
        bytes[strtab as usize..strtab as usize + strings.len()].copy_from_slice(&strings);
        bytes
    }

    /// 构造包含一个符号和一个 DT_NEEDED 依赖的最小 ELF 夹具。
    fn elf_fixture_with_dependency(symbol: &str, dependency: &str) -> Vec<u8> {
        let symbol_strings = format!("\0{symbol}\0");
        let dependency_offset = symbol_strings.len();
        let strings = format!("{symbol_strings}{dependency}\0").into_bytes();
        let mut bytes = vec![0_u8; 0x300];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        put_u16(&mut bytes, 18, 0x003e);
        put_u64(&mut bytes, 40, 0x100);
        put_u16(&mut bytes, 58, 64);
        put_u16(&mut bytes, 60, 4);
        let symtab = 0x200;
        let strtab = 0x240;
        put_u32(&mut bytes, 0x140 + 4, 2);
        put_u64(&mut bytes, 0x140 + 24, symtab);
        put_u64(&mut bytes, 0x140 + 32, 48);
        put_u32(&mut bytes, 0x140 + 40, 2);
        put_u64(&mut bytes, 0x140 + 56, 24);
        put_u32(&mut bytes, 0x180 + 4, 3);
        put_u64(&mut bytes, 0x180 + 24, strtab);
        put_u64(&mut bytes, 0x180 + 32, strings.len() as u64);
        put_u32(&mut bytes, 0x1c0 + 4, 6);
        put_u64(&mut bytes, 0x1c0 + 24, 0x280);
        put_u64(&mut bytes, 0x1c0 + 32, 32);
        put_u32(&mut bytes, 0x1c0 + 40, 2);
        put_u64(&mut bytes, 0x1c0 + 56, 16);
        put_u64(&mut bytes, 0x280, 1);
        put_u64(&mut bytes, 0x288, dependency_offset as u64);
        put_u32(&mut bytes, symtab as usize + 24, 1);
        bytes[strtab as usize..strtab as usize + strings.len()].copy_from_slice(&strings);
        bytes
    }

    /// 构造包含一个符号的最小 Mach-O 夹具。
    fn macho_fixture(symbol: &str) -> Vec<u8> {
        macho_fixture_with_cpu(symbol, 0x0100_0007)
    }

    /// 构造包含一个符号和指定 CPU 类型的最小 Mach-O 夹具。
    fn macho_fixture_with_cpu(symbol: &str, cpu: u32) -> Vec<u8> {
        let strings = format!("\0{symbol}\0").into_bytes();
        let mut bytes = vec![0_u8; 0x120];
        bytes[..4].copy_from_slice(&0xfeedfacfu32.to_le_bytes());
        put_u32(&mut bytes, 4, cpu);
        put_u32(&mut bytes, 16, 1);
        put_u32(&mut bytes, 20, 24);
        put_u32(&mut bytes, 32, 2);
        put_u32(&mut bytes, 36, 24);
        put_u32(&mut bytes, 40, 0x80);
        put_u32(&mut bytes, 44, 1);
        put_u32(&mut bytes, 48, 0x90);
        put_u32(&mut bytes, 52, strings.len() as u32);
        put_u32(&mut bytes, 0x80, 1);
        bytes[0x90..0x90 + strings.len()].copy_from_slice(&strings);
        bytes
    }

    /// 构造包含一个符号的最小 PE/COFF 夹具。
    fn pe_fixture(symbol: &str) -> Vec<u8> {
        let strings = format!("\0\0\0\0{symbol}\0").into_bytes();
        let optional_offset = 0x58;
        let symbol_offset = optional_offset + 0xf0;
        let string_offset = symbol_offset + 18;
        let mut bytes = vec![0_u8; 0x240];
        bytes[..2].copy_from_slice(b"MZ");
        put_u32(&mut bytes, 0x3c, 0x40);
        bytes[0x40..0x44].copy_from_slice(b"PE\0\0");
        put_u16(&mut bytes, 0x44, 0x8664);
        put_u32(&mut bytes, 0x4c, symbol_offset as u32);
        put_u32(&mut bytes, 0x50, 1);
        put_u16(&mut bytes, 0x54, 0xf0);
        put_u16(&mut bytes, optional_offset, 0x20b);
        put_u32(&mut bytes, optional_offset + 108, 0);
        put_u32(&mut bytes, symbol_offset + 4, 4);
        put_u32(&mut bytes, string_offset, strings.len() as u32);
        bytes[string_offset + 4..string_offset + strings.len()].copy_from_slice(&strings[4..]);
        bytes
    }

    /// 向夹具写入小端 16 位整数。
    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    /// 向夹具写入小端 32 位整数。
    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// 向夹具写入大端 32 位整数。
    fn put_be_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    /// 向夹具写入小端 64 位整数。
    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
