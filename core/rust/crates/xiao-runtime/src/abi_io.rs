//! `print`/`input` intrinsic 的 Runtime ABI 实现。

use std::io::Write;
use std::slice;

use xiao_runtime_abi::{XiaoAbiStatus, XiaoValue};

use super::{
    RuntimeError, RuntimeValue, runtime_to_value, set_pending_runtime_error, status_from_error,
    value_to_runtime,
};

/// 格式化一组 ABI 值并写入标准输出；调用方继续拥有并释放传入值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_print_values(values: *const XiaoValue, count: usize) -> i32 {
    if count > 1_000_000 || (count != 0 && values.is_null()) {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    let values = if count == 0 {
        &[][..]
    } else {
        unsafe { slice::from_raw_parts(values, count) }
    };
    let mut rendered = Vec::with_capacity(count);
    for value in values {
        let value = match unsafe { value_to_runtime(value) } {
            Ok(value) => value,
            Err(error) => return error,
        };
        match value.display_text() {
            Ok(text) => rendered.push(text),
            Err(error) => return status_from_error(&error),
        }
    }
    let mut stdout = std::io::stdout().lock();
    if writeln!(stdout, "{}", rendered.join(" ")).is_ok() {
        XiaoAbiStatus::Ok.code()
    } else {
        XiaoAbiStatus::RuntimeError.code()
    }
}

/// 可选提示后从标准输入读取一行；调用方拥有返回值并负责释放其中的句柄。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_input(prompt: *const XiaoValue, has_prompt: u8) -> XiaoValue {
    if has_prompt > 1 || (has_prompt == 1 && prompt.is_null()) {
        set_pending_runtime_error(RuntimeError::invalid_value("input 的提示参数边界无效"));
        return XiaoValue::none();
    }
    if has_prompt == 1 {
        let prompt = match unsafe { value_to_runtime(&*prompt) } {
            Ok(value) => value,
            Err(error) => {
                set_pending_runtime_error(RuntimeError::invalid_value(format!(
                    "input 提示值无效（状态码 {error}）"
                )));
                return XiaoValue::none();
            }
        };
        if !matches!(prompt, RuntimeValue::Str(_)) {
            set_pending_runtime_error(RuntimeError::type_mismatch("str", prompt.type_name()));
            return XiaoValue::none();
        }
        let text = match prompt.display_text() {
            Ok(text) => text,
            Err(error) => {
                set_pending_runtime_error(error);
                return XiaoValue::none();
            }
        };
        let mut stdout = std::io::stdout().lock();
        if stdout
            .write_all(text.as_bytes())
            .and_then(|_| stdout.flush())
            .is_err()
        {
            set_pending_runtime_error(RuntimeError::invalid_value("写入标准输出失败"));
            return XiaoValue::none();
        }
    }
    let mut line = String::new();
    let bytes = match std::io::stdin().read_line(&mut line) {
        Ok(bytes) => bytes,
        Err(_) => {
            set_pending_runtime_error(RuntimeError::invalid_value("读取标准输入失败"));
            return XiaoValue::none();
        }
    };
    if bytes == 0 {
        set_pending_runtime_error(RuntimeError::invalid_value("标准输入已结束"));
        return XiaoValue::none();
    }
    while line.ends_with(['\n', '\r']) {
        line.pop();
    }
    let value = match RuntimeValue::new_string(line) {
        Ok(value) => value,
        Err(error) => {
            set_pending_runtime_error(error);
            return XiaoValue::none();
        }
    };
    match runtime_to_value(&value) {
        Ok(value) => value,
        Err(error) => {
            set_pending_runtime_error(RuntimeError::invalid_value(format!(
                "input 返回值构造失败（状态码 {error}）"
            )));
            XiaoValue::none()
        }
    }
}
