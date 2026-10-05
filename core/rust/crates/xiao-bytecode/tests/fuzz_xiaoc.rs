//! 19A `.xiaoc` 结构化模糊测试；种子和变异步骤固定，失败信息带步骤，可直接重放。
//!
//! 基线必须能解码，变异后要么仍合法，要么以带 `XIAOC-nnn` 稳定码的错误拒绝，
//! 且任何输入都不得 panic。

use std::panic::{AssertUnwindSafe, catch_unwind};

use xiao_bytecode::{XiaocMetadata, decode_xiaoc, encode_xiaoc, lower_program};
use xiao_driver::{FrontendCompiler, FrontendRequest};

const SEED: u64 = 0x0019_a002;
const ROUNDS: usize = 512;
const MAX_INPUT: usize = 1 << 20;

/// xorshift64*；只用于让变异可按种子重放。
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, upper: usize) -> usize {
        (self.next() % upper as u64) as usize
    }
}

fn seed_xiaoc() -> Vec<u8> {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "def add(int a, int b) -> int\n    return a + b\nresult = add(1, 2)\nprint(\"fuzz\")\n",
        ))
        .expect("固定源码应可编译");
    encode_xiaoc(&lower_program(artifact.ir()), XiaocMetadata::new("fuzz"))
        .expect("固定程序应可编码")
}

/// 返回变异后的字节和按顺序记录的变异步骤。
fn mutate(base: &[u8], round: usize) -> (Vec<u8>, Vec<String>) {
    let mut rng = Rng::new(SEED ^ (round as u64).wrapping_mul(0x9E37_79B9));
    let mut bytes = base.to_vec();
    let mut steps = Vec::new();
    for _ in 0..=rng.below(4) {
        if bytes.is_empty() {
            break;
        }
        let at = rng.below(bytes.len());
        match rng.below(5) {
            0 => {
                let mask = (rng.next() as u8) | 1;
                bytes[at] ^= mask;
                steps.push(format!("xor[{at}]^={mask:#04x}"));
            }
            1 => {
                bytes.remove(at);
                steps.push(format!("remove[{at}]"));
            }
            2 => {
                let value = rng.next() as u8;
                bytes.insert(at, value);
                steps.push(format!("insert[{at}]={value:#04x}"));
            }
            3 => {
                bytes.truncate(at);
                steps.push(format!("truncate@{at}"));
            }
            _ => {
                let len = (rng.below(8) + 1).min(bytes.len() - at);
                bytes[at..at + len].fill(0xFF);
                steps.push(format!("fill[{at}..{}]=0xff", at + len));
            }
        }
    }
    (bytes, steps)
}

#[test]
fn base_xiaoc_decodes() {
    decode_xiaoc(&seed_xiaoc()).expect("基线 `.xiaoc` 必须可解码，否则变异没有意义");
}

#[test]
fn seeded_xiaoc_mutations_never_panic_and_are_rejected_with_stable_codes() {
    let base = seed_xiaoc();
    let (mut accepted, mut rejected) = (0_usize, 0_usize);
    for round in 0..ROUNDS {
        let (candidate, steps) = mutate(&base, round);
        assert!(candidate.len() <= MAX_INPUT, "round={round}");
        let context = format!("seed={SEED:#x} round={round} steps={steps:?}");
        let result = catch_unwind(AssertUnwindSafe(|| decode_xiaoc(&candidate)))
            .unwrap_or_else(|_| panic!("`.xiaoc` 解析 panic：{context}"));
        match result {
            Ok(_) => accepted += 1,
            Err(error) => {
                rejected += 1;
                let text = error.to_string();
                let code = text.split(':').next().unwrap_or_default();
                assert!(
                    code.len() == "XIAOC-000".len()
                        && code.starts_with("XIAOC-")
                        && code["XIAOC-".len()..].bytes().all(|b| b.is_ascii_digit()),
                    "拒绝缺少稳定错误码（{text}）：{context}"
                );
            }
        }
    }
    assert!(
        rejected > ROUNDS / 2,
        "变异应主要被拒绝，实际 rejected={rejected} accepted={accepted}"
    );
}
