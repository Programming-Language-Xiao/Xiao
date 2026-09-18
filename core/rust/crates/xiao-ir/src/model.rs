//! 类型化 Xiao IR 的递归值对象。
//!
//! 本模块只保存已经通过前端分析的语义信息。IR 不持有宿主指针、不执行用户
//! 代码，也不依赖 CLI 或 Runtime 的内部布局。所有源码位置都转换为可序列化
//! 的字节区间，便于跨平台快照和后端消费。

use serde::{Deserialize, Serialize};

/// 当前类型化 IR 的格式版本。
pub const IR_VERSION: u32 = 1;

/// Xiao 源码中的稳定半开字节区间。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct IrSpan {
    /// 起始字节偏移。
    pub start: usize,
    /// 结束字节偏移（不包含）。
    pub end: usize,
}

impl IrSpan {
    /// 创建一个 IR 源码区间。
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// 前端输出的类型化程序根节点。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrProgram {
    /// IR 格式版本。
    pub version: u32,
    /// 语言版本字符串。
    pub language_version: String,
    /// 目标平台描述；前端阶段不解释其具体 ABI。
    pub target: String,
    /// 是否有经过配置层校验的 `config.xiao`。
    pub config_present: bool,
    /// 外部包图中的稳定包身份摘要。
    pub external_packages: Vec<String>,
    /// 程序入口模式。
    pub entry_mode: IrEntryMode,
    /// 主入口模块的递归语句树。
    pub body: Vec<IrStatement>,
    /// 项目中可用的文件模块和命名空间摘要。
    pub modules: Vec<IrModule>,
    /// 生命周期阶段产生的控制流图。
    pub control_flow: IrControlFlow,
    /// 生命周期、所有权和释放计划。
    pub ownership: IrOwnership,
    /// 前端登记的运行时检查。
    pub runtime_checks: Vec<IrRuntimeCheck>,
    /// 类型阶段生成的规范化选择计划。
    ///
    /// 选择表达式通过 `IrExpressionKind::Selector::selection_plan` 引用此表；
    /// 后端只能消费这里的镜像，不能重新解析选择器源码。
    pub selection_plans: Vec<IrSelectionPlan>,
    /// 类型阶段生成的事务性广播写入计划。
    pub broadcast_assignment_plans: Vec<IrBroadcastAssignmentPlan>,
    /// `random.seed` 的类型阶段计划。
    pub random_seed_plans: Vec<IrRandomSeedPlan>,
    /// 入口源码区间；脚本模式使用程序区间。
    pub span: IrSpan,
}

impl IrProgram {
    /// 创建一个指定入口和主体的 IR 程序。
    #[must_use]
    pub fn new(entry_mode: IrEntryMode, body: Vec<IrStatement>, span: IrSpan) -> Self {
        Self {
            version: IR_VERSION,
            language_version: "0.1.0".to_owned(),
            target: "host".to_owned(),
            config_present: false,
            external_packages: Vec::new(),
            entry_mode,
            body,
            modules: Vec::new(),
            control_flow: IrControlFlow::default(),
            ownership: IrOwnership::default(),
            runtime_checks: Vec::new(),
            selection_plans: Vec::new(),
            broadcast_assignment_plans: Vec::new(),
            random_seed_plans: Vec::new(),
            span,
        }
    }

    /// 判断程序是否包含任何语句。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.body.is_empty() && self.modules.is_empty()
    }

    /// 使用统一验证器检查程序不变量。
    #[must_use]
    pub fn validate(&self) -> crate::IrValidationResult {
        crate::IrValidator::new().validate(self)
    }

    /// 将程序编码为稳定 JSON 快照。
    pub fn to_json(&self) -> Result<String, crate::SnapshotError> {
        crate::to_json(self)
    }
}

/// 程序入口模式。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum IrEntryMode {
    /// 顶层语句组成脚本入口。
    Script,
    /// 显式 `[main]` 入口。
    Project {
        /// `[main]` 表头位置。
        span: IrSpan,
    },
}

/// 一个文件模块或目录命名空间的 IR 摘要。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrModule {
    /// 逻辑模块名称。
    pub name: String,
    /// `file` 或 `namespace`。
    pub kind: String,
    /// 规范化文件系统路径。
    pub path: String,
    /// 文件模块的语句；命名空间为空。
    pub body: Vec<IrStatement>,
    /// 可供导入的符号摘要。
    pub symbols: Vec<IrSymbol>,
}

/// 模块导出符号摘要。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrSymbol {
    /// 符号名称。
    pub name: String,
    /// 符号类别。
    pub kind: String,
    /// `local` 或 `reexport`。
    pub origin: String,
    /// 声明位置。
    pub span: IrSpan,
}

/// 一个带源码位置和类型化表达式的 IR 语句。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrStatement {
    /// 语句的具体形态。
    pub kind: IrStatementKind,
    /// 语句源码区间。
    pub span: IrSpan,
    /// 与语句关联的文档注释区间。
    pub leading_docs: Vec<IrSpan>,
}

/// IR 语句形态。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum IrStatementKind {
    /// 独立表达式。
    Expression {
        /// 表达式。
        value: IrExpression,
    },
    /// 简单名称赋值。
    Assignment {
        /// 目标名称。
        target: IrName,
        /// 赋值表达式。
        value: IrExpression,
    },
    /// 复合或选择器赋值。
    ExtendedAssignment {
        /// 左值表达式。
        target: IrExpression,
        /// 赋值运算符的稳定拼写。
        operator: String,
        /// 右值表达式。
        value: IrExpression,
    },
    /// 带类型约束的声明。
    Declaration {
        /// 绑定名称。
        target: IrName,
        /// 显式类型；没有时为动态。
        declared_type: IrType,
        /// 可选路径约束。
        constraint_path: Option<IrPath>,
        /// 可选初始化值。
        value: Option<IrExpression>,
    },
    /// 编译期常量声明。
    ConstDeclaration {
        /// 常量名称。
        target: IrName,
        /// 可选显式标量类型。
        declared_type: Option<String>,
        /// 初始化表达式。
        value: IrExpression,
    },
    /// 导入语句摘要。
    Import {
        /// 导入的稳定描述。
        description: String,
    },
    /// 表声明。
    Table {
        /// 表名称。
        name: IrName,
        /// `singleton` 或 `instance`。
        table_kind: String,
        /// 表体。
        body: Vec<IrStatement>,
    },
    /// 函数定义。
    Function {
        /// 函数名称。
        name: IrName,
        /// 参数列表。
        parameters: Vec<IrParameter>,
        /// 返回类型。
        return_type: IrType,
        /// 函数体。
        body: Vec<IrStatement>,
    },
    /// 条件分支。
    If {
        /// 首个条件。
        condition: IrExpression,
        /// 首个分支体。
        body: Vec<IrStatement>,
        /// 后续 `elif` 分支。
        elif_branches: Vec<IrElifBranch>,
        /// 可选 `else` 体。
        else_body: Option<Vec<IrStatement>>,
    },
    /// `for` 循环。
    For {
        /// 循环变量。
        target: IrName,
        /// 可迭代表达式。
        iterable: IrExpression,
        /// 循环体。
        body: Vec<IrStatement>,
    },
    /// `while` 循环。
    While {
        /// 条件表达式。
        condition: IrExpression,
        /// 循环体。
        body: Vec<IrStatement>,
    },
    /// 返回语句。
    Return {
        /// 可选返回值。
        value: Option<IrExpression>,
    },
    /// `break`。
    Break,
    /// `continue`。
    Continue,
    /// 错误控制流。
    Try {
        /// 受保护主体。
        body: Vec<IrStatement>,
        /// 错误处理器。
        catches: Vec<IrCatchClause>,
        /// 最终清理体。
        finally_body: Option<Vec<IrStatement>>,
    },
    /// 抛出可恢复错误。
    Raise {
        /// 要抛出的错误表达式。
        value: IrExpression,
    },
}

/// `elif` 分支。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrElifBranch {
    /// 条件表达式。
    pub condition: IrExpression,
    /// 分支体。
    pub body: Vec<IrStatement>,
    /// 分支源码区间。
    pub span: IrSpan,
}

/// `catch` 处理器。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrCatchClause {
    /// 错误绑定名称。
    pub binding: IrName,
    /// 错误类型名称。
    pub error_type: IrName,
    /// 处理器体。
    pub body: Vec<IrStatement>,
    /// 处理器源码区间。
    pub span: IrSpan,
}

/// 函数参数。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrParameter {
    /// 参数名称。
    pub name: IrName,
    /// 参数种类稳定拼写。
    pub kind: String,
    /// 参数类型。
    pub ty: IrType,
    /// 默认值。
    pub default: Option<IrExpression>,
    /// 参数源码区间。
    pub span: IrSpan,
}

/// 类型化表达式。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrExpression {
    /// 表达式的类型化形态。
    pub kind: IrExpressionKind,
    /// 静态推断类型。
    pub ty: IrType,
    /// 表达式源码区间。
    pub span: IrSpan,
}

/// IR 表达式形态。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum IrExpressionKind {
    /// 字面量及其原始文本。
    Literal {
        /// 规范化后的字面量类别。
        literal: String,
        /// 源码原始文本。
        text: String,
    },
    /// 名称引用。
    Name {
        /// 被引用的名称。
        name: IrName,
    },
    /// 数组字面量。
    Array {
        /// 数组元素。
        elements: Vec<IrExpression>,
    },
    /// 元组字面量。
    Tuple {
        /// 元组元素。
        elements: Vec<IrExpression>,
    },
    /// 字典表字面量。
    DictTable {
        /// 字典条目。
        entries: Vec<IrDictEntry>,
    },
    /// 集合字面量。
    Set {
        /// 集合元素。
        elements: Vec<IrExpression>,
    },
    /// 字典列字面量。
    DictColumn {
        /// 字典列条目。
        entries: Vec<IrDictEntry>,
    },
    /// 括号分组。
    Group {
        /// 被分组的表达式。
        expression: Box<IrExpression>,
    },
    /// 一元运算。
    Unary {
        /// 运算符。
        operator: String,
        /// 操作数。
        operand: Box<IrExpression>,
    },
    /// 二元运算。
    Binary {
        /// 运算符。
        operator: String,
        /// 左操作数。
        left: Box<IrExpression>,
        /// 右操作数。
        right: Box<IrExpression>,
    },
    /// 普通调用。
    Call {
        /// 被调用对象。
        callee: Box<IrExpression>,
        /// 参数。
        arguments: Vec<IrCallArgument>,
    },
    /// `new` 构造调用。
    NewCall {
        /// 构造目标。
        callee: Box<IrExpression>,
        /// 参数。
        arguments: Vec<IrCallArgument>,
    },
    /// 成员访问。
    Member {
        /// 对象。
        object: Box<IrExpression>,
        /// 成员名称。
        member: IrName,
    },
    /// 显式转换。
    Cast {
        /// 源表达式。
        expression: Box<IrExpression>,
        /// 目标类型稳定拼写。
        target: String,
    },
    /// 统一选择器。
    Selector {
        /// 来源表达式。
        source: Box<IrExpression>,
        /// 选择器描述。
        selector: IrSelector,
        /// 可选步长。
        step: Option<Box<IrExpression>>,
        /// 规范化选择计划表中的索引。
        selection_plan: Option<u32>,
    },
}

/// 调用参数。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrCallArgument {
    /// 参数类别稳定拼写。
    pub kind: String,
    /// 可选关键字名称。
    pub name: Option<IrName>,
    /// 参数值。
    pub value: IrExpression,
    /// 参数源码区间。
    pub span: IrSpan,
}

/// 字典条目。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrDictEntry {
    /// 键的稳定文本。
    pub key: String,
    /// 值表达式。
    pub value: IrExpression,
    /// 条目源码区间。
    pub span: IrSpan,
}

/// 名称及其反引号标记。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrName {
    /// 名称文本；反引号只由 `backticked` 字段表示。
    pub text: String,
    /// 是否使用反引号名称。
    pub backticked: bool,
    /// 名称源码区间。
    pub span: IrSpan,
}

/// 路径段。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrPathSegment {
    /// 段的稳定类别和值。
    pub kind: IrPathSegmentKind,
    /// 段源码区间。
    pub span: IrSpan,
}

/// 路径段类别。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum IrPathSegmentKind {
    /// 数字索引。
    Index {
        /// 原始数字文本。
        text: String,
        /// 是否为负数。
        negative: bool,
    },
    /// 键名路径段。
    Name {
        /// 路径中的名称。
        name: IrName,
    },
}

/// 类型约束或选择器使用的路径。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrPath {
    /// 路径段。
    pub segments: Vec<IrPathSegment>,
    /// 路径源码区间。
    pub span: IrSpan,
}

/// 高级选择器。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrSelector {
    /// 选择项。
    pub items: Vec<IrSelectorItem>,
    /// 选择器源码区间。
    pub span: IrSpan,
}

/// 选择器项。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum IrSelectorItem {
    /// 精确路径。
    Exact {
        /// 被选中的精确路径。
        path: IrPath,
        /// 项源码区间。
        span: IrSpan,
    },
    /// 闭区间。
    Range {
        /// 起点。
        start: IrPath,
        /// 终点。
        end: IrPath,
        /// 项位置。
        span: IrSpan,
    },
    /// 单边范围。
    OpenRange {
        /// 起点。
        start: Option<IrPath>,
        /// 终点。
        end: Option<IrPath>,
        /// 是否包含起点。
        include_start: bool,
        /// 是否包含终点。
        include_end: bool,
        /// 项位置。
        span: IrSpan,
    },
    /// 全选。
    All {
        /// 项源码区间。
        span: IrSpan,
    },
    /// 随机选择。
    Random {
        /// 随机模式稳定拼写：`without_replacement` 或 `with_replacement`。
        ///
        /// 这里存的是**语义拼写**，不是源码标点 `?` / `!?`；源码拼写只属于语法层。
        mode: String,
        /// 数量。
        count: Box<IrExpression>,
        /// 项位置。
        span: IrSpan,
    },
}

/// 类型化 IR 类型。
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum IrType {
    /// 标量类型。
    Scalar {
        /// 标量稳定名称。
        name: String,
    },
    /// `none`。
    None,
    /// HM 变量。
    Variable {
        /// 类型变量编号。
        id: u32,
    },
    /// 函数类型。
    Function {
        /// 参数类型。
        parameters: Vec<IrType>,
        /// 返回类型。
        return_type: Box<IrType>,
    },
    /// 数组类型。
    Array {
        /// 数组形状。
        shape: IrArrayShape,
    },
    /// 元组类型。
    Tuple {
        /// 元素类型。
        elements: Vec<IrType>,
    },
    /// 字典表。
    DictTable {
        /// 字典条目类型。
        entries: Vec<IrDictTypeEntry>,
    },
    /// 字典列。
    DictColumn {
        /// 字典列条目类型。
        entries: Vec<IrDictTypeEntry>,
    },
    /// 集合类型。
    Set {
        /// 成员类型。
        members: Vec<IrType>,
        /// 是否允许动态成员。
        allows_dynamic: bool,
        /// 是否静态为空。
        empty: bool,
        /// 是否未知。
        unknown: bool,
    },
    /// 表类型。
    Table {
        /// 表名称。
        name: String,
        /// 表种类稳定名称。
        kind: String,
    },
    /// 动态类型。
    Dynamic,
}

/// 数组形状。
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum IrArrayShape {
    /// 同构数组。
    Homogeneous {
        /// 元素类型。
        element: Box<IrType>,
        /// 可选的静态长度。
        length: Option<usize>,
    },
    /// 异构数组。
    Heterogeneous {
        /// 每个位置的元素类型。
        elements: Vec<IrType>,
    },
    /// 未知数组。
    Unknown,
}

/// 字典类型条目。
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct IrDictTypeEntry {
    /// 键。
    pub key: String,
    /// 值类型。
    pub value: Box<IrType>,
}

/// 一个运行时检查计划。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrRuntimeCheck {
    /// 检查类别。
    pub kind: String,
    /// 检查源码区间。
    pub span: IrSpan,
}

/// 选择路径中的规范化段。
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum IrSelectionPathSegment {
    /// 数字索引；`raw` 保留负索引语义，`resolved` 是静态位置。
    Index {
        /// 源码中的有符号索引。
        raw: i128,
        /// 已知长度下解析出的零基位置。
        resolved: Option<usize>,
    },
    /// 字典键。
    Key(String),
}

/// 规范化选择路径。
pub type IrSelectionPath = Vec<IrSelectionPathSegment>;

/// 一个规范化选择项。
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum IrSelectionItemPlan {
    /// 精确路径。
    Exact {
        /// 被选择的路径。
        path: IrSelectionPath,
    },
    /// 范围路径。
    Range {
        /// 起点路径。
        start: IrSelectionPath,
        /// 终点路径。
        end: IrSelectionPath,
        /// 是否包含起点。
        include_start: bool,
        /// 是否包含终点。
        include_end: bool,
    },
    /// 全选。
    All,
    /// 随机选择。
    Random {
        /// `without_replacement` 或 `with_replacement`。
        mode: String,
        /// 静态数量。
        count: Option<usize>,
        /// 数量是否动态。
        dynamic_count: bool,
    },
}

/// 规范化步长。
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct IrStepPlan {
    /// 静态步长；动态表达式为 `None`。
    pub value: Option<i128>,
    /// 是否需要运行时求值。
    pub dynamic: bool,
}

/// 类型阶段选择计划的可序列化镜像。
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct IrSelectionPlan {
    /// 选择表达式源码区间。
    pub span: IrSpan,
    /// 来源类型。
    pub source_type: IrType,
    /// 结果类型。
    pub result_type: IrType,
    /// 按源码顺序排列的选择项。
    pub items: Vec<IrSelectionItemPlan>,
    /// 静态展开路径。
    pub selected_paths: Vec<IrSelectionPath>,
    /// 静态目标叶子类型。
    pub target_types: Vec<IrType>,
    /// 可选步长。
    pub step: Option<IrStepPlan>,
    /// 是否含运行时边界。
    pub requires_runtime_check: bool,
    /// 是否有放回随机项。
    pub with_replacement: bool,
    /// 是否有重复目标。
    pub has_duplicates: bool,
}

impl IrSelectionPlan {
    /// 判断该计划是否可以使用精确 `IndexGet`。
    #[must_use]
    pub fn is_single_value(&self) -> bool {
        self.items.len() == 1
            && self.selected_paths.len() == 1
            && matches!(self.items.first(), Some(IrSelectionItemPlan::Exact { .. }))
            && !self.requires_runtime_check
    }
}

/// 事务性广播赋值计划的可序列化镜像。
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct IrBroadcastAssignmentPlan {
    /// 赋值源码区间。
    pub span: IrSpan,
    /// 被写入的根绑定。
    pub root_name: Option<String>,
    /// 静态目标路径。
    pub target_paths: Vec<IrSelectionPath>,
    /// 右值类型。
    pub value_type: IrType,
    /// 是否需要运行时检查。
    pub dynamic: bool,
    /// 是否要求事务性提交。
    pub transactional: bool,
}

/// `random.seed` 计划的可序列化镜像。
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct IrRandomSeedPlan {
    /// 调用源码区间。
    pub span: IrSpan,
    /// 静态种子。
    pub value: Option<u128>,
    /// 是否动态求值。
    pub dynamic: bool,
}

/// 生命周期控制流图。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrControlFlow {
    /// 图入口块编号。
    pub entry: Option<u32>,
    /// 基本块，按编号排序。
    pub blocks: Vec<IrBasicBlock>,
}

/// 控制流基本块。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrBasicBlock {
    /// 基本块编号。
    pub id: u32,
    /// 所属作用域编号。
    pub scope: u32,
    /// 块源码区间。
    pub span: Option<IrSpan>,
    /// 语句源码区间。
    pub statements: Vec<IrSpan>,
    /// 后继块及边类别。
    pub successors: Vec<IrControlFlowSuccessor>,
    /// 作用域退出类别。
    pub exits: Vec<String>,
}

/// 控制流后继。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrControlFlowSuccessor {
    /// 目标块。
    pub target: u32,
    /// 边类别。
    pub kind: String,
}

/// 所有权和释放计划摘要。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrOwnership {
    /// 作用域。
    pub scopes: Vec<IrScope>,
    /// 值槽。
    pub values: Vec<IrValue>,
    /// 强拥有边。
    pub strong_edges: Vec<IrOwnershipEdge>,
    /// 弱引用边。
    pub weak_edges: Vec<IrOwnershipEdge>,
    /// 释放计划。
    pub release_plans: Vec<IrReleasePlan>,
    /// 动态生命周期检查。
    pub dynamic_checks: Vec<IrDynamicLifetimeCheck>,
}

/// 作用域摘要。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrScope {
    /// 作用域编号。
    pub id: u32,
    /// 父作用域编号。
    pub parent: Option<u32>,
    /// 作用域类别。
    pub kind: String,
    /// 源码区间。
    pub span: IrSpan,
    /// 嵌套深度。
    pub depth: usize,
    /// 值槽编号。
    pub values: Vec<u32>,
}

/// 值槽摘要。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrValue {
    /// 值编号。
    pub id: u32,
    /// 可选名称。
    pub name: Option<String>,
    /// 所属作用域。
    pub scope: u32,
    /// 源码区间。
    pub span: IrSpan,
    /// 静态类型。
    pub ty: Option<IrType>,
    /// 存储类别。
    pub storage: String,
    /// 声明序号。
    pub declaration_order: usize,
    /// 是否参数。
    pub parameter: bool,
    /// 是否常量。
    pub constant: bool,
    /// 是否临时值。
    pub temporary: bool,
    /// 逃逸原因。
    pub escapes: Vec<String>,
}

/// 所有权边。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrOwnershipEdge {
    /// 来源值编号。
    pub from: u32,
    /// 目标值编号。
    pub to: u32,
    /// 边类别。
    pub kind: String,
    /// 边来源。
    pub reason: String,
    /// 源码区间。
    pub span: Option<IrSpan>,
}

/// 释放动作。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrReleaseAction {
    /// 值编号。
    pub value: u32,
    /// 释放顺序。
    pub order: usize,
    /// 强/弱释放。
    pub kind: String,
}

/// 作用域退出释放计划。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrReleasePlan {
    /// 作用域编号。
    pub scope: u32,
    /// 退出类别。
    pub exit: String,
    /// 动作序列。
    pub actions: Vec<IrReleaseAction>,
    /// 转移出去的值。
    pub transferred: Vec<u32>,
}

/// 动态生命周期检查。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrDynamicLifetimeCheck {
    /// 源码区间。
    pub span: IrSpan,
    /// 可选值编号。
    pub value: Option<u32>,
    /// 保守策略原因。
    pub reason: String,
}
