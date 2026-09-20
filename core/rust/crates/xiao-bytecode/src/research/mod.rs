//! 09R 冻结产物的兼容重导出层。
//!
//! 实质实现位于 crate 根的生产模块；这里保留 `research::...` 全部既有路径，
//! 供 09R 共享向量、基准和迁移期调用方使用。新生产代码应直接依赖父 crate 的
//! 根路径，研究编码仍只在内存中流转，不构成公开 `.xiaoc` 文件格式。

/// 兼容暴露生产 CFG 模块。
pub use crate::cfg;
/// 兼容暴露生产编码模块。
pub mod encode {
    pub use crate::encode::*;
}
/// 兼容暴露生产活跃分析模块。
pub use crate::liveness;
/// 兼容暴露生产降低模块。
pub use crate::lower;
/// 兼容暴露生产调用签名模块。
pub use crate::sig;
/// 兼容暴露生产三地址模型模块。
pub use crate::tac;
/// 兼容暴露生产验证模块。
pub use crate::verify;

pub use crate::{
    ArithOp, BlockId, CallSig, CallSigTable, CategoryMap, CompareOp, ConstId, ConstPool, EncodeError,
    EncodeOptions, EncodedBlock, EncodedFunction, EncodedProgram, FORMAT_VERSION, FuncId,
    LiveInterval, Liveness, OPCODE_MAX, OPCODE_MIN, OperandWidth, ParamKind, PathStep, PcMap,
    RegisterClass, SetCompareOp, SetOpKind, SigId, TAC_BYTECODE_ABI_VERSION, TAC_RUNTIME_ABI_VERSION,
    TAC_VERSION, TacAbi, TacArgument, TacBlock, TacConstant, TacFunction, TacHandler, TacInstr,
    TacOp, TacProgram, TacReleaseAction, TacReleasePlan, TacTableDefinition, TacVerification, VReg,
    analyze_liveness, build_pc_map, decode, decode_encoded, encode_with_width, jump_targets,
    lower_program, protected_successors, successors, validate_encoded, verify_program,
};

/// 兼容保留编码函数的旧根路径。
pub use encode::encode;
