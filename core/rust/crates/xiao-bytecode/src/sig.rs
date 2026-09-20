//! 调用签名表，补齐 `IrProgram` 缺失的调用 ABI 描述。
//!
//! `IrStatementKind::Function` 只给出**被调方**的形参列表；调用点的
//! `IrExpression.ty` 即使解析成 `IrType::Function`，也只保留参数类型和返回
//! 类型，丢掉了 `IrParameter.kind`（位置专用、`*args`、关键字专用、`**kwargs`）
//! 和默认值。因此当前不存在可直接消费的调用 ABI 描述，降低器必须在构建期
//! 合成这张表，调用点只携带 [`SigId`]。
//!
//! 被调用对象类型不是 `IrType::Function` 时退化为 [`CallSig::dynamic`]，走全
//! 动态派发路径：慢，但语义必须与静态路径完全一致。

use xiao_ir::IrType;

use crate::tac::SigId;

/// 形参类别。
///
/// 拼写与 `IrParameter.kind` 逐字一致，由 [`ParamKind::from_name`] 解析。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ParamKind {
    /// 位置或关键字形参。
    PositionalOrKeyword,
    /// `/` 之前的位置专用形参。
    PositionalOnly,
    /// `*` 之后的关键字专用形参。
    KeywordOnly,
    /// `*args` 可变位置形参。
    VarArgs,
    /// `**kwargs` 可变关键字形参。
    VarKeywords,
}

impl ParamKind {
    /// 按 `IrParameter.kind` 的稳定拼写解析；未知拼写返回 `None`。
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "positional_or_keyword" => Self::PositionalOrKeyword,
            "positional_only" => Self::PositionalOnly,
            "keyword_only" => Self::KeywordOnly,
            "var_args" => Self::VarArgs,
            "var_keywords" => Self::VarKeywords,
            _ => return None,
        })
    }

    /// 判断该形参能否通过关键字传入。
    #[must_use]
    pub const fn accepts_keyword(self) -> bool {
        matches!(self, Self::PositionalOrKeyword | Self::KeywordOnly)
    }
}

/// 一条调用签名。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallSig {
    /// 形参名称，按声明顺序。
    pub parameter_names: Vec<String>,
    /// 形参类别，与名称一一对应。
    pub parameter_kinds: Vec<ParamKind>,
    /// 形参静态类型。
    pub parameter_types: Vec<IrType>,
    /// 每个形参是否有默认值。
    pub has_defaults: Vec<bool>,
    /// `*args` 形参的寄存器位置（在被调方帧中）。
    pub var_args_slot: Option<usize>,
    /// `**kwargs` 形参的寄存器位置。
    pub kw_args_slot: Option<usize>,
    /// 返回类型。
    pub return_type: IrType,
}

impl CallSig {
    /// 创建一个全部为普通形参的签名。
    #[must_use]
    pub fn plain(parameter_types: Vec<IrType>, return_type: IrType) -> Self {
        let count = parameter_types.len();
        Self {
            parameter_names: vec![String::new(); count],
            parameter_kinds: vec![ParamKind::PositionalOrKeyword; count],
            parameter_types,
            has_defaults: vec![false; count],
            var_args_slot: None,
            kw_args_slot: None,
            return_type,
        }
    }

    /// 创建一个全动态签名，用于被调用对象类型未知的调用点。
    #[must_use]
    pub fn dynamic() -> Self {
        Self {
            parameter_names: Vec::new(),
            parameter_kinds: Vec::new(),
            parameter_types: Vec::new(),
            has_defaults: Vec::new(),
            var_args_slot: None,
            kw_args_slot: None,
            return_type: IrType::Dynamic,
        }
    }

    /// 判断签名是否全动态。
    #[must_use]
    pub fn is_dynamic(&self) -> bool {
        self.parameter_types.is_empty() && self.parameter_names.is_empty()
    }

    /// 判断签名是否含可变参数。
    #[must_use]
    pub fn has_variadic(&self) -> bool {
        self.var_args_slot.is_some() || self.kw_args_slot.is_some()
    }
}

/// 调用签名表。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CallSigTable {
    entries: Vec<CallSig>,
}

impl CallSigTable {
    /// 创建空签名表。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 登记一条签名并返回索引；相同签名复用同一索引。
    pub fn intern(&mut self, signature: CallSig) -> SigId {
        if let Some(index) = self.entries.iter().position(|item| *item == signature) {
            return SigId::new(index as u32);
        }
        self.entries.push(signature);
        SigId::new((self.entries.len() - 1) as u32)
    }

    /// 按键取值。
    #[must_use]
    pub fn get(&self, id: SigId) -> Option<&CallSig> {
        self.entries.get(id.get() as usize)
    }

    /// 返回签名数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 判断签名表为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 按签名索引顺序遍历全部条目。
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &CallSig> {
        self.entries.iter()
    }

    /// 从保持原始索引顺序的条目重建签名表。
    pub(crate) fn from_entries(entries: Vec<CallSig>) -> Self {
        Self { entries }
    }
}
