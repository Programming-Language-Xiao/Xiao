//! Xiao N0-A LLVM 原生后端。
//!
//! 后端只消费已经由 `xiao-driver` 生成并验证的 [`xiao_ir::IrProgram`]，把 N0-A 支持的
//! 静态标量和控制流写成 LLVM IR 文本。LLVM 开发库不是 Rust 编译期依赖；验证、目标文件
//! 生成和链接都通过调用方显式注入的外部工具链完成。

/// 链接后原生产物的对象格式、符号和 Runtime 组成检查。
mod artifact;
/// 15B 固定条件性能基线与噪声阈值。
mod baseline;
/// 原生产物构建请求和运行观察适配。
mod build;
/// 动态值、容器、表和正常释放计划的 Runtime ABI 降低。
mod dynamic;
/// 后端结构化错误定义。
mod error;
/// 类型化 IR 到 LLVM 文本的降低实现。
mod ir;
/// 15A LLVM Pass 映射、开关和 Runtime 裁剪报告。
mod optimization;
/// 15B 可复现构建比较与差异白名单。
mod reproducible;
/// 规范化目标描述和固定宽度约束。
mod target;
/// LLVM 文本转义和构建指纹的共用纯函数。
mod text;
/// 外部 LLVM 工具链调用和指纹。
mod toolchain;

/// 产物层符号、依赖和 Runtime 组成验证接口。
pub use artifact::{
    ArtifactAcceptanceMode, ArtifactInspection, ArtifactModeReport, ArtifactRuntimeComposition,
    ArtifactVerification, SymbolTableComparison, SymbolTableReport, SymbolTableStatus,
    compare_symbol_table_reports, inspect_artifact, inspect_symbol_table, verify_artifact,
    verify_artifact_mode,
};
/// 性能基线模型。
pub use baseline::{
    BaselineCondition, BaselineError, PerformanceBaseline, PerformanceSample, PerformanceSpread,
    measure_baseline,
};
/// 构建请求、原生产物和运行观察接口。
pub use build::{BuildRequest, NativeArtifact, NativeBuild, NativeRun, NativeRunResult};
/// 后端失败类型和统一结果别名。
pub use error::{CodegenError, Result};
/// LLVM 文本降低选项、入口观察策略和降低入口。
pub use ir::{
    BASELINE_OPTIMIZATION_LEVEL, CodegenOptions, EntryObservation, LlvmModule, NativeInlineFrame,
    NativeSourceMapEntry, NativeStartup, validate_program,
};
/// LLVM Pass 计划和可证明 Runtime 裁剪报告。
pub use optimization::{
    LlvmOptimizationPlan, LlvmOptimizationReport, LlvmPassKind, LlvmPassSwitches,
};
/// 可复现构建比较模型。
pub use reproducible::{
    ArtifactReproducibilityReport, ReproducibilityReport, ReproducibleDifference,
    ReproducibleDifferenceKind, compare_artifact_bytes, compare_reproducible_builds,
    normalize_llvm_text,
};
/// 目标字节序、对象格式和规范化目标描述。
pub use target::{Endian, ObjectFormat, TargetDescription};
/// 计算跨模块复用的稳定 FNV-1a 文本哈希。
pub use text::stable_hash;
/// 外部 LLVM 工具链描述、版本和构建指纹。
pub use toolchain::{
    Toolchain, ToolchainFingerprint, ToolchainVersions, parse_native_static_libraries,
    query_native_static_libraries,
};
/// 共享优化器的稳定报告类型。
pub use xiao_optimizer::OptimizationReport;

/// 后端接口版本；参与原生产物指纹。
///
/// 版本 4 接通 ABI 1.8 表生命周期与返回帧；版本 3 只生成回调并保留构造拒绝。
/// 保留版本 2 的 COFF `sret`/间接参数约定。
/// 该版本进入工具链/产物指纹，避免旧方法拒绝或旧清理文本命中缓存。
pub const CODEGEN_VERSION: u32 = 4;

/// 将同一份类型化 IR 降低为静态标量或 Runtime ABI LLVM 模块。
pub fn lower_program(program: &xiao_ir::IrProgram, options: &CodegenOptions) -> Result<LlvmModule> {
    let level = xiao_optimizer::OptimizationLevel::try_from(options.optimization_level).map_err(
        |error| CodegenError::Unsupported {
            feature: error.to_string(),
            span: None,
        },
    )?;
    let mut config =
        xiao_optimizer::OptimizationConfig::baseline(options.target.fingerprint_fields())
            .with_level(level);
    config.debug_info = options.debug_startup.is_some();
    config.diagnostic_events = options.debug_startup.is_some();
    let native_config = config.clone();
    let pipeline = xiao_optimizer::OptimizationPipeline::new(config).map_err(|error| {
        CodegenError::InvalidIr {
            message: error.to_string(),
        }
    })?;
    let optimized = pipeline
        .run(program)
        .map_err(|error| CodegenError::InvalidIr {
            message: error.to_string(),
        })?;
    let mut module = if dynamic::program_uses_runtime(&optimized.program) {
        dynamic::lower_program(&optimized.program, options)?
    } else {
        ir::lower_static_program(&optimized.program, options)?
    };
    module.optimization_report = optimized.report;
    let plan = LlvmOptimizationPlan::from_config(&native_config, &options.target, "unknown")
        .map_err(|message| CodegenError::InvalidIr { message })?;
    module.native_optimization_report =
        LlvmOptimizationReport::from_plan(plan, &module.runtime_components);
    Ok(module)
}
