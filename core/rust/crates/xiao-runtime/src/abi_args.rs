//! 拥有输出槽的 args C ABI。

use super::*;
use crate::arguments::{ArgumentsSnapshot, EMPTY_ARGUMENTS};
use std::sync::Arc;
use xiao_intrinsics::validate_capability_mask;

thread_local! {
    static ENTRY_ARGUMENTS: RefCell<Option<Arc<ArgumentsSnapshot>>> = const { RefCell::new(None) };
}

/// 捕获当前原生运行的快照，供调度器传递给子任务。
pub(super) fn entry_arguments() -> Option<Arc<ArgumentsSnapshot>> {
    ENTRY_ARGUMENTS.with_borrow(Clone::clone)
}

/// 归还入口或任务占用的线程时恢复上一份快照。
pub(super) struct ArgumentsGuard(Option<Arc<ArgumentsSnapshot>>);

impl Drop for ArgumentsGuard {
    fn drop(&mut self) {
        ENTRY_ARGUMENTS.replace(self.0.take());
    }
}

/// 为主入口或原生并发任务安装已接纳的快照。
pub(super) fn enter_arguments(snapshot: Option<Arc<ArgumentsSnapshot>>) -> ArgumentsGuard {
    ArgumentsGuard(ENTRY_ARGUMENTS.replace(snapshot))
}

/// 生成器的独立可执行入口适配器。初始化成功后才调用应用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_args_run_native(entry: Option<extern "C" fn() -> i32>) -> i32 {
    let Some(entry) = entry else {
        return XiaoAbiStatus::InvalidArgument.code();
    };
    let snapshot = match ArgumentsSnapshot::from_host() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let _ = writeln!(std::io::stderr().lock(), "{}: {error}", error.code());
            return 2;
        }
    };
    let _arguments = enter_arguments(Some(Arc::new(snapshot)));
    entry()
}

/// 在已经初始化的输出槽上执行一次 args 读取。
fn read_arguments(
    out: *mut XiaoValue,
    capabilities: u64,
    read: impl FnOnce(&ArgumentsSnapshot, u64) -> RuntimeResult<XiaoValue>,
) -> i32 {
    let run = || {
        let output = unsafe { out.as_ref() }.ok_or(XiaoAbiStatus::Null.code())?;
        if validate_capability_mask(capabilities).is_err() {
            return Err(XiaoAbiStatus::InvalidArgument.code());
        }
        unsafe { super::abi_value_validation::validate_structure(output, true) }?;
        let value = ENTRY_ARGUMENTS.with_borrow(|snapshot| {
            status(read(
                snapshot.as_deref().unwrap_or(&EMPTY_ARGUMENTS),
                capabilities,
            ))
        })?;
        unsafe { write_value(out, value) }
    };
    run().map_or_else(|error| error, |()| XiaoAbiStatus::Ok.code())
}

/// 读取当前运行的程序名。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_args_program_name(out: *mut XiaoValue, capabilities: u64) -> i32 {
    read_arguments(out, capabilities, |snapshot, capabilities| {
        let name = snapshot.program_name(capabilities)?;
        Ok(value_from_owned_handle(
            XiaoValueTag::Str,
            name.into_strong_handle(),
        ))
    })
}

/// 读取当前运行的参数，调用方拥有独立数组。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_args_argv(out: *mut XiaoValue, capabilities: u64) -> i32 {
    read_arguments(out, capabilities, |snapshot, capabilities| {
        let argv = snapshot.argv(capabilities)?;
        Ok(value_from_owned_handle(
            XiaoValueTag::Array,
            argv.into_strong_handle(),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use xiao_intrinsics::CAPABILITY_PROCESS;

    /// 读取 ABI 调用设置的可恢复诊断编号。
    fn assert_pending_code(expected: &str) {
        PENDING_ERROR.with_borrow(|pending| {
            let Some(PendingError::Recoverable(error)) = pending else {
                panic!("缺少挂起的 Runtime 错误");
            };
            assert_eq!(error.code(), expected);
        });
    }

    /// 失败保留槽和句柄，成功交付拥有的新对象。
    #[test]
    fn abi_validates_before_replacing_owned_output() {
        let mut out = XiaoValue::int(7);
        assert_eq!(
            xiao_runtime_args_argv(std::ptr::null_mut(), CAPABILITY_PROCESS),
            1
        );
        assert_eq!(xiao_runtime_args_argv(&mut out, 1 << 8), 2);
        let mut invalid = XiaoValue {
            tag: XiaoValueTag::Str,
            payload: XiaoValuePayload {
                handle: std::ptr::null_mut(),
            },
        };
        assert_eq!(xiao_runtime_args_argv(&mut invalid, CAPABILITY_PROCESS), 3);
        assert_eq!(xiao_runtime_args_argv(&mut out, 0), 6);
        assert_pending_code("X06-RUNTIME-026");
        assert_eq!(
            unsafe { value_to_runtime(&out) }.unwrap(),
            RuntimeValue::Int(7)
        );
        xiao_runtime_error_clear();

        let fail = crate::allocation_testing::StringAllocationFailure::after(0);
        assert_eq!(
            xiao_runtime_args_program_name(&mut out, CAPABILITY_PROCESS),
            6
        );
        assert!(fail.was_triggered());
        assert_pending_code(crate::ALLOCATION_CODE);
        assert_eq!(
            unsafe { value_to_runtime(&out) }.unwrap(),
            RuntimeValue::Int(7)
        );
        xiao_runtime_error_clear();

        assert_eq!(
            xiao_runtime_args_program_name(&mut out, CAPABILITY_PROCESS),
            0
        );
        let RuntimeValue::Str(name) = unsafe { value_to_runtime(&out) }.unwrap() else {
            panic!("程序名应为 str");
        };
        assert!(name.is_empty());
        assert_eq!(xiao_runtime_args_argv(&mut out, CAPABILITY_PROCESS), 0);
        let RuntimeValue::Array(values) = unsafe { value_to_runtime(&out) }.unwrap() else {
            panic!("argv 应为数组");
        };
        values.push(RuntimeValue::Int(8)).unwrap();
        assert_eq!(xiao_runtime_args_argv(&mut out, CAPABILITY_PROCESS), 0);
        let RuntimeValue::Array(next) = unsafe { value_to_runtime(&out) }.unwrap() else {
            panic!("argv 应为数组");
        };
        assert!(next.is_empty());
        assert_eq!(values.len(), 1);
        xiao_runtime_value_release_strong(&mut out);
    }
}
