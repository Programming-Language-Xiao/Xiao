//! 15B 可复现构建比较与差异白名单。

use std::collections::BTreeSet;

use crate::target::ObjectFormat;

/// 可复现比较中允许的差异类别。
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReproducibleDifferenceKind {
    /// 外部工具链版本不同。
    ToolchainVersion,
    /// 目标平台/对象格式不同。
    Target,
    /// 二进制布局、对齐或链接器节顺序不同。
    BinaryLayout,
    /// 文件体积不同。
    Size,
    /// 测量耗时不同。
    Time,
}

impl ReproducibleDifferenceKind {
    /// 返回稳定名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ToolchainVersion => "toolchain-version",
            Self::Target => "target",
            Self::BinaryLayout => "binary-layout",
            Self::Size => "size",
            Self::Time => "time",
        }
    }
}

/// 一条有明确理由的差异白名单记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReproducibleDifference {
    /// 差异类别。
    pub kind: ReproducibleDifferenceKind,
    /// 面向审计的理由。
    pub reason: String,
}

/// 两次构建的可复现比较报告。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReproducibilityReport {
    /// 第一次规范 LLVM 文本摘要。
    pub first_fingerprint: String,
    /// 第二次规范 LLVM 文本摘要。
    pub second_fingerprint: String,
    /// LLVM 文本是否逐字一致。
    pub llvm_identical: bool,
    /// 两次符号表是否都已规范排序且顺序一致。
    pub symbols_identical: bool,
    /// 允许存在的差异白名单。
    pub whitelist: Vec<ReproducibleDifference>,
    /// 未被白名单解释的差异名称。
    pub unexpected_differences: Vec<String>,
    /// 明确执行过的归一化字段。
    pub normalized_fields: Vec<String>,
}

impl ReproducibilityReport {
    /// 判断比较是否通过。
    #[must_use]
    pub fn passed(&self) -> bool {
        self.unexpected_differences.is_empty()
    }
}

/// 产物层逐字节可复现比较报告。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactReproducibilityReport {
    /// 第一次归一化产物摘要。
    pub first_fingerprint: String,
    /// 第二次归一化产物摘要。
    pub second_fingerprint: String,
    /// 归一化后的字节是否完全一致。
    pub identical: bool,
    /// 明确执行过的字段归一化。
    pub normalized_fields: Vec<String>,
    /// 未被白名单解释的差异。
    pub unexpected_differences: Vec<String>,
}

impl ArtifactReproducibilityReport {
    /// 判断产物比较是否通过。
    #[must_use]
    pub fn passed(&self) -> bool {
        self.identical || self.unexpected_differences.is_empty()
    }
}

/// 比较两个链接后产物，并显式归一化 PE `TimeDateStamp`。
#[must_use]
pub fn compare_artifact_bytes(
    first: &[u8],
    second: &[u8],
    format: ObjectFormat,
    whitelist: &[ReproducibleDifference],
) -> ArtifactReproducibilityReport {
    let first_raw = first;
    let second_raw = second;
    let (_first, mut normalized_fields) = normalize_artifact(first_raw, format);
    let (_second, second_fields) = normalize_artifact(second_raw, format);
    normalized_fields.extend(second_fields);
    normalized_fields.sort();
    normalized_fields.dedup();
    let first_comparable = remove_absolute_path_ranges(first_raw, format);
    let second_comparable = remove_absolute_path_ranges(second_raw, format);
    let identical = first_comparable == second_comparable;
    let allowed = whitelist
        .iter()
        .map(|item| item.kind.as_str())
        .collect::<BTreeSet<_>>();
    let unexpected_differences = if !identical && !allowed.contains("binary-layout") {
        vec!["artifact-bytes".to_owned()]
    } else {
        Vec::new()
    };
    ArtifactReproducibilityReport {
        first_fingerprint: stable_hash(&first_comparable),
        second_fingerprint: stable_hash(&second_comparable),
        identical,
        normalized_fields,
        unexpected_differences,
    }
}

/// 从比较副本中移除已明确记录的绝对路径区间。
///
/// `normalize_artifact` 仍然对原字节执行等长零填充，保留产物长度和字段位置；
/// 比较副本额外移除路径区间，用于处理两次构建中路径长度不同导致的后续字节偏移。
fn remove_absolute_path_ranges(bytes: &[u8], format: ObjectFormat) -> Vec<u8> {
    let (normalized, _) = normalize_artifact(bytes, format);
    let ranges = absolute_path_ranges(bytes);
    if ranges.is_empty() {
        return normalized;
    }
    let mut result = Vec::with_capacity(normalized.len());
    let mut cursor = 0;
    for (start, end) in ranges {
        if start > normalized.len() || end > normalized.len() || start < cursor {
            return normalized;
        }
        result.extend_from_slice(&normalized[cursor..start]);
        cursor = end;
    }
    result.extend_from_slice(&normalized[cursor..]);
    result
}

/// 规范化 LLVM 文本中的路径、时间和符号顺序敏感注释。
pub fn normalize_llvm_text(text: &str) -> String {
    let mut lines = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("; xiao-build-time=") && !trimmed.starts_with("; xiao-temp-path=")
        })
        .map(normalize_line)
        .collect::<Vec<_>>();
    // 只排序专门的符号清单注释；LLVM 指令和声明顺序仍保持原样。
    let mut symbols = lines
        .iter()
        .filter(|line| line.starts_with("; xiao-symbol="))
        .cloned()
        .collect::<Vec<_>>();
    if symbols.len() > 1 {
        let symbol_set = symbols.iter().cloned().collect::<BTreeSet<_>>();
        symbols = symbol_set.into_iter().collect();
        lines.retain(|line| !line.starts_with("; xiao-symbol="));
        lines.extend(symbols);
    }
    let mut normalized = lines.join("\n");
    normalized.push('\n');
    normalized
}

/// 比较两次 LLVM 文本和符号表，构建稳定的白名单报告。
#[must_use]
pub fn compare_reproducible_builds(
    first_llvm: &str,
    second_llvm: &str,
    first_symbols: &[String],
    second_symbols: &[String],
    whitelist: Vec<ReproducibleDifference>,
) -> ReproducibilityReport {
    let first = normalize_llvm_text(first_llvm);
    let second = normalize_llvm_text(second_llvm);
    let mut first_symbols = first_symbols.to_vec();
    let mut second_symbols = second_symbols.to_vec();
    first_symbols.sort();
    second_symbols.sort();
    let llvm_identical = first == second;
    let symbols_identical = first_symbols == second_symbols;
    let allowed = whitelist
        .iter()
        .map(|item| item.kind.as_str())
        .collect::<BTreeSet<_>>();
    let mut unexpected = Vec::new();
    if !llvm_identical
        && !allowed.contains("binary-layout")
        && !allowed.contains("toolchain-version")
        && !allowed.contains("target")
    {
        unexpected.push("llvm-text".to_owned());
    }
    if !symbols_identical
        && !allowed.contains("binary-layout")
        && !allowed.contains("toolchain-version")
        && !allowed.contains("target")
    {
        unexpected.push("symbols".to_owned());
    }
    ReproducibilityReport {
        first_fingerprint: stable_hash(first.as_bytes()),
        second_fingerprint: stable_hash(second.as_bytes()),
        llvm_identical,
        symbols_identical,
        whitelist,
        unexpected_differences: unexpected,
        normalized_fields: vec![
            "xiao-build-time comments".to_owned(),
            "xiao-temp-path comments".to_owned(),
            "absolute path tokens".to_owned(),
            "xiao-symbol comment order".to_owned(),
        ],
    }
}

fn normalize_line(line: &str) -> String {
    let mut normalized = line.replace('\\', "/");
    let tokens = normalized
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for token in tokens {
        if token.contains(":/") || token.starts_with("/tmp/") || token.contains("/Temp/") {
            normalized = normalized.replace(&token, "<path>");
        }
    }
    normalized
}

fn normalize_artifact(bytes: &[u8], format: ObjectFormat) -> (Vec<u8>, Vec<String>) {
    let mut normalized = bytes.to_vec();
    let mut fields = Vec::new();
    if format == ObjectFormat::Coff && normalized.len() >= 0x40 {
        let pe_offset = u32::from_le_bytes([
            normalized[0x3c],
            normalized[0x3d],
            normalized[0x3e],
            normalized[0x3f],
        ]) as usize;
        if pe_offset
            .checked_add(12)
            .is_some_and(|end| end <= normalized.len())
            && normalized.get(pe_offset..pe_offset + 4) == Some(b"PE\0\0")
        {
            normalized[pe_offset + 8..pe_offset + 12].fill(0);
            fields.push("PE.TimeDateStamp".to_owned());
            normalize_pe_debug_timestamps(&mut normalized, pe_offset, &mut fields);
        }
    }
    if format == ObjectFormat::Elf {
        normalize_elf_build_ids(&mut normalized, &mut fields);
    }
    for (start, end) in absolute_path_ranges(&normalized) {
        normalized[start..end].fill(0);
        fields.push(format!("artifact-path@0x{start:x}"));
    }
    (normalized, fields)
}

/// 归一化 ELF 链接器生成的 GNU build-id。
///
/// build-id 是对链接输入的摘要；输入中包含临时对象路径时，重复构建会得到不同
/// 的 note 描述字节。它和 PE 时间戳一样属于链接器元数据，不能作为程序语义差异。
fn normalize_elf_build_ids(bytes: &mut [u8], fields: &mut Vec<String>) {
    let mut offset = 0_usize;
    while offset.saturating_add(16) <= bytes.len() {
        let little = bytes[offset..offset + 4] == [4, 0, 0, 0]
            && bytes[offset + 12..offset + 16] == *b"GNU\0";
        let big = bytes[offset..offset + 4] == [0, 0, 0, 4]
            && bytes[offset + 12..offset + 16] == *b"GNU\0";
        if !little && !big {
            offset += 1;
            continue;
        }
        let description_size = if little {
            u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap())
        } else {
            u32::from_be_bytes(bytes[offset + 4..offset + 8].try_into().unwrap())
        } as usize;
        let note_type = if little {
            u32::from_le_bytes(bytes[offset + 8..offset + 12].try_into().unwrap())
        } else {
            u32::from_be_bytes(bytes[offset + 8..offset + 12].try_into().unwrap())
        };
        if note_type != 3 {
            offset += 1;
            continue;
        }
        let description_start = (offset + 16 + 3) & !3;
        let Some(description_end) = description_start.checked_add(description_size) else {
            break;
        };
        if description_end > bytes.len() {
            break;
        }
        bytes[description_start..description_end].fill(0);
        fields.push(format!("ELF.GNU.BuildId@0x{description_start:x}"));
        offset = description_end;
    }
}

/// 归一化 PE 调试目录中的链接器时间戳。
///
/// 链接器除了 COFF 文件头外，还会在 `IMAGE_DEBUG_DIRECTORY` 写入同一个时间戳；
/// 只清理文件头会让两次真实链接仍然在 POGO/CodeView 目录处产生一字节差异。
fn normalize_pe_debug_timestamps(bytes: &mut [u8], pe_offset: usize, fields: &mut Vec<String>) {
    let Some(coff_offset) = pe_offset.checked_add(4) else {
        return;
    };
    let Some(section_count) = read_le_u16(bytes, coff_offset + 2) else {
        return;
    };
    let Some(optional_size) = read_le_u16(bytes, coff_offset + 16) else {
        return;
    };
    let Some(optional_offset) = coff_offset.checked_add(20) else {
        return;
    };
    let Some(optional_end) = optional_offset.checked_add(optional_size as usize) else {
        return;
    };
    if optional_end > bytes.len() {
        return;
    }
    let Some(optional_magic) = read_le_u16(bytes, optional_offset) else {
        return;
    };
    let (directory_count_offset, directories_offset) = match optional_magic {
        0x10b => (92_usize, 96_usize),
        0x20b => (108_usize, 112_usize),
        _ => return,
    };
    let Some(directory_count) = read_le_u32(bytes, optional_offset + directory_count_offset) else {
        return;
    };
    if directory_count <= 6 || optional_offset + directories_offset + 6 * 8 + 8 > optional_end {
        return;
    }
    let debug_directory = optional_offset + directories_offset + 6 * 8;
    let Some(debug_rva) = read_le_u32(bytes, debug_directory) else {
        return;
    };
    let Some(debug_size) = read_le_u32(bytes, debug_directory + 4) else {
        return;
    };
    if debug_rva == 0 || debug_size < 28 {
        return;
    }
    let section_offset = optional_end;
    let mut sections = Vec::with_capacity(section_count as usize);
    for index in 0..section_count as usize {
        let Some(offset) = section_offset.checked_add(index.saturating_mul(40)) else {
            return;
        };
        let Some(end) = offset.checked_add(40) else {
            return;
        };
        if end > bytes.len() {
            return;
        }
        let Some(virtual_address) = read_le_u32(bytes, offset + 12) else {
            return;
        };
        let Some(raw_size) = read_le_u32(bytes, offset + 16) else {
            return;
        };
        let Some(raw_offset) = read_le_u32(bytes, offset + 20) else {
            return;
        };
        sections.push((virtual_address, raw_size, raw_offset));
    }
    let Some(debug_offset) = pe_rva_to_file_offset(debug_rva, &sections, bytes.len()) else {
        return;
    };
    let count = (debug_size as usize) / 28;
    for index in 0..count {
        let Some(entry_offset) = debug_offset.checked_add(index.saturating_mul(28)) else {
            return;
        };
        let Some(timestamp_end) = entry_offset.checked_add(8) else {
            return;
        };
        if timestamp_end > bytes.len() {
            return;
        }
        bytes[entry_offset + 4..entry_offset + 8].fill(0);
        fields.push(format!(
            "PE.DebugDirectory.TimeDateStamp@0x{entry_offset:x}"
        ));
    }
}

/// 从产物字节中安全读取小端 16 位字段。
fn read_le_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(2)?;
    let value = bytes.get(offset..end)?;
    Some(u16::from_le_bytes([value[0], value[1]]))
}

/// 从产物字节中安全读取小端 32 位字段。
fn read_le_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let value = bytes.get(offset..end)?;
    Some(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

/// 将 PE 调试目录 RVA 映射为文件偏移。
fn pe_rva_to_file_offset(
    rva: u32,
    sections: &[(u32, u32, u32)],
    file_length: usize,
) -> Option<usize> {
    for (virtual_address, raw_size, raw_offset) in sections {
        if rva < *virtual_address || rva - *virtual_address >= *raw_size {
            continue;
        }
        let offset = (*raw_offset as usize).checked_add((rva - *virtual_address) as usize)?;
        if offset < file_length {
            return Some(offset);
        }
    }
    (rva as usize <= file_length).then_some(rva as usize)
}

/// 找出调试信息中常见的 ASCII 绝对路径。
///
/// 归一化只改写路径自身的字节并保持长度不变；路径位置仍被记录在报告中，
/// 因而不会把无法解释的差异静默吞掉。调试信息通常以 NUL 结尾，遇到非打印
/// 字节时也会停止，避免越过二进制字段。
fn absolute_path_ranges(bytes: &[u8]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let is_windows = index + 2 < bytes.len()
            && bytes[index].is_ascii_alphabetic()
            && bytes[index + 1] == b':'
            && matches!(bytes[index + 2], b'/' | b'\\');
        let is_unix = bytes[index..].starts_with(b"/tmp/")
            || bytes[index..].starts_with(b"/home/")
            || bytes[index..].starts_with(b"/Users/");
        if !is_windows && !is_unix {
            index += 1;
            continue;
        }
        let mut end = index;
        while end < bytes.len() {
            let byte = bytes[end];
            if byte == 0 || !(byte.is_ascii_graphic() || byte == b' ') {
                break;
            }
            end += 1;
        }
        if end > index + if is_windows { 3 } else { 1 } {
            ranges.push((index, end));
            index = end;
        } else {
            index += 1;
        }
    }
    ranges
}

fn stable_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("xiao-repro-fnv1a64-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths_times_and_symbol_order() {
        let first = "; xiao-build-time=1\n; xiao-symbol=z\n; xiao-symbol=a\npath C:\\tmp\\x\n";
        let second = "; xiao-build-time=2\n; xiao-symbol=a\n; xiao-symbol=z\npath C:\\tmp\\y\n";
        let report = compare_reproducible_builds(
            first,
            second,
            &["z".to_owned(), "a".to_owned()],
            &["a".to_owned(), "z".to_owned()],
            Vec::new(),
        );
        assert!(report.passed());
        assert!(report.llvm_identical);
        assert!(report.symbols_identical);
    }

    #[test]
    fn binary_layout_whitelist_is_explicit() {
        let report = compare_reproducible_builds(
            "a\n",
            "b\n",
            &[],
            &["x".to_owned()],
            vec![ReproducibleDifference {
                kind: ReproducibleDifferenceKind::BinaryLayout,
                reason: "不同对象格式的节布局".to_owned(),
            }],
        );
        assert!(report.passed());
        assert_eq!(report.whitelist.len(), 1);
    }

    #[test]
    fn normalizes_pe_timestamp_and_records_the_field() {
        let mut first = vec![0_u8; 0x100];
        first[0x3c..0x40].copy_from_slice(&0x80_u32.to_le_bytes());
        first[0x80..0x84].copy_from_slice(b"PE\0\0");
        first[0x88..0x8c].copy_from_slice(&1_u32.to_le_bytes());
        let mut second = first.clone();
        second[0x88..0x8c].copy_from_slice(&2_u32.to_le_bytes());
        let report = compare_artifact_bytes(&first, &second, ObjectFormat::Coff, &[]);
        assert!(report.identical);
        assert_eq!(report.normalized_fields, vec!["PE.TimeDateStamp"]);
    }

    #[test]
    fn normalizes_pe_debug_directory_timestamp() {
        let mut first = vec![0_u8; 0x220];
        first[0x3c..0x40].copy_from_slice(&0x80_u32.to_le_bytes());
        first[0x80..0x84].copy_from_slice(b"PE\0\0");
        first[0x88..0x8c].copy_from_slice(&1_u32.to_le_bytes());
        let coff = 0x84;
        first[coff + 16..coff + 18].copy_from_slice(&0xf0_u16.to_le_bytes());
        let optional = coff + 20;
        first[optional..optional + 2].copy_from_slice(&0x20b_u16.to_le_bytes());
        first[optional + 108..optional + 112].copy_from_slice(&7_u32.to_le_bytes());
        let debug_directory = optional + 112 + 6 * 8;
        first[debug_directory..debug_directory + 4].copy_from_slice(&0x180_u32.to_le_bytes());
        first[debug_directory + 4..debug_directory + 8].copy_from_slice(&28_u32.to_le_bytes());
        first[0x184..0x188].copy_from_slice(&1_u32.to_le_bytes());
        let mut second = first.clone();
        second[0x88..0x8c].copy_from_slice(&2_u32.to_le_bytes());
        second[0x184..0x188].copy_from_slice(&2_u32.to_le_bytes());
        let report = compare_artifact_bytes(&first, &second, ObjectFormat::Coff, &[]);
        assert!(report.identical);
        assert!(
            report
                .normalized_fields
                .iter()
                .any(|field| field.starts_with("PE.DebugDirectory.TimeDateStamp@"))
        );
    }

    #[test]
    fn normalizes_elf_gnu_build_id() {
        let mut first = vec![0_u8; 64];
        first[0..4].copy_from_slice(&4_u32.to_le_bytes());
        first[4..8].copy_from_slice(&4_u32.to_le_bytes());
        first[8..12].copy_from_slice(&3_u32.to_le_bytes());
        first[12..16].copy_from_slice(b"GNU\0");
        first[16..20].copy_from_slice(&[1, 2, 3, 4]);
        let mut second = first.clone();
        second[16..20].copy_from_slice(&[5, 6, 7, 8]);
        let report = compare_artifact_bytes(&first, &second, ObjectFormat::Elf, &[]);
        assert!(report.identical);
        assert_eq!(
            report.normalized_fields,
            vec!["ELF.GNU.BuildId@0x10".to_owned()]
        );
    }

    #[test]
    fn normalizes_debug_paths_in_place_and_records_offsets() {
        let first = b"prefix C:\\build\\xiao\\alpha.cpp\0suffix /home/a/src.cpp\0";
        let second = b"prefix C:\\build\\xiao\\bravo.cpp\0suffix /home/b/src.cpp\0";
        assert_eq!(first.len(), second.len());
        let report = compare_artifact_bytes(first, second, ObjectFormat::Elf, &[]);
        assert!(report.identical);
        assert!(
            report
                .normalized_fields
                .iter()
                .any(|field| field.starts_with("artifact-path@0x"))
        );
        assert_eq!(first.len(), second.len());
    }

    #[test]
    fn records_unix_and_windows_path_fields_separately() {
        let bytes = b"C:\\work\\x.cpp\0/home/user/x.cpp\0/Users/user/y.cpp\0/tmp/z.cpp\0";
        let report = compare_artifact_bytes(bytes, bytes, ObjectFormat::MachO, &[]);
        assert_eq!(
            report
                .normalized_fields
                .iter()
                .filter(|field| field.starts_with("artifact-path@0x"))
                .count(),
            4
        );
    }
}
