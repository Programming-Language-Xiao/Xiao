//! 动态降低器使用的文本解析、转义和安全性辅助。

use xiao_ir::{IrExpression, IrSpan, IrType};

use crate::error::{CodegenError, Result};

/// 解析带下划线的 64 位整数文本。
pub(super) fn parse_i64(text: &str, span: IrSpan) -> Result<String> {
    text.replace('_', "")
        .parse::<i64>()
        .map(|value| value.to_string())
        .map_err(|_| CodegenError::InvalidIr {
            message: format!("整数无法编码（{}..{}）", span.start, span.end),
        })
}

/// 解析带下划线的 32 位整数文本。
pub(super) fn parse_i32(text: &str, span: IrSpan) -> Result<String> {
    text.replace('_', "")
        .parse::<i32>()
        .map(|value| value.to_string())
        .map_err(|_| CodegenError::InvalidIr {
            message: format!("短整数无法编码（{}..{}）", span.start, span.end),
        })
}

/// 解析有限浮点文本。
pub(super) fn format_float(text: &str, span: IrSpan) -> Result<String> {
    let value = text
        .replace('_', "")
        .parse::<f64>()
        .map_err(|_| CodegenError::InvalidIr {
            message: format!("浮点无法编码（{}..{}）", span.start, span.end),
        })?;
    if !value.is_finite() {
        return Err(CodegenError::InvalidIr {
            message: format!("浮点必须有限（{}..{}）", span.start, span.end),
        });
    }
    Ok(format!("{value:.17e}"))
}

/// 判断动态路径是否可以安全透传一个显式转换。
///
/// 动态转换的运行时检查仍由字节码侧和后续 N0-C 负责；这里仅接受类型层已经证明为
/// identity 的转换，避免把 `str as bool` 或数值转换误当成位布局相同的值。
pub(super) fn is_identity_cast(inner: &IrExpression, target: &str, result_type: &IrType) -> bool {
    matches!(
        (&inner.ty, result_type),
        (
            IrType::Scalar { name: source },
            IrType::Scalar { name: result }
        ) if source == target && result == target
    )
}

/// 去除 Xiao 字符串字面量的外层引号。
pub(super) fn unquote(text: &str) -> Option<String> {
    if !text.starts_with('"') || !text.ends_with('"') {
        return None;
    }
    Some(xiao_types::decode_string_literal(text))
}

/// 转义 LLVM C 字符串常量中的字节。
pub(super) fn escape_bytes(bytes: &[u8]) -> String {
    let mut output = String::new();
    for byte in bytes {
        if (0x20..=0x7e).contains(byte) && *byte != b'"' && *byte != b'\\' {
            output.push(*byte as char);
        } else {
            output.push_str(&format!("\\{byte:02X}"));
        }
    }
    output
}
