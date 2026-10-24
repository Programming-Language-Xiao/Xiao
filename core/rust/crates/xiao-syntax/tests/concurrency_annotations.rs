//! 22B D-1 乙并发专用类型注解语法回归。

use xiao_source::SourceFile;
use xiao_syntax::{DeclaredType, FunctionTypeAnnotation, Parser, Statement};

#[test]
fn parses_concurrency_declarations() {
    let source = SourceFile::from_text(
        "thread<int> worker\nshared<str> text\nruntime rt\npool jobs\n",
    );
    let result = Parser::new(&source).parse();
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    let statements = result.program.expect("program").statements;
    assert!(matches!(
        statements[0],
        Statement::Declaration {
            declared_type: DeclaredType::Thread(ref inner),
            ..
        } if matches!(inner.as_ref(), FunctionTypeAnnotation::Scalar(xiao_syntax::ScalarType::Int))
    ));
    assert!(matches!(
        statements[1],
        Statement::Declaration {
            declared_type: DeclaredType::Shared(ref inner),
            ..
        } if matches!(inner.as_ref(), FunctionTypeAnnotation::Scalar(xiao_syntax::ScalarType::Str))
    ));
    assert!(matches!(
        statements[2],
        Statement::Declaration {
            declared_type: DeclaredType::Runtime,
            ..
        }
    ));
    assert!(matches!(
        statements[3],
        Statement::Declaration {
            declared_type: DeclaredType::Pool,
            ..
        }
    ));
}

#[test]
fn parses_concurrency_function_annotations_and_stable_text() {
    let source = SourceFile::from_text(
        "def f(thread<int> input, shared<str> text, runtime rt) -> lock_guard<int>\n    return none\n",
    );
    let result = Parser::new(&source).parse();
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    let Statement::Function {
        parameters,
        return_type: Some(return_type),
        ..
    } = &result.program.expect("program").statements[0]
    else {
        panic!("expected function");
    };
    assert!(matches!(
        parameters[0].annotation,
        Some(FunctionTypeAnnotation::Thread(ref inner))
            if matches!(inner.as_ref(), FunctionTypeAnnotation::Scalar(xiao_syntax::ScalarType::Int))
    ));
    assert!(matches!(
        parameters[1].annotation,
        Some(FunctionTypeAnnotation::Shared(ref inner))
            if matches!(inner.as_ref(), FunctionTypeAnnotation::Scalar(xiao_syntax::ScalarType::Str))
    ));
    assert_eq!(return_type.as_string(), "lock_guard<int>");
}

#[test]
fn rejects_unknown_generic_concurrency_spelling() {
    let source = SourceFile::from_text("foo<int> value\n");
    let result = Parser::new(&source).parse();
    assert!(!result.is_success());
    assert!(result.program.is_some());
}
