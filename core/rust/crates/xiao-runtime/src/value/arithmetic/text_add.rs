//! 已校验十进制系数的线性加减；按冻结对齐模型检查W，再按实际结果检查D/H。

use super::{
    Operation,
    common::{self, Family},
};
use crate::{
    errors::RuntimeResult,
    value::{RuntimeValue, numeric},
};
use std::{borrow::Cow, cmp::Ordering};
use xiao_types::numeric_text::{NumericTextKind, NumericTextView};

/// 只接管整数/文本十进制加减；二进制来源仍通过原精确提升与成熟大数路径。
pub(super) fn binary(
    op: Operation,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> Option<RuntimeResult<RuntimeValue>> {
    if !matches!(op, Operation::Add | Operation::Subtract)
        || matches!(left, RuntimeValue::Float(_) | RuntimeValue::Sfloat(_))
        || matches!(right, RuntimeValue::Float(_) | RuntimeValue::Sfloat(_))
    {
        return None;
    }
    Some(add_subtract(op, left, right))
}

/// 固定宽整数只有有界十进制表示，文本数值借读已有内容，不建立共享缓存。
fn text(value: &RuntimeValue) -> (Cow<'_, str>, NumericTextKind) {
    match value {
        RuntimeValue::Lint(text) => (Cow::Borrowed(text), NumericTextKind::Integer),
        RuntimeValue::Lfloat(text) => (Cow::Borrowed(text), NumericTextKind::Decimal),
        RuntimeValue::Int(value) => (Cow::Owned(value.to_string()), NumericTextKind::Integer),
        RuntimeValue::Sint(value) => (Cow::Owned(value.to_string()), NumericTextKind::Integer),
        _ => unreachable!("调用前已排除非数值与二进制来源"),
    }
}

/// 缓冲区只按已接纳的D或已预检的W申请，不按指数展开无界字符串。
fn buffer(capacity: usize) -> RuntimeResult<Vec<u8>> {
    let mut value = Vec::new();
    value
        .try_reserve_exact(capacity)
        .map_err(|_| numeric::allocation_error())?;
    Ok(value)
}

/// 显示系数去前导零、保留尾零，点和分隔符只在输入借用视图中跳过。
fn digits(view: NumericTextView<'_>) -> RuntimeResult<Vec<u8>> {
    let mut value = buffer(view.digit_count())?;
    value.extend(view.coefficient_digits());
    Ok(value)
}

/// 从最低位借读；对齐零保持虚拟形式，不创建巨大10幂。
fn digit(value: &[u8], zeros: usize, position: usize) -> u8 {
    if position < zeros {
        return 0;
    }
    let offset = position - zeros;
    if offset >= value.len() {
        0
    } else {
        value[value.len() - offset - 1] - b'0'
    }
}

/// 比较对齐后的绝对系数，不把显示文本当作数值字符串排序。
fn magnitude(left: &[u8], lz: usize, right: &[u8], rz: usize) -> Ordering {
    (left.len() + lz).cmp(&(right.len() + rz)).then_with(|| {
        left.iter()
            .copied()
            .chain(std::iter::repeat_n(b'0', lz))
            .cmp(right.iter().copied().chain(std::iter::repeat_n(b'0', rz)))
    })
}

/// 逐位进位/借位，输入不变；减法调用方先保证左绝对值不小于右。
fn combine(left: &[u8], lz: usize, right: &[u8], rz: usize, add: bool) -> RuntimeResult<Vec<u8>> {
    let length = (left.len() + lz).max(right.len() + rz);
    let mut result = buffer(length + usize::from(add))?;
    let mut carry = 0_i16;
    for position in 0..length {
        let a = i16::from(digit(left, lz, position));
        let b = i16::from(digit(right, rz, position));
        let next = if add { a + b + carry } else { a - b - carry };
        if add {
            result.push(b'0' + (next % 10) as u8);
            carry = next / 10;
        } else {
            result.push(
                b'0' + if next < 0 {
                    (next + 10) as u8
                } else {
                    next as u8
                },
            );
            carry = i16::from(next < 0);
        }
    }
    if add && carry != 0 {
        result.push(b'0' + carry as u8);
    }
    debug_assert!(add || carry == 0);
    while result.last() == Some(&b'0') {
        result.pop();
    }
    result.reverse();
    Ok(result)
}

/// 按参数顺序接纳输入，零不抬高scale；工作上界L+1与实际结果位数分别检查。
fn add_subtract(
    op: Operation,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    common::validate_input(left)?;
    common::validate_input(right)?;
    let (left_text, lk) = text(left);
    let (right_text, rk) = text(right);
    let left = numeric::validated_text(&left_text, lk)?;
    let right = numeric::validated_text(&right_text, rk)?;
    let family = if lk == NumericTextKind::Decimal || rk == NumericTextKind::Decimal {
        Family::Decimal
    } else {
        Family::Integer
    };
    let ln = left.is_negative();
    let rn = right.is_negative() ^ (op == Operation::Subtract);
    if left.is_zero() {
        return common::finish_digits(&digits(right)?, rn, right.display_scale(), family);
    }
    if right.is_zero() {
        return common::finish_digits(&digits(left)?, ln, left.display_scale(), family);
    }
    let scale = left.display_scale().max(right.display_scale());
    let lz = (scale - left.display_scale()) as usize;
    let rz = (scale - right.display_scale()) as usize;
    let length = (left.digit_count() + lz).max(right.digit_count() + rz);
    let label = if family == Family::Integer {
        "integer-add-subtract"
    } else {
        "decimal-add-subtract"
    };
    common::check_work(label, length + 1)?;
    let left = digits(left)?;
    let right = digits(right)?;
    let (result, negative) = if ln == rn {
        (combine(&left, lz, &right, rz, true)?, ln)
    } else {
        match magnitude(&left, lz, &right, rz) {
            Ordering::Greater => (combine(&left, lz, &right, rz, false)?, ln),
            Ordering::Less => (combine(&right, rz, &left, lz, false)?, rn),
            Ordering::Equal => (Vec::new(), false),
        }
    };
    common::checked_digits(label, result.len().max(1))?;
    common::finish_digits(&result, negative, scale, family)
}

/// 独立有理数与成熟大数参考，验收进借位和资源门槛。
#[cfg(test)]
#[path = "text_add_tests.rs"]
mod tests;
