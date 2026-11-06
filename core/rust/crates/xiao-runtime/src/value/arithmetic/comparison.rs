//! 数值系数与虚拟补零比较；不展开scale差，不比较原始显示文本。

use super::common::Number;
use bigdecimal::{Signed, Zero};
use num_bigint::BigInt;
use std::cmp::Ordering;

/// 非零系数按十进制位权逐位比较；补零只存在于迭代器中。
pub(super) fn magnitude(
    left: &BigInt,
    left_zeros: u32,
    right: &BigInt,
    right_zeros: u32,
) -> Ordering {
    let left_text = left.to_str_radix(10);
    let right_text = right.to_str_radix(10);
    let left = left_text.trim_start_matches('-').as_bytes();
    let right = right_text.trim_start_matches('-').as_bytes();
    let lengths = (left.len() + left_zeros as usize).cmp(&(right.len() + right_zeros as usize));
    if lengths != Ordering::Equal {
        return lengths;
    }
    left.iter()
        .copied()
        .chain(std::iter::repeat_n(b'0', left_zeros as usize))
        .cmp(
            right
                .iter()
                .copied()
                .chain(std::iter::repeat_n(b'0', right_zeros as usize)),
        )
}

/// 同一十进制位权下进行有符号数值比较，包括零和异号快路径。
pub(super) fn numbers(left: &Number, right: &Number) -> Ordering {
    if left.is_zero() || right.is_zero() {
        return left.coefficient.signum().cmp(&right.coefficient.signum());
    }
    if left.coefficient.is_negative() != right.coefficient.is_negative() {
        return left.coefficient.signum().cmp(&right.coefficient.signum());
    }
    let scale = left.scale.max(right.scale);
    let result = magnitude(
        &left.coefficient,
        (scale - left.scale) as u32,
        &right.coefficient,
        (scale - right.scale) as u32,
    );
    if left.coefficient.is_negative() {
        result.reverse()
    } else {
        result
    }
}

/// floor(log10(A/B))只可能是dA-dB或其减一，决策完全精确。
pub(super) fn ratio_order(a: &BigInt, da: usize, b: &BigInt, db: usize) -> i32 {
    debug_assert!(!a.is_zero() && !b.is_zero());
    let difference = da as i32 - db as i32;
    let order = magnitude(a, (-difference).max(0) as u32, b, difference.max(0) as u32);
    difference - i32::from(order == Ordering::Less)
}
