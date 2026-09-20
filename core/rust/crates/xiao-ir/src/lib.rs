//! 类型化 Xiao 中间表示的 crate 入口。
//!
//! `xiao-ir` 只定义可验证、可版本化的递归 IR 值对象，并提供从前序 AST、
//! 类型和生命周期结果到 IR 的单向降低。它不执行 Xiao 代码，也不依赖 CLI、
//! VM、LLVM 或平台实现。

/// AST/类型/生命周期结果到 IR 的单向降低器。
mod lower;
/// 类型化 IR 的公开递归数据模型。
mod model;
/// 稳定 JSON 快照编码与解码辅助。
mod snapshot;
/// 静态表签名的可序列化镜像与无推断转换。
mod tables;
/// IR 不变量验证器。
mod validate;

/// 重导出降低入口和源码区间转换辅助。
pub use lower::{ir_span, lower_program, lower_type};
/// 重导出 IR 数据模型和版本常量。
pub use model::*;
/// 重导出 JSON 快照接口。
pub use snapshot::{IR_SNAPSHOT_VERSION, SnapshotError, from_json, to_json};
/// 重导出表签名镜像和类型还原入口。
pub use tables::{IrTableMember, IrTableSignature, restore_type};
/// 重导出验证结果、稳定诊断类型和后端释放序列对账入口。
pub use validate::{
    IR_INVALID_CODE, IR_RELEASE_MISMATCH_CODE, IR_VERSION_CODE, IrValidationError,
    IrValidationResult, IrValidator, ObservedRelease, reconcile_release_plans, validate,
};
