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
    /// 已分配身份，尚未开放调用。
    Reserved,
    Tombstone,
}
/// 内置入口类别。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntrinsicKind {
    ScalarConversion,
    CharConstructor,
    SetConstructor,
    SetAdd,
    SetRemove,
    ArrayPush,
    ErrorConstructor,
    ErrorCode,
    LfloatDivide,
    Cmath,
    Time,
    Args,
    Fs,
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
    Two,
    Three,
    Four,
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
    StrArray,
    StrIntDict,
    Bool,
    Set,
    SetElement,
    ArrayElement,
    ErrorParameters,
    LfloatDivideParameters,
    TimeValueAndUnit,
    TwoStringsAndBool,
    Error,
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
    MutatesContainer,
    WritesStdout,
    ReadsStdin,
    ReadsFilesystem,
    ReadsFilesystemAndAllocates,
    WritesFilesystem,
    SpawnsProcess,
    WaitsProcess,
    ReleasesProcess,
    ReadsClock,
    ReadsArguments,
    Sleeps,
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
    Clock,
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
/// 环境快照能力位。
pub const CAPABILITY_ENVIRONMENT: u64 = 1 << 7;
/// 省略授权时沿用 ABI 1.19 的默认范围。
pub const DEFAULT_CAPABILITY_MASK: u64 = CAPABILITY_FILESYSTEM
    | CAPABILITY_CLOCK
    | CAPABILITY_PROCESS
    | CAPABILITY_NETWORK
    | CAPABILITY_NATIVE_EXTENSION
    | CAPABILITY_STDOUT
    | CAPABILITY_STDIN;
/// 当前允许显式请求的能力位掩码。
pub const KNOWN_CAPABILITY_MASK: u64 = DEFAULT_CAPABILITY_MASK | CAPABILITY_ENVIRONMENT;

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
            Self::Clock => Some(CAPABILITY_CLOCK),
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
            Self::Clock => "clock",
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
    SetAdd,
    SetRemove,
    ArrayPush,
    MakeError,
    ErrorCode,
    LfloatDivide,
    CmathSqrt,
    CmathFloor,
    CmathCeil,
    CmathRound,
    CmathTrunc,
    CmathAbs,
    CmathMin,
    CmathMax,
    CmathPow,
    CmathSin,
    CmathCos,
    CmathTan,
    CmathExp,
    CmathLn,
    CmathLog10,
    TimeUtcNow,
    TimeMonotonic,
    TimeFormatUtc,
    TimeSleep,
    ArgsProgramName,
    ArgsArgv,
    FsWriteText,
    FsListDir,
    FsRemove,
    FsRename,
    FsMetadata,
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
    ErrorCode,
    LfloatDivide,
    CmathSqrt,
    CmathFloor,
    CmathCeil,
    CmathRound,
    CmathTrunc,
    CmathAbs,
    CmathMin,
    CmathMax,
    CmathPow,
    CmathSin,
    CmathCos,
    CmathTan,
    CmathExp,
    CmathLn,
    CmathLog10,
    TimeUtcNow,
    TimeMonotonic,
    TimeFormatUtc,
    TimeSleep,
    ArgsProgramName,
    ArgsArgv,
    FsWriteText,
    FsListDir,
    FsRemove,
    FsRename,
    FsMetadata,
    SetAdd,
    SetRemove,
    ArrayPush,
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
/// F-5冻结的cmath名称及后端绑定键；仅声明数据，不安装VM/LLVM执行器。
pub const CMATH_DECLARATIONS: [(&str, VmBinding, AbiBinding); 15] = [
    ("cmath_sqrt", VmBinding::CmathSqrt, AbiBinding::CmathSqrt),
    ("cmath_floor", VmBinding::CmathFloor, AbiBinding::CmathFloor),
    ("cmath_ceil", VmBinding::CmathCeil, AbiBinding::CmathCeil),
    ("cmath_round", VmBinding::CmathRound, AbiBinding::CmathRound),
    ("cmath_trunc", VmBinding::CmathTrunc, AbiBinding::CmathTrunc),
    ("cmath_abs", VmBinding::CmathAbs, AbiBinding::CmathAbs),
    ("cmath_min", VmBinding::CmathMin, AbiBinding::CmathMin),
    ("cmath_max", VmBinding::CmathMax, AbiBinding::CmathMax),
    ("cmath_pow", VmBinding::CmathPow, AbiBinding::CmathPow),
    ("cmath_sin", VmBinding::CmathSin, AbiBinding::CmathSin),
    ("cmath_cos", VmBinding::CmathCos, AbiBinding::CmathCos),
    ("cmath_tan", VmBinding::CmathTan, AbiBinding::CmathTan),
    ("cmath_exp", VmBinding::CmathExp, AbiBinding::CmathExp),
    ("cmath_ln", VmBinding::CmathLn, AbiBinding::CmathLn),
    ("cmath_log10", VmBinding::CmathLog10, AbiBinding::CmathLog10),
];

/// 一元cmath的必填参数。
pub const CMATH_UNARY_PARAMETERS: [(&str, ValueType); 1] = [("value", ValueType::Float)];
/// min/max的必填参数。
pub const CMATH_COMPARISON_PARAMETERS: [(&str, ValueType); 2] =
    [("left", ValueType::Float), ("right", ValueType::Float)];
/// pow的必填参数。
pub const CMATH_POWER_PARAMETERS: [(&str, ValueType); 2] =
    [("base", ValueType::Float), ("exponent", ValueType::Float)];

/// 读取34～48的冻结形参顺序；参数绑定与类型/IR接线留B4。
#[must_use]
pub const fn cmath_parameters(id: IntrinsicId) -> Option<&'static [(&'static str, ValueType)]> {
    match id.get() {
        34..=39 | 43..=48 => Some(&CMATH_UNARY_PARAMETERS),
        40 | 41 => Some(&CMATH_COMPARISON_PARAMETERS),
        42 => Some(&CMATH_POWER_PARAMETERS),
        _ => None,
    }
}

/// time 入口的位置和命名实参共享同一形参顺序。
#[must_use]
pub const fn time_parameters(id: IntrinsicId) -> Option<&'static [(&'static str, ValueType)]> {
    match id.get() {
        49 | 50 => Some(&[("unit", ValueType::Str)]),
        51 => Some(&[("unix_time", ValueType::Int), ("unit", ValueType::Str)]),
        52 => Some(&[("duration", ValueType::Int), ("unit", ValueType::Str)]),
        _ => None,
    }
}

/// fs 位置与命名参数共同使用的冻结形参表。
#[must_use]
pub const fn fs_parameters(id: IntrinsicId) -> Option<&'static [(&'static str, ValueType)]> {
    match id.get() {
        55 => Some(&[
            ("path", ValueType::Str),
            ("text", ValueType::Str),
            ("overwrite", ValueType::Bool),
        ]),
        56 | 57 | 59 => Some(&[("path", ValueType::Str)]),
        58 => Some(&[
            ("source", ValueType::Str),
            ("destination", ValueType::Str),
            ("overwrite", ValueType::Bool),
        ]),
        _ => None,
    }
}

impl AbiBinding {
    /// time 的链接符号，其他绑定沿各自入口发射。
    #[must_use]
    pub const fn time_symbol(self) -> Option<&'static str> {
        match self {
            Self::TimeUtcNow => Some("xiao_runtime_time_utc_now"),
            Self::TimeMonotonic => Some("xiao_runtime_time_monotonic"),
            Self::TimeFormatUtc => Some("xiao_runtime_time_format_utc"),
            Self::TimeSleep => Some("xiao_runtime_time_sleep"),
            _ => None,
        }
    }

    /// 返回 args 的固定 C ABI 符号。
    #[must_use]
    pub const fn args_symbol(self) -> Option<&'static str> {
        match self {
            Self::ArgsProgramName => Some("xiao_runtime_args_program_name"),
            Self::ArgsArgv => Some("xiao_runtime_args_argv"),
            _ => None,
        }
    }

    /// 返回 fs 的固定 C ABI 符号。
    #[must_use]
    pub const fn fs_symbol(self) -> Option<&'static str> {
        match self {
            Self::FsWriteText => Some("xiao_runtime_fs_write_text"),
            Self::FsListDir => Some("xiao_runtime_fs_list_dir"),
            Self::FsRemove => Some("xiao_runtime_fs_remove"),
            Self::FsRename => Some("xiao_runtime_fs_rename"),
            Self::FsMetadata => Some("xiao_runtime_fs_metadata"),
            _ => None,
        }
    }
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
                IntrinsicKind::LfloatDivide => {
                    if declaration.id.get() != 33
                        || declaration.signature
                            != (Signature {
                                arity: Arity::Four,
                                parameter: ValueType::LfloatDivideParameters,
                                return_type: ValueType::Lfloat,
                            })
                        || declaration.vm_binding != VmBinding::LfloatDivide
                        || declaration.abi_binding != AbiBinding::LfloatDivide
                        || declaration.effects != Effects::Allocates
                        || declaration.capability != Capability::None
                    {
                        return Err(ContractError::SignatureMismatch(declaration.id));
                    }
                }
                IntrinsicKind::Cmath => {
                    let contract = declaration
                        .id
                        .get()
                        .checked_sub(34)
                        .and_then(|index| CMATH_DECLARATIONS.get(index as usize));
                    let Some(&(name, vm, abi)) = contract else {
                        return Err(ContractError::SignatureMismatch(declaration.id));
                    };
                    let parameters = cmath_parameters(declaration.id).expect("已校验cmath编号");
                    let arity = if parameters.len() == 1 {
                        Arity::One
                    } else {
                        Arity::Two
                    };
                    if declaration.public_name != name
                        || declaration.signature
                            != (Signature {
                                arity,
                                parameter: ValueType::Float,
                                return_type: ValueType::Float,
                            })
                        || declaration.effects != Effects::Pure
                        || declaration.capability != Capability::None
                        || declaration.vm_binding != vm
                        || declaration.abi_binding != abi
                        || declaration.error_code != "runtime.type_mismatch"
                    {
                        return Err(ContractError::SignatureMismatch(declaration.id));
                    }
                }
                IntrinsicKind::ErrorCode => {
                    if declaration.signature
                        != (Signature {
                            arity: Arity::One,
                            parameter: ValueType::Error,
                            return_type: ValueType::Str,
                        })
                        || declaration.vm_binding != VmBinding::ErrorCode
                        || declaration.abi_binding != AbiBinding::ErrorCode
                        || declaration.effects != Effects::Allocates
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
                IntrinsicKind::SetAdd | IntrinsicKind::SetRemove => {
                    let (vm, abi, result) = if declaration.kind == IntrinsicKind::SetAdd {
                        (VmBinding::SetAdd, AbiBinding::SetAdd, ValueType::None)
                    } else {
                        (VmBinding::SetRemove, AbiBinding::SetRemove, ValueType::Bool)
                    };
                    if declaration.signature.arity != Arity::Two
                        || declaration.signature.parameter != ValueType::SetElement
                        || declaration.signature.return_type != result
                        || declaration.vm_binding != vm
                        || declaration.abi_binding != abi
                        || declaration.effects != Effects::MutatesContainer
                        || declaration.capability != Capability::None
                    {
                        return Err(ContractError::SignatureMismatch(declaration.id));
                    }
                }
                IntrinsicKind::ArrayPush => {
                    if declaration.signature.arity != Arity::Two
                        || declaration.signature.parameter != ValueType::ArrayElement
                        || declaration.signature.return_type != ValueType::None
                        || declaration.vm_binding != VmBinding::ArrayPush
                        || declaration.abi_binding != AbiBinding::ArrayPush
                        || declaration.effects != Effects::MutatesContainer
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
                IntrinsicKind::Args => {
                    let (result, vm, abi) = match declaration.id.get() {
                        53 => (
                            ValueType::Str,
                            VmBinding::ArgsProgramName,
                            AbiBinding::ArgsProgramName,
                        ),
                        54 => (
                            ValueType::StrArray,
                            VmBinding::ArgsArgv,
                            AbiBinding::ArgsArgv,
                        ),
                        _ => return Err(ContractError::SignatureMismatch(declaration.id)),
                    };
                    if declaration.signature
                        != (Signature {
                            arity: Arity::Zero,
                            parameter: ValueType::None,
                            return_type: result,
                        })
                        || declaration.capability != Capability::Process
                        || declaration.effects != Effects::ReadsArguments
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

/// 四参显式除法的冻结参数顺序；返回的索引只重排已按源码顺序求值的实参。
pub const LFLOAT_DIVIDE_PARAMETERS: [(&str, ValueType); 4] = [
    ("numerator", ValueType::Lfloat),
    ("denominator", ValueType::Lfloat),
    ("precision", ValueType::Int),
    ("rounding", ValueType::Str),
];

/// 绑定四个必填位置/命名参数；拒绝展开、重复、缺参和命名后的裸位置参数。
pub fn lfloat_divide_argument_order(arguments: &[(&str, Option<&str>)]) -> Option<[usize; 4]> {
    if arguments.len() != 4 {
        return None;
    }
    let mut order = [None; 4];
    let mut positional = 0;
    let mut saw_keyword = false;
    for (source, (kind, name)) in arguments.iter().enumerate() {
        let destination = match (*kind, name) {
            ("positional", None) if !saw_keyword => {
                let index = positional;
                positional += 1;
                index
            }
            ("keyword", Some(name)) => {
                saw_keyword = true;
                LFLOAT_DIVIDE_PARAMETERS
                    .iter()
                    .position(|(expected, _)| expected == name)?
            }
            _ => return None,
        };
        if order.get_mut(destination)?.replace(source).is_some() {
            return None;
        }
    }
    Some([order[0]?, order[1]?, order[2]?, order[3]?])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    /// 核对活动 ID 与原有 cmath 的形参、效果及墓碑身份。
    fn cmath_fifteen_declarations_keep_the_frozen_identity_and_parameters() {
        validate_table().unwrap();
        let mut active: Vec<_> = declarations()
            .iter()
            .filter(|d| d.status == DeclarationStatus::Active)
            .map(|d| d.id.get())
            .collect();
        active.sort_unstable();
        assert_eq!(active, (1..=59).collect::<Vec<_>>());
        for (index, &(name, vm, abi)) in CMATH_DECLARATIONS.iter().enumerate() {
            let id = IntrinsicId::new(34 + index as u32).unwrap();
            let declaration = active_by_id(id).unwrap();
            assert_eq!(by_name(name), Some(declaration));
            assert_eq!(declaration.kind, IntrinsicKind::Cmath);
            assert_eq!(declaration.vm_binding, vm);
            assert_eq!(declaration.abi_binding, abi);
            assert_eq!(declaration.effects, Effects::Pure);
            assert_eq!(declaration.capability, Capability::None);
            assert!(
                cmath_parameters(id)
                    .unwrap()
                    .iter()
                    .all(|(_, ty)| *ty == ValueType::Float)
            );
        }
        assert_eq!(cmath_parameters(IntrinsicId::new(33).unwrap()), None);
        assert_eq!(cmath_parameters(IntrinsicId::new(49).unwrap()), None);
        assert_eq!(cmath_parameters(IntrinsicId::new(100).unwrap()), None);
        assert_eq!(
            by_id(IntrinsicId::new(100).unwrap()).unwrap().status,
            DeclarationStatus::Tombstone
        );
    }

    #[test]
    fn explicit_division_binding_preserves_source_order_and_rejects_ambiguous_shapes() {
        assert_eq!(
            lfloat_divide_argument_order(&[
                ("keyword", Some("rounding")),
                ("keyword", Some("precision")),
                ("keyword", Some("denominator")),
                ("keyword", Some("numerator"))
            ]),
            Some([3, 2, 1, 0])
        );
        assert_eq!(
            lfloat_divide_argument_order(&[
                ("positional", None),
                ("positional", None),
                ("positional", None),
                ("positional", None)
            ]),
            Some([0, 1, 2, 3])
        );
        for args in [
            [
                ("keyword", Some("numerator")),
                ("positional", None),
                ("keyword", Some("precision")),
                ("keyword", Some("rounding")),
            ],
            [
                ("positional", None),
                ("keyword", Some("numerator")),
                ("keyword", Some("precision")),
                ("keyword", Some("rounding")),
            ],
            [
                ("positional", None),
                ("positional", None),
                ("spread", None),
                ("keyword", Some("rounding")),
            ],
        ] {
            assert!(lfloat_divide_argument_order(&args).is_none());
        }
    }
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
    fn array_push_contract_is_active_and_bound_on_both_paths() {
        let push = by_name("array_push").expect("array_push 声明");
        assert_eq!(push.id, IntrinsicId::new_unchecked(31));
        assert_eq!(push.vm_binding, VmBinding::ArrayPush);
        assert_eq!(push.abi_binding, AbiBinding::ArrayPush);
        assert_eq!(push.signature.arity, Arity::Two);
        assert_eq!(push.signature.parameter, ValueType::ArrayElement);
        assert_eq!(push.signature.return_type, ValueType::None);
    }
    #[test]
    fn error_code_has_a_unique_owned_string_contract() {
        let declaration = by_name("error_code").expect("error_code 声明");
        assert_eq!(declaration.id.get(), 32);
        assert_eq!(declaration.kind, IntrinsicKind::ErrorCode);
        assert_eq!(declaration.signature.parameter, ValueType::Error);
        assert_eq!(declaration.signature.return_type, ValueType::Str);
        assert_eq!(declaration.vm_binding, VmBinding::ErrorCode);
        assert_eq!(declaration.abi_binding, AbiBinding::ErrorCode);
        assert_eq!(declaration.effects, Effects::Allocates);
        assert_eq!(declaration.capability, Capability::None);
        assert_eq!(declarations().len(), 64);
        assert_eq!(
            declarations()
                .iter()
                .filter(|entry| entry.public_name == "error_code")
                .count(),
            1
        );
        assert!(active_by_id(IntrinsicId::new_unchecked(100)).is_none());
        validate_table().expect("完整契约校验");
    }
    #[test]
    fn capability_bits_cover_standard_streams_and_reject_unknown_bits() {
        assert_eq!(Capability::Stdout.bit(), Some(CAPABILITY_STDOUT));
        assert_eq!(Capability::Stdin.bit(), Some(CAPABILITY_STDIN));
        assert!(validate_capability_mask(KNOWN_CAPABILITY_MASK).is_ok());
        assert_eq!(DEFAULT_CAPABILITY_MASK, 0x7f);
        assert_eq!(KNOWN_CAPABILITY_MASK, 0xff);
        assert_eq!(Capability::Clock.bit(), Some(CAPABILITY_CLOCK));
        assert!(validate_capability_mask(CAPABILITY_ENVIRONMENT).is_ok());
        assert!(validate_capability_mask(1 << 8).is_err());
    }
    #[test]
    fn b3_reserved_ids_have_identity_without_callable_bindings() {
        for raw in 60..=63 {
            let id = IntrinsicId::new(raw).unwrap();
            let declaration = by_id(id).unwrap();
            assert_eq!(declaration.status, DeclarationStatus::Reserved);
            assert_eq!(declaration.vm_binding, VmBinding::None);
            assert_eq!(declaration.abi_binding, AbiBinding::None);
            assert!(active_by_id(id).is_none());
            assert!(by_name(declaration.public_name).is_none());
        }
    }

    #[test]
    /// 逐项核对 fs 的形参、能力及链接符号。
    fn fs_parameters_and_symbols_match_the_active_contract() {
        for (id, name, fields) in [
            (55, "write_text", &["path", "text", "overwrite"][..]),
            (56, "list_dir", &["path"][..]),
            (57, "remove", &["path"][..]),
            (58, "rename", &["source", "destination", "overwrite"][..]),
            (59, "metadata", &["path"][..]),
        ] {
            let id = IntrinsicId::new(id).unwrap();
            let declared = active_by_id(id).unwrap();
            let parameters = fs_parameters(id).unwrap();
            assert_eq!(declared.kind, IntrinsicKind::Fs);
            assert_eq!(declared.capability, Capability::Filesystem);
            assert_eq!(declared.public_name, format!("os_fs_{name}"));
            assert_eq!(
                declared.abi_binding.fs_symbol().unwrap(),
                format!("xiao_runtime_fs_{name}")
            );
            assert!(
                parameters
                    .iter()
                    .map(|&(name, _)| name)
                    .eq(fields.iter().copied())
            );
        }
    }
    #[test]
    fn removed_ids_remain_tombstones() {
        let tombstone = by_id(IntrinsicId::new(100).expect("墓碑 ID")).expect("墓碑");
        assert_eq!(tombstone.status, DeclarationStatus::Tombstone);
        assert!(by_name("__reserved_100").is_none());
    }
}
