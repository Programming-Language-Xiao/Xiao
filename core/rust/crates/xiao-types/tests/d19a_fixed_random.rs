//! 19A（19.3）固定随机源：抽取顺序、放回语义和错误边界。
//!
//! 期望值由独立的 BigInt 实现按 xorshift64* 公式算出，不取自被测实现的输出。

use xiao_syntax::RandomMode;
use xiao_types::{RandomSelectionError, SeededRandom, sample_indices};

#[test]
fn seed_sequence_matches_independent_xorshift64_star() {
    let mut random = SeededRandom::new(42);
    let actual = [
        random.next_u64(),
        random.next_u64(),
        random.next_u64(),
        random.next_u64(),
    ];
    assert_eq!(
        actual,
        [
            0x56ce_4ab7_719b_a3a0,
            0xc841_eb53_ebbb_2dda,
            0xca46_6be0_c998_0276,
            0xf1ac_c733_4a7b_70df,
        ]
    );
}

#[test]
fn zero_and_folded_zero_seeds_use_the_fallback_state() {
    let first = SeededRandom::new(0).next_u64();
    assert_eq!(first, 0x0d83_b3e2_9a21_487a);
    assert_eq!(SeededRandom::new((1_u128 << 64) | 1).next_u64(), first);
    assert_ne!(SeededRandom::new(1).next_u64(), first);
}

#[test]
fn high_seed_bits_participate_in_the_fold() {
    let mut random = SeededRandom::new((1_u128 << 64) + 5);
    assert_eq!(random.next_u64(), 0x44d9_2dbf_7520_5191);
    assert_eq!(random.next_u64(), 0xb1e5_b57a_9752_7191);
}

#[test]
fn without_replacement_follows_the_expected_draw_order() {
    let mut random = SeededRandom::new(42);
    assert_eq!(
        sample_indices(4, 4, RandomMode::WithoutReplacement, &mut random),
        Ok(vec![0, 2, 3, 1])
    );
    let mut random = SeededRandom::new(42);
    assert_eq!(
        sample_indices(4, 2, RandomMode::WithoutReplacement, &mut random),
        Ok(vec![0, 2])
    );
    let mut random = SeededRandom::new(7);
    assert_eq!(
        sample_indices(10, 5, RandomMode::WithoutReplacement, &mut random),
        Ok(vec![2, 5, 9, 0, 8])
    );
}

#[test]
fn with_replacement_follows_the_expected_draw_order_and_may_repeat() {
    let mut random = SeededRandom::new(42);
    let drawn = sample_indices(2, 8, RandomMode::WithReplacement, &mut random).unwrap();
    assert_eq!(drawn, vec![0, 0, 0, 1, 0, 1, 0, 1]);
    let mut random = SeededRandom::new(42);
    assert_eq!(
        sample_indices(5, 6, RandomMode::WithReplacement, &mut random),
        Ok(vec![0, 3, 1, 0, 3, 0])
    );
    let mut random = SeededRandom::new(1);
    assert_eq!(
        sample_indices(3, 5, RandomMode::WithReplacement, &mut random),
        Ok(vec![1, 2, 1, 0, 2])
    );
}

#[test]
fn the_two_modes_consume_the_same_stream_differently() {
    let mut without = SeededRandom::new(42);
    let mut with = SeededRandom::new(42);
    let a = sample_indices(4, 4, RandomMode::WithoutReplacement, &mut without).unwrap();
    let b = sample_indices(4, 4, RandomMode::WithReplacement, &mut with).unwrap();
    let mut sorted = a.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![0, 1, 2, 3]);
    assert_ne!(a, b);
    assert_eq!(without.state(), with.state());
}

#[test]
fn same_seed_reproduces_and_different_seeds_diverge() {
    let run = |seed: u128| {
        let mut random = SeededRandom::new(seed);
        sample_indices(100, 20, RandomMode::WithoutReplacement, &mut random).unwrap()
    };
    assert_eq!(run(42), run(42));
    assert_ne!(run(42), run(43));
}

#[test]
fn reseed_restarts_the_sequence() {
    let mut random = SeededRandom::new(42);
    let first = random.next_u64();
    random.next_u64();
    random.reseed(42);
    assert_eq!(random.next_u64(), first);
}

#[test]
fn error_boundaries_are_distinguished_and_consume_no_randomness() {
    let mut random = SeededRandom::new(1);
    let before = random.state();
    assert_eq!(
        sample_indices(0, 1, RandomMode::WithReplacement, &mut random),
        Err(RandomSelectionError::EmptySource)
    );
    assert_eq!(
        sample_indices(0, 1, RandomMode::WithoutReplacement, &mut random),
        Err(RandomSelectionError::EmptySource)
    );
    assert_eq!(
        sample_indices(3, 4, RandomMode::WithoutReplacement, &mut random),
        Err(RandomSelectionError::WithoutReplacementTooMany {
            count: 4,
            available: 3
        })
    );
    assert_eq!(random.state(), before);
}

#[test]
fn zero_count_is_empty_even_for_an_empty_source() {
    let mut random = SeededRandom::new(1);
    let before = random.state();
    for mode in [RandomMode::WithReplacement, RandomMode::WithoutReplacement] {
        assert_eq!(sample_indices(0, 0, mode, &mut random), Ok(Vec::new()));
        assert_eq!(sample_indices(5, 0, mode, &mut random), Ok(Vec::new()));
    }
    assert_eq!(random.state(), before);
}

#[test]
fn with_replacement_allows_counts_above_the_source_length() {
    let mut random = SeededRandom::new(9);
    let drawn = sample_indices(1, 50, RandomMode::WithReplacement, &mut random).unwrap();
    assert_eq!(drawn, vec![0; 50]);
}

#[test]
fn error_display_text_is_stable() {
    assert_eq!(
        RandomSelectionError::EmptySource.to_string(),
        "cannot draw from an empty source"
    );
    assert_eq!(
        RandomSelectionError::WithoutReplacementTooMany {
            count: 4,
            available: 3
        }
        .to_string(),
        "cannot draw 4 values without replacement from 3 values"
    );
    assert_eq!(
        RandomSelectionError::InvalidCount.to_string(),
        "random selection count is invalid"
    );
}
