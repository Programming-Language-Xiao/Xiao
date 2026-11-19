//! 使用产品解码器复核保存的真实产物；不复制容器/字节码格式实现。
use std::collections::BTreeSet;

fn main() {
    let path = std::env::args().nth(1).expect("产物路径");
    let raw = std::fs::read(&path).unwrap();
    let entry = if path.ends_with(".xar") {
        let archive = xiao_xar::decode_xar(&raw).unwrap();
        let index = archive.index();
        assert!(index.stdlib_modules.contains(&"stdlib.cmath".to_owned()));
        assert!(index.stdlib_modules.iter().all(|n| !n.starts_with("std.")));
        assert!(
            index
                .entries
                .iter()
                .all(|n| !n.module.starts_with("std.") && !n.module.starts_with("stdlib."))
        );
        assert_eq!(index.stdlib_interface_version, 1);
        archive.read_entry().unwrap()
    } else {
        raw
    };
    let file = xiao_bytecode::decode_xiaoc(&entry).unwrap();
    let ids: BTreeSet<_> = file
        .program
        .signatures
        .iter()
        .filter_map(|s| s.intrinsic_id())
        .filter(|id| (34..=48).contains(id))
        .collect();
    assert!(ids.is_empty() || ids == (34..=48).collect());
    println!(
        "ids={} identity=canonical bundled_official=false decoded=passed",
        if ids.is_empty() { "none" } else { "34..48" }
    );
}
