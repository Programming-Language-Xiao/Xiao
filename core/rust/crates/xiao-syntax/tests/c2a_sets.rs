//! C2-A 集合字面量和花括号消歧的语法规格测试。

use xiao_source::SourceFile;
use xiao_syntax::{Expression, NodeIndex, Parser, Statement};

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
/// 非空花括号中的纯值条目应形成集合节点，并保留源码顺序供诊断使用。
fn parses_non_empty_set_literal() {
    let program = parse_ok("values = {1, \"x\", true}\n");
    let Statement::Assignment {
        value: Expression::SetLiteral { elements, span },
        ..
    } = &program.statements[0]
    else {
        panic!("expected set literal");
    };
    assert_eq!(elements.len(), 3);
    assert_eq!(span.start(), 9);
    assert_eq!(span.end(), 23);
    assert!(matches!(elements[0], Expression::Literal { .. }));
    assert!(matches!(elements[1], Expression::Literal { .. }));
    assert!(matches!(elements[2], Expression::Literal { .. }));
}

#[test]
/// 空花括号仍是空字典表；`set()` 保持普通空参数调用节点。
fn distinguishes_empty_dict_from_set_constructor() {
    let program = parse_ok("empty_dict = {}\nempty_set = set()\n");
    assert!(matches!(
        program.statements[0].try_expression(),
        Some(Expression::DictTableLiteral { entries, .. }) if entries.is_empty()
    ));
    let Some(Expression::Call {
        callee, arguments, ..
    }) = program.statements[1].try_expression()
    else {
        panic!("expected set() call");
    };
    assert!(matches!(callee.as_ref(), Expression::Name(name) if !name.backticked));
    assert!(arguments.is_empty());
}

#[test]
/// 集合允许递归表达式和物理换行，嵌套容器仍由各自节点表示。
fn parses_multiline_and_nested_set_values() {
    let program = parse_ok("values = {\n    [1, 2],\n    (true, none),\n    {name = \"x\"}\n}\n");
    let Statement::Assignment {
        value: Expression::SetLiteral { elements, .. },
        ..
    } = &program.statements[0]
    else {
        panic!("expected set literal");
    };
    assert_eq!(elements.len(), 3);
    assert!(matches!(elements[0], Expression::ArrayLiteral { .. }));
    assert!(matches!(elements[1], Expression::TupleLiteral { .. }));
    assert!(matches!(elements[2], Expression::DictTableLiteral { .. }));
}

#[test]
/// 集合与字典条目混用时产生稳定容器诊断，并恢复到右花括号。
fn diagnoses_mixed_set_and_dictionary_entries() {
    let result = Parser::new(&SourceFile::from_text("values = {1, name = 2}\nnext = 1\n")).parse();
    assert!(
        result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == "X03-PARSE-002"
                && diagnostic.message_id() == "x03.parse.mixed_brace_entries"
        }),
        "expected mixed-entry diagnostic: {:?}",
        result.diagnostics
    );
    let program = result.program.expect("parser should return a program");
    assert_eq!(program.statements.len(), 2);
    assert!(matches!(
        program.statements[1].try_expression(),
        Some(Expression::Literal { .. })
    ));
}

#[test]
/// 集合节点及其元素应纳入 AST 先序节点索引。
fn indexes_set_elements_in_preorder() {
    let program = parse_ok("values = {1, (2, 3)}\n");
    let index = NodeIndex::build(&program);
    assert!(index.len() >= 6, "node index too small: {}", index.len());
}

#[test]
/// 花括号步长后缀仍按选择器解析，不应被误判为集合字面量。
fn preserves_step_selector_after_expression() {
    let program = parse_ok("selected = values{2}[0]\n");
    assert!(matches!(
        program.statements[0].try_expression(),
        Some(Expression::Selector { step: Some(_), .. })
    ));
}
