//! P0 最小 AST 与解析器的规格快照入口。

use serde::Deserialize;
use xiao_source::SourceFile;
use xiao_syntax::INVALID_CHARACTER_CODE;
use xiao_syntax::{
    Expression, INVALID_ASSIGNMENT_TARGET_CODE, INVALID_EXPRESSION_CODE, LiteralKind,
    MISSING_ASSIGNMENT_VALUE_CODE, Parser, Statement, UNSUPPORTED_BLOCK_CODE,
    UNSUPPORTED_EXPRESSION_CODE,
};

/// 一个源码区间的 JSON 期望值。
#[derive(Debug, Deserialize)]
struct ExpectedSpan {
    /// 半开区间起点。
    start: usize,
    /// 半开区间终点。
    end: usize,
}

/// 一个名称表达式或赋值目标的 JSON 期望值。
#[derive(Debug, Deserialize)]
struct ExpectedName {
    /// 名称区间起点。
    start: usize,
    /// 名称区间终点。
    end: usize,
    /// 名称原始文本。
    text: String,
    /// 是否为反引号名称。
    backticked: bool,
}

/// 一个 P0 表达式的 JSON 期望值。
#[derive(Debug, Deserialize)]
struct ExpectedExpression {
    /// 表达式类别：`literal` 或 `name`。
    kind: String,
    /// 字面量类别；名称表达式为空。
    literal_kind: Option<String>,
    /// 表达式区间起点。
    start: usize,
    /// 表达式区间终点。
    end: usize,
    /// 原始表达式文本。
    text: String,
    /// 名称是否由反引号包裹。
    backticked: Option<bool>,
}

/// 一个 P0 语句的 JSON 期望值。
#[derive(Debug, Deserialize)]
struct ExpectedStatement {
    /// 语句类别：`expression` 或 `assignment`。
    kind: String,
    /// 语句区间起点。
    start: usize,
    /// 语句区间终点。
    end: usize,
    /// 挂接到语句前的文档注释区间。
    leading_docs: Vec<ExpectedSpan>,
    /// 表达式语句的表达式。
    expression: Option<ExpectedExpression>,
    /// 赋值语句的左侧名称。
    target: Option<ExpectedName>,
    /// 赋值语句的右侧表达式。
    value: Option<ExpectedExpression>,
}

/// 一个解析诊断的 JSON 期望值。
#[derive(Debug, Deserialize)]
struct ExpectedDiagnostic {
    /// 稳定诊断编号。
    code: String,
    /// 稳定消息键。
    message_id: String,
    /// 诊断区间起点。
    start: usize,
    /// 诊断区间终点。
    end: usize,
}

/// 一份 P0 AST/诊断快照。
#[derive(Debug, Deserialize)]
struct Snapshot {
    /// 输入源码。
    source: String,
    /// 成功恢复出的语句。
    statements: Vec<ExpectedStatement>,
    /// 未关联到语句的文档注释。
    orphan_doc_comments: Vec<ExpectedSpan>,
    /// 词法和解析诊断。
    diagnostics: Vec<ExpectedDiagnostic>,
}

/// 读取并验证一份 P0 JSON 快照。
fn assert_snapshot(raw: &str) {
    let snapshot: Snapshot = serde_json::from_str(raw).expect("P0 快照 JSON 必须有效");
    let source = SourceFile::from_text(&snapshot.source);
    let result = Parser::new(&source).parse();
    let program = result.program.expect("P0 应返回程序根节点");

    assert_eq!(program.span.start(), 0);
    assert_eq!(program.span.end(), source.len_bytes());
    assert_eq!(program.statements.len(), snapshot.statements.len());
    for (actual, expected) in program.statements.iter().zip(snapshot.statements.iter()) {
        assert_eq!(actual.span().start(), expected.start);
        assert_eq!(actual.span().end(), expected.end);
        assert_eq!(actual.leading_docs().len(), expected.leading_docs.len());
        for (actual_doc, expected_doc) in actual.leading_docs().iter().zip(&expected.leading_docs) {
            assert_eq!(actual_doc.start(), expected_doc.start);
            assert_eq!(actual_doc.end(), expected_doc.end);
        }
        match (actual, expected.kind.as_str()) {
            (Statement::Expression { expression, .. }, "expression") => {
                assert!(expected.target.is_none());
                assert!(expected.value.is_none());
                assert_expression(
                    expression,
                    expected.expression.as_ref().expect("缺少表达式"),
                    &source,
                );
            }
            (Statement::Assignment { target, value, .. }, "assignment") => {
                assert!(expected.expression.is_none());
                assert_name(
                    target,
                    expected.target.as_ref().expect("缺少赋值目标"),
                    &source,
                );
                assert_expression(
                    value,
                    expected.value.as_ref().expect("缺少赋值右值"),
                    &source,
                );
            }
            _ => panic!("语句类别或 AST 形状不匹配：{expected:?}"),
        }
    }

    assert_eq!(
        program.orphan_doc_comments.len(),
        snapshot.orphan_doc_comments.len()
    );
    for (actual, expected) in program
        .orphan_doc_comments
        .iter()
        .zip(&snapshot.orphan_doc_comments)
    {
        assert_eq!(actual.start(), expected.start);
        assert_eq!(actual.end(), expected.end);
    }

    assert_eq!(result.diagnostics.len(), snapshot.diagnostics.len());
    for (actual, expected) in result.diagnostics.iter().zip(&snapshot.diagnostics) {
        assert_eq!(actual.code(), expected.code);
        assert_eq!(actual.message_id(), expected.message_id);
        let span = actual.span().expect("P0 诊断必须关联源码区间");
        assert_eq!(span.start(), expected.start);
        assert_eq!(span.end(), expected.end);
    }
}

/// 验证字面量类别和原始文本都被保留。
fn assert_expression(actual: &Expression, expected: &ExpectedExpression, source: &SourceFile) {
    assert_eq!(actual.span().start(), expected.start);
    assert_eq!(actual.span().end(), expected.end);
    assert_eq!(source.slice(actual.span()), expected.text);
    match (actual, expected.kind.as_str()) {
        (Expression::Literal { kind, .. }, "literal") => {
            assert_eq!(
                Some(*kind),
                expected_literal_kind(expected.literal_kind.as_deref())
            );
            assert!(expected.backticked.is_none());
        }
        (Expression::Name(name), "name") => {
            assert_eq!(
                name.backticked,
                expected.backticked.expect("名称缺少 backticked")
            );
            assert_eq!(name.text(source), expected.text);
            assert!(expected.literal_kind.is_none());
        }
        _ => panic!("表达式类别或 AST 形状不匹配：{expected:?}"),
    }
}

/// 验证名称目标的源码位置和反引号标记。
fn assert_name(actual: &xiao_syntax::Name, expected: &ExpectedName, source: &SourceFile) {
    assert_eq!(actual.span.start(), expected.start);
    assert_eq!(actual.span.end(), expected.end);
    assert_eq!(actual.text(source), expected.text);
    assert_eq!(actual.backticked, expected.backticked);
}

/// 将快照中的字面量类别转换为公开枚举。
fn expected_literal_kind(kind: Option<&str>) -> Option<LiteralKind> {
    match kind {
        Some("Integer") => Some(LiteralKind::Integer),
        Some("Float") => Some(LiteralKind::Float),
        Some("String") => Some(LiteralKind::String),
        Some("Boolean") => Some(LiteralKind::Boolean),
        Some("None") => Some(LiteralKind::None),
        Some(other) => panic!("未知字面量类别：{other}"),
        None => None,
    }
}

#[test]
/// 验证 P0 五类字面量表达式。
fn p0_literals_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/02-parser/p0-literals.json"
    )));
}

#[test]
/// 验证普通名称和 UTF-8 反引号名称表达式。
fn p0_names_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/02-parser/p0-names.json"
    )));
}

#[test]
/// 验证普通和反引号名称的简单赋值。
fn p0_assignments_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/02-parser/p0-assignments.json"
    )));
}

#[test]
/// 验证文档注释跨空行关联以及孤立注释保留。
fn p0_doc_comments_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/02-parser/p0-doc-comments.json"
    )));
}

#[test]
/// 验证解析错误恢复后仍能得到后续合法语句。
fn p0_errors_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/02-parser/p0-errors.json"
    )));
}

#[test]
/// 验证缩进代码块在 P0 被拒绝且后续顶层语句仍可解析。
fn rejects_indented_block_and_keeps_following_statement() {
    let source = SourceFile::from_text("a = 1\n    nested = 2\ntop = 3");
    let result = Parser::new(&source).parse();
    let program = result.program.expect("应返回程序根节点");
    assert_eq!(program.statements.len(), 2);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code(), UNSUPPORTED_BLOCK_CODE);
}

#[test]
/// 验证独立非法语法的诊断编号和错误恢复边界。
fn reports_each_p0_boundary_error() {
    let source = SourceFile::from_text("~\n1 = 2\na =\nb ~ c\nnext = 4");
    let result = Parser::new(&source).parse();
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        vec![
            INVALID_EXPRESSION_CODE,
            INVALID_ASSIGNMENT_TARGET_CODE,
            MISSING_ASSIGNMENT_VALUE_CODE,
            UNSUPPORTED_EXPRESSION_CODE,
        ]
    );
    let program = result.program.expect("应返回程序根节点");
    assert_eq!(program.statements.len(), 1);
}

#[test]
/// 确认没有真实尾部换行时，最后一条语句仍以 EOF 结束。
fn accepts_statement_terminated_by_eof() {
    let source = SourceFile::from_text("answer = 42");
    let result = Parser::new(&source).parse();
    assert!(result.is_success());
    assert_eq!(result.program.expect("应有程序").statements.len(), 1);
}

#[test]
/// 确认词法错误会被保留，但不会被解析器重复报告。
fn preserves_lexical_diagnostic_and_recovers() {
    let source = SourceFile::from_text("§\nanswer = 1");
    let result = Parser::new(&source).parse();
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code(), INVALID_CHARACTER_CODE);
    assert_eq!(
        result.program.expect("应返回程序根节点").statements.len(),
        1
    );
}

#[test]
/// 确认空文件、重复空行和普通注释行不会凭空产生语句或诊断。
fn accepts_empty_and_repeated_newlines() {
    let source = SourceFile::from_text("\n\n# comment\n\n");
    let result = Parser::new(&source).parse();
    assert!(result.is_success());
    let program = result.program.expect("应返回程序根节点");
    assert!(program.is_empty());
    assert_eq!(program.span.end(), source.len_bytes());
}

#[test]
/// 确认 CRLF 只作为逻辑边界，AST 区间不包含两个字节的换行符。
fn preserves_crlf_statement_boundaries() {
    let source = SourceFile::from_text("a = 1\r\nb = 2");
    let result = Parser::new(&source).parse();
    assert!(result.is_success());
    let program = result.program.expect("应返回程序根节点");
    assert_eq!(program.statements.len(), 2);
    assert_eq!(program.statements[0].span().start(), 0);
    assert_eq!(program.statements[0].span().end(), 5);
    assert_eq!(program.statements[1].span().start(), 7);
    assert_eq!(program.statements[1].span().end(), 12);
}

#[test]
/// 确认 P1 已开放调用和索引，并继续恢复后续顶层语句。
fn accepts_calls_and_indexes() {
    let source = SourceFile::from_text("call()\nvalue[0]\nvalid = 1");
    let result = Parser::new(&source).parse();
    assert!(result.diagnostics.is_empty());
    assert_eq!(
        result.program.expect("应返回程序根节点").statements.len(),
        3
    );
}

#[test]
/// 确认文档注释也能挂接到独立名称表达式，而不只挂接到赋值。
fn attaches_docs_to_expression_statement() {
    let source = SourceFile::from_text("### name docs ###\nname");
    let result = Parser::new(&source).parse();
    let program = result.program.expect("应返回程序根节点");
    assert_eq!(program.orphan_doc_comments.len(), 0);
    assert_eq!(program.statements.len(), 1);
    assert_eq!(program.statements[0].leading_docs().len(), 1);
}

/// 去掉行注释，避免注释里提到的模块名被算成依赖。
fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 断言文件没有把 `module` 当作完整标识符引用。
///
/// 只找 `use crate::模块::` 这类子串拦不住等价写法：`use crate::{模块}`、
/// `use crate::{模块, 另一个}` 和 `crate::模块::…` 的全限定路径都不含该子串。
/// 这里先去掉注释，再按标识符切词比较，所以 `xiao_syntax` 这类同前缀名字
/// 不会误报。`xiao-bytecode` 侧的 `assert_no_module_reference` 是同一套做法。
fn assert_no_module_reference(source_name: &str, source: &str, module: &str) {
    let code = strip_line_comments(source);
    let referenced = code
        .split(|character: char| !(character.is_alphanumeric() || character == '_'))
        .any(|token| token == module);
    assert!(!referenced, "{source_name} 不得依赖 {module}");
}

#[test]
/// 锁住解析器门面与语句扩展的边界，避免大段语句实现回流到 `parser.rs`。
fn parser_statement_split_keeps_dependency_boundary() {
    let facade = include_str!("../src/parser.rs");
    let statements = include_str!("../src/parser/statements.rs");
    let imports = include_str!("../src/parser/imports.rs");

    assert!(facade.contains("mod statements;"));
    assert!(facade.contains("#[path = \"parser/imports.rs\"]"));
    assert!(!facade.contains("#[path = \"parser/statements.rs\"]"));
    for signature in [
        "fn parse_statement(",
        "fn parse_table_statement(",
        "fn parse_function_statement(",
        "fn parse_if_statement(",
        "fn parse_try_statement(",
    ] {
        assert!(
            !facade.contains(signature),
            "语句实现不得回流到 parser.rs: {signature}"
        );
    }
    assert!(statements.contains("pub(super) fn parse_statement("));
    assert!(statements.contains("use crate::parser::"));
    // 扩展只消费 Token、诊断和公开 AST：不得直接拿词法器或类型层。
    assert_no_module_reference("parser/statements.rs", statements, "lexer");
    assert_no_module_reference("parser/statements.rs", statements, "xiao_types");
    // 00F 声明 import 扩展不得反向依赖语句扩展，这里锁住它。
    assert_no_module_reference("parser/imports.rs", imports, "statements");
    assert_no_module_reference("parser/imports.rs", imports, "xiao_types");
}
