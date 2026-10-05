//! 19A `.xiaoc` 结构化模糊测试；种子和变异顺序固定，失败可直接重放。

use std::panic::{AssertUnwindSafe, catch_unwind};

use xiao_bytecode::{XiaocMetadata, encode_xiaoc, lower_program};
use xiao_driver::{FrontendCompiler, FrontendRequest};

const MAX_INPUT: usize = 1 << 20;

fn seed_xiaoc() -> Vec<u8> {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text("value = 1\n"))
        .expect("固定源码应可编译");
    encode_xiaoc(&lower_program(artifact.ir()), XiaocMetadata::new("fuzz"))
        .expect("固定程序应可编码")
}

#[test]
fn deterministic_xiaoc_mutations_never_panic_or_allocate_unbounded_input() {
    let seed = seed_xiaoc();
    for round in 0..256_usize {
        let mut candidate = seed.clone();
        let index = (round.wrapping_mul(37).wrapping_add(11)) % candidate.len();
        candidate[index] ^= (round as u8).wrapping_mul(13).wrapping_add(1);
        if round % 17 == 0 {
            candidate.truncate(candidate.len().saturating_sub(round % 23));
        }
        assert!(candidate.len() <= MAX_INPUT);
        let result = catch_unwind(AssertUnwindSafe(|| xiao_bytecode::decode_xiaoc(&candidate)));
        assert!(
            result.is_ok(),
            "`.xiaoc` 解析在 seed=0x19a02 round={round} 时 panic"
        );
    }
}
