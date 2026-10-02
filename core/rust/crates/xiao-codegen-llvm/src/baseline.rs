//! 15B 五维性能基线与噪声阈值模型。

use std::fmt::{Display, Formatter};

/// 一次固定条件下的原生性能样本。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PerformanceSample {
    /// 编译耗时（毫秒）。
    pub compile_ms: f64,
    /// 进程启动耗时（毫秒）。
    pub startup_ms: f64,
    /// 程序运行耗时（毫秒）。
    pub runtime_ms: f64,
    /// 进程峰值内存（字节）。
    pub peak_memory_bytes: u64,
    /// 最终产物体积（字节）。
    pub artifact_bytes: u64,
}

/// 固定硬件/容器条件的描述；不会写入产物指纹。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaselineCondition {
    /// 目标三元组。
    pub target: String,
    /// 工具链版本摘要。
    pub toolchain: String,
    /// 固定硬件或容器标签。
    pub environment: String,
    /// 测量重复次数。
    pub repetitions: usize,
}

/// 性能基线报告。
#[derive(Clone, Debug, PartialEq)]
pub struct PerformanceBaseline {
    /// 固定条件。
    pub condition: BaselineCondition,
    /// 噪声阈值百分比。
    pub noise_threshold_percent: f64,
    /// 原始样本；不进入构建产物。
    pub samples: Vec<PerformanceSample>,
    /// 各维度中位数。
    pub median: PerformanceSample,
    /// 各维度最大相对离散度百分比。
    pub spread_percent: PerformanceSpread,
}

/// 五个维度的相对离散度百分比。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PerformanceSpread {
    /// 编译耗时离散度。
    pub compile_ms: f64,
    /// 启动耗时离散度。
    pub startup_ms: f64,
    /// 运行耗时离散度。
    pub runtime_ms: f64,
    /// 峰值内存离散度。
    pub peak_memory_percent: f64,
    /// 产物体积离散度。
    pub artifact_percent: f64,
}

/// 性能基线输入错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BaselineError {
    /// 重复次数不合法。
    InvalidRepetitions,
    /// 样本数量与重复次数不一致。
    SampleCount {
        /// 期望样本数量。
        expected: usize,
        /// 实际样本数量。
        actual: usize,
    },
    /// 噪声阈值不是有限非负值。
    InvalidNoiseThreshold,
}

impl Display for BaselineError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRepetitions => formatter.write_str("性能基线重复次数必须大于零"),
            Self::SampleCount { expected, actual } => {
                write!(
                    formatter,
                    "性能样本数量不匹配：需要 {expected}，收到 {actual}"
                )
            }
            Self::InvalidNoiseThreshold => formatter.write_str("噪声阈值必须是有限非负值"),
        }
    }
}

impl std::error::Error for BaselineError {}

/// 从固定条件和样本建立五维基线。
pub fn measure_baseline(
    condition: BaselineCondition,
    samples: Vec<PerformanceSample>,
    noise_threshold_percent: f64,
) -> Result<PerformanceBaseline, BaselineError> {
    if condition.repetitions == 0 {
        return Err(BaselineError::InvalidRepetitions);
    }
    if condition.repetitions != samples.len() {
        return Err(BaselineError::SampleCount {
            expected: condition.repetitions,
            actual: samples.len(),
        });
    }
    if !noise_threshold_percent.is_finite() || noise_threshold_percent < 0.0 {
        return Err(BaselineError::InvalidNoiseThreshold);
    }
    let median = PerformanceSample {
        compile_ms: median(samples.iter().map(|sample| sample.compile_ms)),
        startup_ms: median(samples.iter().map(|sample| sample.startup_ms)),
        runtime_ms: median(samples.iter().map(|sample| sample.runtime_ms)),
        peak_memory_bytes: median_u64(samples.iter().map(|sample| sample.peak_memory_bytes)),
        artifact_bytes: median_u64(samples.iter().map(|sample| sample.artifact_bytes)),
    };
    let spread_percent = PerformanceSpread {
        compile_ms: spread(
            samples.iter().map(|sample| sample.compile_ms),
            median.compile_ms,
        ),
        startup_ms: spread(
            samples.iter().map(|sample| sample.startup_ms),
            median.startup_ms,
        ),
        runtime_ms: spread(
            samples.iter().map(|sample| sample.runtime_ms),
            median.runtime_ms,
        ),
        peak_memory_percent: spread_u64(
            samples.iter().map(|sample| sample.peak_memory_bytes),
            median.peak_memory_bytes,
        ),
        artifact_percent: spread_u64(
            samples.iter().map(|sample| sample.artifact_bytes),
            median.artifact_bytes,
        ),
    };
    Ok(PerformanceBaseline {
        condition,
        noise_threshold_percent,
        samples,
        median,
        spread_percent,
    })
}

fn median<I>(values: I) -> f64
where
    I: Iterator<Item = f64>,
{
    let mut values = values.collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn median_u64<I>(values: I) -> u64
where
    I: Iterator<Item = u64>,
{
    let mut values = values.collect::<Vec<_>>();
    values.sort_unstable();
    values[values.len() / 2]
}

fn spread<I>(values: I, center: f64) -> f64
where
    I: Iterator<Item = f64>,
{
    let max = values
        .map(|value| (value - center).abs())
        .fold(0.0, f64::max);
    if center == 0.0 {
        max
    } else {
        max / center * 100.0
    }
}

fn spread_u64<I>(values: I, center: u64) -> f64
where
    I: Iterator<Item = u64>,
{
    let max = values
        .map(|value| value.abs_diff(center) as f64)
        .fold(0.0, f64::max);
    if center == 0 {
        max
    } else {
        max / center as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn condition() -> BaselineCondition {
        BaselineCondition {
            target: "x86_64-unknown-linux-gnu".to_owned(),
            toolchain: "clang-21".to_owned(),
            environment: "ci-fixed-linux".to_owned(),
            repetitions: 3,
        }
    }

    #[test]
    fn median_and_noise_are_stable() {
        let baseline = measure_baseline(
            condition(),
            vec![
                PerformanceSample {
                    compile_ms: 10.0,
                    startup_ms: 2.0,
                    runtime_ms: 5.0,
                    peak_memory_bytes: 100,
                    artifact_bytes: 200,
                },
                PerformanceSample {
                    compile_ms: 12.0,
                    startup_ms: 2.5,
                    runtime_ms: 5.5,
                    peak_memory_bytes: 110,
                    artifact_bytes: 210,
                },
                PerformanceSample {
                    compile_ms: 11.0,
                    startup_ms: 2.2,
                    runtime_ms: 5.2,
                    peak_memory_bytes: 105,
                    artifact_bytes: 205,
                },
            ],
            10.0,
        )
        .expect("基线");
        assert_eq!(baseline.median.compile_ms, 11.0);
        assert!(baseline.spread_percent.runtime_ms > 0.0);
        assert_eq!(baseline.condition.repetitions, 3);
    }

    #[test]
    fn rejects_uncontrolled_sample_shapes() {
        let error = measure_baseline(condition(), Vec::new(), 10.0).expect_err("样本数量");
        assert!(matches!(error, BaselineError::SampleCount { .. }));
    }
}
