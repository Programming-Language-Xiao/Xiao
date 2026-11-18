//! 原生元数据的有界对象节定位。只支持当前冻结的六个 64 位小端目标，不搜索可执行代码。

use super::{NativeMetadataError, invalid};
use crate::{Endian, ObjectFormat, TargetDescription};

/// 取外来偏移对应的切片，先检查加法与宿主整数宽度。
fn range(bytes: &[u8], offset: u64, size: u64) -> Result<&[u8], NativeMetadataError> {
    let start = usize::try_from(offset).map_err(|_| invalid("节偏移溢出"))?;
    let size = usize::try_from(size).map_err(|_| invalid("节长度溢出"))?;
    bytes
        .get(
            start
                ..start
                    .checked_add(size)
                    .ok_or_else(|| invalid("节边界溢出"))?,
        )
        .ok_or_else(|| invalid("对象节被截断"))
}
/// 有界读取对象容器的小端 16 位字段。
fn u16_at(bytes: &[u8], offset: u64) -> Result<u16, NativeMetadataError> {
    Ok(u16::from_le_bytes(
        range(bytes, offset, 2)?.try_into().unwrap(),
    ))
}
/// 有界读取对象容器的小端 32 位字段。
fn u32_at(bytes: &[u8], offset: u64) -> Result<u32, NativeMetadataError> {
    Ok(u32::from_le_bytes(
        range(bytes, offset, 4)?.try_into().unwrap(),
    ))
}
/// 有界读取对象容器的小端 64 位字段。
fn u64_at(bytes: &[u8], offset: u64) -> Result<u64, NativeMetadataError> {
    Ok(u64::from_le_bytes(
        range(bytes, offset, 8)?.try_into().unwrap(),
    ))
}
/// 去掉对象格式固定宽度名称的 NUL 填充。
fn name(bytes: &[u8]) -> &[u8] {
    bytes.split(|byte| *byte == 0).next().unwrap_or_default()
}
/// 专用节必须唯一，重复节不能选择性放过其中一份记录。
fn accept<'a>(found: &mut Option<&'a [u8]>, section: &'a [u8]) -> Result<(), NativeMetadataError> {
    if found.replace(section).is_some() {
        return Err(invalid("产物含重复 Xiao 元数据节"));
    }
    Ok(())
}

/// 校验容器架构并定位唯一的只读 metadata 节，缺失返回 None。
pub(super) fn find<'a>(
    bytes: &'a [u8],
    target: &TargetDescription,
) -> Result<Option<&'a [u8]>, NativeMetadataError> {
    target
        .validate()
        .map_err(|error| invalid(error.to_string()))?;
    if target.pointer_width != 64 {
        return Err(NativeMetadataError {
            code: "X20-TARGET-001",
            message: "并发原生产物仅支持 64 位".to_owned(),
        });
    }
    if target.endian != Endian::Little {
        return Err(invalid("当前元数据容器读取器只支持小端目标"));
    }
    let arm = target.triple.starts_with("aarch64-");
    if !arm && !target.triple.starts_with("x86_64-") {
        return Err(invalid("未知原生产物架构"));
    }
    match target.object_format {
        ObjectFormat::Elf => elf(bytes, if arm { 183 } else { 62 }),
        ObjectFormat::Coff => pe(bytes, if arm { 0xaa64 } else { 0x8664 }),
        ObjectFormat::MachO => macho(bytes, if arm { 0x0100000c } else { 0x01000007 }),
    }
}

/// 从已核对架构的 ELF64 节名目录定位只读 .note.xiao。
fn elf(bytes: &[u8], machine: u16) -> Result<Option<&[u8]>, NativeMetadataError> {
    if !matches!(u16_at(bytes, 16)?, 2 | 3) {
        return Err(invalid("ELF 输入不是已链接的可执行产物"));
    }
    if range(bytes, 0, 6)? != b"\x7fELF\x02\x01" || u16_at(bytes, 18)? != machine {
        return Err(invalid("ELF 类型或目标不符"));
    }
    let offset = u64_at(bytes, 40)?;
    let stride = u16_at(bytes, 58)? as u64;
    let count = u16_at(bytes, 60)? as u64;
    let strings = u16_at(bytes, 62)? as u64;
    if stride < 64 || count == 0 || strings >= count {
        return Err(invalid("ELF 节目录无效"));
    }
    let table = range(bytes, offset, stride * count)?;
    let names = &table[(strings * stride) as usize..][..64];
    let names = range(bytes, u64_at(names, 24)?, u64_at(names, 32)?)?;
    let mut found = None;
    for index in 0..count {
        let section = &table[(index * stride) as usize..][..64];
        let start = u32_at(section, 0)? as usize;
        let title = names.get(start..).ok_or_else(|| invalid("ELF 节名越界"))?;
        if name(title) == b".note.xiao" {
            if u64_at(section, 8)? & 1 != 0 {
                return Err(invalid("Xiao 元数据不得可写"));
            }
            accept(
                &mut found,
                range(bytes, u64_at(section, 24)?, u64_at(section, 32)?)?,
            )?;
        }
    }
    Ok(found)
}

/// 从 PE64 节表定位只读 .xiaomta，所有 RVA 外偏移均先作边界检查。
fn pe(bytes: &[u8], machine: u16) -> Result<Option<&[u8]>, NativeMetadataError> {
    if range(bytes, 0, 2)? != b"MZ" {
        return Err(invalid("PE 标识无效"));
    }
    let header = u32_at(bytes, 60)? as u64;
    if u16_at(bytes, header + 22)? & 2 == 0 {
        return Err(invalid("PE 输入不是可执行映像"));
    }
    if range(bytes, header, 4)? != b"PE\0\0" || u16_at(bytes, header + 4)? != machine {
        return Err(invalid("PE 类型或目标不符"));
    }
    let count = u16_at(bytes, header + 6)? as u64;
    let optional = u16_at(bytes, header + 20)? as u64;
    if optional < 112 || u16_at(bytes, header + 24)? != 0x20b {
        return Err(invalid("PE64 可选头无效"));
    }
    let table = range(bytes, header + 24 + optional, count * 40)?;
    let mut found = None;
    for section in table.chunks_exact(40) {
        if name(&section[..8]) == b".xiaomta" {
            if u32_at(section, 36)? & 0x80000000 != 0 {
                return Err(invalid("Xiao 元数据不得可写"));
            }
            accept(
                &mut found,
                range(
                    bytes,
                    u32_at(section, 20)? as u64,
                    u32_at(section, 16)? as u64,
                )?,
            )?;
        }
    }
    Ok(found)
}

/// 从当前单架构 Mach-O64 的 segment/section 命令定位只读 __XIAO,__meta。
fn macho(bytes: &[u8], cpu: u32) -> Result<Option<&[u8]>, NativeMetadataError> {
    if u32_at(bytes, 12)? != 2 {
        return Err(invalid("Mach-O 输入不是已链接可执行产物"));
    }
    if u32_at(bytes, 0)? != 0xfeedfacf || u32_at(bytes, 4)? != cpu {
        return Err(invalid(
            "Mach-O 类型或目标不符（通用二进制需先选择目标切片）",
        ));
    }
    let count = u32_at(bytes, 16)?;
    let commands = range(bytes, 32, u32_at(bytes, 20)? as u64)?;
    let mut cursor = 0_u64;
    let mut found = None;
    for _ in 0..count {
        let command = u32_at(commands, cursor)?;
        let size = u32_at(commands, cursor + 4)? as u64;
        if size < 8 {
            return Err(invalid("Mach-O 命令大小无效"));
        }
        let load = range(commands, cursor, size)?;
        if command == 0x19 {
            let count = u32_at(load, 64)? as u64;
            let sections = range(load, 72, count * 80)?;
            for section in sections.chunks_exact(80) {
                if name(&section[..16]) == b"__meta" && name(&section[16..32]) == b"__XIAO" {
                    if u32_at(load, 60)? & 2 != 0 {
                        return Err(invalid("Xiao 元数据段不得可写"));
                    }
                    accept(
                        &mut found,
                        range(bytes, u32_at(section, 48)? as u64, u64_at(section, 40)?)?,
                    )?;
                }
            }
        }
        cursor = cursor
            .checked_add(size)
            .ok_or_else(|| invalid("Mach-O 命令偏移溢出"))?;
    }
    if cursor != commands.len() as u64 {
        return Err(invalid("Mach-O 命令目录大小不符"));
    }
    Ok(found)
}
