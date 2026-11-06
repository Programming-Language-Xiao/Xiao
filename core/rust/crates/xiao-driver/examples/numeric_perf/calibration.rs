//! F-3校准的独立停止条件与增长上限；零读数不能伪造成1ns。

/// 所有批次的总重复上限。
pub const MAX_REPEAT: usize = 1 << 24;

/// 单次至少50ms取1；短调用批次达到100ms或上限后停止增长。
pub fn finished(repeats: usize, seconds: f64) -> bool {
    (repeats == 1 && seconds >= 0.05) || seconds >= 0.1 || repeats == MAX_REPEAT
}

/// 每轮增长2～16倍并截到2^24；无效时长不得进入归一化统计。
pub fn next_repeat(repeats: usize, seconds: f64) -> usize {
    assert!((1..=MAX_REPEAT).contains(&repeats) && seconds.is_finite() && seconds >= 0.0);
    let factor = if seconds == 0.0 {
        16
    } else {
        (0.1 / seconds).ceil().clamp(2.0, 16.0) as usize
    };
    repeats.saturating_mul(factor).min(MAX_REPEAT)
}
