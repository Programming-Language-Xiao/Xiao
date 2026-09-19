//! 统一三地址语义模型。
//!
//! 本模块是 09R 研究工程的一部分，**不是稳定语言接口**。它只描述「算什么」，
//! 把「值放在哪里、怎么编码」完全留给机型。三条冻结的边界：
//!
//! - 指令不记录寄存器类别，类别另由 [`CategoryMap`] 携带，因为类别归属是机型
//!   决策，写进指令会让三种候选机型不再是同一输入的可比实现。
//! - 装箱与拆箱**在这里决定位置**并显式发出，机型只决定怎么做，这样三机型的
//!   `Box`/`Unbox` 序列可以逐条对照。
//! - 释放序列按冻结计划的 `order` 逐条展开，这里不允许重排、去重或下沉。

use xiao_ir::IrSpan;
use xiao_lifetime::ReleaseActionKind;
use xiao_syntax::ScalarType;

/// 研究用三地址格式的版本。
pub const TAC_VERSION: u32 = 1;

/// 虚拟寄存器编号。
///
/// 编号空间与 `IrValue.id` **互相独立**：标量临时值在生命周期结果里没有条目，
/// 却同样需要寄存器。需要与释放计划对齐的值另由降低器维护的双向映射恢复。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VReg(u32);

impl VReg {
    /// 从原始编号创建虚拟寄存器。
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回原始编号。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// 常量池索引。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConstId(u32);

impl ConstId {
    /// 从原始编号创建常量索引。
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回原始编号。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// 函数索引。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FuncId(u32);

impl FuncId {
    /// 从原始编号创建函数索引。
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回原始编号。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// 基本块索引。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BlockId(u32);

impl BlockId {
    /// 从原始编号创建基本块索引。
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回原始编号。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// 调用签名索引。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SigId(u32);

impl SigId {
    /// 从原始编号创建签名索引。
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回原始编号。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// 寄存器类别。
///
/// `None` 是零宽值，不分配寄存器文件；`Poly` 是控制流合流点无法收敛时的退化
/// 类别，必须落到帧槽而不能只活在易失寄存器里。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RegisterClass {
    /// 固定宽度整数。
    Int,
    /// 固定宽度浮点。
    Float,
    /// 布尔值，独立于整数。
    Bool,
    /// 对象句柄，包括 `str`、容器、表实例和运行时数值对象。
    ObjHandle,
    /// 带运行时类型标签的动态值。
    Dynamic,
    /// 零宽的 `none`。
    None,
    /// 合流点无法收敛的退化类别。
    Poly,
}

impl RegisterClass {
    /// 返回稳定名称。
    #[must_use]
    pub const fn as_name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::ObjHandle => "obj",
            Self::Dynamic => "dynamic",
            Self::None => "none",
            Self::Poly => "poly",
        }
    }
}

/// 虚拟寄存器到类别的映射。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CategoryMap {
    /// 按寄存器编号稠密存放的类别。
    classes: Vec<RegisterClass>,
}

impl CategoryMap {
    /// 创建空映射。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            classes: Vec::new(),
        }
    }

    /// 登记一个寄存器的类别；同一寄存器被重复登记时取保守合并。
    pub fn insert(&mut self, register: VReg, class: RegisterClass) {
        let index = register.get() as usize;
        if self.classes.len() <= index {
            self.classes.resize(index + 1, RegisterClass::Poly);
        }
        self.classes[index] = merge_class(self.classes[index], class);
    }

    /// 查询寄存器类别；未登记时返回 `Poly`。
    #[must_use]
    pub fn get(&self, register: VReg) -> RegisterClass {
        self.classes
            .get(register.get() as usize)
            .copied()
            .unwrap_or(RegisterClass::Poly)
    }

    /// 返回已登记的寄存器数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.classes.len()
    }

    /// 判断映射为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    /// 按虚拟寄存器编号返回已登记类别。
    pub fn iter(&self) -> impl ExactSizeIterator<Item = RegisterClass> + '_ {
        self.classes.iter().copied()
    }

    /// 从按虚拟寄存器编号排列的类别重建映射。
    pub(crate) fn from_classes(classes: Vec<RegisterClass>) -> Self {
        Self { classes }
    }
}

/// 合并两个类别的保守结果；不同类别在合流点退化为 `Poly`。
const fn merge_class(left: RegisterClass, right: RegisterClass) -> RegisterClass {
    if matches!(left, RegisterClass::Poly) {
        return right;
    }
    if matches!(right, RegisterClass::Poly) {
        return left;
    }
    if matches!(
        (left, right),
        (RegisterClass::Int, RegisterClass::Int)
            | (RegisterClass::Float, RegisterClass::Float)
            | (RegisterClass::Bool, RegisterClass::Bool)
            | (RegisterClass::ObjHandle, RegisterClass::ObjHandle)
            | (RegisterClass::Dynamic, RegisterClass::Dynamic)
            | (RegisterClass::None, RegisterClass::None)
    ) {
        left
    } else {
        RegisterClass::Poly
    }
}

/// 常量池中的一个值。
#[derive(Clone, Debug, PartialEq)]
pub enum TacConstant {
    /// 默认 64 位整数。
    Int(i64),
    /// 32 位整数。
    Sint(i32),
    /// 可扩展宽度整数的规范十进制文本。
    Lint(String),
    /// 默认 64 位浮点。
    Float(f64),
    /// 32 位浮点。
    Sfloat(f32),
    /// 可扩展精度浮点的规范文本。
    Lfloat(String),
    /// 布尔值。
    Bool(bool),
    /// 字符串内容。
    Str(String),
}

impl TacConstant {
    /// 返回常量对应的寄存器类别。
    #[must_use]
    pub const fn register_class(&self) -> RegisterClass {
        match self {
            Self::Int(_) | Self::Sint(_) => RegisterClass::Int,
            Self::Float(_) | Self::Sfloat(_) => RegisterClass::Float,
            Self::Lint(_) | Self::Lfloat(_) => RegisterClass::ObjHandle,
            Self::Bool(_) => RegisterClass::Bool,
            Self::Str(_) => RegisterClass::ObjHandle,
        }
    }
}

/// 常量池。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstPool {
    entries: Vec<TacConstant>,
}

impl ConstPool {
    /// 创建空常量池。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 登记一个常量并返回其索引；相同常量复用同一索引。
    pub fn intern(&mut self, constant: TacConstant) -> ConstId {
        if let Some(index) = self.entries.iter().position(|item| *item == constant) {
            return ConstId::new(index as u32);
        }
        self.entries.push(constant);
        ConstId::new((self.entries.len() - 1) as u32)
    }

    /// 按键取值。
    #[must_use]
    pub fn get(&self, id: ConstId) -> Option<&TacConstant> {
        self.entries.get(id.get() as usize)
    }

    /// 返回常量数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 判断常量池为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 按常量索引顺序遍历全部条目。
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &TacConstant> {
        self.entries.iter()
    }

    /// 从保持原始索引顺序的条目重建常量池。
    pub(crate) fn from_entries(entries: Vec<TacConstant>) -> Self {
        Self { entries }
    }
}

/// 算术运算类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArithOp {
    /// 加法与字符串拼接。
    Add,
    /// 减法与布尔整数调整。
    Subtract,
    /// 乘法。
    Multiply,
    /// 浮点除法。
    Divide,
    /// 向下取整的整数除法。
    FloorDivide,
    /// 与整除配对的取模。
    Remainder,
    /// 幂运算。
    Power,
}

/// 比较运算类别，结果恒为布尔。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompareOp {
    /// `<`。
    Less,
    /// `<=`。
    LessEqual,
    /// `>`。
    Greater,
    /// `>=`。
    GreaterEqual,
    /// `==`。
    Equal,
    /// `!=`。
    NotEqual,
}

/// 集合代数运算类别，结果仍是集合。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SetOpKind {
    /// 并集。
    Union,
    /// 交集。
    Intersection,
    /// 差集：左操作数独有。
    Difference,
    /// 对称差。
    SymmetricDifference,
}

/// 集合关系比较类别，结果恒为布尔。
///
/// 变体按「左操作数相对右操作数」的方向命名：`Subset` 表示左是右的子集。
/// 成员判定用 [`Self::Member`] 而不是 `Contains`，因为后者读起来是「左包含右」，
/// 而 `x in s` 里左操作数是被包含的一方，方向正好相反。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SetCompareOp {
    /// `==`：无序双向包含，不是句柄身份也不是元素序列逐位相等。
    Equal,
    /// `!=`。
    NotEqual,
    /// `<`：真子集，要求包含且不相等。
    ProperSubset,
    /// `<=`：子集，允许两边相等。
    Subset,
    /// `>`：真超集。
    ProperSuperset,
    /// `>=`：超集，允许两边相等。
    Superset,
    /// `in`：左操作数是右操作数的成员。
    Member,
    /// `not in`。
    NotMember,
}

/// 调用实参的类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArgKind {
    /// 位置实参。
    Positional,
    /// 关键字实参。
    Keyword,
    /// `*` 展开。
    VarArgs,
    /// `**` 展开。
    KwArgs,
}

/// 一条精确索引路径段。
///
/// 数字索引保留源码的有符号语义，负值由运行时按容器长度归一化；解析只发生在
/// 降低期，运行时不重新解释源码文本。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathStep {
    /// 有符号数字索引。
    Index(i128),
    /// 字典键。
    Key(String),
}

/// 一条调用实参。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TacArgument {
    /// 实参类别。
    pub kind: ArgKind,
    /// 关键字实参的名称。
    pub name: Option<String>,
    /// 实参寄存器。
    pub value: VReg,
}

impl TacArgument {
    /// 创建一条位置实参。
    #[must_use]
    pub const fn positional(value: VReg) -> Self {
        Self {
            kind: ArgKind::Positional,
            name: None,
            value,
        }
    }

    /// 创建一条关键字实参。
    #[must_use]
    pub fn keyword(name: impl Into<String>, value: VReg) -> Self {
        Self {
            kind: ArgKind::Keyword,
            name: Some(name.into()),
            value,
        }
    }
}

/// 一条三地址指令的操作。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TacOp {
    /// 从常量池加载。
    LoadConst(ConstId),
    /// 加载零宽的 `none`。
    LoadNone,
    /// 加载函数引用。
    LoadFunc(FuncId),
    /// 转移所有权：源寄存器随之失效，不做引用计数增减。
    Move(VReg),
    /// 复制值：源寄存器保持有效，堆值多持一次引用。
    ///
    /// 与 `Move` 的分工是所有权语义：把临时值写进绑定用 `Move`（转移），
    /// 把具名绑定赋给另一个绑定用 `Copy`（两边都要继续可用）。
    Copy(VReg),
    /// 把静态值装箱成动态值。
    Box(VReg),
    /// 从动态值取出静态值。
    Unbox(VReg),
    /// 显式标量转换。
    Cast {
        /// 源寄存器。
        value: VReg,
        /// 目标标量。
        target: ScalarType,
    },
    /// 算术运算；需要提升时由降低器先插入显式转换。
    Arith {
        /// 运算类别。
        op: ArithOp,
        /// 左操作数。
        left: VReg,
        /// 右操作数。
        right: VReg,
    },
    /// 比较运算，结果写入 `dst`。
    Compare {
        /// 运算类别。
        op: CompareOp,
        /// 左操作数。
        left: VReg,
        /// 右操作数。
        right: VReg,
    },
    /// 集合代数运算；`dst` 是结果集合。
    ///
    /// 与 [`Self::Arith`] 分开是有意的：集合的「加法」不是可折叠的算术，
    /// 复用 `Arith` 会让优化器与后端无从判断能否强度削减。
    SetOp {
        /// 运算类别。
        op: SetOpKind,
        /// 左操作数寄存器。
        left: VReg,
        /// 右操作数寄存器。
        right: VReg,
    },
    /// 集合关系比较；`dst` 恒为布尔。
    ///
    /// 与 [`Self::Compare`] 分开的理由同 [`Self::SetOp`]：`Compare` 的 `Less`
    /// 在两个整数上是「小于」，在两个集合上却是「真子集」。
    SetCompare {
        /// 比较类别。
        op: SetCompareOp,
        /// 左操作数寄存器。
        left: VReg,
        /// 右操作数寄存器。
        right: VReg,
    },
    /// 构造数组。
    NewArray {
        /// 元素寄存器。
        elements: Vec<VReg>,
    },
    /// 构造元组。
    NewTuple {
        /// 元素寄存器。
        elements: Vec<VReg>,
    },
    /// 构造无序字典表。
    NewDictTable {
        /// 按键值对排列的条目。
        entries: Vec<(String, VReg)>,
    },
    /// 构造字典列。
    NewDictColumn {
        /// 按键值对排列的条目。
        entries: Vec<(String, VReg)>,
    },
    /// 构造集合。
    NewSet {
        /// 元素寄存器。
        elements: Vec<VReg>,
    },
    /// 按精确路径读取；路径只做精确索引，批量 1 只接受单段路径。
    IndexGet {
        /// 来源容器。
        source: VReg,
        /// 精确路径。
        path: Vec<PathStep>,
    },
    /// 按类型阶段规范化计划执行高级选择。
    ///
    /// `plan` 索引 [`TacProgram::selection_plans`]；步长和动态随机数量只携带
    /// 已降低的寄存器，不允许执行器重新解析源码或重算选择形状。
    SelectorApply {
        /// 来源容器寄存器。
        source: VReg,
        /// 规范化选择计划索引。
        plan: u32,
        /// 动态步长寄存器。
        step: Option<VReg>,
        /// 与计划项一一对应的动态随机数量寄存器。
        random_counts: Vec<Option<VReg>>,
    },
    /// 将一个标量按事务性广播计划写入根容器。
    ///
    /// `plan` 索引 [`TacProgram::broadcast_assignment_plans`]；执行器必须先验证
    /// 全部目标，再提交任何写入。
    BroadcastAssign {
        /// 根容器寄存器。
        root: VReg,
        /// 标量值寄存器。
        value: VReg,
        /// 广播计划索引。
        plan: u32,
    },
    /// 设置当前 VM 的选择器随机源种子。
    RandomSeed {
        /// 种子寄存器。
        value: VReg,
        /// 随机种子计划索引。
        plan: u32,
    },
    /// 无条件跳转。
    Jump(BlockId),
    /// 条件分支。
    BranchIf {
        /// 条件寄存器，必须是布尔类别。
        condition: VReg,
        /// 条件成立时的目标块。
        if_true: BlockId,
        /// 条件不成立时的目标块。
        if_false: BlockId,
    },
    /// 静态调用。
    Call {
        /// 被调函数的静态索引。
        callee: FuncId,
        /// 调用签名。
        signature: SigId,
        /// 实参。
        arguments: Vec<TacArgument>,
    },
    /// 动态派发调用。
    CallDynamic {
        /// 被调用对象。
        callee: VReg,
        /// 实参。
        arguments: Vec<TacArgument>,
    },
    /// 函数返回；无返回值时 `dst` 为空。
    Return {
        /// 返回值寄存器。
        value: Option<VReg>,
    },
    /// 抛出可恢复错误。
    Raise {
        /// 错误对象。
        value: VReg,
    },
    /// 构造一个可恢复错误对象。
    MakeError {
        /// 语言层错误类型名称。
        type_name: String,
        /// 可选错误码寄存器。
        code: Option<VReg>,
        /// 可选人类可读消息寄存器。
        message: Option<VReg>,
    },
    /// 调用一个 `finally` 子程序并挂起当前退出类别。
    CallSub {
        /// 子程序入口块。
        sub: BlockId,
    },
    /// 从 `finally` 子程序返回并恢复挂起的退出类别。
    RetFromSub,
    /// 运行时检查；失败时跳到失败块。
    Check {
        /// `IrRuntimeCheck.kind` 的稳定拼写。
        kind: String,
        /// 被检查的运行时值。
        value: VReg,
        /// 检查失败时的目标块。
        on_failure: BlockId,
    },
    /// 按冻结计划逐条释放一个值。
    Release {
        /// 被释放的值。
        value: VReg,
        /// 强释放或弱释放。
        kind: ReleaseActionKind,
    },
    /// 标记一个值已转移出去，抑制其释放。
    Transfer {
        /// 被转移的值。
        value: VReg,
    },
    /// 执行一个作用域在指定退出边上的冻结释放计划。
    RunReleasePlan {
        /// 作用域编号。
        scope: u32,
        /// 退出边稳定名称。
        exit: String,
    },
    /// 进入一个静态作用域。
    EnterScope(u32),
    /// 离开一个静态作用域。
    ExitScope {
        /// 作用域编号。
        scope: u32,
        /// 退出边稳定名称。
        exit: String,
    },
}

/// 一条带源码位置的三地址指令。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TacInstr {
    /// 指令操作。
    pub op: TacOp,
    /// 结果寄存器；无结果的指令为空。
    pub dst: Option<VReg>,
    /// 对应源码区间，用于回填错误堆栈的字节码偏移。
    pub span: IrSpan,
}

impl TacInstr {
    /// 创建一条无结果指令。
    #[must_use]
    pub fn new(op: TacOp, span: IrSpan) -> Self {
        Self {
            op,
            dst: None,
            span,
        }
    }

    /// 创建一条有结果指令。
    #[must_use]
    pub fn with_dst(op: TacOp, dst: VReg, span: IrSpan) -> Self {
        Self {
            op,
            dst: Some(dst),
            span,
        }
    }
}

/// 一个基本块。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TacBlock {
    /// 块编号。
    pub id: BlockId,
    /// 所属静态作用域。
    pub scope: u32,
    /// 按执行顺序排列的指令。
    pub instructions: Vec<TacInstr>,
}

/// 一条异常处理器表条目。
///
/// 条目按嵌套深度降序存放；查表等于「本帧条目线性扫描 + pc 区间命中」。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TacHandler {
    /// 受保护区间的起止块（含起点，不含终点）。
    pub protected: (BlockId, BlockId),
    /// 处理器入口块。
    pub handler: BlockId,
    /// 所属作用域。
    pub scope: u32,
    /// 命中处理器后应执行的退出边。
    pub exit: String,
    /// 捕获的错误类型名；`Error`/`XiaoError` 匹配一切可恢复错误。
    pub catch_type: Option<String>,
    /// 错误绑定寄存器。
    pub binding: Option<VReg>,
}

/// 一个已降低的函数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TacFunction {
    /// 函数名；脚本入口使用空名。
    pub name: String,
    /// 调用签名索引；动态签名时为空。
    pub signature: Option<SigId>,
    /// 入口块。
    pub entry: BlockId,
    /// 基本块。
    pub blocks: Vec<TacBlock>,
    /// 形参寄存器，按声明顺序。
    pub parameters: Vec<VReg>,
    /// 局部变量寄存器，按声明顺序。
    pub locals: Vec<VReg>,
    /// 本函数局部编号空间内的寄存器类别。
    ///
    /// `VReg` 会在每个函数重新从零编号，因此类别必须与函数一起携带；物理
    /// 分配不得读取 [`TacProgram::categories`] 的入口兼容视图。
    pub categories: CategoryMap,
    /// 该函数涉及的作用域编号。
    pub scopes: Vec<u32>,
    /// 异常处理器表。
    pub handlers: Vec<TacHandler>,
    /// `IrValue.id` 到寄存器的映射；释放计划按值编号寻址，解释器据此找到载体槽位。
    pub value_registers: std::collections::BTreeMap<u32, VReg>,
    /// 函数源码区间。
    pub span: IrSpan,
}

impl TacFunction {
    /// 按编号取基本块。
    #[must_use]
    pub fn block(&self, id: BlockId) -> Option<&TacBlock> {
        self.blocks.get(id.get() as usize)
    }
}

/// 后端共享的版本字段。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TacAbi {
    /// 研究用编码版本。
    pub bytecode_abi_version: u32,
    /// Runtime 句柄与值表示版本。
    pub runtime_abi_version: u32,
    /// 来源 `IrProgram` 的版本。
    pub ir_version: u32,
    /// 语言版本字符串。
    pub language_version: String,
    /// 目标平台描述。
    pub target: String,
}

/// 一份降低完成的三地址程序。
#[derive(Clone, Debug, PartialEq)]
pub struct TacProgram {
    /// 三地址格式版本。
    pub version: u32,
    /// 版本字段。
    pub abi: TacAbi,
    /// 常量池。
    pub constants: ConstPool,
    /// 调用签名表。
    pub signatures: CallSigTable,
    /// 函数表；索引 0 是脚本入口。
    pub functions: Vec<TacFunction>,
    /// 脚本入口函数的寄存器类别兼容视图。
    ///
    /// 新代码应读取 [`TacFunction::categories`]。该字段只保留给 R2a 已有的
    /// 单函数观察代码，不能用于命名函数的物理寄存器分配。
    pub categories: CategoryMap,
    /// 冻结的释放计划；解释器在退出点上按 `(作用域, 退出边)` 取出执行。
    pub plans: Vec<crate::research::lower::TacReleasePlan>,
    /// 类型阶段规范化选择计划表。
    pub selection_plans: Vec<xiao_ir::IrSelectionPlan>,
    /// 类型阶段事务性广播计划表。
    pub broadcast_assignment_plans: Vec<xiao_ir::IrBroadcastAssignmentPlan>,
    /// 类型阶段随机种子计划表。
    pub random_seed_plans: Vec<xiao_ir::IrRandomSeedPlan>,
    /// 本批次尚未降低的构造说明；为空表示全部语句都已降低。
    ///
    /// 这里刻意保留说明而不是静默跳过：未降低的构造会让程序少算一部分，
    /// 验证器据此直接拒绝，避免出现「能跑但结果不对」。
    pub unsupported: Vec<String>,
}

/// 调用签名表；定义与降低器分开，因为它是数据而不是降低规则。
pub use crate::research::sig::CallSigTable;
