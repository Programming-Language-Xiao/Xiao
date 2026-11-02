//! 受控测量的几何校准：量化时钟的零读数不能解释成一纳秒。

/// 每轮至多放大16倍，测得真实批次后再决定下一轮，避免极短读数造成数千万次过冲。
pub(super) fn next_batch(repeats: usize, seconds: f64) -> usize {
    let factor = if seconds == 0.0 {
        16
    } else {
        (0.12 / seconds).ceil().clamp(2.0, 16.0) as usize
    };
    repeats.checked_mul(factor).expect("校准批次溢出")
}

#[cfg(test)]
/// 量化时钟的校准稳定性回归。
mod tests {
    use super::next_batch;

    #[test]
    /// 250ns调用配1us量化时钟会先读到零；旧的一纳秒兜底会放大至约30秒一批。
    fn quantized_clock_converges_without_an_oversized_batch() {
        let mut repeats = 1;
        for _ in 0..32 {
            let elapsed = ((repeats as f64 * 0.000_000_25) / 0.000_001).floor() * 0.000_001;
            if elapsed >= 0.1 {
                assert!(elapsed <= 0.25, "校准过冲：{repeats}次，{elapsed}秒");
                return;
            }
            repeats = next_batch(repeats, elapsed);
        }
        panic!("量化时钟校准未收敛");
    }
}
