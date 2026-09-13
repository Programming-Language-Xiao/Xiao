//! Xiao 词法 Token 的基础实现。
//!
//! 本模块负责把经过 UTF-8 校验的源码转换为带 [`SourceSpan`] 的 Token
//! 流。当前覆盖 01 阶段的 L0 与 L1：源码位置、基本字面量、保留字、
//! 括号和基础运算符。解析器、AST、缩进层和类型检查仍属于后续里程碑；
//! 词法器不会执行 Xiao 程序或猜测表达式语义。

use xiao_diagnostics::Diagnostic;
use xiao_source::{SourceError, SourceFile, SourcePosition, SourceSpan};

/// 非法 UTF-8 源文件的稳定诊断编号（从源码层重新导出）。
pub use xiao_source::INVALID_UTF8_CODE;

/// L0 非法字符的稳定诊断编号。
pub const INVALID_CHARACTER_CODE: &str = "X01-LEX-001";

/// 字符串字面量没有闭合时使用的稳定诊断编号。
pub const UNTERMINATED_STRING_CODE: &str = "X01-LEX-002";

/// 字符串中出现不支持的转义序列时使用的稳定诊断编号。
pub const INVALID_ESCAPE_CODE: &str = "X01-LEX-003";

/// 数字字面量结构不完整时使用的稳定诊断编号。
pub const INVALID_NUMBER_CODE: &str = "X01-LEX-004";

/// Xiao 语言中的保留字类别。
///
/// 保留字仍然携带原始源码区间；解析器可以通过 [`TokenKind::Keyword`]
/// 区分名称和语法保留字。类型名称也在词法层登记，以便后续类型解析
/// 使用同一份稳定 Token 流。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeywordKind {
    /// 函数定义关键字 `def`。
    Def,
    /// 条件关键字 `if`。
    If,
    /// 条件分支关键字 `elif`。
    Elif,
    /// 条件分支关键字 `else`。
    Else,
    /// 迭代关键字 `for`。
    For,
    /// 成员迭代关键字 `in`。
    In,
    /// 循环关键字 `while`。
    While,
    /// 返回关键字 `return`。
    Return,
    /// 循环控制关键字 `break`。
    Break,
    /// 循环控制关键字 `continue`。
    Continue,
    /// 编译期常量关键字 `const`。
    Const,
    /// 模块导入关键字 `import`。
    Import,
    /// 模块导入关键字 `from`。
    From,
    /// 显式转换关键字 `as`。
    As,
    /// 构造调用关键字 `new`。
    New,
    /// 显式释放关键字 `free`。
    Free,
    /// 逻辑与关键字 `and`。
    And,
    /// 逻辑或关键字 `or`。
    Or,
    /// 逻辑非关键字 `not`。
    Not,
    /// 身份比较关键字 `is`。
    Is,
    /// 默认 64 位整数类型关键字 `int`。
    Int,
    /// 32 位整数类型关键字 `sint`。
    Sint,
    /// 无限宽度整数类型关键字 `lint`。
    Lint,
    /// 默认 64 位浮点类型关键字 `float`。
    Float,
    /// 32 位浮点类型关键字 `sfloat`。
    Sfloat,
    /// 无限精度浮点类型关键字 `lfloat`。
    Lfloat,
    /// 字符串类型关键字 `str`。
    Str,
    /// 布尔类型关键字 `bool`。
    Bool,
}

impl KeywordKind {
    /// 返回保留字的稳定源码拼写。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Def => "def",
            Self::If => "if",
            Self::Elif => "elif",
            Self::Else => "else",
            Self::For => "for",
            Self::In => "in",
            Self::While => "while",
            Self::Return => "return",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Const => "const",
            Self::Import => "import",
            Self::From => "from",
            Self::As => "as",
            Self::New => "new",
            Self::Free => "free",
            Self::And => "and",
            Self::Or => "or",
            Self::Not => "not",
            Self::Is => "is",
            Self::Int => "int",
            Self::Sint => "sint",
            Self::Lint => "lint",
            Self::Float => "float",
            Self::Sfloat => "sfloat",
            Self::Lfloat => "lfloat",
            Self::Str => "str",
            Self::Bool => "bool",
        }
    }

    /// 根据源码单词查找保留字；未知单词返回 `None`。
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        Some(match word {
            "def" => Self::Def,
            "if" => Self::If,
            "elif" => Self::Elif,
            "else" => Self::Else,
            "for" => Self::For,
            "in" => Self::In,
            "while" => Self::While,
            "return" => Self::Return,
            "break" => Self::Break,
            "continue" => Self::Continue,
            "const" => Self::Const,
            "import" => Self::Import,
            "from" => Self::From,
            "as" => Self::As,
            "new" => Self::New,
            "free" => Self::Free,
            "and" => Self::And,
            "or" => Self::Or,
            "not" => Self::Not,
            "is" => Self::Is,
            "int" => Self::Int,
            "sint" => Self::Sint,
            "lint" => Self::Lint,
            "float" => Self::Float,
            "sfloat" => Self::Sfloat,
            "lfloat" => Self::Lfloat,
            "str" => Self::Str,
            "bool" => Self::Bool,
            _ => return None,
        })
    }
}

/// L1 词法器能够识别的 Token 种类。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    /// ASCII 普通标识符。
    Identifier,
    /// 十进制整数字面量（原始文本保留在源码区间中）。
    Integer,
    /// 十进制浮点字面量（原始文本保留在源码区间中）。
    Float,
    /// 单引号或双引号字符串字面量。
    String,
    /// `true` 或 `false` 布尔字面量。
    Boolean,
    /// `none` 空值字面量。
    None,
    /// 语法或类型保留字。
    Keyword(KeywordKind),
    /// 赋值符号 `=`。
    Equal,
    /// 相等比较符 `==`。
    EqualEqual,
    /// 不等比较符 `!=`。
    BangEqual,
    /// 小于比较符 `<`。
    Less,
    /// 小于等于比较符 `<=`。
    LessEqual,
    /// 大于比较符 `>`。
    Greater,
    /// 大于等于比较符 `>=`。
    GreaterEqual,
    /// 加法或集合并集候选符 `+`。
    Plus,
    /// 减法符 `-`。
    Minus,
    /// 乘法符 `*`。
    Star,
    /// 除法符 `/`。
    Slash,
    /// 整除符 `//`。
    FloorDiv,
    /// 取模符 `%`。
    Percent,
    /// 幂运算符 `**`。
    Power,
    /// 复合加法符 `+=`。
    PlusEqual,
    /// 复合减法符 `-=`。
    MinusEqual,
    /// 复合乘法符 `*=`。
    StarEqual,
    /// 复合除法符 `/=`。
    SlashEqual,
    /// 复合整除符 `//=`。
    FloorDivEqual,
    /// 复合取模符 `%=`。
    PercentEqual,
    /// 复合幂运算符 `**=`。
    PowerEqual,
    /// 左圆括号 `(`。
    LeftParen,
    /// 右圆括号 `)`。
    RightParen,
    /// 左方括号 `[`。
    LeftBracket,
    /// 右方括号 `]`。
    RightBracket,
    /// 左花括号 `{`。
    LeftBrace,
    /// 右花括号 `}`。
    RightBrace,
    /// 逗号 `,`。
    Comma,
    /// 冒号 `:`。
    Colon,
    /// 点号 `.`。
    Dot,
    /// 波浪号范围符 `~`。
    Tilde,
    /// 随机选择数量前缀 `?`。
    Question,
    /// 感叹号 `!`。
    Bang,
    /// 放回随机选择前缀 `!?`。
    BangQuestion,
    /// 重定义前缀 `@`。
    At,
    /// 插值或外部标记前缀 `$`。
    Dollar,
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
            Self::Float => "Float",
            Self::String => "String",
            Self::Boolean => "Boolean",
            Self::None => "None",
            Self::Keyword(keyword) => keyword.as_str(),
            Self::Equal => "Equal",
            Self::EqualEqual => "EqualEqual",
            Self::BangEqual => "BangEqual",
            Self::Less => "Less",
            Self::LessEqual => "LessEqual",
            Self::Greater => "Greater",
            Self::GreaterEqual => "GreaterEqual",
            Self::Plus => "Plus",
            Self::Minus => "Minus",
            Self::Star => "Star",
            Self::Slash => "Slash",
            Self::FloorDiv => "FloorDiv",
            Self::Percent => "Percent",
            Self::Power => "Power",
            Self::PlusEqual => "PlusEqual",
            Self::MinusEqual => "MinusEqual",
            Self::StarEqual => "StarEqual",
            Self::SlashEqual => "SlashEqual",
            Self::FloorDivEqual => "FloorDivEqual",
            Self::PercentEqual => "PercentEqual",
            Self::PowerEqual => "PowerEqual",
            Self::LeftParen => "LeftParen",
            Self::RightParen => "RightParen",
            Self::LeftBracket => "LeftBracket",
            Self::RightBracket => "RightBracket",
            Self::LeftBrace => "LeftBrace",
            Self::RightBrace => "RightBrace",
            Self::Comma => "Comma",
            Self::Colon => "Colon",
            Self::Dot => "Dot",
            Self::Tilde => "Tilde",
            Self::Question => "Question",
            Self::Bang => "Bang",
            Self::BangQuestion => "BangQuestion",
            Self::At => "At",
            Self::Dollar => "Dollar",
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

    /// 返回其中携带的保留字；非保留字 Token 返回 `None`。
    #[must_use]
    pub const fn keyword(self) -> Option<KeywordKind> {
        match self {
            Self::Keyword(keyword) => Some(keyword),
            _ => None,
        }
    }

    /// 判断 Token 是否为字面量。
    #[must_use]
    pub const fn is_literal(self) -> bool {
        matches!(
            self,
            Self::Integer | Self::Float | Self::String | Self::Boolean | Self::None
        )
    }

    /// 判断 Token 是否为运算符。
    #[must_use]
    pub const fn is_operator(self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::EqualEqual
                | Self::BangEqual
                | Self::Less
                | Self::LessEqual
                | Self::Greater
                | Self::GreaterEqual
                | Self::Plus
                | Self::Minus
                | Self::Star
                | Self::Slash
                | Self::FloorDiv
                | Self::Percent
                | Self::Power
                | Self::PlusEqual
                | Self::MinusEqual
                | Self::StarEqual
                | Self::SlashEqual
                | Self::FloorDivEqual
                | Self::PercentEqual
                | Self::PowerEqual
                | Self::Tilde
                | Self::Question
                | Self::Bang
                | Self::BangQuestion
        )
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

/// 词法诊断使用的统一结构化类型。
pub type LexDiagnostic = Diagnostic;

/// 一次完整词法扫描的结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexResult {
    /// 按源码顺序产生的 Token（包含 Invalid 和 Eof）。
    pub tokens: Vec<Token>,
    /// 扫描过程中收集的诊断。
    pub diagnostics: Vec<LexDiagnostic>,
}

/// Xiao L1 基础词法器。
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
            b'>' => self.scan_one_or_two(b'>', b'=', TokenKind::GreaterEqual, TokenKind::Greater),
            b'+' => self.scan_one_or_two(b'+', b'=', TokenKind::PlusEqual, TokenKind::Plus),
            b'-' => self.scan_minus(),
            b'*' => self.scan_star(),
            b'/' => self.scan_slash(),
            b'%' => self.scan_one_or_two(b'%', b'=', TokenKind::PercentEqual, TokenKind::Percent),
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
    use super::{
        INVALID_CHARACTER_CODE, INVALID_ESCAPE_CODE, INVALID_NUMBER_CODE, KeywordKind, Lexer,
        TokenKind, UNTERMINATED_STRING_CODE,
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
            "== != <= >= += -= *= /= // //= %= ** **= -> !? ! ~ ? < > + - * / %",
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
}
