//! Xiao 词法 Token 的基础实现。
//!
//! 当前只实现 01 阶段的 L0 最小 Token 流。解析器、AST、缩进层和完整
//! 语言关键字会在后续里程碑中加入；本模块不会在词法阶段执行类型检查
//! 或程序代码。

use xiao_diagnostics::Diagnostic;
use xiao_source::{SourceError, SourceFile, SourcePosition, SourceSpan};

/// 非法 UTF-8 源文件的稳定诊断编号（从源码层重新导出）。
pub use xiao_source::INVALID_UTF8_CODE;

/// L0 非法字符的稳定诊断编号。
pub const INVALID_CHARACTER_CODE: &str = "X01-LEX-001";

/// L0 能够识别的 Token 种类。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    /// ASCII 普通标识符。
    Identifier,
    /// 十进制整数字面量（原始文本保留在源码区间中）。
    Integer,
    /// 赋值符号 `=`。
    Equal,
    /// LF 或 CRLF 产生的逻辑换行。
    Newline,
    /// 文件结束的零宽 Token。
    Eof,
    /// 无法识别的单个 Unicode 标量。
    Invalid,
}

impl TokenKind {
    /// 返回用于快照和机器输出的稳定名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identifier => "Identifier",
            Self::Integer => "Integer",
            Self::Equal => "Equal",
            Self::Newline => "Newline",
            Self::Eof => "Eof",
            Self::Invalid => "Invalid",
        }
    }

    /// 判断 Token 是否为文件结束。
    #[must_use]
    pub const fn is_eof(self) -> bool {
        matches!(self, Self::Eof)
    }
}

/// 一个带源码区间的词法 Token。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Token {
    kind: TokenKind,
    span: SourceSpan,
}

impl Token {
    /// 创建一个 Token。
    #[must_use]
    pub const fn new(kind: TokenKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }

    /// 返回 Token 种类。
    #[must_use]
    pub const fn kind(self) -> TokenKind {
        self.kind
    }

    /// 返回 Token 的源码区间。
    #[must_use]
    pub const fn span(self) -> SourceSpan {
        self.span
    }

    /// 从源码中取得 Token 的原始文本。
    #[must_use]
    pub fn text(self, source: &SourceFile) -> &str {
        source.slice(self.span)
    }

    /// 返回 Token 起点的行列位置。
    pub fn start_position(self, source: &SourceFile) -> Result<SourcePosition, SourceError> {
        source.position_at(self.span.start())
    }

    /// 返回 Token 终点的行列位置（终点不属于 Token）。
    pub fn end_position(self, source: &SourceFile) -> Result<SourcePosition, SourceError> {
        source.position_at(self.span.end())
    }
}

/// L0 词法诊断使用的统一结构化类型。
pub type LexDiagnostic = Diagnostic;

/// 一次完整词法扫描的结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexResult {
    /// 按源码顺序产生的 Token（包含 Invalid 和 Eof）。
    pub tokens: Vec<Token>,
    /// 扫描过程中收集的诊断。
    pub diagnostics: Vec<LexDiagnostic>,
}

/// L0 最小词法器。
pub struct Lexer<'source> {
    source: &'source SourceFile,
    offset: usize,
    diagnostics: Vec<LexDiagnostic>,
    emitted_eof: bool,
}

impl<'source> Lexer<'source> {
    /// 创建位于源码开头的词法器。
    #[must_use]
    pub fn new(source: &'source SourceFile) -> Self {
        Self {
            source,
            offset: 0,
            diagnostics: Vec::new(),
            emitted_eof: false,
        }
    }

    /// 返回当前扫描字节偏移。
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// 返回已经收集的诊断视图。
    #[must_use]
    pub fn diagnostics(&self) -> &[LexDiagnostic] {
        &self.diagnostics
    }

    /// 扫描并返回下一个 Token。
    ///
    /// EOF 之后再次调用仍返回同一位置的 EOF，不会继续改变游标。
    pub fn next_token(&mut self) -> Token {
        if self.emitted_eof {
            return self.eof_token();
        }

        self.skip_horizontal_whitespace();
        if self.offset >= self.source.len_bytes() {
            self.emitted_eof = true;
            return self.eof_token();
        }

        let start = self.offset;
        let bytes = self.source.as_bytes();
        match bytes[start] {
            b'\n' => {
                self.offset += 1;
                self.token(TokenKind::Newline, start, self.offset)
            }
            b'\r' if bytes.get(start + 1) == Some(&b'\n') => {
                self.offset += 2;
                self.token(TokenKind::Newline, start, self.offset)
            }
            b'=' => {
                self.offset += 1;
                self.token(TokenKind::Equal, start, self.offset)
            }
            b'0'..=b'9' => self.scan_integer(),
            byte if is_ascii_identifier_start(byte) => self.scan_identifier(),
            _ => self.scan_invalid(),
        }
    }

    /// 消费全部输入并返回 Token 与诊断。
    #[must_use]
    pub fn tokenize(mut self) -> LexResult {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let done = token.kind().is_eof();
            tokens.push(token);
            if done {
                break;
            }
        }
        LexResult {
            tokens,
            diagnostics: self.diagnostics,
        }
    }

    /// 跳过横向空白；Tab 在缩进和扫描阶段都按四个逻辑空格处理。
    fn skip_horizontal_whitespace(&mut self) {
        while let Some(character) = self.source.text()[self.offset..].chars().next() {
            if matches!(character, ' ' | '\t') {
                self.offset += character.len_utf8();
            } else {
                break;
            }
        }
    }

    /// 扫描一个 ASCII 标识符。
    fn scan_identifier(&mut self) -> Token {
        let start = self.offset;
        while let Some(byte) = self.source.as_bytes().get(self.offset).copied() {
            if is_ascii_identifier_continue(byte) {
                self.offset += 1;
            } else {
                break;
            }
        }
        self.token(TokenKind::Identifier, start, self.offset)
    }

    /// 扫描一个十进制整数字面量。
    fn scan_integer(&mut self) -> Token {
        let start = self.offset;
        while matches!(self.source.as_bytes().get(self.offset), Some(b'0'..=b'9')) {
            self.offset += 1;
        }
        self.token(TokenKind::Integer, start, self.offset)
    }

    /// 消费一个无法识别的 Unicode 标量并记录诊断。
    fn scan_invalid(&mut self) -> Token {
        let start = self.offset;
        let character = self.source.text()[start..]
            .chars()
            .next()
            .expect("invalid scan must start before EOF");
        self.offset += character.len_utf8();
        let token = self.token(TokenKind::Invalid, start, self.offset);
        self.diagnostics.push(Diagnostic::error_at(
            INVALID_CHARACTER_CODE,
            "x01.lex.invalid_character",
            token.span(),
            format!("无法识别的字符：{character:?}"),
        ));
        token
    }

    /// 创建一个已经验证属于源码的 Token。
    fn token(&self, kind: TokenKind, start: usize, end: usize) -> Token {
        let span = self
            .source
            .span(start, end)
            .expect("lexer offsets must remain valid UTF-8 boundaries");
        Token::new(kind, span)
    }

    /// 创建当前 EOF Token。
    fn eof_token(&self) -> Token {
        self.token(TokenKind::Eof, self.offset, self.offset)
    }
}

/// 判断 ASCII 标识符首字符。
fn is_ascii_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

/// 判断 ASCII 标识符后续字符。
fn is_ascii_identifier_continue(byte: u8) -> bool {
    is_ascii_identifier_start(byte) || byte.is_ascii_digit()
}

#[cfg(test)]
/// 覆盖 L0 Token 顺序、位置区间和错误恢复策略的单元测试。
mod tests {
    use super::{INVALID_CHARACTER_CODE, Lexer, TokenKind};
    use xiao_source::{SourceFile, SourcePosition};

    /// 提取一段源码的 Token 类型，便于快照断言。
    fn kinds(source: &str) -> Vec<TokenKind> {
        Lexer::new(&SourceFile::from_text(source))
            .tokenize()
            .tokens
            .into_iter()
            .map(|token| token.kind())
            .collect()
    }

    #[test]
    /// 确认最小赋值源码产生稳定的标识符、赋值、整数和 EOF。
    fn tokenizes_minimal_assignment() {
        let source = SourceFile::from_text("a = 1");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Identifier,
                TokenKind::Equal,
                TokenKind::Integer,
                TokenKind::Eof
            ]
        );
        assert_eq!(result.tokens[0].text(&source), "a");
        assert_eq!(result.tokens[2].text(&source), "1");
        assert_eq!(
            result.tokens[0]
                .start_position(&source)
                .expect("起点应有效"),
            SourcePosition::new(0, 1, 1)
        );
        assert_eq!(
            result.tokens[2].end_position(&source).expect("终点应有效"),
            SourcePosition::new(5, 1, 6)
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    /// 确认 LF 与 CRLF 都只产生一个换行 Token。
    fn emits_one_newline_for_lf_and_crlf() {
        assert_eq!(
            kinds("a\n"),
            vec![TokenKind::Identifier, TokenKind::Newline, TokenKind::Eof]
        );
        assert_eq!(
            kinds("a\r\n"),
            vec![TokenKind::Identifier, TokenKind::Newline, TokenKind::Eof]
        );
    }

    #[test]
    /// 确认没有真实换行的文件不会在 EOF 前补 Token。
    fn does_not_synthesize_newline_at_eof() {
        assert_eq!(kinds("a"), vec![TokenKind::Identifier, TokenKind::Eof]);
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    /// 确认 ASCII 标识符规则和前导零整数文本保持不变。
    fn accepts_ascii_identifier_and_leading_zero_integer() {
        let source = SourceFile::from_text("_name1 001");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(result.tokens[0].kind(), TokenKind::Identifier);
        assert_eq!(result.tokens[0].text(&source), "_name1");
        assert_eq!(result.tokens[1].kind(), TokenKind::Integer);
        assert_eq!(result.tokens[1].text(&source), "001");
    }

    #[test]
    /// 确认非法字符报告诊断后仍继续扫描后续整数。
    fn reports_invalid_character_and_keeps_scanning() {
        let source = SourceFile::from_text("a = § 1");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Identifier,
                TokenKind::Equal,
                TokenKind::Invalid,
                TokenKind::Integer,
                TokenKind::Eof
            ]
        );
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code(), INVALID_CHARACTER_CODE);
        assert_eq!(
            result.diagnostics[0]
                .span()
                .expect("诊断应包含区间")
                .start(),
            4
        );
    }

    #[test]
    /// 确认 Tab 作为横向空白被统一跳过。
    fn expands_horizontal_tabs_as_ignored_whitespace() {
        let result = Lexer::new(&SourceFile::from_text("\t\ta\t=\t1")).tokenize();
        assert_eq!(result.diagnostics.len(), 0);
        assert_eq!(result.tokens.len(), 4);
    }

    #[test]
    /// 确认未配对的回车不是合法换行。
    fn lone_carriage_return_is_invalid() {
        let result = Lexer::new(&SourceFile::from_text("a\rb")).tokenize();
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code(), INVALID_CHARACTER_CODE);
    }
}
