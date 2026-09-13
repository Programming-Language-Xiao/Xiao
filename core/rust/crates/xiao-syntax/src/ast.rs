//! Xiao P0/P1 抽象语法树与源码节点身份。
//!
//! AST 保留原始 UTF-8 字节区间，不执行类型检查或运行时行为。节点索引作为旁路
//! 元数据提供稳定身份，避免把类型层字段耦合进语法节点布局。

use xiao_source::{SourceFile, SourceSpan};

use crate::selectors::{Selector, SelectorItem};
use crate::token::{KeywordKind, TokenKind};

/// P0 解析得到的程序根节点。
///
/// `statements` 按源码顺序保存成功恢复出的顶层语句。解析器即使发现
/// 错误也会尽可能继续，因此 `ParseResult::program` 通常仍然包含部分
/// 结果；没有能够绑定到后续语句的文档注释会保存在 `orphan_doc_comments`。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    /// 按源码顺序解析出的顶层语句。
    pub statements: Vec<Statement>,
    /// 文件末尾没有相邻后续语句的文档注释区间。
    pub orphan_doc_comments: Vec<SourceSpan>,
    /// 覆盖整个输入源码的程序区间。
    pub span: SourceSpan,
}

impl Program {
    /// 返回程序覆盖的源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// 判断程序是否没有成功解析出的语句和文档注释。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty() && self.orphan_doc_comments.is_empty()
    }
}

/// P1 的一元运算符。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UnaryOperator {
    /// 一元正号。
    Plus,
    /// 一元负号。
    Minus,
    /// 逻辑非关键字 `not`。
    Not,
}

impl UnaryOperator {
    /// 返回运算符的 Xiao 源码拼写。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Not => "not",
        }
    }
}

/// P1 的二元运算符。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BinaryOperator {
    /// 幂运算 `**`，右结合。
    Power,
    /// 乘法 `*`。
    Multiply,
    /// 除法 `/`。
    Divide,
    /// 整除 `//`。
    FloorDivide,
    /// 取模 `%`。
    Remainder,
    /// 加法或集合并集候选符 `+`。
    Add,
    /// 减法 `-`。
    Subtract,
    /// 小于比较 `<`。
    Less,
    /// 小于等于比较 `<=`。
    LessEqual,
    /// 大于比较 `>`。
    Greater,
    /// 大于等于比较 `>=`。
    GreaterEqual,
    /// 相等比较 `==`。
    Equal,
    /// 不等比较 `!=`。
    NotEqual,
    /// 成员判断 `in`。
    In,
    /// 否定成员判断 `not in`。
    NotIn,
    /// 身份比较 `is`。
    Is,
    /// 否定身份比较 `is not`。
    IsNot,
    /// 逻辑与 `and`。
    And,
    /// 逻辑或 `or`。
    Or,
}

impl BinaryOperator {
    /// 返回运算符的 Xiao 源码拼写。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Power => "**",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::FloorDivide => "//",
            Self::Remainder => "%",
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::In => "in",
            Self::NotIn => "not in",
            Self::Is => "is",
            Self::IsNot => "is not",
            Self::And => "and",
            Self::Or => "or",
        }
    }
}

/// P1 的赋值运算符。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssignmentOperator {
    /// 普通赋值 `=`。
    Assign,
    /// 复合加法 `+=`。
    AddAssign,
    /// 复合减法 `-=`。
    SubtractAssign,
    /// 复合乘法 `*=`。
    MultiplyAssign,
    /// 复合除法 `/=`。
    DivideAssign,
    /// 复合整除 `//=`。
    FloorDivideAssign,
    /// 复合取模 `%=`。
    RemainderAssign,
    /// 复合幂运算 `**=`。
    PowerAssign,
}

impl AssignmentOperator {
    /// 返回赋值运算符的 Xiao 源码拼写。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubtractAssign => "-=",
            Self::MultiplyAssign => "*=",
            Self::DivideAssign => "/=",
            Self::FloorDivideAssign => "//=",
            Self::RemainderAssign => "%=",
            Self::PowerAssign => "**=",
        }
    }
}

/// `as` 转换在 P1 允许的标量目标类型。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScalarType {
    /// 默认 64 位整数。
    Int,
    /// 32 位整数。
    Sint,
    /// 无限宽度整数。
    Lint,
    /// 默认 64 位浮点。
    Float,
    /// 32 位浮点。
    Sfloat,
    /// 无限精度浮点。
    Lfloat,
    /// 字符串。
    Str,
    /// 布尔值。
    Bool,
}

impl ScalarType {
    /// 返回类型的 Xiao 源码拼写。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
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

    /// 将类型关键字映射为 P1 转换目标；其他关键字返回 `None`。
    #[must_use]
    pub const fn from_keyword(keyword: KeywordKind) -> Option<Self> {
        Some(match keyword {
            KeywordKind::Int => Self::Int,
            KeywordKind::Sint => Self::Sint,
            KeywordKind::Lint => Self::Lint,
            KeywordKind::Float => Self::Float,
            KeywordKind::Sfloat => Self::Sfloat,
            KeywordKind::Lfloat => Self::Lfloat,
            KeywordKind::Str => Self::Str,
            KeywordKind::Bool => Self::Bool,
            _ => return None,
        })
    }
}

/// P0/P1/P2 支持的顶层语句。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Statement {
    /// 一个独立的字面量或名称表达式语句。
    Expression {
        /// 语句的表达式。
        expression: Expression,
        /// 与该语句相邻、按源码顺序出现的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 语句源码区间，不包含结尾换行。
        span: SourceSpan,
    },
    /// 一个简单名称赋值语句。
    Assignment {
        /// 赋值左侧名称。
        target: Name,
        /// 赋值右侧的 P0 表达式。
        value: Expression,
        /// 与该语句相邻、按源码顺序出现的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 语句源码区间，不包含结尾换行。
        span: SourceSpan,
    },
    /// P1 的复合赋值或选择器目标赋值。
    ExtendedAssignment {
        /// 赋值左侧表达式；P1 只记录结构，不执行可写性检查。
        target: Expression,
        /// 赋值运算符。
        operator: AssignmentOperator,
        /// 赋值右侧表达式。
        value: Expression,
        /// 与该语句相邻、按源码顺序出现的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 语句源码区间，不包含结尾换行。
        span: SourceSpan,
    },
    /// 带标量类型前缀的静态声明，例如 `int count = 1` 或 `str name`。
    ///
    /// `value` 为空表示声明但尚未初始化；读取未初始化名称由类型检查阶段
    /// 报告，而不是由语法层拒绝。容器类型前缀和路径约束留给后续阶段。
    Declaration {
        /// 声明目标名称。
        target: Name,
        /// 声明时锁定的标量类型。
        declared_type: ScalarType,
        /// 可选初始化表达式。
        value: Option<Expression>,
        /// 与该语句相邻、按源码顺序出现的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 语句源码区间，不包含结尾换行。
        span: SourceSpan,
    },
    /// 编译期常量声明，例如 `const PI = 3.14`。
    ///
    /// 常量可以带可选标量类型前缀（`const int LIMIT = 1`）；初始化表达式
    /// 必须存在，是否能够在编译期求值由类型检查阶段验证。
    ConstDeclaration {
        /// 常量目标名称。
        target: Name,
        /// 可选的显式标量类型。
        declared_type: Option<ScalarType>,
        /// 必需的初始化表达式。
        value: Expression,
        /// 与该语句相邻、按源码顺序出现的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 语句源码区间，不包含结尾换行。
        span: SourceSpan,
    },
}

impl Statement {
    /// 返回语句覆盖的源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Expression { span, .. }
            | Self::Assignment { span, .. }
            | Self::ExtendedAssignment { span, .. }
            | Self::Declaration { span, .. }
            | Self::ConstDeclaration { span, .. } => *span,
        }
    }

    /// 返回挂接到语句前面的文档注释区间。
    #[must_use]
    pub fn leading_docs(&self) -> &[SourceSpan] {
        match self {
            Self::Expression { leading_docs, .. }
            | Self::Assignment { leading_docs, .. }
            | Self::ExtendedAssignment { leading_docs, .. }
            | Self::Declaration { leading_docs, .. }
            | Self::ConstDeclaration { leading_docs, .. } => leading_docs,
        }
    }

    /// 返回语句携带的主表达式；赋值语句返回右侧表达式。
    ///
    /// 无初始化声明没有主表达式，调用方应先使用 [`Self::try_expression`]
    /// 判断；直接调用本方法会以明确消息 panic。
    #[must_use]
    pub const fn expression(&self) -> &Expression {
        match self {
            Self::Expression { expression, .. } => expression,
            Self::Assignment { value, .. }
            | Self::ExtendedAssignment { value, .. }
            | Self::Declaration {
                value: Some(value), ..
            }
            | Self::ConstDeclaration { value, .. } => value,
            Self::Declaration { value: None, .. } => {
                // 无初始化声明没有主表达式；为了保持旧的便捷 API，返回一个
                // 仅用于诊断的静态哨兵并不安全，因此改用 panic 明确告知调用方。
                panic!("未初始化声明没有 expression；请先检查 initializer()")
            }
        }
    }

    /// 返回语句的可选主表达式；无初始化声明返回 `None`，不会触发 panic。
    #[must_use]
    pub const fn try_expression(&self) -> Option<&Expression> {
        match self {
            Self::Expression { expression, .. }
            | Self::Assignment {
                value: expression, ..
            }
            | Self::ExtendedAssignment {
                value: expression, ..
            }
            | Self::Declaration {
                value: Some(expression),
                ..
            }
            | Self::ConstDeclaration {
                value: expression, ..
            } => Some(expression),
            Self::Declaration { value: None, .. } => None,
        }
    }

    /// 返回赋值运算符；独立表达式语句返回 `None`。
    #[must_use]
    pub const fn assignment_operator(&self) -> Option<AssignmentOperator> {
        match self {
            Self::Expression { .. } => None,
            Self::Assignment { .. } => Some(AssignmentOperator::Assign),
            Self::ExtendedAssignment { operator, .. } => Some(*operator),
            Self::Declaration { .. } | Self::ConstDeclaration { .. } => None,
        }
    }

    /// 返回声明目标；非声明语句返回 `None`。
    #[must_use]
    pub const fn declaration_target(&self) -> Option<Name> {
        match self {
            Self::Declaration { target, .. } | Self::ConstDeclaration { target, .. } => {
                Some(*target)
            }
            _ => None,
        }
    }

    /// 返回声明的显式类型；未带类型的常量和非声明语句返回 `None`。
    #[must_use]
    pub const fn declared_type(&self) -> Option<ScalarType> {
        match self {
            Self::Declaration { declared_type, .. } => Some(*declared_type),
            Self::ConstDeclaration { declared_type, .. } => *declared_type,
            _ => None,
        }
    }

    /// 判断语句是否为编译期常量声明。
    #[must_use]
    pub const fn is_const_declaration(&self) -> bool {
        matches!(self, Self::ConstDeclaration { .. })
    }

    /// 返回可选的初始化表达式；无初始化声明或非声明语句返回 `None`。
    #[must_use]
    pub const fn initializer(&self) -> Option<&Expression> {
        match self {
            Self::Declaration { value, .. } => value.as_ref(),
            Self::ConstDeclaration { value, .. } => Some(value),
            _ => None,
        }
    }
}

/// P0 支持的名称及其原始源码位置。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Name {
    /// 名称完整源码区间；反引号名称包含首尾反引号。
    pub span: SourceSpan,
    /// 名称是否由反引号包裹。
    pub backticked: bool,
}

impl Name {
    /// 返回名称完整源码区间。
    #[must_use]
    pub const fn span(self) -> SourceSpan {
        self.span
    }

    /// 从源码中读取名称原始文本。
    ///
    /// 返回值保留反引号和其中的转义，不在 P0 阶段执行名称解码。
    #[must_use]
    pub fn text(self, source: &SourceFile) -> &str {
        source.slice(self.span)
    }

    /// 从源码中读取名称去除外层反引号后的文本。
    ///
    /// 该方法只去除一对分隔符，不解释反斜杠转义；需要语义化名称的
    /// 解码工作留给后续名称解析阶段。
    #[must_use]
    pub fn unquoted_text(self, source: &SourceFile) -> &str {
        let text = self.text(source);
        if self.backticked && text.len() >= 2 {
            &text[1..text.len() - 1]
        } else {
            text
        }
    }
}

/// P0/P1 的表达式语法树。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    /// 一个保留原始源码区间的字面量。
    Literal {
        /// 字面量类别。
        kind: LiteralKind,
        /// 字面量源码区间。
        span: SourceSpan,
    },
    /// 一个普通、反引号或构造式类型名称引用。
    Name(Name),
    /// 括号分组表达式。
    Group {
        /// 被分组的表达式。
        expression: Box<Expression>,
        /// 包含括号的源码区间。
        span: SourceSpan,
    },
    /// 一元表达式。
    Unary {
        /// 一元运算符。
        operator: UnaryOperator,
        /// 操作数。
        operand: Box<Expression>,
        /// 表达式源码区间。
        span: SourceSpan,
    },
    /// 二元表达式。
    Binary {
        /// 二元运算符。
        operator: BinaryOperator,
        /// 左操作数。
        left: Box<Expression>,
        /// 右操作数。
        right: Box<Expression>,
        /// 表达式源码区间。
        span: SourceSpan,
    },
    /// 函数或构造器的普通调用。
    Call {
        /// 被调用的表达式。
        callee: Box<Expression>,
        /// 按源码顺序保存的参数。
        arguments: Vec<Expression>,
        /// 表达式源码区间。
        span: SourceSpan,
    },
    /// `new Type(args...)` 构造调用。
    NewCall {
        /// 被构造的类型或名称表达式。
        callee: Box<Expression>,
        /// 按源码顺序保存的参数。
        arguments: Vec<Expression>,
        /// 表达式源码区间。
        span: SourceSpan,
    },
    /// 点号成员访问。
    Member {
        /// 成员所属对象。
        object: Box<Expression>,
        /// 成员名称。
        member: Name,
        /// 表达式源码区间。
        span: SourceSpan,
    },
    /// `value as scalar_type` 显式转换。
    Cast {
        /// 被转换的表达式。
        expression: Box<Expression>,
        /// 标量目标类型。
        target: ScalarType,
        /// 表达式源码区间。
        span: SourceSpan,
    },
    /// 带可选步长的索引/选择器后缀。
    Selector {
        /// 被选择的来源表达式。
        source: Box<Expression>,
        /// 可选步长表达式。
        step: Option<Box<Expression>>,
        /// 方括号选择器。
        selector: Selector,
        /// 表达式源码区间。
        span: SourceSpan,
    },
}

impl Expression {
    /// 返回表达式覆盖的源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Literal { span, .. } => *span,
            Self::Name(name) => name.span,
            Self::Group { span, .. }
            | Self::Unary { span, .. }
            | Self::Binary { span, .. }
            | Self::Call { span, .. }
            | Self::NewCall { span, .. }
            | Self::Member { span, .. }
            | Self::Cast { span, .. }
            | Self::Selector { span, .. } => *span,
        }
    }

    /// 返回字面量类别；名称表达式返回 `None`。
    #[must_use]
    pub const fn literal_kind(&self) -> Option<LiteralKind> {
        match self {
            Self::Literal { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// 从源码中读取表达式的原始文本。
    #[must_use]
    pub fn text<'source>(&self, source: &'source SourceFile) -> &'source str {
        source.slice(self.span())
    }
}

/// P0 解析器识别的字面量类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LiteralKind {
    /// 十进制整数。
    Integer,
    /// 十进制浮点数。
    Float,
    /// 单引号或双引号字符串。
    String,
    /// `true` 或 `false`。
    Boolean,
    /// `none` 空值。
    None,
}

impl LiteralKind {
    /// 将词法 Token 类别映射为 P0 字面量类别。
    #[must_use]
    pub const fn from_token_kind(kind: TokenKind) -> Option<Self> {
        match kind {
            TokenKind::Integer => Some(Self::Integer),
            TokenKind::Float => Some(Self::Float),
            TokenKind::String => Some(Self::String),
            TokenKind::Boolean => Some(Self::Boolean),
            TokenKind::None => Some(Self::None),
            _ => None,
        }
    }
}

/// 模块内稳定的 AST 节点身份。
///
/// 身份由 NodeIndex::build 按源码先序分配；它不编码内存地址，也不跨模块复用。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(u32);

impl NodeId {
    /// 创建一个节点身份；仅供语法和类型层构造索引时使用。
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回节点身份的数值表示。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// AST 节点到稳定身份的旁路索引。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIndex {
    /// 按源码先序保存节点区间和身份。
    pub nodes: Vec<(NodeId, SourceSpan)>,
}

impl NodeIndex {
    /// 从程序构建稳定的节点索引。
    #[must_use]
    pub fn build(program: &Program) -> Self {
        let mut index = Self { nodes: Vec::new() };
        for statement in &program.statements {
            index.visit_statement(statement);
        }
        index
    }

    /// 返回索引中的节点数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 判断索引是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 为一个源码区间分配下一个稳定节点编号。
    fn push(&mut self, span: SourceSpan) {
        let id = NodeId::new(self.nodes.len() as u32);
        self.nodes.push((id, span));
    }

    /// 按先序访问语句及其子表达式。
    fn visit_statement(&mut self, statement: &Statement) {
        self.push(statement.span());
        match statement {
            Statement::Expression { expression, .. }
            | Statement::Assignment {
                value: expression, ..
            }
            | Statement::ConstDeclaration {
                value: expression, ..
            } => self.visit_expression(expression),
            Statement::Declaration {
                value: Some(expression),
                ..
            } => self.visit_expression(expression),
            Statement::Declaration { value: None, .. } => {}
            Statement::ExtendedAssignment { target, value, .. } => {
                self.visit_expression(target);
                self.visit_expression(value);
            }
        }
    }

    /// 按语法书写顺序递归访问表达式。
    fn visit_expression(&mut self, expression: &Expression) {
        self.push(expression.span());
        match expression {
            Expression::Literal { .. } | Expression::Name(_) => {}
            Expression::Group { expression, .. }
            | Expression::Unary {
                operand: expression,
                ..
            }
            | Expression::Cast { expression, .. } => self.visit_expression(expression),
            Expression::Binary { left, right, .. } => {
                self.visit_expression(left);
                self.visit_expression(right);
            }
            Expression::Call {
                callee, arguments, ..
            }
            | Expression::NewCall {
                callee, arguments, ..
            } => {
                self.visit_expression(callee);
                for argument in arguments {
                    self.visit_expression(argument);
                }
            }
            Expression::Member { object, .. } => self.visit_expression(object),
            Expression::Selector {
                source,
                step,
                selector,
                ..
            } => {
                self.visit_expression(source);
                if let Some(step) = step {
                    self.visit_expression(step);
                }
                for item in &selector.items {
                    if let SelectorItem::Random { count, .. } = item {
                        self.visit_expression(count);
                    }
                }
            }
        }
    }
}
