//! C1 随机选择的确定性基础设施。
//!
//! 类型检查器只记录随机操作和种子；本模块提供后续 Runtime 可以直接
//! 使用的无状态抽样算法，并允许测试注入自己的随机源。这里不依赖操作
//! 系统时间，也不把随机状态写入类型对象。

use std::fmt::{self, Display, Formatter};

use xiao_syntax::RandomMode;

/// 选择器随机源的最小接口。
pub trait RandomSource {
    /// 返回下一个均匀分布的无符号 64 位值。
    fn next_u64(&mut self) -> u64;
}

/// 一个跨平台、可复现的轻量伪随机源。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeededRandom {
    state: u64,
}

impl SeededRandom {
    /// 使用非负整数语义种子创建随机源。
    #[must_use]
    pub const fn new(seed: u128) -> Self {
        let folded = (seed as u64) ^ ((seed >> 64) as u64);
        Self {
            state: if folded == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                folded
            },
        }
    }

    /// 返回当前内部状态，用于测试和诊断，不作为语言级随机 API。
    #[must_use]
    pub const fn state(&self) -> u64 {
        self.state
    }

    /// 重新设置随机源状态。
    pub const fn reseed(&mut self, seed: u128) {
        *self = Self::new(seed);
    }

    /// 返回下一个伪随机值的便捷方法。
    pub fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.state = value;
        value.wrapping_mul(2_685_821_657_736_338_717)
    }
}

impl RandomSource for SeededRandom {
    /// 使用 xorshift64* 产生下一个值。
    fn next_u64(&mut self) -> u64 {
        Self::next_u64(self)
    }
}

/// 随机抽样失败的结构化原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RandomSelectionError {
    /// 抽取数量为负数或无法表示为无符号数量。
    InvalidCount,
    /// 无放回抽取数量超过候选元素数。
    WithoutReplacementTooMany { count: usize, available: usize },
    /// 空候选集无法完成正数次抽取。
    EmptySource,
}

impl Display for RandomSelectionError {
    /// 输出稳定的开发者诊断文本。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCount => formatter.write_str("random selection count is invalid"),
            Self::WithoutReplacementTooMany { count, available } => write!(
                formatter,
                "cannot draw {count} values without replacement from {available} values"
            ),
            Self::EmptySource => formatter.write_str("cannot draw from an empty source"),
        }
    }
}

impl std::error::Error for RandomSelectionError {}

/// 使用注入的随机源抽取候选位置。
pub fn sample_indices<R: RandomSource>(
    source_len: usize,
    count: usize,
    mode: RandomMode,
    random: &mut R,
) -> Result<Vec<usize>, RandomSelectionError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if source_len == 0 {
        return Err(RandomSelectionError::EmptySource);
    }
    if mode == RandomMode::WithoutReplacement && count > source_len {
        return Err(RandomSelectionError::WithoutReplacementTooMany {
            count,
            available: source_len,
        });
    }
    match mode {
        RandomMode::WithReplacement => Ok((0..count)
            .map(|_| bounded_index(random.next_u64(), source_len))
            .collect()),
        RandomMode::WithoutReplacement => {
            let mut candidates = (0..source_len).collect::<Vec<_>>();
            let mut output = Vec::with_capacity(count);
            for index in 0..count {
                let remaining = source_len - index;
                let offset = bounded_index(random.next_u64(), remaining);
                output.push(candidates.swap_remove(offset));
            }
            Ok(output)
        }
    }
}

/// 将随机源输出映射到 `[0, upper)`；调用方保证 `upper > 0`。
fn bounded_index(value: u64, upper: usize) -> usize {
    (value % upper as u64) as usize
}

#[cfg(test)]
/// 覆盖随机源复现性、两种抽样模式和边界错误。
mod tests {
    use super::{RandomSelectionError, SeededRandom, sample_indices};
    use xiao_syntax::RandomMode;

    #[test]
    /// 相同种子应产生相同状态序列。
    fn seeded_source_is_reproducible() {
        let mut first = SeededRandom::new(7);
        let mut second = SeededRandom::new(7);
        assert_eq!(first.next_u64(), second.next_u64());
        assert_eq!(first.next_u64(), second.next_u64());
    }

    #[test]
    /// 无放回不重复，放回可以重复且允许超过候选数量。
    fn samples_follow_modes() {
        let mut random = SeededRandom::new(11);
        let without =
            sample_indices(4, 4, RandomMode::WithoutReplacement, &mut random).expect("数量合法");
        let mut sorted = without.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![0, 1, 2, 3]);
        let with =
            sample_indices(2, 8, RandomMode::WithReplacement, &mut random).expect("放回允许超量");
        assert_eq!(with.len(), 8);
        assert!(with.iter().all(|value| *value < 2));
    }

    #[test]
    /// 空来源和无放回超量必须区分错误类别。
    fn reports_sampling_boundaries() {
        let mut random = SeededRandom::new(1);
        assert_eq!(
            sample_indices(0, 1, RandomMode::WithReplacement, &mut random),
            Err(RandomSelectionError::EmptySource)
        );
        assert!(matches!(
            sample_indices(2, 3, RandomMode::WithoutReplacement, &mut random),
            Err(RandomSelectionError::WithoutReplacementTooMany { .. })
        ));
        assert_eq!(
            sample_indices(0, 0, RandomMode::WithoutReplacement, &mut random),
            Ok(Vec::new())
        );
    }
}
