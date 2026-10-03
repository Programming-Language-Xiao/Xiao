//! Xiao 内置函数的后端无关契约。
//!
//! 本 crate 位于类型、IR、字节码、VM、LLVM 和 Runtime 的共同下层，只保存声明数据。
//! 它不依赖任何实现层，也不把 Rust 函数指针、运行时值或 LLVM 类型写进契约。

#![allow(missing_docs)]

use std::fmt;

/// `0` 永远不是有效 intrinsic。
pub const INVALID_INTRINSIC_ID: u32 = 0;

/// 稳定的内置函数数字身份，固定为 `u32`。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IntrinsicId(u32);

impl IntrinsicId {
    /// 从有效数字创建 ID；`0` 会被拒绝。
    pub const fn new(raw: u32) -> Option<Self> {
        if raw == INVALID_INTRINSIC_ID {
            None
        } else {
            Some(Self(raw))
        }
    }
    /// 生成器在已校验的声明中创建 ID。
    pub const fn new_unchecked(raw: u32) -> Self {
        Self(raw)
    }
    /// 读取固定宽度数字。
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for IntrinsicId {
    /// 以稳定十进制形式展示 ID。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 声明的生命周期状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationStatus {
    Active,
    Tombstone,
}
/// 内置入口类别。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntrinsicKind {
    ScalarConversion,
    SetConstructor,
    ErrorConstructor,
    Print,
    Input,
    Reserved,
}
/// 参数数量规则。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arity {
    Zero,
    One,
    OptionalOne,
    Variadic,
    Error,
    None,
}
/// 后端无关的值类别。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueType {
    None,
    Dynamic,
    Int,
    Sint,
    Lint,
    Float,
    Sfloat,
    Lfloat,
    Str,
    Bool,
    Set,
    ErrorParameters,
}
/// 可观察效果摘要。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effects {
    Pure,
    Allocates,
    WritesStdout,
    ReadsStdin,
    None,
}
/// 能力需求。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    None,
    Stdout,
    Stdin,
}
/// VM 绑定键；实现层再把它映射为函数。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VmBinding {
    ScalarCast,
    SetNew,
    MakeError,
    Print,
    Input,
    None,
}
/// Runtime ABI 绑定键。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbiBinding {
    ErrorNew,
    Print,
    Input,
    None,
}
/// 签名的后端无关部分。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Signature {
    /// 参数数量规则。
    pub arity: Arity,
    /// 参数值类别。
    pub parameter: ValueType,
    /// 返回值类别。
    pub return_type: ValueType,
}
/// 一条完整声明。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntrinsicDecl {
    /// 稳定数字身份。
    pub id: IntrinsicId,
    /// 前端源码名称。
    pub public_name: &'static str,
    /// 内置入口类别。
    pub kind: IntrinsicKind,
    /// 后端无关签名。
    pub signature: Signature,
    /// 可观察效果。
    pub effects: Effects,
    /// 需要的宿主能力。
    pub capability: Capability,
    /// 稳定错误身份。
    pub error_code: &'static str,
    /// VM 绑定键。
    pub vm_binding: VmBinding,
    /// Runtime ABI 绑定键。
    pub abi_binding: AbiBinding,
    /// 声明生命周期状态。
    pub status: DeclarationStatus,
}

include!(concat!(env!("OUT_DIR"), "/intrinsics_generated.rs"));

/// 返回全部声明；顺序是声明文件顺序，不能用于身份分派。
pub const fn declarations() -> &'static [IntrinsicDecl] {
    DECLARATIONS
}
/// 按稳定 ID 查找活动声明或墓碑。
pub fn by_id(id: IntrinsicId) -> Option<&'static IntrinsicDecl> {
    DECLARATIONS.iter().find(|item| item.id == id)
}
/// 按稳定 ID ���找可执行的活动声明；墓碑和未知 ID 都返回 None。
pub fn active_by_id(id: IntrinsicId) -> Option<&'static IntrinsicDecl> {
    by_id(id).filter(|item| item.status == DeclarationStatus::Active)
}
/// 从源码普通名称查找活动声明；名称只属于前端适配，不进入机器格式。
pub fn by_name(name: &str) -> Option<&'static IntrinsicDecl> {
    DECLARATIONS
        .iter()
        .find(|item| item.status == DeclarationStatus::Active && item.public_name == name)
}
/// 对外暴露表一致性校验，供编码器、VM 和移除验证共同调用。
pub fn validate_table() -> Result<(), ContractError> {
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for declaration in DECLARATIONS {
        if declaration.id.get() == 0 {
            return Err(ContractError::ZeroId);
        }
        if !ids.insert(declaration.id) {
            return Err(ContractError::DuplicateId(declaration.id));
        }
        if declaration.status == DeclarationStatus::Active {
            if !names.insert(declaration.public_name) {
                return Err(ContractError::DuplicateName(declaration.public_name));
            }
            if declaration.vm_binding == VmBinding::None {
                return Err(ContractError::MissingVmBinding(declaration.id));
            }
            if matches!(
                declaration.kind,
                IntrinsicKind::Print | IntrinsicKind::Input
            ) && declaration.abi_binding == AbiBinding::None
            {
                return Err(ContractError::MissingAbiBinding(declaration.id));
            }
            match declaration.kind {
                IntrinsicKind::ScalarConversion => {
                    if declaration.signature.arity != Arity::One
                        || declaration.signature.parameter != ValueType::Dynamic
                        || declaration.vm_binding != VmBinding::ScalarCast
                        || declaration.abi_binding != AbiBinding::None
                        || declaration.effects != Effects::Pure
                        || declaration.capability != Capability::None
                    {
                        return Err(ContractError::SignatureMismatch(declaration.id));
                    }
                }
                IntrinsicKind::SetConstructor => {
                    if declaration.signature.arity != Arity::Zero
                        || declaration.signature.return_type != ValueType::Set
                        || declaration.vm_binding != VmBinding::SetNew
                        || declaration.abi_binding != AbiBinding::None
                        || declaration.effects != Effects::Allocates
                        || declaration.capability != Capability::None
                    {
                        return Err(ContractError::SignatureMismatch(declaration.id));
                    }
                }
                IntrinsicKind::Print
                    if declaration.signature.arity != Arity::Variadic
                        || declaration.signature.parameter != ValueType::Dynamic
                        || declaration.signature.return_type != ValueType::None
                        || declaration.vm_binding != VmBinding::Print
                        || declaration.abi_binding != AbiBinding::Print
                        || declaration.effects != Effects::WritesStdout
                        || declaration.capability != Capability::Stdout =>
                {
                    return Err(ContractError::SignatureMismatch(declaration.id));
                }
                IntrinsicKind::Input
                    if declaration.signature.arity != Arity::OptionalOne
                        || declaration.signature.parameter != ValueType::Str
                        || declaration.signature.return_type != ValueType::Str =>
                {
                    return Err(ContractError::SignatureMismatch(declaration.id));
                }
                IntrinsicKind::Input
                    if declaration.vm_binding != VmBinding::Input
                        || declaration.abi_binding != AbiBinding::Input
                        || declaration.effects != Effects::ReadsStdin
                        || declaration.capability != Capability::Stdin =>
                {
                    return Err(ContractError::SignatureMismatch(declaration.id));
                }
                IntrinsicKind::ErrorConstructor
                    if declaration.signature.arity != Arity::Error
                        || declaration.signature.return_type != ValueType::Dynamic
                        || declaration.vm_binding != VmBinding::MakeError
                        || declaration.abi_binding != AbiBinding::ErrorNew
                        || declaration.effects != Effects::Allocates
                        || declaration.capability != Capability::None =>
                {
                    return Err(ContractError::SignatureMismatch(declaration.id));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// 契约表校验错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    ZeroId,
    DuplicateId(IntrinsicId),
    DuplicateName(&'static str),
    MissingVmBinding(IntrinsicId),
    MissingAbiBinding(IntrinsicId),
    SignatureMismatch(IntrinsicId),
    UnknownId(IntrinsicId),
}

impl fmt::Display for ContractError {
    /// 以稳定文本描述契约校验失败。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroId => write!(f, "IntrinsicId 0 无效"),
            Self::DuplicateId(id) => write!(f, "重复 IntrinsicId {id}"),
            Self::DuplicateName(name) => write!(f, "重复 intrinsic 名称 {name}"),
            Self::MissingVmBinding(id) => write!(f, "缺少 VM 绑定 {id}"),
            Self::MissingAbiBinding(id) => write!(f, "缺少 ABI 绑定 {id}"),
            Self::SignatureMismatch(id) => write!(f, "签名不匹配 {id}"),
            Self::UnknownId(id) => write!(f, "未知 IntrinsicId {id}"),
        }
    }
}

impl std::error::Error for ContractError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_table_is_valid_and_zero_is_rejected() {
        validate_table().expect("生成表必须自洽");
        assert!(IntrinsicId::new(0).is_none());
        assert!(by_id(IntrinsicId::new_unchecked(0)).is_none());
        assert!(active_by_id(IntrinsicId::new_unchecked(100)).is_none());
    }
    #[test]
    fn print_is_variadic_and_returns_none() {
        let print = by_name("print").expect("print 声明");
        assert_eq!(print.signature.arity, Arity::Variadic);
        assert_eq!(print.signature.return_type, ValueType::None);
    }
    #[test]
    fn removed_ids_remain_tombstones() {
        let tombstone = by_id(IntrinsicId::new(100).expect("墓碑 ID")).expect("墓碑");
        assert_eq!(tombstone.status, DeclarationStatus::Tombstone);
        assert!(by_name("__reserved_100").is_none());
    }
}
