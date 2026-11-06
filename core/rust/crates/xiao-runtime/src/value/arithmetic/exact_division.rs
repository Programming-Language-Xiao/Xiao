//! 精确有限十进制除法；不调用上游默认除法上下文。

use super::common::{self, Family, Number};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use bigdecimal::{Signed, Zero};
use num_bigint::BigInt;
use xiao_types::numeric_text::NUMERIC_DIGIT_LIMIT as D;

/// 正系数的二进制gcd，使用已锁定num-bigint的移位/减法，不创建新依赖。
///
/// F-3允许更快gcd；5D+2是逐次欧几里得参考路径的界。本路径每次对奇数
/// 作差后至少除2，故两侧bit长度之和严格下降，所有工作值不超过输入系数。
fn gcd(mut left: BigInt, mut right: BigInt) -> RuntimeResult<BigInt> {
    let bound = common::decimal_digits(&left).max(common::decimal_digits(&right));
    common::check_work("gcd-remainder", bound)?;
    if left.is_zero() {
        return Ok(right);
    }
    if right.is_zero() {
        return Ok(left);
    }
    let step_bound = left.bits() + right.bits();
    let shift = left
        .trailing_zeros()
        .unwrap()
        .min(right.trailing_zeros().unwrap());
    left >>= left.trailing_zeros().unwrap();
    right >>= right.trailing_zeros().unwrap();
    let mut steps = 0_u64;
    while left != right {
        steps += 1;
        if steps > step_bound {
            return Err(common::limit("二进制gcd违反bit长度递减不变量"));
        }
        common::check_work("gcd-remainder", bound)?;
        if left > right {
            left -= &right;
            left >>= left.trailing_zeros().unwrap();
        } else {
            right -= &left;
            right >>= right.trailing_zeros().unwrap();
        }
    }
    Ok(left << shift)
}

#[cfg(test)]
mod gcd_tests {
    use super::*;

    /// 独立u128欧几里得参考，只用于有限测试数据。
    fn reference(mut left: u128, mut right: u128) -> u128 {
        while right != 0 {
            let next = left % right;
            left = right;
            right = next;
        }
        left
    }

    #[test]
    /// 覆盖零、相等、二因子及两侧大小关系，不能只验证互质返回1。
    fn binary_gcd_matches_every_small_pair() {
        for left in 0..128_u128 {
            for right in 0..128_u128 {
                assert_eq!(
                    gcd(BigInt::from(left), BigInt::from(right)).unwrap(),
                    BigInt::from(reference(left, right))
                );
            }
        }
    }

    #[test]
    /// 确定性128位输入与共同大二因子保持精确，无随机外部状态。
    fn binary_gcd_matches_wide_reference_and_common_shifts() {
        let mut state = 0x123456789abcdef_u128;
        for _ in 0..256 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let left = state;
            state = state.rotate_left(47) ^ 0xfedcba9876543210_u128;
            let right = state;
            let expected = BigInt::from(reference(left, right));
            assert_eq!(
                gcd(BigInt::from(left), BigInt::from(right)).unwrap(),
                expected
            );
            assert_eq!(
                gcd(BigInt::from(left) << 257, BigInt::from(right) << 257).unwrap(),
                expected << 257
            );
        }
    }
}

/// 先约分、再精确判定2/5因子，034不得被后续展开预算抢先覆盖。
pub(super) fn divide(left: &Number, right: &Number) -> RuntimeResult<RuntimeValue> {
    if right.is_zero() {
        return Err(RuntimeError::division_by_zero("/"));
    }
    if left.is_zero() {
        return common::finish(BigInt::from(0), 0, Family::Decimal);
    }
    let negative = left.coefficient.sign() != right.coefficient.sign();
    let a = left.coefficient.abs();
    let b = right.coefficient.abs();
    let divisor = gcd(a.clone(), b.clone())?;
    let a = a / &divisor;
    let mut b = b / divisor;
    let twos = b.trailing_zeros().unwrap_or(0);
    b >>= twos;
    let chunk = BigInt::from(7_450_580_596_923_828_125_u64); // 5^27，避免逐个剥除的大量长除。
    let five = BigInt::from(5);
    let one = BigInt::from(1);
    let mut fives = 0_u64;
    while b != one && (&b % &chunk).is_zero() {
        b /= &chunk;
        fives += 27;
    }
    while b != one && (&b % &five).is_zero() {
        b /= &five;
        fives += 1;
    }
    if b != one {
        return Err(RuntimeError::non_terminating_division());
    }
    if twos + fives > (4 * D) as u64 {
        return Err(common::limit("有限因子逻辑工作量超过上界"));
    }
    let k = twos.max(fives);
    let difference = twos.abs_diff(fives);
    common::check_work(
        "finite-division-coefficient",
        common::decimal_digits(&a) + difference as usize,
    )?;
    let mut coefficient = if difference == 0 {
        a
    } else {
        common::check_work("finite-division-factor", difference as usize + 1)?;
        let factor = BigInt::from(if twos > fives { 5 } else { 2 }).pow(difference as u32);
        let factor = common::checked_value("finite-division-factor", factor)?;
        common::checked_value("finite-division-coefficient", a * factor)?
    };
    if negative {
        coefficient = -coefficient;
    }
    let base_scale = left.scale - right.scale;
    let normalized = common::normalize(coefficient, base_scale + k as i32)?;
    let display = base_scale.max(normalized.scale);
    common::check_scales(i64::from(display), i64::from(normalized.scale))?;
    let padding = (display - normalized.scale) as u32;
    if normalized.digits + padding as usize > D {
        return Err(common::limit("精确除法的显示系数超过32768位"));
    }
    let coefficient = common::align(&normalized.coefficient, normalized.digits, padding)?;
    common::finish(coefficient, display, Family::Decimal)
}
