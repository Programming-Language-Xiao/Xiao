//! 19A `.xar` 结构化模糊测试；种子和变异步骤固定，失败信息带步骤，可直接重放。
//!
//! 基线必须能解码，变异后要么仍合法，要么以归档格式错误拒绝；输入是内存字节，
//! 所以不得出现底层 IO 错误，也不得 panic。

#![allow(clippy::result_large_err)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use xiao_artifacts::{
    ArchiveEntry, ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR, ObjectKind,
};
use xiao_xar::{XarError, XarObject, decode_xar, encode_xar};

const SEED: u64 = 0x0019_a003;
const ROUNDS: usize = 512;
const MAX_INPUT: usize = 64 * 1024;

/// xorshift64*；只用于让变异可按种子重放。
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
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

fn seed_archive() -> Vec<u8> {
    let object = XarObject::from_bytes(ObjectKind::Resource, b"fuzz-archive");
    let index = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "asset.bin".to_owned(),
        entries: vec![ArchiveEntry {
            logical_path: "asset.bin".to_owned(),
            object_kind: ObjectKind::Resource,
            digest: object.digest,
            module: "fuzz".to_owned(),
            target: "portable".to_owned(),
            length: object.bytes.len() as u64,
        }],
        dependency_lock_digest: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_owned(),
        runtime_abi_min: 1,
        runtime_abi_max: 1,
        platform: "portable".to_owned(),
        debug_activation: false,
        language_locale: "zh-CN".to_owned(),
    };
    encode_xar(&index, std::slice::from_ref(&object)).expect("固定归档应可编码")
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
fn base_archive_decodes() {
    let seed = seed_archive();
    assert!(seed.len() <= MAX_INPUT);
    decode_xar(&seed).expect("基线 `.xar` 必须可解码，否则变异没有意义");
}

#[test]
fn seeded_archive_mutations_never_panic_and_are_rejected_without_io_errors() {
    let base = seed_archive();
    let (mut accepted, mut rejected) = (0_usize, 0_usize);
    for round in 0..ROUNDS {
        let (candidate, steps) = mutate(&base, round);
        assert!(candidate.len() <= MAX_INPUT, "round={round}");
        let context = format!("seed={SEED:#x} round={round} steps={steps:?}");
        let result = catch_unwind(AssertUnwindSafe(|| decode_xar(&candidate)))
            .unwrap_or_else(|_| panic!("归档解析 panic：{context}"));
        match result {
            Ok(_) => accepted += 1,
            Err(XarError::Io(error)) => {
                panic!("内存输入不应产生 IO 错误（{error}）：{context}")
            }
            Err(_) => rejected += 1,
        }
    }
    assert!(
        rejected > ROUNDS / 2,
        "变异应主要被拒绝，实际 rejected={rejected} accepted={accepted}"
    );
}
