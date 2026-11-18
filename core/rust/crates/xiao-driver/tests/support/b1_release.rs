//! REM-C 原生清理前置的同源向量；默认 VM、既有原生差分目标与显式诊断共用。

use std::path::Path;

/// 独立登记的源文件和黄金输出。
#[derive(serde::Deserialize)]
pub(crate) struct Case {
    /// 仅作为产物名的固定案例身份。
    pub(crate) id: String,
    /// 仓库内的完整 Xiao 源码。
    pub(crate) source: String,
    /// 独立黄金 stdout。
    pub(crate) stdout: String,
}

/// 读取固定 fixture 清单；与产品模块/公开入口计数分开。
pub(crate) fn cases() -> Vec<Case> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let directory = root.join("tests/fixtures/20o-b1/imp/release-edges");
    let mut cases: Vec<Case> =
        serde_json::from_str(&std::fs::read_to_string(directory.join("golden.json")).unwrap())
            .unwrap();
    for case in &mut cases {
        case.source = std::fs::read_to_string(directory.join(format!("{}.xiao", case.id))).unwrap();
    }
    cases
}
