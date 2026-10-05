//! 19A 确定性 IR 快照模糊测试；只验证拒绝边界和 panic 安全，不执行用户代码。

use std::panic::{AssertUnwindSafe, catch_unwind};

use xiao_ir::from_json;

const MAX_INPUT: usize = 4096;

fn mutate(seed: u64, round: usize) -> Vec<u8> {
    let mut value = br#"{"version":1,"functions":[]}"#.to_vec();
    let mut state = seed.wrapping_add(round as u64);
    for byte in &mut value {
        state ^= state << 7;
        state ^= state >> 9;
        if state & 3 == 0 {
            *byte ^= state as u8;
        }
    }
    if round % 5 == 0 {
        value.extend_from_slice(&[0, 1, 2, 3]);
    }
    value.truncate(MAX_INPUT);
    value
}

#[test]
fn deterministic_ir_mutations_never_panic_or_escape_input_bound() {
    for round in 0..256 {
        let bytes = mutate(0x0019_a001, round);
        assert!(bytes.len() <= MAX_INPUT);
        let text = String::from_utf8_lossy(&bytes);
        let result = catch_unwind(AssertUnwindSafe(|| from_json(&text)));
        assert!(
            result.is_ok(),
            "IR 快照解析在 seed=0x19a01 round={round} 时 panic"
        );
    }
}
