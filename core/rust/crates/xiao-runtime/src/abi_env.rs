//! 原生环境入口、线程上下文与 env 借读 C ABI。

use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;

use super::{abi_value_validation::validate_structure, *};
use crate::environment::{
    EnvironmentSnapshot, Failure, MAX_INJECTED_BYTES, Operation, SnapshotError,
};
use xiao_intrinsics::{CAPABILITY_ENVIRONMENT, validate_capability_mask};
use xiao_runtime_abi::native_control::{
    CANCEL_ENV, CONTROL_PREFIX, DEADLINE_ENV, INTERRUPTED_EXIT, Interruption, monotonic_ns,
};
use xiao_runtime_abi::{NATIVE_ENVIRONMENT_NONCE_ENV, NATIVE_ENVIRONMENT_STDIN_PREFIX};

thread_local! {
    static ENTRY_ENVIRONMENT: RefCell<Option<Arc<NativeEnvironment>>> = const { RefCell::new(None) };
}

/// 启动入口固定的快照和授权，在线程之间一起传递。
pub(super) struct NativeEnvironment {
    snapshot: EnvironmentSnapshot,
    capabilities: u64,
    control: NativeControl,
}

/// 从私有启动握手读取的运行级截止和取消通知。
#[derive(Default)]
struct NativeControl {
    deadline_ns: Option<u128>,
    cancellation: Option<PathBuf>,
}

impl NativeControl {
    /// 从子进程私有环境变量读取截止和取消文件。
    fn from_launch() -> Result<Self, std::num::ParseIntError> {
        Ok(Self {
            deadline_ns: std::env::var_os(DEADLINE_ENV)
                .map(|deadline| deadline.to_string_lossy().parse())
                .transpose()?,
            cancellation: std::env::var_os(CANCEL_ENV).map(PathBuf::from),
        })
    }

    /// 返回取消优先于截止的中断原因。
    fn interruption(&self) -> Option<Interruption> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(|path| std::fs::metadata(path).is_ok_and(|metadata| metadata.len() != 0))
        {
            return Some(Interruption::Cancelled);
        }
        if self
            .deadline_ns
            .is_some_and(|deadline| monotonic_ns().map_or(true, |now| now >= deadline))
        {
            return Some(Interruption::Deadline);
        }
        None
    }
}

impl NativeEnvironment {
    /// 在 env 提交前检查运行中断和独立任务取消。
    fn checkpoint(&self) -> Result<(), i32> {
        if let Some(interruption) = self.control.interruption() {
            // 运行级中断沿已冻结的不可捕获错误通道传播，退出码仍为 2。
            set_pending_error(PendingError::Fatal(FatalError::new(
                FatalKind::Internal,
                interruption.code(),
                "x11.driver.rejected",
                interruption.message(),
            )));
            return Err(XiaoAbiStatus::Fatal.code());
        }
        let status = super::abi_concurrency::cancellation_status();
        if status == XiaoAbiStatus::Ok.code() {
            Ok(())
        } else {
            Err(status)
        }
    }
}

/// 从提交线程捕获同一次运行的快照，供任务调度使用。
pub(super) fn entry_environment() -> Option<Arc<NativeEnvironment>> {
    ENTRY_ENVIRONMENT.with_borrow(Clone::clone)
}

/// 入口或任务结束时恢复调用者的上下文。
pub(super) struct EnvironmentGuard(Option<Arc<NativeEnvironment>>);

impl Drop for EnvironmentGuard {
    /// 恢复进入前保存的线程上下文。
    fn drop(&mut self) {
        ENTRY_ENVIRONMENT.replace(self.0.take());
    }
}

/// 在当前线程安装一次运行的共享快照。
pub(super) fn enter_environment(environment: Option<Arc<NativeEnvironment>>) -> EnvironmentGuard {
    EnvironmentGuard(ENTRY_ENVIRONMENT.replace(environment))
}

/// 安装已接纳快照和运行控制后进入用户代码。
fn run_entry(
    entry: impl FnOnce(&NativeControl) -> i32,
    snapshot: Result<EnvironmentSnapshot, SnapshotError>,
    capabilities: u64,
    control: NativeControl,
) -> i32 {
    match snapshot {
        Ok(snapshot) => {
            let environment = Arc::new(NativeEnvironment {
                snapshot,
                capabilities,
                control,
            });
            let _environment = enter_environment(Some(Arc::clone(&environment)));
            entry(&environment.control)
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
    if let Err(error) = validate_capability_mask(capabilities) {
        let error = SnapshotError::InvalidCapabilities(error);
        let _ = writeln!(std::io::stderr().lock(), "{}: {error}", error.code());
        return 2;
    }
    let handshake = if capabilities & CAPABILITY_ENVIRONMENT != 0 {
        environment_handshake()
    } else {
        None
    };
    let control = if handshake.is_some() {
        let Ok(control) = NativeControl::from_launch() else {
            let _ = writeln!(
                std::io::stderr().lock(),
                "X09-DRIVER-003: 原生运行截止字段无效"
            );
            return 2;
        };
        control
    } else {
        NativeControl::default()
    };
    if let Some(status) = reject_interrupted_entry(&control) {
        return status;
    }
    let snapshot = match handshake {
        Some(LaunchEnvironment::Stdin) => read_injected_environment(std::io::stdin().lock()),
        Some(LaunchEnvironment::Host) => EnvironmentSnapshot::from_host(capabilities, || {
            std::env::vars_os().filter(|pair| {
                crate::environment::host_pair(pair)
                    && ![NATIVE_ENVIRONMENT_NONCE_ENV, DEADLINE_ENV, CANCEL_ENV]
                        .iter()
                        .any(|name| {
                            let key = pair.0.as_encoded_bytes();
                            if cfg!(windows) {
                                key.eq_ignore_ascii_case(name.as_bytes())
                            } else {
                                key == name.as_bytes()
                            }
                        })
            })
        }),
        None => EnvironmentSnapshot::capture_host(capabilities),
    };
    run_entry(
        |control| {
            let _arguments = if capture_arguments != 0 {
                match crate::arguments::ArgumentsSnapshot::from_host_after_marker(
                    handshake.is_some(),
                ) {
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
            reject_interrupted_entry(control).unwrap_or_else(|| entry())
        },
        snapshot,
        capabilities,
        control,
    )
}

/// 在执行用户代码前拒绝已取消或已到期的启动。
fn reject_interrupted_entry(control: &NativeControl) -> Option<i32> {
    control.interruption().map(|interruption| {
        let _ = writeln!(
            std::io::stderr().lock(),
            "{}: {}",
            interruption.code(),
            interruption.message()
        );
        INTERRUPTED_EXIT
    })
}

/// 原生启动握手的快照来源。
enum LaunchEnvironment {
    /// 从标准输入读取显式 JSON 快照。
    Stdin,
    /// 从宿主环境捕获一次快照。
    Host,
}

/// 仅消费与子进程私有环境变量匹配的握手参数。
fn environment_handshake() -> Option<LaunchEnvironment> {
    let arg = std::env::args_os().nth(1)?;
    let arg = arg.to_str()?;
    let (nonce, source) = if let Some(nonce) = arg.strip_prefix(NATIVE_ENVIRONMENT_STDIN_PREFIX) {
        (nonce, LaunchEnvironment::Stdin)
    } else {
        (arg.strip_prefix(CONTROL_PREFIX)?, LaunchEnvironment::Host)
    };
    (nonce.len() == 32
        && nonce.bytes().all(|byte| byte.is_ascii_hexdigit())
        && std::env::var(NATIVE_ENVIRONMENT_NONCE_ENV).is_ok_and(|expected| expected == nonce))
    .then_some(source)
}

/// 标准输入没有长度字段。先有界读取，遇到超限块时不扩容或解码。
fn read_injected_environment(
    mut input: impl BufRead,
) -> Result<EnvironmentSnapshot, SnapshotError> {
    let mut data = Vec::new();
    loop {
        let chunk = input
            .fill_buf()
            .map_err(|_| SnapshotError::InvalidEncoding)?;
        if chunk.is_empty() {
            break;
        }
        let remaining = MAX_INJECTED_BYTES - data.len();
        if chunk.len() > remaining {
            return Err(SnapshotError::LimitExceeded);
        }
        let len = data.len() + chunk.len();
        if len > data.capacity() {
            let capacity = (data.capacity() * 2).max(len).min(MAX_INJECTED_BYTES);
            data.try_reserve_exact(capacity - data.len())
                .map_err(|_| SnapshotError::AllocationFailed)?;
        }
        data.extend_from_slice(chunk);
        let consumed = chunk.len();
        input.consume(consumed);
    }
    EnvironmentSnapshot::from_injected_json(&data)
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
        if len > MAX_INJECTED_BYTES || len > isize::MAX as usize {
            return Err(SnapshotError::LimitExceeded);
        }
        if data.is_null() {
            return if len == 0 {
                Ok(EnvironmentSnapshot::default())
            } else {
                Err(SnapshotError::InvalidEncoding)
            };
        }
        // SAFETY: 调用者在本次调用期间借出完整可读的 UTF-8 JSON 载荷。
        let data = unsafe { std::slice::from_raw_parts(data, len) };
        EnvironmentSnapshot::from_injected_json(data)
    })();
    run_entry(
        |_| entry(),
        snapshot,
        capabilities,
        NativeControl::default(),
    )
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
            let Some(entry) = &entry else {
                set_pending_error(PendingError::Fatal(FatalError::internal(
                    "env 调用缺少原生运行上下文",
                )));
                return Err(XiaoAbiStatus::Fatal.code());
            };
            entry.checkpoint()
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

/// env ABI 单元测试。
#[cfg(test)]
#[path = "abi_env_tests.rs"]
mod tests;
