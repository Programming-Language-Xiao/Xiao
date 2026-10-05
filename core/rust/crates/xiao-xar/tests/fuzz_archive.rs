//! 19A `.xar` 结构化模糊测试；种子和输入上限固定，失败可直接重放。

use xiao_artifacts::{
    ArchiveEntry, ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR, ObjectKind,
};
use xiao_xar::{XarObject, decode_xar, encode_xar};

#[test]
fn deterministic_archive_mutations_never_panic_or_exceed_input_bound() {
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
    let seed = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
    assert!(seed.len() <= 64 * 1024);
    for round in 0..256_usize {
        let mut candidate = seed.clone();
        let index = (round.wrapping_mul(23).wrapping_add(7)) % candidate.len();
        candidate[index] ^= (round as u8).wrapping_mul(29).wrapping_add(5);
        if round % 13 == 0 {
            candidate.truncate(candidate.len().saturating_sub(round % 31));
        }
        let result = std::panic::catch_unwind(|| {
            let _ = decode_xar(&candidate);
        });
        assert!(
            result.is_ok(),
            "归档解析 seed=0x19a03 round={round} 时 panic"
        );
    }
}
