//! B4-2A：cmath intrinsic 在 TAC 与既有字节码中的往返契约。

use xiao_bytecode::{
    BlockId, CallSig, EncodeError, FORMAT_VERSION, FuncId, OperandWidth, RegisterClass,
    TAC_BYTECODE_ABI_VERSION, TAC_STRUCTURE_CODE, TAC_VERSION, TacArgument, TacOp, TacProgram,
    build_pc_map, decode, decode_encoded, encode, lower_program, verify_production,
};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_ir::{IrProgram, IrSpan};

const CMATH: &[(&str, &[&str])] = &[
    ("sqrt", &["value"]),
    ("floor", &["value"]),
    ("ceil", &["value"]),
    ("round", &["value"]),
    ("trunc", &["value"]),
    ("abs", &["value"]),
    ("min", &["left", "right"]),
    ("max", &["left", "right"]),
    ("pow", &["base", "exponent"]),
    ("sin", &["value"]),
    ("cos", &["value"]),
    ("tan", &["value"]),
    ("exp", &["value"]),
    ("ln", &["value"]),
    ("log10", &["value"]),
];

#[derive(Clone, Copy, Debug)]
enum Form {
    Positional,
    Named,
    ReverseNamed,
}

impl Form {
    fn label(self) -> &'static str {
        match self {
            Self::Positional => "positional",
            Self::Named => "named",
            Self::ReverseNamed => "reverse_named",
        }
    }
}

#[derive(Debug)]
struct ExpectedCall {
    function: String,
    name: &'static str,
    id: u32,
    form: Form,
    expression: String,
    argument_names: Vec<Option<&'static str>>,
    parameter_indexes: Vec<usize>,
    span: IrSpan,
}

#[derive(Clone, Debug)]
struct CallSite {
    block_index: usize,
    block_id: BlockId,
    instruction_index: usize,
    id: u32,
    callee: FuncId,
    arguments: Vec<TacArgument>,
    span: IrSpan,
}

fn add_case(
    source: &mut String,
    cases: &mut Vec<ExpectedCall>,
    id: u32,
    name: &'static str,
    parameters: &[&'static str],
    form: Form,
) {
    let function = format!("b4_{name}_{}", form.label());
    let parameter_declarations = parameters
        .iter()
        .map(|parameter| format!("float {parameter}"))
        .collect::<Vec<_>>()
        .join(", ");
    let (expression, argument_names, parameter_indexes) = match (parameters, form) {
        ([value], Form::Positional) => (format!("cmath_{name}({value})"), vec![None], vec![0]),
        ([value], Form::Named) => (
            format!("cmath_{name}({value} = {value})"),
            vec![Some(*value)],
            vec![0],
        ),
        ([left, right], Form::Positional) => (
            format!("cmath_{name}({left}, {right})"),
            vec![None, None],
            vec![0, 1],
        ),
        ([left, right], Form::Named) => (
            format!("cmath_{name}({left} = {left}, {right} = {right})"),
            vec![Some(*left), Some(*right)],
            vec![0, 1],
        ),
        ([left, right], Form::ReverseNamed) => (
            format!("cmath_{name}({right} = {right}, {left} = {left})"),
            vec![Some(*right), Some(*left)],
            vec![1, 0],
        ),
        _ => unreachable!("冻结 cmath 只含一元与二元函数"),
    };

    source.push_str(&format!(
        "def {function}({parameter_declarations}) -> float\n    return "
    ));
    let start = source.len();
    source.push_str(&expression);
    let end = source.len();
    source.push('\n');
    cases.push(ExpectedCall {
        function,
        name,
        id,
        form,
        expression,
        argument_names,
        parameter_indexes,
        span: IrSpan::new(start, end),
    });
}

fn fixture() -> (String, IrProgram, TacProgram, Vec<ExpectedCall>) {
    let mut source = String::new();
    let mut cases = Vec::new();
    for (index, (name, parameters)) in CMATH.iter().enumerate() {
        let id = 34 + index as u32;
        add_case(
            &mut source,
            &mut cases,
            id,
            name,
            parameters,
            Form::Positional,
        );
        add_case(&mut source, &mut cases, id, name, parameters, Form::Named);
        if parameters.len() == 2 {
            add_case(
                &mut source,
                &mut cases,
                id,
                name,
                parameters,
                Form::ReverseNamed,
            );
        }
    }

    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source.clone()))
        .expect("B4-1 已接通的 15 项 cmath 原语应通过类型/IR 阶段");
    let tac = lower_program(&artifact.ir);
    (source, artifact.ir, tac, cases)
}

fn call_site(tac: &TacProgram, function_index: usize) -> CallSite {
    let function = &tac.functions[function_index];
    let sites = function
        .blocks
        .iter()
        .enumerate()
        .flat_map(|(block_index, block)| {
            block.instructions.iter().enumerate().filter_map(
                move |(instruction_index, instruction)| {
                    let TacOp::Call {
                        callee,
                        signature,
                        arguments,
                    } = &instruction.op
                    else {
                        return None;
                    };
                    let id = tac
                        .signatures
                        .get(*signature)
                        .and_then(CallSig::intrinsic_id)?;
                    Some(CallSite {
                        block_index,
                        block_id: block.id,
                        instruction_index,
                        id,
                        callee: *callee,
                        arguments: arguments.clone(),
                        span: instruction.span,
                    })
                },
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        sites.len(),
        1,
        "{} 应恰有一个 intrinsic 调用",
        function.name
    );
    sites.into_iter().next().unwrap()
}

fn assert_case(source: &str, tac: &TacProgram, case: &ExpectedCall) -> (usize, CallSite) {
    let (function_index, function) = tac
        .functions
        .iter()
        .enumerate()
        .find(|(_, function)| function.name == case.function)
        .unwrap_or_else(|| panic!("缺少 TAC 函数 {}", case.function));
    let site = call_site(tac, function_index);
    let instruction = &function.blocks[site.block_index].instructions[site.instruction_index];
    let result = instruction.dst.expect("float 调用应有结果寄存器");
    assert_eq!(function.categories.get(result), RegisterClass::Float);
    assert_eq!(site.id, case.id, "{}", case.function);
    assert_eq!(site.callee.get(), 0, "intrinsic 使用保留入口函数编号 0");
    assert_eq!(site.span, case.span, "{}", case.function);
    assert_eq!(
        source.get(site.span.start..site.span.end),
        Some(case.expression.as_str()),
        "{} 的 TAC span 应指向原始调用表达式",
        case.function
    );
    assert_eq!(site.arguments.len(), case.parameter_indexes.len());
    for (index, argument) in site.arguments.iter().enumerate() {
        let expected = match case.argument_names[index] {
            Some(name) => TacArgument::keyword(
                name.to_owned(),
                function.parameters[case.parameter_indexes[index]],
            ),
            None => TacArgument::positional(function.parameters[case.parameter_indexes[index]]),
        };
        assert_eq!(
            argument, &expected,
            "{} 的实参类别、名称与寄存器顺序",
            case.function
        );
    }
    assert!(
        site.span.end > site.span.start,
        "{} 的源码 span 不能是空区间",
        case.function
    );
    (function_index, site)
}

#[test]
/// 34～48 逐项以两种操作数宽度往返，保留 ID、实参顺序和独立 PC→源码 span 映射。
fn cmath_intrinsics_round_trip_tac_and_bytecode_without_version_changes() {
    let (source, ir, tac, cases) = fixture();
    assert_eq!(cases.len(), 33);
    assert!(
        tac.unsupported.is_empty(),
        "TAC lowering 不应产生 unsupported"
    );
    verify_production(&ir, &tac).expect("完整 cmath TAC 应通过生产校验");

    for case in &cases {
        assert_case(&source, &tac, case);
    }

    assert_eq!(TAC_VERSION, 1);
    assert_eq!(TAC_BYTECODE_ABI_VERSION, 1);
    assert_eq!(FORMAT_VERSION, 4);
    assert_eq!(
        tac.abi.runtime_abi_version,
        xiao_bytecode::TAC_RUNTIME_ABI_VERSION
    );
    assert_eq!(
        (
            xiao_bytecode::XIAOC_FORMAT_MAJOR,
            xiao_bytecode::XIAOC_FORMAT_MINOR
        ),
        (1, 0)
    );
    assert_eq!(xiao_bytecode::XIAOC_DIRECTORY_VERSION, 1);
    for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
        let encoded = encode(&tac, width).expect("15 项 cmath TAC 应可编码");
        assert_eq!(encoded.bytes.get(4), Some(&3), "无并发计划时沿用格式 3");
        let decoded = decode(&encoded.bytes).expect("15 项 cmath TAC 应可解码");
        assert_eq!(
            decoded, tac,
            "完整 TAC（含实参类别、结果与 span）必须无损往返"
        );
        verify_production(&ir, &decoded).expect("解码产物也必须通过生产校验");
        let rebuilt = decode_encoded(&encoded.bytes).expect("应可重建 PC/span 目录");
        assert_eq!(rebuilt, encoded, "字节与 PC/span 目录必须确定性重建");

        let pc_map = build_pc_map(&decoded, width).expect("应从解码产物建立 PC→span 映射");
        for case in &cases {
            let (function_index, original_site) = assert_case(&source, &tac, case);
            assert_eq!(
                encoded.span_at(
                    function_index,
                    original_site.block_index,
                    original_site.instruction_index
                ),
                Some(case.span),
                "{} 编码目录 span",
                case.function
            );
            let function = &tac.functions[function_index];
            let instruction = &function.blocks[original_site.block_index].instructions
                [original_site.instruction_index];
            assert_eq!(instruction.span, case.span);
            let pc = pc_map
                .pc_at(
                    FuncId::new(function_index as u32),
                    original_site.block_id,
                    original_site.instruction_index,
                )
                .expect("TAC 指令应有物理 PC");
            assert_eq!(
                pc_map.span_at_pc(FuncId::new(function_index as u32), pc),
                Some(case.span),
                "{} 的 PC→span 映射",
                case.function
            );
            assert_case(&source, &decoded, case);
            println!(
                "B4-2A-CASE id={} name={} form={} span={}..{} width={width:?} pc={pc} args={:?}",
                case.id,
                case.name,
                case.form.label(),
                case.span.start,
                case.span.end,
                case.argument_names
            );
        }
    }
    println!(
        "B4-2A-SUMMARY ids=34..48 calls={} widths=leb128,fixed_u16 tac_version={TAC_VERSION} bytecode_abi={TAC_BYTECODE_ABI_VERSION} format={FORMAT_VERSION} result=passed",
        cases.len()
    );
}

#[test]
/// 墓碑与未知 ID 都必须被生产校验和编码入口拒绝。
fn cmath_unknown_intrinsic_ids_are_rejected_before_encoding() {
    let (_, ir, tac, cases) = fixture();
    let case = cases.first().expect("应存在 cmath 调用");
    let (function_index, _) = tac
        .functions
        .iter()
        .enumerate()
        .find(|(_, function)| function.name == case.function)
        .unwrap();

    for id in [64, 100, u32::MAX] {
        let mut invalid = tac.clone();
        let invalid_signature = invalid.signatures.intern(CallSig::intrinsic(id));
        let function = &mut invalid.functions[function_index];
        let instruction = function
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.op, TacOp::Call { .. }))
            .expect("应找到可替换的 intrinsic 调用");
        let TacOp::Call { signature, .. } = &mut instruction.op else {
            unreachable!();
        };
        *signature = invalid_signature;

        let validation =
            verify_production(&ir, &invalid).expect_err("生产校验必须拒绝未知/墓碑 intrinsic ID");
        let reason = format!("未知或已废弃的 intrinsic ID {id}");
        assert_eq!(validation.code, TAC_STRUCTURE_CODE);
        assert!(
            validation.message.ends_with(&reason),
            "校验错误应指出 ID {id}：{validation}"
        );
        for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
            assert_eq!(
                encode(&invalid, width),
                Err(EncodeError::InvalidFormat(reason.clone())),
                "{width:?} 编码错误必须指出 ID {id}"
            );
        }
        println!("B4-2A-NEGATIVE id={id} verifier=reject encoder=reject result=passed");
    }
}
