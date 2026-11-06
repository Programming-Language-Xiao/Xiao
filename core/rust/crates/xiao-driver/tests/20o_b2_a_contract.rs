//! B2-A5的完整语言样例和类型/IR/字节码承载回归。

use std::fs;
use std::path::Path;
use std::time::Duration;
use xiao_driver::{DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver};
use xiao_vm::VmEvent;

#[path = "20o_b2_a_support/mod.rs"]
mod support;

/// 通过正式前端、TAC/字节码与VM执行完整源码，保持独立stdout黄金。
fn check_fixture(name: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let path = root
        .join("tests/fixtures/docs/numeric")
        .join(format!("{name}.xiao"));
    let source = fs::read_to_string(&path).unwrap();
    let expected = fs::read_to_string(path.with_extension("stdout")).unwrap();
    let request = FrontendRequest::from_text_at(source, path);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .unwrap_or_else(|error| panic!("{name}: {error:?}"));
    let outcome = FrontendVmDriver::new().run_artifact(
        &artifact,
        &DriverRequest::new(request).with_timeout(Duration::from_secs(10)),
    );
    assert!(outcome.is_success(), "{name}: {outcome:?}");
    let stdout: String = outcome
        .as_executed()
        .unwrap()
        .events()
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(stdout, expected, "{name}");
    println!("B2-A5-LANGUAGE case={name} backend=vm result=success");
}

#[test]
/// 超i128的完整算术与bool奇偶从源码执行。
fn a5_integer_language_uses_exact_runtime_arithmetic() {
    check_fixture("a5_integer_arithmetic");
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
    support::native_fixtures(&[
        "a5_integer_arithmetic",
        "a5_decimal_arithmetic",
        "a5_conversions",
        "a5_carriers",
        "a5_errors",
    ]);
}
