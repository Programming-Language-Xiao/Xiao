//! 05-A 导入语法的规格回归测试。

use xiao_source::SourceFile;
use xiao_syntax::{ImportStatement, Name, NodeIndex, Parser, Statement};

/// 解析一段应当没有错误诊断的源码。
fn parse_ok(source: &str) -> xiao_syntax::Program {
    let result = Parser::new(&SourceFile::from_text(source)).parse();
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    result.program.expect("parser should return a program")
}

/// 返回一条语句中的名称原文，供别名断言使用。
fn name_text(name: Name, source: &SourceFile) -> String {
    name.unquoted_text(source).to_owned()
}

#[test]
/// `import` 支持多个绝对路径和显式别名。
fn parses_multiple_module_imports_and_aliases() {
    let source = SourceFile::from_text("import net.http, net.http as http\n");
    let program = parse_ok(source.text());
    let Statement::Import { import, span, .. } = &program.statements[0] else {
        panic!("expected import statement");
    };
    let ImportStatement::Modules { imports, .. } = import else {
        panic!("expected module import list");
    };
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].path.segments.len(), 2);
    assert!(imports[0].alias.is_none());
    assert_eq!(name_text(imports[1].alias.expect("alias"), &source), "http");
    assert!(span.start() < span.end());
}

#[test]
/// `from` 支持多个选择名称、别名和反引号名称。
fn parses_selected_imports_and_backticked_names() {
    let source =
        SourceFile::from_text("from app.models import User, `用户` as user, value as `值`\n");
    let program = parse_ok(source.text());
    let Statement::Import { import, .. } = &program.statements[0] else {
        panic!("expected import statement");
    };
    let ImportStatement::From {
        module, imports, ..
    } = import
    else {
        panic!("expected from import");
    };
    assert_eq!(module.segments.len(), 2);
    assert_eq!(imports.len(), 3);
    assert!(imports[1].name.backticked);
    assert_eq!(name_text(imports[1].name, &source), "用户");
    assert!(imports[2].alias.expect("alias").backticked);
    assert_eq!(name_text(imports[2].alias.expect("alias"), &source), "值");
}

#[test]
/// 导入可以位于函数、条件和循环的缩进体中。
fn preserves_imports_inside_nested_blocks() {
    let program = parse_ok(
        "def load()\n    import local.module\nif ready\n    from branch import value\nfor item in items\n    import loop.module\n",
    );
    assert_eq!(program.statements.len(), 3);
    let Statement::Function { body, .. } = &program.statements[0] else {
        panic!("expected function");
    };
    assert!(matches!(body[0], Statement::Import { .. }));
    let Statement::If { body, .. } = &program.statements[1] else {
        panic!("expected if");
    };
    assert!(matches!(body[0], Statement::Import { .. }));
    let Statement::For { body, .. } = &program.statements[2] else {
        panic!("expected for");
    };
    assert!(matches!(body[0], Statement::Import { .. }));
}

#[test]
/// 导入语句会接收相邻文档注释，且节点索引保持源码先序。
fn attaches_docs_and_indexes_import_statement() {
    let source = SourceFile::from_text("### module docs ###\nimport app\nvalue = 1\n");
    let program = parse_ok(source.text());
    let Statement::Import { leading_docs, .. } = &program.statements[0] else {
        panic!("expected import statement");
    };
    assert_eq!(leading_docs.len(), 1);
    let index = NodeIndex::build(&program);
    assert!(!index.is_empty());
    assert_eq!(index.nodes[0].1, program.statements[0].span());
}

#[test]
/// 相对、通配、尾逗号和缺失别名都给出 05-A 稳定诊断。
fn diagnoses_unsupported_import_forms() {
    let source = SourceFile::from_text(
        "from . import value\nfrom app import *\nimport app,\nimport app as\n",
    );
    let result = Parser::new(&source).parse();
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"X05-PARSE-004"));
    assert!(codes.contains(&"X05-PARSE-002"));
    assert!(codes.contains(&"X05-PARSE-003"));
}

#[test]
/// 模块路径段只接受普通 ASCII 标识符，不能用反引号名称替代。
fn rejects_non_ascii_module_path_segments() {
    let result = Parser::new(&SourceFile::from_text("import `网络`.http\n")).parse();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "X05-PARSE-001")
    );
}
