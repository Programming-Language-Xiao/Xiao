//! TAC、ABI 元数据和源码映射的编码实现。

use super::codec::{Writer, write_optional_index, write_optional_string, write_optional_usize};
use super::tags::{
    arg_kind_tag, arith_tag, compare_tag, opcode, param_kind_tag, register_class_tag, release_tag,
    scalar_tag, set_compare_tag, set_op_tag,
};
use super::validate::validate_signature;
use super::{
    CallSigTable, CategoryMap, ConstPool, EncodeError, EncodedBlock, EncodedFunction, IrSpan,
    PathStep, TacArgument, TacConstant, TacFunction, TacInstr, TacOp, TacProgram, VReg,
};
use xiao_ir::{IrArrayShape, IrDictTypeEntry, IrType};

/// 按常量池索引顺序写常量：1 字节类别标签加载荷。
///
/// 载荷按类型分开编码：`Int`/`Sint` 写小端定宽整数，`Float`/`Sfloat` 先取
/// `to_bits` 再写位模式（用 `PartialEq` 或十进制往返会丢掉 `-0.0` 的符号和 NaN
/// 的载荷位），`Lint`/`Lfloat` 保持规范十进制文本，字符串写长度前缀加 UTF-8
/// 字节。索引就是写出顺序，所以顺序不能重排。
pub(super) fn encode_constants(writer: &mut Writer, pool: &ConstPool) -> Result<(), EncodeError> {
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
pub(super) fn encode_signatures(
    writer: &mut Writer,
    table: &CallSigTable,
) -> Result<(), EncodeError> {
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
pub(super) fn encode_function(
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
        TacOp::SetOp { op, left, right } => {
            writer.byte(set_op_tag(*op));
            writer.index(left.get(), "VReg")?;
            writer.index(right.get(), "VReg")
        }
        TacOp::SetCompare { op, left, right } => {
            writer.byte(set_compare_tag(*op));
            writer.index(left.get(), "VReg")?;
            writer.index(right.get(), "VReg")
        }
        TacOp::Len { source } => writer.index(source.get(), "VReg"),
        TacOp::LoadTable {
            table,
            construct,
            arguments,
        } => {
            writer.index(*table, "TableId")?;
            writer.byte(u8::from(*construct));
            encode_arguments(writer, arguments)
        }
        TacOp::MemberGet { object, member } => {
            writer.index(object.get(), "VReg")?;
            writer.string(member)
        }
        TacOp::MemberSet {
            object,
            member,
            value,
        } => {
            writer.index(object.get(), "VReg")?;
            writer.string(member)?;
            writer.index(value.get(), "VReg")
        }
        TacOp::IndexGetDynamic { source, index } => {
            writer.index(source.get(), "VReg")?;
            writer.index(index.get(), "VReg")
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
            expected,
        } => {
            writer.string(kind)?;
            writer.index(value.get(), "VReg")?;
            writer.index(on_failure.get(), "BlockId")?;
            writer.byte(u8::from(expected.is_some()));
            if let Some(expected) = expected {
                encode_type(writer, expected)?;
            }
            Ok(())
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
pub(super) fn encode_categories(
    writer: &mut Writer,
    categories: &CategoryMap,
) -> Result<(), EncodeError> {
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
pub(super) fn encode_plans(writer: &mut Writer, program: &TacProgram) -> Result<(), EncodeError> {
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
pub(super) fn encode_selection_plans(
    writer: &mut Writer,
    program: &TacProgram,
) -> Result<(), EncodeError> {
    writer.count(program.selection_plans.len(), "selection_plans")?;
    for plan in &program.selection_plans {
        let json = serde_json::to_string(plan)
            .map_err(|error| EncodeError::InvalidFormat(format!("选择计划序列化失败：{error}")))?;
        writer.string(&json)?;
    }
    Ok(())
}

/// 写事务性广播计划表。
pub(super) fn encode_broadcast_plans(
    writer: &mut Writer,
    program: &TacProgram,
) -> Result<(), EncodeError> {
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
pub(super) fn encode_random_seed_plans(
    writer: &mut Writer,
    program: &TacProgram,
) -> Result<(), EncodeError> {
    writer.count(program.random_seed_plans.len(), "random_seed_plans")?;
    for plan in &program.random_seed_plans {
        let json = serde_json::to_string(plan).map_err(|error| {
            EncodeError::InvalidFormat(format!("随机种子计划序列化失败：{error}"))
        })?;
        writer.string(&json)?;
    }
    Ok(())
}

/// 写入格式 3 新增的表定义段；函数索引遵循所选操作数宽度。
pub(super) fn encode_table_definitions(
    writer: &mut Writer,
    program: &TacProgram,
) -> Result<(), EncodeError> {
    writer.count(program.table_definitions.len(), "table_definitions")?;
    for table in &program.table_definitions {
        let json = serde_json::to_string(&table.signature)
            .map_err(|error| EncodeError::InvalidFormat(format!("表签名序列化失败：{error}")))?;
        writer.string(&json)?;
        writer.index(table.fields.get(), "FuncId")?;
        writer.count(table.methods.len(), "table.methods")?;
        for (name, function) in &table.methods {
            writer.string(name)?;
            writer.index(function.get(), "FuncId")?;
        }
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
