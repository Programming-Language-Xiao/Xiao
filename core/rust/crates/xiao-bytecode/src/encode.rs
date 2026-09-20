//! 09R3 冻结的 TAC 内存编码器。
//!
//! 这里定义的是内存中的冻结编码，不是公开的 `.xiaoc` 文件格式。编码器只展开
//! 已经存在的 TAC/ABI 事实，不重新推断类型、重算生命周期，也不重排指令。格式
//! 使用稳定的显式 opcode 表；操作数可以选择无符号 LEB128 或定宽 `u16`。

/// 字节级读写原语。
mod codec;
/// 严格解码实现。
mod decoder;
/// 编码实现。
mod encoder;
/// 稳定标签映射。
mod tags;
#[cfg(test)]
/// 编码器回归测试。
mod tests;
/// 编码输入与可复用不变量校验。
mod validate;

use codec::Writer;
use decoder::{decode_inner, validate_decoded};
use encoder::{
    encode_broadcast_plans, encode_categories, encode_constants, encode_function, encode_plans,
    encode_random_seed_plans, encode_selection_plans, encode_signatures,
};
use validate::validate_input;

use xiao_ir::IrSpan;

use super::lower::{TAC_BYTECODE_ABI_VERSION, TAC_RUNTIME_ABI_VERSION};
use super::sig::{CallSig, CallSigTable, ParamKind};
use super::tac::{
    ArgKind, ArithOp, BlockId, CategoryMap, CompareOp, ConstId, ConstPool, FuncId, PathStep,
    RegisterClass, SetCompareOp, SetOpKind, SigId, TAC_VERSION, TacAbi, TacArgument, TacBlock,
    TacConstant, TacFunction, TacHandler, TacInstr, TacOp, TacProgram, VReg,
};

/// 内存编码的魔数。
///
/// `X9` 前缀刻意与真实的 `.xiaoc` 容器区分开：这里编出来的是内存编码，
/// 不承诺任何文件级兼容，因此需要一个能立刻判死的头，避免把实验字节流当成
/// 产物格式误读。
const MAGIC: [u8; 4] = *b"X9RD";
/// 字节布局的版本号，与 ABI 版本分工不同。
///
/// ABI 版本描述语义契约（R1 冻结），这个字段描述**字节怎么排**。只要布局
/// 变动就必须递增它，解码端据此直接拒绝旧字节，而不是照着新规则错读旧数据。
pub const FORMAT_VERSION: u8 = 3;

/// 当前冻结的最小 opcode。
pub const OPCODE_MIN: u8 = 0;

/// 当前冻结的最大 opcode。
pub const OPCODE_MAX: u8 = 40;
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
    /// 冻结编码里的寄存器号和索引绝大多数是小编号，变长比定宽短；定宽是给
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

/// 一份内存中的冻结编码结果。
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

/// 内存编码的结构化错误。
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

/// 把 TAC 程序编码为内存中的冻结字节串。
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
    encoder::encode_table_definitions(&mut writer, program)?;
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

/// 解码冻结字节串并恢复 TAC 语义模型。
pub fn decode(bytes: &[u8]) -> Result<TacProgram, EncodeError> {
    let (program, width) = decode_inner(bytes)?;
    validate_decoded(&program, width)?;
    Ok(program)
}

/// 解码冻结字节串并同时重建物理目录。
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

#[cfg(test)]
use codec::{Reader, write_optional_index};
#[cfg(test)]
use decoder::decode_instruction;
#[cfg(test)]
use tags::{SCALAR_TAGS, opcode, release_from_tag, release_tag, scalar_from_tag, scalar_tag};
