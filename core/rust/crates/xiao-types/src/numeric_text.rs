//! B2-A 共用的十进制文本接纳与无分配数值身份。
//!
//! 类型层只识别文本，不依赖 Runtime 或大数对象。可失败入口先调用
//! [`validate_numeric_text`]；`Eq`/`Hash` 只为已校验文本建立借用视图。

use std::hash::{Hash, Hasher};

/// 输入数值文本的 UTF-8 字节上限。
pub const NUMERIC_TEXT_LIMIT: usize = 65_536;
/// 输入及最终显示系数的十进制位数上限。
pub const NUMERIC_DIGIT_LIMIT: usize = 32_768;
/// 显式整数工作值的十进制位数上限。
pub const NUMERIC_WORK_DIGIT_LIMIT: usize = 65_536;
/// 显示 scale、规范 scale 和显式指数的绝对值上限。
pub const NUMERIC_SCALE_LIMIT: i32 = 100_000;

/// 既有数值文本标签的语法类别。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericTextKind {
    /// 可带符号的 ASCII 十进制整数；源码分隔符由既有降低步骤处理。
    Integer,
    /// F-11 十进制语法，包含小数点、指数和相邻数字间的下划线。
    Decimal,
}

/// 可失败接纳入口的两种稳定失败原因。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericTextError {
    /// 原文长度合格，但不符合相应的完整语法。
    InvalidSyntax,
    /// 原文、系数、显式指数或任一 scale 超过冻结预算。
    LimitExceeded,
}

/// 数值文本的借用解析结果；不保存或分配大整数。
#[derive(Clone, Copy, Debug)]
pub struct NumericTextView<'a> {
    /// 首个非零数字至尾数结束，允许包含小数点或分隔符。
    coefficient: &'a [u8],
    /// 去前导零、保留显示尾零的系数位数；零为一位。
    digits: usize,
    /// 显示系数的尾零数；零固定为零。
    trailing_zeros: usize,
    /// 非零值的符号；零不带负号。
    negative: bool,
    /// 显示 scale；零固定为零。
    scale: i32,
    /// 原文显式指数的绝对值，饱和到预算加一。
    exponent_magnitude: i32,
}

impl<'a> NumericTextView<'a> {
    /// 判断数值是否为零。
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.coefficient == b"0"
    }

    /// 返回已归一零符号后的负号标记。
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.negative
    }

    /// 返回显示系数位数，保留有意义的尾零。
    #[must_use]
    pub const fn digit_count(self) -> usize {
        self.digits
    }

    /// 返回显示 scale。
    #[must_use]
    pub const fn display_scale(self) -> i32 {
        self.scale
    }

    /// 返回去尾零后的规范 scale，不展开虚拟补零。
    #[must_use]
    pub const fn canonical_scale(self) -> i32 {
        self.scale - self.trailing_zeros as i32
    }

    /// 依次借读显示系数数字，跳过小数点及分隔符。
    pub fn coefficient_digits(self) -> impl Iterator<Item = u8> + 'a {
        self.coefficient.iter().copied().filter(u8::is_ascii_digit)
    }

    /// 依次借读规范系数数字；不复制或展开任何系数。
    pub fn canonical_digits(self) -> impl Iterator<Item = u8> + 'a {
        self.coefficient_digits()
            .take(self.digits - self.trailing_zeros)
    }
}

impl PartialEq for NumericTextView<'_> {
    /// 数值身份忽略显示尾零与文本指数写法。
    fn eq(&self, other: &Self) -> bool {
        self.negative == other.negative
            && self.canonical_scale() == other.canonical_scale()
            && self.canonical_digits().eq(other.canonical_digits())
    }
}

impl Eq for NumericTextView<'_> {}

impl Hash for NumericTextView<'_> {
    /// 哈希规范符号、scale及数字序列；整个路径不分配。
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.negative.hash(state);
        self.canonical_scale().hash(state);
        (self.digits - self.trailing_zeros).hash(state);
        for digit in self.canonical_digits() {
            state.write_u8(digit);
        }
    }
}

/// 按 F-12 检查原文预算、完整语法，然后检查数值预算。
///
/// 不在尾部语法检查完成前抢报系数或指数超限；零仍检查原文和显式指数。
pub fn validate_numeric_text(
    text: &str,
    kind: NumericTextKind,
) -> Result<NumericTextView<'_>, NumericTextError> {
    if text.len() > NUMERIC_TEXT_LIMIT {
        return Err(NumericTextError::LimitExceeded);
    }
    validate_syntax(text.as_bytes(), kind)?;
    let view = identity_of_validated_numeric_text(text);
    if view.exponent_magnitude > NUMERIC_SCALE_LIMIT
        || view.digits > NUMERIC_DIGIT_LIMIT
        || view.scale.abs() > NUMERIC_SCALE_LIMIT
        || view.canonical_scale().abs() > NUMERIC_SCALE_LIMIT
    {
        return Err(NumericTextError::LimitExceeded);
    }
    Ok(view)
}

/// 为受信任、已经通过接纳入口的数值文本建立只读身份视图。
///
/// 此函数不重新做可失败接纳，不创建错误、不构造大数、不退回文本比较。
/// 调用方必须先在数值构造/恢复入口建立合法性与预算不变量。
#[must_use]
pub fn identity_of_validated_numeric_text(text: &str) -> NumericTextView<'_> {
    let bytes = text.as_bytes();
    let negative = bytes.first() == Some(&b'-');
    let end = bytes
        .iter()
        .position(|byte| matches!(byte, b'e' | b'E'))
        .unwrap_or(bytes.len());
    let mut first = None;
    let mut digits = 0;
    let mut trailing_zeros = 0;
    let mut after_point = false;
    let mut fractional_digits = 0_i32;
    for (index, &byte) in bytes[..end].iter().enumerate() {
        if byte == b'.' {
            after_point = true;
        } else if byte.is_ascii_digit() {
            fractional_digits += i32::from(after_point);
            if byte != b'0' {
                first.get_or_insert(index);
                trailing_zeros = 0;
            } else if first.is_some() {
                trailing_zeros += 1;
            }
            digits += usize::from(first.is_some());
        }
    }
    let mut exponent_magnitude = 0_i32;
    let mut exponent_negative = false;
    for &byte in bytes.get(end + 1..).unwrap_or_default() {
        if byte == b'-' {
            exponent_negative = true;
        } else if byte.is_ascii_digit() {
            exponent_magnitude =
                (exponent_magnitude * 10 + i32::from(byte - b'0')).min(NUMERIC_SCALE_LIMIT + 1);
        }
    }
    let exponent = if exponent_negative {
        -exponent_magnitude
    } else {
        exponent_magnitude
    };
    match first {
        Some(first) => NumericTextView {
            coefficient: &bytes[first..end],
            digits,
            trailing_zeros,
            negative,
            scale: fractional_digits - exponent,
            exponent_magnitude,
        },
        None => NumericTextView {
            coefficient: b"0",
            digits: 1,
            trailing_zeros: 0,
            negative: false,
            scale: 0,
            exponent_magnitude,
        },
    }
}

/// 完整扫描同一数字段，要求下划线严格位于相邻数字之间。
fn scan_digits(
    bytes: &[u8],
    index: &mut usize,
    separators: bool,
) -> Result<usize, NumericTextError> {
    let mut count = 0;
    while let Some(&byte) = bytes.get(*index) {
        if byte.is_ascii_digit() {
            count += 1;
            *index += 1;
        } else if byte == b'_' && separators {
            if count == 0 || !bytes.get(*index + 1).is_some_and(u8::is_ascii_digit) {
                return Err(NumericTextError::InvalidSyntax);
            }
            *index += 1;
        } else {
            break;
        }
    }
    Ok(count)
}

/// 校验纯ASCII语法；任何数值预算判断都在本函数成功后执行。
fn validate_syntax(bytes: &[u8], kind: NumericTextKind) -> Result<(), NumericTextError> {
    let mut index = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let decimal = kind == NumericTextKind::Decimal;
    let mut digits = scan_digits(bytes, &mut index, decimal)?;
    if decimal && bytes.get(index) == Some(&b'.') {
        index += 1;
        digits += scan_digits(bytes, &mut index, true)?;
    }
    if digits == 0 {
        return Err(NumericTextError::InvalidSyntax);
    }
    if decimal && matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        index += usize::from(matches!(bytes.get(index), Some(b'+' | b'-')));
        if scan_digits(bytes, &mut index, true)? == 0 {
            return Err(NumericTextError::InvalidSyntax);
        }
    }
    if index == bytes.len() {
        Ok(())
    } else {
        Err(NumericTextError::InvalidSyntax)
    }
}

#[cfg(test)]
#[path = "numeric_text_tests.rs"]
mod tests;
