//! P1 索引路径与选择器的语法树类型。
//!
//! 本模块只描述选择器的源码结构，不执行索引、边界检查或随机抽取。
//! 这些运行时语义由后续容器和类型阶段负责；因此路径和选择项始终保留
//! 原始 [`SourceSpan`]，便于诊断与后端映射。

use xiao_source::SourceSpan;

use crate::{Expression, Name};

/// 一个可嵌套的索引路径，例如 `3/2` 或 `1/`键``。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexPath {
    /// 按源码顺序保存的路径段。
    pub segments: Vec<PathSegment>,
    /// 覆盖整条路径的源码区间。
    pub span: SourceSpan,
}

impl IndexPath {
    /// 创建一条索引路径。
    #[must_use]
    pub const fn new(segments: Vec<PathSegment>, span: SourceSpan) -> Self {
        Self { segments, span }
    }

    /// 返回路径源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// 返回路径段数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// 判断路径是否没有段。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

/// 索引路径中的一个段。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathSegment {
    /// 数字索引；`negative` 表示源码带有前导减号。
    Integer {
        /// 覆盖可选减号和数字的源码区间。
        span: SourceSpan,
        /// 是否为负索引。
        negative: bool,
    },
    /// 字典键或命名路径段。
    Name(Name),
}

impl PathSegment {
    /// 返回路径段源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Integer { span, .. } => *span,
            Self::Name(name) => name.span,
        }
    }

    /// 判断该段是否为负整数索引。
    #[must_use]
    pub const fn is_negative_integer(&self) -> bool {
        matches!(self, Self::Integer { negative: true, .. })
    }
}

/// 方括号中的完整选择器。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selector {
    /// 按源码顺序保存的选择项；重复项不会在语法阶段去重。
    pub items: Vec<SelectorItem>,
    /// 覆盖方括号（含分隔符）的源码区间。
    pub span: SourceSpan,
}

impl Selector {
    /// 创建一个选择器。
    #[must_use]
    pub const fn new(items: Vec<SelectorItem>, span: SourceSpan) -> Self {
        Self { items, span }
    }

    /// 返回选择器源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// 一个选择项。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectorItem {
    /// 一个或多个嵌套路径段组成的精确选择。
    Exact {
        /// 被选择的路径。
        path: IndexPath,
        /// 选择项源码区间。
        span: SourceSpan,
    },
    /// 两端均包含的闭区间 `start~end`。
    Range {
        /// 起点路径。
        start: IndexPath,
        /// 终点路径。
        end: IndexPath,
        /// 选择项源码区间。
        span: SourceSpan,
    },
    /// 单边范围；缺失的一端以 `None` 表示。
    OpenRange {
        /// 可选起点。
        start: Option<IndexPath>,
        /// 可选终点。
        end: Option<IndexPath>,
        /// 起点是否包含（仅在 `start` 存在时有效）。
        include_start: bool,
        /// 终点是否包含（仅在 `end` 存在时有效）。
        include_end: bool,
        /// 选择项源码区间。
        span: SourceSpan,
    },
    /// 全量选择 `[=]`。
    All {
        /// 选择项源码区间。
        span: SourceSpan,
    },
    /// 随机选择 `[?count]` 或 `[!?count]`。
    Random {
        /// 无放回或放回模式。
        mode: RandomMode,
        /// 抽取数量表达式。
        count: Box<Expression>,
        /// 选择项源码区间。
        span: SourceSpan,
    },
}

impl SelectorItem {
    /// 返回选择项源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Exact { span, .. }
            | Self::Range { span, .. }
            | Self::OpenRange { span, .. }
            | Self::All { span }
            | Self::Random { span, .. } => *span,
        }
    }
}

/// 随机选择模式。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RandomMode {
    /// 无放回抽取，对应 `?`。
    WithoutReplacement,
    /// 放回抽取，对应 `!?`。
    WithReplacement,
}

impl RandomMode {
    /// 返回随机选择前缀的 Xiao 源码拼写。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WithoutReplacement => "?",
            Self::WithReplacement => "!?",
        }
    }
}
