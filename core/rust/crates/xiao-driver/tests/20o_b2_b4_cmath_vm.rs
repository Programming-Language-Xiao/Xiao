//! B4-2B：直接原语的生产 VM、解码 span、错误、清理与 ABI 挂起槽隔离。
//! 数值位模式/畸形实参通过内部 TAC 研究入口注入，不冒充源级非法类型或模块导入证据。

use xiao_bytecode::{CallSig, OperandWidth, TacOp, TacProgram, decode, encode, lower_program};
use xiao_driver::{DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver};
use xiao_intrinsics::IntrinsicId;
use xiao_runtime::{RuntimeResult, RuntimeValue};
use xiao_vm::{RunOutcome, RunRequest, RunResult, VmEvent};

#[derive(Clone, Copy)]
/// 独立的 Runtime 参考调用表；VM 结果必须与对应门面一致。
enum Entry {
    Unary(fn(f64) -> RuntimeResult<f64>),
    Binary(fn(f64, f64) -> RuntimeResult<f64>),
}

impl Entry {
    /// 依据真实元数调用 Runtime 参考入口。
    fn call(self, a: f64, b: f64) -> RuntimeResult<f64> {
        match self {
            Self::Unary(f) => f(a),
            Self::Binary(f) => f(a, b),
        }
    }
}

/// 以冻结顺序列出 15 个直接原语及 Runtime 门面。
fn entries() -> [(&'static str, Entry); 15] {
    use Entry::{Binary as B, Unary as U};
    use xiao_runtime::*;
    [
        ("sqrt", U(cmath_sqrt)),
        ("floor", U(cmath_floor)),
        ("ceil", U(cmath_ceil)),
        ("round", U(cmath_round)),
        ("trunc", U(cmath_trunc)),
        ("abs", U(cmath_abs)),
        ("min", B(cmath_min)),
        ("max", B(cmath_max)),
        ("pow", B(cmath_pow)),
        ("sin", U(cmath_sin)),
        ("cos", U(cmath_cos)),
        ("tan", U(cmath_tan)),
        ("exp", U(cmath_exp)),
        ("ln", U(cmath_ln)),
        ("log10", U(cmath_log10)),
    ]
}

/// 通过真实前端检查并保留完整 IR。
fn compile(source: &str) -> xiao_driver::FrontendArtifact {
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
}

/// 只拼接语言 print 输出，不混入调试或释放事件。
fn output(events: &[VmEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// 内部位模式注入：将已编译函数选作研究入口，形参与释放计划均保留。
fn injection_program(index: usize, named: bool) -> TacProgram {
    let (name, entry) = entries()[index];
    let parameters =
        xiao_intrinsics::cmath_parameters(IntrinsicId::new(34 + index as u32).unwrap()).unwrap();
    let (signature, arguments) = match entry {
        Entry::Unary(_) => (
            "float a".to_owned(),
            if named {
                "value = a".into()
            } else {
                "a".into()
            },
        ),
        Entry::Binary(_) => (
            "float a, float b".to_owned(),
            if named {
                format!("{} = b, {} = a", parameters[1].0, parameters[0].0)
            } else {
                "a, b".into()
            },
        ),
    };
    let source = format!("def probe({signature}) -> float\n    return cmath_{name}({arguments})\n");
    let mut program = lower_program(&compile(&source).ir);
    let selected = program
        .functions
        .iter()
        .position(|function| function.name == "probe")
        .unwrap();
    program.functions.swap(0, selected);
    program.categories = program.functions[0].categories.clone();
    program
}

/// 在既有研究入口注入动态参数，逐一检查三种载体。
fn injected(program: &TacProgram, values: &[RuntimeValue], carrier: usize) -> RunOutcome {
    match carrier {
        0 => xiao_vm::run_with_values::<xiao_vm::StackCarrier>(program, Default::default(), values),
        1 => xiao_vm::run_with_values::<xiao_vm::RegisterCarrier>(
            program,
            Default::default(),
            values,
        ),
        _ => {
            xiao_vm::run_with_values::<xiao_vm::HybridCarrier>(program, Default::default(), values)
        }
    }
}

#[test]
/// 15 项直接原语经真实前端、生产 VM 和 .xiaoc 执行，无模块导入。
fn all_direct_primitives_execute_in_source_and_production_xiaoc() {
    let source = "print(cmath_sqrt(value = 4.0))\nprint(cmath_floor(1.5))\nprint(cmath_ceil(1.5))\nprint(cmath_round(2.5))\nprint(cmath_trunc(-1.5))\nprint(cmath_abs(-3.0))\nprint(cmath_min(right = 7.0, left = 2.0))\nprint(cmath_max(right = 7.0, left = 2.0))\nprint(cmath_pow(exponent = 3.0, base = 2.0))\nprint(cmath_sin(0.0))\nprint(cmath_cos(0.0))\nprint(cmath_tan(0.0))\nprint(cmath_exp(0.0))\nprint(cmath_ln(1.0))\nprint(cmath_log10(1.0))\n";
    let golden = "2\n1\n2\n2\n-1\n3\n2\n7\n8\n0\n1\n0\n1\n0\n0\n";
    let request = FrontendRequest::from_text(source);
    let artifact = compile(source);
    for level in [
        xiao_optimizer::OptimizationLevel::O0,
        xiao_optimizer::OptimizationLevel::O3,
    ] {
        let outcome = FrontendVmDriver::new().run_artifact(
            &artifact,
            &DriverRequest::new(request.clone()).with_optimization_level(level),
        );
        assert!(outcome.is_success(), "{outcome:?}");
        assert_eq!(output(outcome.as_executed().unwrap().events()), golden);
    }
    let bytes =
        xiao_bytecode::encode_xiaoc(&lower_program(&artifact.ir), Default::default()).unwrap();
    let result = xiao_vm::run_xiaoc_production(&bytes, Default::default(), 4096, None).unwrap();
    assert!(result.result.is_success(), "{result:?}");
    assert_eq!(output(&result.events), golden);
    assert_eq!(xiao_runtime_abi::ABI_MINOR_VERSION, 20);
    assert_eq!(xiao_codegen_llvm::CODEGEN_VERSION, 26);
    println!("B4-2B-VM primitives=15 source=O0,O3 xiaoc=production result=passed");
}

#[test]
/// 逐 ID 对照 Runtime，覆盖±0、子正规数、溢出、定义域和每个参数的NaN/Inf；三载体一致。
fn decoded_vm_calls_match_runtime_bits_and_errors_for_all_ids() {
    let cases = [
        (4.0, 2.0),
        (1.5, 2.5),
        (0.0, 0.0),
        (-0.0, 0.0),
        (-0.0, -0.0),
        (-0.0, 3.0),
        (-1.0, 0.5),
        (f64::from_bits(1), 2.0),
        (f64::MAX, 2.0),
        (-745.0, 3.0),
        (f64::NAN, 1.0),
        (f64::INFINITY, 1.0),
        (f64::NEG_INFINITY, 1.0),
        (1.0, f64::NAN),
        (1.0, f64::INFINITY),
        (1.0, f64::NEG_INFINITY),
    ];
    let mut checked = 0;
    for (index, (name, entry)) in entries().into_iter().enumerate() {
        for named in [false, true] {
            let encoded = encode(&injection_program(index, named), OperandWidth::Leb128).unwrap();
            let program = decode(&encoded.bytes).unwrap();
            for (a, b) in cases {
                let mut values = vec![RuntimeValue::Float(a)];
                if matches!(entry, Entry::Binary(_)) {
                    values.push(RuntimeValue::Float(b));
                }
                let expected = entry
                    .call(a, b)
                    .map(f64::to_bits)
                    .map_err(|e| e.code().to_owned());
                for carrier in 0..3 {
                    let result = injected(&program, &values, carrier);
                    match &expected {
                        Ok(bits) => {
                            assert!(result.result.is_success(), "{name}: {result:?}");
                            let Some(RuntimeValue::Float(number)) = result.value else {
                                panic!("{name}: {result:?}");
                            };
                            assert_eq!(
                                number.to_bits(),
                                *bits,
                                "{name} named={named} carrier={carrier}"
                            );
                        }
                        Err(code) => {
                            assert_eq!(
                                result.result.error_code(),
                                Some(code.as_str()),
                                "{name}: {result:?}"
                            );
                            assert!(result.value.is_none(), "失败不得生成部分值");
                        }
                    }
                    checked += 1;
                }
            }
        }
    }
    println!(
        "B4-2B-VM-IDENTITY cases={checked} ids=34..48 carriers=3 binding=positional,reversed-named result=passed"
    );
}

#[test]
/// 畸形内部 TAC 的少参/多参/重名/未知形参/命名后位置及所有非float值走002；未知ID编码前拒绝。
fn vm_rejects_invalid_shapes_types_and_unknown_ids() {
    for (index, (name, entry)) in entries().into_iter().enumerate() {
        let base = injection_program(index, false);
        let parameters =
            xiao_intrinsics::cmath_parameters(IntrinsicId::new(34 + index as u32).unwrap())
                .unwrap();
        for value in [
            RuntimeValue::Int(1),
            RuntimeValue::Sint(1),
            RuntimeValue::Sfloat(1.0),
            RuntimeValue::Lint("1".into()),
            RuntimeValue::Lfloat("1".into()),
            RuntimeValue::Bool(true),
            RuntimeValue::None,
            RuntimeValue::new_string("1").unwrap(),
        ] {
            for slot in 0..if matches!(entry, Entry::Binary(_)) {
                2
            } else {
                1
            } {
                // 完整类型检查必须先于其余实参的 NaN 域错误。
                let mut values =
                    vec![RuntimeValue::Float(f64::NAN); base.functions[0].parameters.len()];
                values[slot] = value.clone();
                let result = injected(&base, &values, 0);
                assert_eq!(
                    result.result.error_code(),
                    Some("X06-RUNTIME-002"),
                    "{name}: {result:?}"
                );
            }
        }
        for mutation in 0..6 {
            let mut program = base.clone();
            let unknown =
                (mutation == 5).then(|| program.signatures.intern(CallSig::intrinsic(60)));
            let mut changed = false;
            for instruction in program.functions[0]
                .blocks
                .iter_mut()
                .flat_map(|b| &mut b.instructions)
            {
                if let TacOp::Call {
                    signature,
                    arguments,
                    ..
                } = &mut instruction.op
                    && program
                        .signatures
                        .get(*signature)
                        .unwrap()
                        .intrinsic_id()
                        .is_some_and(|id| id == 34 + index as u32)
                {
                    match mutation {
                        0 => {
                            arguments.pop();
                        }
                        1 => arguments.push(arguments[0].clone()),
                        2 => arguments[0].name = Some("unknown".into()),
                        3 if arguments.len() == 2 => {
                            arguments[0].name = Some(parameters[0].0.into());
                        }
                        4 if arguments.len() == 2 => {
                            for arg in arguments.iter_mut() {
                                arg.name = Some(parameters[0].0.into());
                            }
                        }
                        5 => *signature = unknown.unwrap(),
                        _ => continue,
                    }
                    changed = true;
                }
            }
            if !changed {
                continue;
            }
            if mutation == 5 {
                assert!(encode(&program, OperandWidth::Leb128).is_err());
            } else {
                let values = vec![RuntimeValue::Float(-1.0); base.functions[0].parameters.len()];
                let result = injected(&program, &values, 0);
                assert_eq!(
                    result.result.error_code(),
                    Some("X06-RUNTIME-002"),
                    "{name} mutation={mutation}: {result:?}"
                );
            }
        }
    }
    println!(
        "B4-2B-VM-NEGATIVE ids=15 dynamic_types=8 shape=checked reserved=60-rejected result=passed"
    );
}

#[test]
/// 编码再解码后的009保留原始调用span；catch/finally和未使用Pure调用覆盖O0～O3。
fn domain_errors_keep_decoded_span_and_observable_cleanup() {
    let source = "cmath_sqrt(-1.0)\n";
    let artifact = compile(source);
    for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
        let encoded = encode(&lower_program(&artifact.ir), width).unwrap();
        let program = decode(&encoded.bytes).unwrap();
        let result = xiao_vm::run_request(&RunRequest::new(&artifact.ir, &program));
        let RunResult::Error(error) = result.result else {
            panic!("{result:?}");
        };
        assert_eq!(error.code(), "X06-RUNTIME-009");
        assert_eq!(
            error.location(),
            Some(xiao_source::SourceSpan::new(0, 16).unwrap())
        );
        assert!(result.value.is_none());
    }
    let source = "def probe(bool fail)\n    held = [\"owned\"]\n    try\n        temporary = \"2.0\"\n        if fail\n            cmath_sqrt(-float(lfloat(temporary)))\n        else\n            cmath_sqrt(float(lfloat(temporary)))\n    catch err as ArithmeticError\n        print(error_code(err))\n    finally\n        print(held[0])\nprobe(true)\nprobe(false)\n";
    let artifact = compile(source);
    // 解码后的同一失败调用必须真实进入处理器，并执行 finally 与释放计划。
    for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
        let program = lower_program(&artifact.ir);
        let encoded = encode(&program, width).unwrap();
        let decoded = decode(&encoded.bytes).unwrap();
        let result = xiao_vm::run_request(
            &RunRequest::new(&artifact.ir, &decoded).with_event_capacity(4096),
        );
        assert!(result.result.is_success(), "{width:?}: {result:?}");
        assert_eq!(output(&result.events), "owned\nX06-RUNTIME-009\nowned\n");
        assert_eq!(
            result
                .events
                .iter()
                .filter(|event| matches!(event, VmEvent::HandlerMatched {
            catch_type: Some(kind), .. } if kind == "ArithmeticError"))
                .count(),
            1
        );
        assert!(
            result
                .events
                .iter()
                .any(|event| matches!(event, VmEvent::ValueReleased { .. }))
        );
        println!(
            "B4-2B-VM-DECODED-CATCH width={width:?} error=009 handler=ArithmeticError finally=2 releases=present result=passed"
        );
    }
    let mut baseline = None;
    for level in [
        xiao_optimizer::OptimizationLevel::O0,
        xiao_optimizer::OptimizationLevel::O1,
        xiao_optimizer::OptimizationLevel::O2,
        xiao_optimizer::OptimizationLevel::O3,
    ] {
        let outcome = FrontendVmDriver::new().run_artifact(
            &artifact,
            &DriverRequest::new(FrontendRequest::from_text(source))
                .with_optimization_level(level)
                .with_event_capacity(4096),
        );
        assert!(outcome.is_success(), "{level:?}: {outcome:?}");
        let events = outcome.as_executed().unwrap().events();
        // 保留既有 VM 先执行 finally 再进入 catch 的展开规则。
        assert_eq!(output(events), "owned\nX06-RUNTIME-009\nowned\n");
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, VmEvent::HandlerMatched { .. }))
                .count(),
            1
        );
        let releases: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                VmEvent::ValueReleased {
                    scope,
                    exit,
                    value,
                    kind,
                } => Some((*scope, exit.clone(), *value, kind.clone())),
                _ => None,
            })
            .collect();
        assert!(!releases.is_empty());
        if let Some(expected) = &baseline {
            assert_eq!(&releases, expected, "{level:?}");
        } else {
            baseline = Some(releases);
        }
    }
    println!(
        "B4-2B-VM-ERROR error=009 decoded_span=exact catch=1 finally=2 optimization=O0..O3 release_sequence=exact result=passed"
    );
}

#[test]
/// 命名参数按规范形参绑定，但实参表达式仍依源码顺序先抛009或013。
fn named_argument_expression_failures_keep_source_evaluation_order() {
    for (arguments, code) in [
        (
            "right = cmath_sqrt(-1.0), left = 1.0 / 0.0",
            "X06-RUNTIME-009",
        ),
        (
            "right = 1.0 / 0.0, left = cmath_sqrt(-1.0)",
            "X06-RUNTIME-013",
        ),
    ] {
        // 除零藏在已检查函数参数里，避免定宽常量表达式的编译期拒绝。
        let args = arguments.replace("1.0 / 0.0", "float(divide(1, 0))");
        let source = format!(
            "def divide(int a, int b) -> int\n    return a // b\ntry\n    cmath_min({args})\ncatch err as ArithmeticError\n    print(error_code(err))\n"
        );
        let request = FrontendRequest::from_text(source);
        let outcome = FrontendVmDriver::new().run(&DriverRequest::new(request));
        assert!(outcome.is_success(), "{outcome:?}");
        assert_eq!(
            output(outcome.as_executed().unwrap().events()),
            format!("{code}\n")
        );
    }
}

#[test]
/// VM成功、类型错与域错均不得消费或替换Runtime ABI已挂起的错误。
fn vm_calls_leave_runtime_abi_pending_error_unchanged() {
    use xiao_runtime_abi::*;
    let program = injection_program(0, false);
    unsafe {
        xiao_runtime_error_clear();
    }
    for number in [
        RuntimeValue::Float(4.0),
        RuntimeValue::Float(-1.0),
        RuntimeValue::Int(1),
    ] {
        let result = injected(&program, std::slice::from_ref(&number), 0);
        assert!(matches!(
            result.result,
            RunResult::Success | RunResult::Error(_)
        ));
        assert_eq!(unsafe { xiao_runtime_error_class() }, 0);
        let bytes = |text: &'static str| XiaoAbiBytes {
            ptr: text.as_ptr(),
            len: text.len(),
        };
        assert_eq!(
            unsafe {
                xiao_runtime_error_raise_type(
                    bytes("TypeError"),
                    bytes("B4-2B-SENTINEL"),
                    bytes("keep-pending"),
                    XiaoAbiErrorLocation::none(),
                )
            },
            0
        );
        let snapshot = || {
            let mut out = std::mem::MaybeUninit::<XiaoAbiErrorSnapshot>::uninit();
            assert_eq!(unsafe { xiao_runtime_error_snapshot(out.as_mut_ptr()) }, 0);
            unsafe { out.assume_init() }
        };
        let before = snapshot();
        let _ = injected(&program, &[number], 0);
        assert_eq!(snapshot(), before);
        unsafe {
            xiao_runtime_error_clear();
        }
    }
}

#[test]
/// 失败调用释放借读临时强引用，输入堆值保持只读；丢弃原值后弱句柄不能复活。
fn failing_cmath_calls_do_not_leak_heap_arguments_or_partial_results() {
    for index in 0..15 {
        let program = injection_program(index, false);
        for carrier in 0..3 {
            let value = RuntimeValue::new_string("cmath-type-error-input").unwrap();
            let RuntimeValue::Str(handle) = &value else {
                unreachable!();
            };
            let weak = handle.downgrade();
            let before = handle.strong_count();
            let mut values = vec![RuntimeValue::Float(1.0); program.functions[0].parameters.len()];
            values[0] = value.clone();
            let outcome = injected(&program, &values, carrier);
            assert_eq!(outcome.result.error_code(), Some("X06-RUNTIME-002"));
            assert!(outcome.value.is_none());
            drop(values);
            assert_eq!(handle.strong_count(), before);
            assert_eq!(value.display_text().unwrap(), "cmath-type-error-input");
            drop(value);
            assert!(
                weak.upgrade().is_err(),
                "id={} carrier={carrier}",
                34 + index
            );
        }
    }
    println!(
        "B4-2B-VM-OWNERSHIP cases=45 errors=002 input=readonly temporary_refs=restored partial_results=none result=passed"
    );
}
