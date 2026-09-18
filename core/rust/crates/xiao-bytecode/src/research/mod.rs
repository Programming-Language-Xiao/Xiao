//! 09R2 研究子模块：统一三地址降低与验证。
//!
//! 本子模块是为 09R 特别研究工程建立的原型，**不是稳定语言接口**。试验性
//! 指令、寄存器类别和调用签名在 09R3 冻结前不得被 `xiao-driver`、CLI 或任何
//! 生产路径依赖；研究产物也不得使用 `.xiaoc` 扩展名落盘。
//!
//! 冻结结论与施工顺序见 `docs/DevDocs/09r-bytecode-machine-research.md`。

/// TAC 的显式控制流后继。
pub mod cfg;
/// 研究用 TAC 指令编码、解码与物理指令目录。
pub mod encode;
/// 机型无关的活跃区间分析。
pub mod liveness;
/// `IrProgram` 到统一三地址模型的单向降低。
pub mod lower;
/// 调用签名表，补齐 `IrProgram` 缺失的调用 ABI 描述。
pub mod sig;
/// 统一三地址数据模型。
pub mod tac;
/// 三地址自校验与释放序列对账。
pub mod verify;

/// 重导出 CFG 与活跃分析入口。
pub use cfg::{jump_targets, protected_successors, successors};
/// 重导出研究编码接口与结构化错误。
pub use encode::{
    EncodeError, EncodeOptions, EncodedBlock, EncodedFunction, EncodedProgram, OperandWidth, PcMap,
    build_pc_map, decode, decode_encoded, encode, encode_with_width, validate_encoded,
};
/// 重导出活跃区间结果与分析函数。
pub use liveness::{LiveInterval, Liveness, analyze as analyze_liveness};
/// 重导出降低入口与释放计划携带类型。
pub use lower::{
    TAC_BYTECODE_ABI_VERSION, TAC_RUNTIME_ABI_VERSION, TacReleaseAction, TacReleasePlan,
    lower_program,
};
/// 重导出调用签名表与形参类别。
pub use sig::{CallSig, CallSigTable, ParamKind};
/// 重导出三地址数据模型的全部公开类型。
pub use tac::{
    ArithOp, BlockId, CategoryMap, CompareOp, ConstId, ConstPool, FuncId, PathStep, RegisterClass,
    SigId, TAC_VERSION, TacAbi, TacArgument, TacBlock, TacConstant, TacFunction, TacHandler,
    TacInstr, TacOp, TacProgram, VReg,
};
/// 重导出三地址验证入口与结果类型。
pub use verify::{TacVerification, verify_program};
