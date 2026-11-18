//! A1常量池恢复与共享数值身份，不依赖新增源级转换或算术。

use super::*;

#[test]
/// 常量池保留原始文本，实际加载时统一接纳并保留显示精度。
fn numeric_constants_restore_checked_runtime_values() {
    let first = constant_value(&TacConstant::Lfloat("1.50".to_owned())).unwrap();
    let second = constant_value(&TacConstant::Lfloat("15e-1".to_owned())).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.display_text().unwrap(), "1.50");
    assert_eq!(second.display_text().unwrap(), "1.5");
    assert_eq!(
        constant_value(&TacConstant::Lint("+00042".to_owned())).unwrap(),
        RuntimeValue::new_lint("42").unwrap()
    );
}

#[test]
/// 语法和预算错误在加载时保持可恢复错误，不能折叠成编译错误。
fn numeric_constants_reject_invalid_payloads_with_frozen_codes() {
    for (value, expected) in [
        (TacConstant::Lfloat("bad".to_owned()), "X06-RUNTIME-012"),
        (
            TacConstant::Lfloat("0e100001".to_owned()),
            "X06-RUNTIME-033",
        ),
        (TacConstant::Lint("9".repeat(32_769)), "X06-RUNTIME-033"),
    ] {
        match constant_value(&value) {
            Err(Fault::Error(error)) => assert_eq!(error.code(), expected),
            other => panic!("必须走可恢复错误：{other:?}"),
        }
    }
}
