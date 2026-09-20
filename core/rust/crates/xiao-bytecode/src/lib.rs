//! Xiao 字节码指令、常量池、降低器和格式接口。
//!
//! 09R 冻结的三地址模型已经转为生产路径；`research` 子模块只保留兼容重导出，
//! 方便 09R 共享向量和基准设施在迁移期间继续使用原路径。

/// TAC 的显式控制流后继。
pub mod cfg;
/// 字节码编码、解码与物理指令目录。
pub mod encode;
/// 机型无关的活跃区间分析。
pub mod liveness;
/// `IrProgram` 到统一三地址模型的单向降低。
pub mod lower;
/// 调用签名表与形参类别。
pub mod sig;
/// 统一三地址数据模型。
pub mod tac;
/// 三地址自校验与释放序列对账。
pub mod verify;

/// 重导出 CFG 与活跃分析入口。
pub use cfg::{jump_targets, protected_successors, successors};
/// 重导出编码接口与结构化错误。
pub use encode::{
    EncodeError, EncodeOptions, EncodedBlock, EncodedFunction, EncodedProgram, FORMAT_VERSION,
    OPCODE_MAX, OPCODE_MIN, OperandWidth, PcMap, build_pc_map, decode, decode_encoded, encode,
    encode_with_width, validate_encoded,
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
    SetCompareOp, SetOpKind, SigId, TAC_VERSION, TacAbi, TacArgument, TacBlock, TacConstant,
    TacFunction, TacHandler, TacInstr, TacOp, TacProgram, TacTableDefinition, VReg,
};
/// 重导出三地址验证入口与结果类型。
pub use verify::{TacVerification, verify_program};

/// 09R 冻结产物的兼容重导出层；沿革见 09R2D，冻结依据见 09R3。
pub mod research;
