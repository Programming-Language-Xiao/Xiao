//! C0 容器字面量和声明路径的语法规格测试。

use xiao_source::SourceFile;
use xiao_syntax::{DictKey, Expression, Parser, PathSegment, Statement};

/// 解析一段源码并要求语法层没有诊断。
fn parse_ok(source: &str) -> xiao_syntax::Program {
    let result = Parser::new(&SourceFile::from_text(source)).parse();
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    result.program.expect("parser should return a program")
}

#[test]
/// 验证数组、空元组、单元素元组和分组表达式不会互相混淆。
fn parses_array_and_python_tuples() {
    let program = parse_ok("items = [1, \"x\", true]\nempty = ()\none = (1,)\ngroup = (1)\n");
    let Statement::Assignment {
        value: Expression::ArrayLiteral { elements, .. },
        ..
    } = &program.statements[0]
    else {
        panic!("expected array");
    };
    assert_eq!(elements.len(), 3);
    assert!(
        matches!(program.statements[1].try_expression(), Some(Expression::TupleLiteral { elements, .. }) if elements.is_empty())
    );
    assert!(
        matches!(program.statements[2].try_expression(), Some(Expression::TupleLiteral { elements, .. }) if elements.len() == 1)
    );
}

#[test]
/// 验证字典表使用花括号、字典列使用尖括号，并保留键的原始种类。
fn parses_dictionary_shapes_and_keys() {
    let program =
        parse_ok("table = {name = \"x\", `编号` = 1}\ncolumn = <name = \"x\", level = 1>\n");
    let Statement::Assignment {
        value: Expression::DictTableLiteral { entries, .. },
        ..
    } = &program.statements[0]
    else {
        panic!("expected dictionary table");
    };
    assert!(matches!(entries[0].key, DictKey::Name(_)));
    assert!(matches!(entries[1].key, DictKey::Name(name) if name.backticked));
    assert!(matches!(
        &program.statements[1],
        Statement::Assignment {
            value: Expression::DictColumnLiteral { entries, .. },
            ..
        } if entries.len() == 2
    ));
}

#[test]
/// 字典列可以嵌套在数组中，闭尖括号不能被误判成比较运算符。
fn parses_nested_dictionary_column() {
    let program = parse_ok(
        "value = [<name = 1>]\nnested = <outer = <inner = 1>>\ncomparison = <value = (1 > 0)>\nchained = <value = 1> + 2\n",
    );
    assert!(matches!(
        program.statements[0].try_expression(),
        Some(Expression::ArrayLiteral { elements, .. })
            if matches!(elements.first(), Some(Expression::DictColumnLiteral { .. }))
    ));
    assert!(matches!(
        program.statements[1].try_expression(),
        Some(Expression::DictColumnLiteral { entries, .. })
            if matches!(entries.first().map(|entry| &entry.value), Some(Expression::DictColumnLiteral { .. }))
    ));
    assert!(matches!(
        program.statements[3].try_expression(),
        Some(Expression::Binary { .. })
    ));
}

#[test]
/// 验证声明路径按零基数字段保存，并支持嵌套 `/`。
fn preserves_declaration_constraint_path() {
    let program = parse_ok("int list[3/2]\n");
    let Statement::Declaration {
        constraint_path: Some(path),
        value: None,
        ..
    } = &program.statements[0]
    else {
        panic!("expected path declaration");
    };
    assert_eq!(path.segments.len(), 2);
    assert!(matches!(
        path.segments[0],
        PathSegment::Integer {
            negative: false,
            ..
        }
    ));
    assert!(matches!(
        path.segments[1],
        PathSegment::Integer {
            negative: false,
            ..
        }
    ));
}

#[test]
/// 验证 C0 字典键必须使用 `=`，而不是后续表达式赋值语法。
fn diagnoses_invalid_dictionary_entry() {
    let result = Parser::new(&SourceFile::from_text("value = {name: 1}\n")).parse();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-PARSE-002")
    );
}

#[test]
/// `const name[path]` 在运行时锁定语义冻结前必须明确拒绝，不能静默丢弃路径。
fn rejects_const_path_constraints() {
    let result = Parser::new(&SourceFile::from_text("const int values[0] = 1\n")).parse();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-PARSE-004")
    );
}
