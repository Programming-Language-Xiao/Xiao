//! 类型检查器的公开结果模型。
//!
//! 这些类型构成 `xiao-types` 对外的稳定检查结果面；实现模块只负责填充它们。

use std::collections::BTreeMap;

use xiao_diagnostics::Diagnostic;
use xiao_source::SourceSpan;
use xiao_syntax::EntryMode;

use crate::containers::ContainerMaterializationPlan;
use crate::environment::{Binding, TypeEnvironment};
use crate::functions::FunctionSignature;
use crate::types::Type;

/// 后端需要保留的运行时检查种类。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeCheckKind {
    /// 固定宽度整数或浮点范围检查。
    NumericRange,
    /// `str` 到 `bool` 的四种合法拼写检查。
    StringBoolean,
    /// 动态值转换的运行时类型检查。
    DynamicConversion,
    /// 动态算术的除零、溢出或操作数检查。
    Arithmetic,
    /// 动态选择器索引、范围端点或字符串位置检查。
    SelectorBounds,
    /// 动态选择器步长检查。
    SelectorStep,
    /// 动态随机抽取数量和候选集边界检查。
    RandomCount,
    /// 动态 `random.seed` 非负性和表示范围检查。
    RandomSeed,
    /// 动态集合元素的可哈希性检查。
    SetHashability,
    /// 动态集合成员判断的类型/容器检查。
    SetMembership,
    /// 动态集合代数操作数、结果或形状检查。
    SetOperation,
    /// 动态集合比较的成员/关系检查。
    SetComparison,
    /// 动态值作为 `if`/`while` 条件的布尔检查。
    BooleanCondition,
    /// 动态值参与 `for in` 迭代时的可迭代性检查。
    Iterable,
}

impl RuntimeCheckKind {
    /// 返回运行时检查的稳定名称。
    #[must_use]
    pub const fn as_name(self) -> &'static str {
        match self {
            Self::NumericRange => "numeric_range",
            Self::StringBoolean => "string_boolean",
            Self::DynamicConversion => "dynamic_conversion",
            Self::Arithmetic => "arithmetic",
            Self::SelectorBounds => "selector_bounds",
            Self::SelectorStep => "selector_step",
            Self::RandomCount => "random_count",
            Self::RandomSeed => "random_seed",
            Self::SetHashability => "set_hashability",
            Self::SetMembership => "set_membership",
            Self::SetOperation => "set_operation",
            Self::SetComparison => "set_comparison",
            Self::BooleanCondition => "boolean_condition",
            Self::Iterable => "iterable",
        }
    }

    /// 将降低器携带的稳定名称还原为检查类别。
    ///
    /// 检查名称由类型层统一维护，IR 和字节码层都消费这一入口，避免两层
    /// 各自维护一份容易漂移的白名单。
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "numeric_range" => Self::NumericRange,
            "string_boolean" => Self::StringBoolean,
            "dynamic_conversion" => Self::DynamicConversion,
            "arithmetic" => Self::Arithmetic,
            "selector_bounds" => Self::SelectorBounds,
            "selector_step" => Self::SelectorStep,
            "random_count" => Self::RandomCount,
            "random_seed" => Self::RandomSeed,
            "set_hashability" => Self::SetHashability,
            "set_membership" => Self::SetMembership,
            "set_operation" => Self::SetOperation,
            "set_comparison" => Self::SetComparison,
            "boolean_condition" => Self::BooleanCondition,
            "iterable" => Self::Iterable,
            _ => return None,
        })
    }

    /// 返回首版全部可降低检查类别。
    ///
    /// 数组用于穷尽性测试和跨层桥接；新增枚举变体时必须同步更新这里，
    /// 从而让类型层测试在同一批次暴露接线缺口。
    #[must_use]
    pub const fn all() -> [Self; 14] {
        [
            Self::NumericRange,
            Self::StringBoolean,
            Self::DynamicConversion,
            Self::Arithmetic,
            Self::SelectorBounds,
            Self::SelectorStep,
            Self::RandomCount,
            Self::RandomSeed,
            Self::SetHashability,
            Self::SetMembership,
            Self::SetOperation,
            Self::SetComparison,
            Self::BooleanCondition,
            Self::Iterable,
        ]
    }
}

/// 一个带源码区间的运行时检查标记。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuntimeCheck {
    /// 需要插入检查的源码区间。
    pub span: SourceSpan,
    /// 检查种类。
    pub kind: RuntimeCheckKind,
    /// `set_membership` 的可选声明成员类型；其他检查保持为空。
    pub expected: Option<Type>,
}

/// 一个类型化 AST 节点的旁路记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedNode {
    /// 节点源码区间。
    pub span: SourceSpan,
    /// 节点推断/检查后的类型。
    pub ty: Type,
}

/// 一次完整 P2 类型检查的结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeCheckResult {
    /// 按访问顺序保存的类型化节点，不改写原始 AST。
    pub nodes: Vec<TypedNode>,
    /// 累积的类型诊断。
    pub diagnostics: Vec<Diagnostic>,
    /// 需要由 Runtime 或后端执行的动态检查。
    pub runtime_checks: Vec<RuntimeCheck>,
    /// 检查结束时的环境快照，供后续 IR 阶段消费。
    pub environment: TypeEnvironment,
    /// 空数组路径声明对应的静态物化计划。
    pub materialization_plans: Vec<ContainerMaterializationPlan>,
    /// 有序容器选择器的规范化计划。
    pub selection_plans: Vec<crate::selection_model::SelectionPlan>,
    /// 选择器左值的事务性标量广播计划。
    pub broadcast_assignment_plans: Vec<crate::selection_model::BroadcastAssignmentPlan>,
    /// `random.seed` 调用的运行上下文种子计划。
    pub random_seed_plans: Vec<crate::selection_model::RandomSeedPlan>,
    /// 顶层程序入口模式。
    pub entry_mode: EntryMode,
    /// 已登记并完成推断的函数签名。
    pub function_signatures: BTreeMap<String, FunctionSignature>,
    /// 已登记并完成检查的表签名。
    pub table_signatures: BTreeMap<String, crate::tables::TableSignature>,
}

impl TypeCheckResult {
    /// 判断是否包含错误诊断。
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// 判断检查是否成功且没有错误。
    #[must_use]
    pub fn is_success(&self) -> bool {
        !self.has_errors()
    }

    /// 返回与源码区间完全匹配的最后一个类型记录。
    #[must_use]
    pub fn type_at(&self, span: SourceSpan) -> Option<&Type> {
        self.nodes
            .iter()
            .rev()
            .find(|node| node.span == span)
            .map(|node| &node.ty)
    }

    /// 返回某个规范化名称的最终绑定。
    #[must_use]
    pub fn binding(&self, name: &str) -> Option<&Binding> {
        self.environment
            .lookup(name)
            .or_else(|| self.environment.lookup(&format!("ascii:{name}")))
            .or_else(|| self.environment.lookup(&format!("backtick:{name}")))
    }

    /// 返回类型记录的只读视图。
    #[must_use]
    pub fn nodes(&self) -> &[TypedNode] {
        &self.nodes
    }

    /// 返回运行时检查标记的只读视图。
    #[must_use]
    pub fn runtime_checks(&self) -> &[RuntimeCheck] {
        &self.runtime_checks
    }

    /// 返回诊断的只读视图。
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// 返回容器默认值/形状计划的只读视图。
    #[must_use]
    pub fn materialization_plans(&self) -> &[ContainerMaterializationPlan] {
        &self.materialization_plans
    }

    /// 返回有序容器选择计划的只读视图。
    #[must_use]
    pub fn selection_plans(&self) -> &[crate::selection_model::SelectionPlan] {
        &self.selection_plans
    }

    /// 返回选择器广播赋值计划的只读视图。
    #[must_use]
    pub fn broadcast_assignment_plans(&self) -> &[crate::selection_model::BroadcastAssignmentPlan] {
        &self.broadcast_assignment_plans
    }

    /// 返回 `random.seed` 计划的只读视图。
    #[must_use]
    pub fn random_seed_plans(&self) -> &[crate::selection_model::RandomSeedPlan] {
        &self.random_seed_plans
    }

    /// 返回程序入口模式。
    #[must_use]
    pub const fn entry_mode(&self) -> EntryMode {
        self.entry_mode
    }

    /// 返回函数签名表的只读视图。
    #[must_use]
    pub fn function_signatures(&self) -> &BTreeMap<String, FunctionSignature> {
        &self.function_signatures
    }

    /// 返回表签名表的只读视图。
    #[must_use]
    pub fn table_signatures(&self) -> &BTreeMap<String, crate::tables::TableSignature> {
        &self.table_signatures
    }
}
