//! P2 静态标量声明语法的规格测试。

use xiao_source::SourceFile;
use xiao_syntax::{DeclaredType, Expression, Parser, ScalarType, Statement};
use xiao_syntax::{INVALID_DECLARATION_TARGET_CODE, MISSING_CONST_VALUE_CODE};

/// 解析只包含一条声明的源码并返回语句。
fn one_statement(source: &str) -> Statement {
    let result = Parser::new(&SourceFile::from_text(source)).parse();
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    result
        .program
        .expect("parser should return a program")
        .statements
        .into_iter()
        .next()
        .expect("source should contain one statement")
}

#[test]
/// 验证带初始化和无初始化的标量声明都保留类型、名称和源码区间。
fn parses_typed_declarations() {
    let initialized_source = SourceFile::from_text("int count = 1\n");
    let initialized_result = Parser::new(&initialized_source).parse();
    assert!(initialized_result.diagnostics.is_empty());
    let initialized = initialized_result
        .program
        .expect("program")
        .statements
        .into_iter()
        .next()
        .expect("statement");
    let Statement::Declaration {
        target,
        declared_type,
        value: Some(value),
        ..
    } = initialized
    else {
        panic!("expected initialized declaration");
    };
    assert_eq!(declared_type, DeclaredType::Scalar(ScalarType::Int));
    assert_eq!(target.text(&initialized_source), "count");
    assert!(matches!(value, Expression::Literal { .. }));

    let uninitialized = one_statement("str `未初始化`\n");
    assert!(matches!(
        uninitialized,
        Statement::Declaration {
            declared_type: DeclaredType::Scalar(ScalarType::Str),
            value: None,
            target,
            ..
        } if target.backticked
    ));
}

#[test]
/// 验证 `const` 的可选类型前缀和必需初始化表达式。
fn parses_const_declarations() {
    let inferred = one_statement("const PI = 3.14\n");
    assert!(matches!(
        inferred,
        Statement::ConstDeclaration {
            declared_type: None,
            value: Expression::Literal { .. },
            ..
        }
    ));

    let explicit = one_statement("const int LIMIT = 42");
    assert!(matches!(
        explicit,
        Statement::ConstDeclaration {
            declared_type: Some(ScalarType::Int),
            ..
        }
    ));
}

#[test]
/// 验证标量类型构造调用不会被误判为声明。
fn keeps_scalar_constructor_calls_as_expressions() {
    let statement = one_statement("value = bool(raw)\n");
    assert!(matches!(
        statement,
        Statement::Assignment {
            value: Expression::Call { .. },
            ..
        }
    ));
}

#[test]
/// 验证缺少常量初值和声明目标时提供稳定 P2 解析诊断。
fn diagnoses_incomplete_declarations() {
    let result = Parser::new(&SourceFile::from_text("const answer\nint = 1\n")).parse();
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&MISSING_CONST_VALUE_CODE));
    assert!(codes.contains(&INVALID_DECLARATION_TARGET_CODE));
}
