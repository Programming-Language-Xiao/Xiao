//! Xiao L2 词法器与缩进状态机。
//!
//! 词法器只负责把源码转换为稳定 Token 流和结构化诊断；它不解析表达式、推断
//! 类型或执行程序。状态通过显式字段维护，便于后续模块独立测试。

use std::collections::VecDeque;

use xiao_diagnostics::Diagnostic;
use xiao_source::{SourceFile, SourceSpan};

use crate::diagnostics::*;
use crate::token::{KeywordKind, LexDiagnostic, Token, TokenKind};

/// 一次完整词法扫描的结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexResult {
    /// 按源码顺序产生的 Token（包含 Invalid 和 Eof）。
    pub tokens: Vec<Token>,
    /// 扫描过程中收集的诊断。
    pub diagnostics: Vec<LexDiagnostic>,
}

/// 一个仍然打开的代码分隔符。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OpenDelimiter {
    /// 左圆括号。
    Parenthesis(SourceSpan),
    /// 左方括号。
    Bracket(SourceSpan),
    /// 左花括号。
    Brace(SourceSpan),
}

impl OpenDelimiter {
    /// 返回该分隔符对应的显示名称。
    fn name(self) -> &'static str {
        match self {
            Self::Parenthesis(_) => "(",
            Self::Bracket(_) => "[",
            Self::Brace(_) => "{",
        }
    }

    /// 判断一个关闭 Token 是否与该分隔符匹配。
    fn matches(self, kind: TokenKind) -> bool {
        matches!(
            (self, kind),
            (Self::Parenthesis(_), TokenKind::RightParen)
                | (Self::Bracket(_), TokenKind::RightBracket)
                | (Self::Brace(_), TokenKind::RightBrace)
        )
    }
}

/// Xiao L2 词法器。
pub struct Lexer<'source> {
    source: &'source SourceFile,
    offset: usize,
    diagnostics: Vec<LexDiagnostic>,
    pending: VecDeque<Token>,
    indent_stack: Vec<usize>,
    delimiters: Vec<OpenDelimiter>,
    at_line_start: bool,
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
            pending: VecDeque::new(),
            indent_stack: vec![0],
            delimiters: Vec::new(),
            at_line_start: true,
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
        loop {
            if let Some(token) = self.pending.pop_front() {
                return token;
            }
            if self.emitted_eof {
                return self.eof_token();
            }
            if self.at_line_start {
                self.prepare_line_start();
                if !self.pending.is_empty() {
                    continue;
                }
            }
            if !self.at_line_start {
                self.skip_horizontal_whitespace();
            }
            if self.offset >= self.source.len_bytes() {
                self.finish_eof();
                continue;
            }

            let start = self.offset;
            let bytes = self.source.as_bytes();
            let token = match bytes[start] {
                b'\n' => {
                    self.offset += 1;
                    self.token(TokenKind::Newline, start, self.offset)
                }
                b'\r' if bytes.get(start + 1) == Some(&b'\n') => {
                    self.offset += 2;
                    self.token(TokenKind::Newline, start, self.offset)
                }
                b'#' if bytes.get(start + 1) == Some(&b'#')
                    && bytes.get(start + 2) == Some(&b'#') =>
                {
                    self.scan_doc_comment()
                }
                b'#' => {
                    self.skip_line_comment();
                    continue;
                }
                b'`' => self.scan_backtick_identifier(),
                b'\'' | b'"' => self.scan_string(),
                b'0'..=b'9' => self.scan_integer(),
                b'.' if bytes
                    .get(start + 1)
                    .is_some_and(|byte| byte.is_ascii_digit()) =>
                {
                    self.scan_fractional_number()
                }
                byte if is_ascii_identifier_start(byte) => self.scan_identifier_or_keyword(),
                b'=' => self.scan_one_or_two(b'=', b'=', TokenKind::EqualEqual, TokenKind::Equal),
                b'!' => self.scan_bang(),
                b'<' => self.scan_one_or_two(b'<', b'=', TokenKind::LessEqual, TokenKind::Less),
                b'>' => {
                    self.scan_one_or_two(b'>', b'=', TokenKind::GreaterEqual, TokenKind::Greater)
                }
                b'+' => self.scan_one_or_two(b'+', b'=', TokenKind::PlusEqual, TokenKind::Plus),
                b'-' => self.scan_minus(),
                b'*' => self.scan_star(),
                b'&' => self.scan_one_or_two(
                    b'&',
                    b'=',
                    TokenKind::AmpersandEqual,
                    TokenKind::Ampersand,
                ),
                b'^' => self.scan_one_or_two(b'^', b'=', TokenKind::CaretEqual, TokenKind::Caret),
                b'/' => self.scan_slash(),
                b'%' => {
                    self.scan_one_or_two(b'%', b'=', TokenKind::PercentEqual, TokenKind::Percent)
                }
                b'|' => self.scan_single(TokenKind::Pipe),
                b'(' => self.scan_single(TokenKind::LeftParen),
                b')' => self.scan_single(TokenKind::RightParen),
                b'[' => self.scan_single(TokenKind::LeftBracket),
                b']' => self.scan_single(TokenKind::RightBracket),
                b'{' => self.scan_single(TokenKind::LeftBrace),
                b'}' => self.scan_single(TokenKind::RightBrace),
                b',' => self.scan_single(TokenKind::Comma),
                b':' => self.scan_single(TokenKind::Colon),
                b'.' => self.scan_single(TokenKind::Dot),
                b'~' => self.scan_single(TokenKind::Tilde),
                b'?' => self.scan_single(TokenKind::Question),
                b'@' => self.scan_single(TokenKind::At),
                b'$' => self.scan_single(TokenKind::Dollar),
                _ => self.scan_invalid(),
            };
            self.observe_token(&token);
            self.at_line_start = token.kind() == TokenKind::Newline;
            return token;
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

    /// 处理一行开头的空白、注释和缩进，并把需要延迟发出的 Token 放入队列。
    fn prepare_line_start(&mut self) {
        let whitespace_start = self.offset;
        let logical_width = self.consume_line_prefix();
        if self.offset >= self.source.len_bytes() {
            // 只有空白的文件尾部不建立新的缩进层；EOF 阶段会弹出已有层。
            return;
        }

        let bytes = self.source.as_bytes();
        if is_line_break_at(bytes, self.offset) {
            // 空行保留其物理换行，但不改变缩进栈。
            return;
        }
        if bytes.get(self.offset) == Some(&b'#') {
            if starts_doc_comment(bytes, self.offset) {
                let token = self.scan_doc_comment();
                self.pending.push_back(token);
                self.at_line_start = false;
            } else {
                self.skip_line_comment();
                // 保持行首状态，下一轮会发出该行的 Newline 或处理 EOF。
            }
            return;
        }

        if self.delimiters.is_empty() {
            self.apply_indentation(whitespace_start, self.offset, logical_width);
        }
        self.at_line_start = false;
    }

    /// 消费行首的空格和 Tab，并返回展开后的逻辑宽度。
    fn consume_line_prefix(&mut self) -> usize {
        let mut width = 0;
        while let Some(byte) = self.source.as_bytes().get(self.offset).copied() {
            match byte {
                b' ' => {
                    self.offset += 1;
                    width += 1;
                }
                b'\t' => {
                    self.offset += 1;
                    width += 4;
                }
                _ => break,
            }
        }
        width
    }

    /// 根据当前行逻辑宽度更新缩进栈，并生成 `Indent`/`Dedent` Token。
    fn apply_indentation(&mut self, start: usize, end: usize, width: usize) {
        let current = *self.indent_stack.last().expect("缩进栈始终至少包含根层级");
        let span = self
            .source
            .span(start, end)
            .expect("行首空白必须是合法源码区间");
        let width_invalid = width % 4 != 0;
        if width_invalid {
            self.push_indent_diagnostic(span, width);
        }

        // 非法宽度只能恢复到已经出现的层级，不能因为向下取整而偷偷创建新层。
        let target = if width_invalid {
            self.indent_stack
                .iter()
                .copied()
                .filter(|level| *level <= width)
                .max()
                .unwrap_or(0)
        } else {
            width
        };

        if target > current && !width_invalid {
            self.indent_stack.push(target);
            self.pending.push_back(Token::new(TokenKind::Indent, span));
            return;
        }
        if target == current {
            return;
        }

        while self.indent_stack.len() > 1 && *self.indent_stack.last().expect("缩进栈非空") > target
        {
            self.indent_stack.pop();
            self.pending
                .push_back(self.zero_width_token(TokenKind::Dedent, end));
        }
        let resolved = *self.indent_stack.last().expect("弹栈后仍保留根层级");
        if resolved != target && !width_invalid {
            self.push_indent_diagnostic(span, width);
        }
    }

    /// 记录缩进宽度错误；恢复策略由调用方归入最近的已知层级。
    fn push_indent_diagnostic(&mut self, span: SourceSpan, width: usize) {
        self.diagnostics.push(Diagnostic::error_at(
            INCONSISTENT_INDENT_CODE,
            "x01.lex.inconsistent_indent",
            span,
            format!("缩进宽度 {width} 不能对应四个空格的层级"),
        ));
    }

    /// 扫描一个反引号包裹的 UTF-8 名称。
    fn scan_backtick_identifier(&mut self) -> Token {
        let start = self.offset;
        self.offset += 1;
        let mut invalid_escape = None;
        while self.offset < self.source.len_bytes() {
            let character = self.source.text()[self.offset..]
                .chars()
                .next()
                .expect("反引号扫描偏移必须位于字符边界");
            match character {
                '`' => {
                    self.offset += 1;
                    let token = self.token(TokenKind::BacktickIdentifier, start, self.offset);
                    if let Some((escape_start, escape_end)) = invalid_escape {
                        self.push_backtick_escape_diagnostic(escape_start, escape_end);
                        return Token::new(TokenKind::Invalid, token.span());
                    }
                    return token;
                }
                '\n' | '\r' => {
                    let token = self.token(TokenKind::Invalid, start, self.offset);
                    self.diagnostics.push(Diagnostic::error_at(
                        UNTERMINATED_BACKTICK_CODE,
                        "x01.lex.unterminated_backtick",
                        token.span(),
                        "反引号名称不能跨越换行且没有闭合".to_string(),
                    ));
                    return token;
                }
                '\\' => {
                    let escape_start = self.offset;
                    self.offset += 1;
                    if self.offset >= self.source.len_bytes() {
                        let token = self.token(TokenKind::Invalid, start, self.offset);
                        self.diagnostics.push(Diagnostic::error_at(
                            UNTERMINATED_BACKTICK_CODE,
                            "x01.lex.unterminated_backtick",
                            token.span(),
                            "反引号名称以未完成的转义结尾".to_string(),
                        ));
                        return token;
                    }
                    let escaped = self.source.text()[self.offset..]
                        .chars()
                        .next()
                        .expect("转义目标必须位于字符边界");
                    if matches!(escaped, '\n' | '\r') {
                        // 反引号名称禁止跨物理行；把换行退回给主扫描循环。
                        self.offset = escape_start + 1;
                        let token = self.token(TokenKind::Invalid, start, self.offset);
                        self.diagnostics.push(Diagnostic::error_at(
                            UNTERMINATED_BACKTICK_CODE,
                            "x01.lex.unterminated_backtick",
                            token.span(),
                            "反引号名称不能跨越换行且没有闭合".to_string(),
                        ));
                        return token;
                    }
                    self.offset += escaped.len_utf8();
                    if !matches!(escaped, '`' | '\\') && invalid_escape.is_none() {
                        invalid_escape = Some((escape_start, self.offset));
                    }
                }
                _ => self.offset += character.len_utf8(),
            }
        }

        let token = self.token(TokenKind::Invalid, start, self.offset);
        self.diagnostics.push(Diagnostic::error_at(
            UNTERMINATED_BACKTICK_CODE,
            "x01.lex.unterminated_backtick",
            token.span(),
            "反引号名称没有闭合".to_string(),
        ));
        token
    }

    /// 扫描一个文档注释，直到下一个 `###` 标记。
    fn scan_doc_comment(&mut self) -> Token {
        let start = self.offset;
        self.offset += 3;
        while self.offset < self.source.len_bytes() {
            if starts_doc_comment(self.source.as_bytes(), self.offset) {
                self.offset += 3;
                return self.token(TokenKind::DocComment, start, self.offset);
            }
            let character = self.source.text()[self.offset..]
                .chars()
                .next()
                .expect("文档注释扫描偏移必须位于字符边界");
            self.offset += character.len_utf8();
        }
        let token = self.token(TokenKind::Invalid, start, self.offset);
        self.diagnostics.push(Diagnostic::error_at(
            UNTERMINATED_DOC_COMMENT_CODE,
            "x01.lex.unterminated_doc_comment",
            token.span(),
            "文档注释没有闭合的 ### 标记".to_string(),
        ));
        token
    }

    /// 跳过当前物理行的普通注释，但不消费换行符。
    fn skip_line_comment(&mut self) {
        while self.offset < self.source.len_bytes()
            && !is_line_break_at(self.source.as_bytes(), self.offset)
        {
            let character = self.source.text()[self.offset..]
                .chars()
                .next()
                .expect("注释扫描偏移必须位于字符边界");
            self.offset += character.len_utf8();
        }
    }

    /// 在指定字节偏移创建零宽 Token。
    fn zero_width_token(&self, kind: TokenKind, offset: usize) -> Token {
        self.token(kind, offset, offset)
    }

    /// 记录非法反引号转义。
    fn push_backtick_escape_diagnostic(&mut self, start: usize, end: usize) {
        let span = self
            .source
            .span(start, end)
            .expect("反引号转义区间必须是合法源码边界");
        self.diagnostics.push(Diagnostic::error_at(
            INVALID_BACKTICK_ESCAPE_CODE,
            "x01.lex.invalid_backtick_escape",
            span,
            "反引号名称只允许转义反引号或反斜杠".to_string(),
        ));
    }

    /// 记录分隔符开闭关系，并在不匹配时生成诊断。
    fn observe_token(&mut self, token: &Token) {
        match token.kind() {
            TokenKind::LeftParen => self
                .delimiters
                .push(OpenDelimiter::Parenthesis(token.span())),
            TokenKind::LeftBracket => self.delimiters.push(OpenDelimiter::Bracket(token.span())),
            TokenKind::LeftBrace => self.delimiters.push(OpenDelimiter::Brace(token.span())),
            TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                if let Some(open) = self.delimiters.last().copied() {
                    if open.matches(token.kind()) {
                        self.delimiters.pop();
                    } else {
                        self.push_unmatched_delimiter_diagnostic(token.span(), open.name());
                    }
                } else {
                    self.push_unmatched_delimiter_diagnostic(token.span(), "无");
                }
            }
            _ => {}
        }
    }

    /// 记录关闭分隔符不匹配诊断。
    fn push_unmatched_delimiter_diagnostic(&mut self, span: SourceSpan, expected: &str) {
        self.diagnostics.push(Diagnostic::error_at(
            UNMATCHED_DELIMITER_CODE,
            "x01.lex.unmatched_delimiter",
            span,
            format!("关闭分隔符与当前打开的 {expected} 不匹配"),
        ));
    }

    /// 在 EOF 前补齐反缩进并报告仍未闭合的分隔符。
    fn finish_eof(&mut self) {
        if self.emitted_eof {
            return;
        }
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.pending
                .push_back(self.zero_width_token(TokenKind::Dedent, self.offset));
        }
        for delimiter in &self.delimiters {
            let span = match delimiter {
                OpenDelimiter::Parenthesis(span)
                | OpenDelimiter::Bracket(span)
                | OpenDelimiter::Brace(span) => *span,
            };
            self.diagnostics.push(Diagnostic::error_at(
                UNTERMINATED_DELIMITER_CODE,
                "x01.lex.unterminated_delimiter",
                span,
                format!("分隔符 {} 没有闭合", delimiter.name()),
            ));
        }
        self.emitted_eof = true;
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

    /// 扫描一个 ASCII 标识符，并将保留字映射为对应 Token。
    fn scan_identifier_or_keyword(&mut self) -> Token {
        let start = self.offset;
        while let Some(byte) = self.source.as_bytes().get(self.offset).copied() {
            if is_ascii_identifier_continue(byte) {
                self.offset += 1;
            } else {
                break;
            }
        }
        let word = &self.source.text()[start..self.offset];
        let kind = match word {
            "true" | "false" => TokenKind::Boolean,
            "none" => TokenKind::None,
            _ => KeywordKind::from_word(word)
                .map(TokenKind::Keyword)
                .unwrap_or(TokenKind::Identifier),
        };
        self.token(kind, start, self.offset)
    }

    /// 扫描一个十进制整数字面量或带小数/指数的浮点字面量。
    fn scan_integer(&mut self) -> Token {
        let start = self.offset;
        while matches!(self.source.as_bytes().get(self.offset), Some(b'0'..=b'9')) {
            self.offset += 1;
        }
        let mut is_float = false;
        if self.source.as_bytes().get(self.offset) == Some(&b'.') {
            is_float = true;
            self.offset += 1;
            while matches!(self.source.as_bytes().get(self.offset), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
        }
        let exponent = self.scan_exponent();
        if let Err((error_start, error_end)) = exponent {
            return self.invalid_number(start, error_start, error_end);
        }
        if exponent == Ok(true) {
            is_float = true;
        }
        self.token(
            if is_float {
                TokenKind::Float
            } else {
                TokenKind::Integer
            },
            start,
            self.offset,
        )
    }

    /// 扫描以点号开头的浮点字面量，例如 `.5` 或 `.5e2`。
    fn scan_fractional_number(&mut self) -> Token {
        let start = self.offset;
        self.offset += 1;
        while matches!(self.source.as_bytes().get(self.offset), Some(b'0'..=b'9')) {
            self.offset += 1;
        }
        if let Err((error_start, error_end)) = self.scan_exponent() {
            return self.invalid_number(start, error_start, error_end);
        }
        self.token(TokenKind::Float, start, self.offset)
    }

    /// 在当前偏移处尝试扫描十进制指数部分。
    fn scan_exponent(&mut self) -> Result<bool, (usize, usize)> {
        let bytes = self.source.as_bytes();
        let exponent_start = self.offset;
        if !matches!(bytes.get(self.offset), Some(b'e' | b'E')) {
            return Ok(false);
        }
        self.offset += 1;
        if matches!(bytes.get(self.offset), Some(b'+' | b'-')) {
            self.offset += 1;
        }
        let digits_start = self.offset;
        while matches!(bytes.get(self.offset), Some(b'0'..=b'9')) {
            self.offset += 1;
        }
        if self.offset == digits_start {
            Err((exponent_start, self.offset))
        } else {
            Ok(true)
        }
    }

    /// 创建数字结构错误 Token 并记录其指数片段诊断。
    fn invalid_number(
        &mut self,
        number_start: usize,
        error_start: usize,
        error_end: usize,
    ) -> Token {
        let token = self.token(TokenKind::Invalid, number_start, self.offset);
        let span = self
            .source
            .span(error_start, error_end)
            .expect("数字错误区间必须是有效源码边界");
        self.diagnostics.push(Diagnostic::error_at(
            INVALID_NUMBER_CODE,
            "x01.lex.invalid_number",
            span,
            "数字的指数部分缺少十进制数字".to_string(),
        ));
        token
    }

    /// 扫描一个字符串字面量并验证基本转义。
    fn scan_string(&mut self) -> Token {
        let start = self.offset;
        let quote = self.source.as_bytes()[self.offset];
        self.offset += 1;
        let mut invalid_escape = None;
        while self.offset < self.source.len_bytes() {
            let current = self.source.as_bytes()[self.offset];
            if current == quote {
                self.offset += 1;
                let token = self.token(TokenKind::String, start, self.offset);
                if let Some((escape_start, escape_end, escaped)) = invalid_escape {
                    self.diagnostics.push(Diagnostic::error_at(
                        INVALID_ESCAPE_CODE,
                        "x01.lex.invalid_escape",
                        self.source
                            .span(escape_start, escape_end)
                            .expect("escape span must be valid"),
                        format!("不支持的字符串转义：\\{escaped}"),
                    ));
                    return self.token(TokenKind::Invalid, start, self.offset);
                }
                return token;
            }
            if matches!(current, b'\n' | b'\r') {
                let token = self.token(TokenKind::Invalid, start, self.offset);
                self.diagnostics.push(Diagnostic::error_at(
                    UNTERMINATED_STRING_CODE,
                    "x01.lex.unterminated_string",
                    token.span(),
                    "字符串在换行前没有闭合".to_string(),
                ));
                return token;
            }
            if current == b'\\' {
                let escape_start = self.offset;
                self.offset += 1;
                if self.offset >= self.source.len_bytes() {
                    let token = self.token(TokenKind::Invalid, start, self.offset);
                    self.diagnostics.push(Diagnostic::error_at(
                        UNTERMINATED_STRING_CODE,
                        "x01.lex.unterminated_string",
                        token.span(),
                        "字符串以未完成的转义结尾".to_string(),
                    ));
                    return token;
                }
                let escaped = self.source.text()[self.offset..]
                    .chars()
                    .next()
                    .expect("转义目标必须位于有效字符边界");
                self.offset += escaped.len_utf8();
                if !is_valid_escape(escaped) && invalid_escape.is_none() {
                    invalid_escape = Some((escape_start, self.offset, escaped));
                }
                continue;
            }
            let character = self.source.text()[self.offset..]
                .chars()
                .next()
                .expect("字符串扫描偏移必须位于字符边界");
            self.offset += character.len_utf8();
        }
        let token = self.token(TokenKind::Invalid, start, self.offset);
        self.diagnostics.push(Diagnostic::error_at(
            UNTERMINATED_STRING_CODE,
            "x01.lex.unterminated_string",
            token.span(),
            "字符串没有闭合".to_string(),
        ));
        token
    }

    /// 扫描 `!`、`!?` 或 `!=`，优先采用最长匹配。
    fn scan_bang(&mut self) -> Token {
        let start = self.offset;
        let kind = match self.source.as_bytes().get(self.offset + 1) {
            Some(b'?') => {
                self.offset += 2;
                TokenKind::BangQuestion
            }
            Some(b'=') => {
                self.offset += 2;
                TokenKind::BangEqual
            }
            _ => {
                self.offset += 1;
                TokenKind::Bang
            }
        };
        self.token(kind, start, self.offset)
    }

    /// 扫描减号或减法复合赋值。
    fn scan_minus(&mut self) -> Token {
        let start = self.offset;
        let kind = if self.source.as_bytes().get(self.offset + 1) == Some(&b'=') {
            self.offset += 2;
            TokenKind::MinusEqual
        } else {
            self.offset += 1;
            TokenKind::Minus
        };
        self.token(kind, start, self.offset)
    }

    /// 扫描乘法、幂运算及其复合赋值。
    fn scan_star(&mut self) -> Token {
        let start = self.offset;
        let bytes = self.source.as_bytes();
        let kind = if bytes.get(self.offset + 1) == Some(&b'*') {
            if bytes.get(self.offset + 2) == Some(&b'=') {
                self.offset += 3;
                TokenKind::PowerEqual
            } else {
                self.offset += 2;
                TokenKind::Power
            }
        } else if bytes.get(self.offset + 1) == Some(&b'=') {
            self.offset += 2;
            TokenKind::StarEqual
        } else {
            self.offset += 1;
            TokenKind::Star
        };
        self.token(kind, start, self.offset)
    }

    /// 扫描除法、整除及其复合赋值。
    fn scan_slash(&mut self) -> Token {
        let start = self.offset;
        let bytes = self.source.as_bytes();
        let kind = if bytes.get(self.offset + 1) == Some(&b'/') {
            if bytes.get(self.offset + 2) == Some(&b'=') {
                self.offset += 3;
                TokenKind::FloorDivEqual
            } else {
                self.offset += 2;
                TokenKind::FloorDiv
            }
        } else if bytes.get(self.offset + 1) == Some(&b'=') {
            self.offset += 2;
            TokenKind::SlashEqual
        } else {
            self.offset += 1;
            TokenKind::Slash
        };
        self.token(kind, start, self.offset)
    }

    /// 扫描一个固定长度为一或二字节的 Token。
    fn scan_one_or_two(&mut self, _first: u8, second: u8, two: TokenKind, one: TokenKind) -> Token {
        let start = self.offset;
        if self.source.as_bytes().get(self.offset + 1) == Some(&second) {
            self.offset += 2;
            self.token(two, start, self.offset)
        } else {
            self.offset += 1;
            self.token(one, start, self.offset)
        }
    }

    /// 扫描一个单字节标点 Token。
    fn scan_single(&mut self, kind: TokenKind) -> Token {
        let start = self.offset;
        self.offset += 1;
        self.token(kind, start, self.offset)
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

/// 判断指定偏移是否位于合法的物理换行起点。
fn is_line_break_at(bytes: &[u8], offset: usize) -> bool {
    bytes.get(offset) == Some(&b'\n')
        || (bytes.get(offset) == Some(&b'\r') && bytes.get(offset + 1) == Some(&b'\n'))
}

/// 判断指定偏移是否开始一个文档注释标记。
fn starts_doc_comment(bytes: &[u8], offset: usize) -> bool {
    bytes
        .get(offset..offset.saturating_add(3))
        .is_some_and(|slice| slice == b"###")
}

/// 判断字符串转义字母是否属于 L1 支持的集合。
fn is_valid_escape(character: char) -> bool {
    matches!(
        character,
        '\\' | '\'' | '"' | 'n' | 'r' | 't' | '0' | 'b' | 'f' | 'v' | 'a'
    )
}

#[cfg(test)]
/// 覆盖 L0 Token 顺序、位置区间和错误恢复策略的单元测试。
mod tests {
    use crate::{
        INCONSISTENT_INDENT_CODE, INVALID_BACKTICK_ESCAPE_CODE, INVALID_CHARACTER_CODE,
        INVALID_ESCAPE_CODE, INVALID_NUMBER_CODE, KeywordKind, Lexer, TokenKind,
        UNMATCHED_DELIMITER_CODE, UNTERMINATED_BACKTICK_CODE, UNTERMINATED_DELIMITER_CODE,
        UNTERMINATED_DOC_COMMENT_CODE, UNTERMINATED_STRING_CODE,
    };
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
    /// 确认非行首 Tab 作为横向空白被统一跳过。
    fn expands_horizontal_tabs_as_ignored_whitespace() {
        let result = Lexer::new(&SourceFile::from_text("a\t=\t1")).tokenize();
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

    #[test]
    /// 确认字符串、浮点、布尔和空值字面量使用独立 Token。
    fn tokenizes_l1_literals() {
        let source = SourceFile::from_text("true false none 12.50 .5 1e3 \"星\" 'x'");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Boolean,
                TokenKind::Boolean,
                TokenKind::None,
                TokenKind::Float,
                TokenKind::Float,
                TokenKind::Float,
                TokenKind::String,
                TokenKind::String,
                TokenKind::Eof,
            ]
        );
        assert!(result.diagnostics.is_empty());
        assert_eq!(result.tokens[3].text(&source), "12.50");
        assert_eq!(result.tokens[6].text(&source), "\"星\"");
    }

    #[test]
    /// 确认保留字与普通名称区分，并保留正式类型关键字 `str`。
    fn classifies_keywords_without_rewriting_names() {
        let source = SourceFile::from_text("def start str custom");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(
            result.tokens[0].kind(),
            TokenKind::Keyword(KeywordKind::Def)
        );
        assert_eq!(
            result.tokens[2].kind(),
            TokenKind::Keyword(KeywordKind::Str)
        );
        assert_eq!(result.tokens[3].kind(), TokenKind::Identifier);
        assert_eq!(result.tokens[0].kind().keyword(), Some(KeywordKind::Def));
    }

    #[test]
    /// 确认复合运算符和选择器标记采用最长匹配。
    fn applies_longest_operator_matching() {
        let source = SourceFile::from_text(
            "== != <= >= += -= *= /= // //= %= ** **= -> !? ! ~ ? < > + - * / % |",
        );
        let result = Lexer::new(&source).tokenize();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::EqualEqual,
                TokenKind::BangEqual,
                TokenKind::LessEqual,
                TokenKind::GreaterEqual,
                TokenKind::PlusEqual,
                TokenKind::MinusEqual,
                TokenKind::StarEqual,
                TokenKind::SlashEqual,
                TokenKind::FloorDiv,
                TokenKind::FloorDivEqual,
                TokenKind::PercentEqual,
                TokenKind::Power,
                TokenKind::PowerEqual,
                TokenKind::Minus,
                TokenKind::Greater,
                TokenKind::BangQuestion,
                TokenKind::Bang,
                TokenKind::Tilde,
                TokenKind::Question,
                TokenKind::Less,
                TokenKind::Greater,
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Pipe,
                TokenKind::Eof,
            ]
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    /// 确认括号、容器标记和表头所需标点都保留为独立 Token。
    fn tokenizes_delimiters_and_path_punctuation() {
        let source = SourceFile::from_text("() [] {} , : . @ $ / 3/2");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::LeftBracket,
                TokenKind::RightBracket,
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Dot,
                TokenKind::At,
                TokenKind::Dollar,
                TokenKind::Slash,
                TokenKind::Integer,
                TokenKind::Slash,
                TokenKind::Integer,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    /// 确认未闭合字符串产生 Invalid Token 和稳定诊断，并把换行留给后续扫描。
    fn diagnoses_unterminated_string() {
        let source = SourceFile::from_text("\"hello\nnext");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(result.tokens[0].kind(), TokenKind::Invalid);
        assert_eq!(result.tokens[1].kind(), TokenKind::Newline);
        assert_eq!(result.diagnostics[0].code(), UNTERMINATED_STRING_CODE);
    }

    #[test]
    /// 确认不支持的字符串转义不会被静默接受。
    fn diagnoses_invalid_string_escape() {
        let source = SourceFile::from_text("\"bad\\q\"");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(result.tokens[0].kind(), TokenKind::Invalid);
        assert_eq!(result.diagnostics[0].code(), INVALID_ESCAPE_CODE);
    }

    #[test]
    /// 确认非法转义后的多字节字符不会把游标留在 UTF-8 中间。
    fn recovers_from_unicode_escape_target() {
        let source = SourceFile::from_text("\"bad\\星\" ok");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(result.tokens[0].kind(), TokenKind::Invalid);
        assert_eq!(result.tokens[1].kind(), TokenKind::Identifier);
        assert_eq!(result.tokens[1].text(&source), "ok");
        assert_eq!(result.diagnostics[0].code(), INVALID_ESCAPE_CODE);
    }

    #[test]
    /// 确认不完整指数被标记为数字错误，并能继续扫描后续名称。
    fn diagnoses_incomplete_number_exponent() {
        let source = SourceFile::from_text("1e+ next");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(result.tokens[0].kind(), TokenKind::Invalid);
        assert_eq!(result.tokens[0].text(&source), "1e+");
        assert_eq!(result.tokens[1].kind(), TokenKind::Identifier);
        assert_eq!(result.tokens[1].text(&source), "next");
        assert_eq!(result.diagnostics[0].code(), INVALID_NUMBER_CODE);
    }

    #[test]
    /// 确认反引号名称支持 UTF-8、空格和反斜杠转义。
    fn tokenizes_backtick_identifiers() {
        let source = SourceFile::from_text("`新 变量` `def` `a\\`b` `a\\\\b`");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::BacktickIdentifier,
                TokenKind::BacktickIdentifier,
                TokenKind::BacktickIdentifier,
                TokenKind::BacktickIdentifier,
                TokenKind::Eof,
            ]
        );
        assert_eq!(result.tokens[0].text(&source), "`新 变量`");
        assert_eq!(result.tokens[2].text(&source), "`a\\`b`");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    /// 确认反引号非法转义和未闭合名称具有独立诊断。
    fn diagnoses_backtick_errors() {
        let invalid = Lexer::new(&SourceFile::from_text("`bad\\q` ok")).tokenize();
        assert_eq!(invalid.tokens[0].kind(), TokenKind::Invalid);
        assert_eq!(invalid.tokens[1].kind(), TokenKind::Identifier);
        assert_eq!(invalid.diagnostics[0].code(), INVALID_BACKTICK_ESCAPE_CODE);

        let unterminated = Lexer::new(&SourceFile::from_text("`bad\nnext")).tokenize();
        assert_eq!(unterminated.tokens[0].kind(), TokenKind::Invalid);
        assert_eq!(unterminated.tokens[1].kind(), TokenKind::Newline);
        assert_eq!(
            unterminated.diagnostics[0].code(),
            UNTERMINATED_BACKTICK_CODE
        );
    }

    #[test]
    /// 确认普通注释被跳过、文档注释保留且物理换行不丢失。
    fn handles_line_and_doc_comments() {
        let source = SourceFile::from_text("a # ordinary\n### doc ###\nb\n");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::DocComment,
                TokenKind::Newline,
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
        assert_eq!(result.tokens[2].text(&source), "### doc ###");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    /// 确认文档注释可以跨行并作为一个完整 Token 保存。
    fn tokenizes_multiline_doc_comment() {
        let source = SourceFile::from_text("### first\nsecond ###\na");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(result.tokens[0].kind(), TokenKind::DocComment);
        assert_eq!(result.tokens[0].text(&source), "### first\nsecond ###");
        assert_eq!(result.tokens[1].kind(), TokenKind::Newline);
        assert_eq!(result.tokens[2].kind(), TokenKind::Identifier);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    /// 确认未闭合文档注释不会吞掉 EOF 之后不存在的 Token。
    fn diagnoses_unterminated_doc_comment() {
        let result = Lexer::new(&SourceFile::from_text("### missing")).tokenize();
        assert_eq!(result.tokens[0].kind(), TokenKind::Invalid);
        assert_eq!(result.tokens[1].kind(), TokenKind::Eof);
        assert_eq!(result.diagnostics[0].code(), UNTERMINATED_DOC_COMMENT_CODE);
    }

    #[test]
    /// 确认多层缩进、EOF 反缩进和四空格层级顺序稳定。
    fn emits_indent_and_dedent_tokens() {
        let source = SourceFile::from_text("a\n    b\n        c\n    d\ne\n");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Dedent,
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Dedent,
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
        assert_eq!(result.tokens[2].text(&source), "    ");
        assert!(result.tokens[8].span().is_empty());
        assert!(result.tokens[11].span().is_empty());
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    /// 确认空行、普通注释行和 Tab 不会虚增缩进层级。
    fn ignores_non_code_lines_for_indentation() {
        let source = SourceFile::from_text("a\n\n# note\n\tb\n    c");
        let result = Lexer::new(&source).tokenize();
        let kinds = result
            .tokens
            .iter()
            .map(|token| token.kind())
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Newline,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Identifier,
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    /// 确认括号内部保留物理换行但不生成缩进 Token。
    fn does_not_indent_inside_delimiters() {
        let source = SourceFile::from_text("a(\n        b\n    )\nc");
        let result = Lexer::new(&source).tokenize();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Identifier,
                TokenKind::LeftParen,
                TokenKind::Newline,
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::RightParen,
                TokenKind::Newline,
                TokenKind::Identifier,
                TokenKind::Eof,
            ]
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    /// 确认非四空格缩进报告错误并归入最近的已知层级。
    fn recovers_from_inconsistent_indentation() {
        let source = SourceFile::from_text("a\n  b\n    c\nd");
        let result = Lexer::new(&source).tokenize();
        assert!(
            result
                .tokens
                .iter()
                .any(|token| token.kind() == TokenKind::Indent)
        );
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code())
                .collect::<Vec<_>>(),
            vec![INCONSISTENT_INDENT_CODE]
        );
        assert_eq!(
            result.tokens.last().expect("应有 EOF").kind(),
            TokenKind::Eof
        );
    }

    #[test]
    /// 确认非法的过深缩进只回退到已有层级，不创建隐含缩进层。
    fn does_not_create_implicit_indent_for_invalid_width() {
        let source = SourceFile::from_text("a\n    b\n      c\n    d");
        let result = Lexer::new(&source).tokenize();
        let kinds = result
            .tokens
            .iter()
            .map(|token| token.kind())
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Identifier,
                TokenKind::Newline,
                TokenKind::Identifier,
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code())
                .collect::<Vec<_>>(),
            vec![INCONSISTENT_INDENT_CODE]
        );
    }

    #[test]
    /// 确认未闭合和不匹配的分隔符不会导致词法器停滞。
    fn diagnoses_delimiter_errors() {
        let unmatched = Lexer::new(&SourceFile::from_text(")")).tokenize();
        assert_eq!(unmatched.diagnostics[0].code(), UNMATCHED_DELIMITER_CODE);

        let unterminated = Lexer::new(&SourceFile::from_text("(a")).tokenize();
        assert_eq!(
            unterminated.tokens.last().expect("应有 EOF").kind(),
            TokenKind::Eof
        );
        assert_eq!(
            unterminated.diagnostics[0].code(),
            UNTERMINATED_DELIMITER_CODE
        );
    }
}
