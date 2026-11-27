//! 原生环境入口、线程上下文与 env 借读 C ABI。

use std::io::Read;
use std::sync::Arc;

use super::{abi_value_validation::validate_structure, *};
use crate::environment::{EnvironmentSnapshot, Failure, Operation, SnapshotError};
use xiao_intrinsics::validate_capability_mask;
use xiao_runtime_abi::NATIVE_ENVIRONMENT_STDIN;

thread_local! {
    static ENTRY_ENVIRONMENT: RefCell<Option<Arc<NativeEnvironment>>> = const { RefCell::new(None) };
}

/// 启动入口固定的快照和授权，在线程之间一起传递。
pub(super) struct NativeEnvironment {
    snapshot: EnvironmentSnapshot,
    capabilities: u64,
}

/// 从提交线程捕获同一次运行的快照，供任务调度使用。
pub(super) fn entry_environment() -> Option<Arc<NativeEnvironment>> {
    ENTRY_ENVIRONMENT.with_borrow(Clone::clone)
}

/// 入口或任务结束时恢复调用者的上下文。
pub(super) struct EnvironmentGuard(Option<Arc<NativeEnvironment>>);

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        ENTRY_ENVIRONMENT.replace(self.0.take());
    }
}

/// 在当前线程安装一次运行的共享快照。
pub(super) fn enter_environment(environment: Option<Arc<NativeEnvironment>>) -> EnvironmentGuard {
    EnvironmentGuard(ENTRY_ENVIRONMENT.replace(environment))
}

fn run_entry(
    entry: impl FnOnce() -> i32,
    snapshot: Result<EnvironmentSnapshot, SnapshotError>,
    capabilities: u64,
) -> i32 {
    match snapshot {
        Ok(snapshot) => {
            let _environment = enter_environment(Some(Arc::new(NativeEnvironment {
                snapshot,
                capabilities,
            })));
            entry()
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr().lock(), "{}: {error}", error.code());
            2
        }
    }
}

/// 独立原生程序在用户代码之前接纳一次环境快照。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_env_run_native(
    entry: Option<extern "C" fn() -> i32>,
    capabilities: u64,
    capture_arguments: i32,
) -> i32 {
    let Some(entry) = entry else {
        return 2;
    };
    let injected = std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == NATIVE_ENVIRONMENT_STDIN);
    let snapshot = if injected {
        validate_capability_mask(capabilities)
            .map_err(SnapshotError::InvalidCapabilities)
            .and_then(|_| read_injected_environment())
    } else {
        EnvironmentSnapshot::capture_host(capabilities)
    };
    run_entry(
        || {
            let _arguments = if capture_arguments != 0 {
                match crate::arguments::ArgumentsSnapshot::from_host_after_marker(injected) {
                    Ok(snapshot) => {
                        Some(super::abi_args::enter_arguments(Some(Arc::new(snapshot))))
                    }
                    Err(error) => {
                        let _ = writeln!(std::io::stderr().lock(), "{}: {error}", error.code());
                        return 2;
                    }
                }
            } else {
                None
            };
            entry()
        },
        snapshot,
        capabilities,
    )
}

/// 驱动发送经接纳的名值对。JSON 转义至多展开六倍，另留出数组结构的字节。
fn read_injected_environment() -> Result<EnvironmentSnapshot, SnapshotError> {
    const MAX_PAYLOAD_BYTES: usize = 6 * 1_048_576 + 8 * 1_024 + 2;
    let mut data = Vec::new();
    std::io::stdin()
        .lock()
        .take((MAX_PAYLOAD_BYTES + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|_| SnapshotError::InvalidEncoding)?;
    if data.len() > MAX_PAYLOAD_BYTES {
        return Err(SnapshotError::LimitExceeded);
    }
    decode_environment(&data)
}

fn decode_environment(data: &[u8]) -> Result<EnvironmentSnapshot, SnapshotError> {
    let pairs: Vec<(String, String)> =
        serde_json::from_slice(data).map_err(|_| SnapshotError::InvalidEncoding)?;
    EnvironmentSnapshot::from_pairs(&pairs)
}

/// 原生嵌入入口。省略载荷时从空快照开始。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xiao_runtime_env_run_injected(
    entry: Option<extern "C" fn() -> i32>,
    data: *const u8,
    len: usize,
    capabilities: u64,
) -> i32 {
    let Some(entry) = entry else {
        return 2;
    };
    let snapshot = (|| {
        validate_capability_mask(capabilities).map_err(SnapshotError::InvalidCapabilities)?;
        if data.is_null() && len == 0 {
            return Ok(EnvironmentSnapshot::default());
        }
        if data.is_null() || len > isize::MAX as usize {
            return Err(SnapshotError::InvalidEncoding);
        }
        // SAFETY: 调用者在本次调用期间借出完整可读的 UTF-8 JSON 载荷。
        let data = unsafe { std::slice::from_raw_parts(data, len) };
        decode_environment(data)
    })();
    run_entry(|| entry(), snapshot, capabilities)
}

/// 先校验独立的 ABI 槽，然后委托共享 Runtime。只有成功时才交付新输出。
fn execute<const N: usize>(
    operation: Operation,
    inputs: [*const XiaoValue; N],
    output: Option<*mut XiaoValue>,
    capabilities: u64,
) -> i32 {
    let run = || {
        if inputs.iter().any(|input| input.is_null()) || output.is_some_and(|out| out.is_null()) {
            return Err(XiaoAbiStatus::Null.code());
        }
        if output.is_some_and(|out| inputs.contains(&out.cast_const()))
            || (N == 2 && inputs[0] == inputs[1])
            || validate_capability_mask(capabilities).is_err()
        {
            return Err(XiaoAbiStatus::InvalidArgument.code());
        }
        // SAFETY: 调用方在整个调用期间保有输入和独立输出槽的强引用。
        let inputs = inputs.map(|input| unsafe { &*input });
        for input in inputs {
            unsafe { validate_structure(input, false) }?;
        }
        if let Some(out) = output {
            unsafe { validate_structure(&*out, true) }?;
        }
        let mut values = std::array::from_fn::<_, N, _>(|_| RuntimeValue::None);
        for (slot, input) in values.iter_mut().zip(inputs) {
            if input.tag != XiaoValueTag::Str {
                return Err(status_from_error(&RuntimeError::type_mismatch(
                    "str",
                    format!("{:?}", input.tag),
                )));
            }
            *slot = unsafe { value_to_runtime(input) }?;
        }
        let entry = entry_environment();
        let empty;
        let (snapshot, capabilities) = match entry.as_deref() {
            Some(entry) => (&entry.snapshot, capabilities & entry.capabilities),
            None => {
                empty = EnvironmentSnapshot::default();
                (&empty, capabilities)
            }
        };
        let result = snapshot.execute(operation, &values.each_ref(), capabilities, || {
            if entry.is_none() {
                set_pending_error(PendingError::Fatal(FatalError::internal(
                    "env 调用缺少原生运行上下文",
                )));
                return Err(XiaoAbiStatus::Fatal.code());
            }
            let status = super::abi_concurrency::cancellation_status();
            if status == XiaoAbiStatus::Ok.code() {
                Ok(())
            } else {
                Err(status)
            }
        });
        let value = match result {
            Ok(value) => value,
            Err(Failure::Runtime(error)) => return Err(status_from_error(&error)),
            Err(Failure::Interrupted(control)) => return Err(control),
        };
        if let Some(out) = output {
            let value = runtime_to_value(&value)?;
            unsafe { write_value(out, value) }?;
        }
        Ok(())
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)) {
        Ok(result) => result.map_or_else(|status| status, |()| XiaoAbiStatus::Ok.code()),
        Err(_) => {
            set_pending_error(PendingError::Fatal(FatalError::internal(
                "环境运行入口发生内部故障",
            )));
            XiaoAbiStatus::Fatal.code()
        }
    }
}

/// 用拥有的零项或一项字符串数组替换输出槽。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_env_get(
    name: *const XiaoValue,
    out: *mut XiaoValue,
    capabilities: u64,
) -> i32 {
    execute(Operation::Get, [name], Some(out), capabilities)
}

/// 借读名称和值，更新当前运行的环境视图。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_env_set(
    name: *const XiaoValue,
    value: *const XiaoValue,
    capabilities: u64,
) -> i32 {
    execute(Operation::Set, [name, value], None, capabilities)
}

/// 删除当前运行的条目并用布尔值替换输出槽。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_env_unset(
    name: *const XiaoValue,
    out: *mut XiaoValue,
    capabilities: u64,
) -> i32 {
    execute(Operation::Unset, [name], Some(out), capabilities)
}

#[cfg(test)]
#[path = "abi_env_tests.rs"]
mod tests;
