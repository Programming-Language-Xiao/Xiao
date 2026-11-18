//! P2 声明语法 JSON 快照入口。

use serde::Deserialize;
use xiao_source::SourceFile;
use xiao_syntax::{Parser, Statement};

/// 一条 P2 解析快照用例。
#[derive(Debug, Deserialize)]
struct SnapshotCase {
    /// 用例名称。
    name: String,
    /// 输入源码。
    source: String,
    /// 期望类别。
    expect: String,
    /// 期望诊断编号。
    diagnostics: Vec<String>,
}

/// P2 快照文件。
#[derive(Debug, Deserialize)]
struct SnapshotFile {
    /// 阶段标识。
    stage: String,
    /// 用例列表。
    cases: Vec<SnapshotCase>,
}

#[test]
/// 验证声明 AST 形状和 P2 解析诊断保持稳定。
fn declaration_snapshot_is_stable() {
    let snapshot: SnapshotFile = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/04-types/declarations.json"
    )))
    .expect("P2 快照 JSON 必须有效");
    assert_eq!(snapshot.stage, "P2");
    for case in snapshot.cases {
        let result = Parser::new(&SourceFile::from_text(&case.source)).parse();
        let actual = result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(actual, case.diagnostics, "snapshot case: {}", case.name);
        if case.expect == "declaration" {
            assert!(matches!(
                result.program.expect("program").statements.as_slice(),
                [Statement::Declaration { .. }]
            ));
        } else if case.expect == "const" {
            assert!(matches!(
                result.program.expect("program").statements.as_slice(),
                [Statement::ConstDeclaration { .. }]
            ));
        } else {
            assert!(result.has_errors(), "snapshot case: {}", case.name);
        }
    }
}
