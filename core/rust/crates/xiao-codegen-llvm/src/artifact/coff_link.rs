//! 受控 MSVC 链接映射的解析与映像身份校验。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::{
    DEBUG_ACTIVATION_SYMBOLS, RUNTIME_SYMBOL_PREFIX, artifact_error, normalize_symbol,
    read_artifact_bytes, read_u16, read_u32, read_u64,
};
use crate::error::Result;
use crate::target::Endian;

/// 受控 MSVC 链接清单与最终映像的字节身份。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoffLinkIdentity {
    /// 本次链接保留的 map 路径。
    pub map_path: PathBuf,
    /// 最终 PE 映像的 SHA-256。
    pub artifact_sha256: String,
    /// 链接器生成的完整映射的 SHA-256。
    pub map_sha256: String,
}

/// 只由工具链在本次链接结束后构造。
pub(crate) struct CoffLinkEvidence {
    pub(super) identity: CoffLinkIdentity,
    pub(super) symbols: Vec<String>,
    map_bytes: Vec<u8>,
}

impl CoffLinkEvidence {
    /// 在完成产物验证后发布原始链接清单。
    pub(crate) fn save_map(&self) -> Result<()> {
        crate::build::write_llvm_artifact_bytes(&self.identity.map_path, &self.map_bytes)
    }

    /// 比对 PE 中的真实地址范围和 MSVC 清单，保存二者的摘要。
    pub(crate) fn read(executable: &Path, map: &Path, output: &Path) -> Result<Self> {
        let artifact = read_artifact_bytes(executable)?;
        let map_bytes = read_artifact_bytes(map)?;
        let fail = || artifact_error(executable, "MSVC 链接映射或映像身份无效".to_owned());
        let text = String::from_utf8_lossy(&map_bytes);
        let pe_offset = u64::from(read_u32(
            &artifact,
            0x3c,
            Endian::Little,
            executable,
            "PE header offset",
        )?);
        let pe_timestamp = read_u32(
            &artifact,
            pe_offset + 8,
            Endian::Little,
            executable,
            "PE timestamp",
        )?;
        let image_base = read_u64(
            &artifact,
            pe_offset + 48,
            Endian::Little,
            executable,
            "PE image base",
        )?;
        let section_count = u64::from(read_u16(
            &artifact,
            pe_offset + 6,
            Endian::Little,
            executable,
            "PE section count",
        )?);
        let optional_size = u64::from(read_u16(
            &artifact,
            pe_offset + 20,
            Endian::Little,
            executable,
            "PE optional size",
        )?);
        let sections = pe_offset + 24 + optional_size;
        let lines: Vec<_> = text.lines().map(str::trim).collect();
        let stamp = lines
            .iter()
            .find_map(|line| line.strip_prefix("Timestamp is "))
            .and_then(|line| line.split_whitespace().next())
            .and_then(|value| u32::from_str_radix(value, 16).ok());
        let base = lines
            .iter()
            .find_map(|line| line.strip_prefix("Preferred load address is "))
            .and_then(|value| u64::from_str_radix(value, 16).ok());
        if stamp != Some(pe_timestamp) || base != Some(image_base) {
            return Err(fail());
        }
        let start = lines
            .iter()
            .position(|line| line.contains("Publics by Value") && line.contains("Rva+Base"))
            .ok_or_else(fail)?;
        let end = lines[start + 1..]
            .iter()
            .position(|line| line.starts_with("entry point at"))
            .ok_or_else(fail)?
            + start
            + 1;
        let mut symbols = BTreeSet::new();
        for line in lines[start + 1..end].iter().filter(|line| !line.is_empty()) {
            let mut values = line.split_whitespace();
            let (segment, offset) = values
                .next()
                .and_then(|address| address.split_once(':'))
                .ok_or_else(fail)?;
            let name = values.next().ok_or_else(fail)?;
            let address = values
                .next()
                .and_then(|value| u64::from_str_radix(value, 16).ok())
                .ok_or_else(fail)?;
            if !name.starts_with(RUNTIME_SYMBOL_PREFIX) && !DEBUG_ACTIVATION_SYMBOLS.contains(&name)
            {
                continue;
            }
            let segment = u64::from_str_radix(segment, 16).map_err(|_| fail())?;
            let offset = u64::from_str_radix(offset, 16).map_err(|_| fail())?;
            if segment == 0 || segment > section_count {
                return Err(fail());
            }
            let section = sections + (segment - 1) * 40;
            let size = u64::from(read_u32(
                &artifact,
                section + 8,
                Endian::Little,
                executable,
                "PE section size",
            )?);
            let rva = u64::from(read_u32(
                &artifact,
                section + 12,
                Endian::Little,
                executable,
                "PE section RVA",
            )?);
            if offset >= size || address.checked_sub(image_base) != rva.checked_add(offset) {
                return Err(fail());
            }
            symbols.insert(normalize_symbol(name));
        }
        if symbols.is_empty() {
            return Err(fail());
        }
        Ok(Self {
            identity: CoffLinkIdentity {
                map_path: output.to_path_buf(),
                artifact_sha256: format!("{:x}", Sha256::digest(&artifact)),
                map_sha256: format!("{:x}", Sha256::digest(&map_bytes)),
            },
            symbols: symbols.into_iter().collect(),
            map_bytes,
        })
    }
}
