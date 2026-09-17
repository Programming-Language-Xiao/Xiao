//! C2-B 显式异构集合类型注解的语法规格测试。

use xiao_source::SourceFile;
use xiao_syntax::{DeclaredType, Expression, Parser, Statement, TypeTerm};
use xiao_syntax::{
    INVALID_SET_TYPE_ANNOTATION_CODE, MISSING_DELIMITER_CODE, UNSUPPORTED_CONST_SET_TYPE_CODE,
    UNSUPPORTED_SET_TYPE_PATH_CODE,
};

/// 解析一段源码并要求得到完整程序。
fn parse(source: &str) -> xiao_syntax::ParseResult {
    Parser::new(&SourceFile::from_text(source)).parse()
}

#[test]
/// `set<T | U>` 应保留类型项顺序、重复项和完整源码节点信息。
fn parses_heterogeneous_set_annotation() {
    let result = parse("set<int | str | bool> values = {1, \"x\", true}\n");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let statement = &result.program.expect("program").statements[0];
    let Statement::Declaration {
        declared_type: DeclaredType::Set(annotation),
        value: Some(Expression::SetLiteral { .. }),
        constraint_path: None,
        ..
    } = statement
    else {
        panic!("expected heterogeneous set declaration");
    };
    assert_eq!(
        annotation.members,
        vec![
            TypeTerm::Scalar(xiao_syntax::ScalarType::Int),
            TypeTerm::Scalar(xiao_syntax::ScalarType::Str),
            TypeTerm::Scalar(xiao_syntax::ScalarType::Bool),
        ]
    );
    assert!(annotation.span.start() < annotation.span.end());
}

#[test]
/// 注解中的软换行可被接受，`set()` 仍然是普通调用表达式而非声明。
fn accepts_multiline_annotation_and_keeps_constructor_call() {
    let result = parse("set<\n    int\n    | str\n> values = set()\nempty = set()\n");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let program = result.program.expect("program");
    assert!(matches!(
        &program.statements[0],
        Statement::Declaration {
            declared_type: DeclaredType::Set(_),
            value: Some(Expression::Call { .. }),
            ..
        }
    ));
    assert!(matches!(
        &program.statements[1],
        Statement::Assignment {
            value: Expression::Call { .. },
            ..
        }
    ));
}

#[test]
/// `none` 可以作为类型并集成员，重复类型项由类型层负责规范化。
fn accepts_none_and_duplicate_terms() {
    let result = parse("set<none | int | none> values\n");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let Statement::Declaration {
        declared_type: DeclaredType::Set(annotation),
        ..
    } = &result.program.expect("program").statements[0]
    else {
        panic!("expected set declaration");
    };
    assert_eq!(annotation.members.len(), 3);
    assert!(
        annotation
            .members
            .iter()
            .filter(|term| **term == TypeTerm::None)
            .count()
            == 2
    );
}

#[test]
/// 空并集、非法类型项、集合路径和 `const` 集合都应产生稳定解析诊断。
fn diagnoses_unsupported_set_annotation_forms() {
    let result = parse(
        "set<> empty\nset<array> bad\nset<int> values[0]\nconst set<int> frozen = set()\nnext = 1\n",
    );
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&INVALID_SET_TYPE_ANNOTATION_CODE));
    assert!(codes.contains(&UNSUPPORTED_SET_TYPE_PATH_CODE));
    assert!(codes.contains(&UNSUPPORTED_CONST_SET_TYPE_CODE));
    let program = result.program.expect("recovered program");
    assert!(program.statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::Assignment {
                value: Expression::Literal { .. },
                ..
            }
        )
    }));
}

#[test]
/// 缺少闭尖括号或并集右项时应恢复到后续语句，不吞掉合法代码。
fn recovers_incomplete_set_annotation() {
    let result = parse("set<int | str values = set()\nset<int | > broken\nnext = 1\n");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == MISSING_DELIMITER_CODE)
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == INVALID_SET_TYPE_ANNOTATION_CODE)
    );
    let program = result.program.expect("recovered program");
    let source =
        SourceFile::from_text("set<int | str values = set()\nset<int | > broken\nnext = 1\n");
    assert!(program.statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::Assignment {
                target,
                value: Expression::Literal { .. }, ..
            } if target.unquoted_text(&source) == "next"
        )
    }));
}

#[test]
/// 带空格的 `set < number` 仍按普通比较表达式解析。
fn does_not_misclassify_set_comparison() {
    let result = parse("set < 1\n");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(matches!(
        &result.program.expect("program").statements[0],
        Statement::Expression {
            expression: Expression::Binary { .. },
            ..
        }
    ));
}
