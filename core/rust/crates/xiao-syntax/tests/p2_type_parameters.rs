//! 20F 显式函数类型参数和 `set<T>` 注解语法回归。

use xiao_source::SourceFile;
use xiao_syntax::{FunctionTypeAnnotation, Parser, Statement, TypeTerm};

#[test]
/// 函数头、参数和返回值都能保留显式类型参数。
fn parses_generic_set_parameter_and_return() {
    let source =
        SourceFile::from_text("def f<T, U>(set<T> values, T value) -> set<U>\n    return set()\n");
    let result = Parser::new(&source).parse();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let Statement::Function {
        type_parameters,
        parameters,
        return_type,
        ..
    } = &result.program.unwrap().statements[0]
    else {
        panic!("函数")
    };
    assert_eq!(
        type_parameters
            .iter()
            .map(|name| name.unquoted_text(&source))
            .collect::<Vec<_>>(),
        ["T", "U"]
    );
    assert!(
        matches!(parameters[0].annotation, Some(FunctionTypeAnnotation::Set(ref set)) if matches!(&set.members[0], TypeTerm::TypeParameter(name) if name == "T"))
    );
    assert!(
        matches!(return_type, Some(FunctionTypeAnnotation::Set(set)) if matches!(&set.members[0], TypeTerm::TypeParameter(name) if name == "U"))
    );
}

#[test]
/// 空列表、重复名和标量关键字名使用 20F 稳定诊断。
fn rejects_invalid_type_parameter_lists() {
    for source in [
        "def f<>(int value) -> int\n    return value\n",
        "def f<T, T>(int value) -> int\n    return value\n",
        "def f<int>(int value) -> int\n    return value\n",
        "def f<file>(int value) -> int\n    return value\n",
        "def f<`file`>(int value) -> int\n    return value\n",
        "def f<T>(U value) -> int\n    return value\n",
        "def f<T U>(T value) -> T\n    return value\n",
        "def f<T,,U>(T value) -> T\n    return value\n",
    ] {
        let result = Parser::new(&SourceFile::from_text(source)).parse();
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code() == "X04-PARSE-007"),
            "{source:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
/// 显式类型参数只能出现在顶层函数头。
fn rejects_non_top_level_type_parameters() {
    let result = Parser::new(&SourceFile::from_text(
        "if true\n    def nested<T>(T value) -> T\n        return value\n",
    ))
    .parse();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "X04-PARSE-007"),
        "{:?}",
        result.diagnostics
    );
}

#[test]
/// 类型参数项不会泄漏到普通变量声明的集合类型语法。
fn rejects_type_parameter_in_variable_declaration() {
    let result = Parser::new(&SourceFile::from_text("set<T> values = set()\n")).parse();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "X03-PARSE-005"),
        "{:?}",
        result.diagnostics
    );
}
