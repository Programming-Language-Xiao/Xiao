//! Xiao 统一优化 Pass、验证器、快照和可复现指纹的 crate 入口。

mod config;
mod differential;
mod facts;
mod pipeline;

/// 优化配置、级别和规范化指纹。
pub use config::{
    EmbeddedLocale, OptimizationConfig, OptimizationConfigError, OptimizationFingerprint,
    OptimizationLevel,
};
/// 未优化与优化结果的统一语义差分口径。
pub use differential::{
    DifferentialCase, DifferentialCaseReport, DifferentialDifference, DifferentialInput,
    DifferentialObservation, DifferentialSuiteReport, FourWayCaseReport, FourWayDifference,
    NamedDifferentialDifference,
    FourWayExecutionSide, FourWayObservation, FourWaySuiteReport, ThreeWayCaseReport,
    ThreeWayDifference, ThreeWayExecutionSide, ThreeWayObservation, ThreeWaySuiteReport,
    compare_four_observations, compare_named_observations, compare_observations,
    compare_three_observations,
    run_four_way_differential_suite_with, run_o0_differential_suite,
    run_o0_differential_suite_with, run_three_way_differential_suite_with,
    run_three_way_differential_suite_with_level,
};
/// IR 效果、所有权和别名只读事实。
pub use facts::{AliasFacts, EffectFacts, OwnershipFacts, ProgramFacts};
/// 共享 Pass 接口、快照、验证和流水线报告。
pub use pipeline::{
    CostMetrics, IrSnapshot, OPTIMIZER_VERSION, OptimizationError, OptimizationPass,
    OptimizationPipeline, OptimizationReport, OptimizationResult, PassAssumptions, PassMetadata,
    PassReport, PassResult, PassStatus, SkipReason, ValidationStatus, run_baseline,
};
