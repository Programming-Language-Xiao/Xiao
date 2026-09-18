//! 09R2 研究用 TAC 编码器。
//!
//! 这里定义的是内存中的研究编码，不是公开的 `.xiaoc` 文件格式。编码器只展开
//! 已经存在的 TAC/ABI 事实，不重新推断类型、重算生命周期，也不重排指令。格式
//! 使用稳定的显式 opcode 表；操作数可以选择无符号 LEB128 或定宽 `u16`。

use std::collections::BTreeMap;

use xiao_ir::{IR_VERSION, IrArrayShape, IrDictTypeEntry, IrSpan, IrType};
use xiao_lifetime::{ExitKind, ReleaseActionKind};
use xiao_syntax::ScalarType;

use super::lower::{TAC_BYTECODE_ABI_VERSION, TAC_RUNTIME_ABI_VERSION};
use super::sig::{CallSig, CallSigTable, ParamKind};
use super::tac::{
    ArgKind, ArithOp, BlockId, CategoryMap, CompareOp, ConstId, ConstPool, FuncId, PathStep,
    RegisterClass, SigId, TAC_VERSION, TacAbi, TacArgument, TacBlock, TacConstant, TacFunction,
    TacHandler, TacInstr, TacOp, TacProgram, VReg,
};

const MAGIC: [u8; 4] = *b"X9RD";
const FORMAT_VERSION: u8 = 1;
const MAX_COLLECTION: u64 = 1 << 20;
const MAX_STRING: u64 = 1 << 24;

/// 指令操作数的宽度策略。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperandWidth {
    /// 使用无符号 LEB128 编码寄存器号和索引。
    Leb128,
    /// 使用小端定宽 `u16` 编码寄存器号和索引。
    FixedU16,
}

impl OperandWidth {
    const fn tag(self) -> u8 {
        match self {
            Self::Leb128 => 0,
            Self::FixedU16 => 1,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, EncodeError> {
        match tag {
            0 => Ok(Self::Leb128),
            1 => Ok(Self::FixedU16),
            other => Err(EncodeError::InvalidEnum {
                field: "operand_width".to_owned(),
                value: other as u64,
            }),
        }
    }
}

/// 编码器选项。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncodeOptions {
    /// 寄存器和索引的操作数宽度。
    pub operand_width: OperandWidth,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            operand_width: OperandWidth::Leb128,
        }
    }
}

impl From<OperandWidth> for EncodeOptions {
    fn from(operand_width: OperandWidth) -> Self {
        Self { operand_width }
    }
}

/// 编码后一个基本块的物理目录项。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedBlock {
    /// TAC 基本块编号。
    pub id: BlockId,
    /// 函数内第一个指令的物理 pc。
    pub pc: u32,
    /// 该块的指令字节串。
    pub bytes: Vec<u8>,
    /// 每条指令的函数内起始 pc。
    pub instruction_pcs: Vec<u32>,
    /// 与指令起始 pc 一一对应的源码区间。
    pub spans: Vec<IrSpan>,
}

/// 编码后一个函数的物理目录项。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedFunction {
    /// 函数名称。
    pub name: String,
    /// 函数内的基本块目录。
    pub blocks: Vec<EncodedBlock>,
    /// 函数指令流的总字节数。
    pub code_len: u32,
}

impl EncodedFunction {
    fn span_at_pc(&self, pc: u32) -> Option<IrSpan> {
        for (block_index, block) in self.blocks.iter().enumerate() {
            let next_block = self
                .blocks
                .get(block_index + 1)
                .map_or(self.code_len, |item| item.pc);
            if pc < block.pc || pc >= next_block {
                continue;
            }
            for (index, start) in block.instruction_pcs.iter().enumerate() {
                let end = block
                    .instruction_pcs
                    .get(index + 1)
                    .copied()
                    .unwrap_or(next_block);
                if pc >= *start && pc < end {
                    return block.spans.get(index).copied();
                }
            }
        }
        None
    }
}

/// 一份内存中的研究编码结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedProgram {
    /// 编码所携带的 ABI 版本和目标描述。
    pub abi: TacAbi,
    /// 使用的操作数宽度。
    pub operand_width: OperandWidth,
    /// 完整编码字节。
    pub bytes: Vec<u8>,
    /// 函数和基本块的物理目录。
    pub functions: Vec<EncodedFunction>,
}

/// 供执行器只读查询的函数内物理 pc 表。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PcMap {
    functions: Vec<EncodedFunction>,
}

impl PcMap {
    /// 查询一条 TAC 指令的函数内物理 pc。
    #[must_use]
    pub fn pc_at(&self, function: FuncId, block: BlockId, instruction: usize) -> Option<u32> {
        self.functions
            .get(function.get() as usize)
            .and_then(|item| item.blocks.get(block.get() as usize))
            .and_then(|item| item.instruction_pcs.get(instruction))
            .copied()
    }

    /// 按函数内物理 pc 反解源码区间。
    #[must_use]
    pub fn span_at_pc(&self, function: FuncId, pc: u32) -> Option<IrSpan> {
        self.functions
            .get(function.get() as usize)
            .and_then(|item| item.span_at_pc(pc))
    }

    /// 返回函数数量。
    #[must_use]
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
}

impl EncodedProgram {
    /// 解码该编码并返回 TAC 语义模型。
    pub fn decode(&self) -> Result<TacProgram, EncodeError> {
        decode(&self.bytes)
    }

    /// 返回指定函数、基本块中一条指令的源码区间。
    #[must_use]
    pub fn span_at(&self, function: usize, block: usize, instruction: usize) -> Option<IrSpan> {
        self.functions
            .get(function)
            .and_then(|item| item.blocks.get(block))
            .and_then(|item| item.spans.get(instruction))
            .copied()
    }

    /// 按函数内物理 pc 反解源码区间。
    ///
    /// 指向一条指令中间字节时仍命中该指令；指向函数尾部或空洞时返回 `None`。
    #[must_use]
    pub fn span_at_pc(&self, function: usize, pc: u32) -> Option<IrSpan> {
        self.functions.get(function)?.span_at_pc(pc)
    }
}

/// 研究编码的结构化错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncodeError {
    /// 魔数不匹配。
    InvalidMagic,
    /// 格式版本不匹配。
    UnsupportedFormatVersion { expected: u8, actual: u8 },
    /// ABI/TAC/IR 版本字段不匹配。
    VersionMismatch {
        /// 字段名。
        field: String,
        /// 当前实现期望值。
        expected: u64,
        /// 输入值。
        actual: u64,
    },
    /// 输入在指定字段处提前结束。
    UnexpectedEof { context: String },
    /// 输入含未知 opcode。
    UnknownOpcode(u8),
    /// 引用超出所属表的范围。
    InvalidReference {
        /// 引用类别。
        kind: String,
        /// 引用编号。
        index: u64,
        /// 表长度。
        limit: usize,
    },
    /// 定宽操作数无法容纳该编号。
    IntegerOverflow { field: String, value: u64 },
    /// 长度或集合数量不合法。
    InvalidLength { field: String, value: u64 },
    /// 稳定枚举标签未知。
    InvalidEnum { field: String, value: u64 },
    /// 源码区间不满足半开区间不变量。
    InvalidSpan { start: usize, end: usize },
    /// 产物含有尚未降低的构造。
    Unsupported(String),
    /// 编码尾部有未消费字节。
    TrailingBytes(usize),
    /// 其他结构错误。
    InvalidFormat(String),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic => formatter.write_str("研究编码魔数错误"),
            Self::UnsupportedFormatVersion { expected, actual } => {
                write!(
                    formatter,
                    "研究编码格式版本不匹配：期望 {expected}，得到 {actual}"
                )
            }
            Self::VersionMismatch {
                field,
                expected,
                actual,
            } => write!(
                formatter,
                "{field} 版本不匹配：期望 {expected}，得到 {actual}"
            ),
            Self::UnexpectedEof { context } => write!(formatter, "研究编码在 {context} 处截断"),
            Self::UnknownOpcode(opcode) => write!(formatter, "未知 TAC opcode {opcode}"),
            Self::InvalidReference { kind, index, limit } => {
                write!(formatter, "{kind} 引用 {index} 越界（长度 {limit}）")
            }
            Self::IntegerOverflow { field, value } => {
                write!(formatter, "{field} 的值 {value} 超出定宽操作数范围")
            }
            Self::InvalidLength { field, value } => write!(formatter, "{field} 长度 {value} 非法"),
            Self::InvalidEnum { field, value } => write!(formatter, "{field} 标签 {value} 未知"),
            Self::InvalidSpan { start, end } => write!(formatter, "源码区间 {start}..{end} 非法"),
            Self::Unsupported(note) => write!(formatter, "TAC 含未支持构造：{note}"),
            Self::TrailingBytes(count) => write!(formatter, "编码尾部有 {count} 个未消费字节"),
            Self::InvalidFormat(note) => formatter.write_str(note),
        }
    }
}

impl std::error::Error for EncodeError {}

/// 把 TAC 程序编码为内存中的研究字节串。
pub fn encode(
    program: &TacProgram,
    options: impl Into<EncodeOptions>,
) -> Result<EncodedProgram, EncodeError> {
    let options = options.into();
    validate_input(program)?;
    let mut writer = Writer::new(options.operand_width);
    writer.bytes.extend_from_slice(&MAGIC);
    writer.bytes.push(FORMAT_VERSION);
    writer.bytes.push(options.operand_width.tag());
    writer.uleb(program.version as u64);
    writer.uleb(program.abi.bytecode_abi_version as u64);
    writer.uleb(program.abi.runtime_abi_version as u64);
    writer.uleb(program.abi.ir_version as u64);
    writer.string(&program.abi.language_version)?;
    writer.string(&program.abi.target)?;
    encode_constants(&mut writer, &program.constants)?;
    encode_signatures(&mut writer, &program.signatures)?;

    let mut functions = Vec::with_capacity(program.functions.len());
    writer.count(program.functions.len(), "functions")?;
    for function in &program.functions {
        let mut directory = EncodedFunction {
            name: function.name.clone(),
            blocks: Vec::with_capacity(function.blocks.len()),
            code_len: 0,
        };
        encode_function(&mut writer, &mut directory, program, function)?;
        functions.push(directory);
    }
    encode_categories(&mut writer, &program.categories)?;
    encode_plans(&mut writer, program)?;
    writer.count(program.unsupported.len(), "unsupported")?;
    for note in &program.unsupported {
        writer.string(note)?;
    }
    Ok(EncodedProgram {
        abi: program.abi.clone(),
        operand_width: options.operand_width,
        bytes: writer.bytes,
        functions,
    })
}

/// 使用指定操作数宽度编码 TAC 程序的便捷入口。
pub fn encode_with_width(
    program: &TacProgram,
    width: OperandWidth,
) -> Result<EncodedProgram, EncodeError> {
    encode(program, width)
}

/// 从 TAC 建立只读物理 pc 表。
///
/// `unsupported` 只表示前端尚未启用某个语义构造，不影响已经生成的 TAC 指令
/// 位置；建立调试映射时会暂时忽略该说明，但仍保留所有结构和引用校验。
pub fn build_pc_map(program: &TacProgram, width: OperandWidth) -> Result<PcMap, EncodeError> {
    let mut map_program = program.clone();
    map_program.unsupported.clear();
    let encoded = encode(&map_program, width)?;
    Ok(PcMap {
        functions: encoded.functions,
    })
}

/// 解码研究字节串并恢复 TAC 语义模型。
pub fn decode(bytes: &[u8]) -> Result<TacProgram, EncodeError> {
    let (program, _) = decode_inner(bytes)?;
    validate_decoded(&program)?;
    Ok(program)
}

/// 解码研究字节串并同时重建物理目录。
pub fn decode_encoded(bytes: &[u8]) -> Result<EncodedProgram, EncodeError> {
    let (program, width) = decode_inner(bytes)?;
    validate_decoded(&program)?;
    encode(&program, width)
}

/// 校验一份已编码结果仍能完整解码。
pub fn validate_encoded(encoded: &EncodedProgram) -> Result<(), EncodeError> {
    let decoded = decode(&encoded.bytes)?;
    if decoded.abi != encoded.abi {
        return Err(EncodeError::InvalidFormat(
            "编码目录与 ABI 头不一致".to_owned(),
        ));
    }
    Ok(())
}

fn validate_input(program: &TacProgram) -> Result<(), EncodeError> {
    if program.version != TAC_VERSION {
        return Err(EncodeError::VersionMismatch {
            field: "tac_version".to_owned(),
            expected: TAC_VERSION as u64,
            actual: program.version as u64,
        });
    }
    check_version(
        "bytecode_abi_version",
        program.abi.bytecode_abi_version,
        TAC_BYTECODE_ABI_VERSION,
    )?;
    check_version(
        "runtime_abi_version",
        program.abi.runtime_abi_version,
        TAC_RUNTIME_ABI_VERSION,
    )?;
    check_version("ir_version", program.abi.ir_version, IR_VERSION)?;
    if let Some(note) = program.unsupported.first() {
        return Err(EncodeError::Unsupported(note.clone()));
    }
    for signature in program.signatures.iter() {
        validate_signature(signature)?;
    }
    validate_references(program)
}

fn check_version(field: &str, actual: u32, expected: u32) -> Result<(), EncodeError> {
    if actual != expected {
        return Err(EncodeError::VersionMismatch {
            field: field.to_owned(),
            expected: expected as u64,
            actual: actual as u64,
        });
    }
    Ok(())
}

fn validate_references(program: &TacProgram) -> Result<(), EncodeError> {
    for plan in &program.plans {
        check_exit_name(&plan.exit)?;
        for action in &plan.actions {
            let _ = release_tag(action.kind)?;
        }
    }
    for (function_index, function) in program.functions.iter().enumerate() {
        if let Some(signature) = function.signature {
            check_sig(program, signature)?;
        }
        if function.entry.get() as usize >= function.blocks.len() {
            return Err(EncodeError::InvalidReference {
                kind: format!("function[{function_index}].entry"),
                index: function.entry.get() as u64,
                limit: function.blocks.len(),
            });
        }
        for (block_index, block) in function.blocks.iter().enumerate() {
            if block.id.get() as usize != block_index {
                return Err(EncodeError::InvalidFormat(format!(
                    "函数 {function_index} 的块目录不是按 BlockId 排列：位置 {block_index} 是 {}",
                    block.id.get()
                )));
            }
            for instruction in &block.instructions {
                validate_op(program, function, instruction, function_index)?;
            }
        }
        for handler in &function.handlers {
            for (name, id) in [
                ("protected.start", handler.protected.0),
                ("handler", handler.handler),
            ] {
                check_block(
                    function,
                    id,
                    format!("function[{function_index}].handler.{name}"),
                )?;
            }
            if handler.protected.1.get() as usize > function.blocks.len() {
                return Err(EncodeError::InvalidReference {
                    kind: format!("function[{function_index}].handler.protected.end"),
                    index: handler.protected.1.get() as u64,
                    limit: function.blocks.len() + 1,
                });
            }
            if handler.protected.0.get() > handler.protected.1.get() {
                return Err(EncodeError::InvalidFormat(format!(
                    "函数 {function_index} 的 handler 保护区间倒置"
                )));
            }
            check_exit_name(&handler.exit)?;
            if let Some(binding) = handler.binding {
                check_index_width(
                    binding.get() as u64,
                    "handler.binding",
                    OperandWidth::Leb128,
                )?;
            }
        }
    }
    Ok(())
}

fn validate_op(
    program: &TacProgram,
    function: &TacFunction,
    instruction: &TacInstr,
    function_index: usize,
) -> Result<(), EncodeError> {
    let check_vreg = |register: VReg| {
        let _ = function;
        let _ = register;
        Ok::<(), EncodeError>(())
    };
    let check_block_ref =
        |block: BlockId| check_block(function, block, "instruction.block".to_owned());
    match &instruction.op {
        TacOp::LoadConst(id) => check_const(program, *id),
        TacOp::LoadFunc(id) => check_func(program, *id),
        TacOp::Move(value)
        | TacOp::Copy(value)
        | TacOp::Box(value)
        | TacOp::Unbox(value)
        | TacOp::Raise { value }
        | TacOp::Transfer { value } => check_vreg(*value),
        TacOp::Cast { value, .. } => check_vreg(*value),
        TacOp::Arith { left, right, .. } | TacOp::Compare { left, right, .. } => {
            check_vreg(*left)?;
            check_vreg(*right)
        }
        TacOp::NewArray { elements }
        | TacOp::NewTuple { elements }
        | TacOp::NewSet { elements } => {
            for value in elements {
                check_vreg(*value)?;
            }
            Ok(())
        }
        TacOp::NewDictTable { entries } | TacOp::NewDictColumn { entries } => {
            for (_, value) in entries {
                check_vreg(*value)?;
            }
            Ok(())
        }
        TacOp::IndexGet { source, .. } => check_vreg(*source),
        TacOp::Jump(target) | TacOp::CallSub { sub: target } => check_block_ref(*target),
        TacOp::BranchIf {
            condition,
            if_true,
            if_false,
        } => {
            check_vreg(*condition)?;
            check_block_ref(*if_true)?;
            check_block_ref(*if_false)
        }
        TacOp::Call {
            callee,
            signature,
            arguments,
        } => {
            check_func(program, *callee)?;
            check_sig(program, *signature)?;
            for argument in arguments {
                check_vreg(argument.value)?;
            }
            Ok(())
        }
        TacOp::CallDynamic { callee, arguments } => {
            check_vreg(*callee)?;
            for argument in arguments {
                check_vreg(argument.value)?;
            }
            Ok(())
        }
        TacOp::Return { value } => value.map_or(Ok(()), check_vreg),
        TacOp::MakeError { code, message, .. } => {
            code.map_or(Ok(()), check_vreg)?;
            message.map_or(Ok(()), check_vreg)
        }
        TacOp::RetFromSub | TacOp::LoadNone => Ok(()),
        TacOp::Check {
            value, on_failure, ..
        } => {
            check_vreg(*value)?;
            check_block_ref(*on_failure)
        }
        TacOp::Release { value, kind } => {
            let _ = release_tag(*kind)?;
            check_vreg(*value)
        }
        TacOp::RunReleasePlan { scope, exit } => {
            check_exit_name(exit)?;
            if !program
                .plans
                .iter()
                .any(|plan| plan.scope == *scope && plan.exit == *exit)
            {
                return Err(EncodeError::InvalidReference {
                    kind: format!("function[{function_index}].release_plan"),
                    index: *scope as u64,
                    limit: program.plans.len(),
                });
            }
            Ok(())
        }
        TacOp::EnterScope(_) => Ok(()),
        TacOp::ExitScope { exit, .. } => check_exit_name(exit),
    }
}

fn check_exit_name(name: &str) -> Result<(), EncodeError> {
    if ExitKind::from_name(name).is_none() {
        return Err(EncodeError::InvalidEnum {
            field: "ExitKind".to_owned(),
            value: 0,
        });
    }
    Ok(())
}

fn check_const(program: &TacProgram, id: ConstId) -> Result<(), EncodeError> {
    if id.get() as usize >= program.constants.len() {
        return Err(EncodeError::InvalidReference {
            kind: "ConstId".to_owned(),
            index: id.get() as u64,
            limit: program.constants.len(),
        });
    }
    Ok(())
}

fn check_func(program: &TacProgram, id: FuncId) -> Result<(), EncodeError> {
    if id.get() as usize >= program.functions.len() {
        return Err(EncodeError::InvalidReference {
            kind: "FuncId".to_owned(),
            index: id.get() as u64,
            limit: program.functions.len(),
        });
    }
    Ok(())
}

fn check_sig(program: &TacProgram, id: SigId) -> Result<(), EncodeError> {
    if id.get() as usize >= program.signatures.len() {
        return Err(EncodeError::InvalidReference {
            kind: "SigId".to_owned(),
            index: id.get() as u64,
            limit: program.signatures.len(),
        });
    }
    Ok(())
}

fn check_block(function: &TacFunction, id: BlockId, kind: String) -> Result<(), EncodeError> {
    if id.get() as usize >= function.blocks.len() {
        return Err(EncodeError::InvalidReference {
            kind,
            index: id.get() as u64,
            limit: function.blocks.len(),
        });
    }
    Ok(())
}

fn encode_constants(writer: &mut Writer, pool: &ConstPool) -> Result<(), EncodeError> {
    writer.count(pool.len(), "constants")?;
    for constant in pool.iter() {
        match constant {
            TacConstant::Int(value) => {
                writer.byte(0);
                writer.bytes.extend_from_slice(&value.to_le_bytes());
            }
            TacConstant::Sint(value) => {
                writer.byte(1);
                writer.bytes.extend_from_slice(&value.to_le_bytes());
            }
            TacConstant::Lint(value) => {
                writer.byte(2);
                writer.string(value)?;
            }
            TacConstant::Float(value) => {
                writer.byte(3);
                writer
                    .bytes
                    .extend_from_slice(&value.to_bits().to_le_bytes());
            }
            TacConstant::Sfloat(value) => {
                writer.byte(4);
                writer
                    .bytes
                    .extend_from_slice(&value.to_bits().to_le_bytes());
            }
            TacConstant::Lfloat(value) => {
                writer.byte(5);
                writer.string(value)?;
            }
            TacConstant::Bool(value) => {
                writer.byte(6);
                writer.byte(u8::from(*value));
            }
            TacConstant::Str(value) => {
                writer.byte(7);
                writer.string(value)?;
            }
        }
    }
    Ok(())
}

fn encode_signatures(writer: &mut Writer, table: &CallSigTable) -> Result<(), EncodeError> {
    writer.count(table.len(), "signatures")?;
    for signature in table.iter() {
        validate_signature(signature)?;
        let lengths = [
            signature.parameter_names.len(),
            signature.parameter_kinds.len(),
            signature.parameter_types.len(),
            signature.has_defaults.len(),
        ];
        if lengths.iter().any(|length| *length != lengths[0]) {
            return Err(EncodeError::InvalidFormat(
                "调用签名的参数平行数组长度不一致".to_owned(),
            ));
        }
        writer.count(lengths[0], "signature.parameters")?;
        for index in 0..lengths[0] {
            writer.string(&signature.parameter_names[index])?;
            writer.byte(param_kind_tag(signature.parameter_kinds[index]));
            encode_type(writer, &signature.parameter_types[index])?;
            writer.byte(u8::from(signature.has_defaults[index]));
        }
        write_optional_index(writer, signature.var_args_slot.map(|value| value as u32))?;
        write_optional_index(writer, signature.kw_args_slot.map(|value| value as u32))?;
        encode_type(writer, &signature.return_type)?;
    }
    Ok(())
}

fn validate_signature(signature: &CallSig) -> Result<(), EncodeError> {
    let lengths = [
        signature.parameter_names.len(),
        signature.parameter_kinds.len(),
        signature.parameter_types.len(),
        signature.has_defaults.len(),
    ];
    if lengths.iter().any(|length| *length != lengths[0]) {
        return Err(EncodeError::InvalidFormat(
            "调用签名的参数平行数组长度不一致".to_owned(),
        ));
    }
    for (field, slot) in [
        ("var_args_slot", signature.var_args_slot),
        ("kw_args_slot", signature.kw_args_slot),
    ] {
        if let Some(slot) = slot {
            if slot >= lengths[0] {
                return Err(EncodeError::InvalidReference {
                    kind: field.to_owned(),
                    index: slot as u64,
                    limit: lengths[0],
                });
            }
        }
    }
    Ok(())
}

fn encode_function(
    writer: &mut Writer,
    directory: &mut EncodedFunction,
    program: &TacProgram,
    function: &TacFunction,
) -> Result<(), EncodeError> {
    writer.string(&function.name)?;
    write_optional_index(writer, function.signature.map(|value| value.get()))?;
    writer.index(function.entry.get(), "function.entry")?;
    encode_span(writer, function.span)?;
    encode_categories(writer, &function.categories)?;
    encode_vregs(writer, &function.parameters)?;
    encode_vregs(writer, &function.locals)?;
    writer.count(function.scopes.len(), "function.scopes")?;
    for scope in &function.scopes {
        writer.uleb(*scope as u64);
    }
    writer.count(function.value_registers.len(), "function.value_registers")?;
    for (value, register) in &function.value_registers {
        writer.uleb(*value as u64);
        writer.index(register.get(), "value_register")?;
    }

    writer.count(function.blocks.len(), "function.blocks")?;
    let mut pc = 0_u32;
    let mut block_pcs = Vec::with_capacity(function.blocks.len());
    for block in &function.blocks {
        block_pcs.push(pc);
        writer.index(block.id.get(), "block.id")?;
        writer.uleb(block.scope as u64);
        writer.count(block.instructions.len(), "block.instructions")?;
        let mut block_writer = Writer::new(writer.width);
        let mut instruction_pcs = Vec::with_capacity(block.instructions.len());
        let mut spans = Vec::with_capacity(block.instructions.len());
        let mut block_pc = 0_u32;
        for instruction in &block.instructions {
            instruction_pcs.push(pc.checked_add(block_pc).ok_or_else(|| {
                EncodeError::IntegerOverflow {
                    field: "instruction.pc".to_owned(),
                    value: u64::from(pc) + u64::from(block_pc),
                }
            })?);
            spans.push(instruction.span);
            encode_instruction(&mut block_writer, program, function, instruction)?;
            block_pc = u32::try_from(block_writer.bytes.len()).map_err(|_| {
                EncodeError::IntegerOverflow {
                    field: "block.pc".to_owned(),
                    value: block_writer.bytes.len() as u64,
                }
            })?;
        }
        writer.count(block_writer.bytes.len(), "block.byte_length")?;
        writer.bytes.extend_from_slice(&block_writer.bytes);
        encode_span_map(writer, &instruction_pcs, &spans)?;
        directory.blocks.push(EncodedBlock {
            id: block.id,
            pc,
            bytes: block_writer.bytes,
            instruction_pcs,
            spans,
        });
        pc = pc
            .checked_add(
                directory
                    .blocks
                    .last()
                    .map_or(0, |item| item.bytes.len() as u32),
            )
            .ok_or_else(|| EncodeError::IntegerOverflow {
                field: "function.pc".to_owned(),
                value: pc as u64,
            })?;
    }
    directory.code_len = pc;
    writer.count(function.handlers.len(), "function.handlers")?;
    for handler in &function.handlers {
        writer.uleb(block_pcs[handler.protected.0.get() as usize] as u64);
        let protected_end_pc = if handler.protected.1.get() as usize == block_pcs.len() {
            pc
        } else {
            block_pcs[handler.protected.1.get() as usize]
        };
        writer.uleb(protected_end_pc as u64);
        writer.uleb(block_pcs[handler.handler.get() as usize] as u64);
        // 保留逻辑块号以便空块（相同 pc）仍能无损往返；pc 是异常路由的物理边界。
        writer.index(handler.protected.0.get(), "handler.protected.start.block")?;
        writer.index(handler.protected.1.get(), "handler.protected.end.block")?;
        writer.index(handler.handler.get(), "handler.entry.block")?;
        writer.uleb(handler.scope as u64);
        writer.string(&handler.exit)?;
        write_optional_string(writer, handler.catch_type.as_deref())?;
        write_optional_index(writer, handler.binding.map(|value| value.get()))?;
    }
    Ok(())
}

fn encode_instruction(
    writer: &mut Writer,
    program: &TacProgram,
    function: &TacFunction,
    instruction: &TacInstr,
) -> Result<(), EncodeError> {
    writer.byte(opcode(&instruction.op));
    write_optional_index(writer, instruction.dst.map(|value| value.get()))?;
    encode_op(writer, program, function, &instruction.op)
}

fn encode_span_map(
    writer: &mut Writer,
    instruction_pcs: &[u32],
    spans: &[IrSpan],
) -> Result<(), EncodeError> {
    if instruction_pcs.len() != spans.len() {
        return Err(EncodeError::InvalidFormat(
            "pc 与源码映射长度不一致".to_owned(),
        ));
    }
    writer.count(instruction_pcs.len(), "span_map")?;
    let mut previous_pc = 0_u32;
    let mut previous_start = 0_i128;
    let mut previous_end = 0_i128;
    for (pc, span) in instruction_pcs.iter().copied().zip(spans.iter().copied()) {
        if span.start > span.end {
            return Err(EncodeError::InvalidSpan {
                start: span.start,
                end: span.end,
            });
        }
        let pc_delta = pc
            .checked_sub(previous_pc)
            .ok_or_else(|| EncodeError::InvalidFormat("指令 pc 未按执行流递增".to_owned()))?;
        writer.uleb(pc_delta as u64);
        let start = i128::try_from(span.start).map_err(|_| EncodeError::IntegerOverflow {
            field: "span.start".to_owned(),
            value: u64::MAX,
        })?;
        let end = i128::try_from(span.end).map_err(|_| EncodeError::IntegerOverflow {
            field: "span.end".to_owned(),
            value: u64::MAX,
        })?;
        writer.sleb(start - previous_start);
        writer.sleb(end - previous_end);
        previous_pc = pc;
        previous_start = start;
        previous_end = end;
    }
    Ok(())
}

fn encode_op(
    writer: &mut Writer,
    program: &TacProgram,
    _function: &TacFunction,
    op: &TacOp,
) -> Result<(), EncodeError> {
    match op {
        TacOp::LoadConst(id) => writer.index(id.get(), "ConstId"),
        TacOp::LoadNone | TacOp::RetFromSub => Ok(()),
        TacOp::LoadFunc(id) => writer.index(id.get(), "FuncId"),
        TacOp::Move(value)
        | TacOp::Copy(value)
        | TacOp::Box(value)
        | TacOp::Unbox(value)
        | TacOp::Raise { value }
        | TacOp::Transfer { value } => writer.index(value.get(), "VReg"),
        TacOp::Cast { value, target } => {
            writer.index(value.get(), "VReg")?;
            writer.byte(scalar_tag(*target));
            Ok(())
        }
        TacOp::Arith { op, left, right } => {
            writer.byte(arith_tag(*op));
            writer.index(left.get(), "VReg")?;
            writer.index(right.get(), "VReg")
        }
        TacOp::Compare { op, left, right } => {
            writer.byte(compare_tag(*op));
            writer.index(left.get(), "VReg")?;
            writer.index(right.get(), "VReg")
        }
        TacOp::NewArray { elements }
        | TacOp::NewTuple { elements }
        | TacOp::NewSet { elements } => encode_vregs(writer, elements),
        TacOp::NewDictTable { entries } | TacOp::NewDictColumn { entries } => {
            writer.count(entries.len(), "dict.entries")?;
            for (key, value) in entries {
                writer.string(key)?;
                writer.index(value.get(), "VReg")?;
            }
            Ok(())
        }
        TacOp::IndexGet { source, path } => {
            writer.index(source.get(), "VReg")?;
            writer.count(path.len(), "index.path")?;
            for step in path {
                match step {
                    PathStep::Index(value) => {
                        writer.byte(0);
                        writer.sleb(*value);
                    }
                    PathStep::Key(value) => {
                        writer.byte(1);
                        writer.string(value)?;
                    }
                }
            }
            Ok(())
        }
        TacOp::Jump(target) | TacOp::CallSub { sub: target } => {
            writer.index(target.get(), "BlockId")
        }
        TacOp::BranchIf {
            condition,
            if_true,
            if_false,
        } => {
            writer.index(condition.get(), "VReg")?;
            writer.index(if_true.get(), "BlockId")?;
            writer.index(if_false.get(), "BlockId")
        }
        TacOp::Call {
            callee,
            signature,
            arguments,
        } => {
            writer.index(callee.get(), "FuncId")?;
            writer.index(signature.get(), "SigId")?;
            encode_arguments(writer, arguments)
        }
        TacOp::CallDynamic { callee, arguments } => {
            writer.index(callee.get(), "VReg")?;
            encode_arguments(writer, arguments)
        }
        TacOp::Return { value } => write_optional_index(writer, value.map(|item| item.get())),
        TacOp::MakeError {
            type_name,
            code,
            message,
        } => {
            writer.string(type_name)?;
            write_optional_index(writer, code.map(|item| item.get()))?;
            write_optional_index(writer, message.map(|item| item.get()))
        }
        TacOp::Check {
            kind,
            value,
            on_failure,
        } => {
            writer.string(kind)?;
            writer.index(value.get(), "VReg")?;
            writer.index(on_failure.get(), "BlockId")
        }
        TacOp::Release { value, kind } => {
            writer.index(value.get(), "VReg")?;
            writer.byte(release_tag(*kind)?);
            Ok(())
        }
        TacOp::RunReleasePlan { scope, exit } => {
            if !program
                .plans
                .iter()
                .any(|plan| plan.scope == *scope && plan.exit == *exit)
            {
                return Err(EncodeError::InvalidReference {
                    kind: "RunReleasePlan".to_owned(),
                    index: *scope as u64,
                    limit: program.plans.len(),
                });
            }
            writer.uleb(*scope as u64);
            writer.string(exit)
        }
        TacOp::EnterScope(scope) => {
            writer.uleb(*scope as u64);
            Ok(())
        }
        TacOp::ExitScope { scope, exit } => {
            writer.uleb(*scope as u64);
            writer.string(exit)
        }
    }
}

fn encode_arguments(writer: &mut Writer, arguments: &[TacArgument]) -> Result<(), EncodeError> {
    writer.count(arguments.len(), "call.arguments")?;
    for argument in arguments {
        writer.byte(arg_kind_tag(argument.kind));
        write_optional_string(writer, argument.name.as_deref())?;
        writer.index(argument.value.get(), "VReg")?;
    }
    Ok(())
}

fn encode_vregs(writer: &mut Writer, values: &[VReg]) -> Result<(), EncodeError> {
    writer.count(values.len(), "vregs")?;
    for value in values {
        writer.index(value.get(), "VReg")?;
    }
    Ok(())
}

fn encode_categories(writer: &mut Writer, categories: &CategoryMap) -> Result<(), EncodeError> {
    writer.count(categories.len(), "categories")?;
    for class in categories.iter() {
        writer.byte(register_class_tag(class));
    }
    Ok(())
}

fn encode_plans(writer: &mut Writer, program: &TacProgram) -> Result<(), EncodeError> {
    writer.count(program.plans.len(), "release_plans")?;
    for plan in &program.plans {
        writer.uleb(plan.scope as u64);
        writer.string(&plan.exit)?;
        writer.count(plan.actions.len(), "release_actions")?;
        for action in &plan.actions {
            writer.uleb(action.value as u64);
            writer.uleb(action.order as u64);
            writer.byte(release_tag(action.kind)?);
        }
        writer.count(plan.transferred.len(), "transferred")?;
        for value in &plan.transferred {
            writer.uleb(*value as u64);
        }
    }
    Ok(())
}

fn encode_span(writer: &mut Writer, span: IrSpan) -> Result<(), EncodeError> {
    if span.start > span.end {
        return Err(EncodeError::InvalidSpan {
            start: span.start,
            end: span.end,
        });
    }
    writer.uleb(span.start as u64);
    writer.uleb(span.end as u64);
    Ok(())
}

fn encode_type(writer: &mut Writer, ty: &IrType) -> Result<(), EncodeError> {
    match ty {
        IrType::Scalar { name } => {
            writer.byte(0);
            writer.string(name)?;
        }
        IrType::None => writer.byte(1),
        IrType::Variable { id } => {
            writer.byte(2);
            writer.uleb(*id as u64);
        }
        IrType::Function {
            parameters,
            return_type,
        } => {
            writer.byte(3);
            writer.count(parameters.len(), "type.parameters")?;
            for parameter in parameters {
                encode_type(writer, parameter)?;
            }
            encode_type(writer, return_type)?;
        }
        IrType::Array { shape } => {
            writer.byte(4);
            encode_array_shape(writer, shape)?;
        }
        IrType::Tuple { elements } => {
            writer.byte(5);
            writer.count(elements.len(), "type.elements")?;
            for element in elements {
                encode_type(writer, element)?;
            }
        }
        IrType::DictTable { entries } | IrType::DictColumn { entries } => {
            writer.byte(if matches!(ty, IrType::DictTable { .. }) {
                6
            } else {
                7
            });
            encode_dict_types(writer, entries)?;
        }
        IrType::Set {
            members,
            allows_dynamic,
            empty,
            unknown,
        } => {
            writer.byte(8);
            writer.count(members.len(), "type.members")?;
            for member in members {
                encode_type(writer, member)?;
            }
            writer.byte(u8::from(*allows_dynamic));
            writer.byte(u8::from(*empty));
            writer.byte(u8::from(*unknown));
        }
        IrType::Table { name, kind } => {
            writer.byte(9);
            writer.string(name)?;
            writer.string(kind)?;
        }
        IrType::Dynamic => writer.byte(10),
    }
    Ok(())
}

fn encode_array_shape(writer: &mut Writer, shape: &IrArrayShape) -> Result<(), EncodeError> {
    match shape {
        IrArrayShape::Homogeneous { element, length } => {
            writer.byte(0);
            encode_type(writer, element)?;
            write_optional_usize(writer, *length)?;
        }
        IrArrayShape::Heterogeneous { elements } => {
            writer.byte(1);
            writer.count(elements.len(), "array.elements")?;
            for element in elements {
                encode_type(writer, element)?;
            }
        }
        IrArrayShape::Unknown => writer.byte(2),
    }
    Ok(())
}

fn encode_dict_types(writer: &mut Writer, entries: &[IrDictTypeEntry]) -> Result<(), EncodeError> {
    writer.count(entries.len(), "dict_type.entries")?;
    for entry in entries {
        writer.string(&entry.key)?;
        encode_type(writer, &entry.value)?;
    }
    Ok(())
}

fn decode_inner(bytes: &[u8]) -> Result<(TacProgram, OperandWidth), EncodeError> {
    let mut reader = Reader::new(bytes);
    if reader.take_exact(4, "magic")? != MAGIC {
        return Err(EncodeError::InvalidMagic);
    }
    let format = reader.byte("format_version")?;
    if format != FORMAT_VERSION {
        return Err(EncodeError::UnsupportedFormatVersion {
            expected: FORMAT_VERSION,
            actual: format,
        });
    }
    let width = OperandWidth::from_tag(reader.byte("operand_width")?)?;
    let tac_version = reader.u32_uleb("tac_version")?;
    check_version("tac_version", tac_version, TAC_VERSION)?;
    let bytecode_abi_version = reader.u32_uleb("bytecode_abi_version")?;
    check_version(
        "bytecode_abi_version",
        bytecode_abi_version,
        TAC_BYTECODE_ABI_VERSION,
    )?;
    let runtime_abi_version = reader.u32_uleb("runtime_abi_version")?;
    check_version(
        "runtime_abi_version",
        runtime_abi_version,
        TAC_RUNTIME_ABI_VERSION,
    )?;
    let ir_version = reader.u32_uleb("ir_version")?;
    check_version("ir_version", ir_version, IR_VERSION)?;
    let language_version = reader.string("language_version")?;
    let target = reader.string("target")?;
    let constants = decode_constants(&mut reader, width)?;
    let signatures = decode_signatures(&mut reader, width)?;
    let function_count = reader.count("functions")?;
    let mut functions = Vec::with_capacity(function_count);
    for _ in 0..function_count {
        functions.push(decode_function(&mut reader, width)?);
    }
    let categories = decode_categories(&mut reader, "program.categories")?;
    let plans = decode_plans(&mut reader)?;
    let unsupported_count = reader.count("unsupported")?;
    let mut unsupported = Vec::with_capacity(unsupported_count);
    for _ in 0..unsupported_count {
        unsupported.push(reader.string("unsupported.item")?);
    }
    if !reader.is_empty() {
        return Err(EncodeError::TrailingBytes(reader.remaining()));
    }
    Ok((
        TacProgram {
            version: tac_version,
            abi: TacAbi {
                bytecode_abi_version,
                runtime_abi_version,
                ir_version,
                language_version,
                target,
            },
            constants,
            signatures,
            functions,
            categories,
            plans,
            unsupported,
        },
        width,
    ))
}

fn decode_constants(
    reader: &mut Reader<'_>,
    width: OperandWidth,
) -> Result<ConstPool, EncodeError> {
    let count = reader.count("constants")?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(match reader.byte("constant.tag")? {
            0 => TacConstant::Int(i64::from_le_bytes(reader.fixed::<8>("constant.int")?)),
            1 => TacConstant::Sint(i32::from_le_bytes(reader.fixed::<4>("constant.sint")?)),
            2 => TacConstant::Lint(reader.string("constant.lint")?),
            3 => TacConstant::Float(f64::from_bits(u64::from_le_bytes(
                reader.fixed::<8>("constant.float")?,
            ))),
            4 => TacConstant::Sfloat(f32::from_bits(u32::from_le_bytes(
                reader.fixed::<4>("constant.sfloat")?,
            ))),
            5 => TacConstant::Lfloat(reader.string("constant.lfloat")?),
            6 => TacConstant::Bool(read_bool(reader, "constant.bool")?),
            7 => TacConstant::Str(reader.string("constant.str")?),
            tag => {
                return Err(EncodeError::InvalidEnum {
                    field: "constant.tag".to_owned(),
                    value: tag as u64,
                });
            }
        });
    }
    let _ = width;
    Ok(ConstPool::from_entries(entries))
}

fn decode_signatures(
    reader: &mut Reader<'_>,
    width: OperandWidth,
) -> Result<CallSigTable, EncodeError> {
    let count = reader.count("signatures")?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let parameter_count = reader.count("signature.parameters")?;
        let mut parameter_names = Vec::with_capacity(parameter_count);
        let mut parameter_kinds = Vec::with_capacity(parameter_count);
        let mut parameter_types = Vec::with_capacity(parameter_count);
        let mut has_defaults = Vec::with_capacity(parameter_count);
        for _ in 0..parameter_count {
            parameter_names.push(reader.string("signature.parameter_name")?);
            parameter_kinds.push(param_kind_from_tag(reader.byte("parameter_kind")?)?);
            parameter_types.push(decode_type(reader)?);
            has_defaults.push(read_bool(reader, "signature.has_default")?);
        }
        let var_args_slot = reader.optional_index(width, "signature.var_args_slot")?;
        let kw_args_slot = reader.optional_index(width, "signature.kw_args_slot")?;
        let return_type = decode_type(reader)?;
        entries.push(CallSig {
            parameter_names,
            parameter_kinds,
            parameter_types,
            has_defaults,
            var_args_slot: var_args_slot.map(|value| value as usize),
            kw_args_slot: kw_args_slot.map(|value| value as usize),
            return_type,
        });
    }
    Ok(CallSigTable::from_entries(entries))
}

fn decode_function(
    reader: &mut Reader<'_>,
    width: OperandWidth,
) -> Result<TacFunction, EncodeError> {
    let name = reader.string("function.name")?;
    let signature = reader
        .optional_index(width, "function.signature")?
        .map(SigId::new);
    let entry = BlockId::new(reader.index(width, "function.entry")?);
    let span = decode_span(reader)?;
    let categories = decode_categories(reader, "function.categories")?;
    let parameters = decode_vregs(reader, width, "function.parameters")?;
    let locals = decode_vregs(reader, width, "function.locals")?;
    let scope_count = reader.count("function.scopes")?;
    let mut scopes = Vec::with_capacity(scope_count);
    for _ in 0..scope_count {
        scopes.push(reader.u32_uleb("function.scope")?);
    }
    let value_count = reader.count("function.value_registers")?;
    let mut value_registers = BTreeMap::new();
    for _ in 0..value_count {
        let value = reader.u32_uleb("value_registers.value")?;
        let register = VReg::new(reader.index(width, "value_registers.register")?);
        if value_registers.insert(value, register).is_some() {
            return Err(EncodeError::InvalidFormat(
                "value_registers 含重复值编号".to_owned(),
            ));
        }
    }
    let block_count = reader.count("function.blocks")?;
    let mut blocks = Vec::with_capacity(block_count);
    let mut block_pcs = Vec::with_capacity(block_count);
    let mut function_pc = 0_u32;
    for expected_id in 0..block_count {
        let id = BlockId::new(reader.index(width, "block.id")?);
        if id.get() as usize != expected_id {
            return Err(EncodeError::InvalidFormat(
                "块目录必须按 BlockId 递增排列".to_owned(),
            ));
        }
        let scope = reader.u32_uleb("block.scope")?;
        let instruction_count = reader.count("block.instructions")?;
        block_pcs.push(function_pc);
        let byte_length = reader.count("block.byte_length")?;
        let bytes = reader.take_exact(byte_length, "block.bytes")?;
        let mut block_reader = Reader::with_width(bytes, width);
        let mut instructions = Vec::with_capacity(instruction_count);
        let mut instruction_pcs = Vec::with_capacity(instruction_count);
        for _ in 0..instruction_count {
            let local_pc =
                u32::try_from(block_reader.offset).map_err(|_| EncodeError::IntegerOverflow {
                    field: "instruction.pc".to_owned(),
                    value: block_reader.offset as u64,
                })?;
            instruction_pcs.push(function_pc.checked_add(local_pc).ok_or_else(|| {
                EncodeError::IntegerOverflow {
                    field: "instruction.pc".to_owned(),
                    value: u64::from(function_pc) + u64::from(local_pc),
                }
            })?);
            instructions.push(decode_instruction(&mut block_reader, width)?);
        }
        if !block_reader.is_empty() {
            return Err(EncodeError::InvalidFormat(
                "基本块字节串在声明的指令数后仍有数据".to_owned(),
            ));
        }
        let spans = decode_span_map(reader, &instruction_pcs)?;
        for (instruction, span) in instructions.iter_mut().zip(spans) {
            instruction.span = span;
        }
        function_pc = function_pc
            .checked_add(
                u32::try_from(byte_length).map_err(|_| EncodeError::IntegerOverflow {
                    field: "function.pc".to_owned(),
                    value: byte_length as u64,
                })?,
            )
            .ok_or_else(|| EncodeError::IntegerOverflow {
                field: "function.pc".to_owned(),
                value: u64::from(function_pc) + byte_length as u64,
            })?;
        blocks.push(TacBlock {
            id,
            scope,
            instructions,
        });
    }
    let handler_count = reader.count("function.handlers")?;
    let mut handlers = Vec::with_capacity(handler_count);
    for _ in 0..handler_count {
        let protected_start_pc = reader.u32_uleb("handler.protected.start.pc")?;
        let protected_end_pc = reader.u32_uleb("handler.protected.end.pc")?;
        let handler_pc = reader.u32_uleb("handler.entry.pc")?;
        let protected_start = BlockId::new(reader.index(width, "handler.protected.start.block")?);
        let protected_end = BlockId::new(reader.index(width, "handler.protected.end.block")?);
        let handler = BlockId::new(reader.index(width, "handler.entry.block")?);
        validate_handler_pc(
            protected_start_pc,
            block_pcs.get(protected_start.get() as usize).copied(),
            "handler.protected.start.pc",
        )?;
        let expected_end_pc = if protected_end.get() as usize == block_pcs.len() {
            function_pc
        } else {
            block_pcs
                .get(protected_end.get() as usize)
                .copied()
                .ok_or_else(|| EncodeError::InvalidReference {
                    kind: "handler.protected.end.block".to_owned(),
                    index: protected_end.get() as u64,
                    limit: block_pcs.len() + 1,
                })?
        };
        if protected_end_pc != expected_end_pc {
            return Err(EncodeError::InvalidFormat(
                "handler 保护区间结束 pc 与块目录不一致".to_owned(),
            ));
        }
        validate_handler_pc(
            handler_pc,
            block_pcs.get(handler.get() as usize).copied(),
            "handler.entry.pc",
        )?;
        let scope = reader.u32_uleb("handler.scope")?;
        let exit = reader.string("handler.exit")?;
        let catch_type = reader.optional_string("handler.catch_type")?;
        let binding = reader
            .optional_index(width, "handler.binding")?
            .map(VReg::new);
        handlers.push(TacHandler {
            protected: (protected_start, protected_end),
            handler,
            scope,
            exit,
            catch_type,
            binding,
        });
    }
    Ok(TacFunction {
        name,
        signature,
        entry,
        blocks,
        parameters,
        locals,
        categories,
        scopes,
        handlers,
        value_registers,
        span,
    })
}

fn validate_handler_pc(
    actual: u32,
    expected: Option<u32>,
    field: &'static str,
) -> Result<(), EncodeError> {
    let expected = expected.ok_or_else(|| EncodeError::InvalidReference {
        kind: field.to_owned(),
        index: u64::from(actual),
        limit: 0,
    })?;
    if actual != expected {
        return Err(EncodeError::InvalidFormat(format!(
            "{field} 与块目录不一致：期望 {expected}，得到 {actual}"
        )));
    }
    Ok(())
}

fn decode_instruction(
    reader: &mut Reader<'_>,
    width: OperandWidth,
) -> Result<TacInstr, EncodeError> {
    let opcode_value = reader.byte("opcode")?;
    let dst = reader
        .optional_index(width, "instruction.dst")?
        .map(VReg::new);
    let op = decode_op(reader, width, opcode_value)?;
    Ok(TacInstr {
        op,
        dst,
        span: IrSpan::new(0, 0),
    })
}

fn decode_op(
    reader: &mut Reader<'_>,
    width: OperandWidth,
    opcode_value: u8,
) -> Result<TacOp, EncodeError> {
    let index = |reader: &mut Reader<'_>, field: &'static str| reader.index(width, field);
    Ok(match opcode_value {
        0 => TacOp::LoadConst(ConstId::new(index(reader, "ConstId")?)),
        1 => TacOp::LoadNone,
        2 => TacOp::LoadFunc(FuncId::new(index(reader, "FuncId")?)),
        3 => TacOp::Move(VReg::new(index(reader, "VReg")?)),
        4 => TacOp::Copy(VReg::new(index(reader, "VReg")?)),
        5 => TacOp::Box(VReg::new(index(reader, "VReg")?)),
        6 => TacOp::Unbox(VReg::new(index(reader, "VReg")?)),
        7 => TacOp::Cast {
            value: VReg::new(index(reader, "VReg")?),
            target: scalar_from_tag(reader.byte("ScalarType")?)?,
        },
        8 => TacOp::Arith {
            op: arith_from_tag(reader.byte("ArithOp")?)?,
            left: VReg::new(index(reader, "VReg")?),
            right: VReg::new(index(reader, "VReg")?),
        },
        9 => TacOp::Compare {
            op: compare_from_tag(reader.byte("CompareOp")?)?,
            left: VReg::new(index(reader, "VReg")?),
            right: VReg::new(index(reader, "VReg")?),
        },
        10 => TacOp::NewArray {
            elements: decode_vregs(reader, width, "array.elements")?,
        },
        11 => TacOp::NewTuple {
            elements: decode_vregs(reader, width, "tuple.elements")?,
        },
        12 => TacOp::NewDictTable {
            entries: decode_dict_entries(reader, width)?,
        },
        13 => TacOp::NewDictColumn {
            entries: decode_dict_entries(reader, width)?,
        },
        14 => TacOp::NewSet {
            elements: decode_vregs(reader, width, "set.elements")?,
        },
        15 => {
            let source = VReg::new(index(reader, "VReg")?);
            let count = reader.count("index.path")?;
            let mut path = Vec::with_capacity(count);
            for _ in 0..count {
                path.push(match reader.byte("path.tag")? {
                    0 => PathStep::Index(reader.sleb("path.index")?),
                    1 => PathStep::Key(reader.string("path.key")?),
                    tag => {
                        return Err(EncodeError::InvalidEnum {
                            field: "path.tag".to_owned(),
                            value: tag as u64,
                        });
                    }
                });
            }
            TacOp::IndexGet { source, path }
        }
        16 => TacOp::Jump(BlockId::new(index(reader, "BlockId")?)),
        17 => TacOp::BranchIf {
            condition: VReg::new(index(reader, "VReg")?),
            if_true: BlockId::new(index(reader, "BlockId")?),
            if_false: BlockId::new(index(reader, "BlockId")?),
        },
        18 => TacOp::Call {
            callee: FuncId::new(index(reader, "FuncId")?),
            signature: SigId::new(index(reader, "SigId")?),
            arguments: decode_arguments(reader, width)?,
        },
        19 => TacOp::CallDynamic {
            callee: VReg::new(index(reader, "VReg")?),
            arguments: decode_arguments(reader, width)?,
        },
        20 => TacOp::Return {
            value: reader.optional_index(width, "return.value")?.map(VReg::new),
        },
        21 => TacOp::Raise {
            value: VReg::new(index(reader, "VReg")?),
        },
        22 => TacOp::MakeError {
            type_name: reader.string("error.type_name")?,
            code: reader.optional_index(width, "error.code")?.map(VReg::new),
            message: reader
                .optional_index(width, "error.message")?
                .map(VReg::new),
        },
        23 => TacOp::CallSub {
            sub: BlockId::new(index(reader, "BlockId")?),
        },
        24 => TacOp::RetFromSub,
        25 => TacOp::Check {
            kind: reader.string("check.kind")?,
            value: VReg::new(index(reader, "VReg")?),
            on_failure: BlockId::new(index(reader, "BlockId")?),
        },
        26 => TacOp::Release {
            value: VReg::new(index(reader, "VReg")?),
            kind: release_from_tag(reader.byte("ReleaseActionKind")?)?,
        },
        27 => TacOp::Transfer {
            value: VReg::new(index(reader, "VReg")?),
        },
        28 => TacOp::RunReleasePlan {
            scope: reader.u32_uleb("release.scope")?,
            exit: reader.string("release.exit")?,
        },
        29 => TacOp::EnterScope(reader.u32_uleb("scope")?),
        30 => TacOp::ExitScope {
            scope: reader.u32_uleb("scope")?,
            exit: reader.string("scope.exit")?,
        },
        other => return Err(EncodeError::UnknownOpcode(other)),
    })
}

fn decode_arguments(
    reader: &mut Reader<'_>,
    width: OperandWidth,
) -> Result<Vec<TacArgument>, EncodeError> {
    let count = reader.count("call.arguments")?;
    let mut arguments = Vec::with_capacity(count);
    for _ in 0..count {
        let kind = arg_kind_from_tag(reader.byte("argument.kind")?)?;
        let name = reader.optional_string("argument.name")?;
        let value = VReg::new(reader.index(width, "argument.value")?);
        arguments.push(TacArgument { kind, name, value });
    }
    Ok(arguments)
}

fn decode_dict_entries(
    reader: &mut Reader<'_>,
    width: OperandWidth,
) -> Result<Vec<(String, VReg)>, EncodeError> {
    let count = reader.count("dict.entries")?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push((
            reader.string("dict.key")?,
            VReg::new(reader.index(width, "VReg")?),
        ));
    }
    Ok(entries)
}

fn decode_vregs(
    reader: &mut Reader<'_>,
    width: OperandWidth,
    field: &'static str,
) -> Result<Vec<VReg>, EncodeError> {
    let count = reader.count(field)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(VReg::new(reader.index(width, "VReg")?));
    }
    Ok(values)
}

fn decode_categories(
    reader: &mut Reader<'_>,
    field: &'static str,
) -> Result<CategoryMap, EncodeError> {
    let count = reader.count(field)?;
    let mut classes = Vec::with_capacity(count);
    for _ in 0..count {
        classes.push(register_class_from_tag(reader.byte("RegisterClass")?)?);
    }
    Ok(CategoryMap::from_classes(classes))
}

fn decode_plans(reader: &mut Reader<'_>) -> Result<Vec<super::lower::TacReleasePlan>, EncodeError> {
    let count = reader.count("release_plans")?;
    let mut plans = Vec::with_capacity(count);
    for _ in 0..count {
        let scope = reader.u32_uleb("plan.scope")?;
        let exit = reader.string("plan.exit")?;
        let action_count = reader.count("release_actions")?;
        let mut actions = Vec::with_capacity(action_count);
        for _ in 0..action_count {
            actions.push(super::lower::TacReleaseAction {
                value: reader.u32_uleb("release_action.value")?,
                order: reader.usize_uleb("release_action.order")?,
                kind: release_from_tag(reader.byte("ReleaseActionKind")?)?,
            });
        }
        let transferred_count = reader.count("transferred")?;
        let mut transferred = Vec::with_capacity(transferred_count);
        for _ in 0..transferred_count {
            transferred.push(reader.u32_uleb("transferred.value")?);
        }
        plans.push(super::lower::TacReleasePlan {
            scope,
            exit,
            actions,
            transferred,
        });
    }
    Ok(plans)
}

fn decode_span(reader: &mut Reader<'_>) -> Result<IrSpan, EncodeError> {
    let start = reader.usize_uleb("span.start")?;
    let end = reader.usize_uleb("span.end")?;
    if start > end {
        return Err(EncodeError::InvalidSpan { start, end });
    }
    Ok(IrSpan::new(start, end))
}

fn decode_span_map(
    reader: &mut Reader<'_>,
    instruction_pcs: &[u32],
) -> Result<Vec<IrSpan>, EncodeError> {
    let count = reader.count("span_map")?;
    if count != instruction_pcs.len() {
        return Err(EncodeError::InvalidFormat(format!(
            "pc 映射数量 {count} 与指令数量 {} 不一致",
            instruction_pcs.len()
        )));
    }
    let mut spans = Vec::with_capacity(count);
    let mut previous_pc = 0_u32;
    let mut previous_start = 0_usize;
    let mut previous_end = 0_usize;
    for expected_pc in instruction_pcs {
        let delta = reader.u32_uleb("span_map.pc_delta")?;
        let pc = previous_pc
            .checked_add(delta)
            .ok_or_else(|| EncodeError::IntegerOverflow {
                field: "span_map.pc".to_owned(),
                value: u64::from(previous_pc) + u64::from(delta),
            })?;
        if pc != *expected_pc {
            return Err(EncodeError::InvalidFormat(
                "span_map 的 pc_delta 与指令目录不一致".to_owned(),
            ));
        }
        let start_delta = reader.sleb("span_map.start_delta")?;
        let end_delta = reader.sleb("span_map.end_delta")?;
        let start = add_signed_usize(previous_start, start_delta, "span_map.start")?;
        let end = add_signed_usize(previous_end, end_delta, "span_map.end")?;
        if start > end {
            return Err(EncodeError::InvalidSpan { start, end });
        }
        spans.push(IrSpan::new(start, end));
        previous_pc = pc;
        previous_start = start;
        previous_end = end;
    }
    Ok(spans)
}

fn add_signed_usize(base: usize, delta: i128, field: &str) -> Result<usize, EncodeError> {
    if delta >= 0 {
        let delta = usize::try_from(delta).map_err(|_| EncodeError::IntegerOverflow {
            field: field.to_owned(),
            value: u64::MAX,
        })?;
        base.checked_add(delta)
            .ok_or_else(|| EncodeError::IntegerOverflow {
                field: field.to_owned(),
                value: u64::MAX,
            })
    } else {
        let magnitude = delta.unsigned_abs();
        let magnitude = usize::try_from(magnitude).map_err(|_| EncodeError::IntegerOverflow {
            field: field.to_owned(),
            value: u64::MAX,
        })?;
        base.checked_sub(magnitude)
            .ok_or_else(|| EncodeError::IntegerOverflow {
                field: field.to_owned(),
                value: u64::MAX,
            })
    }
}

fn decode_type(reader: &mut Reader<'_>) -> Result<IrType, EncodeError> {
    let tag = reader.byte("type.tag")?;
    Ok(match tag {
        0 => IrType::Scalar {
            name: reader.string("type.scalar")?,
        },
        1 => IrType::None,
        2 => IrType::Variable {
            id: reader.u32_uleb("type.variable")?,
        },
        3 => {
            let count = reader.count("type.parameters")?;
            let mut parameters = Vec::with_capacity(count);
            for _ in 0..count {
                parameters.push(decode_type(reader)?);
            }
            IrType::Function {
                parameters,
                return_type: Box::new(decode_type(reader)?),
            }
        }
        4 => IrType::Array {
            shape: decode_array_shape(reader)?,
        },
        5 => {
            let count = reader.count("type.elements")?;
            let mut elements = Vec::with_capacity(count);
            for _ in 0..count {
                elements.push(decode_type(reader)?);
            }
            IrType::Tuple { elements }
        }
        6 | 7 => {
            let count = reader.count("dict_type.entries")?;
            let mut entries = Vec::with_capacity(count);
            for _ in 0..count {
                entries.push(IrDictTypeEntry {
                    key: reader.string("dict_type.key")?,
                    value: Box::new(decode_type(reader)?),
                });
            }
            if tag == 6 {
                IrType::DictTable { entries }
            } else {
                IrType::DictColumn { entries }
            }
        }
        8 => {
            let count = reader.count("type.members")?;
            let mut members = Vec::with_capacity(count);
            for _ in 0..count {
                members.push(decode_type(reader)?);
            }
            IrType::Set {
                members,
                allows_dynamic: read_bool(reader, "set.allows_dynamic")?,
                empty: read_bool(reader, "set.empty")?,
                unknown: read_bool(reader, "set.unknown")?,
            }
        }
        9 => IrType::Table {
            name: reader.string("type.table.name")?,
            kind: reader.string("type.table.kind")?,
        },
        10 => IrType::Dynamic,
        tag => {
            return Err(EncodeError::InvalidEnum {
                field: "type.tag".to_owned(),
                value: tag as u64,
            });
        }
    })
}

fn decode_array_shape(reader: &mut Reader<'_>) -> Result<IrArrayShape, EncodeError> {
    Ok(match reader.byte("array.shape")? {
        0 => IrArrayShape::Homogeneous {
            element: Box::new(decode_type(reader)?),
            length: reader.optional_usize("array.length")?,
        },
        1 => {
            let count = reader.count("array.elements")?;
            let mut elements = Vec::with_capacity(count);
            for _ in 0..count {
                elements.push(decode_type(reader)?);
            }
            IrArrayShape::Heterogeneous { elements }
        }
        2 => IrArrayShape::Unknown,
        tag => {
            return Err(EncodeError::InvalidEnum {
                field: "array.shape".to_owned(),
                value: tag as u64,
            });
        }
    })
}

fn validate_decoded(program: &TacProgram) -> Result<(), EncodeError> {
    validate_input(program)
}

fn opcode(op: &TacOp) -> u8 {
    match op {
        TacOp::LoadConst(_) => 0,
        TacOp::LoadNone => 1,
        TacOp::LoadFunc(_) => 2,
        TacOp::Move(_) => 3,
        TacOp::Copy(_) => 4,
        TacOp::Box(_) => 5,
        TacOp::Unbox(_) => 6,
        TacOp::Cast { .. } => 7,
        TacOp::Arith { .. } => 8,
        TacOp::Compare { .. } => 9,
        TacOp::NewArray { .. } => 10,
        TacOp::NewTuple { .. } => 11,
        TacOp::NewDictTable { .. } => 12,
        TacOp::NewDictColumn { .. } => 13,
        TacOp::NewSet { .. } => 14,
        TacOp::IndexGet { .. } => 15,
        TacOp::Jump(_) => 16,
        TacOp::BranchIf { .. } => 17,
        TacOp::Call { .. } => 18,
        TacOp::CallDynamic { .. } => 19,
        TacOp::Return { .. } => 20,
        TacOp::Raise { .. } => 21,
        TacOp::MakeError { .. } => 22,
        TacOp::CallSub { .. } => 23,
        TacOp::RetFromSub => 24,
        TacOp::Check { .. } => 25,
        TacOp::Release { .. } => 26,
        TacOp::Transfer { .. } => 27,
        TacOp::RunReleasePlan { .. } => 28,
        TacOp::EnterScope(_) => 29,
        TacOp::ExitScope { .. } => 30,
    }
}

const SCALAR_TAGS: [(ScalarType, u8); 8] = [
    (ScalarType::Int, 0),
    (ScalarType::Sint, 1),
    (ScalarType::Lint, 2),
    (ScalarType::Float, 3),
    (ScalarType::Sfloat, 4),
    (ScalarType::Lfloat, 5),
    (ScalarType::Str, 6),
    (ScalarType::Bool, 7),
];

fn scalar_tag(value: ScalarType) -> u8 {
    SCALAR_TAGS
        .iter()
        .find(|(item, _)| *item == value)
        .map_or(0, |(_, tag)| *tag)
}

fn scalar_from_tag(tag: u8) -> Result<ScalarType, EncodeError> {
    SCALAR_TAGS
        .iter()
        .find(|(_, item)| *item == tag)
        .map(|(value, _)| *value)
        .ok_or_else(|| EncodeError::InvalidEnum {
            field: "ScalarType".to_owned(),
            value: tag as u64,
        })
}

fn release_tag(value: ReleaseActionKind) -> Result<u8, EncodeError> {
    ReleaseActionKind::ALL
        .iter()
        .position(|item| *item == value)
        .map(|tag| tag as u8)
        .ok_or_else(|| EncodeError::InvalidEnum {
            field: "ReleaseActionKind".to_owned(),
            value: u64::MAX,
        })
}

fn release_from_tag(tag: u8) -> Result<ReleaseActionKind, EncodeError> {
    ReleaseActionKind::ALL
        .get(tag as usize)
        .copied()
        .ok_or_else(|| EncodeError::InvalidEnum {
            field: "ReleaseActionKind".to_owned(),
            value: tag as u64,
        })
}

fn arith_tag(value: ArithOp) -> u8 {
    match value {
        ArithOp::Add => 0,
        ArithOp::Subtract => 1,
        ArithOp::Multiply => 2,
        ArithOp::Divide => 3,
        ArithOp::FloorDivide => 4,
        ArithOp::Remainder => 5,
        ArithOp::Power => 6,
    }
}

fn arith_from_tag(tag: u8) -> Result<ArithOp, EncodeError> {
    Ok(match tag {
        0 => ArithOp::Add,
        1 => ArithOp::Subtract,
        2 => ArithOp::Multiply,
        3 => ArithOp::Divide,
        4 => ArithOp::FloorDivide,
        5 => ArithOp::Remainder,
        6 => ArithOp::Power,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "ArithOp".to_owned(),
                value: value as u64,
            });
        }
    })
}

fn compare_tag(value: CompareOp) -> u8 {
    match value {
        CompareOp::Less => 0,
        CompareOp::LessEqual => 1,
        CompareOp::Greater => 2,
        CompareOp::GreaterEqual => 3,
        CompareOp::Equal => 4,
        CompareOp::NotEqual => 5,
    }
}

fn compare_from_tag(tag: u8) -> Result<CompareOp, EncodeError> {
    Ok(match tag {
        0 => CompareOp::Less,
        1 => CompareOp::LessEqual,
        2 => CompareOp::Greater,
        3 => CompareOp::GreaterEqual,
        4 => CompareOp::Equal,
        5 => CompareOp::NotEqual,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "CompareOp".to_owned(),
                value: value as u64,
            });
        }
    })
}

fn arg_kind_tag(value: ArgKind) -> u8 {
    match value {
        ArgKind::Positional => 0,
        ArgKind::Keyword => 1,
        ArgKind::VarArgs => 2,
        ArgKind::KwArgs => 3,
    }
}

fn arg_kind_from_tag(tag: u8) -> Result<ArgKind, EncodeError> {
    Ok(match tag {
        0 => ArgKind::Positional,
        1 => ArgKind::Keyword,
        2 => ArgKind::VarArgs,
        3 => ArgKind::KwArgs,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "ArgKind".to_owned(),
                value: value as u64,
            });
        }
    })
}

fn param_kind_tag(value: ParamKind) -> u8 {
    match value {
        ParamKind::PositionalOrKeyword => 0,
        ParamKind::PositionalOnly => 1,
        ParamKind::KeywordOnly => 2,
        ParamKind::VarArgs => 3,
        ParamKind::VarKeywords => 4,
    }
}

fn param_kind_from_tag(tag: u8) -> Result<ParamKind, EncodeError> {
    Ok(match tag {
        0 => ParamKind::PositionalOrKeyword,
        1 => ParamKind::PositionalOnly,
        2 => ParamKind::KeywordOnly,
        3 => ParamKind::VarArgs,
        4 => ParamKind::VarKeywords,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "ParamKind".to_owned(),
                value: value as u64,
            });
        }
    })
}

fn register_class_tag(value: RegisterClass) -> u8 {
    match value {
        RegisterClass::Int => 0,
        RegisterClass::Float => 1,
        RegisterClass::Bool => 2,
        RegisterClass::ObjHandle => 3,
        RegisterClass::Dynamic => 4,
        RegisterClass::None => 5,
        RegisterClass::Poly => 6,
    }
}

fn register_class_from_tag(tag: u8) -> Result<RegisterClass, EncodeError> {
    Ok(match tag {
        0 => RegisterClass::Int,
        1 => RegisterClass::Float,
        2 => RegisterClass::Bool,
        3 => RegisterClass::ObjHandle,
        4 => RegisterClass::Dynamic,
        5 => RegisterClass::None,
        6 => RegisterClass::Poly,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "RegisterClass".to_owned(),
                value: value as u64,
            });
        }
    })
}

fn write_optional_index(writer: &mut Writer, value: Option<u32>) -> Result<(), EncodeError> {
    match value {
        Some(value) => {
            writer.byte(1);
            writer.index(value, "optional_index")?;
        }
        None => writer.byte(0),
    }
    Ok(())
}

fn write_optional_string(writer: &mut Writer, value: Option<&str>) -> Result<(), EncodeError> {
    match value {
        Some(value) => {
            writer.byte(1);
            writer.string(value)?;
        }
        None => writer.byte(0),
    }
    Ok(())
}

fn write_optional_usize(writer: &mut Writer, value: Option<usize>) -> Result<(), EncodeError> {
    match value {
        Some(value) => {
            writer.byte(1);
            writer.uleb(value as u64);
        }
        None => writer.byte(0),
    }
    Ok(())
}

fn read_bool(reader: &mut Reader<'_>, field: &'static str) -> Result<bool, EncodeError> {
    match reader.byte(field)? {
        0 => Ok(false),
        1 => Ok(true),
        value => Err(EncodeError::InvalidEnum {
            field: field.to_owned(),
            value: value as u64,
        }),
    }
}

struct Writer {
    bytes: Vec<u8>,
    width: OperandWidth,
}

impl Writer {
    const fn new(width: OperandWidth) -> Self {
        Self {
            bytes: Vec::new(),
            width,
        }
    }

    fn byte(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn uleb(&mut self, mut value: u64) {
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            self.bytes.push(byte);
            if value == 0 {
                break;
            }
        }
    }

    fn sleb(&mut self, mut value: i128) {
        loop {
            let byte = (value as u8) & 0x7f;
            let sign = (byte & 0x40) != 0;
            value >>= 7;
            let done = (value == 0 && !sign) || (value == -1 && sign);
            self.bytes.push(if done { byte } else { byte | 0x80 });
            if done {
                break;
            }
        }
    }

    fn index(&mut self, value: u32, field: &str) -> Result<(), EncodeError> {
        match self.width {
            OperandWidth::Leb128 => {
                self.uleb(value as u64);
                Ok(())
            }
            OperandWidth::FixedU16 => {
                let value = u16::try_from(value).map_err(|_| EncodeError::IntegerOverflow {
                    field: field.to_owned(),
                    value: value as u64,
                })?;
                self.bytes.extend_from_slice(&value.to_le_bytes());
                Ok(())
            }
        }
    }

    fn string(&mut self, value: &str) -> Result<(), EncodeError> {
        let bytes = value.as_bytes();
        if bytes.len() as u64 > MAX_STRING {
            return Err(EncodeError::InvalidLength {
                field: "string".to_owned(),
                value: bytes.len() as u64,
            });
        }
        self.uleb(bytes.len() as u64);
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn count(&mut self, count: usize, field: &str) -> Result<(), EncodeError> {
        if count as u64 > MAX_COLLECTION {
            return Err(EncodeError::InvalidLength {
                field: field.to_owned(),
                value: count as u64,
            });
        }
        self.uleb(count as u64);
        Ok(())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self::with_width(bytes, OperandWidth::Leb128)
    }

    fn with_width(bytes: &'a [u8], _width: OperandWidth) -> Self {
        Self { bytes, offset: 0 }
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn byte(&mut self, context: &'static str) -> Result<u8, EncodeError> {
        let value = *self
            .bytes
            .get(self.offset)
            .ok_or_else(|| EncodeError::UnexpectedEof {
                context: context.to_owned(),
            })?;
        self.offset += 1;
        Ok(value)
    }

    fn take_exact(
        &mut self,
        length: usize,
        context: &'static str,
    ) -> Result<&'a [u8], EncodeError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| EncodeError::InvalidLength {
                field: context.to_owned(),
                value: length as u64,
            })?;
        if end > self.bytes.len() {
            return Err(EncodeError::UnexpectedEof {
                context: context.to_owned(),
            });
        }
        let result = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(result)
    }

    fn fixed<const N: usize>(&mut self, context: &'static str) -> Result<[u8; N], EncodeError> {
        self.take_exact(N, context)?
            .try_into()
            .map_err(|_| EncodeError::UnexpectedEof {
                context: context.to_owned(),
            })
    }

    fn uleb(&mut self, context: &'static str) -> Result<u64, EncodeError> {
        let mut value = 0_u64;
        let mut shift = 0_u32;
        for _ in 0..10 {
            let byte = self.byte(context)?;
            let part = (byte & 0x7f) as u64;
            if shift >= 64 || (shift == 63 && part > 1) {
                return Err(EncodeError::IntegerOverflow {
                    field: context.to_owned(),
                    value: u64::MAX,
                });
            }
            value |= part << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
        Err(EncodeError::IntegerOverflow {
            field: context.to_owned(),
            value: u64::MAX,
        })
    }

    fn sleb(&mut self, context: &'static str) -> Result<i128, EncodeError> {
        let mut bits = 0_u128;
        let mut shift = 0_u32;
        for index in 0..19 {
            let byte = self.byte(context)?;
            let payload = byte & 0x7f;
            if shift == 126 {
                let valid = if byte & 0x40 == 0 {
                    payload <= 1
                } else {
                    payload >= 0x7e
                };
                if !valid {
                    return Err(EncodeError::IntegerOverflow {
                        field: context.to_owned(),
                        value: u64::MAX,
                    });
                }
                bits |= u128::from(payload & 0x03) << shift;
            } else if shift < 126 {
                bits |= u128::from(payload) << shift;
            } else {
                return Err(EncodeError::IntegerOverflow {
                    field: context.to_owned(),
                    value: u64::MAX,
                });
            }
            shift += 7;
            if byte & 0x80 == 0 {
                if byte & 0x40 != 0 && shift < 128 {
                    bits |= (!0_u128) << shift;
                }
                return Ok(bits as i128);
            }
            if index == 18 {
                break;
            }
        }
        Err(EncodeError::IntegerOverflow {
            field: context.to_owned(),
            value: u64::MAX,
        })
    }

    fn u32_uleb(&mut self, context: &'static str) -> Result<u32, EncodeError> {
        u32::try_from(self.uleb(context)?).map_err(|_| EncodeError::IntegerOverflow {
            field: context.to_owned(),
            value: u64::MAX,
        })
    }

    fn usize_uleb(&mut self, context: &'static str) -> Result<usize, EncodeError> {
        usize::try_from(self.uleb(context)?).map_err(|_| EncodeError::IntegerOverflow {
            field: context.to_owned(),
            value: u64::MAX,
        })
    }

    fn count(&mut self, context: &'static str) -> Result<usize, EncodeError> {
        let value = self.uleb(context)?;
        if value > MAX_COLLECTION {
            return Err(EncodeError::InvalidLength {
                field: context.to_owned(),
                value,
            });
        }
        let count = usize::try_from(value).map_err(|_| EncodeError::InvalidLength {
            field: context.to_owned(),
            value,
        })?;
        if count > self.remaining() + 1 {
            return Err(EncodeError::InvalidLength {
                field: context.to_owned(),
                value,
            });
        }
        Ok(count)
    }

    fn string(&mut self, context: &'static str) -> Result<String, EncodeError> {
        let length = self.uleb(context)?;
        if length > MAX_STRING {
            return Err(EncodeError::InvalidLength {
                field: context.to_owned(),
                value: length,
            });
        }
        let bytes = self.take_exact(
            usize::try_from(length).map_err(|_| EncodeError::InvalidLength {
                field: context.to_owned(),
                value: length,
            })?,
            context,
        )?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| EncodeError::InvalidFormat(format!("{context} 不是合法 UTF-8")))
    }

    fn optional_string(&mut self, context: &'static str) -> Result<Option<String>, EncodeError> {
        match self.byte(context)? {
            0 => Ok(None),
            1 => Ok(Some(self.string(context)?)),
            value => Err(EncodeError::InvalidEnum {
                field: context.to_owned(),
                value: value as u64,
            }),
        }
    }

    fn index(&mut self, width: OperandWidth, context: &'static str) -> Result<u32, EncodeError> {
        match width {
            OperandWidth::Leb128 => self.u32_uleb(context),
            OperandWidth::FixedU16 => Ok(u16::from_le_bytes(self.fixed::<2>(context)?) as u32),
        }
    }

    fn optional_index(
        &mut self,
        width: OperandWidth,
        context: &'static str,
    ) -> Result<Option<u32>, EncodeError> {
        match self.byte(context)? {
            0 => Ok(None),
            1 => Ok(Some(self.index(width, context)?)),
            value => Err(EncodeError::InvalidEnum {
                field: context.to_owned(),
                value: value as u64,
            }),
        }
    }

    fn optional_usize(&mut self, context: &'static str) -> Result<Option<usize>, EncodeError> {
        match self.byte(context)? {
            0 => Ok(None),
            1 => Ok(Some(self.usize_uleb(context)?)),
            value => Err(EncodeError::InvalidEnum {
                field: context.to_owned(),
                value: value as u64,
            }),
        }
    }
}

fn check_index_width(value: u64, field: &str, width: OperandWidth) -> Result<(), EncodeError> {
    if matches!(width, OperandWidth::FixedU16) && value > u16::MAX as u64 {
        return Err(EncodeError::IntegerOverflow {
            field: field.to_owned(),
            value,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::lower::{TacReleaseAction, TacReleasePlan};

    fn instruction(index: usize, op: TacOp) -> TacInstr {
        TacInstr {
            op,
            dst: (index < 16).then(|| VReg::new((index + 20) as u32)),
            span: IrSpan::new(100 + index * 3, 102 + index * 3),
        }
    }

    fn all_types() -> Vec<IrType> {
        vec![
            IrType::Scalar {
                name: "int".to_owned(),
            },
            IrType::None,
            IrType::Variable { id: 9 },
            IrType::Function {
                parameters: vec![IrType::Dynamic],
                return_type: Box::new(IrType::None),
            },
            IrType::Array {
                shape: IrArrayShape::Homogeneous {
                    element: Box::new(IrType::Dynamic),
                    length: Some(3),
                },
            },
            IrType::Array {
                shape: IrArrayShape::Heterogeneous {
                    elements: vec![IrType::None, IrType::Dynamic],
                },
            },
            IrType::Array {
                shape: IrArrayShape::Unknown,
            },
            IrType::Tuple {
                elements: vec![IrType::None, IrType::Dynamic],
            },
            IrType::DictTable {
                entries: vec![IrDictTypeEntry {
                    key: "a".to_owned(),
                    value: Box::new(IrType::Dynamic),
                }],
            },
            IrType::DictColumn {
                entries: vec![IrDictTypeEntry {
                    key: "b".to_owned(),
                    value: Box::new(IrType::None),
                }],
            },
            IrType::Set {
                members: vec![IrType::Dynamic],
                allows_dynamic: true,
                empty: false,
                unknown: true,
            },
            IrType::Table {
                name: "Point".to_owned(),
                kind: "record".to_owned(),
            },
            IrType::Dynamic,
        ]
    }

    fn all_ops_program() -> TacProgram {
        let mut constants = ConstPool::new();
        let constant = constants.intern(TacConstant::Int(-7));
        constants.intern(TacConstant::Sint(-3));
        constants.intern(TacConstant::Lint("12345678901234567890".to_owned()));
        constants.intern(TacConstant::Float(f64::from_bits(0x7ff8_0000_0000_0042)));
        constants.intern(TacConstant::Float(-0.0));
        constants.intern(TacConstant::Sfloat(f32::from_bits(0x7fc0_0021)));
        constants.intern(TacConstant::Lfloat("1.234567890123456789".to_owned()));
        constants.intern(TacConstant::Bool(true));
        constants.intern(TacConstant::Str("雪\0行".to_owned()));

        let mut signatures = CallSigTable::new();
        let call_signature =
            signatures.intern(CallSig::plain(vec![IrType::Dynamic], IrType::Dynamic));
        let types = all_types();
        signatures.intern(CallSig {
            parameter_names: (0..types.len()).map(|index| format!("p{index}")).collect(),
            parameter_kinds: (0..types.len())
                .map(|index| match index % 5 {
                    0 => ParamKind::PositionalOrKeyword,
                    1 => ParamKind::PositionalOnly,
                    2 => ParamKind::KeywordOnly,
                    3 => ParamKind::VarArgs,
                    _ => ParamKind::VarKeywords,
                })
                .collect(),
            parameter_types: types,
            has_defaults: (0..all_types().len()).map(|index| index % 2 == 0).collect(),
            var_args_slot: Some(3),
            kw_args_slot: Some(4),
            return_type: IrType::Dynamic,
        });

        let arguments = vec![
            TacArgument::positional(VReg::new(0)),
            TacArgument::keyword("named", VReg::new(1)),
            TacArgument {
                kind: ArgKind::VarArgs,
                name: None,
                value: VReg::new(2),
            },
            TacArgument {
                kind: ArgKind::KwArgs,
                name: None,
                value: VReg::new(3),
            },
        ];
        let ops = vec![
            TacOp::LoadConst(constant),
            TacOp::LoadNone,
            TacOp::LoadFunc(FuncId::new(1)),
            TacOp::Move(VReg::new(0)),
            TacOp::Copy(VReg::new(1)),
            TacOp::Box(VReg::new(2)),
            TacOp::Unbox(VReg::new(3)),
            TacOp::Cast {
                value: VReg::new(4),
                target: ScalarType::Sfloat,
            },
            TacOp::Arith {
                op: ArithOp::Power,
                left: VReg::new(5),
                right: VReg::new(6),
            },
            TacOp::Compare {
                op: CompareOp::NotEqual,
                left: VReg::new(7),
                right: VReg::new(8),
            },
            TacOp::NewArray {
                elements: vec![VReg::new(0), VReg::new(1)],
            },
            TacOp::NewTuple {
                elements: vec![VReg::new(2), VReg::new(3)],
            },
            TacOp::NewDictTable {
                entries: vec![("first".to_owned(), VReg::new(4))],
            },
            TacOp::NewDictColumn {
                entries: vec![("second".to_owned(), VReg::new(5))],
            },
            TacOp::NewSet {
                elements: vec![VReg::new(6), VReg::new(7)],
            },
            TacOp::IndexGet {
                source: VReg::new(8),
                path: vec![PathStep::Index(-129), PathStep::Key("key".to_owned())],
            },
            TacOp::Jump(BlockId::new(1)),
            TacOp::BranchIf {
                condition: VReg::new(9),
                if_true: BlockId::new(1),
                if_false: BlockId::new(2),
            },
            TacOp::Call {
                callee: FuncId::new(1),
                signature: call_signature,
                arguments: arguments.clone(),
            },
            TacOp::CallDynamic {
                callee: VReg::new(10),
                arguments,
            },
            TacOp::Return {
                value: Some(VReg::new(11)),
            },
            TacOp::Raise {
                value: VReg::new(12),
            },
            TacOp::MakeError {
                type_name: "ValueError".to_owned(),
                code: Some(VReg::new(13)),
                message: Some(VReg::new(14)),
            },
            TacOp::CallSub {
                sub: BlockId::new(2),
            },
            TacOp::RetFromSub,
            TacOp::Check {
                kind: "numeric_range".to_owned(),
                value: VReg::new(15),
                on_failure: BlockId::new(2),
            },
            TacOp::Release {
                value: VReg::new(16),
                kind: ReleaseActionKind::Strong,
            },
            TacOp::Transfer {
                value: VReg::new(17),
            },
            TacOp::RunReleasePlan {
                scope: 7,
                exit: "normal".to_owned(),
            },
            TacOp::EnterScope(7),
            TacOp::ExitScope {
                scope: 7,
                exit: "return".to_owned(),
            },
        ];
        assert_eq!(ops.len(), 31);
        let opcodes = ops.iter().map(opcode).collect::<Vec<_>>();
        assert_eq!(opcodes, (0_u8..31).collect::<Vec<_>>());

        let mut categories = CategoryMap::new();
        for (index, class) in [
            RegisterClass::Int,
            RegisterClass::Float,
            RegisterClass::Bool,
            RegisterClass::ObjHandle,
            RegisterClass::Dynamic,
            RegisterClass::None,
            RegisterClass::Poly,
        ]
        .into_iter()
        .enumerate()
        {
            categories.insert(VReg::new(index as u32), class);
        }
        let blocks = vec![
            TacBlock {
                id: BlockId::new(0),
                scope: 0,
                instructions: ops
                    .into_iter()
                    .enumerate()
                    .map(|(index, op)| instruction(index, op))
                    .collect(),
            },
            TacBlock {
                id: BlockId::new(1),
                scope: 7,
                instructions: vec![TacInstr::new(
                    TacOp::Jump(BlockId::new(2)),
                    IrSpan::new(400, 401),
                )],
            },
            TacBlock {
                id: BlockId::new(2),
                scope: 7,
                instructions: vec![TacInstr::new(TacOp::RetFromSub, IrSpan::new(500, 500))],
            },
        ];
        let function = TacFunction {
            name: "main".to_owned(),
            signature: Some(call_signature),
            entry: BlockId::new(0),
            blocks,
            parameters: vec![VReg::new(0)],
            locals: vec![VReg::new(1), VReg::new(2)],
            categories: categories.clone(),
            scopes: vec![0, 7],
            handlers: vec![TacHandler {
                protected: (BlockId::new(0), BlockId::new(2)),
                handler: BlockId::new(2),
                scope: 7,
                exit: "catch".to_owned(),
                catch_type: Some("Error".to_owned()),
                binding: Some(VReg::new(3)),
            }],
            value_registers: BTreeMap::from([(44, VReg::new(4)), (45, VReg::new(5))]),
            span: IrSpan::new(80, 700),
        };
        let callee = TacFunction {
            name: "callee".to_owned(),
            signature: Some(call_signature),
            entry: BlockId::new(0),
            blocks: vec![TacBlock {
                id: BlockId::new(0),
                scope: 0,
                instructions: vec![TacInstr::new(
                    TacOp::Return { value: None },
                    IrSpan::new(800, 801),
                )],
            }],
            parameters: vec![VReg::new(0)],
            locals: Vec::new(),
            categories: categories.clone(),
            scopes: vec![0],
            handlers: Vec::new(),
            value_registers: BTreeMap::new(),
            span: IrSpan::new(780, 820),
        };
        TacProgram {
            version: TAC_VERSION,
            abi: TacAbi {
                bytecode_abi_version: TAC_BYTECODE_ABI_VERSION,
                runtime_abi_version: TAC_RUNTIME_ABI_VERSION,
                ir_version: IR_VERSION,
                language_version: "0.1.0-test".to_owned(),
                target: "test-target".to_owned(),
            },
            constants,
            signatures,
            functions: vec![function, callee],
            categories,
            plans: vec![TacReleasePlan {
                scope: 7,
                exit: "normal".to_owned(),
                actions: vec![TacReleaseAction {
                    value: 44,
                    order: 0,
                    kind: ReleaseActionKind::Weak,
                }],
                transferred: vec![45],
            }],
            unsupported: Vec::new(),
        }
    }

    fn assert_constant_eq(left: &TacConstant, right: &TacConstant) {
        match (left, right) {
            (TacConstant::Float(left), TacConstant::Float(right)) => {
                assert_eq!(left.to_bits(), right.to_bits());
            }
            (TacConstant::Sfloat(left), TacConstant::Sfloat(right)) => {
                assert_eq!(left.to_bits(), right.to_bits());
            }
            _ => assert_eq!(left, right),
        }
    }

    fn assert_program_eq(left: &TacProgram, right: &TacProgram) {
        assert_eq!(left.version, right.version);
        assert_eq!(left.abi, right.abi);
        assert_eq!(left.signatures, right.signatures);
        assert_eq!(left.functions, right.functions);
        assert_eq!(left.categories, right.categories);
        assert_eq!(left.plans, right.plans);
        assert_eq!(left.unsupported, right.unsupported);
        assert_eq!(left.constants.len(), right.constants.len());
        for (left, right) in left.constants.iter().zip(right.constants.iter()) {
            assert_constant_eq(left, right);
        }
    }

    #[test]
    fn all_opcodes_and_abi_fields_round_trip_in_both_widths() {
        let program = all_ops_program();
        let mut sizes = Vec::new();
        for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
            let encoded = encode(&program, width).expect("完整 TAC 应可编码");
            validate_encoded(&encoded).expect("编码应可自校验");
            let decoded = decode(&encoded.bytes).expect("完整 TAC 应可解码");
            assert_program_eq(&program, &decoded);
            assert_eq!(encoded.functions[0].blocks[0].instruction_pcs.len(), 31);
            assert_eq!(encoded.span_at(0, 0, 7), Some(IrSpan::new(121, 123)));
            let pc = encoded.functions[0].blocks[0].instruction_pcs[7];
            assert_eq!(encoded.span_at_pc(0, pc), Some(IrSpan::new(121, 123)));
            assert_eq!(encoded.span_at_pc(0, pc + 1), Some(IrSpan::new(121, 123)));
            assert_eq!(encoded.span_at_pc(0, encoded.functions[0].code_len), None);
            assert_ne!(pc as usize, 121, "物理 pc 不得退化为源码偏移");
            assert!(encoded.functions[0].blocks[1].pc > 0);
            sizes.push(encoded.bytes.len());
        }
        assert!(sizes[0] < sizes[1], "小编号下 LEB128 应比定宽编码更短");
    }

    #[test]
    fn unsigned_and_signed_leb128_cover_integer_boundaries() {
        let mut writer = Writer::new(OperandWidth::Leb128);
        for value in [0, 127, 128, u32::MAX] {
            writer.index(value, "test").expect("LEB128 应容纳 u32");
        }
        for value in [i128::MIN, -129, -1, 0, 127, 128, i128::MAX] {
            writer.sleb(value);
        }
        let mut reader = Reader::new(&writer.bytes);
        for value in [0, 127, 128, u32::MAX] {
            assert_eq!(reader.index(OperandWidth::Leb128, "test"), Ok(value));
        }
        for value in [i128::MIN, -129, -1, 0, 127, 128, i128::MAX] {
            assert_eq!(reader.sleb("test"), Ok(value));
        }
        assert!(reader.is_empty());
    }

    #[test]
    fn damaged_inputs_are_rejected_structurally() {
        let encoded = encode(&all_ops_program(), OperandWidth::Leb128).expect("基线编码应成功");

        let mut bad_version = encoded.bytes.clone();
        bad_version[7] = 2;
        assert!(matches!(
            decode(&bad_version),
            Err(EncodeError::VersionMismatch { ref field, .. }) if field == "bytecode_abi_version"
        ));

        assert!(matches!(
            decode(&encoded.bytes[..encoded.bytes.len() - 1]),
            Err(EncodeError::UnexpectedEof { .. }) | Err(EncodeError::InvalidLength { .. })
        ));
        let mut trailing = encoded.bytes.clone();
        trailing.push(0);
        assert_eq!(decode(&trailing), Err(EncodeError::TrailingBytes(1)));

        let mut unknown = Writer::new(OperandWidth::Leb128);
        unknown.byte(255);
        write_optional_index(&mut unknown, None).expect("可写空结果");
        let mut reader = Reader::new(&unknown.bytes);
        assert_eq!(
            decode_instruction(&mut reader, OperandWidth::Leb128),
            Err(EncodeError::UnknownOpcode(255))
        );

        let mut bad_release = Writer::new(OperandWidth::Leb128);
        bad_release.byte(26);
        write_optional_index(&mut bad_release, None).expect("可写空结果");
        bad_release.index(0, "VReg").expect("可写寄存器");
        bad_release.byte(9);
        let mut reader = Reader::new(&bad_release.bytes);
        assert!(matches!(
            decode_instruction(&mut reader, OperandWidth::Leb128),
            Err(EncodeError::InvalidEnum { ref field, value: 9 })
                if field == "ReleaseActionKind"
        ));

        let mut bad_length = Writer::new(OperandWidth::Leb128);
        bad_length.uleb(MAX_STRING + 1);
        let mut reader = Reader::new(&bad_length.bytes);
        assert!(matches!(
            reader.string("test.string"),
            Err(EncodeError::InvalidLength { .. })
        ));
    }

    #[test]
    fn bad_references_and_fixed_width_overflow_are_rejected() {
        let mut program = all_ops_program();
        program.functions[0].blocks[0].instructions[0].op = TacOp::LoadConst(ConstId::new(999));
        assert!(matches!(
            encode(&program, OperandWidth::Leb128),
            Err(EncodeError::InvalidReference { ref kind, .. }) if kind == "ConstId"
        ));

        let mut program = all_ops_program();
        program.functions[0].blocks[0].instructions[18].op = TacOp::Call {
            callee: FuncId::new(99),
            signature: SigId::new(0),
            arguments: Vec::new(),
        };
        assert!(matches!(
            encode(&program, OperandWidth::Leb128),
            Err(EncodeError::InvalidReference { ref kind, .. }) if kind == "FuncId"
        ));

        let mut program = all_ops_program();
        program.functions[0].blocks[0].instructions[18].op = TacOp::Call {
            callee: FuncId::new(1),
            signature: SigId::new(99),
            arguments: Vec::new(),
        };
        assert!(matches!(
            encode(&program, OperandWidth::Leb128),
            Err(EncodeError::InvalidReference { ref kind, .. }) if kind == "SigId"
        ));

        let mut program = all_ops_program();
        program.functions[0].blocks[0].instructions[16].op = TacOp::Jump(BlockId::new(99));
        assert!(matches!(
            encode(&program, OperandWidth::Leb128),
            Err(EncodeError::InvalidReference { ref kind, .. }) if kind == "instruction.block"
        ));

        let mut program = all_ops_program();
        program.functions[0].blocks[0].instructions[3].op = TacOp::Move(VReg::new(65_536));
        assert!(matches!(
            encode(&program, OperandWidth::FixedU16),
            Err(EncodeError::IntegerOverflow { value: 65_536, .. })
        ));
    }

    #[test]
    fn scalar_and_release_tags_have_one_bidirectional_mapping() {
        for (scalar, tag) in SCALAR_TAGS {
            assert_eq!(scalar_tag(scalar), tag);
            assert_eq!(scalar_from_tag(tag), Ok(scalar));
        }
        for (tag, kind) in ReleaseActionKind::ALL.into_iter().enumerate() {
            assert_eq!(release_tag(kind), Ok(tag as u8));
            assert_eq!(release_from_tag(tag as u8), Ok(kind));
        }
    }
}
