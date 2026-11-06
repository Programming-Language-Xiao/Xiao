//! B2-A5的完整语言样例和类型/IR/字节码承载回归。

use xiao_driver::{FrontendCompiler, FrontendRequest};

#[path = "20o_b2_a_support/mod.rs"]
mod support;

/// 与维护精确入口共用完整源码、教程逐字检查和独立黄金。
fn check_fixture(name: &str) {
    support::vm_fixture(name);
}

#[test]
/// 超i128的完整算术与bool奇偶从源码执行。
fn a5_integer_language_uses_exact_runtime_arithmetic() {
    check_fixture("a5_integer_arithmetic");
}

#[test]
/// const与后续安全加宽赋值必须保持声明的真实标签和显示精度。
fn a5_assignments_and_constants_keep_the_declared_numeric_value() {
    check_fixture("a5_bindings");
}

#[test]
/// 十进制尺度、商余、幂和极端scale差从源码执行。
fn a5_decimal_language_preserves_frozen_scales() {
    check_fixture("a5_decimal_arithmetic");
}

#[test]
/// 公开转换、二进制实际值与跨类型比较均走语言链路。
fn a5_conversion_language_preserves_values_and_promotion() {
    check_fixture("a5_conversions");
}

#[test]
/// 六类型36个有序组合、36项显式转换及复合赋值执行独立小整数黄金。
fn a5_complete_numeric_language_matrix_matches_independent_golden() {
    check_fixture("a5_matrix");
}

#[test]
/// 高精度商余准入不开放其余浮点商余，也不放宽bool或字符串混合算术。
fn a5_static_invalid_numeric_combinations_remain_rejected() {
    let mut rejected = 0;
    for left in ["sint", "int", "sfloat", "float"] {
        for right in ["sint", "int", "sfloat", "float"] {
            if ![left, right].iter().any(|ty| ty.contains("float")) {
                continue;
            }
            for operator in ["//", "%"] {
                let source = format!(
                    "{left} a = {left}(7)\n{right} b = {right}(2)\nprint(a {operator} b)\n"
                );
                assert!(
                    FrontendCompiler::new()
                        .compile(&FrontendRequest::from_text(source))
                        .is_err()
                );
                rejected += 1;
            }
        }
    }
    for expression in [
        "true + lfloat(1)",
        "lint(1) + true",
        "lint(1) + \"x\"",
        "lfloat(1) * false",
    ] {
        assert!(
            FrontendCompiler::new()
                .compile(&FrontendRequest::from_text(format!(
                    "print({expression})\n"
                )))
                .is_err()
        );
        rejected += 1;
    }
    assert_eq!(rejected, 28);
}

#[test]
/// 显式参数/返回转换保留实际标签，并发边界保留文本尺度和大整数。
fn a5_language_carriers_preserve_numeric_types_and_values() {
    check_fixture("a5_carriers");
}

#[test]
/// 可捕获值错误、未使用结果和finally从完整源码贯通。
fn a5_language_errors_remain_recoverable_and_observable() {
    check_fixture("a5_errors");
}

#[test]
/// 函数签名仍严格匹配；不借A5新开隐式参数/返回转换。
fn a5_preserves_existing_function_type_boundaries() {
    for source in [
        "def exact(lfloat value) -> lfloat\n    return value\nprint(exact(lint(1)))\n",
        "def exact() -> lfloat\n    return 1\nprint(exact())\n",
    ] {
        assert!(
            FrontendCompiler::new()
                .compile(&FrontendRequest::from_text(source))
                .is_err()
        );
    }
}

#[test]
#[ignore = "需要本任务新Runtime与显式LLVM/平台链接器"]
/// 与VM同一语言语料在O0/O3逐字一致。
fn a5_language_values_match_native_golden_outputs() {
    support::native_suite();
}

#[test]
/// VM精确入口的完整集合包括错误位置、受控分配与真实归档重放。
fn a5_complete_vm_entry_covers_resources_and_portable_replay() {
    support::vm_suite();
}

#[test]
/// 同一编码后的完整矩阵在栈/分类型寄存器/混合载体逐字一致，特别检查复合结果类别。
fn a5_encoded_numeric_matrix_matches_all_existing_carriers() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let path = root.join("tests/fixtures/docs/numeric/a5_matrix.xiao");
    let golden = std::fs::read_to_string(path.with_extension("stdout"))
        .unwrap()
        .replace("\r\n", "\n");
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            std::fs::read_to_string(path).unwrap(),
        ))
        .unwrap();
    let bytes = xiao_bytecode::encode_xiaoc(
        &xiao_bytecode::lower_program(&artifact.ir),
        xiao_bytecode::XiaocMetadata::new("a5-carrier-matrix"),
    )
    .unwrap();
    let program = xiao_bytecode::decode_xiaoc(&bytes).unwrap().program;
    for (name, result) in [
        ("stack", xiao_vm::run(&program, Default::default())),
        (
            "register",
            xiao_vm::run_register(&program, Default::default()),
        ),
        ("hybrid", xiao_vm::run_hybrid(&program, Default::default())),
    ] {
        assert!(result.result.is_success(), "{name}: {result:?}");
        let stdout: String = result
            .events
            .iter()
            .filter_map(|event| match event {
                xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(stdout, golden, "{name}");
    }
}

#[test]
/// 容器结构合法的坏数值文本仍须在恢复入口拒绝，不能靠哈希/比较吞掉错误。
fn a5_portable_numeric_payloads_cannot_bypass_admission() {
    use xiao_bytecode::TacConstant;
    use xiao_runtime::{VmTransferValue, XconValue, encode_xcon};
    let source = "print(18446744073709551616)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap();
    for (payload, expected) in [
        ("bad".to_owned(), "X06-RUNTIME-012"),
        ("9".repeat(32769), "X06-RUNTIME-033"),
    ] {
        let mut program = xiao_bytecode::lower_program(&artifact.ir);
        let mut replaced = 0;
        let mut constants = xiao_bytecode::ConstPool::new();
        for (index, constant) in program.constants.iter().enumerate() {
            let value = if matches!(constant, TacConstant::Lint(_)) {
                replaced += 1;
                TacConstant::Lint(payload.clone())
            } else {
                constant.clone()
            };
            assert_eq!(constants.intern(value).get() as usize, index);
        }
        program.constants = constants;
        assert_eq!(replaced, 1);
        let bytes = xiao_bytecode::encode_xiaoc(
            &program,
            xiao_bytecode::XiaocMetadata::new("a5-invalid-payload"),
        )
        .unwrap();
        let result = xiao_vm::run_xiaoc(&bytes, Default::default()).unwrap();
        let xiao_vm::RunResult::Error(error) = result.result else {
            panic!("{result:?}")
        };
        assert_eq!(error.code(), expected);
        let transfer = VmTransferValue::Xcon(encode_xcon(&XconValue::Lint(payload)).unwrap());
        assert_eq!(transfer.decode(0).unwrap_err().code(), expected);
    }
    let invalid =
        VmTransferValue::Xcon(encode_xcon(&XconValue::Lfloat("1e100001".to_owned())).unwrap());
    assert_eq!(invalid.decode(0).unwrap_err().code(), "X06-RUNTIME-033");
    assert_eq!(
        VmTransferValue::Xcon(b"bad-frame".to_vec())
            .decode(0)
            .unwrap_err()
            .code(),
        "X06-RUNTIME-029"
    );
}
