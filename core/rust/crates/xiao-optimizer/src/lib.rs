//! Xiao 统一优化 Pass、验证器、快照和可复现指纹的 crate 入口。

mod config;
mod facts;
mod pipeline;

/// 优化配置、级别和规范化指纹。
pub use config::{
    EmbeddedLocale, OptimizationConfig, OptimizationConfigError, OptimizationFingerprint,
    OptimizationLevel,
};
/// IR 效果、所有权和别名只读事实。
pub use facts::{AliasFacts, EffectFacts, OwnershipFacts, ProgramFacts};
/// 共享 Pass 接口、快照、验证和流水线报告。
pub use pipeline::{
    IrSnapshot, OPTIMIZER_VERSION, OptimizationError, OptimizationPass, OptimizationPipeline,
    OptimizationReport, OptimizationResult, PassAssumptions, PassMetadata, PassReport, PassResult,
    PassStatus, SkipReason, ValidationStatus, run_baseline,
};
