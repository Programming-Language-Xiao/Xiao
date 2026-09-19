//! 研究编码字节流的严格解码实现。

use super::codec::{Reader, read_bool};
use super::tags::{
    arg_kind_from_tag, arith_from_tag, compare_from_tag, param_kind_from_tag,
    register_class_from_tag, release_from_tag, scalar_from_tag, set_compare_from_tag,
    set_op_from_tag,
};
use super::validate::{check_version, validate_input};
use super::{
    BlockId, CallSig, CallSigTable, CategoryMap, ConstId, ConstPool, EncodeError, FORMAT_VERSION,
    FuncId, IrSpan, MAGIC, OperandWidth, PathStep, SigId, TAC_BYTECODE_ABI_VERSION,
    TAC_RUNTIME_ABI_VERSION, TAC_VERSION, TacAbi, TacArgument, TacBlock, TacConstant, TacFunction,
    TacHandler, TacInstr, TacOp, TacProgram, VReg,
};
use crate::research::lower::{TacReleaseAction, TacReleasePlan};
use std::collections::BTreeMap;
use xiao_ir::{IR_VERSION, IrArrayShape, IrDictTypeEntry, IrType};

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
pub(super) fn decode_inner(bytes: &[u8]) -> Result<(TacProgram, OperandWidth), EncodeError> {
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
pub(super) fn decode_instruction(
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
        34 => TacOp::SetOp {
            op: set_op_from_tag(reader.byte("SetOpKind")?)?,
            left: VReg::new(index(reader, "VReg")?),
            right: VReg::new(index(reader, "VReg")?),
        },
        35 => TacOp::SetCompare {
            op: set_compare_from_tag(reader.byte("SetCompareOp")?)?,
            left: VReg::new(index(reader, "VReg")?),
            right: VReg::new(index(reader, "VReg")?),
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
fn decode_plans(reader: &mut Reader<'_>) -> Result<Vec<TacReleasePlan>, EncodeError> {
    let count = reader.count("release_plans")?;
    let mut plans = Vec::with_capacity(count);
    for _ in 0..count {
        let scope = reader.u32_uleb("plan.scope")?;
        let exit = reader.string("plan.exit")?;
        let action_count = reader.count("release_actions")?;
        let mut actions = Vec::with_capacity(action_count);
        for _ in 0..action_count {
            actions.push(TacReleaseAction {
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
        plans.push(TacReleasePlan {
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
pub(super) fn validate_decoded(
    program: &TacProgram,
    width: OperandWidth,
) -> Result<(), EncodeError> {
    validate_input(program, width)
}
