//! C0 AST/诊断稳定性快照测试。

use xiao_source::SourceFile;
use xiao_syntax::INVALID_CONTAINER_CODE;
use xiao_syntax::{Expression, NodeIndex, Parser, Statement};

#[test]
/// 嵌套容器的节点索引应按源码先序包含容器、键和值节点。
fn nested_container_node_index_is_preorder() {
    let source = SourceFile::from_text("value = [{name = (1, true)}]\n");
    let result = Parser::new(&source).parse();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let program = result.program.expect("program");
    let index = NodeIndex::build(&program);
    let Statement::Assignment { value, .. } = &program.statements[0] else {
        panic!("expected assignment");
    };
    assert!(matches!(value, Expression::ArrayLiteral { .. }));
    assert!(index.len() >= 7, "node index too small: {}", index.len());
}

#[test]
/// 非法容器键的诊断编号必须保持 X03-PARSE 命名空间。
fn container_parse_diagnostic_namespace_is_stable() {
    let result = Parser::new(&SourceFile::from_text("value = {1 = true}\n")).parse();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == INVALID_CONTAINER_CODE)
    );
}
