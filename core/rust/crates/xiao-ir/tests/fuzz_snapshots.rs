//! 19A 确定性 IR 快照模糊测试；只验证拒绝边界和 panic 安全，不执行用户代码。
//!
//! 基线是一份真实 `IrProgram` 的快照，先确认它能解析并往返，再对它做种子驱动的
//! 字节变异。变异必须真的产生拒绝，且拒绝只能是解码或版本两类错误。

use std::panic::{AssertUnwindSafe, catch_unwind};

use xiao_ir::{IrEntryMode, IrProgram, IrSpan, SnapshotError, from_json};

const SEED: u64 = 0x0019_a001;
const ROUNDS: usize = 512;
const MAX_INPUT: usize = 64 * 1024;

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

fn base_snapshot() -> String {
    IrProgram::new(IrEntryMode::Script, Vec::new(), IrSpan::new(0, 0))
        .to_json()
        .expect("基线 IR 应可编码")
}

fn mutate(base: &[u8], round: usize) -> Vec<u8> {
    let mut rng = Rng::new(SEED ^ (round as u64).wrapping_mul(0x9E37_79B9));
    let mut bytes = base.to_vec();
    for _ in 0..=rng.below(4) {
        if bytes.is_empty() {
            break;
        }
        let at = rng.below(bytes.len());
        match rng.below(4) {
            0 => bytes[at] ^= (rng.next() as u8) | 1,
            1 => {
                bytes.remove(at);
            }
            2 => bytes.insert(at, rng.next() as u8),
            _ => bytes.truncate(at),
        }
    }
    bytes.truncate(MAX_INPUT);
    bytes
}

#[test]
fn base_snapshot_parses_and_round_trips() {
    let text = base_snapshot();
    let program = from_json(&text).expect("基线快照必须可解析，否则变异没有意义");
    assert_eq!(program.to_json().unwrap(), text);
}

#[test]
fn seeded_ir_mutations_never_panic_and_are_rejected_with_typed_errors() {
    let base = base_snapshot().into_bytes();
    let (mut accepted, mut rejected) = (0_usize, 0_usize);
    for round in 0..ROUNDS {
        let bytes = mutate(&base, round);
        assert!(bytes.len() <= MAX_INPUT);
        let text = String::from_utf8_lossy(&bytes);
        let result = catch_unwind(AssertUnwindSafe(|| from_json(&text)))
            .unwrap_or_else(|_| panic!("IR 快照解析在 seed={SEED:#x} round={round} 时 panic"));
        match result {
            Ok(_) => accepted += 1,
            Err(SnapshotError::Decode(_) | SnapshotError::UnsupportedVersion(_)) => rejected += 1,
            Err(other) => panic!("round={round} 解码返回了不该出现的错误类别: {other}"),
        }
    }
    assert!(
        rejected > ROUNDS / 2,
        "变异应主要被拒绝，实际 rejected={rejected} accepted={accepted}"
    );
}

#[test]
fn unsupported_version_is_reported_as_version_error() {
    let text = base_snapshot().replacen("\"version\":1", "\"version\":999", 1);
    assert!(matches!(
        from_json(&text),
        Err(SnapshotError::UnsupportedVersion(999))
    ));
}
