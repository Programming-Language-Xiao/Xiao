//! 共享优化 Pass 接口、验证边界和快照回滚。

use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use xiao_ir::{IrProgram, SnapshotError, from_json, to_json};

use crate::config::{OptimizationConfig, OptimizationConfigError, OptimizationLevel};
use crate::facts::ProgramFacts;

/// 当前共享优化器接口版本。
pub const OPTIMIZER_VERSION: u32 = 1;

/// 一个 Pass 的效果、控制流和所有权假设。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PassAssumptions {
    /// 是否允许重排纯表达式。
    pub reorders_pure_expressions: bool,
    /// 是否可能改变控制流。
    pub changes_control_flow: bool,
    /// 是否读取所有权事实。
    pub reads_ownership: bool,
    /// 是否保证保留源码映射。
    pub preserves_source_map: bool,
}

/// Pass 的稳定身份和语义假设。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PassMetadata {
    /// 稳定 Pass 名称。
    pub name: String,
    /// Pass 实现版本。
    pub version: u32,
    /// Pass 的显式假设。
    pub assumptions: PassAssumptions,
}

/// Pass 执行结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PassResult {
    /// Pass 产生了有效结果。
    Applied {
        /// 是否实际改变了 IR。
        changed: bool,
    },
    /// Pass 在当前配置或目标下明确跳过。
    Skipped {
        /// 稳定跳过原因。
        reason: SkipReason,
    },
    /// Pass 主动失败；管线不会提交候选 IR。
    Failed {
        /// 稳定的开发者原因。
        message: String,
    },
}

/// 优化收益不足、能力未实现或证明不可用时的稳定原因。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SkipReason {
    /// O0 不执行语义优化。
    OptimizationDisabled,
    /// 级别占位或 Pass 尚未实现。
    NotImplemented,
    /// 目标能力不足。
    TargetUnsupported,
    /// 当前事实不足以完成证明。
    ProofUnavailable,
    /// 预计收益不足。
    NoBenefit,
}

impl Display for SkipReason {
    /// 输出稳定的跳过原因名称。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::OptimizationDisabled => "optimization-disabled",
            Self::NotImplemented => "not-implemented",
            Self::TargetUnsupported => "target-unsupported",
            Self::ProofUnavailable => "proof-unavailable",
            Self::NoBenefit => "no-benefit",
        };
        formatter.write_str(name)
    }
}

/// 一个可独立快照和验证的共享 Pass。
pub trait OptimizationPass: Send + Sync {
    /// 返回 Pass 的稳定身份和假设。
    fn metadata(&self) -> PassMetadata;
    /// 在候选 IR 上执行变换；不能修改事实摘要或伪造所有权。
    fn run(&self, program: &mut IrProgram, facts: &ProgramFacts) -> PassResult;
}

/// 一份可回滚的 IR 快照。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrSnapshot {
    /// 快照阶段名称。
    pub label: String,
    /// 稳定 JSON 内容。
    pub json: String,
    /// 快照内容指纹。
    pub fingerprint: String,
}

/// Pass 的执行状态。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PassStatus {
    /// 已执行并通过验证。
    Applied,
    /// 已明确跳过。
    Skipped,
}

/// IR 验证状态。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ValidationStatus {
    /// 验证通过。
    Passed,
    /// 验证失败。
    Failed,
}

/// 单个 Pass 的稳定报告。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PassReport {
    /// Pass 元数据。
    pub metadata: PassMetadata,
    /// 执行或跳过状态。
    pub status: PassStatus,
    /// 跳过原因；执行时为空。
    pub skip_reason: Option<String>,
    /// 是否改变了 IR。
    pub changed: bool,
    /// 输入快照指纹。
    pub input_fingerprint: String,
    /// 输出快照指纹；跳过时等于输入指纹。
    pub output_fingerprint: String,
    /// 该 Pass 后的验证状态。
    pub validation: ValidationStatus,
}

/// 共享优化管线的稳定报告。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OptimizationReport {
    /// 优化器实现版本。
    pub optimizer_version: u32,
    /// 规范化优化级别。
    pub level: OptimizationLevel,
    /// 配置指纹。
    pub config_fingerprint: String,
    /// 输入 IR 指纹。
    pub input_fingerprint: String,
    /// 输出 IR 指纹。
    pub output_fingerprint: String,
    /// 输入 IR 的可比较成本指标。
    pub input_cost: CostMetrics,
    /// 输出 IR 的可比较成本指标。
    pub output_cost: CostMetrics,
    /// Pass 执行报告。
    pub passes: Vec<PassReport>,
    /// 最终验证状态。
    pub validation: ValidationStatus,
}

impl OptimizationReport {
    /// 创建用于后端结构体初始化的空基线报告。
    #[must_use]
    pub fn empty() -> Self {
        Self {
            optimizer_version: OPTIMIZER_VERSION,
            level: OptimizationLevel::O0,
            config_fingerprint: String::new(),
            input_fingerprint: String::new(),
            output_fingerprint: String::new(),
            input_cost: CostMetrics::default(),
            output_cost: CostMetrics::default(),
            passes: Vec::new(),
            validation: ValidationStatus::Passed,
        }
    }
}

/// 优化管线输出。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizationResult {
    /// 已规范化并通过验证的 IR。
    pub program: IrProgram,
    /// 稳定执行报告。
    pub report: OptimizationReport,
    /// 输入、规范化和各 Pass 的回滚快照。
    pub snapshots: Vec<IrSnapshot>,
}

/// 代码尺寸、控制流、所有权和检查数量的可比较基线指标。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CostMetrics {
    /// 快照 JSON 字节数。
    pub snapshot_bytes: usize,
    /// 控制流基本块数量。
    pub control_flow_blocks: usize,
    /// 源码映射区间数量。
    pub source_mapped_spans: usize,
    /// 所有权值槽数量。
    pub ownership_values: usize,
    /// 运行时检查数量。
    pub runtime_checks: usize,
}

impl CostMetrics {
    /// 从快照和只读事实建立指标。
    #[must_use]
    pub fn from_snapshot(snapshot: &IrSnapshot, facts: &ProgramFacts) -> Self {
        Self {
            snapshot_bytes: snapshot.json.len(),
            control_flow_blocks: facts.control_flow_blocks,
            source_mapped_spans: facts.source_mapped_spans,
            ownership_values: facts.ownership.values,
            runtime_checks: facts.runtime_checks,
        }
    }
}

/// 优化管线失败；其中保留最后一份有效快照，阻止未验证结果外泄。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OptimizationError {
    /// 配置不合法。
    Config(OptimizationConfigError),
    /// 快照无法编码或解码。
    Snapshot(String),
    /// 输入或 Pass 输出没有通过 IR 验证。
    Validation {
        /// 失败阶段或 Pass 名称。
        stage: String,
        /// 稳定验证错误列表。
        errors: Vec<String>,
        /// 失败前最后一份有效快照。
        last_valid: IrSnapshot,
    },
    /// Pass 主动报告失败。
    Pass {
        /// Pass 名称。
        name: String,
        /// 开发者原因。
        message: String,
        /// 失败前最后一份有效快照。
        last_valid: IrSnapshot,
    },
}

impl Display for OptimizationError {
    /// 输出稳定的优化失败摘要。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "优化配置失败：{error}"),
            Self::Snapshot(message) => write!(formatter, "优化快照失败：{message}"),
            Self::Validation { stage, errors, .. } => {
                write!(formatter, "优化验证失败（{stage}）：{}", errors.join("；"))
            }
            Self::Pass { name, message, .. } => {
                write!(formatter, "优化 Pass {name} 失败：{message}")
            }
        }
    }
}

impl std::error::Error for OptimizationError {}

/// 共享优化管线。
pub struct OptimizationPipeline {
    config: OptimizationConfig,
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl OptimizationPipeline {
    /// 创建一条规范化配置的优化管线。
    pub fn new(config: OptimizationConfig) -> Result<Self, OptimizationError> {
        Ok(Self {
            config: config.normalize().map_err(OptimizationError::Config)?,
            passes: Vec::new(),
        })
    }

    /// 追加一个共享 Pass；Pass 顺序由调用方显式登记并进入报告。
    #[must_use]
    pub fn with_pass(mut self, pass: impl OptimizationPass + 'static) -> Self {
        self.passes.push(Box::new(pass));
        self
    }

    /// 返回当前规范化配置的只读视图。
    #[must_use]
    pub fn config(&self) -> &OptimizationConfig {
        &self.config
    }

    /// 执行规范化、验证和可选 Pass。
    pub fn run(&self, program: &IrProgram) -> Result<OptimizationResult, OptimizationError> {
        validate_program("input", program, None)?;
        let input = snapshot("input", program)?;
        let normalized = canonicalize(program)?;
        validate_program("normalized", &normalized, Some(input.clone()))?;
        let normalized_snapshot = snapshot("normalized", &normalized)?;
        let config_fingerprint = self
            .config
            .fingerprint()
            .map_err(OptimizationError::Config)?;
        let mut snapshots = vec![input.clone(), normalized_snapshot.clone()];
        let mut current = normalized;
        let initial_facts = ProgramFacts::from_program(&current);
        let input_cost = CostMetrics::from_snapshot(&normalized_snapshot, &initial_facts);
        let mut reports = vec![PassReport {
            metadata: PassMetadata {
                name: "normalize".to_owned(),
                version: 1,
                assumptions: PassAssumptions {
                    preserves_source_map: true,
                    ..PassAssumptions::default()
                },
            },
            status: PassStatus::Applied,
            skip_reason: None,
            changed: input.fingerprint != normalized_snapshot.fingerprint,
            input_fingerprint: input.fingerprint.clone(),
            output_fingerprint: normalized_snapshot.fingerprint.clone(),
            validation: ValidationStatus::Passed,
        }];

        if self.config.level != OptimizationLevel::O0
            && self.passes.is_empty()
            && self.config.pass_set.is_empty()
        {
            reports.push(PassReport {
                metadata: PassMetadata {
                    name: format!("semantic-{}-placeholder", self.config.level.as_str()),
                    version: 1,
                    assumptions: PassAssumptions::default(),
                },
                status: PassStatus::Skipped,
                skip_reason: Some(SkipReason::NotImplemented.to_string()),
                changed: false,
                input_fingerprint: normalized_snapshot.fingerprint.clone(),
                output_fingerprint: normalized_snapshot.fingerprint.clone(),
                validation: ValidationStatus::Passed,
            });
        }
        if self.passes.is_empty() {
            for name in &self.config.pass_set {
                reports.push(PassReport {
                    metadata: PassMetadata {
                        name: name.clone(),
                        version: 0,
                        assumptions: PassAssumptions::default(),
                    },
                    status: PassStatus::Skipped,
                    skip_reason: Some(SkipReason::NotImplemented.to_string()),
                    changed: false,
                    input_fingerprint: normalized_snapshot.fingerprint.clone(),
                    output_fingerprint: normalized_snapshot.fingerprint.clone(),
                    validation: ValidationStatus::Passed,
                });
            }
        }

        for pass in &self.passes {
            let metadata = pass.metadata();
            let before = snapshot(&format!("before:{}", metadata.name), &current)?;
            let mut candidate = current.clone();
            let facts = ProgramFacts::from_program(&current);
            let result = pass.run(&mut candidate, &facts);
            match result {
                PassResult::Skipped { reason } => reports.push(PassReport {
                    metadata,
                    status: PassStatus::Skipped,
                    skip_reason: Some(reason.to_string()),
                    changed: false,
                    input_fingerprint: before.fingerprint.clone(),
                    output_fingerprint: before.fingerprint.clone(),
                    validation: ValidationStatus::Passed,
                }),
                PassResult::Applied { changed } => {
                    validate_program(&metadata.name, &candidate, Some(before.clone()))?;
                    let after = snapshot(&format!("after:{}", metadata.name), &candidate)?;
                    reports.push(PassReport {
                        metadata,
                        status: PassStatus::Applied,
                        skip_reason: None,
                        changed,
                        input_fingerprint: before.fingerprint.clone(),
                        output_fingerprint: after.fingerprint.clone(),
                        validation: ValidationStatus::Passed,
                    });
                    snapshots.push(before);
                    snapshots.push(after);
                    current = candidate;
                }
                PassResult::Failed { message } => {
                    return Err(OptimizationError::Pass {
                        name: metadata.name,
                        message,
                        last_valid: before,
                    });
                }
            }
        }
        let output = snapshot("output", &current)?;
        let output_facts = ProgramFacts::from_program(&current);
        let output_cost = CostMetrics::from_snapshot(&output, &output_facts);
        snapshots.push(output.clone());
        Ok(OptimizationResult {
            program: current,
            report: OptimizationReport {
                optimizer_version: OPTIMIZER_VERSION,
                level: self.config.level,
                config_fingerprint: config_fingerprint.as_str().to_owned(),
                input_fingerprint: input.fingerprint,
                output_fingerprint: output.fingerprint,
                input_cost,
                output_cost,
                passes: reports,
                validation: ValidationStatus::Passed,
            },
            snapshots,
        })
    }
}

/// 执行不带语义 Pass 的 O0 完整规范化和验证管线。
pub fn run_baseline(
    program: &IrProgram,
    target: impl Into<String>,
) -> Result<OptimizationResult, OptimizationError> {
    OptimizationPipeline::new(OptimizationConfig::baseline(target))?.run(program)
}

fn canonicalize(program: &IrProgram) -> Result<IrProgram, OptimizationError> {
    let json = to_json(program).map_err(snapshot_error)?;
    from_json(&json).map_err(snapshot_error)
}

fn snapshot(label: &str, program: &IrProgram) -> Result<IrSnapshot, OptimizationError> {
    let json = to_json(program).map_err(snapshot_error)?;
    let fingerprint = stable_hash(json.as_bytes());
    Ok(IrSnapshot {
        label: label.to_owned(),
        json,
        fingerprint,
    })
}

fn validate_program(
    stage: &str,
    program: &IrProgram,
    last_valid: Option<IrSnapshot>,
) -> Result<(), OptimizationError> {
    let validation = program.validate();
    if validation.is_success() {
        return Ok(());
    }
    let fallback = match last_valid {
        Some(snapshot) => snapshot,
        None => snapshot(stage, program)?,
    };
    Err(OptimizationError::Validation {
        stage: stage.to_owned(),
        errors: validation.errors.iter().map(ToString::to_string).collect(),
        last_valid: fallback,
    })
}

fn snapshot_error(error: SnapshotError) -> OptimizationError {
    OptimizationError::Snapshot(error.to_string())
}

fn stable_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("xiao-ir-fnv1a64-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use xiao_ir::{IrEntryMode, IrSpan};

    fn empty_program() -> IrProgram {
        IrProgram::new(IrEntryMode::Script, Vec::new(), IrSpan::new(0, 0))
    }

    struct NoopPass;

    impl OptimizationPass for NoopPass {
        fn metadata(&self) -> PassMetadata {
            PassMetadata {
                name: "test.noop".to_owned(),
                version: 1,
                assumptions: PassAssumptions {
                    preserves_source_map: true,
                    ..PassAssumptions::default()
                },
            }
        }

        fn run(&self, _program: &mut IrProgram, _facts: &ProgramFacts) -> PassResult {
            PassResult::Applied { changed: false }
        }
    }

    struct InvalidPass;

    impl OptimizationPass for InvalidPass {
        fn metadata(&self) -> PassMetadata {
            PassMetadata {
                name: "test.invalid".to_owned(),
                version: 1,
                assumptions: PassAssumptions::default(),
            }
        }

        fn run(&self, program: &mut IrProgram, _facts: &ProgramFacts) -> PassResult {
            program.version = 999;
            PassResult::Applied { changed: true }
        }
    }

    struct FailingPass;

    impl OptimizationPass for FailingPass {
        fn metadata(&self) -> PassMetadata {
            PassMetadata {
                name: "test.failing".to_owned(),
                version: 1,
                assumptions: PassAssumptions::default(),
            }
        }

        fn run(&self, _program: &mut IrProgram, _facts: &ProgramFacts) -> PassResult {
            PassResult::Failed {
                message: "测试失败".to_owned(),
            }
        }
    }

    #[test]
    fn normalizes_config_and_fingerprint_independently_of_input_order() {
        let first = OptimizationConfig::baseline("elf")
            .with_module_graph(["b", "a", "a"])
            .with_dependency_lock(["z", "y"]);
        let second = OptimizationConfig::baseline("elf")
            .with_module_graph(["a", "b"])
            .with_dependency_lock(["y", "z"]);
        assert_eq!(
            first.fingerprint().expect("指纹").as_str(),
            second.fingerprint().expect("指纹").as_str()
        );
    }

    #[test]
    fn o0_runs_normalization_and_validation_without_semantic_pass() {
        let result = run_baseline(&empty_program(), "elf").expect("O0 管线");
        assert_eq!(result.report.level, OptimizationLevel::O0);
        assert_eq!(result.report.validation, ValidationStatus::Passed);
        assert_eq!(result.report.passes.len(), 1);
        assert_eq!(result.snapshots.last().expect("输出快照").label, "output");
    }

    #[test]
    fn placeholder_level_reports_skip_instead_of_claiming_optimization() {
        let config = OptimizationConfig::baseline("elf").with_level(OptimizationLevel::O1);
        let result = OptimizationPipeline::new(config)
            .expect("配置")
            .run(&empty_program())
            .expect("占位管线");
        assert_eq!(result.report.passes[1].status, PassStatus::Skipped);
        assert_eq!(
            result.report.passes[1].skip_reason.as_deref(),
            Some("not-implemented")
        );
        assert_eq!(
            result.report.input_fingerprint,
            result.report.output_fingerprint
        );
    }

    #[test]
    fn pass_output_is_validated_and_failure_keeps_last_valid_snapshot() {
        let result = OptimizationPipeline::new(OptimizationConfig::baseline("elf"))
            .expect("配置")
            .with_pass(InvalidPass)
            .run(&empty_program())
            .expect_err("非法 Pass 必须阻断");
        let OptimizationError::Validation { last_valid, .. } = result else {
            panic!("应返回验证错误");
        };
        assert_eq!(last_valid.label, "before:test.invalid");
    }

    #[test]
    fn explicit_pass_failure_is_blocked_with_last_valid_snapshot() {
        let result = OptimizationPipeline::new(OptimizationConfig::baseline("elf"))
            .expect("配置")
            .with_pass(FailingPass)
            .run(&empty_program())
            .expect_err("Pass 失败必须阻断");
        let OptimizationError::Pass {
            name,
            message,
            last_valid,
        } = result
        else {
            panic!("应返回 Pass 失败");
        };
        assert_eq!(name, "test.failing");
        assert_eq!(message, "测试失败");
        assert_eq!(last_valid.label, "before:test.failing");
    }

    #[test]
    fn pass_can_read_facts_without_mutating_them() {
        let result = OptimizationPipeline::new(OptimizationConfig::baseline("elf"))
            .expect("配置")
            .with_pass(NoopPass)
            .run(&empty_program())
            .expect("只读事实 Pass");
        assert_eq!(result.report.passes[1].metadata.name, "test.noop");
    }
}
