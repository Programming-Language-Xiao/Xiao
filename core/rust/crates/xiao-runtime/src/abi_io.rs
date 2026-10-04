//! `print`/`input` intrinsic 的 Runtime ABI 实现。

use std::io::Write;
use std::slice;

use xiao_runtime_abi::{XiaoAbiStatus, XiaoValue};

use super::{
    RuntimeError, RuntimeValue, runtime_to_value, set_pending_runtime_error, status_from_error,
    value_to_runtime,
};

#[cfg(test)]
static TEST_OUTPUT: std::sync::OnceLock<std::sync::Mutex<Vec<String>>> = std::sync::OnceLock::new();

#[cfg(test)]
fn record_test_output(text: String) {
    TEST_OUTPUT
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("测试输出锁不应中毒")
        .push(text);
}

#[cfg(test)]
fn take_test_output() -> Vec<String> {
    std::mem::take(
        &mut *TEST_OUTPUT
            .get_or_init(|| std::sync::Mutex::new(Vec::new()))
            .lock()
            .expect("测试输出锁不应中毒"),
    )
}

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
    let output = format!("{}\n", rendered.join(" "));
    let mut stdout = std::io::stdout().lock();
    if stdout
        .write_all(output.as_bytes())
        .and_then(|_| stdout.flush())
        .is_ok()
    {
        #[cfg(test)]
        record_test_output(output);
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

#[cfg(test)]
mod tests {
    use std::ptr;

    use super::{xiao_runtime_input, xiao_runtime_print_values};
    use xiao_runtime_abi::XiaoValue;
    use xiao_runtime_abi::{XiaoAbiStatus, XiaoErrorClass};

    #[test]
    /// 真实调用输出包装的参数边界；删除 ABI 包装会使该语义用例失败。
    fn removal_verification_print_wrapper_rejects_null_nonempty_input() {
        assert_eq!(
            xiao_runtime_print_values(ptr::null(), 1),
            XiaoAbiStatus::InvalidArgument.code()
        );
    }

    #[test]
    /// 符号保留但输出行为被破坏时，该运行期用例必须失败。
    fn removal_verification_print_wrapper_writes_stdout_behavior() {
        let value = XiaoValue::none();
        assert_eq!(
            xiao_runtime_print_values(&value, 1),
            XiaoAbiStatus::Ok.code()
        );
        assert_eq!(super::take_test_output(), vec!["none\n".to_owned()]);
    }

    #[test]
    /// 真实调用输入包装的契约边界；非法提示标志必须进入挂起错误槽。
    fn removal_verification_input_wrapper_rejects_invalid_prompt_flag() {
        let _ = xiao_runtime_input(ptr::null(), 2);
        assert_eq!(super::super::pending_class(), XiaoErrorClass::Recoverable);
        super::super::xiao_runtime_error_clear();
    }
}
