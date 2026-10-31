//! 22E 原生产物的自描述元数据；固定小端格式与对象格式无关，校验不执行用户代码。

use crate::{CODEGEN_VERSION, ObjectFormat, TargetDescription};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use xiao_runtime_abi::{ABI_ENCODED_VERSION, SUPPORTED_CONCURRENCY_FEATURES};
use xiao_standard_library::{StandardLibraryIdentity, current_standard_library_identity};

#[path = "native_metadata_sections.rs"]
/// 有界的 ELF/PE/Mach-O 元数据节定位。
mod sections;
#[cfg(test)]
#[path = "native_metadata_tests.rs"]
/// codec、要求和恶意容器拒绝回归。
mod tests;

/// 与 Runtime ABI 独立的原生元数据格式版本。
pub const NATIVE_METADATA_VERSION: u16 = 1;
/// 编解码上限，防止读取器根据恶意长度分配无界内存。
const MAX_METADATA_BYTES: usize = 64 * 1024;
/// 只在指定对象节中接受的固定标识，不对可执行文件全文搜索。
const MAGIC: &[u8; 8] = b"XIAOMETA";

/// 可独立校验的原生产物要求；required-feature 沿用 `.xar` 位表，bit 0 不允许出现。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NativeMetadata {
    /// 元数据格式版本。
    pub version: u16,
    /// 必需 Runtime ABI 范围下界。
    pub runtime_abi_min: u64,
    /// 必需 Runtime ABI 范围上界；首版精确为 1.17，不隐式扩大。
    pub runtime_abi_max: u64,
    /// 冻结并发能力位，不含 archive_requirements。
    pub required_features: u64,
    /// LLVM 生成器版本。
    pub generator_version: u32,
    /// 规范目标 triple。
    pub target: String,
    /// 指针宽度（位）。
    pub pointer_width: u16,
    /// 由标准库权威元数据生成的兼容身份。
    pub stdlib: StandardLibraryIdentity,
}

/// 读取/验证错误复用 22E 已确认的版本、未知位和目标拒绝身份。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeMetadataError {
    /// 稳定诊断编号。
    pub code: &'static str,
    /// 结构化原因的可读说明。
    pub message: String,
}

impl fmt::Display for NativeMetadataError {
    /// 错误编号与原因一起显示，调用方可直接读取 code 字段。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for NativeMetadataError {}

/// 校验宿主/Runtime 的显式能力，不把编译器当前版本冒充旧 Runtime 实测。
#[derive(Clone, Debug)]
pub struct NativeRuntimeSupport {
    /// 当前 Runtime ABI。
    pub runtime_abi: u64,
    /// Runtime 实际支持的并发位集。
    pub features: u64,
    /// 当前校验器理解的最大生成器版本。
    pub generator_version: u32,
}
impl Default for NativeRuntimeSupport {
    /// 当前工具链的精确 ABI、已知位集及生成器理解上限。
    fn default() -> Self {
        Self {
            runtime_abi: ABI_ENCODED_VERSION,
            features: SUPPORTED_CONCURRENCY_FEATURES,
            generator_version: CODEGEN_VERSION,
        }
    }
}

/// 缺失/格式/版本/要求/目标不符统一使用已有 requirements 拒绝码。
fn invalid(message: impl Into<String>) -> NativeMetadataError {
    NativeMetadataError {
        code: "X17-XAR-007",
        message: message.into(),
    }
}

impl NativeMetadata {
    /// 从同源类型化 IR 构造产物要求，未知操作返回结构化拒绝。
    pub fn for_program(
        program: &xiao_ir::IrProgram,
        target: &TargetDescription,
    ) -> Result<Self, NativeMetadataError> {
        target
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        if target.pointer_width != 64 {
            return Err(NativeMetadataError {
                code: "X20-TARGET-001",
                message: "并发原生产物仅支持 64 位目标".to_owned(),
            });
        }
        Ok(Self {
            version: NATIVE_METADATA_VERSION,
            runtime_abi_min: ABI_ENCODED_VERSION,
            runtime_abi_max: ABI_ENCODED_VERSION,
            required_features: xiao_ir::required_concurrency_features(program).map_err(invalid)?,
            generator_version: CODEGEN_VERSION,
            target: target.triple.clone(),
            pointer_width: target.pointer_width,
            stdlib: current_standard_library_identity(),
        })
    }

    /// 确定性编码：固定 60B 头，三个 u32 长度前缀 UTF-8 字符串，无未知尾部字段。
    pub fn encode(&self) -> Result<Vec<u8>, NativeMetadataError> {
        let format = if self.target.contains("windows") {
            ObjectFormat::Coff
        } else if self.target.contains("darwin") {
            ObjectFormat::MachO
        } else {
            ObjectFormat::Elf
        };
        let target = TargetDescription::new(
            self.target.clone(),
            self.pointer_width,
            crate::Endian::Little,
            format,
        )
        .map_err(|error| invalid(error.to_string()))?;
        self.validate(&target, &NativeRuntimeSupport::default(), 0)?;
        let mut bytes = MAGIC.to_vec();
        bytes.extend(self.version.to_le_bytes());
        bytes.extend(0_u16.to_le_bytes());
        bytes.extend(0_u32.to_le_bytes());
        bytes.extend(self.runtime_abi_min.to_le_bytes());
        bytes.extend(self.runtime_abi_max.to_le_bytes());
        bytes.extend(self.required_features.to_le_bytes());
        bytes.extend(self.generator_version.to_le_bytes());
        bytes.extend(self.pointer_width.to_le_bytes());
        bytes.extend(0_u16.to_le_bytes());
        bytes.extend(self.stdlib.stdlib_interface_version.to_le_bytes());
        bytes.extend(
            xiao_runtime_abi::encoded_abi_version(
                self.stdlib.runtime_abi_major,
                self.stdlib.runtime_abi_minor,
            )
            .to_le_bytes(),
        );
        for text in [
            &self.target,
            &self.stdlib.language_version,
            &self.stdlib.target,
        ] {
            if text.len() > 4096 {
                return Err(invalid("元数据字符串超限"));
            }
            bytes.extend((text.len() as u32).to_le_bytes());
            bytes.extend(text.as_bytes());
        }
        if bytes.len() > MAX_METADATA_BYTES {
            return Err(invalid("元数据超限"));
        }
        let length = bytes.len() as u32;
        bytes[12..16].copy_from_slice(&length.to_le_bytes());
        Ok(bytes)
    }

    /// 解码独立的元数据记录；PE 节填充在对象读取层剔除，记录本身不接受尾部垃圾。
    pub fn decode(bytes: &[u8]) -> Result<Self, NativeMetadataError> {
        if bytes.len() < 60 || bytes.len() > MAX_METADATA_BYTES || &bytes[..8] != MAGIC {
            return Err(invalid("元数据标识或长度无效"));
        }
        let mut reader = Reader { bytes, offset: 8 };
        let version = reader.u16()?;
        if version != NATIVE_METADATA_VERSION || reader.u16()? != 0 {
            return Err(invalid("元数据版本或保留字段无效"));
        }
        if reader.u32()? as usize != bytes.len() {
            return Err(invalid("元数据总长度不符"));
        }
        let runtime_abi_min = reader.u64()?;
        let runtime_abi_max = reader.u64()?;
        let required_features = reader.u64()?;
        let generator_version = reader.u32()?;
        let pointer_width = reader.u16()?;
        if reader.u16()? != 0 {
            return Err(invalid("元数据保留字段非零"));
        }
        let stdlib_interface_version = reader.u32()?;
        let stdlib_abi = reader.u64()?;
        if stdlib_abi >> 48 != 0 {
            return Err(invalid("标准库 ABI 编码超限"));
        }
        let target = reader.string()?;
        let language_version = reader.string()?;
        let portable = reader.string()?;
        if reader.offset != bytes.len() {
            return Err(invalid("元数据含额外字段"));
        }
        Ok(Self {
            version,
            runtime_abi_min,
            runtime_abi_max,
            required_features,
            generator_version,
            target,
            pointer_width,
            stdlib: StandardLibraryIdentity {
                language_version,
                stdlib_interface_version,
                runtime_abi_major: (stdlib_abi >> 16) as u32,
                runtime_abi_minor: (stdlib_abi & 0xffff) as u32,
                target: portable,
            },
        })
    }

    /// 在读取器中校验实际 Runtime、已知位、目标和标准库身份；不加载或运行入口。
    pub fn validate(
        &self,
        target: &TargetDescription,
        support: &NativeRuntimeSupport,
        required: u64,
    ) -> Result<(), NativeMetadataError> {
        if self.pointer_width != 64 || target.pointer_width != 64 {
            return Err(NativeMetadataError {
                code: "X20-TARGET-001",
                message: "并发产物需要 64 位目标".to_owned(),
            });
        }
        if (self.required_features | required) & !SUPPORTED_CONCURRENCY_FEATURES != 0 {
            return Err(NativeMetadataError {
                code: "X17-XAR-011",
                message: "原生元数据含未知或归档专用 feature 位".to_owned(),
            });
        }
        target
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        if self.version != NATIVE_METADATA_VERSION
            || self.runtime_abi_min != ABI_ENCODED_VERSION
            || self.runtime_abi_max != ABI_ENCODED_VERSION
            || support.runtime_abi < self.runtime_abi_min
            || support.runtime_abi > self.runtime_abi_max
            || self.generator_version > support.generator_version
            || self.generator_version < 14
            || self.target != target.triple
            || self.required_features & !support.features != 0
            || required & !self.required_features != 0
            || self.stdlib != current_standard_library_identity()
        {
            return Err(invalid("原生产物的版本、特性要求、目标或标准库身份不满足"));
        }
        Ok(())
    }
}

/// 有界小端读取器；所有长度均先检查再切片，不按外来长度预分配。
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    /// 检查加法及边界后消费字节，不进行外来长度的内存分配。
    fn take(&mut self, n: usize) -> Result<&'a [u8], NativeMetadataError> {
        let end = self
            .offset
            .checked_add(n)
            .ok_or_else(|| invalid("元数据长度溢出"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| invalid("元数据被截断"))?;
        self.offset = end;
        Ok(value)
    }
    /// 读取小端 16 位字段。
    fn u16(&mut self) -> Result<u16, NativeMetadataError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    /// 读取小端 32 位字段。
    fn u32(&mut self) -> Result<u32, NativeMetadataError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    /// 读取小端 64 位字段。
    fn u64(&mut self) -> Result<u64, NativeMetadataError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    /// 读取长度不超过 4096B 的非空 UTF-8 字符串。
    fn string(&mut self) -> Result<String, NativeMetadataError> {
        let n = self.u32()? as usize;
        if n == 0 || n > 4096 {
            return Err(invalid("元数据字符串长度无效"));
        }
        std::str::from_utf8(self.take(n)?)
            .map(str::to_owned)
            .map_err(|_| invalid("元数据字符串非 UTF-8"))
    }
}

/// 从指定目标对象的专用节读取元数据，旧的无元数据文件返回 None；不会全文搜索 magic。
pub fn read_native_metadata(
    path: impl AsRef<Path>,
    target: &TargetDescription,
) -> Result<Option<NativeMetadata>, NativeMetadataError> {
    let bytes = std::fs::read(path).map_err(|error| invalid(error.to_string()))?;
    read_native_metadata_bytes(&bytes, target)
}

/// 工具链验收入口；读取并检查要求，但从不启动待验收的可执行文件。
pub fn validate_native_metadata_file(
    path: impl AsRef<Path>,
    target: &TargetDescription,
    support: &NativeRuntimeSupport,
    required_features: u64,
    require_metadata: bool,
) -> Result<Option<NativeMetadata>, NativeMetadataError> {
    let metadata = read_native_metadata(path, target)?;
    match &metadata {
        Some(value) => value.validate(target, support, required_features)?,
        None if require_metadata || required_features != 0 => {
            return Err(invalid("原生产物缺少必需的并发元数据"));
        }
        None => {}
    }
    Ok(metadata)
}

/// 验证内存中的真实 ELF/PE/Mach-O 专用节，用于文件读取和畸形产物回归。
pub fn read_native_metadata_bytes(
    bytes: &[u8],
    target: &TargetDescription,
) -> Result<Option<NativeMetadata>, NativeMetadataError> {
    let Some(section) = sections::find(bytes, target)? else {
        return Ok(None);
    };
    let record = if target.object_format == ObjectFormat::Elf {
        if section.len() < 20
            || section[..4] != 5_u32.to_le_bytes()
            || &section[12..20] != b"XIAO\0\0\0\0"
        {
            return Err(invalid("Xiao ELF note 头无效"));
        }
        let length = u32::from_le_bytes(section[4..8].try_into().unwrap()) as usize;
        if section[8..12] != 1_u32.to_le_bytes() {
            return Err(invalid("Xiao ELF note 类型无效"));
        }
        let end = 20_usize
            .checked_add(length)
            .ok_or_else(|| invalid("note 长度溢出"))?;
        if section
            .get(end..)
            .is_none_or(|padding| padding.len() > 3 || padding.iter().any(|byte| *byte != 0))
        {
            return Err(invalid("Xiao ELF note 尾部无效或含第二条记录"));
        }
        section
            .get(
                20..20_usize
                    .checked_add(length)
                    .ok_or_else(|| invalid("note 长度溢出"))?,
            )
            .ok_or_else(|| invalid("Xiao ELF note 被截断"))?
    } else {
        let length = section.get(12..16).ok_or_else(|| invalid("专用节被截断"))?;
        let length = u32::from_le_bytes(length.try_into().unwrap()) as usize;
        if section
            .get(length..)
            .is_none_or(|padding| padding.iter().any(|byte| *byte != 0))
        {
            return Err(invalid("元数据节含额外非填充字节"));
        }
        section
            .get(..length)
            .ok_or_else(|| invalid("专用节声明长度越界"))?
    };
    NativeMetadata::decode(record).map(Some)
}

/// 发射只读全局及保留标记，节数据在链接/strip 后仍能被独立读取。
pub(crate) fn emit(
    metadata: &NativeMetadata,
    format: ObjectFormat,
) -> Result<String, NativeMetadataError> {
    let mut bytes = metadata.encode()?;
    let section = match format {
        ObjectFormat::Coff => ".xiaomta",
        ObjectFormat::MachO => "__XIAO,__meta",
        ObjectFormat::Elf => ".note.xiao",
    };
    if format == ObjectFormat::Elf {
        let mut note = 5_u32.to_le_bytes().to_vec();
        note.extend((bytes.len() as u32).to_le_bytes());
        note.extend(1_u32.to_le_bytes());
        note.extend(b"XIAO\0\0\0\0");
        note.append(&mut bytes);
        while note.len() % 4 != 0 {
            note.push(0);
        }
        bytes = note;
    }
    let escaped = bytes
        .iter()
        .map(|byte| format!("\\{byte:02X}"))
        .collect::<String>();
    Ok(format!(
        "\n@xiao_native_metadata = constant [{} x i8] c\"{escaped}\", section \"{section}\", align 4\n@llvm.used = appending global [1 x ptr] [ptr @xiao_native_metadata], section \"llvm.metadata\"\n",
        bytes.len()
    ))
}
