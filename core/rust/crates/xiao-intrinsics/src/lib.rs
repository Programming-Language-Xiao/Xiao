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
    CharConstructor,
    SetConstructor,
    ErrorConstructor,
    Print,
    Input,
    ResourceOpenFile,
    ResourceOpenDirectory,
    ResourceClose,
    ResourceReadText,
    ProcessSpawn,
    ProcessWait,
    ProcessClose,
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
    File,
    Directory,
    Resource,
    Process,
}
/// 可观察效果摘要。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effects {
    Pure,
    Allocates,
    WritesStdout,
    ReadsStdin,
    ReadsFilesystem,
    WritesFilesystem,
    SpawnsProcess,
    WaitsProcess,
    ReleasesProcess,
    None,
}
/// 能力需求。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    None,
    Stdout,
    Stdin,
    Filesystem,
    Process,
}

/// 文件系统能力位。
pub const CAPABILITY_FILESYSTEM: u64 = 1 << 0;
/// 时钟能力位。
pub const CAPABILITY_CLOCK: u64 = 1 << 1;
/// 进程能力位。
pub const CAPABILITY_PROCESS: u64 = 1 << 2;
/// 网络能力位。
pub const CAPABILITY_NETWORK: u64 = 1 << 3;
/// 原生扩展能力位。
pub const CAPABILITY_NATIVE_EXTENSION: u64 = 1 << 4;
/// 标准输出能力位。
pub const CAPABILITY_STDOUT: u64 = 1 << 5;
/// 标准输入能力位。
pub const CAPABILITY_STDIN: u64 = 1 << 6;
/// 当前已知能力位的完整掩码；默认请求使用它。
pub const KNOWN_CAPABILITY_MASK: u64 = CAPABILITY_FILESYSTEM
    | CAPABILITY_CLOCK
    | CAPABILITY_PROCESS
    | CAPABILITY_NETWORK
    | CAPABILITY_NATIVE_EXTENSION
    | CAPABILITY_STDOUT
    | CAPABILITY_STDIN;

/// 能力掩码越界错误。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityMaskError {
    /// 收到的掩码。
    pub value: u64,
}

impl std::fmt::Display for CapabilityMaskError {
    /// 输出稳定的字段和值。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "能力掩码含未知位（收到 0x{:016x}，已知掩码 0x{:016x}）",
            self.value, KNOWN_CAPABILITY_MASK
        )
    }
}

impl std::error::Error for CapabilityMaskError {}

impl Capability {
    /// 返回能力对应的位；`none` 不要求任何位。
    #[must_use]
    pub const fn bit(self) -> Option<u64> {
        match self {
            Self::None => None,
            Self::Stdout => Some(CAPABILITY_STDOUT),
            Self::Stdin => Some(CAPABILITY_STDIN),
            Self::Filesystem => Some(CAPABILITY_FILESYSTEM),
            Self::Process => Some(CAPABILITY_PROCESS),
        }
    }

    /// 返回稳定的能力名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Stdout => "stdout",
            Self::Stdin => "stdin",
            Self::Filesystem => "filesystem",
            Self::Process => "process",
        }
    }
}

/// 校验能力掩码只使用当前冻结的能力位。
pub const fn validate_capability_mask(value: u64) -> Result<(), CapabilityMaskError> {
    if value & !KNOWN_CAPABILITY_MASK == 0 {
        Ok(())
    } else {
        Err(CapabilityMaskError { value })
    }
}
/// VM 绑定键；实现层再把它映射为函数。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VmBinding {
    ScalarCast,
    CharConstructor,
    SetNew,
    MakeError,
    Print,
    Input,
    ResourceOpenFile,
    ResourceOpenDirectory,
    ResourceClose,
    ResourceReadText,
    ProcessSpawn,
    ProcessWait,
    ProcessClose,
    None,
}
/// Runtime ABI 绑定键。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbiBinding {
    ErrorNew,
    Print,
    Input,
    ResourceOpenFile,
    ResourceOpenDirectory,
    ResourceClose,
    ResourceReadText,
    ProcessSpawn,
    ProcessWait,
    ProcessClose,
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
                IntrinsicKind::CharConstructor => {
                    if declaration.signature.arity != Arity::One
                        || declaration.signature.parameter != ValueType::Dynamic
                        || declaration.signature.return_type != ValueType::Str
                        || declaration.vm_binding != VmBinding::CharConstructor
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
                IntrinsicKind::ResourceOpenFile
                | IntrinsicKind::ResourceOpenDirectory
                | IntrinsicKind::ResourceClose
                | IntrinsicKind::ResourceReadText => {
                    if declaration.signature.arity != Arity::One
                        || declaration.capability != Capability::Filesystem
                        || declaration.abi_binding == AbiBinding::None
                        || declaration.vm_binding == VmBinding::None
                    {
                        return Err(ContractError::SignatureMismatch(declaration.id));
                    }
                }
                IntrinsicKind::ProcessSpawn
                | IntrinsicKind::ProcessWait
                | IntrinsicKind::ProcessClose => {
                    let (parameter, result, effects, vm, abi) = match declaration.kind {
                        IntrinsicKind::ProcessSpawn => (
                            ValueType::Str,
                            ValueType::Process,
                            Effects::SpawnsProcess,
                            VmBinding::ProcessSpawn,
                            AbiBinding::ProcessSpawn,
                        ),
                        IntrinsicKind::ProcessWait => (
                            ValueType::Process,
                            ValueType::Int,
                            Effects::WaitsProcess,
                            VmBinding::ProcessWait,
                            AbiBinding::ProcessWait,
                        ),
                        IntrinsicKind::ProcessClose => (
                            ValueType::Process,
                            ValueType::None,
                            Effects::ReleasesProcess,
                            VmBinding::ProcessClose,
                            AbiBinding::ProcessClose,
                        ),
                        _ => unreachable!(),
                    };
                    if declaration.signature
                        != (Signature {
                            arity: Arity::One,
                            parameter,
                            return_type: result,
                        })
                        || declaration.capability != Capability::Process
                        || declaration.effects != effects
                        || declaration.vm_binding != vm
                        || declaration.abi_binding != abi
                    {
                        return Err(ContractError::SignatureMismatch(declaration.id));
                    }
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
    fn capability_bits_cover_standard_streams_and_reject_unknown_bits() {
        assert_eq!(Capability::Stdout.bit(), Some(CAPABILITY_STDOUT));
        assert_eq!(Capability::Stdin.bit(), Some(CAPABILITY_STDIN));
        assert!(validate_capability_mask(KNOWN_CAPABILITY_MASK).is_ok());
        assert!(validate_capability_mask(1 << 7).is_err());
    }
    #[test]
    fn removed_ids_remain_tombstones() {
        let tombstone = by_id(IntrinsicId::new(100).expect("墓碑 ID")).expect("墓碑");
        assert_eq!(tombstone.status, DeclarationStatus::Tombstone);
        assert!(by_name("__reserved_100").is_none());
    }
}
