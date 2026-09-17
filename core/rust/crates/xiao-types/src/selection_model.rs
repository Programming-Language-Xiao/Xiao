//! C1 有序容器选择的后端无关计划模型。
//!
//! 本模块只保存选择器经过类型阶段规范化后的结构，不读取或修改运行时
//! 容器。解释器、字节码和 LLVM 后端应消费同一份 [`SelectionPlan`]，从而
//! 不在各自实现一套范围、步长或随机选择语义。

use xiao_source::SourceSpan;
use xiao_syntax::RandomMode;

use crate::containers::ContainerPathSegment;
use crate::types::Type;

/// 规范化 Python 风格负索引。
///
/// `raw` 为负时按 `length + raw` 折算，折算结果必须落在 `[0, length)` 内，
/// 否则返回 `None`。这是选择器负索引语义的**唯一实现**：类型检查器与运行时
/// 都必须经它，不得各自再写一份。
#[must_use]
pub fn normalize_index(raw: i128, length: usize) -> Option<usize> {
    let index = if raw < 0 {
        (length as i128).checked_add(raw)?
    } else {
        raw
    };
    usize::try_from(index).ok().filter(|index| *index < length)
}

/// 一个选择路径中的语义段。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SelectionPathSegment {
    /// 数字索引；`raw` 保留负索引的源码语义，`resolved` 是已知长度下的
    /// 零基实际位置。未知长度时 `resolved` 为 `None`，交给 Runtime 检查。
    Index {
        /// 规范化前的有符号索引。
        raw: i128,
        /// 静态解析出的非负位置。
        resolved: Option<usize>,
    },
    /// 字典表或字典列的规范化键名。
    Key(String),
}

impl SelectionPathSegment {
    /// 创建一个尚未根据容器长度解析的数字索引。
    #[must_use]
    pub const fn index(raw: i128) -> Self {
        Self::Index {
            raw,
            resolved: None,
        }
    }

    /// 创建一个已经解析到实际位置的数字索引。
    #[must_use]
    pub const fn resolved_index(raw: i128, resolved: usize) -> Self {
        Self::Index {
            raw,
            resolved: Some(resolved),
        }
    }

    /// 创建一个键名路径段。
    #[must_use]
    pub fn key(value: impl Into<String>) -> Self {
        Self::Key(value.into())
    }

    /// 返回原始数字索引；键名段返回 `None`。
    #[must_use]
    pub const fn raw_index(&self) -> Option<i128> {
        match self {
            Self::Index { raw, .. } => Some(*raw),
            Self::Key(_) => None,
        }
    }

    /// 返回静态解析后的数字位置；未解析或键名段返回 `None`。
    #[must_use]
    pub const fn resolved_index_value(&self) -> Option<usize> {
        match self {
            Self::Index { resolved, .. } => *resolved,
            Self::Key(_) => None,
        }
    }

    /// 返回键名段的文本；数字段返回 `None`。
    #[must_use]
    pub fn key_value(&self) -> Option<&str> {
        match self {
            Self::Index { .. } => None,
            Self::Key(value) => Some(value),
        }
    }

    /// 将已经完全解析的路径转换为类型层路径。
    #[must_use]
    pub fn into_resolved_segment(&self) -> Option<ContainerPathSegment> {
        match self {
            Self::Index {
                resolved: Some(index),
                ..
            } => Some(ContainerPathSegment::Index(*index)),
            Self::Key(value) => Some(ContainerPathSegment::Key(value.clone())),
            Self::Index { resolved: None, .. } => None,
        }
    }
}

/// 选择器路径；段顺序与源码中的 `/` 路径一致。
pub type SelectionPath = Vec<SelectionPathSegment>;

/// 一个选择项经过规范化后的描述。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SelectionItemPlan {
    /// 一个精确路径。
    Exact {
        /// 被选择的路径。
        path: SelectionPath,
    },
    /// 两端边界信息和路径。
    Range {
        /// 起点路径。
        start: SelectionPath,
        /// 终点路径。
        end: SelectionPath,
        /// 是否包含起点。
        include_start: bool,
        /// 是否包含终点。
        include_end: bool,
    },
    /// 全量选择 `[=]`。
    All,
    /// 随机选择。
    Random {
        /// 无放回或放回模式。
        mode: RandomMode,
        /// 静态解析出的数量；动态数量为 `None`。
        count: Option<usize>,
        /// 数量是否需要 Runtime 检查。
        dynamic_count: bool,
    },
}

/// 步长的规范化描述。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StepPlan {
    /// 静态解析出的步长；动态表达式为 `None`。
    pub value: Option<i128>,
    /// 是否需要 Runtime 检查和求值。
    pub dynamic: bool,
}

impl StepPlan {
    /// 创建静态步长计划。
    #[must_use]
    pub const fn known(value: i128) -> Self {
        Self {
            value: Some(value),
            dynamic: false,
        }
    }

    /// 创建动态步长计划。
    #[must_use]
    pub const fn dynamic() -> Self {
        Self {
            value: None,
            dynamic: true,
        }
    }
}

/// 一次完整的选择表达式计划。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SelectionPlan {
    /// 选择表达式源码区间。
    pub span: SourceSpan,
    /// 来源容器的静态类型。
    pub source_type: Type,
    /// 选择结果的静态类型。
    pub result_type: Type,
    /// 按源码顺序保存的选择项。
    pub items: Vec<SelectionItemPlan>,
    /// 已经静态展开的路径；动态选择项可能没有对应路径。
    pub selected_paths: Vec<SelectionPath>,
    /// 每个静态目标路径对应的叶子类型，顺序与 `selected_paths` 一致。
    pub target_types: Vec<Type>,
    /// 可选步长。
    pub step: Option<StepPlan>,
    /// 是否含有需要 Runtime 才能确定的边界或结果。
    pub requires_runtime_check: bool,
    /// 是否存在放回随机选择。
    pub with_replacement: bool,
    /// 是否已经发现重复路径，或在放回随机中存在产生重复的可能。
    pub has_duplicates: bool,
}

impl SelectionPlan {
    /// 判断结果是否是单个精确元素读取。
    #[must_use]
    pub fn is_single_value(&self) -> bool {
        self.items.len() == 1
            && self.selected_paths.len() == 1
            && matches!(self.items.first(), Some(SelectionItemPlan::Exact { .. }))
            && !self.requires_runtime_check
    }

    /// 判断计划是否含有随机选择。
    #[must_use]
    pub fn has_random(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item, SelectionItemPlan::Random { .. }))
    }

    /// 返回静态选择路径的只读视图。
    #[must_use]
    pub fn paths(&self) -> &[SelectionPath] {
        &self.selected_paths
    }

    /// 返回静态目标叶子类型的只读视图。
    #[must_use]
    pub fn target_types(&self) -> &[Type] {
        &self.target_types
    }
}

/// 选择器左值的事务性标量广播计划。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BroadcastAssignmentPlan {
    /// 赋值表达式源码区间。
    pub span: SourceSpan,
    /// 被写入的根绑定名称；复杂来源暂不允许写入。
    pub root_name: Option<String>,
    /// 静态展开的目标路径。
    pub target_paths: Vec<SelectionPath>,
    /// 右侧值的静态类型。
    pub value_type: Type,
    /// 是否需要 Runtime 才能完成目标检查。
    pub dynamic: bool,
    /// 失败时是否要求整体回滚。
    pub transactional: bool,
}

impl BroadcastAssignmentPlan {
    /// 判断该计划是否已知所有目标路径。
    #[must_use]
    pub const fn has_static_targets(&self) -> bool {
        !self.dynamic
    }
}

/// `random.seed` 调用的类型阶段记录。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RandomSeedPlan {
    /// 调用源码区间。
    pub span: SourceSpan,
    /// 静态解析出的种子；动态值为 `None`。
    pub value: Option<u128>,
    /// 是否需要 Runtime 检查非负性和范围。
    pub dynamic: bool,
}

impl RandomSeedPlan {
    /// 判断种子是否已在编译期确定。
    #[must_use]
    pub const fn is_static(&self) -> bool {
        self.value.is_some() && !self.dynamic
    }
}
