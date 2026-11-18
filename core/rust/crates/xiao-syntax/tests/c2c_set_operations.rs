//! C2-C 集合运算的词法、优先级和复合赋值语法规格测试。

use xiao_source::SourceFile;
use xiao_syntax::{
    AssignmentOperator, BinaryOperator, Expression, Lexer, Parser, Statement, TokenKind,
};

/// 解析一段源码并要求没有语法诊断。
fn parse_ok(source: &str) -> xiao_syntax::Program {
    let file = SourceFile::from_text(source);
    let result = Parser::new(&file).parse();
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    result.program.expect("parser should return a program")
}

#[test]
/// 新增集合运算和复合赋值 Token 采用最长匹配。
fn tokenizes_set_operation_operators() {
    let source = SourceFile::from_text("& ^= ^ &=");
    let result = Lexer::new(&source).tokenize();
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(
        result
            .tokens
            .iter()
            .map(|token| token.kind())
            .collect::<Vec<_>>(),
        vec![
            TokenKind::Ampersand,
            TokenKind::CaretEqual,
            TokenKind::Caret,
            TokenKind::AmpersandEqual,
            TokenKind::Eof,
        ]
    );
}

#[test]
/// `&` 高于 `^`，并且两者低于加法，保持常见位运算层级。
fn preserves_set_operator_precedence() {
    let program = parse_ok("value = left + middle & right ^ tail\n");
    let Statement::Assignment { value, .. } = &program.statements[0] else {
        panic!("expected assignment");
    };
    let Expression::Binary {
        operator: BinaryOperator::SymmetricDifference,
        left: outer_left,
        right: outer_right,
        ..
    } = value
    else {
        panic!("expected symmetric difference at the root");
    };
    assert!(matches!(outer_right.as_ref(), Expression::Name(_)));
    let Expression::Binary {
        operator: BinaryOperator::Intersect,
        left: intersect_left,
        right: intersect_right,
        ..
    } = outer_left.as_ref()
    else {
        panic!("expected intersection below symmetric difference");
    };
    assert!(matches!(intersect_right.as_ref(), Expression::Name(_)));
    let Expression::Binary {
        operator: BinaryOperator::Add,
        ..
    } = intersect_left.as_ref()
    else {
        panic!("expected addition below intersection");
    };
}

#[test]
/// 四种集合原地运算都映射为独立的赋值运算符。
fn parses_all_set_compound_assignments() {
    let program = parse_ok("left += right\nleft -= right\nleft &= right\nleft ^= right\n");
    let operators = program
        .statements
        .iter()
        .map(Statement::assignment_operator)
        .collect::<Vec<_>>();
    assert_eq!(
        operators,
        vec![
            Some(AssignmentOperator::AddAssign),
            Some(AssignmentOperator::SubtractAssign),
            Some(AssignmentOperator::IntersectAssign),
            Some(AssignmentOperator::SymmetricDifferenceAssign),
        ]
    );
}
