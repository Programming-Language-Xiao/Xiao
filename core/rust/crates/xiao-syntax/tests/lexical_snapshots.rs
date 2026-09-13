//! L0 词法规格快照的集成测试入口。

use serde::Deserialize;
use xiao_source::SourceFile;
use xiao_syntax::{Lexer, TokenKind};

/// 一个快照中记录的 Token 期望值。
#[derive(Debug, Deserialize)]
struct ExpectedToken {
    kind: String,
    start: usize,
    end: usize,
    text: String,
}

/// 一个 L0 源码与其期望扫描结果。
#[derive(Debug, Deserialize)]
struct Snapshot {
    source: String,
    tokens: Vec<ExpectedToken>,
    diagnostics: Vec<ExpectedDiagnostic>,
}

/// 一个稳定诊断的期望值。
#[derive(Debug, Deserialize)]
struct ExpectedDiagnostic {
    code: String,
    start: usize,
    end: usize,
}

/// 读取并验证一份 JSON 快照。
fn assert_snapshot(raw: &str) {
    let snapshot: Snapshot = serde_json::from_str(raw).expect("快照 JSON 必须有效");
    let source = SourceFile::from_text(&snapshot.source);
    let result = Lexer::new(&source).tokenize();

    assert_eq!(result.tokens.len(), snapshot.tokens.len());
    for (actual, expected) in result.tokens.iter().zip(snapshot.tokens.iter()) {
        assert_eq!(actual.kind().as_str(), expected.kind);
        assert_eq!(actual.span().start(), expected.start);
        assert_eq!(actual.span().end(), expected.end);
        assert_eq!(actual.text(&source), expected.text);
    }

    assert_eq!(result.diagnostics.len(), snapshot.diagnostics.len());
    for (actual, expected) in result.diagnostics.iter().zip(snapshot.diagnostics.iter()) {
        assert_eq!(actual.code(), expected.code);
        let span = actual.span().expect("L0 诊断必须关联源码区间");
        assert_eq!(span.start(), expected.start);
        assert_eq!(span.end(), expected.end);
    }
}

#[test]
/// 验证最小赋值 Token 流的顺序和区间。
fn minimal_assignment_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/01-lexical/minimal-assignment.json"
    )));
}

#[test]
/// 验证 CRLF 作为一个逻辑换行但保留两个字节区间。
fn crlf_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/01-lexical/crlf.json"
    )));
}

#[test]
/// 验证非法字符诊断不会阻止后续 Token 扫描。
fn invalid_character_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/01-lexical/invalid-character.json"
    )));
}

#[test]
/// 验证 L1 字面量的种类、原始文本和换行区间。
fn l1_literals_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/01-lexical/l1-literals.json"
    )));
}

#[test]
/// 验证 L1 保留字、分隔符、路径符号和最长运算符匹配。
fn l1_operators_and_delimiters_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/01-lexical/l1-operators-and-delimiters.json"
    )));
}

#[test]
/// 验证字符串转义错误仍能恢复到后续合法名称。
fn l1_invalid_string_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/01-lexical/l1-invalid-string.json"
    )));
}

#[test]
/// 验证不完整数字指数的诊断区间和错误恢复。
fn l1_invalid_number_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/01-lexical/l1-invalid-number.json"
    )));
}

#[test]
/// 确认快照仍覆盖 EOF Token，避免未使用的 TokenKind 导入退化。
fn eof_kind_is_stable() {
    assert!(TokenKind::Eof.is_eof());
}
