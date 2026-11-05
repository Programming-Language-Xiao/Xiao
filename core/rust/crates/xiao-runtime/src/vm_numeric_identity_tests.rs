//! A1的XCON数值恢复测试；只扩展接纳，不改22类协议或历史断言。

use super::*;

#[test]
/// XCON v1仍传文本，接收Runtime独立恢复显示精度与规范身份。
fn xcon_numeric_identity_round_trips_through_the_existing_frame() {
    assert_eq!(crate::XCON_VERSION, 1);
    let first = VmTransferValue::Xcon(encode_xcon(&XconValue::Lfloat("1.50".to_owned())).unwrap());
    let second =
        VmTransferValue::Xcon(encode_xcon(&XconValue::Lfloat("15e-1".to_owned())).unwrap());
    let first = first.decode(0).unwrap();
    let second = second.decode(0).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.display_text().unwrap(), "1.50");
    assert_eq!(second.display_text().unwrap(), "1.5");
}

#[test]
/// 结构正确的XCON非法数值在共享边界报012/033，不混成结构错误029。
fn xcon_numeric_payloads_are_checked_before_becoming_runtime_values() {
    for (payload, expected) in [
        (XconValue::Lint("bad".to_owned()), crate::INVALID_VALUE_CODE),
        (
            XconValue::Lfloat("1e100001".to_owned()),
            crate::NUMERIC_LIMIT_CODE,
        ),
        (
            XconValue::Lint("9".repeat(32_769)),
            crate::NUMERIC_LIMIT_CODE,
        ),
    ] {
        let transfer = VmTransferValue::Xcon(encode_xcon(&payload).unwrap());
        assert_eq!(transfer.decode(0).unwrap_err().code(), expected);
    }
    let corrupt = VmTransferValue::Xcon(b"invalid-frame".to_vec());
    assert_eq!(corrupt.decode(0).unwrap_err().code(), "X06-RUNTIME-029");
}
