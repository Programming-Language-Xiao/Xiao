//! `config.xiao` 的不可执行 Token 解析器。
//!
//! 解析器直接消费 `xiao-syntax::Lexer` 的 Token 流，只接受顶层表头、键值赋值
//! 和字面量容器。普通 Xiao 解析器没有被调用，因此函数、控制流、导入和表达式
//! 不可能在配置读取阶段被执行。

use std::collections::{BTreeMap, btree_map::Entry};

use xiao_diagnostics::{Diagnostic, DiagnosticParam};
use xiao_source::{SourceFile, SourceSpan};
use xiao_syntax::{Lexer, Token, TokenKind};

use crate::diagnostics::*;
use crate::model::{ConfigDocument, ConfigEntry, ConfigTable, ConfigValue};
use crate::validation::{validate_config, validate_project_config};

/// 一次配置解析的结果；即使有错误，也尽量返回可恢复的部分文档。
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigParseResult {
    /// 已解析出的配置文档；词法/结构错误不会抹掉已经读取的表。
    pub document: Option<ConfigDocument>,
    /// 按源码顺序收集的词法、结构和模式诊断。
    pub diagnostics: ConfigDiagnostics,
}

impl ConfigParseResult {
    /// 判断结果是否包含错误级别诊断。
    #[must_use]
    pub fn has_errors(&self) -> bool {
        has_errors(&self.diagnostics)
    }

    /// 判断结果是否有可用文档且没有错误。
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.document.is_some() && !self.has_errors()
    }

    /// 借用部分或完整配置文档。
    #[must_use]
    pub const fn document(&self) -> Option<&ConfigDocument> {
        self.document.as_ref()
    }

    /// 借用完整诊断列表。
    #[must_use]
    pub fn diagnostics(&self) -> &[ConfigDiagnostic] {
        &self.diagnostics
    }
}

/// 解析并校验一份项目配置。
///
/// 该入口允许全局配置使用的扩展表，但若存在 `[project]`，其 `name` 和
/// `version` 仍必须同时出现。需要强制项目身份时使用 [`parse_config_project`]。
pub fn parse_config(source: &SourceFile) -> Result<ConfigDocument, ConfigDiagnostics> {
    let result = parse_and_validate(source, false);
    if result.is_success() {
        Ok(result.document.expect("成功结果必须包含配置文档"))
    } else {
        Err(result.diagnostics)
    }
}

/// 解析并校验要求项目身份的 `config.xiao`。
pub fn parse_config_project(source: &SourceFile) -> Result<ConfigDocument, ConfigDiagnostics> {
    let result = parse_and_validate(source, true);
    if result.is_success() {
        Ok(result.document.expect("成功结果必须包含配置文档"))
    } else {
        Err(result.diagnostics)
    }
}

/// 返回包含部分文档和完整诊断的解析结果。
pub fn parse_config_document(source: &SourceFile) -> ConfigParseResult {
    parse_and_validate(source, false)
}

/// 按照 Xiao 前端的常见门面命名解析配置并返回可恢复结果。
pub fn parse(source: &SourceFile) -> ConfigParseResult {
    parse_config_document(source)
}

/// 从 UTF-8 文本解析配置，作为 [`parse_config`] 的便捷入口。
pub fn parse_config_text(text: &str) -> Result<ConfigDocument, ConfigDiagnostics> {
    parse_config(&SourceFile::from_text(text))
}

/// 解析、再按项目或通用配置模式校验。
fn parse_and_validate(source: &SourceFile, require_project: bool) -> ConfigParseResult {
    let raw = RawConfigParser::new(source).parse();
    let mut diagnostics = raw.diagnostics;
    let mut document = raw.document;

    if !has_errors(&diagnostics) {
        if let Some(current) = document.as_ref() {
            let validation = if require_project {
                validate_project_config(current)
            } else {
                validate_config(current)
            };
            match validation {
                Ok(normalized) => document = Some(normalized),
                Err(mut errors) => diagnostics.append(&mut errors),
            }
        }
    }

    ConfigParseResult {
        document,
        diagnostics,
    }
}

/// 一个只负责 Token 到配置节点转换的内部解析器。
struct RawConfigParser<'source> {
    /// 原始源码及位置索引。
    source: &'source SourceFile,
    /// 词法器生成的完整 Token 流。
    tokens: Vec<Token>,
    /// 已收集的词法和配置结构诊断。
    diagnostics: ConfigDiagnostics,
    /// 当前 Token 下标。
    cursor: usize,
}

/// 一个带源码区间的已解析值。
struct ParsedValue {
    /// 配置值。
    value: ConfigValue,
    /// 覆盖该值的源码区间。
    span: SourceSpan,
}

impl<'source> RawConfigParser<'source> {
    /// 使用共享词法器创建配置解析器。
    fn new(source: &'source SourceFile) -> Self {
        let lexical = Lexer::new(source).tokenize();
        Self {
            source,
            tokens: lexical.tokens,
            diagnostics: lexical.diagnostics,
            cursor: 0,
        }
    }

    /// 解析全部顶层表。
    fn parse(mut self) -> RawParseResult {
        let mut tables = BTreeMap::new();
        while !self.at(TokenKind::Eof) {
            self.skip_layout();
            if self.at(TokenKind::Eof) {
                break;
            }
            if !self.at(TokenKind::LeftBracket) {
                let token = self.current();
                self.push_error(
                    CONFIG_BOUNDARY_CODE,
                    "x05.config.top_level_requires_table",
                    token.span(),
                    "配置文件顶层只能出现表头，不能出现可执行语句".to_owned(),
                );
                self.synchronize_line();
                continue;
            }
            if !self.is_top_level_token(self.current()) {
                let token = self.bump();
                self.push_error(
                    CONFIG_BOUNDARY_CODE,
                    "x05.config.indented_table_header",
                    token.span(),
                    "配置表头必须位于源码行首".to_owned(),
                );
                self.synchronize_line();
                continue;
            }

            let Some((name, header_span, header_end)) = self.parse_table_header() else {
                self.synchronize_line();
                continue;
            };
            let (entries, table_end) = self.parse_table_body(header_end);
            let table_span = self.source_span(header_span.start(), table_end);
            if tables.contains_key(&name) {
                self.push_error(
                    DUPLICATE_TABLE_CODE,
                    "x05.config.duplicate_table",
                    header_span,
                    format!("配置表 {name:?} 不能重复声明"),
                );
            } else {
                tables.insert(name.clone(), ConfigTable::new(name, entries, table_span));
            }
        }

        let span = self
            .source
            .span(0, self.source.len_bytes())
            .expect("源码整体区间必须有效");
        RawParseResult {
            document: Some(ConfigDocument::new(tables, span)),
            diagnostics: self.diagnostics,
        }
    }

    /// 解析单层 `[name]` 表头；双层表头和复杂表达式均被拒绝。
    fn parse_table_header(&mut self) -> Option<(String, SourceSpan, usize)> {
        let open = self.bump();
        if self.at(TokenKind::LeftBracket) {
            let second = self.bump();
            self.push_error(
                CONFIG_INVALID_TABLE_HEADER_CODE,
                "x05.config.instance_table_forbidden",
                self.source_span(open.span().start(), second.span().end()),
                "config.xiao 不允许可实例化的双层表头".to_owned(),
            );
            return None;
        }

        let name_token = self.current();
        let Some(name) = self.parse_name_token() else {
            self.push_error(
                CONFIG_INVALID_TABLE_HEADER_CODE,
                "x05.config.invalid_table_name",
                name_token.span(),
                "表头必须使用 ASCII 名称或反引号名称".to_owned(),
            );
            return None;
        };
        if self.at(TokenKind::Dot) {
            let dot = self.bump();
            self.push_error(
                CONFIG_INVALID_TABLE_HEADER_CODE,
                "x05.config.dotted_table_forbidden",
                dot.span(),
                "首版 config.xiao 不支持点号嵌套表头".to_owned(),
            );
            while !matches!(
                self.current().kind(),
                TokenKind::RightBracket | TokenKind::Newline | TokenKind::Eof
            ) {
                self.bump();
            }
        }

        let close = if self.at(TokenKind::RightBracket) {
            self.bump()
        } else {
            let token = self.current();
            self.push_error(
                CONFIG_INVALID_TABLE_HEADER_CODE,
                "x05.config.missing_table_close",
                token.span(),
                "表头缺少右方括号".to_owned(),
            );
            return None;
        };
        let span = self.source_span(open.span().start(), close.span().end());
        Some((name.to_ascii_lowercase(), span, close.span().end()))
    }

    /// 解析表头之后直到下一个顶层表头的键值成员。
    fn parse_table_body(&mut self, header_end: usize) -> (BTreeMap<String, ConfigEntry>, usize) {
        let mut entries = BTreeMap::new();
        let mut end = header_end;
        self.skip_layout();

        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::LeftBracket) {
                if self.is_top_level_token(self.current()) {
                    break;
                }
                let nested = self.bump();
                self.push_error(
                    UNSUPPORTED_CONSTRUCT_CODE,
                    "x05.config.nested_table_forbidden",
                    nested.span(),
                    "配置表不能在其他表中嵌套声明".to_owned(),
                );
                self.synchronize_line();
                continue;
            }
            if matches!(
                self.current().kind(),
                TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent | TokenKind::DocComment
            ) {
                self.bump();
                continue;
            }

            let key_token = self.current();
            let Some(key) = self.parse_name_or_string_key() else {
                self.push_error(
                    UNSUPPORTED_CONSTRUCT_CODE,
                    "x05.config.executable_member_forbidden",
                    key_token.span(),
                    "配置表成员必须是静态键值声明".to_owned(),
                );
                self.synchronize_line();
                continue;
            };
            if !self.at(TokenKind::Equal) {
                let token = self.current();
                self.push_error(
                    MISSING_SEPARATOR_CODE,
                    "x05.config.missing_equals",
                    token.span(),
                    "配置键和值必须使用 = 连接".to_owned(),
                );
                self.synchronize_line();
                continue;
            }
            self.bump();
            let Some(parsed) = self.parse_value() else {
                self.synchronize_line();
                continue;
            };
            end = parsed.span.end();
            if !self.is_entry_boundary(self.current()) {
                let token = self.current();
                self.push_error(
                    UNSUPPORTED_CONSTRUCT_CODE,
                    "x05.config.trailing_expression_forbidden",
                    token.span(),
                    "配置值后不能出现运算、调用或其他表达式".to_owned(),
                );
                self.synchronize_line();
            }
            let span = self.source_span(key_token.span().start(), parsed.span.end());
            let entry = ConfigEntry::new(key.clone(), parsed.value, span);
            match entries.entry(key) {
                Entry::Vacant(slot) => {
                    slot.insert(entry);
                }
                Entry::Occupied(slot) => {
                    self.push_error(
                        DUPLICATE_KEY_CODE,
                        "x05.config.duplicate_key",
                        key_token.span(),
                        format!("配置键 {:?} 不能重复声明", slot.key()),
                    );
                }
            }
        }
        (entries, end)
    }

    /// 解析允许的递归静态值。
    fn parse_value(&mut self) -> Option<ParsedValue> {
        self.skip_value_layout();
        let token = self.current();
        match token.kind() {
            TokenKind::String => {
                self.bump();
                let value = self.decode_quoted(token)?;
                Some(ParsedValue {
                    value: ConfigValue::String(value),
                    span: token.span(),
                })
            }
            TokenKind::Integer | TokenKind::Float => self.parse_number(None),
            TokenKind::Plus | TokenKind::Minus => self.parse_signed_number(),
            TokenKind::Boolean => {
                self.bump();
                Some(ParsedValue {
                    value: ConfigValue::Boolean(token.text(self.source) == "true"),
                    span: token.span(),
                })
            }
            TokenKind::LeftBracket => self.parse_array(),
            TokenKind::LeftBrace => self.parse_dictionary(),
            _ => {
                self.bump();
                self.push_error(
                    UNSUPPORTED_CONSTRUCT_CODE,
                    "x05.config.non_literal_value",
                    token.span(),
                    "配置值只能是字符串、数值、布尔值、数组或字典表字面量".to_owned(),
                );
                None
            }
        }
    }

    /// 解析带可选正负号的数字。
    fn parse_signed_number(&mut self) -> Option<ParsedValue> {
        let sign = self.bump();
        if !matches!(self.current().kind(), TokenKind::Integer | TokenKind::Float) {
            self.push_error(
                UNSUPPORTED_CONSTRUCT_CODE,
                "x05.config.sign_requires_number",
                sign.span(),
                "正负号在配置中只能修饰数值字面量".to_owned(),
            );
            return None;
        }
        self.parse_number(Some(sign))
    }

    /// 将整数或浮点 Token 解码为配置数值。
    fn parse_number(&mut self, sign: Option<Token>) -> Option<ParsedValue> {
        let token = self.bump();
        let mut text = token.text(self.source).to_owned();
        if let Some(sign) = sign {
            text.insert(
                0,
                sign.text(self.source)
                    .chars()
                    .next()
                    .expect("符号 Token 非空"),
            );
        }
        let span = sign.map_or(token.span(), |prefix| {
            self.source_span(prefix.span().start(), token.span().end())
        });
        match token.kind() {
            TokenKind::Integer => match text.parse::<i128>() {
                Ok(value) => Some(ParsedValue {
                    value: ConfigValue::Integer(value),
                    span,
                }),
                Err(_) => {
                    self.push_error(
                        CONFIG_INVALID_VALUE_CODE,
                        "x05.config.integer_out_of_range",
                        span,
                        "整数超出配置数值范围".to_owned(),
                    );
                    None
                }
            },
            TokenKind::Float => match text.parse::<f64>() {
                Ok(value) if value.is_finite() => Some(ParsedValue {
                    value: ConfigValue::Float(value),
                    span,
                }),
                _ => {
                    self.push_error(
                        CONFIG_INVALID_VALUE_CODE,
                        "x05.config.invalid_float",
                        span,
                        "浮点值必须是有限数值".to_owned(),
                    );
                    None
                }
            },
            _ => unreachable!("parse_number 只接收数字 Token"),
        }
    }

    /// 解析数组及其递归元素。
    fn parse_array(&mut self) -> Option<ParsedValue> {
        let open = self.bump();
        let mut values = Vec::new();
        self.skip_value_layout();
        if self.at(TokenKind::RightBracket) {
            let close = self.bump();
            return Some(ParsedValue {
                value: ConfigValue::Array(values),
                span: self.source_span(open.span().start(), close.span().end()),
            });
        }
        loop {
            self.skip_value_layout();
            if self.at(TokenKind::RightBracket) {
                let close = self.bump();
                return Some(ParsedValue {
                    value: ConfigValue::Array(values),
                    span: self.source_span(open.span().start(), close.span().end()),
                });
            }
            let Some(value) = self.parse_value() else {
                self.recover_container(TokenKind::RightBracket);
                return None;
            };
            values.push(value.value);
            self.skip_value_layout();
            if self.at(TokenKind::Comma) {
                self.bump();
                self.skip_value_layout();
                continue;
            }
            if self.at(TokenKind::RightBracket) {
                let close = self.bump();
                return Some(ParsedValue {
                    value: ConfigValue::Array(values),
                    span: self.source_span(open.span().start(), close.span().end()),
                });
            }
            let token = self.current();
            self.push_error(
                MISSING_SEPARATOR_CODE,
                "x05.config.missing_array_comma",
                token.span(),
                "数组元素之间必须使用逗号分隔".to_owned(),
            );
            self.recover_container(TokenKind::RightBracket);
            return None;
        }
    }

    /// 解析花括号字典表；字典键和值必须使用 `=`。
    fn parse_dictionary(&mut self) -> Option<ParsedValue> {
        let open = self.bump();
        let mut entries = BTreeMap::new();
        self.skip_value_layout();
        if self.at(TokenKind::RightBrace) {
            let close = self.bump();
            return Some(ParsedValue {
                value: ConfigValue::Dictionary(entries),
                span: self.source_span(open.span().start(), close.span().end()),
            });
        }
        loop {
            self.skip_value_layout();
            if self.at(TokenKind::RightBrace) {
                let close = self.bump();
                return Some(ParsedValue {
                    value: ConfigValue::Dictionary(entries),
                    span: self.source_span(open.span().start(), close.span().end()),
                });
            }
            let key_token = self.current();
            let Some(key) = self.parse_name_or_string_key() else {
                self.push_error(
                    CONFIG_INVALID_VALUE_CODE,
                    "x05.config.invalid_dictionary_key",
                    key_token.span(),
                    "字典表键必须是名称或字符串".to_owned(),
                );
                self.recover_container(TokenKind::RightBrace);
                return None;
            };
            if !self.at(TokenKind::Equal) {
                let token = self.current();
                self.push_error(
                    MISSING_SEPARATOR_CODE,
                    "x05.config.dictionary_requires_equals",
                    token.span(),
                    "字典表键和值必须使用 = 连接".to_owned(),
                );
                self.recover_container(TokenKind::RightBrace);
                return None;
            }
            self.bump();
            let Some(value) = self.parse_value() else {
                self.recover_container(TokenKind::RightBrace);
                return None;
            };
            match entries.entry(key) {
                Entry::Vacant(slot) => {
                    slot.insert(value.value);
                }
                Entry::Occupied(slot) => {
                    self.push_error(
                        DUPLICATE_KEY_CODE,
                        "x05.config.duplicate_dictionary_key",
                        key_token.span(),
                        format!("字典表键 {:?} 不能重复声明", slot.key()),
                    );
                }
            }
            self.skip_value_layout();
            if self.at(TokenKind::Comma) {
                self.bump();
                continue;
            }
            if self.at(TokenKind::RightBrace) {
                let close = self.bump();
                return Some(ParsedValue {
                    value: ConfigValue::Dictionary(entries),
                    span: self.source_span(open.span().start(), close.span().end()),
                });
            }
            let token = self.current();
            self.push_error(
                MISSING_SEPARATOR_CODE,
                "x05.config.missing_dictionary_comma",
                token.span(),
                "字典表条目之间必须使用逗号分隔".to_owned(),
            );
            self.recover_container(TokenKind::RightBrace);
            return None;
        }
    }

    /// 解析名称键或字符串键并返回解码后的文本。
    fn parse_name_or_string_key(&mut self) -> Option<String> {
        let token = self.current();
        match token.kind() {
            TokenKind::Identifier | TokenKind::BacktickIdentifier => {
                self.bump();
                self.decode_name(token)
            }
            TokenKind::String => {
                self.bump();
                self.decode_quoted(token)
            }
            _ => None,
        }
    }

    /// 解析只允许名称的表头。
    fn parse_name_token(&mut self) -> Option<String> {
        let token = self.current();
        if !matches!(
            token.kind(),
            TokenKind::Identifier | TokenKind::BacktickIdentifier
        ) {
            return None;
        }
        self.bump();
        self.decode_name(token)
    }

    /// 解码名称 Token 的源码文本。
    fn decode_name(&mut self, token: Token) -> Option<String> {
        let text = token.text(self.source);
        if token.kind() == TokenKind::Identifier {
            return Some(text.to_owned());
        }
        decode_backtick(text).ok().or_else(|| {
            self.push_error(
                CONFIG_INVALID_VALUE_CODE,
                "x05.config.invalid_backtick_name",
                token.span(),
                "反引号名称包含非法转义".to_owned(),
            );
            None
        })
    }

    /// 解码带引号字符串；词法器已经先行验证转义结构。
    fn decode_quoted(&mut self, token: Token) -> Option<String> {
        decode_quoted_text(token.text(self.source))
            .ok()
            .or_else(|| {
                self.push_error(
                    CONFIG_INVALID_VALUE_CODE,
                    "x05.config.invalid_string",
                    token.span(),
                    "字符串字面量无法解码".to_owned(),
                );
                None
            })
    }

    /// 跳过顶层布局 Token。
    fn skip_layout(&mut self) {
        while matches!(
            self.current().kind(),
            TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent | TokenKind::DocComment
        ) {
            self.bump();
        }
    }

    /// 跳过容器内部的软换行和缩进。
    fn skip_value_layout(&mut self) {
        self.skip_layout();
    }

    /// 判断当前 Token 是否可作为键值成员后的边界。
    fn is_entry_boundary(&self, token: Token) -> bool {
        matches!(
            token.kind(),
            TokenKind::Newline
                | TokenKind::Indent
                | TokenKind::Dedent
                | TokenKind::Eof
                | TokenKind::LeftBracket
        )
    }

    /// 判断 Token 是否位于源码行首，可作为下一个顶层表头。
    fn is_top_level_token(&self, token: Token) -> bool {
        token
            .start_position(self.source)
            .map(|position| position.column == 1)
            .unwrap_or(false)
    }

    /// 从容器错误中恢复到逗号、闭分隔符或 EOF。
    fn recover_container(&mut self, close: TokenKind) {
        while !matches!(self.current().kind(), TokenKind::Comma | TokenKind::Eof) && !self.at(close)
        {
            self.bump();
        }
        if self.at(TokenKind::Comma) {
            self.bump();
        }
    }

    /// 消费当前物理行，保留下一行表头给外层循环。
    fn synchronize_line(&mut self) {
        while !matches!(self.current().kind(), TokenKind::Newline | TokenKind::Eof) {
            self.bump();
        }
        if self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    /// 返回当前 Token。
    fn current(&self) -> Token {
        self.tokens
            .get(self.cursor)
            .copied()
            .or_else(|| self.tokens.last().copied())
            .expect("词法结果至少包含 EOF")
    }

    /// 消费当前 Token。
    fn bump(&mut self) -> Token {
        let token = self.current();
        if !token.kind().is_eof() && self.cursor < self.tokens.len() {
            self.cursor += 1;
        }
        token
    }

    /// 判断当前 Token 类型。
    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind() == kind
    }

    /// 创建源码区间。
    fn source_span(&self, start: usize, end: usize) -> SourceSpan {
        self.source
            .span(start, end)
            .expect("解析器只能组合合法源码边界")
    }

    /// 添加一条带结构化名称参数的配置诊断。
    fn push_error(&mut self, code: &str, message_id: &str, span: SourceSpan, message: String) {
        let diagnostic = Diagnostic::error_at(code, message_id, span, message).with_params([(
            "stage".to_owned(),
            DiagnosticParam::Text("config".to_owned()),
        )]);
        self.diagnostics.push(diagnostic);
    }
}

/// 原始解析器内部结果。
struct RawParseResult {
    /// 部分配置文档。
    document: Option<ConfigDocument>,
    /// 词法和结构诊断。
    diagnostics: ConfigDiagnostics,
}

/// 解码带单引号或双引号的 Xiao 字符串。
fn decode_quoted_text(text: &str) -> Result<String, ()> {
    let mut chars = text.chars();
    let quote = chars.next().ok_or(())?;
    if !matches!(quote, '\'' | '"') || chars.next_back() != Some(quote) {
        return Err(());
    }
    let inner = &text[quote.len_utf8()..text.len() - quote.len_utf8()];
    let mut output = String::with_capacity(inner.len());
    let mut escaped = false;
    for character in inner.chars() {
        if escaped {
            let decoded = match character {
                '\\' => '\\',
                '\'' => '\'',
                '"' => '"',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                'b' => '\u{0008}',
                'f' => '\u{000c}',
                'v' => '\u{000b}',
                'a' => '\u{0007}',
                _ => return Err(()),
            };
            output.push(decoded);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }
    if escaped {
        return Err(());
    }
    Ok(output)
}

/// 解码反引号名称，只允许反引号和反斜杠转义。
fn decode_backtick(text: &str) -> Result<String, ()> {
    if !text.starts_with('`') || !text.ends_with('`') || text.len() < 2 {
        return Err(());
    }
    let inner = &text[1..text.len() - 1];
    let mut output = String::with_capacity(inner.len());
    let mut escaped = false;
    for character in inner.chars() {
        if escaped {
            if !matches!(character, '`' | '\\') {
                return Err(());
            }
            output.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }
    if escaped {
        return Err(());
    }
    Ok(output)
}

#[cfg(test)]
/// 覆盖配置词法边界、容器和不可执行构造的内部测试。
mod tests {
    use super::{parse_config, parse_config_project};
    use crate::model::ConfigValue;
    use xiao_source::SourceFile;

    #[test]
    /// 解析项目、导出和递归静态值。
    fn parses_static_project_config() {
        let source = SourceFile::from_text(
            "[project]\nname = \"demo\"\nversion = \"0.1\"\n[exports]\napi = \"src/api.xiao\"\n[CLI]\ngit = { summary = true }\n",
        );
        let document = parse_config_project(&source).expect("配置应合法");
        assert_eq!(
            document
                .table("project")
                .expect("project")
                .get("name")
                .and_then(|entry| entry.value.as_str()),
            Some("demo")
        );
        assert!(matches!(
            document
                .table("cli")
                .expect("cli")
                .get("git")
                .map(|entry| &entry.value),
            Some(ConfigValue::Dictionary(_))
        ));
    }

    #[test]
    /// 函数调用和表达式必须在读取阶段拒绝。
    fn rejects_executable_values() {
        let source = SourceFile::from_text("[project]\nname = input(\"x\")\nversion = \"1\"\n");
        let errors = parse_config(&source).expect_err("调用不能出现在配置中");
        assert!(errors.iter().any(|error| error.code() == "X05-CONFIG-002"));
    }

    #[test]
    /// 字典使用冒号而非等号时报告稳定分隔符诊断。
    fn rejects_colon_dictionary_separator() {
        let source = SourceFile::from_text(
            "[project]\nname = \"x\"\nversion = \"1\"\n[extra]\nvalue = { key: 1 }\n",
        );
        let errors = parse_config(&source).expect_err("冒号不是配置字典分隔符");
        assert!(errors.iter().any(|error| error.code() == "X05-CONFIG-011"));
    }
}
