//! LLVM 文本和构建指纹共用的纯文本辅助。

/// 转义 LLVM 字符串字面量中的反斜杠和引号。
pub(crate) fn escape_llvm(text: &str) -> String {
    text.replace('\\', "\\5C").replace('"', "\\22")
}

/// 计算用于构建指纹的稳定 FNV-1a 文本。
pub fn stable_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
