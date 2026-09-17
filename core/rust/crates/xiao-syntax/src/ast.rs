//! Xiao P0/P1 抽象语法树与源码节点身份。
//!
//! AST 保留原始 UTF-8 字节区间，不执行类型检查或运行时行为。节点索引作为旁路
//! 元数据提供稳定身份，避免把类型层字段耦合进语法节点布局。

use xiao_source::{SourceFile, SourceSpan};

use crate::imports::ImportStatement;
use crate::selectors::{IndexPath, Selector, SelectorItem};
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
    /// 程序入口模式；未出现 `[main]` 时为脚本模式。
    pub entry_mode: EntryMode,
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

/// 程序的静态入口模式。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryMode {
    /// 顶层可执行语句自动组成入口。
    Script,
    /// 存在显式 `[main]` 入口表头。
    Project {
        /// `[main]` 表头源码区间。
        span: SourceSpan,
    },
}

/// 表声明的两种静态形态。
///
/// 单例表 (`[Name]`) 提供一个稳定的命名空间；可实例化表
/// (`[[Name]]`) 提供 `new` 的构造目标。表值的运行时存储和生命周期
/// 不属于语法层。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TableKind {
    /// `[Name]` 单例表。
    Singleton,
    /// `[[Name]]` 可实例化表。
    Instance,
}

impl TableKind {
    /// 返回表头使用的左侧括号数量。
    #[must_use]
    pub const fn opening_bracket_count(self) -> usize {
        match self {
            Self::Singleton => 1,
            Self::Instance => 2,
        }
    }

    /// 判断该表是否可作为 `new` 的构造目标。
    #[must_use]
    pub const fn is_instantiable(self) -> bool {
        matches!(self, Self::Instance)
    }

    /// 返回稳定的 Xiao 表头拼写。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Singleton => "[Table]",
            Self::Instance => "[[Table]]",
        }
    }
}

impl EntryMode {
    /// 判断是否为脚本入口模式。
    #[must_use]
    pub const fn is_script(self) -> bool {
        matches!(self, Self::Script)
    }

    /// 判断是否为工程入口模式。
    #[must_use]
    pub const fn is_project(self) -> bool {
        matches!(self, Self::Project { .. })
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
    /// 集合交集或位与候选符 `&`。
    Intersect,
    /// 集合对称差或位异或候选符 `^`。
    SymmetricDifference,
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
            Self::Intersect => "&",
            Self::SymmetricDifference => "^",
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
    /// 复合集合交集 `&=`。
    IntersectAssign,
    /// 复合集合对称差 `^=`。
    SymmetricDifferenceAssign,
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
            Self::IntersectAssign => "&=",
            Self::SymmetricDifferenceAssign => "^=",
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

    /// 按 Xiao 源码拼写还原标量类型；未知拼写返回 `None`。
    ///
    /// 与 [`Self::as_str`] 共用同一张拼写表，调用方不应另存一份映射。
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
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

/// C2-B 集合类型注解中允许出现的基础类型项。
///
/// C2-B 只开放标量和 `none`。数组、字典、函数等递归类型会在后续通用
/// 类型语法阶段加入，不能通过临时字符串绕过当前边界。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TypeTerm {
    /// 一个标量类型项。
    Scalar(ScalarType),
    /// `none` 类型项。
    None,
}

impl TypeTerm {
    /// 返回类型项的稳定 Xiao 拼写。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scalar(scalar) => scalar.as_str(),
            Self::None => "none",
        }
    }
}

/// C2-B 的集合类型注解，例如 `set<int | str>`。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SetTypeAnnotation {
    /// 按源码顺序暂存的类型项；类型层会去重并规范化顺序。
    pub members: Vec<TypeTerm>,
    /// 覆盖 `set<...>` 的源码区间。
    pub span: SourceSpan,
}

impl SetTypeAnnotation {
    /// 创建一份集合类型注解。
    #[must_use]
    pub fn new(members: impl Into<Vec<TypeTerm>>, span: SourceSpan) -> Self {
        Self {
            members: members.into(),
            span,
        }
    }

    /// 返回注解是否没有类型项。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

/// 声明语句使用的显式类型注解。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DeclaredType {
    /// 旧版 `int name` 等标量声明。
    Scalar(ScalarType),
    /// C2-B `set<T>` 或 `set<T | U>` 集合声明。
    Set(SetTypeAnnotation),
}

impl DeclaredType {
    /// 返回标量声明项；集合声明返回 `None`。
    #[must_use]
    pub const fn as_scalar(&self) -> Option<ScalarType> {
        match self {
            Self::Scalar(scalar) => Some(*scalar),
            Self::Set(_) => None,
        }
    }

    /// 返回集合声明项；标量声明返回 `None`。
    #[must_use]
    pub const fn as_set(&self) -> Option<&SetTypeAnnotation> {
        match self {
            Self::Scalar(_) => None,
            Self::Set(annotation) => Some(annotation),
        }
    }
}

/// 函数参数的绑定种类。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FunctionParameterKind {
    /// 只能通过位置传入的参数（`/` 标记之前的参数）。
    PositionalOnly,
    /// 可以通过位置或关键字传入的普通参数。
    PositionalOrKeyword,
    /// 只能通过关键字传入的参数（裸 `*` 之后的参数）。
    KeywordOnly,
    /// 可变位置参数（`*args`）。
    VarArgs,
    /// 可变关键字参数（`**kwargs`）。
    VarKeywords,
}

impl FunctionParameterKind {
    /// 判断参数是否会消耗一个普通位置实参。
    #[must_use]
    pub const fn accepts_positional(self) -> bool {
        matches!(
            self,
            Self::PositionalOnly | Self::PositionalOrKeyword | Self::VarArgs
        )
    }

    /// 判断参数是否可以通过关键字传入。
    #[must_use]
    pub const fn accepts_keyword(self) -> bool {
        matches!(
            self,
            Self::PositionalOrKeyword | Self::KeywordOnly | Self::VarKeywords
        )
    }
}

/// 函数参数或返回值的首版显式类型注解。
///
/// 04 阶段先复用标量和 `none` 类型；容器/函数类型注解由后续通用类型语法
/// 阶段扩展。该枚举独立于变量声明的 [`DeclaredType`]，避免把参数规则耦合
/// 到容器路径约束。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FunctionTypeAnnotation {
    /// 一个标量类型。
    Scalar(ScalarType),
    /// `none` 类型。
    None,
}

impl FunctionTypeAnnotation {
    /// 返回注解的稳定 Xiao 拼写。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scalar(scalar) => scalar.as_str(),
            Self::None => "none",
        }
    }
}

/// 函数定义中的一个参数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionParameter {
    /// 参数名称。
    pub name: Name,
    /// 参数绑定种类。
    pub kind: FunctionParameterKind,
    /// 可选的显式类型注解。
    pub annotation: Option<FunctionTypeAnnotation>,
    /// 可选默认值；可变参数没有默认值。
    pub default: Option<Expression>,
    /// 覆盖整个参数的源码区间。
    pub span: SourceSpan,
}

impl FunctionParameter {
    /// 判断该参数是否为可变参数。
    #[must_use]
    pub const fn is_variadic(&self) -> bool {
        matches!(
            self.kind,
            FunctionParameterKind::VarArgs | FunctionParameterKind::VarKeywords
        )
    }
}

/// 调用表达式中一个实参的传递方式。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CallArgumentKind {
    /// 普通位置实参。
    Positional,
    /// `name=value` 关键字实参。
    Keyword,
    /// `*values` 可变位置展开。
    Star,
    /// `**values` 可变关键字展开。
    DoubleStar,
}

/// 调用表达式中的实参。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallArgument {
    /// 关键字实参的名称；其他种类为 `None`。
    pub name: Option<Name>,
    /// 实参表达式（展开参数保存被展开的来源表达式）。
    pub value: Expression,
    /// 实参传递方式。
    pub kind: CallArgumentKind,
    /// 覆盖前缀、名称和表达式的源码区间。
    pub span: SourceSpan,
}

impl CallArgument {
    /// 创建一个普通位置实参。
    #[must_use]
    pub fn positional(value: Expression) -> Self {
        let span = value.span();
        Self {
            name: None,
            value,
            kind: CallArgumentKind::Positional,
            span,
        }
    }

    /// 判断实参是否为关键字或展开形式。
    #[must_use]
    pub const fn is_named_or_expanded(&self) -> bool {
        !matches!(self.kind, CallArgumentKind::Positional)
    }

    /// 返回实参覆盖的源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// `elif` 分支的条件和缩进体。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElifBranch {
    /// 分支条件。
    pub condition: Expression,
    /// 分支缩进体。
    pub body: Vec<Statement>,
    /// 与该分支相邻的文档注释区间。
    pub leading_docs: Vec<SourceSpan>,
    /// 覆盖 `elif` 头和其代码块的源码区间。
    pub span: SourceSpan,
}

/// `catch` 处理器的静态结构。
///
/// 错误类型先保留为名称，而不是在语法层绑定某个 Runtime 错误枚举；这样
/// 用户定义错误和后续错误模块都能沿用同一份 AST。类型层负责检查名称形状、
/// 捕获顺序以及 `FatalError` 的不可恢复边界。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatchClause {
    /// 处理器中绑定错误对象的名称。
    pub binding: Name,
    /// 按类型匹配的错误类型名称。
    pub error_type: Name,
    /// 处理器缩进体。
    pub body: Vec<Statement>,
    /// 与 `catch` 头相邻的文档注释区间。
    pub leading_docs: Vec<SourceSpan>,
    /// 覆盖 `catch` 头和处理器的源码区间。
    pub span: SourceSpan,
}

/// P0/P1/P2/04 支持的语句。
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
    /// 带显式类型前缀的静态声明，例如 `int count = 1` 或
    /// `set<int | str> values = {1, "x"}`。
    ///
    /// `value` 为空表示声明但尚未初始化；读取未初始化名称由类型检查阶段
    /// 报告，而不是由语法层拒绝。容器类型前缀和路径约束留给后续阶段。
    Declaration {
        /// 声明目标名称。
        target: Name,
        /// 声明时锁定的显式类型。
        declared_type: DeclaredType,
        /// 可选的数组嵌套路径约束，例如 `[3/2]`。
        constraint_path: Option<IndexPath>,
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
    /// `import` 或 `from ... import ...` 模块导入语句。
    ///
    /// 导入路径和别名只保留语法结构；文件发现、名称绑定和依赖图由
    /// `xiao-modules` 在后续阶段处理。
    Import {
        /// 导入语句的具体形式。
        import: ImportStatement,
        /// 与导入语句相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 语句源码区间。
        span: SourceSpan,
    },
    /// `[Table]` 或 `[[Table]]` 表声明。
    ///
    /// `body` 按源码顺序保存字段和方法语句。语法层保留普通语句形状，
    /// 表成员白名单、字段类型和生命周期签名由类型层检查；这样解析器
    /// 可以在错误输入上继续恢复，而不会把语义规则耦合进 Token 消费。
    Table {
        /// 表名称；表头名称在解析阶段限定为 ASCII 标识符。
        name: Name,
        /// 表的单例/可实例化形态。
        kind: TableKind,
        /// 表体成员，按源码顺序排列。
        body: Vec<Statement>,
        /// 与表头相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 覆盖表头和表体的源码区间。
        span: SourceSpan,
    },
    /// `def name(...) -> type` 函数定义。
    Function {
        /// 函数名称。
        name: Name,
        /// 参数列表。
        parameters: Vec<FunctionParameter>,
        /// 可选返回类型注解。
        return_type: Option<FunctionTypeAnnotation>,
        /// 缩进函数体。
        body: Vec<Statement>,
        /// 与函数定义相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 覆盖函数头和函数体的源码区间。
        span: SourceSpan,
    },
    /// `if`、`elif`、`else` 条件语句。
    If {
        /// 首个 `if` 条件。
        condition: Expression,
        /// 首个条件体。
        body: Vec<Statement>,
        /// 后续 `elif` 分支。
        elif_branches: Vec<ElifBranch>,
        /// 可选 `else` 体。
        else_body: Option<Vec<Statement>>,
        /// 与 `if` 语句相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 覆盖整个条件链的源码区间。
        span: SourceSpan,
    },
    /// `for name in iterable` 循环。
    For {
        /// 循环绑定名称。
        target: Name,
        /// 被遍历的表达式。
        iterable: Expression,
        /// 循环缩进体。
        body: Vec<Statement>,
        /// 与循环相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 覆盖循环头和循环体的源码区间。
        span: SourceSpan,
    },
    /// `while condition` 循环。
    While {
        /// 循环条件。
        condition: Expression,
        /// 循环缩进体。
        body: Vec<Statement>,
        /// 与循环相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 覆盖循环头和循环体的源码区间。
        span: SourceSpan,
    },
    /// `return` 返回语句。
    Return {
        /// 可选返回表达式；省略时返回 `none`。
        value: Option<Expression>,
        /// 与返回语句相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 返回语句源码区间。
        span: SourceSpan,
    },
    /// `break` 循环控制语句。
    Break {
        /// 与控制语句相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 控制语句源码区间。
        span: SourceSpan,
    },
    /// `continue` 循环控制语句。
    Continue {
        /// 与控制语句相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 控制语句源码区间。
        span: SourceSpan,
    },
    /// `try` 主体、按类型匹配的 `catch` 列表和可选 `finally` 清理体。
    Try {
        /// 受保护的缩进体。
        body: Vec<Statement>,
        /// 按源码顺序排列的错误处理器。
        catches: Vec<CatchClause>,
        /// 可选的最终清理体。
        finally_body: Option<Vec<Statement>>,
        /// 与 `try` 相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 覆盖整个错误控制流结构的源码区间。
        span: SourceSpan,
    },
    /// 抛出一个可恢复错误表达式。
    Raise {
        /// 错误表达式；运行时必须产生 `XiaoError`，不能产生 `FatalError`。
        value: Expression,
        /// 与 `raise` 相邻的文档注释区间。
        leading_docs: Vec<SourceSpan>,
        /// 语句源码区间。
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
            | Self::ConstDeclaration { span, .. }
            | Self::Import { span, .. }
            | Self::Table { span, .. }
            | Self::Function { span, .. }
            | Self::If { span, .. }
            | Self::For { span, .. }
            | Self::While { span, .. }
            | Self::Return { span, .. }
            | Self::Break { span, .. }
            | Self::Continue { span, .. } => *span,
            Self::Try { span, .. } | Self::Raise { span, .. } => *span,
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
            | Self::ConstDeclaration { leading_docs, .. }
            | Self::Import { leading_docs, .. }
            | Self::Table { leading_docs, .. }
            | Self::Function { leading_docs, .. }
            | Self::If { leading_docs, .. }
            | Self::For { leading_docs, .. }
            | Self::While { leading_docs, .. }
            | Self::Return { leading_docs, .. }
            | Self::Break { leading_docs, .. }
            | Self::Continue { leading_docs, .. } => leading_docs,
            Self::Try { leading_docs, .. } | Self::Raise { leading_docs, .. } => leading_docs,
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
            Self::Return {
                value: Some(value), ..
            } => value,
            Self::Declaration { value: None, .. }
            | Self::Import { .. }
            | Self::Table { .. }
            | Self::Function { .. }
            | Self::If { .. }
            | Self::For { .. }
            | Self::While { .. }
            | Self::Return { value: None, .. }
            | Self::Break { .. }
            | Self::Continue { .. } => {
                // 无初始化声明没有主表达式；为了保持旧的便捷 API，返回一个
                // 仅用于诊断的静态哨兵并不安全，因此改用 panic 明确告知调用方。
                panic!("未初始化声明没有 expression；请先检查 initializer()")
            }
            Self::Try { .. } => panic!("try 语句没有单一 expression；请遍历其主体"),
            Self::Raise { value, .. } => value,
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
            Self::Declaration { value: None, .. }
            | Self::Import { .. }
            | Self::Table { .. }
            | Self::Function { .. }
            | Self::If { .. }
            | Self::For { .. }
            | Self::While { .. }
            | Self::Return { value: None, .. }
            | Self::Break { .. }
            | Self::Continue { .. } => None,
            Self::Try { .. } => None,
            Self::Raise { value, .. } => Some(value),
            Self::Return {
                value: Some(expression),
                ..
            } => Some(expression),
        }
    }

    /// 返回赋值运算符；独立表达式语句返回 `None`。
    #[must_use]
    pub const fn assignment_operator(&self) -> Option<AssignmentOperator> {
        match self {
            Self::Expression { .. } => None,
            Self::Assignment { .. } => Some(AssignmentOperator::Assign),
            Self::ExtendedAssignment { operator, .. } => Some(*operator),
            Self::Declaration { .. }
            | Self::ConstDeclaration { .. }
            | Self::Import { .. }
            | Self::Table { .. }
            | Self::Function { .. }
            | Self::If { .. }
            | Self::For { .. }
            | Self::While { .. }
            | Self::Return { .. }
            | Self::Break { .. }
            | Self::Continue { .. } => None,
            Self::Try { .. } | Self::Raise { .. } => None,
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
            Self::Declaration { declared_type, .. } => declared_type.as_scalar(),
            Self::ConstDeclaration { declared_type, .. } => *declared_type,
            _ => None,
        }
    }

    /// 返回声明携带的完整类型注解；未带类型的常量和非声明语句返回 `None`。
    #[must_use]
    pub const fn type_annotation(&self) -> Option<&DeclaredType> {
        match self {
            Self::Declaration { declared_type, .. } => Some(declared_type),
            Self::ConstDeclaration { .. } => None,
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

    /// 返回标量声明携带的可选数组路径约束。
    #[must_use]
    pub const fn declaration_path(&self) -> Option<&IndexPath> {
        match self {
            Self::Declaration {
                constraint_path, ..
            } => constraint_path.as_ref(),
            _ => None,
        }
    }

    /// 返回表声明名称；非表语句返回 `None`。
    #[must_use]
    pub const fn table_name(&self) -> Option<Name> {
        match self {
            Self::Table { name, .. } => Some(*name),
            _ => None,
        }
    }

    /// 返回表声明形态；非表语句返回 `None`。
    #[must_use]
    pub const fn table_kind(&self) -> Option<TableKind> {
        match self {
            Self::Table { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// 返回表体成员；非表语句返回 `None`。
    #[must_use]
    pub fn table_body(&self) -> Option<&[Statement]> {
        match self {
            Self::Table { body, .. } => Some(body),
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

/// P0/P1/C2-A 的表达式语法树。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    /// 一个保留原始源码区间的字面量。
    Literal {
        /// 字面量类别。
        kind: LiteralKind,
        /// 字面量源码区间。
        span: SourceSpan,
    },
    /// 数组字面量；元素可以是任意递归表达式。
    ArrayLiteral {
        /// 按源码顺序保存的数组元素。
        elements: Vec<Expression>,
        /// 包含方括号的源码区间。
        span: SourceSpan,
    },
    /// Python 风格元组字面量。
    TupleLiteral {
        /// 按源码顺序保存的元组元素。
        elements: Vec<Expression>,
        /// 包含圆括号的源码区间。
        span: SourceSpan,
    },
    /// 无序字典表字面量。
    DictTableLiteral {
        /// 按源码顺序保存条目；语义层不依赖该顺序。
        entries: Vec<DictEntry>,
        /// 包含花括号的源码区间。
        span: SourceSpan,
    },
    /// 无序集合字面量。
    ///
    /// 元素按源码顺序暂存只是为了保持诊断和节点索引稳定；集合的
    /// 语义层不得把这个顺序当作可观察的迭代顺序。
    SetLiteral {
        /// 按源码顺序保存集合元素。
        elements: Vec<Expression>,
        /// 包含花括号的源码区间。
        span: SourceSpan,
    },
    /// 保持书写顺序的字典列字面量。
    DictColumnLiteral {
        /// 按源码顺序保存条目。
        entries: Vec<DictEntry>,
        /// 包含尖括号的源码区间。
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
        /// 按源码顺序保存的位置、关键字和展开参数。
        arguments: Vec<CallArgument>,
        /// 表达式源码区间。
        span: SourceSpan,
    },
    /// `new Type(args...)` 构造调用。
    NewCall {
        /// 被构造的类型或名称表达式。
        callee: Box<Expression>,
        /// 按源码顺序保存的位置、关键字和展开参数。
        arguments: Vec<CallArgument>,
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
            Self::Literal { span, .. }
            | Self::ArrayLiteral { span, .. }
            | Self::TupleLiteral { span, .. }
            | Self::DictTableLiteral { span, .. }
            | Self::SetLiteral { span, .. }
            | Self::DictColumnLiteral { span, .. } => *span,
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

    /// 返回表达式源码区间的结束偏移。
    #[must_use]
    pub const fn span_end(&self) -> usize {
        self.span().end()
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

/// 字典表或字典列中的键；C0 只接受名称键和字符串键。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DictKey {
    /// 裸名称或反引号名称键。
    Name(Name),
    /// 字符串字面量键，区间包含引号。
    String(SourceSpan),
}

impl DictKey {
    /// 返回键在源码中的区间。
    #[must_use]
    pub const fn span(self) -> SourceSpan {
        match self {
            Self::Name(name) => name.span,
            Self::String(span) => span,
        }
    }
}

/// 一个字典表或字典列条目。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DictEntry {
    /// 条目的名称或字符串键。
    pub key: DictKey,
    /// 条目对应的值表达式。
    pub value: Expression,
    /// 从键开始到值结束的源码区间。
    pub span: SourceSpan,
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
            Statement::Import { .. } => {}
            Statement::Table { body, .. } => {
                for statement in body {
                    self.visit_statement(statement);
                }
            }
            Statement::ExtendedAssignment { target, value, .. } => {
                self.visit_expression(target);
                self.visit_expression(value);
            }
            Statement::Function {
                parameters, body, ..
            } => {
                for parameter in parameters {
                    self.push(parameter.span);
                    if let Some(default) = &parameter.default {
                        self.visit_expression(default);
                    }
                }
                for statement in body {
                    self.visit_statement(statement);
                }
            }
            Statement::If {
                condition,
                body,
                elif_branches,
                else_body,
                ..
            } => {
                self.visit_expression(condition);
                for statement in body {
                    self.visit_statement(statement);
                }
                for branch in elif_branches {
                    self.push(branch.span);
                    self.visit_expression(&branch.condition);
                    for statement in &branch.body {
                        self.visit_statement(statement);
                    }
                }
                if let Some(body) = else_body {
                    for statement in body {
                        self.visit_statement(statement);
                    }
                }
            }
            Statement::For { iterable, body, .. }
            | Statement::While {
                condition: iterable,
                body,
                ..
            } => {
                self.visit_expression(iterable);
                for statement in body {
                    self.visit_statement(statement);
                }
            }
            Statement::Return {
                value: Some(value), ..
            } => self.visit_expression(value),
            Statement::Return { value: None, .. }
            | Statement::Break { .. }
            | Statement::Continue { .. } => {}
            Statement::Raise { value, .. } => self.visit_expression(value),
            Statement::Try {
                body,
                catches,
                finally_body,
                ..
            } => {
                for statement in body {
                    self.visit_statement(statement);
                }
                for catch in catches {
                    self.push(catch.span);
                    self.push(catch.binding.span);
                    self.push(catch.error_type.span);
                    for statement in &catch.body {
                        self.visit_statement(statement);
                    }
                }
                if let Some(body) = finally_body {
                    for statement in body {
                        self.visit_statement(statement);
                    }
                }
            }
        }
    }

    /// 按语法书写顺序递归访问表达式。
    fn visit_expression(&mut self, expression: &Expression) {
        self.push(expression.span());
        match expression {
            Expression::Literal { .. } | Expression::Name(_) => {}
            Expression::ArrayLiteral { elements, .. }
            | Expression::TupleLiteral { elements, .. }
            | Expression::SetLiteral { elements, .. } => {
                for element in elements {
                    self.visit_expression(element);
                }
            }
            Expression::DictTableLiteral { entries, .. }
            | Expression::DictColumnLiteral { entries, .. } => {
                for entry in entries {
                    self.push(entry.key.span());
                    self.visit_expression(&entry.value);
                }
            }
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
                    self.push(argument.span);
                    self.visit_expression(&argument.value);
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
