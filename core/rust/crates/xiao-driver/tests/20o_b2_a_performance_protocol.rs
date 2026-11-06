//! 默认测试只核对测量协议，不在debug运行最大输入基准。

#[path = "../examples/numeric_perf/calibration.rs"]
mod calibration;

#[test]
/// 零和量化时钟仍有界收敛，不会假造纳秒或一次放大数千万倍。
fn quantized_clock_calibration_retains_real_batches() {
    let mut repeats = 1;
    for _ in 0..32 {
        let elapsed = (repeats as f64 * 0.00000025 / 0.000001).floor() * 0.000001;
        if calibration::finished(repeats, elapsed) {
            assert!(elapsed >= 0.1 && elapsed < 0.3);
            return;
        }
        let next = calibration::next_repeat(repeats, elapsed);
        assert!(next > repeats && next <= repeats * 16 && next <= calibration::MAX_REPEAT);
        repeats = next;
    }
    panic!("量化时钟校准没有收敛");
}

#[test]
/// 50ms单次和100ms批次是不同条件；上限不足50ms仍不可作为达标样本。
fn calibration_stop_conditions_follow_the_frozen_protocol() {
    assert!(calibration::finished(1, 0.05));
    assert!(!calibration::finished(2, 0.05));
    assert!(calibration::finished(2, 0.1));
    assert!(calibration::finished(calibration::MAX_REPEAT, 0.0));
    assert_eq!(
        calibration::next_repeat(calibration::MAX_REPEAT / 2, 0.0),
        calibration::MAX_REPEAT
    );
}

#[test]
/// 非有限和负时长不能靠增长公式进入统计。
fn invalid_clock_samples_are_rejected() {
    for seconds in [f64::NAN, f64::INFINITY, -0.1] {
        assert!(std::panic::catch_unwind(|| calibration::next_repeat(1, seconds)).is_err());
    }
}
