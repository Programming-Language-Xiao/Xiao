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
    let (first, mut normalized_fields) = normalize_artifact(first, format);
    let (second, second_fields) = normalize_artifact(second, format);
    normalized_fields.extend(second_fields);
    normalized_fields.sort();
    normalized_fields.dedup();
    let identical = first == second;
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
        first_fingerprint: stable_hash(&first),
        second_fingerprint: stable_hash(&second),
        identical,
        normalized_fields,
        unexpected_differences,
    }
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
        }
    }
    (normalized, fields)
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
}
