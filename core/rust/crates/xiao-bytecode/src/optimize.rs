//! 字节码级优化管线与保守 Pass。
//!
//! 本模块是 14B 的消费端：配置、Pass 假设、跳过原因和报告枚举全部复用
//! `xiao-optimizer` 的 13A 契约。每次 Pass 都保存前后快照，并在提交候选
//! 程序前重新进行结构编码/解码验证；提供 [`BytecodeOptimizationPipeline::run_checked`]
//! 时还会用输入 IR 对账 TAC 的释放计划与错误路径。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

use xiao_ir::IrProgram;
use xiao_optimizer::{
    OptimizationConfig, OptimizationError as IrOptimizationError, OptimizationLevel,
    PassAssumptions, PassMetadata, PassReport, PassResult, PassStatus, SkipReason,
    ValidationStatus,
};

use crate::cfg::{protected_successors, successors};
use crate::encode::{EncodeOptions, OperandWidth, decode, encode};
use crate::liveness::instruction_use_def;
use crate::tac::{
    BlockId, CategoryMap, ConstId, ConstPool, TacConstant, TacFunction, TacOp, TacProgram, VReg,
};
use crate::{TacVerification, verify_program};

/// 字节码 Pass 运行时可读取的只读规模事实。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BytecodeFacts {
    /// 函数数量。
    pub functions: usize,
    /// 基本块数量。
    pub blocks: usize,
    /// 指令数量。
    pub instructions: usize,
    /// 常量池条目数量。
    pub constants: usize,
    /// 所有函数中最大的局部寄存器编号加一。
    pub register_slots: usize,
}

impl BytecodeFacts {
    fn from_program(program: &TacProgram) -> Self {
        let mut blocks = 0;
        let mut instructions = 0;
        let mut register_slots = 0;
        for function in &program.functions {
            blocks += function.blocks.len();
            for block in &function.blocks {
                instructions += block.instructions.len();
                for instruction in &block.instructions {
                    for register in instruction_registers(instruction) {
                        register_slots = register_slots.max(register.get() as usize + 1);
                    }
                }
            }
        }
        Self {
            functions: program.functions.len(),
            blocks,
            instructions,
            constants: program.constants.len(),
            register_slots,
        }
    }
}

/// 字节码 Pass 的运行结果。
pub trait BytecodeOptimizationPass: Send + Sync {
    /// 返回 13A 兼容的 Pass 身份和假设。
    fn metadata(&self) -> PassMetadata;
    /// 在候选 TAC 上运行 Pass；不得修改 `facts` 或伪造释放计划。
    fn run(&self, program: &mut TacProgram, facts: &BytecodeFacts) -> PassResult;
}

/// 一份可比较的字节码快照。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BytecodeSnapshot {
    /// 快照阶段名称。
    pub label: String,
    /// 稳定快照指纹。
    pub fingerprint: String,
    /// 规范内存编码字节；用于失败回滚和审计。
    pub bytes: Vec<u8>,
}

/// 优化后的字节码和逐 Pass 报告。
#[derive(Clone, Debug, PartialEq)]
pub struct BytecodeOptimizationResult {
    /// 优化后、已通过验证的 TAC。
    pub program: TacProgram,
    /// 未优化参考程序；差分和失败回退都使用它。
    pub reference: TacProgram,
    /// 13A 兼容的配置、Pass 和验证报告。
    pub report: BytecodeOptimizationReport,
    /// 输入、每个 Pass 前后及输出快照。
    pub snapshots: Vec<BytecodeSnapshot>,
}

/// 字节码优化报告。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BytecodeOptimizationReport {
    /// 优化器实现版本；与 13A 共用版本号。
    pub optimizer_version: u32,
    /// 规范化优化级别。
    pub level: OptimizationLevel,
    /// 13A 配置指纹。
    pub config_fingerprint: String,
    /// 输入程序快照指纹。
    pub input_fingerprint: String,
    /// 输出程序快照指纹。
    pub output_fingerprint: String,
    /// 每个 Pass 的前后状态。
    pub passes: Vec<PassReport>,
    /// 最终是否通过验证。
    pub validation: ValidationStatus,
}

/// 字节码优化失败；其中始终保留未优化参考和最后有效快照。
#[derive(Clone, Debug, PartialEq)]
pub enum BytecodeOptimizationError {
    /// 13A 配置不合法。
    Config(Box<IrOptimizationError>),
    /// 内存编码、解码或结构校验失败。
    Encoding(String),
    /// 输入 IR/TAC 或优化结果验证失败。
    Validation {
        /// 失败阶段。
        stage: String,
        /// 稳定验证错误。
        errors: Vec<String>,
        /// 未优化参考。
        reference: Box<TacProgram>,
        /// 失败前最后有效快照。
        last_valid: Box<BytecodeSnapshot>,
    },
    /// Pass 主动失败。
    Pass {
        /// Pass 名称。
        name: String,
        /// 开发者原因。
        message: String,
        /// 未优化参考。
        reference: Box<TacProgram>,
        /// 失败前最后有效快照。
        last_valid: Box<BytecodeSnapshot>,
    },
}

impl Display for BytecodeOptimizationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "字节码优化配置失败：{error}"),
            Self::Encoding(message) => write!(formatter, "字节码优化编码失败：{message}"),
            Self::Validation { stage, errors, .. } => {
                write!(
                    formatter,
                    "字节码优化验证失败（{stage}）：{}",
                    errors.join("；")
                )
            }
            Self::Pass { name, message, .. } => {
                write!(formatter, "字节码 Pass {name} 失败：{message}")
            }
        }
    }
}

impl std::error::Error for BytecodeOptimizationError {}

/// 字节码优化管线。
pub struct BytecodeOptimizationPipeline {
    config: OptimizationConfig,
    passes: Vec<Box<dyn BytecodeOptimizationPass>>,
    operand_width: OperandWidth,
}

impl BytecodeOptimizationPipeline {
    /// 创建使用规范化 13A 配置的管线。
    pub fn new(config: OptimizationConfig) -> Result<Self, BytecodeOptimizationError> {
        Ok(Self {
            config: config.normalize().map_err(|error| {
                BytecodeOptimizationError::Config(Box::new(IrOptimizationError::Config(error)))
            })?,
            passes: Vec::new(),
            operand_width: OperandWidth::Leb128,
        })
    }

    /// 添加一个自定义字节码 Pass。
    #[must_use]
    pub fn with_pass(mut self, pass: impl BytecodeOptimizationPass + 'static) -> Self {
        self.passes.push(Box::new(pass));
        self
    }

    /// 注册本批已实现的保守 Pass。
    #[must_use]
    pub fn with_standard_passes(mut self) -> Self {
        let requested = self.config.pass_set.clone();
        let names = if requested.is_empty() {
            vec![
                "bytecode.constant-pool",
                "bytecode.redundant-move",
                "bytecode.jump-simplify",
                "bytecode.unreachable-block",
                "bytecode.slot-layout",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
        } else {
            requested
        };
        for name in names {
            match name.as_str() {
                "bytecode.constant-pool" | "constant-pool" => {
                    self.passes.push(Box::new(ConstantPoolPass));
                }
                "bytecode.redundant-move" | "redundant-move" => {
                    self.passes.push(Box::new(RedundantMovePass));
                }
                "bytecode.jump-simplify" | "jump-simplify" => {
                    self.passes.push(Box::new(JumpSimplifyPass));
                }
                "bytecode.unreachable-block" | "unreachable-block" => {
                    self.passes.push(Box::new(UnreachableBlockPass));
                }
                "bytecode.slot-layout" | "slot-layout" => {
                    self.passes.push(Box::new(SlotLayoutPass));
                }
                _ => self.passes.push(Box::new(UnknownPass { name })),
            }
        }
        self
    }

    /// 设置规范内存编码使用的操作数宽度。
    #[must_use]
    pub const fn with_operand_width(mut self, width: OperandWidth) -> Self {
        self.operand_width = width;
        self
    }

    /// 执行只依赖 TAC 结构的优化与编码验证。
    pub fn run(
        &self,
        program: &TacProgram,
    ) -> Result<BytecodeOptimizationResult, BytecodeOptimizationError> {
        self.run_inner(program, None)
    }

    /// 执行优化，并在每个 Pass 后用来源 IR 对账释放计划和错误路径。
    pub fn run_checked(
        &self,
        ir: &IrProgram,
        program: &TacProgram,
    ) -> Result<BytecodeOptimizationResult, BytecodeOptimizationError> {
        self.run_inner(program, Some(ir))
    }

    fn run_inner(
        &self,
        program: &TacProgram,
        ir: Option<&IrProgram>,
    ) -> Result<BytecodeOptimizationResult, BytecodeOptimizationError> {
        let reference = program.clone();
        let input = snapshot("input", &reference, self.operand_width)?;
        validate_candidate(
            "input",
            &reference,
            self.operand_width,
            ir,
            None,
            &reference,
            &input,
        )?;
        let config_fingerprint = self.config.fingerprint().map_err(|error| {
            BytecodeOptimizationError::Config(Box::new(IrOptimizationError::Config(error)))
        })?;
        let mut current = reference.clone();
        let mut snapshots = vec![input.clone()];
        let mut reports = Vec::new();
        if self.config.level != OptimizationLevel::O0 && self.passes.is_empty() {
            let after = snapshot(
                "after:bytecode.semantic-placeholder",
                &current,
                self.operand_width,
            )?;
            snapshots.push(after.clone());
            reports.push(pass_report(
                PassMetadata {
                    name: "bytecode.semantic-placeholder".to_owned(),
                    version: 0,
                    assumptions: PassAssumptions {
                        preserves_source_map: true,
                        ..PassAssumptions::default()
                    },
                },
                PassStatus::Skipped,
                Some(SkipReason::NotImplemented.to_string()),
                false,
                &input,
                &after,
            ));
        }
        for pass in &self.passes {
            let metadata = pass.metadata();
            let before = snapshot(
                &format!("before:{}", metadata.name),
                &current,
                self.operand_width,
            )?;
            snapshots.push(before.clone());
            let facts = BytecodeFacts::from_program(&current);
            let mut candidate = current.clone();
            let result = if self.config.level == OptimizationLevel::O0 {
                PassResult::Skipped {
                    reason: SkipReason::OptimizationDisabled,
                }
            } else {
                pass.run(&mut candidate, &facts)
            };
            match result {
                PassResult::Skipped { reason } => {
                    let after = snapshot(
                        &format!("after:{}", metadata.name),
                        &current,
                        self.operand_width,
                    )?;
                    snapshots.push(after.clone());
                    reports.push(pass_report(
                        metadata,
                        PassStatus::Skipped,
                        Some(reason.to_string()),
                        false,
                        &before,
                        &after,
                    ));
                }
                PassResult::Applied { changed } => {
                    validate_candidate(
                        &metadata.name,
                        &candidate,
                        self.operand_width,
                        ir,
                        Some(before.clone()),
                        &reference,
                        &before,
                    )?;
                    let after = snapshot(
                        &format!("after:{}", metadata.name),
                        &candidate,
                        self.operand_width,
                    )?;
                    snapshots.push(after.clone());
                    reports.push(pass_report(
                        metadata,
                        PassStatus::Applied,
                        None,
                        changed,
                        &before,
                        &after,
                    ));
                    current = candidate;
                }
                PassResult::Failed { message } => {
                    return Err(BytecodeOptimizationError::Pass {
                        name: metadata.name,
                        message,
                        reference: Box::new(reference),
                        last_valid: Box::new(before),
                    });
                }
            }
        }
        let output = snapshot("output", &current, self.operand_width)?;
        snapshots.push(output.clone());
        Ok(BytecodeOptimizationResult {
            program: current,
            reference,
            report: BytecodeOptimizationReport {
                optimizer_version: xiao_optimizer::OPTIMIZER_VERSION,
                level: self.config.level,
                config_fingerprint: config_fingerprint.as_str().to_owned(),
                input_fingerprint: input.fingerprint,
                output_fingerprint: output.fingerprint,
                passes: reports,
                validation: ValidationStatus::Passed,
            },
            snapshots,
        })
    }
}

/// 使用标准 Pass 执行字节码优化的便捷入口。
pub fn optimize_bytecode(
    program: &TacProgram,
    config: OptimizationConfig,
) -> Result<BytecodeOptimizationResult, BytecodeOptimizationError> {
    BytecodeOptimizationPipeline::new(config)?
        .with_standard_passes()
        .run(program)
}

/// 使用来源 IR 执行字节码优化的便捷入口。
pub fn optimize_bytecode_checked(
    ir: &IrProgram,
    program: &TacProgram,
    config: OptimizationConfig,
) -> Result<BytecodeOptimizationResult, BytecodeOptimizationError> {
    BytecodeOptimizationPipeline::new(config)?
        .with_standard_passes()
        .run_checked(ir, program)
}

fn snapshot(
    label: &str,
    program: &TacProgram,
    width: OperandWidth,
) -> Result<BytecodeSnapshot, BytecodeOptimizationError> {
    let encoded = encode(
        program,
        EncodeOptions {
            operand_width: width,
        },
    )
    .map_err(|error| BytecodeOptimizationError::Encoding(error.to_string()))?;
    Ok(BytecodeSnapshot {
        label: label.to_owned(),
        fingerprint: stable_hash(&encoded.bytes),
        bytes: encoded.bytes,
    })
}

fn validate_candidate(
    stage: &str,
    program: &TacProgram,
    width: OperandWidth,
    ir: Option<&IrProgram>,
    _last_valid: Option<BytecodeSnapshot>,
    reference: &TacProgram,
    snapshot: &BytecodeSnapshot,
) -> Result<(), BytecodeOptimizationError> {
    let encoded = encode(
        program,
        EncodeOptions {
            operand_width: width,
        },
    )
    .map_err(|error| BytecodeOptimizationError::Encoding(error.to_string()))?;
    let decoded = decode(&encoded.bytes)
        .map_err(|error| BytecodeOptimizationError::Encoding(error.to_string()))?;
    let reencoded = encode(
        &decoded,
        EncodeOptions {
            operand_width: width,
        },
    )
    .map_err(|error| BytecodeOptimizationError::Encoding(error.to_string()))?;
    if reencoded.bytes != encoded.bytes {
        return Err(BytecodeOptimizationError::Validation {
            stage: stage.to_owned(),
            errors: vec!["编码/解码结果不是规范字节序列".to_owned()],
            reference: Box::new(reference.clone()),
            last_valid: Box::new(snapshot.clone()),
        });
    }
    if let Some(ir) = ir {
        let verification = verify_program(ir, program);
        if !verification.is_success() {
            return Err(BytecodeOptimizationError::Validation {
                stage: stage.to_owned(),
                errors: verification_errors(&verification),
                reference: Box::new(reference.clone()),
                last_valid: Box::new(snapshot.clone()),
            });
        }
    }
    Ok(())
}

fn verification_errors(verification: &TacVerification) -> Vec<String> {
    let mut errors = verification.errors.clone();
    errors.extend(verification.unsupported.iter().cloned());
    errors.extend(verification.internal_errors.iter().map(ToString::to_string));
    errors
}

fn pass_report(
    metadata: PassMetadata,
    status: PassStatus,
    skip_reason: Option<String>,
    changed: bool,
    before: &BytecodeSnapshot,
    after: &BytecodeSnapshot,
) -> PassReport {
    PassReport {
        metadata,
        status,
        skip_reason,
        changed,
        input_fingerprint: before.fingerprint.clone(),
        output_fingerprint: after.fingerprint.clone(),
        validation: ValidationStatus::Passed,
    }
}

fn stable_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("xiao-bytecode-fnv1a64-{hash:016x}")
}

struct UnknownPass {
    name: String,
}

impl BytecodeOptimizationPass for UnknownPass {
    fn metadata(&self) -> PassMetadata {
        PassMetadata {
            name: self.name.clone(),
            version: 0,
            assumptions: PassAssumptions {
                preserves_source_map: true,
                ..PassAssumptions::default()
            },
        }
    }

    fn run(&self, _program: &mut TacProgram, _facts: &BytecodeFacts) -> PassResult {
        PassResult::Skipped {
            reason: SkipReason::NotImplemented,
        }
    }
}

/// 常量池去重 Pass。
pub struct ConstantPoolPass;

/// 常量池去重 Pass 的语义别名。
pub type ConstantPoolDedupPass = ConstantPoolPass;

impl BytecodeOptimizationPass for ConstantPoolPass {
    fn metadata(&self) -> PassMetadata {
        PassMetadata {
            name: "bytecode.constant-pool".to_owned(),
            version: 1,
            assumptions: PassAssumptions {
                preserves_source_map: true,
                ..PassAssumptions::default()
            },
        }
    }

    fn run(&self, program: &mut TacProgram, _facts: &BytecodeFacts) -> PassResult {
        let entries = program.constants.iter().cloned().collect::<Vec<_>>();
        let mut unique = Vec::new();
        let mut remap = Vec::with_capacity(entries.len());
        for constant in entries {
            let index = unique
                .iter()
                .position(|candidate| constants_equal(candidate, &constant))
                .unwrap_or_else(|| {
                    unique.push(constant.clone());
                    unique.len() - 1
                });
            remap.push(ConstId::new(index as u32));
        }
        let changed = unique.len() != program.constants.len();
        if !changed {
            return PassResult::Skipped {
                reason: SkipReason::NoBenefit,
            };
        }
        program.constants = ConstPool::from_entries(unique);
        for function in &mut program.functions {
            for block in &mut function.blocks {
                for instruction in &mut block.instructions {
                    if let TacOp::LoadConst(id) = &mut instruction.op {
                        if let Some(mapped) = remap.get(id.get() as usize) {
                            *id = *mapped;
                        }
                    }
                }
            }
        }
        PassResult::Applied { changed: true }
    }
}

/// 无副作用自移动删除 Pass。
pub struct RedundantMovePass;

impl BytecodeOptimizationPass for RedundantMovePass {
    fn metadata(&self) -> PassMetadata {
        PassMetadata {
            name: "bytecode.redundant-move".to_owned(),
            version: 1,
            assumptions: PassAssumptions {
                preserves_source_map: true,
                ..PassAssumptions::default()
            },
        }
    }

    fn run(&self, program: &mut TacProgram, _facts: &BytecodeFacts) -> PassResult {
        let mut changed = false;
        for function in &mut program.functions {
            for block in &mut function.blocks {
                let before = block.instructions.len();
                block.instructions.retain(|instruction| {
                    let redundant = matches!(
                        (&instruction.op, instruction.dst),
                        (TacOp::Move(source), Some(dst)) if *source == dst
                    );
                    if redundant {
                        changed = true;
                    }
                    !redundant
                });
                debug_assert!(block.instructions.len() <= before);
            }
        }
        if changed {
            PassResult::Applied { changed: true }
        } else {
            PassResult::Skipped {
                reason: SkipReason::NoBenefit,
            }
        }
    }
}

/// 等目标条件分支简化 Pass。
pub struct JumpSimplifyPass;

impl BytecodeOptimizationPass for JumpSimplifyPass {
    fn metadata(&self) -> PassMetadata {
        PassMetadata {
            name: "bytecode.jump-simplify".to_owned(),
            version: 1,
            assumptions: PassAssumptions {
                changes_control_flow: true,
                preserves_source_map: true,
                ..PassAssumptions::default()
            },
        }
    }

    fn run(&self, program: &mut TacProgram, _facts: &BytecodeFacts) -> PassResult {
        let mut changed = false;
        for function in &mut program.functions {
            for block in &mut function.blocks {
                for instruction in &mut block.instructions {
                    let replacement = match instruction.op {
                        TacOp::BranchIf {
                            if_true, if_false, ..
                        } if if_true == if_false => Some(TacOp::Jump(if_true)),
                        _ => None,
                    };
                    if let Some(op) = replacement {
                        instruction.op = op;
                        instruction.dst = None;
                        changed = true;
                    }
                }
            }
        }
        if changed {
            PassResult::Applied { changed: true }
        } else {
            PassResult::Skipped {
                reason: SkipReason::NoBenefit,
            }
        }
    }
}

/// 不可达块 Pass；当前证明不足时会稳定跳过。
pub struct UnreachableBlockPass;

impl BytecodeOptimizationPass for UnreachableBlockPass {
    fn metadata(&self) -> PassMetadata {
        PassMetadata {
            name: "bytecode.unreachable-block".to_owned(),
            version: 1,
            assumptions: PassAssumptions {
                changes_control_flow: true,
                preserves_source_map: true,
                ..PassAssumptions::default()
            },
        }
    }

    fn run(&self, program: &mut TacProgram, _facts: &BytecodeFacts) -> PassResult {
        let mut changed = false;
        for function in &mut program.functions {
            changed |= remove_unreachable_blocks(function);
        }
        if changed {
            PassResult::Applied { changed: true }
        } else {
            PassResult::Skipped {
                reason: SkipReason::NoBenefit,
            }
        }
    }
}

/// 槽布局 Pass；当前证明不足时会稳定跳过。
pub struct SlotLayoutPass;

impl BytecodeOptimizationPass for SlotLayoutPass {
    fn metadata(&self) -> PassMetadata {
        PassMetadata {
            name: "bytecode.slot-layout".to_owned(),
            version: 1,
            assumptions: PassAssumptions {
                reads_ownership: true,
                preserves_source_map: true,
                ..PassAssumptions::default()
            },
        }
    }

    fn run(&self, program: &mut TacProgram, _facts: &BytecodeFacts) -> PassResult {
        let mut changed = false;
        for (index, function) in program.functions.iter_mut().enumerate() {
            let function_changed = compact_registers(function, &program.plans);
            if index == 0 && function_changed {
                program.categories = function.categories.clone();
            }
            changed |= function_changed;
        }
        if changed {
            PassResult::Applied { changed: true }
        } else {
            PassResult::Skipped {
                reason: SkipReason::NoBenefit,
            }
        }
    }
}

fn remove_unreachable_blocks(function: &mut TacFunction) -> bool {
    if function.blocks.is_empty() {
        return false;
    }
    let mut graph = successors(function);
    protected_successors(function, &mut graph);
    let mut reachable = BTreeSet::new();
    let mut pending = vec![function.entry];
    while let Some(block) = pending.pop() {
        if !reachable.insert(block) {
            continue;
        }
        if let Some(targets) = graph.get(&block) {
            pending.extend(targets.iter().copied());
        }
    }
    if reachable.len() == function.blocks.len() {
        return false;
    }
    let mut remap = BTreeMap::new();
    let mut next = 0_u32;
    for block in &function.blocks {
        if reachable.contains(&block.id) {
            remap.insert(block.id, BlockId::new(next));
            next += 1;
        }
    }
    function
        .blocks
        .retain(|block| reachable.contains(&block.id));
    for block in &mut function.blocks {
        let old_id = block.id;
        if let Some(new_id) = remap.get(&old_id).copied() {
            block.id = new_id;
            for instruction in &mut block.instructions {
                remap_block_targets(&mut instruction.op, &remap);
            }
        }
    }
    function.entry = remap[&function.entry];
    for handler in &mut function.handlers {
        let old_start = handler.protected.0;
        let old_end = handler.protected.1;
        handler.protected = (
            BlockId::new(reachable.iter().filter(|id| **id < old_start).count() as u32),
            BlockId::new(reachable.iter().filter(|id| **id < old_end).count() as u32),
        );
        handler.handler = remap[&handler.handler];
    }
    true
}

fn remap_block_targets(op: &mut TacOp, remap: &BTreeMap<BlockId, BlockId>) {
    match op {
        TacOp::Jump(target) | TacOp::CallSub { sub: target } => {
            if let Some(mapped) = remap.get(target) {
                *target = *mapped;
            }
        }
        TacOp::BranchIf {
            if_true, if_false, ..
        } => {
            if let Some(mapped) = remap.get(if_true) {
                *if_true = *mapped;
            }
            if let Some(mapped) = remap.get(if_false) {
                *if_false = *mapped;
            }
        }
        TacOp::Check { on_failure, .. } => {
            if let Some(mapped) = remap.get(on_failure) {
                *on_failure = *mapped;
            }
        }
        _ => {}
    }
}

fn compact_registers(function: &mut TacFunction, plans: &[crate::lower::TacReleasePlan]) -> bool {
    let mut registers = BTreeSet::new();
    registers.extend(function.parameters.iter().copied());
    registers.extend(function.locals.iter().copied());
    registers.extend(function.value_registers.values().copied());
    for block in &function.blocks {
        for instruction in &block.instructions {
            let (uses, defs) = instruction_use_def(function, instruction, plans);
            registers.extend(uses);
            registers.extend(defs);
        }
    }
    let remap = registers
        .iter()
        .enumerate()
        .map(|(index, register)| (*register, VReg::new(index as u32)))
        .collect::<BTreeMap<_, _>>();
    if remap.iter().all(|(old, new)| old == new) {
        return false;
    }
    for register in &mut function.parameters {
        *register = remap[register];
    }
    for register in &mut function.locals {
        *register = remap[register];
    }
    for register in function.value_registers.values_mut() {
        *register = remap[register];
    }
    let old_categories = function.categories.iter().collect::<Vec<_>>();
    let mut categories = CategoryMap::new();
    for (old, class) in old_categories.into_iter().enumerate() {
        if let Some(mapped) = remap.get(&VReg::new(old as u32)) {
            categories.insert(*mapped, class);
        }
    }
    function.categories = categories;
    for handler in &mut function.handlers {
        if let Some(binding) = handler.binding.as_mut() {
            *binding = remap[binding];
        }
    }
    for block in &mut function.blocks {
        for instruction in &mut block.instructions {
            if let Some(dst) = instruction.dst.as_mut() {
                *dst = remap[dst];
            }
            remap_registers(&mut instruction.op, &remap);
        }
    }
    true
}

fn remap_registers(op: &mut TacOp, remap: &BTreeMap<VReg, VReg>) {
    let map = |register: &mut VReg| *register = remap[register];
    match op {
        TacOp::LoadConst(_)
        | TacOp::LoadNone
        | TacOp::LoadFunc(_)
        | TacOp::PackageRoot(_)
        | TacOp::ImportModule { .. }
        | TacOp::Jump(_)
        | TacOp::Return { value: None }
        | TacOp::RetFromSub
        | TacOp::RunReleasePlan { .. }
        | TacOp::EnterScope(_)
        | TacOp::ExitScope { .. } => {}
        TacOp::ExportValue { value, .. }
        | TacOp::Move(value)
        | TacOp::Copy(value)
        | TacOp::Box(value)
        | TacOp::Unbox(value)
        | TacOp::Release { value, .. }
        | TacOp::Transfer { value }
        | TacOp::Cast { value, .. }
        | TacOp::Len { source: value } => map(value),
        TacOp::Arith { left, right, .. }
        | TacOp::Compare { left, right, .. }
        | TacOp::SetOp { left, right, .. }
        | TacOp::SetCompare { left, right, .. } => {
            map(left);
            map(right);
        }
        TacOp::NewArray { elements }
        | TacOp::NewTuple { elements }
        | TacOp::NewSet { elements } => {
            elements.iter_mut().for_each(map);
        }
        TacOp::NewDictTable { entries } | TacOp::NewDictColumn { entries } => {
            entries.iter_mut().for_each(|(_, value)| map(value));
        }
        TacOp::IndexGet { source, .. } => map(source),
        TacOp::SelectorApply {
            source,
            step,
            random_counts,
            ..
        } => {
            map(source);
            if let Some(step) = step {
                map(step);
            }
            random_counts.iter_mut().flatten().for_each(map);
        }
        TacOp::BroadcastAssign { root, value, .. } => {
            map(root);
            map(value);
        }
        TacOp::RandomSeed { value, .. } => map(value),
        TacOp::BranchIf { condition, .. } => map(condition),
        TacOp::Call { arguments, .. } | TacOp::CallDynamic { arguments, .. } => {
            arguments
                .iter_mut()
                .for_each(|argument| map(&mut argument.value));
            if let TacOp::CallDynamic { callee, .. } = op {
                map(callee);
            }
        }
        TacOp::Return { value: Some(value) } | TacOp::Raise { value } => map(value),
        TacOp::MakeError { code, message, .. } => {
            if let Some(code) = code {
                map(code);
            }
            if let Some(message) = message {
                map(message);
            }
        }
        TacOp::Check { value, .. } => map(value),
        TacOp::IndexGetDynamic { source, index } => {
            map(source);
            map(index);
        }
        TacOp::LoadTable { arguments, .. } => {
            arguments
                .iter_mut()
                .for_each(|argument| map(&mut argument.value));
        }
        TacOp::MemberGet { object, .. } => map(object),
        TacOp::MemberSet { object, value, .. } => {
            map(object);
            map(value);
        }
        TacOp::CallSub { .. } => {}
    }
}

fn constants_equal(left: &TacConstant, right: &TacConstant) -> bool {
    match (left, right) {
        (TacConstant::Float(left), TacConstant::Float(right)) => left.to_bits() == right.to_bits(),
        (TacConstant::Sfloat(left), TacConstant::Sfloat(right)) => {
            left.to_bits() == right.to_bits()
        }
        _ => left == right,
    }
}

fn instruction_registers(instruction: &crate::tac::TacInstr) -> Vec<crate::tac::VReg> {
    instruction.dst.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tac::{
        BlockId, CategoryMap, RegisterClass, TacAbi, TacBlock, TacFunction, TacInstr, VReg,
    };
    use xiao_ir::IR_VERSION;

    fn program_with_duplicate_constants() -> TacProgram {
        let mut constants = ConstPool::new();
        let first = constants.intern(TacConstant::Int(7));
        let second = constants.intern(TacConstant::Int(7));
        let _ = second;
        let mut categories = CategoryMap::new();
        categories.insert(VReg::new(0), RegisterClass::Int);
        TacProgram {
            version: crate::TAC_VERSION,
            abi: TacAbi {
                bytecode_abi_version: crate::TAC_BYTECODE_ABI_VERSION,
                runtime_abi_version: crate::TAC_RUNTIME_ABI_VERSION,
                ir_version: IR_VERSION,
                language_version: "0.1.0".to_owned(),
                target: "portable".to_owned(),
            },
            constants,
            signatures: Default::default(),
            functions: vec![TacFunction {
                name: String::new(),
                signature: None,
                entry: BlockId::new(0),
                blocks: vec![TacBlock {
                    id: BlockId::new(0),
                    scope: 0,
                    instructions: vec![TacInstr::with_dst(
                        TacOp::LoadConst(first),
                        VReg::new(0),
                        xiao_ir::IrSpan::new(0, 1),
                    )],
                }],
                parameters: Vec::new(),
                locals: vec![VReg::new(0)],
                categories,
                scopes: vec![0],
                handlers: Vec::new(),
                value_registers: std::collections::BTreeMap::new(),
                span: xiao_ir::IrSpan::new(0, 1),
            }],
            categories: CategoryMap::new(),
            plans: Vec::new(),
            selection_plans: Vec::new(),
            broadcast_assignment_plans: Vec::new(),
            random_seed_plans: Vec::new(),
            table_definitions: Vec::new(),
            unsupported: Vec::new(),
        }
    }

    #[test]
    fn standard_passes_report_snapshots_and_skip_reasons() {
        let result = optimize_bytecode(
            &program_with_duplicate_constants(),
            OptimizationConfig::baseline("portable").with_level(OptimizationLevel::O1),
        )
        .expect("字节码优化");
        assert!(
            result
                .report
                .passes
                .iter()
                .any(|pass| pass.metadata.name == "bytecode.constant-pool")
        );
        assert!(
            result
                .snapshots
                .iter()
                .any(|snapshot| snapshot.label == "after:bytecode.constant-pool")
        );
        assert!(
            result
                .report
                .passes
                .iter()
                .any(|pass| pass.status == PassStatus::Skipped)
        );
    }

    #[test]
    fn o0_skips_semantic_passes_without_claiming_optimization() {
        let result = optimize_bytecode(
            &program_with_duplicate_constants(),
            OptimizationConfig::baseline("portable"),
        )
        .expect("O0");
        assert_eq!(result.report.level, OptimizationLevel::O0);
        assert!(
            result
                .report
                .passes
                .iter()
                .all(|pass| pass.status == PassStatus::Skipped)
        );
    }

    #[test]
    fn unreachable_block_pass_remaps_targets_and_keeps_execution_blocks() {
        let mut program = program_with_duplicate_constants();
        program.functions[0].blocks[0]
            .instructions
            .push(TacInstr::new(
                TacOp::Jump(BlockId::new(2)),
                xiao_ir::IrSpan::new(1, 2),
            ));
        program.functions[0].blocks.push(crate::tac::TacBlock {
            id: BlockId::new(1),
            scope: 0,
            instructions: vec![TacInstr::new(
                TacOp::Return { value: None },
                xiao_ir::IrSpan::new(2, 3),
            )],
        });
        program.functions[0].blocks.push(crate::tac::TacBlock {
            id: BlockId::new(2),
            scope: 0,
            instructions: vec![TacInstr::new(
                TacOp::Return { value: None },
                xiao_ir::IrSpan::new(3, 4),
            )],
        });
        let result = optimize_bytecode(
            &program,
            OptimizationConfig::baseline("portable").with_level(OptimizationLevel::O1),
        )
        .expect("优化");
        assert_eq!(result.program.functions[0].blocks.len(), 2);
        assert!(result.report.passes.iter().any(|pass| {
            pass.metadata.name == "bytecode.unreachable-block" && pass.status == PassStatus::Applied
        }));
    }
}
