//! 静态数值失败使用的 Runtime ABI 入口。

use xiao_runtime_abi::{XiaoAbiBytes, XiaoAbiStatus};
use xiao_source::SourceSpan;

use crate::errors::RuntimeError;

/// 构造数值类可恢复错误并附加固定宽度源码区间。
///
/// 该入口避免在 LLVM 静态路径上传递按值的复合位置结构；`kind=0` 为数值溢出/非有限，
/// `kind=1` 为除数为零。所有文本只在调用期间借用，错误对象不保存外部指针。
#[unsafe(no_mangle)]
pub(crate) extern "C" fn xiao_runtime_error_raise_numeric(
    kind: u32,
    operator: *const u8,
    operator_len: usize,
    message: *const u8,
    message_len: usize,
    start: u64,
    end: u64,
) -> i32 {
    let operator = match unsafe {
        super::utf8(XiaoAbiBytes {
            ptr: operator,
            len: operator_len,
        })
    } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let message = match unsafe {
        super::utf8(XiaoAbiBytes {
            ptr: message,
            len: message_len,
        })
    } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return XiaoAbiStatus::InvalidArgument.code();
    };
    let Some(span) = SourceSpan::new(start, end) else {
        return XiaoAbiStatus::InvalidArgument.code();
    };
    let error = match kind {
        0 => RuntimeError::numeric_overflow(message),
        1 => RuntimeError::division_by_zero(operator),
        _ => return XiaoAbiStatus::InvalidArgument.code(),
    }
    .with_location(span);
    super::set_pending_runtime_error(error);
    XiaoAbiStatus::Ok.code()
}
