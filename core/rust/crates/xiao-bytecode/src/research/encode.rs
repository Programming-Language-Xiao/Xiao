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

/// 研究编码的魔数。
///
/// `X9` 前缀刻意与真实的 `.xiaoc` 容器区分开：这里编出来的是内存研究编码，
/// 不承诺任何文件级兼容，因此需要一个能立刻判死的头，避免把实验字节流当成
/// 产物格式误读。
const MAGIC: [u8; 4] = *b"X9RD";
/// 字节布局的版本号，与 ABI 版本分工不同。
///
/// ABI 版本描述语义契约（R1 冻结），这个字段描述**字节怎么排**。只要布局
/// 变动就必须递增它，解码端据此直接拒绝旧字节，而不是照着新规则错读旧数据。
const FORMAT_VERSION: u8 = 1;
/// 集合元素数与块字节长度的上限。
///
/// 解码端读到长度前缀后第一件事就是拿它做上界判断：没有这个上限，一段几字节
/// 的损坏输入就能让解码器按伪造的 u64 去预留内存。
const MAX_COLLECTION: u64 = 1 << 20;
/// 单个字符串的字节长度上限。
///
/// 与 [`MAX_COLLECTION`] 同理，挡的是「先分配再发现读不完」的路径；字符串
/// 常量可以很长，所以这个上限比集合上限宽得多。
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
    /// 返回写进头部的宽度标签。
    ///
    /// 标签是格式的一部分，[`Self::from_tag`] 是它唯一的逆映射；两个方向必须
    /// 一起改，单独改一个方向等于静默改格式。
    const fn tag(self) -> u8 {
        match self {
            Self::Leb128 => 0,
            Self::FixedU16 => 1,
        }
    }

    /// 按头部的宽度标签解析操作数宽度。
    ///
    /// 未知标签报 [`EncodeError::InvalidEnum`]，**不退回默认宽度**：静默退回
    /// 会让同一份字节被两种读法解释，而调用方拿到的却是一个「成功」的结果。
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
    /// 默认使用 [`OperandWidth::Leb128`]。
    ///
    /// 研究编码里的寄存器号和索引绝大多数是小编号，变长比定宽短；定宽是给
    /// 需要固定步长或定长扫描的消费者显式选的，不该是默认。
    fn default() -> Self {
        Self {
            operand_width: OperandWidth::Leb128,
        }
    }
}

impl From<OperandWidth> for EncodeOptions {
    /// 让 `encode(program, OperandWidth::FixedU16)` 直接可用，省掉调用方为了
    /// 传一个宽度而构造选项结构的样板。
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
    /// 按函数内物理 pc 反解源码区间。
    ///
    /// 块目录按 pc 递增排列，所以「下一块的起始 pc」就是本块的排他上界，最后
    /// 一块用 `code_len`。命中落在 `[instruction_pcs[i], instruction_pcs[i+1])`
    /// 之内，因此指向一条指令的**中间字节**也会返回它的区间；落在块间空洞或
    /// 函数尾部返回 `None`。这里是线性扫描，调用点在调试路径上，不值得为它
    /// 维护一棵索引。
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
    UnsupportedFormatVersion {
        /// 当前实现支持的格式版本。
        expected: u8,
        /// 输入携带的格式版本。
        actual: u8,
    },
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
    UnexpectedEof {
        /// 截断发生时正在读取的字段。
        context: String,
    },
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
    IntegerOverflow {
        /// 溢出的字段名称。
        field: String,
        /// 无法表示的原始值。
        value: u64,
    },
    /// 长度或集合数量不合法。
    InvalidLength {
        /// 长度字段名称。
        field: String,
        /// 输入给出的长度。
        value: u64,
    },
    /// 稳定枚举标签未知。
    InvalidEnum {
        /// 枚举字段名称。
        field: String,
        /// 输入给出的标签。
        value: u64,
    },
    /// 源码区间不满足半开区间不变量。
    InvalidSpan {
        /// 区间起点。
        start: usize,
        /// 区间终点。
        end: usize,
    },
    /// 产物含有尚未降低的构造。
    Unsupported(String),
    /// 编码尾部有未消费字节。
    TrailingBytes(usize),
    /// 其他结构错误。
    InvalidFormat(String),
}

impl std::fmt::Display for EncodeError {
    /// 把结构化错误渲染成人读的中文诊断。
    ///
    /// 每个变体都带上出错的具体字段与期望/实际值，因为调用方（检查器、测试）
    /// 往往只打印 `Display`。`InvalidFormat` 直接透传底层说明——构造点已经把
    /// 字段和期望值写进去了，这里再包一层只会丢失信息。
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
    validate_input(program, options.operand_width)?;
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
    encode_selection_plans(&mut writer, program)?;
    encode_broadcast_plans(&mut writer, program)?;
    encode_random_seed_plans(&mut writer, program)?;
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
    let (program, width) = decode_inner(bytes)?;
    validate_decoded(&program, width)?;
    Ok(program)
}

/// 解码研究字节串并同时重建物理目录。
pub fn decode_encoded(bytes: &[u8]) -> Result<EncodedProgram, EncodeError> {
    let (program, width) = decode_inner(bytes)?;
    validate_decoded(&program, width)?;
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

/// 编码前的整体校验，也是编码器唯一的入口关卡。
///
/// 顺序是有意的：先卡版本字段（TAC、bytecode ABI、runtime ABI、IR，全部必须
/// 等于当前实现），再拒绝非空 `unsupported`——宁可整体失败，也不要产出一份
/// 「能解码但少算了一部分」的编码；最后才是签名自洽性与跨表引用。版本错误先
/// 于引用错误报出，因为版本不符时后面的编号根本没有可比对的基准。
///
/// `width` 必须由调用方传入真实的操作数宽度：定宽 `u16` 下部分索引可能放不下，
/// 传错宽度会让那条检查静默失效（见 [`check_index_width`]）。
fn validate_input(program: &TacProgram, width: OperandWidth) -> Result<(), EncodeError> {
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
    validate_references(program, width)
}

/// 比对单个版本字段，不一致时报出字段名与两侧取值。
///
/// 抽出来是为了让编码入口与解码入口共用同一条判定；四处版本字段各写一遍
/// `if` 很容易漏掉其中一处。
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

/// 检查所有「编码时会直接下标」的跨表引用。
///
/// 覆盖的范围比看起来大，因为它同时守住了两条结构性不变量：
///
/// - 块目录必须严格按 `BlockId` 递增排列（`blocks[i].id == i`）。编码端按位置
///   写块，handler 的物理 pc 又靠 `block_pcs[块号]` 直接下标取，一旦目录错位，
///   异常路由会静默指到别的块。
/// - handler 的保护区间必须非倒置，且终点允许等于块数（排他上界可以越过最后
///   一块，落到函数尾部）。
///
/// 这里也顺带过一遍释放动作的类别与所有退出边名称，因为这两者在编码端被当作
/// 「一定能映射到标签」使用。
fn validate_references(program: &TacProgram, width: OperandWidth) -> Result<(), EncodeError> {
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
                check_index_width(binding.get() as u64, "handler.binding", width)?;
            }
        }
    }
    Ok(())
}

/// 检查一条指令引用到的表项都在范围内。
///
/// 校验分三类：常量池（`LoadConst`）、函数表（`LoadFunc`、`Call`）、签名表
/// （`Call`），以及本函数的块目录（跳转、分支、`on_failure`、`Check`）。
/// `RunReleasePlan` 另有一条更强的约束：`(scope, exit)` 必须**成对**出现在
/// 释放计划表里，只对上作用域是没用的，解释器按这一对取计划。
///
/// [`VReg`] 编号**没有可校验的上界**：TAC 没有独立的寄存器表，编号空间与
/// [`CategoryMap`] 的稠密长度不是同一回事（未登记的编号只是默认为 `Poly`，
/// 合法）。因此 `check_vreg` 是一个空闭包——它在每个用到寄存器的分支上保留了
/// 调用位置，等将来有了真正的寄存器表再往里填检查即可。寄存器号唯一的硬约束
/// 是定宽模式下的 `u16` 上界，那由 `Writer::index` 在写出时兜住。
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
        TacOp::SelectorApply {
            source,
            plan,
            step,
            random_counts,
        } => {
            if *plan as usize >= program.selection_plans.len() {
                return Err(EncodeError::InvalidReference {
                    kind: format!("function[{function_index}].selection_plan"),
                    index: *plan as u64,
                    limit: program.selection_plans.len(),
                });
            }
            check_vreg(*source)?;
            step.map_or(Ok(()), check_vreg)?;
            for value in random_counts.iter().flatten() {
                check_vreg(*value)?;
            }
            Ok(())
        }
        TacOp::BroadcastAssign { root, value, plan } => {
            if *plan as usize >= program.broadcast_assignment_plans.len() {
                return Err(EncodeError::InvalidReference {
                    kind: format!("function[{function_index}].broadcast_plan"),
                    index: *plan as u64,
                    limit: program.broadcast_assignment_plans.len(),
                });
            }
            check_vreg(*root)?;
            check_vreg(*value)
        }
        TacOp::RandomSeed { value, plan } => {
            if *plan as usize >= program.random_seed_plans.len() {
                return Err(EncodeError::InvalidReference {
                    kind: format!("function[{function_index}].random_seed_plan"),
                    index: *plan as u64,
                    limit: program.random_seed_plans.len(),
                });
            }
            check_vreg(*value)
        }
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

/// 检查退出边名称是 `ExitKind` 的稳定拼写。
///
/// 退出边名由 `xiao-lifetime` 冻结，编码按名字写字符串而不是编码成编号，所以
/// 校验只能走 `ExitKind::from_name`。诊断里的 `value` 固定填 0：出错的是「名字
/// 不认识」，没有一个可报的数字输入，字段名才是有效信息。
fn check_exit_name(name: &str) -> Result<(), EncodeError> {
    if ExitKind::from_name(name).is_none() {
        return Err(EncodeError::InvalidEnum {
            field: "ExitKind".to_owned(),
            value: 0,
        });
    }
    Ok(())
}

/// 检查 `ConstId` 落在常量池内。
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

/// 检查 `FuncId` 落在函数表内。
///
/// 索引 0 是脚本入口，它可以被 `LoadFunc`/`Call` 正常引用（递归入口是合法的），
/// 所以这里只卡上界，不排除任何编号。
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

/// 检查 `SigId` 落在签名表内。
///
/// 签名的变长平行数组不走这里，而由 [`validate_signature`] 单独负责：这里只
/// 回答「这个编号有没有指向一条存在的签名」。
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

/// 检查块号落在**本函数**的块目录内。
///
/// 块号是函数内编号空间，跨函数比较没有意义，所以基准取 `function` 而不是
/// `program`。`kind` 由调用方给出（跳转目标、`on_failure`、handler 区间……），
/// 直接进诊断，用来区分「哪一处引用越界」——同类错误在一条指令里可能出现多次，
/// 没有这个上下文就只能靠数编号。
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

/// 按常量池索引顺序写常量：1 字节类别标签加载荷。
///
/// 载荷按类型分开编码：`Int`/`Sint` 写小端定宽整数，`Float`/`Sfloat` 先取
/// `to_bits` 再写位模式（用 `PartialEq` 或十进制往返会丢掉 `-0.0` 的符号和 NaN
/// 的载荷位），`Lint`/`Lfloat` 保持规范十进制文本，字符串写长度前缀加 UTF-8
/// 字节。索引就是写出顺序，所以顺序不能重排。
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

/// 写签名表：每条签名先写形参数量，再逐参数写「名字、类别标签、类型、有无
/// 默认值」，最后写 `*args`/`**kwargs` 槽位与返回类型。
///
/// 这里再校验一次平行数组等长（编码入口已经查过），是因为本函数会按下标
/// `[index]` 同时索引四个数组——多一道局部检查比在下标处 panic 划算。
/// 两个槽位是「被调方帧内的形参序号」，用可选索引写：`None` 与 `0` 必须区分，
/// 0 号形参的槽位是合法位置。
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

/// 检查一条签名自洽：四条平行数组等长，且两个变参槽位指向真实存在的形参。
///
/// 槽位越界不会在当前编码里报错（它只是个编号），但运行期会按槽位去被调方帧
/// 取寄存器，取到的是别人的值——所以必须在编码期挡住。
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

/// 写一个函数，并同步填出它的物理目录。
///
/// 头部依次写名字、签名、入口块、函数源码区间、本函数类别表、形参/局部表、
/// 作用域集合和「`IrValue.id` → 寄存器」映射。之后逐块写：
///
/// - 块字节先写进独立的 `block_writer`，因为块头要回填**精确的字节长度**，
///   而长度只有写完才知道。
/// - 每条指令的「函数内 pc」在写出**之前**就算好并记进目录：`instruction_pcs`
///   与 `spans` 一一对应，后者直接取降低期已经定好的 `instruction.span`，
///   编码器不重算区间。
/// - `pc` 用 `checked_add` 累加，溢出报字段名，而不是回绕成一个能解码但全错的
///   目录。
///
/// handler 条目写两份信息：物理 pc 与逻辑块号。pc 是异常路由真正命中的边界，
/// 块号则是为了空块（相邻块 pc 相同）仍能无损往返——只留 pc 会把两个块合并成
/// 一个。
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

/// 写一条指令：opcode 字节、可选 `dst`、再由 [`encode_op`] 写操作数。
///
/// `dst` 走可选索引，因此 `None` 与 `VReg(0)` 在字节上是不同的东西——把空结果
/// 编码成 0 号寄存器会让解码端凭空多出一个写入目标。
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

/// 写指令 pc → 源码区间的增量表。
///
/// 两个方向都用**增量**而不是绝对值：指令在函数内密集排列，源码区间在降低期
/// 也只做局部回填，增量几乎全是小数字，比重复写绝对值省得多。pc 增量为无符号
/// （必须非递减，否则说明块内指令 pc 排错了）；源码起止为**有符号**增量，因为
/// 回填常常往左跳，用无符号会直接溢出。首条的基准是 0。
///
/// `span.start > span.end` 在这里就拒绝：半开区间不变量是下游一切反查的前提，
/// 不能等解码端或消费者去发现。
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

/// 写一条指令的操作数（opcode 已由 [`encode_instruction`] 写出）。
///
/// 操作数形态决定了用哪种写入口：寄存器号和表索引走 `Writer::index`（跟随
/// 头部声明的宽度），集合长度走 `Writer::count`，源码文本/名称走 `Writer::string`。
/// `IndexGet` 的路径段自带标签（数字索引 0、键 1），数字索引用 `sleb` 写——
/// 源码里的负索引有语义，运行时按容器长度归一化，编码期不允许改写成无符号。
///
/// `RunReleasePlan` 在这里补一次 `(scope, exit)` 存在性检查：解码端只会照抄这
/// 一对去查计划表，写出一个查不到的引用等于产出不可执行的编码。
///
/// `_function` 当前不参与载荷编码（引用校验已在 [`validate_references`] 完成），
/// 参数保留是为了与 [`validate_op`] 的形态对称，也留给将来需要函数内上下文的
/// 操作数。
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
        TacOp::SelectorApply {
            source,
            plan,
            step,
            random_counts,
        } => {
            writer.index(source.get(), "VReg")?;
            writer.index(*plan, "SelectionPlanId")?;
            write_optional_index(writer, step.map(VReg::get))?;
            writer.count(random_counts.len(), "selector.random_counts")?;
            for value in random_counts {
                write_optional_index(writer, value.map(VReg::get))?;
            }
            Ok(())
        }
        TacOp::BroadcastAssign { root, value, plan } => {
            writer.index(root.get(), "VReg")?;
            writer.index(value.get(), "VReg")?;
            writer.index(*plan, "BroadcastPlanId")
        }
        TacOp::RandomSeed { value, plan } => {
            writer.index(value.get(), "VReg")?;
            writer.index(*plan, "RandomSeedPlanId")
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

/// 写调用实参序列：每条写类别标签、可选名字、寄存器。
///
/// 名字用可选字符串而不是「关键字实参必有名字」的假设：为位置实参编造空名会
/// 让解码端拿到一个 `Some("")`，语义上和 `None` 不同。
fn encode_arguments(writer: &mut Writer, arguments: &[TacArgument]) -> Result<(), EncodeError> {
    writer.count(arguments.len(), "call.arguments")?;
    for argument in arguments {
        writer.byte(arg_kind_tag(argument.kind));
        write_optional_string(writer, argument.name.as_deref())?;
        writer.index(argument.value.get(), "VReg")?;
    }
    Ok(())
}

/// 写一段寄存器编号序列：先数量再逐项。
///
/// 数组元素、元组元素、集合元素、形参、局部都复用这里。空序列写出一个 0，
/// 是合法且常见的情况（无参函数）。
fn encode_vregs(writer: &mut Writer, values: &[VReg]) -> Result<(), EncodeError> {
    writer.count(values.len(), "vregs")?;
    for value in values {
        writer.index(value.get(), "VReg")?;
    }
    Ok(())
}

/// 写寄存器类别表：只写类别标签，不写编号。
///
/// [`CategoryMap::iter`] 已经按寄存器编号稠密展开（未登记的编号是 `Poly`），
/// 所以「位置即编号」——解码端用 [`CategoryMap::from_classes`] 就能恢复同一张
/// 表。每个函数都携带自己那份类别，因为 `VReg` 在每个函数重新从零编号。
fn encode_categories(writer: &mut Writer, categories: &CategoryMap) -> Result<(), EncodeError> {
    writer.count(categories.len(), "categories")?;
    for class in categories.iter() {
        writer.byte(register_class_tag(class));
    }
    Ok(())
}

/// 写冻结释放计划表：按 `(scope, exit, actions, transferred)` 逐条写。
///
/// 动作保持计划内的 `order` 原样写出，**不重排、不去重**：释放顺序是
/// `xiao-lifetime` 冻结的语义，编码器只负责搬运。`transferred` 是「已转移出去、
/// 不在此处释放」的值编号，它和 `actions` 是两个独立的集合，不能互相推导。
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

/// 写类型阶段的选择计划表。
///
/// 计划镜像已经是 `xiao-ir` 的稳定 serde 数据对象；这里按条目写 JSON 文本，
/// 让编码器只负责搬运而不复制计划字段的标签映射。
fn encode_selection_plans(writer: &mut Writer, program: &TacProgram) -> Result<(), EncodeError> {
    writer.count(program.selection_plans.len(), "selection_plans")?;
    for plan in &program.selection_plans {
        let json = serde_json::to_string(plan)
            .map_err(|error| EncodeError::InvalidFormat(format!("选择计划序列化失败：{error}")))?;
        writer.string(&json)?;
    }
    Ok(())
}

/// 写事务性广播计划表。
fn encode_broadcast_plans(writer: &mut Writer, program: &TacProgram) -> Result<(), EncodeError> {
    writer.count(
        program.broadcast_assignment_plans.len(),
        "broadcast_assignment_plans",
    )?;
    for plan in &program.broadcast_assignment_plans {
        let json = serde_json::to_string(plan)
            .map_err(|error| EncodeError::InvalidFormat(format!("广播计划序列化失败：{error}")))?;
        writer.string(&json)?;
    }
    Ok(())
}

/// 写随机种子计划表。
fn encode_random_seed_plans(writer: &mut Writer, program: &TacProgram) -> Result<(), EncodeError> {
    writer.count(program.random_seed_plans.len(), "random_seed_plans")?;
    for plan in &program.random_seed_plans {
        let json = serde_json::to_string(plan).map_err(|error| {
            EncodeError::InvalidFormat(format!("随机种子计划序列化失败：{error}"))
        })?;
        writer.string(&json)?;
    }
    Ok(())
}

/// 写一个独立源码区间（函数级 `span`），起止各一个 uleb。
///
/// 与 [`encode_span_map`] 的增量表不同，这里只有一条，绝对值反而更直接。同样
/// 先验半开区间不变量再写。
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

/// 递归写 `IrType`：1 字节标签加载荷。
///
/// 几处刻意不合并的地方：`DictTable` 与 `DictColumn` 共用同一段条目编码，但标签
/// 分成 6/7，因为两者在物理布局上是不同的东西；`Set` 额外写三个布尔标志
/// （允许动态成员、已知为空、未知），它们描述的是不同的类型事实，压成一个
/// 「unknown」会丢信息。`Function` 先写形参数组再写返回类型，顺序固定。
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

/// 写数组形状：同质带元素类型与可选长度，异质带逐位置元素类型，未知只有标签。
///
/// 长度是可选值，因为 `Some(0)`（确定为空）与 `None`（长度运行期才定）是两个
/// 不同的形状，不能互相替代。
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

/// 写字典类型的条目：键字符串加值类型。
///
/// 键保持源码文本原样，**不做任何规范化**：它是 `IndexGet` 精确路径的匹配依据，
/// 大小写或转义的改写会让类型信息与实际索引对不上。
fn encode_dict_types(writer: &mut Writer, entries: &[IrDictTypeEntry]) -> Result<(), EncodeError> {
    writer.count(entries.len(), "dict_type.entries")?;
    for entry in entries {
        writer.string(&entry.key)?;
        encode_type(writer, &entry.value)?;
    }
    Ok(())
}

/// 解码主体，返回语义模型与**头部声明的**操作数宽度。
///
/// 读出顺序即格式顺序：魔数 → 布局版本 → 宽度标签 → 四个版本字段 → 语言版本
/// 与目标 → 常量池 → 签名表 → 函数 → 程序级类别兼容视图 → 释放计划 →
/// `unsupported` 说明。版本字段在读到时就逐个比对当前实现，不等读完再判。
///
/// 末尾要求恰好消费完：剩余任何字节都报 [`EncodeError::TrailingBytes`]。静默
/// 忽略尾部会让「编码器多写了一段」这类 bug 永远浮不出来——解码成功、数据却
/// 不是写入时的全部内容。
///
/// 返回宽度而不是丢掉，是因为 [`decode_encoded`] 要按**同一宽度**重新编码才能
/// 重建物理目录。
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
    let selection_plans = decode_selection_plans(&mut reader)?;
    let broadcast_assignment_plans = decode_broadcast_plans(&mut reader)?;
    let random_seed_plans = decode_random_seed_plans(&mut reader)?;
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
            selection_plans,
            broadcast_assignment_plans,
            random_seed_plans,
            unsupported,
        },
        width,
    ))
}

/// 按标签还原常量池。
///
/// 浮点按**位**还原（`from_bits`），保住 `-0.0` 的符号位和 NaN 的载荷位——
/// 这两者在 `PartialEq` 下都等于「随便一个同类值」，只有位级往返才能证明没丢。
/// `Lint`/`Lfloat` 只做 UTF-8 还原，十进制文本的规范化由消费方负责。
///
/// `width` 参数在本函数里用不到：常量池按索引顺序整体写出，条目内部既没有寄存器
/// 号也没有表索引，因此不参与两种操作数宽度的分叉。
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

/// 还原签名表：逐参数读名字、类别标签、类型、默认值标志，再读两个可选槽位与
/// 返回类型。
///
/// 两个槽位用 `optional_index` 读，因此必须按头部声明的宽度解析——签名表里的
/// 槽位是寄存器序号，和指令操作数受同一条宽度策略约束。
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

/// 还原一个函数，并在读的过程中重建它的物理 pc 目录。
///
/// 三处结构约束必须在这里守住：
///
/// - 块目录严格按 `BlockId` 从 0 递增。编码端按位置写块，只有这条成立才能保证
///   `blocks[i].id == i`，后面 handler 的块号才能直接当索引用。
/// - `value_registers` 出现重复值编号直接拒绝：查表结果会取决于插入顺序，
///   同一份字节能解出两种映射。
/// - 块字节串在声明的指令数之后必须为空。多出来的字节没有归属，读掉它就等于
///   承认编码有歧义。
///
/// 指令的源码区间**不在**指令流里，而在块字节之后的增量表里；读完后按位置回填
/// 到每条指令。
///
/// handler 的 pc 与块号写了两份，这里逐条交叉校验：pc 必须等于该块在函数内的
/// 起始 pc（保护区间终点额外允许等于函数 `code_len`，对应「排他上界越过最后
/// 一块」）。解码端不信任 pc，只信块目录里推出来的值。
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

/// 核对 handler 里写的物理 pc 与块目录推出的 pc。
///
/// `expected` 为 `None` 表示块号超出了块目录，而 pc 位置本身是合法的——这种
/// 组合只可能来自损坏或手写字节，报 [`EncodeError::InvalidReference`]，`limit`
/// 填 0 表示「以块目录为准，没有可比的长度」。
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

/// 读一条指令：opcode、可选 `dst`，再交给 [`decode_op`] 读操作数。
///
/// 源码区间先留成 `IrSpan::new(0, 0)`，由 [`decode_span_map`] 按 pc 回填——
/// 指令流本身不携带区间，硬在这里编一个默认区间会让「回填漏了一条」变成静默
/// 的假数据。
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

/// 按 opcode 还原操作数，与 [`encode_op`] 的写出顺序逐条对应。
///
/// 所有寄存器号与表索引都按头部声明的宽度读（`Leb128` 或定宽 `u16`），所以同一
/// 段字节在两种宽度下含义不同——宽度是格式的一部分，必须一路传到底。未知 opcode
/// 报 [`EncodeError::UnknownOpcode`]：跳过它会让后面所有操作数错位，产出一份
/// 「能解码但指令全错」的程序。
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
        31 => {
            let source = VReg::new(index(reader, "VReg")?);
            let plan = index(reader, "SelectionPlanId")?;
            let step = reader
                .optional_index(width, "selector.step")?
                .map(VReg::new);
            let count = reader.count("selector.random_counts")?;
            let mut random_counts = Vec::with_capacity(count);
            for _ in 0..count {
                random_counts.push(
                    reader
                        .optional_index(width, "selector.random_count")?
                        .map(VReg::new),
                );
            }
            TacOp::SelectorApply {
                source,
                plan,
                step,
                random_counts,
            }
        }
        32 => TacOp::BroadcastAssign {
            root: VReg::new(index(reader, "VReg")?),
            value: VReg::new(index(reader, "VReg")?),
            plan: index(reader, "BroadcastPlanId")?,
        },
        33 => TacOp::RandomSeed {
            value: VReg::new(index(reader, "VReg")?),
            plan: index(reader, "RandomSeedPlanId")?,
        },
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

/// 读调用实参序列，与 [`encode_arguments`] 对称：类别标签、可选名字、寄存器。
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

/// 读字典字面量的键值对：键是 UTF-8 文本，值是寄存器。
///
/// 键值对按写出顺序保留，不去重也不排序——字典字面量里重复的键是源码允许的，
/// 谁赢由运行时的构造语义决定，编码层不能替它做决定。
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

/// 读一段寄存器编号序列。
///
/// `field` 由调用方给出，用来区分数组元素、元组元素、集合元素、形参、局部这些
/// 形态相同但位置不同的长度字段——报错时只说「vregs」定位不到是哪一个序列。
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

/// 读稠密类别表并重建 `CategoryMap`。
///
/// 表里只有标签，位置就是寄存器编号；编码端已经把空洞补成 `Poly`，所以这里读
/// 出来的长度同时就是编号上界。`field` 区分程序级入口兼容视图与函数级类别表
/// （两者格式相同，出错的归属不同）。
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

/// 还原冻结释放计划表。
///
/// `order` 用 [`Reader::usize_uleb`] 而不是 `u32_uleb`，因为它是解释器执行顺序的
/// 直接依据，宽度跟宿主 `usize` 走；`scope`/`value` 这些编号仍是 `u32`，宽度跟
/// 模型里的字段类型走。计划不参与两种操作数宽度策略——里面的编号是值编号不是
/// 寄存器号。
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

/// 还原类型阶段选择计划表。
fn decode_selection_plans(
    reader: &mut Reader<'_>,
) -> Result<Vec<xiao_ir::IrSelectionPlan>, EncodeError> {
    let count = reader.count("selection_plans")?;
    let mut plans = Vec::with_capacity(count);
    for _ in 0..count {
        let json = reader.string("selection_plan")?;
        let plan = serde_json::from_str(&json).map_err(|error| {
            EncodeError::InvalidFormat(format!("选择计划反序列化失败：{error}"))
        })?;
        plans.push(plan);
    }
    Ok(plans)
}

/// 还原事务性广播计划表。
fn decode_broadcast_plans(
    reader: &mut Reader<'_>,
) -> Result<Vec<xiao_ir::IrBroadcastAssignmentPlan>, EncodeError> {
    let count = reader.count("broadcast_assignment_plans")?;
    let mut plans = Vec::with_capacity(count);
    for _ in 0..count {
        let json = reader.string("broadcast_assignment_plan")?;
        let plan = serde_json::from_str(&json).map_err(|error| {
            EncodeError::InvalidFormat(format!("广播计划反序列化失败：{error}"))
        })?;
        plans.push(plan);
    }
    Ok(plans)
}

/// 还原随机种子计划表。
fn decode_random_seed_plans(
    reader: &mut Reader<'_>,
) -> Result<Vec<xiao_ir::IrRandomSeedPlan>, EncodeError> {
    let count = reader.count("random_seed_plans")?;
    let mut plans = Vec::with_capacity(count);
    for _ in 0..count {
        let json = reader.string("random_seed_plan")?;
        let plan = serde_json::from_str(&json).map_err(|error| {
            EncodeError::InvalidFormat(format!("随机种子计划反序列化失败：{error}"))
        })?;
        plans.push(plan);
    }
    Ok(plans)
}

/// 读函数级独立源码区间（起止各一个 uleb）。
///
/// 倒置区间在这里就拒绝，和 [`encode_span`] 是同一条不变量的两侧。
fn decode_span(reader: &mut Reader<'_>) -> Result<IrSpan, EncodeError> {
    let start = reader.usize_uleb("span.start")?;
    let end = reader.usize_uleb("span.end")?;
    if start > end {
        return Err(EncodeError::InvalidSpan { start, end });
    }
    Ok(IrSpan::new(start, end))
}

/// 读增量源码映射表，并与指令目录交叉校验后返回逐指令区间。
///
/// 三条硬校验：数量必须等于该块的指令数；每条 delta 累加出的 pc 必须**精确
/// 等于**对应指令的 pc；区间不得倒置。这样任何一处错位都会立刻变成结构错误，
/// 而不是产出一张「查得到但查不准」的映射表——后者会让错误堆栈指向无关的源码
/// 位置，比直接失败危险得多。
///
/// 与 [`encode_span_map`] 对称：pc 走无符号增量，源码起止走有符号增量并经由
/// [`add_signed_usize`] 落地。
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

/// 把有符号增量加到一个无符号基准上，正负两侧都防溢出。
///
/// 源码区间在降低期既可能向右也可能向左回填，所以负增量是正常的，但结果落到
/// 负数（或加爆 `usize`）说明增量表本身损坏，必须报错而不是回绕。错误统一记成
/// `IntegerOverflow`，因为两种情况的处置相同：这份字节不可用。
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

/// 递归还原 `IrType`，标签与 [`encode_type`] 一一对应。
///
/// `DictTable`/`DictColumn` 由 6/7 共用同一段条目读取后再分支，`Set` 的三个布尔
/// 标志分开读——它们顺序固定，不能靠「读到一个 0」去猜剩下还有没有。未知标签
/// 报 [`EncodeError::InvalidEnum`]，不猜成 `Dynamic`。
///
/// 类型是递归结构，每层至少消耗一个标签字节，递归深度因此受输入长度约束而不是
/// 靠单独设限。
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

/// 还原数组形状。
///
/// `Homogeneous` 的长度是可选 uleb：缺失表示长度运行期才定，与 `Some(0)` 的
/// 「确定为空」不是一回事，不能用 0 顶替 `None`。
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

/// 解码产物的整体校验。
///
/// 目前**与 [`validate_input`] 完全等价**（直接转调）：两侧必须用同一套规则，
/// 否则会出现「编码通过、解出来的程序却不合法」的缝隙——同一份字节在两处得到
/// 不同判定，谁对谁错没有依据。保留独立函数名，是为了让「解码后还有一次校验」
/// 在两个调用点显式可见，而不是靠读者自己去追被调函数。
fn validate_decoded(program: &TacProgram, width: OperandWidth) -> Result<(), EncodeError> {
    validate_input(program, width)
}

/// `TacOp` 到稳定 opcode 的映射，0–33 连续。
///
/// 这张表是格式的核心契约：**只能追加，不得重排**。调换两个编号会让旧字节被读
/// 成另一种指令，而这种错误在往返测试里是看不出来的（编码器和解码器用的是同一
/// 张表）。`all_ops_program` 里断言了 `ops` 的顺序恰好产生 `0..34`，新增变体插在
/// 中间会立刻失败。
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
        TacOp::SelectorApply { .. } => 31,
        TacOp::BroadcastAssign { .. } => 32,
        TacOp::RandomSeed { .. } => 33,
    }
}

/// `ScalarType` 与稳定标签的双向表。
///
/// 用数组而不是两段 `match`，是为了让正反两个方向共用同一份数据：写成两段
/// `match` 时「正向写 3、反向读回 4」这种错配编译得过去，测试也未必覆盖到。
/// 顺序即标签，改动等于改格式。
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

/// 查 `ScalarType` 的正向标签。
///
/// **穷尽匹配，没有兜底分支**：`ScalarType` 新增变体而没同步本函数时，这里会
/// **编译失败**，强制作者回来处理。
///
/// 曾经写成「查 [`SCALAR_TAGS`]，查不到退回 0」——那样确实不会崩，但会**静默写出
/// 一个错误标签**，产出一份「能解码、标量类型却被悄悄改写」的字节。错误推迟到
/// 计算结果不对时才暴露，而且没有任何一层能指出是编码器写错了。编译失败比这
/// 危险得多地便宜。
///
/// 取值必须与 [`SCALAR_TAGS`] 一致；两者之间的漂移由
/// `scalar_and_release_tags_have_one_bidirectional_mapping` 逐条钉住。
const fn scalar_tag(value: ScalarType) -> u8 {
    match value {
        ScalarType::Int => 0,
        ScalarType::Sint => 1,
        ScalarType::Lint => 2,
        ScalarType::Float => 3,
        ScalarType::Sfloat => 4,
        ScalarType::Lfloat => 5,
        ScalarType::Str => 6,
        ScalarType::Bool => 7,
    }
}

/// 按标签还原标量类型；未知标签报 [`EncodeError::InvalidEnum`]。
///
/// 不退回默认标量：静默退回会让一份损坏字节被当成合法程序继续往执行器走，
/// 错误会推迟到计算结果不对的时候才暴露。
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

/// `ReleaseActionKind` 到标签的映射，直接取 `ALL` 数组下标。
///
/// 标签顺序就是 `xiao-lifetime` 冻结的枚举顺序，所以这条映射是与上游的接口
/// 契约而不是本地编号：上游调整 `ALL` 的次序就等于改格式。查不到时报
/// [`EncodeError::InvalidEnum`]，`value` 填 `u64::MAX`——出错的是「这个值不在
/// `ALL` 里」，没有可报的输入标签。
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

/// 按 `ALL` 下标还原释放动作类别；越界报 [`EncodeError::InvalidEnum`]。
///
/// 用 `ALL.get` 而不是下标索引，是因为 `tag` 直接来自输入字节：索引越界会 panic，
/// 而这里需要的是一条结构化错误。强释放与弱释放的区分决定引用计数是否递减，
/// 读错标签会静默改变释放语义。
fn release_from_tag(tag: u8) -> Result<ReleaseActionKind, EncodeError> {
    ReleaseActionKind::ALL
        .get(tag as usize)
        .copied()
        .ok_or_else(|| EncodeError::InvalidEnum {
            field: "ReleaseActionKind".to_owned(),
            value: tag as u64,
        })
}

/// 写 `ArithOp` 的稳定标签（加 0 到幂 6）。
///
/// 和 opcode 表一样只能追加：重排会让旧字节被当成另一种运算执行。算术错误不会
/// 被结构校验发现，只会在结果里体现出来。
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

/// 按标签还原算术运算；未知标签报 [`EncodeError::InvalidEnum`]。
///
/// 这里必须报错而不是挑一个默认运算：除法与取模的编号相邻，静默兜底会把一份
/// 损坏字节变成一次「合法的」错运算。
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

/// 写 `CompareOp` 的稳定标签（`<` 0 到 `!=` 5）。
///
/// 顺序即标签，只可追加。比较结果恒为布尔，所以标签错位不会被类别检查发现——
/// 仍然是同一类值，只是比较的语义变了。
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

/// 按标签还原比较运算；未知标签报 [`EncodeError::InvalidEnum`]。
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

/// 写实参类别标签（位置 0、关键字 1、`*` 2、`**` 3）。
///
/// 类别决定被调方怎么绑定形参：位置实参按顺序占槽，关键字实参按名字找，
/// `*`/`**` 展开成变参。读错标签不会越界，只会把值绑到别的形参上。
fn arg_kind_tag(value: ArgKind) -> u8 {
    match value {
        ArgKind::Positional => 0,
        ArgKind::Keyword => 1,
        ArgKind::VarArgs => 2,
        ArgKind::KwArgs => 3,
    }
}

/// 按标签还原实参类别；未知标签报 [`EncodeError::InvalidEnum`]。
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

/// 写形参类别标签（位置或关键字 0、位置专用 1、关键字专用 2、`*args` 3、
/// `**kwargs` 4）。
///
/// 标签顺序与 [`ParamKind`] 的变体声明顺序一致，但**它本身是格式**：变体顺序
/// 变了就得同步改这里，不能靠「声明顺序即标签」的默契。
fn param_kind_tag(value: ParamKind) -> u8 {
    match value {
        ParamKind::PositionalOrKeyword => 0,
        ParamKind::PositionalOnly => 1,
        ParamKind::KeywordOnly => 2,
        ParamKind::VarArgs => 3,
        ParamKind::VarKeywords => 4,
    }
}

/// 按标签还原形参类别；未知标签报 [`EncodeError::InvalidEnum`]。
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

/// 写寄存器类别标签（整数 0、浮点 1、布尔 2、对象句柄 3、动态 4、无 5、合流 6）。
///
/// 类别是物理分配的依据：它决定值落在寄存器文件还是帧槽、要不要带运行时类型标
/// 签。标签错位不会让编码失败，只会让分配器把对象句柄当整数处理。
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

/// 按标签还原寄存器类别；未知标签报 [`EncodeError::InvalidEnum`]。
///
/// 不退回 [`RegisterClass::Poly`]：`Poly` 是「合流点无法收敛」这一具体事实的
/// 表示，用它兜底会把「类别未知」和「类别确实退化」混成一种。
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

/// 写可选索引：`None` 写一个 0 字节，`Some` 先写 1 再按当前宽度写编号。
///
/// 不用「0 表示空」是因为 0 是合法编号——0 号常量、0 号函数（脚本入口）、
/// 0 号寄存器都存在，用 0 当哨兵会把一个真实引用变成「没有」。
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

/// 写可选字符串：`None` 写一个 0 字节，`Some` 先写 1 再写长度前缀文本。
///
/// 同样不能拿空串当哨兵：`Some("")`（比如无名的关键字实参、空的捕获类型名）与
/// `None`（根本没有这一项）在模型里是不同的值。
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

/// 写可选长度：`None` 写一个 0 字节，`Some` 先写 1 再写 uleb。
///
/// 与 [`write_optional_index`] 分开，是因为这里的载荷**固定用 uleb**，不跟随头部
/// 的操作数宽度：可选长度只出现在类型形状里（数组长度），而定宽策略只约束寄存器
/// 号与表索引。
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

/// 读一个布尔标志，只接受 0 和 1。
///
/// 其他任何值都报 [`EncodeError::InvalidEnum`]，不按「非零即真」处理：把 2 当作
/// `true` 会把一个已经损坏的字段静默吞掉，后面再想定位就无从下手。
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

/// 编码写入器：累积字节串，并记住本次编码的操作数宽度。
///
/// 宽度存在写入器里而不是每个 [`Writer::index`] 调用点各传一次，是为了让一段
/// 字节里**不可能**混进两种宽度的编号——解码端只有头部一个宽度标签，混写就是
/// 不可解。
struct Writer {
    /// 已写出的字节。
    bytes: Vec<u8>,
    /// 本次编码使用的操作数宽度，写编号时生效。
    width: OperandWidth,
}

impl Writer {
    /// 建一个空写入器，绑定本次编码的宽度。
    ///
    /// 是 `const`，方便在常量语境（比如块级写入器的初始化）里构造。
    const fn new(width: OperandWidth) -> Self {
        Self {
            bytes: Vec::new(),
            width,
        }
    }

    /// 写一个原始字节（标签、布尔、opcode 都用它）。
    ///
    /// 不做长度或范围检查：能走到这里的值都已经由调用方决定了语义，检查放在
    /// 有字段名可用的一层（比如 [`Writer::index`]）才报得清楚。
    fn byte(&mut self, value: u8) {
        self.bytes.push(value);
    }

    /// 写无符号 LEB128：每字节取低 7 位，最高位表示「后面还有」。
    ///
    /// 0 写成一个 `0x00`，即最小表示唯一——不写补零的冗余形式，否则同一份语义
    /// 会有多种字节表示，往返测试也就失去意义。
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

    /// 写有符号 LEB128：低 7 位加符号位，终止条件是「剩余位全 0（正）或全 1（负）」。
    ///
    /// 取 `i128` 而不是更窄的整数，是因为源码区间的增量可以横跨整个 `usize`
    /// 范围；按 `i64` 实现会在 64 位平台上截断极大的区间差。
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

    /// 按当前宽度写一个寄存器号或表索引。
    ///
    /// `FixedU16` 下超出 `u16` 范围直接报 [`EncodeError::IntegerOverflow`] 并带上
    /// `field`。这正是定宽操作数的契约：编号放不下时必须让编码失败，而不是截断
    /// 成一个指向别的表项的合法编号。
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

    /// 写长度前缀加 UTF-8 字节串。
    ///
    /// 长度是**字节数**不是字符数，所以中文、含 `\0` 的内容都能原样往返（`\0`
    /// 在源码字符串里是合法字符，不是终止符）。超过 [`MAX_STRING`] 拒绝。
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

    /// 写一个集合数量或字节长度。
    ///
    /// 超过 [`MAX_COLLECTION`] 拒绝：写入方向也卡这个上限，是为了保证「能编出来
    /// 的字节一定能解回来」——解码端有同样的上限，写入端不卡就会产出自己读不了
    /// 的编码。
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

/// 解码读取器：一个字节切片加当前消费位置。
///
/// **不保存操作数宽度**：宽度是头部的属性，在每次 [`Reader::index`] /
/// [`Reader::optional_index`] 调用点显式传入。这样同一个读取器既能读入口流
/// （头部自带宽度标签），也能读块字节这类「宽度由外层决定」的子串。
struct Reader<'a> {
    /// 剩余待消费的字节。
    bytes: &'a [u8],
    /// 已消费的字节数，同时是下一条指令的函数内 pc 基准。
    offset: usize,
}

impl<'a> Reader<'a> {
    /// 从字节切片从头开始读。
    ///
    /// 适合整段入口流：头部自己带着宽度标签，后续每次读取再显式传入宽度，因此
    /// 这里不需要挑一个默认宽度。
    fn new(bytes: &'a [u8]) -> Self {
        Self::with_width(bytes, OperandWidth::Leb128)
    }

    /// 带宽度标注的构造入口。
    ///
    /// 当前实现**刻意忽略该参数**：读取器不持有宽度，宽度一律在读取点传入。
    /// 保留这个入口是为了让「这段子串是按哪种宽度写的」在构造处写清楚，读代码的
    /// 人不必回头去追头部。
    fn with_width(bytes: &'a [u8], _width: OperandWidth) -> Self {
        Self { bytes, offset: 0 }
    }

    /// 判断是否已消费到末尾。
    ///
    /// 整个编码流读完后必须为空，否则 [`decode_inner`] 会报 `TrailingBytes`。
    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }

    /// 返回尚未消费的字节数。
    ///
    /// 用于 `TrailingBytes` 诊断，也被 [`Reader::count`] 当作「这个长度有没有可能
    /// 装得下」的粗筛上界。
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    /// 读一个字节，越界报 [`EncodeError::UnexpectedEof`]，并带上 `context` 字段名，
    /// 让截断的位置可定位。
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

    /// 读走恰好 `length` 个字节并返回借用切片。
    ///
    /// `offset + length` 用 `checked_add`：长度是从输入里读出来的，可以大到让
    /// `usize` 回绕；回绕后的上界会落在切片内，于是「越界」变成一次成功的读取。
    /// 越界统一报 [`EncodeError::UnexpectedEof`]，与 `byte` 保持一致。
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

    /// 读定宽 `N` 字节并转成数组，供整数/浮点按小端还原。
    ///
    /// `N` 由调用点从常量给出（整数 8/4、定宽操作数 2），所以长度不足只可能是
    /// 输入被截断，报 [`EncodeError::UnexpectedEof`]。
    fn fixed<const N: usize>(&mut self, context: &'static str) -> Result<[u8; N], EncodeError> {
        self.take_exact(N, context)?
            .try_into()
            .map_err(|_| EncodeError::UnexpectedEof {
                context: context.to_owned(),
            })
    }

    /// 读无符号 LEB128，上限 10 字节（`u64` 的宽度）。
    ///
    /// 每步都检查位移：位移到 64 位以外、或最后一个可用字节带多余高位时报
    /// [`EncodeError::IntegerOverflow`]。不查的话多出来的位会被 `<<` 静默丢弃，
    /// 读出一个比实际小的数——而它多半会被当成合法的表索引或长度用下去。
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

    /// 读有符号 LEB128，按 `i128` 累积，最多 19 字节。
    ///
    /// 第 19 字节只剩 2 个有效位（`shift == 126`），此时还要检查它的其余位是不是
    /// 合法的符号扩展（正数只能剩 `0x00`/`0x01`，负数只能剩 `0x7e`/`0x7f`），
    /// 否则报 [`EncodeError::IntegerOverflow`]：不做这一步，一段超宽的编码会被
    /// 截成一个「看起来正常」的小区间。
    ///
    /// 用 `i128` 而不是 `i64`，是因为源码区间增量要能覆盖整个 `usize` 范围。
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

    /// 读 uleb 并收窄到 `u32`（作用域号、各类版本号用）。
    ///
    /// 放不进 `u32` 时报 [`EncodeError::IntegerOverflow`]，而不是截断取低 32 位。
    fn u32_uleb(&mut self, context: &'static str) -> Result<u32, EncodeError> {
        u32::try_from(self.uleb(context)?).map_err(|_| EncodeError::IntegerOverflow {
            field: context.to_owned(),
            value: u64::MAX,
        })
    }

    /// 读 uleb 并收窄到 `usize`。
    ///
    /// 用于 [`super::lower::TacReleaseAction::order`] 这类「宽度跟宿主走」的字段，
    /// 与 [`Reader::u32_uleb`] 分开写是为了让收窄目标在调用点一眼可辨。
    fn usize_uleb(&mut self, context: &'static str) -> Result<usize, EncodeError> {
        usize::try_from(self.uleb(context)?).map_err(|_| EncodeError::IntegerOverflow {
            field: context.to_owned(),
            value: u64::MAX,
        })
    }

    /// 读一个集合数量，三重校验后才交给调用方去预留容量。
    ///
    /// 1. 不超过 [`MAX_COLLECTION`]；
    /// 2. 能收窄到 `usize`；
    /// 3. 不超过剩余字节数加一——每个条目至少要占一个字节，超过这个上界说明长度
    ///    是伪造的。
    ///
    /// 第 3 条是防「几字节输入骗出巨大分配」的主要手段：前两条都只是定值上限，
    /// 只有拿剩余长度当上界才真正和输入规模挂钩。
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

    /// 读长度前缀加 UTF-8 字节串。
    ///
    /// 长度先卡 [`MAX_STRING`] 再收窄成 `usize` 才取字节。非 UTF-8 报
    /// [`EncodeError::InvalidFormat`] 而不是替换成 U+FFFD：静默替换会改掉标识符
    /// 和字符串常量的内容，而调用方拿到的仍是一个「成功」的结果。
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

    /// 读可选字符串，标志字节只接受 0/1（其他值报 [`EncodeError::InvalidEnum`]）。
    ///
    /// 与 [`write_optional_string`] 对称，`None` 和 `Some("")` 保持可区分。
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

    /// 按给定宽度读一个寄存器号或表索引。
    ///
    /// 这是「两种操作数宽度」在解码侧唯一的分叉点：`Leb128` 走 uleb，`FixedU16`
    /// 读小端两字节。宽度由调用方传入而不是从 `self` 取，见 [`Reader`] 的说明。
    fn index(&mut self, width: OperandWidth, context: &'static str) -> Result<u32, EncodeError> {
        match width {
            OperandWidth::Leb128 => self.u32_uleb(context),
            OperandWidth::FixedU16 => Ok(u16::from_le_bytes(self.fixed::<2>(context)?) as u32),
        }
    }

    /// 读可选编号：标志字节只接受 0/1（其他值报 [`EncodeError::InvalidEnum`]），
    /// 随后按给定宽度读编号。
    ///
    /// 与 [`write_optional_index`] 对称，`None` 与编号 0 保持可区分。
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

    /// 读可选长度（固定用 uleb）。
    ///
    /// 与 [`Reader::optional_index`] 分开，因为可选长度不参与定宽操作数策略；
    /// 目前唯一的用处是数组形状里的长度。
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

/// 定宽模式下检查一个编号能否放进 `u16`。
///
/// **目前不会真的报错**：唯一的调用点在 [`validate_references`] 里固定传
/// [`OperandWidth::Leb128`]，而该分支只在 `FixedU16` 下生效。定宽溢出实际是被
/// [`Writer::index`] 在写出时抓住的（字段名退化成 `optional_index` 之类）。
///
/// 保留它的意义是给「handler.binding 也受定宽约束」这件事留一个显式位置：
/// 那条检查发生在校验阶段，早于任何写出。等调用方能拿到真实宽度后它会立即生效，
/// 届时错误信息里的字段名会比写入端的通用名精确得多。
fn check_index_width(value: u64, field: &str, width: OperandWidth) -> Result<(), EncodeError> {
    if matches!(width, OperandWidth::FixedU16) && value > u16::MAX as u64 {
        return Err(EncodeError::IntegerOverflow {
            field: field.to_owned(),
            value,
        });
    }
    Ok(())
}

/// 本模块的结构性回归测试。
///
/// 覆盖的是**格式契约**而不是业务语义：全部 opcode 与 ABI 头字段在两种操作数
/// 宽度下往返、LEB128 的整数边界、损坏输入必须被结构性拒绝（而不是「随便报个
/// 错」）、越界引用与定宽溢出、以及标签表的双向唯一性。这些测试的存在理由是
/// 编码器与解码器共用同一批映射表——只做往返测试无法发现「两边一起错」，
/// 所以每张表都另有独立的断言。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::lower::{TacReleaseAction, TacReleasePlan};
    use xiao_ir::{IrSelectionItemPlan, IrSelectionPlan};

    /// 造一条测试指令：`dst` 与源码区间都由序号推出。
    ///
    /// 前 16 条带 `dst`、其余不带，覆盖可选字段的两侧；区间按序号错开，这样
    /// 「按 pc 反查」的断言能确认命中的是**哪一条**，而不只是「有命中」。
    fn instruction(index: usize, op: TacOp) -> TacInstr {
        TacInstr {
            op,
            dst: (index < 16).then(|| VReg::new((index + 20) as u32)),
            span: IrSpan::new(100 + index * 3, 102 + index * 3),
        }
    }

    /// 枚举 `IrType` 的每个变体，供类型编码覆盖测试使用。
    ///
    /// 包含三种数组形状、两种字典、`Table` 与 `Dynamic`。`Set` 刻意取
    /// `allows_dynamic = true`、`empty = false`、`unknown = true` 这种非全零也
    /// 非全一的组合，这样三个标志写串顺序或写错一个都能被发现。
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

    /// 构造一份用满全部 34 个 opcode 的 TAC 程序。
    ///
    /// 刻意把每个「难往返」的角落都填上：常量池里有大整数、位模式特殊的浮点
    /// （NaN 载荷、`-0.0`）、超长精度文本和带 `\0` 的中文串；签名表覆盖五种
    /// [`ParamKind`] 与 `*args`/`**kwargs` 槽位；函数带类别表、值→寄存器映射、
    /// handler、释放计划、选择计划与广播/种子计划；块从 34 条指令骤降到 1 条，条数不整齐。
    ///
    /// 函数内的两条断言是**格式守卫**：`ops` 的顺序必须恰好产生 `0..34` 的
    /// opcode。新增变体若插在表中间而不是追加到末尾，这里会先失败，而不是等到
    /// 某天有人拿旧字节解码才发现指令错位。
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
        let selection_plan = IrSelectionPlan {
            span: IrSpan::new(1, 2),
            source_type: IrType::Dynamic,
            result_type: IrType::Dynamic,
            items: vec![IrSelectionItemPlan::All],
            selected_paths: Vec::new(),
            target_types: Vec::new(),
            step: None,
            requires_runtime_check: false,
            with_replacement: false,
            has_duplicates: false,
        };
        let random_seed_plan = xiao_ir::IrRandomSeedPlan {
            span: IrSpan::new(3, 4),
            value: Some(7),
            dynamic: false,
        };
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
            TacOp::SelectorApply {
                source: VReg::new(18),
                plan: 0,
                step: None,
                random_counts: vec![None],
            },
            TacOp::BroadcastAssign {
                root: VReg::new(19),
                value: VReg::new(20),
                plan: 0,
            },
            TacOp::RandomSeed {
                value: VReg::new(21),
                plan: 0,
            },
        ];
        assert_eq!(ops.len(), 34);
        let opcodes = ops.iter().map(opcode).collect::<Vec<_>>();
        assert_eq!(opcodes, (0_u8..34).collect::<Vec<_>>());

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
            selection_plans: vec![selection_plan],
            broadcast_assignment_plans: vec![xiao_ir::IrBroadcastAssignmentPlan {
                span: IrSpan::new(5, 6),
                root_name: Some("values".to_owned()),
                target_paths: Vec::new(),
                value_type: IrType::Dynamic,
                dynamic: false,
                transactional: true,
            }],
            random_seed_plans: vec![random_seed_plan],
            unsupported: Vec::new(),
        }
    }

    /// 比较两个常量，`Float`/`Sfloat` 按**位**比较。
    ///
    /// `PartialEq` 在浮点上放过两类关键变化：`-0.0 == 0.0` 为真，NaN 不等于自身。
    /// 用位比较才能证明编码往返没有改动符号位与 NaN 载荷。
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

    /// 逐字段比较两份 TAC 程序。
    ///
    /// 常量池单独处理：先比数量再逐项走 [`assert_constant_eq`]（浮点要按位比）。
    /// 其余字段直接结构相等。
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

    /// 两种操作数宽度下「编码 → 自校验 → 解码」都必须与原程序一致。
    ///
    /// 除了往返，这里还钉住几条容易被假通过掩盖的性质：物理 pc **不得**退化成
    /// 源码偏移（用 `assert_ne!` 显式排除这种实现），按 pc 反查在指向指令中间
    /// 字节时仍命中、指向函数尾部（`code_len`）时返回 `None`，第二块的 pc 大于 0，
    /// 以及小编号下 LEB128 确实比定宽更短（否则定宽策略就失去存在意义）。
    #[test]
    fn all_opcodes_and_abi_fields_round_trip_in_both_widths() {
        let program = all_ops_program();
        let mut sizes = Vec::new();
        for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
            let encoded = encode(&program, width).expect("完整 TAC 应可编码");
            validate_encoded(&encoded).expect("编码应可自校验");
            let decoded = decode(&encoded.bytes).expect("完整 TAC 应可解码");
            assert_program_eq(&program, &decoded);
            assert_eq!(encoded.functions[0].blocks[0].instruction_pcs.len(), 34);
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

    /// 覆盖 uleb 与 sleb 的整数边界：0、127/128（单字节与双字节的分界）、
    /// `u32::MAX`，以及 `i128::MIN`/`MAX`、-1、-129。
    ///
    /// 先写进同一个写入器再顺序读回，最后断言读取器恰好空——把「写完还有残留」
    /// 和「多读了一个字节」一并挡住。
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

    /// 逐类破坏字节流，断言报出的是**对应的**结构化错误。
    ///
    /// 覆盖：改动 `bytecode_abi_version` 字段必须报 `VersionMismatch` 且带上字段名；
    /// 截断尾部报 `UnexpectedEof` 或 `InvalidLength`（取决于截在哪个位置）；多写
    /// 一个字节报 `TrailingBytes`；未知 opcode、未知释放类别各自报 `UnknownOpcode`
    /// 与 `InvalidEnum`；超长字符串报 `InvalidLength`。
    ///
    /// 这些断言刻意匹配具体错误而不是 `is_err()`：一个损坏输入「恰好」被别的原因
    /// 拒绝掉，才算真正的测试通过。
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

    /// 定宽模式下 `handler.binding` 放不下时，必须报出**精确字段名**。
    ///
    /// 这条检查曾经是死的：调用点硬编码传 `OperandWidth::Leb128`，而该函数只在
    /// `FixedU16` 分支才可能报错，所以它永远不会触发——真正兜住越界的是写出时的
    /// `Writer::index`，报的是通用字段名。**撤掉宽度透传（改回 `Leb128`），本用例
    /// 必须失败。**
    #[test]
    fn fixed_width_reports_the_overflowing_handler_binding() {
        let mut program = all_ops_program();
        let handler = program
            .functions
            .iter_mut()
            .flat_map(|function| function.handlers.iter_mut())
            .next()
            .expect("夹具应带 handler");
        handler.binding = Some(VReg::new(u32::from(u16::MAX) + 1));

        let error = encode_with_width(&program, OperandWidth::FixedU16)
            .expect_err("定宽模式下越界绑定必须被拒绝");
        match error {
            EncodeError::IntegerOverflow { field, .. } => assert_eq!(field, "handler.binding"),
            other => panic!("应报字段级溢出而不是通用错误: {other:?}"),
        }

        // 同一份程序在 LEB128 下必须正常编码：越界只是定宽格式的限制。
        assert!(encode_with_width(&program, OperandWidth::Leb128).is_ok());
    }

    /// 越界引用与定宽溢出必须在**编码期**被拒，且错误里带上引用类别。
    ///
    /// 逐个改坏一份合法程序里的引用：`ConstId`、`FuncId`、`SigId`、跳转目标
    /// `BlockId`，各自断言 `kind` 字段；最后把一条指令的寄存器号改成 65536 并用
    /// `FixedU16` 编码，断言 `IntegerOverflow` 里报的是原值而不是截断后的值。
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

    /// 标量标签与释放类别标签必须正反双向一致。
    ///
    /// 这条测试同时钉住了与 `xiao-lifetime` 的接口契约：`ReleaseActionKind` 的标签
    /// 就是 `ALL` 数组下标，上游调整 `ALL` 的次序会在这里失败，而不是在某个
    /// 运行期表现为「强释放变成了弱释放」。
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
