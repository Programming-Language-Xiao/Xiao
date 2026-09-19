//! 研究编码器的往返、边界和损坏输入回归测试。

use super::*;
use crate::research::lower::{TacReleaseAction, TacReleasePlan};
use std::collections::BTreeMap;
use xiao_ir::{IR_VERSION, IrArrayShape, IrDictTypeEntry, IrType};
use xiao_ir::{IrSelectionItemPlan, IrSelectionPlan};
use xiao_lifetime::ReleaseActionKind;
use xiao_syntax::ScalarType;

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
    let call_signature = signatures.intern(CallSig::plain(vec![IrType::Dynamic], IrType::Dynamic));
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
/// 这条检查必须拿到调用方的真实宽度：`FixedU16` 下应在写出前报出精确字段名，
/// 同一份程序在 `Leb128` 下则仍可编码。
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
