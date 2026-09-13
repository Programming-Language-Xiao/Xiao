//! Xiao Token、关键字和字面量的公开表示。
//!
//! Token 只保存类别与源码区间，不解码用户值；词法器和解析器通过这些稳定类型
//! 通信，避免彼此依赖内部扫描状态。

use xiao_diagnostics::Diagnostic;
use xiao_source::{SourceError, SourceFile, SourcePosition, SourceSpan};

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

/// L2 词法器能够识别的 Token 种类。
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
    /// 反引号包裹的 UTF-8 名称。
    BacktickIdentifier,
    /// 一个完整的 `### ... ###` 文档注释块。
    DocComment,
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
    /// 代码块开始时的缩进。
    Indent,
    /// 代码块结束时的反缩进。
    Dedent,
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
            Self::BacktickIdentifier => "BacktickIdentifier",
            Self::DocComment => "DocComment",
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
            Self::Indent => "Indent",
            Self::Dedent => "Dedent",
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
