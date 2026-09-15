//! 05-C 表语法、文档注释和节点索引规格测试。

use xiao_source::SourceFile;
use xiao_syntax::{NodeIndex, Parser, Statement, TableKind};

#[test]
/// 单例表和可实例化表应保留表头形态、成员顺序及文档注释。
fn parses_singleton_and_instance_tables() {
    let source_text = "### config docs ###\n[Config]\n    ### host docs ###\n    host = \"localhost\"\n    int port = 8080\n    def label(self) -> str\n        return host\n[[User]]\n    def init(self, int id) -> none\n        return\n";
    let source = SourceFile::from_text(source_text);
    let result = Parser::new(&source).parse();
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    let program = result.program.expect("program");
    assert_eq!(program.statements.len(), 2);
    let Statement::Table {
        name,
        kind,
        body,
        leading_docs,
        ..
    } = &program.statements[0]
    else {
        panic!("expected singleton table");
    };
    assert_eq!(*kind, TableKind::Singleton);
    assert_eq!(name.unquoted_text(&source), "Config");
    assert_eq!(body.len(), 3);
    assert_eq!(leading_docs.len(), 1);
    assert!(matches!(
        program.statements[1],
        Statement::Table {
            kind: TableKind::Instance,
            ..
        }
    ));
    let NodeIndex { nodes } = NodeIndex::build(&program);
    assert!(
        nodes.len() > 6,
        "table bodies must participate in NodeIndex"
    );
}

#[test]
/// 表头必须顶层，表体只能保存字段或方法。
fn diagnoses_invalid_table_shapes() {
    let result = Parser::new(&SourceFile::from_text(
        "[Config]\n    if true\n        value = 1\n[Outer]\n    [Inner]\n",
    ))
    .parse();
    let ids = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message_id())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"x05.parse.invalid_table_member"));
    assert!(ids.contains(&"x05.parse.nested_table"));
}

#[test]
/// 没有缩进成员时应报告稳定的空表体诊断。
fn diagnoses_missing_table_body() {
    let result = Parser::new(&SourceFile::from_text("[Empty]\nnext = 1\n")).parse();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "X05-PARSE-007")
    );
}
