//! 原生并发值的 C ABI 适配。
//!
//! 这里的盒子只保存 Runtime 的公开句柄类型或显式共享句柄；LLVM 看到的始终是
//! `XiaoValueTag::Concurrency` 加不透明指针，不能依赖 `RuntimeHandle`、任务池或
//! 共享对象的 Rust 布局。

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use xiao_runtime_abi::{
    XiaoAbiBytes, XiaoAbiStatus, XiaoConcurrencyHandle, XiaoConcurrencyKind, XiaoValue,
    XiaoValueTag,
};

use crate::concurrency::{SharedHandle, SharedOwnedGuard, SharedTag};
use crate::scheduler::runtime_helpers::ThreadHandleReportExt;
use crate::scheduler::{
    RuntimeCancellation, RuntimeExecError, RuntimeHandle, TaskHandle, TaskPool, ThreadHandle,
};
use crate::{RuntimeValue, VmConcurrencyValue, VmTransferValue};

/// 并发 ABI 盒子魔数。
const CONCURRENCY_MAGIC: u64 = 0x5849_414F_434F_4E31;

/// 跨原生执行单元传递的 Runtime 错误包；错误对象本身保留 cause/suppressed。
pub(super) enum NativeErrorPacket {
    /// 可恢复 Xiao 错误。
    Recoverable(crate::RuntimeError),
    /// 不得进入 Xiao catch 的致命错误。
    Fatal(crate::FatalError),
}

/// 把当前 OS 线程挂起的错误包送回原生并发 Runtime。
fn take_native_error_packet() -> Option<NativeErrorPacket> {
    super::PENDING_ERROR.with(|pending| match pending.borrow_mut().take()? {
        super::PendingError::Recoverable(error) => Some(NativeErrorPacket::Recoverable(error)),
        super::PendingError::Fatal(error) => Some(NativeErrorPacket::Fatal(error)),
    })
}

/// 将另一执行单元的错误包恢复到当前 OS 线程错误槽。
fn restore_native_error_packet(packet: NativeErrorPacket) -> u32 {
    match packet {
        NativeErrorPacket::Recoverable(error) => {
            super::set_pending_error(super::PendingError::Recoverable(error));
            xiao_runtime_abi::XiaoErrorClass::Recoverable.raw()
        }
        NativeErrorPacket::Fatal(error) => {
            super::set_pending_error(super::PendingError::Fatal(error));
            xiao_runtime_abi::XiaoErrorClass::Fatal.raw()
        }
    }
}

/// 取消错误 027 是并发控制出口，不参与 Xiao `catch` 类型匹配。
pub(super) fn error_matches(error_type: &str) -> i32 {
    super::PENDING_ERROR.with(|pending| match pending.borrow().as_ref() {
        Some(super::PendingError::Recoverable(error))
            if error.code() != crate::CONCURRENCY_CANCELLED_CODE =>
        {
            i32::from(
                xiao_diagnostics::error_kind_of(error_type)
                    .is_some_and(|kind| kind.matches(error.kind())),
            )
        }
        Some(super::PendingError::Recoverable(_)) | Some(super::PendingError::Fatal(_)) | None => 0,
    })
}

/// 并发盒子中可跨原生边界复制的对象。
#[derive(Clone)]
enum ConcurrencyObject {
    Runtime(RuntimeHandle),
    Thread(Arc<NativeThreadState>),
    Task(Arc<NativeTaskState>),
    Pool(TaskPool),
    Shared(SharedHandle<VmTransferValue>),
    SharedMut(SharedHandle<VmTransferValue>),
    LockGuard(Rc<RefCell<SharedOwnedGuard<VmTransferValue>>>),
}

impl ConcurrencyObject {
    fn kind(&self) -> XiaoConcurrencyKind {
        match self {
            Self::Runtime(_) => XiaoConcurrencyKind::Runtime,
            Self::Thread(_) => XiaoConcurrencyKind::Thread,
            Self::Task(_) => XiaoConcurrencyKind::Task,
            Self::Pool(_) => XiaoConcurrencyKind::Pool,
            Self::Shared(_) => XiaoConcurrencyKind::Shared,
            Self::SharedMut(_) => XiaoConcurrencyKind::SharedMut,
            Self::LockGuard(_) => XiaoConcurrencyKind::LockGuard,
        }
    }
}

/// 原生 trampoline 只允许把固定宽度值带回 Runtime；句柄/堆对象必须经过 XCON 或
/// 显式 shared 通道，不能把原始指针跨 OS 线程复制。
#[derive(Clone, Debug)]
struct NativeRawValue {
    value: NativeRawValueKind,
}

#[derive(Clone, Debug)]
enum NativeRawValueKind {
    /// XCON/显式 shared 边界值。
    Transfer(VmTransferValue),
    /// Runtime 管理的不透明句柄。
    Concurrency(XiaoConcurrencyHandle),
}

unsafe impl Send for NativeRawValueKind {}
unsafe impl Send for NativeRawValue {}

impl Drop for NativeRawValue {
    fn drop(&mut self) {
        if let NativeRawValueKind::Concurrency(handle) = self.value {
            release_handle(handle);
        }
    }
}

struct NativeThreadState {
    handle: Mutex<Option<ThreadHandle<NativeEntryOutcome>>>,
    cancellation: RuntimeCancellation,
    runtime: RuntimeHandle,
}

struct NativeTaskState {
    handle: Mutex<Option<TaskHandle<NativeEntryOutcome>>>,
    cancellation: RuntimeCancellation,
    runtime: RuntimeHandle,
}

enum NativeEntryOutcome {
    Value(NativeRawValue),
    Error(NativeErrorPacket),
    RuntimeFailure(RuntimeExecError),
}

struct NativeDetachedError {
    runtime_id: u64,
    packet: NativeErrorPacket,
}

const NATIVE_DETACHED_ERROR_CAPACITY: usize = 4096;
static NATIVE_DETACHED_ERROR_DROPS: AtomicUsize = AtomicUsize::new(0);

fn native_detached_errors() -> &'static Mutex<Vec<NativeDetachedError>> {
    static ERRORS: OnceLock<Mutex<Vec<NativeDetachedError>>> = OnceLock::new();
    ERRORS.get_or_init(|| Mutex::new(Vec::new()))
}

fn record_native_detached(runtime_id: u64, result: Result<NativeEntryOutcome, RuntimeExecError>) {
    match result {
        Ok(NativeEntryOutcome::Value(_)) => {}
        Ok(NativeEntryOutcome::Error(packet)) => push_native_detached_error(runtime_id, packet),
        Ok(NativeEntryOutcome::RuntimeFailure(error)) | Err(error) => push_native_detached_error(
            runtime_id,
            NativeErrorPacket::Recoverable(crate::RuntimeError::new(
                crate::XiaoErrorKind::Concurrency,
                error.code(),
                "runtime.concurrency_native",
                error.to_string(),
            )),
        ),
    }
}

fn push_native_detached_error(runtime_id: u64, packet: NativeErrorPacket) {
    let Ok(mut errors) = native_detached_errors().lock() else {
        NATIVE_DETACHED_ERROR_DROPS.fetch_add(1, Ordering::AcqRel);
        return;
    };
    if errors.len() >= NATIVE_DETACHED_ERROR_CAPACITY {
        NATIVE_DETACHED_ERROR_DROPS.fetch_add(1, Ordering::AcqRel);
        return;
    }
    errors.push(NativeDetachedError { runtime_id, packet });
}

fn take_native_detached_error_for(runtime_ids: &[u64]) -> Option<(u64, NativeErrorPacket)> {
    let Ok(mut errors) = native_detached_errors().lock() else {
        return None;
    };
    let index = errors
        .iter()
        .position(|error| runtime_ids.contains(&error.runtime_id))?;
    let error = errors.remove(index);
    Some((error.runtime_id, error.packet))
}

impl Drop for NativeThreadState {
    fn drop(&mut self) {
        let Ok(mut handle) = self.handle.lock() else {
            return;
        };
        if let Some(handle) = handle.take() {
            let runtime_id = self.runtime.id();
            handle.detach_with(move |result| record_native_detached(runtime_id, result));
        }
    }
}

impl Drop for NativeTaskState {
    fn drop(&mut self) {
        let Ok(mut handle) = self.handle.lock() else {
            return;
        };
        if let Some(handle) = handle.take() {
            let runtime_id = self.runtime.id();
            handle.detach_with(move |result| record_native_detached(runtime_id, result));
        }
    }
}

/// 可复制的并发句柄盒子；每次 retain 在 `inners` 中增加一个 Runtime 对象别名。
#[repr(C)]
struct AbiConcurrency {
    magic: u64,
    kind: u32,
    _reserved: u32,
    inners: Mutex<Vec<ConcurrencyObject>>,
}

fn box_object(object: ConcurrencyObject) -> XiaoConcurrencyHandle {
    let kind = object.kind().raw();
    Box::into_raw(Box::new(AbiConcurrency {
        magic: CONCURRENCY_MAGIC,
        kind,
        _reserved: 0,
        inners: Mutex::new(vec![object]),
    }))
    .cast()
}

unsafe fn object_ref<'a>(handle: XiaoConcurrencyHandle) -> Result<&'a AbiConcurrency, i32> {
    if handle.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    let object = unsafe { &*handle.cast::<AbiConcurrency>() };
    if object.magic != CONCURRENCY_MAGIC || XiaoConcurrencyKind::from_raw(object.kind).is_none() {
        return Err(XiaoAbiStatus::InvalidHandle.code());
    }
    Ok(object)
}

fn object_kind(handle: XiaoConcurrencyHandle) -> Result<XiaoConcurrencyKind, i32> {
    // SAFETY: `object_ref` 校验了不透明盒子的魔数和种类。
    unsafe { object_ref(handle) }.and_then(|object| {
        XiaoConcurrencyKind::from_raw(object.kind).ok_or(XiaoAbiStatus::InvalidHandle.code())
    })
}

fn clone_object(handle: XiaoConcurrencyHandle) -> Result<ConcurrencyObject, i32> {
    // SAFETY: `object_ref` 只读取 Runtime 返回的 live 盒子。
    let object = unsafe { object_ref(handle) }?;
    object
        .inners
        .lock()
        .map_err(|_| XiaoAbiStatus::RuntimeError.code())?
        .last()
        .cloned()
        .ok_or(XiaoAbiStatus::InvalidHandle.code())
}

pub(super) fn retain_handle(handle: XiaoConcurrencyHandle) -> XiaoConcurrencyHandle {
    let Ok(object) = (unsafe { object_ref(handle) }) else {
        return std::ptr::null_mut();
    };
    let Ok(mut inners) = object.inners.lock() else {
        return std::ptr::null_mut();
    };
    let Some(last) = inners.last().cloned() else {
        return std::ptr::null_mut();
    };
    inners.push(last);
    handle
}

pub(super) fn release_handle(handle: XiaoConcurrencyHandle) {
    if handle.is_null() {
        return;
    }
    let ptr = handle.cast::<AbiConcurrency>();
    let should_drop = {
        // SAFETY: 句柄必须来自本模块的构造/retain；错误地址会被魔数检查拒绝。
        let Ok(object) = (unsafe { object_ref(handle) }) else {
            return;
        };
        let Ok(mut inners) = object.inners.lock() else {
            return;
        };
        inners.pop();
        inners.is_empty()
    };
    if should_drop {
        // SAFETY: 最后一份别名已经移出，盒子不再被使用。
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

pub(super) fn handle_kind(handle: XiaoConcurrencyHandle) -> u32 {
    object_kind(handle).map_or(0, XiaoConcurrencyKind::raw)
}

fn raw_from_value(value: &XiaoValue, runtime_id: u64) -> Result<NativeRawValue, i32> {
    if value.tag == XiaoValueTag::Concurrency {
        let handle = handle_from_value(value)?;
        let retained = retain_handle(handle);
        if retained.is_null() {
            return Err(XiaoAbiStatus::InvalidHandle.code());
        }
        return Ok(NativeRawValue {
            value: NativeRawValueKind::Concurrency(retained),
        });
    }
    // SAFETY: 调用方提供的 ABI 值在当前调用期间保持有效。
    let runtime_value = unsafe { super::value_to_runtime(value) }?;
    let transfer =
        crate::VmTransferValue::encode(&runtime_value, runtime_id).map_err(set_transfer_error)?;
    Ok(NativeRawValue {
        value: NativeRawValueKind::Transfer(transfer),
    })
}

fn set_transfer_error(error: crate::XiaoError) -> i32 {
    super::set_pending_runtime_error(crate::RuntimeError::new(
        crate::XiaoErrorKind::Concurrency,
        error.code(),
        "runtime.concurrency_serialization",
        error.to_string(),
    ));
    XiaoAbiStatus::RuntimeError.code()
}

fn value_from_raw(value: NativeRawValue, runtime_id: u64) -> Result<XiaoValue, i32> {
    match &value.value {
        NativeRawValueKind::Concurrency(handle) => {
            let handle = retain_handle(*handle);
            if handle.is_null() {
                Err(XiaoAbiStatus::InvalidHandle.code())
            } else {
                Ok(value_from_handle(handle))
            }
        }
        NativeRawValueKind::Transfer(transfer) => {
            let decoded = transfer
                .clone()
                .decode(runtime_id)
                .map_err(set_transfer_error)?;
            super::runtime_to_value(&decoded)
        }
    }
}

fn native_arguments(arguments: &[XiaoValue], runtime_id: u64) -> Result<Vec<NativeRawValue>, i32> {
    let mut values = Vec::with_capacity(arguments.len());
    let mut total_bytes = 0_usize;
    for argument in arguments {
        let value = match raw_from_value(argument, runtime_id) {
            Ok(value) => value,
            Err(status) => {
                while values.pop().is_some() {}
                return Err(status);
            }
        };
        let byte_len = match &value.value {
            NativeRawValueKind::Concurrency(_) => 16,
            NativeRawValueKind::Transfer(transfer) => transfer.byte_len(),
        };
        let Some(next_total) = total_bytes.checked_add(byte_len) else {
            while values.pop().is_some() {}
            return Err(set_concurrency_error(
                crate::CONCURRENCY_SERIALIZATION_CODE,
                "边界帧大小溢出",
            ));
        };
        total_bytes = next_total;
        if total_bytes > crate::XCON_MAX_FRAME_BYTES {
            while values.pop().is_some() {}
            return Err(set_concurrency_error(
                crate::CONCURRENCY_SERIALIZATION_CODE,
                "跨线程参数总大小超过 XCON 限制",
            ));
        }
        values.push(value);
    }
    Ok(values)
}

fn invoke_entry(
    entry: xiao_runtime_abi::XiaoConcurrencyEntry,
    context: usize,
    arguments: Vec<NativeRawValue>,
    runtime_id: u64,
) -> NativeEntryOutcome {
    let values = arguments
        .into_iter()
        .map(|value| value_from_raw(value, runtime_id))
        .collect::<Result<Vec<_>, _>>();
    let Ok(mut values) = values else {
        return take_native_error_packet().map_or(
            NativeEntryOutcome::RuntimeFailure(RuntimeExecError::JoinFailed),
            NativeEntryOutcome::Error,
        );
    };
    let mut output = XiaoValue::none();
    // SAFETY: `entry` 来自 LLVM 产物登记的 trampoline；参数和输出槽在本次调用期间有效。
    let status = unsafe {
        entry(
            context as *mut c_void,
            values.as_ptr(),
            values.len(),
            &mut output,
        )
    };
    for value in &mut values {
        crate::abi::xiao_runtime_value_release(value);
    }
    if status != XiaoAbiStatus::Ok.code() {
        crate::abi::xiao_runtime_value_release(&mut output);
        return take_native_error_packet().map_or(
            NativeEntryOutcome::RuntimeFailure(RuntimeExecError::JoinFailed),
            NativeEntryOutcome::Error,
        );
    }
    let raw = raw_from_value(&output, runtime_id);
    crate::abi::xiao_runtime_value_release(&mut output);
    match raw {
        Ok(value) => NativeEntryOutcome::Value(value),
        Err(_) => take_native_error_packet().map_or(
            NativeEntryOutcome::RuntimeFailure(RuntimeExecError::JoinFailed),
            NativeEntryOutcome::Error,
        ),
    }
}

fn native_outcome_to_value(outcome: NativeEntryOutcome, runtime_id: u64) -> Result<XiaoValue, i32> {
    match outcome {
        NativeEntryOutcome::Value(value) => value_from_raw(value, runtime_id),
        NativeEntryOutcome::Error(error) => {
            let class = restore_native_error_packet(error);
            Err(if class == xiao_runtime_abi::XiaoErrorClass::Fatal.raw() {
                XiaoAbiStatus::Fatal.code()
            } else {
                XiaoAbiStatus::RuntimeError.code()
            })
        }
        NativeEntryOutcome::RuntimeFailure(error) => Err(map_exec_error(error)),
    }
}

pub(super) fn value_from_handle(handle: XiaoConcurrencyHandle) -> XiaoValue {
    if handle.is_null() {
        return XiaoValue::none();
    }
    XiaoValue {
        tag: XiaoValueTag::Concurrency,
        payload: xiao_runtime_abi::XiaoValuePayload {
            handle: handle.cast::<xiao_runtime_abi::XiaoOpaqueHandle>(),
        },
    }
}

pub(super) fn handle_from_value(value: &XiaoValue) -> Result<XiaoConcurrencyHandle, i32> {
    if value.tag != XiaoValueTag::Concurrency {
        return Err(XiaoAbiStatus::InvalidArgument.code());
    }
    let handle =
        unsafe { value.payload.handle }.cast::<xiao_runtime_abi::XiaoConcurrencyOpaqueHandle>();
    object_kind(handle).map(|_| handle)
}

pub(super) fn runtime_value_from_value(value: &XiaoValue) -> Result<RuntimeValue, i32> {
    let handle = handle_from_value(value)?;
    let object = clone_object(handle)?;
    Ok(match object {
        ConcurrencyObject::Runtime(runtime) => {
            RuntimeValue::Concurrency(VmConcurrencyValue::Runtime(runtime))
        }
        ConcurrencyObject::Thread(_)
        | ConcurrencyObject::Task(_)
        | ConcurrencyObject::LockGuard(_) => return Err(XiaoAbiStatus::InvalidArgument.code()),
        ConcurrencyObject::Pool(pool) => RuntimeValue::Concurrency(VmConcurrencyValue::Pool(pool)),
        ConcurrencyObject::Shared(shared) => {
            RuntimeValue::Concurrency(VmConcurrencyValue::Shared(shared))
        }
        ConcurrencyObject::SharedMut(shared) => {
            RuntimeValue::Concurrency(VmConcurrencyValue::SharedMut(shared))
        }
    })
}

pub(super) fn value_from_runtime(value: &RuntimeValue) -> Result<XiaoValue, i32> {
    let object = match value {
        RuntimeValue::Concurrency(VmConcurrencyValue::Runtime(runtime)) => {
            ConcurrencyObject::Runtime(runtime.clone())
        }
        RuntimeValue::Concurrency(VmConcurrencyValue::Pool(pool)) => {
            ConcurrencyObject::Pool(pool.clone())
        }
        RuntimeValue::Concurrency(VmConcurrencyValue::Shared(shared)) => {
            ConcurrencyObject::Shared(shared.clone())
        }
        RuntimeValue::Concurrency(VmConcurrencyValue::SharedMut(shared)) => {
            ConcurrencyObject::SharedMut(shared.clone())
        }
        _ => return Err(XiaoAbiStatus::InvalidArgument.code()),
    };
    Ok(value_from_handle(box_object(object)))
}

fn runtime_argument(value: &XiaoValue) -> Result<RuntimeHandle, i32> {
    match clone_object(handle_from_value(value)?)? {
        ConcurrencyObject::Runtime(runtime) => Ok(runtime),
        _ => Err(XiaoAbiStatus::InvalidHandle.code()),
    }
}

fn pool_argument(value: &XiaoValue) -> Result<TaskPool, i32> {
    match clone_object(handle_from_value(value)?)? {
        ConcurrencyObject::Pool(pool) => Ok(pool),
        _ => Err(XiaoAbiStatus::InvalidHandle.code()),
    }
}

fn set_runtime_error(message: impl Into<String>) -> i32 {
    set_concurrency_error(crate::CONCURRENCY_RUNTIME_CLOSED_CODE, message)
}

fn set_concurrency_error(code: &str, message: impl Into<String>) -> i32 {
    super::set_pending_runtime_error(crate::RuntimeError::new(
        crate::XiaoErrorKind::Concurrency,
        code,
        "runtime.concurrency_native",
        message,
    ));
    XiaoAbiStatus::RuntimeError.code()
}

fn map_exec_error(error: RuntimeExecError) -> i32 {
    super::set_pending_runtime_error(crate::RuntimeError::new(
        crate::XiaoErrorKind::Concurrency,
        error.code(),
        "runtime.concurrency_native",
        error.to_string(),
    ));
    XiaoAbiStatus::RuntimeError.code()
}

fn bytes_to_operation(operation: XiaoAbiBytes) -> Result<String, i32> {
    // SAFETY: Runtime ABI 只在调用期间借用操作名，长度和指针由统一视图校验。
    unsafe { super::utf8(operation) }
}

fn expect_count(count: usize, expected: usize) -> Result<(), i32> {
    if count == expected {
        Ok(())
    } else {
        Err(XiaoAbiStatus::InvalidArgument.code())
    }
}

fn call_named(
    operation: &str,
    arguments: *const XiaoValue,
    argument_count: usize,
    out: *mut XiaoValue,
) -> i32 {
    xiao_runtime_concurrency_call_with_entry(
        XiaoAbiBytes {
            ptr: operation.as_ptr(),
            len: operation.len(),
        },
        None,
        std::ptr::null_mut(),
        arguments,
        argument_count,
        out,
    )
}

fn call_named_with_entry(
    operation: &str,
    entry: Option<xiao_runtime_abi::XiaoConcurrencyEntry>,
    context: *mut c_void,
    arguments: *const XiaoValue,
    argument_count: usize,
    out: *mut XiaoValue,
) -> i32 {
    xiao_runtime_concurrency_call_with_entry(
        XiaoAbiBytes {
            ptr: operation.as_ptr(),
            len: operation.len(),
        },
        entry,
        context,
        arguments,
        argument_count,
        out,
    )
}

/// 统一的原生并发调用入口；未实现的冻结操作明确写入 Runtime 错误并返回失败状态。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_call(
    operation: XiaoAbiBytes,
    arguments: *const XiaoValue,
    argument_count: usize,
    out: *mut XiaoValue,
) -> i32 {
    xiao_runtime_concurrency_call_with_entry(
        operation,
        None,
        std::ptr::null_mut(),
        arguments,
        argument_count,
        out,
    )
}

/// 带 trampoline 的统一原生并发入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_call_with_entry(
    operation: XiaoAbiBytes,
    entry: Option<xiao_runtime_abi::XiaoConcurrencyEntry>,
    context: *mut c_void,
    arguments: *const XiaoValue,
    argument_count: usize,
    out: *mut XiaoValue,
) -> i32 {
    if out.is_null() || (argument_count != 0 && arguments.is_null()) {
        return XiaoAbiStatus::Null.code();
    }
    let operation = match bytes_to_operation(operation) {
        Ok(operation) => operation,
        Err(error) => return error,
    };
    let arguments = if argument_count == 0 {
        &[]
    } else {
        // SAFETY: 空指针/长度已在入口检查；调用方在调用期间保持数组有效。
        unsafe { std::slice::from_raw_parts(arguments, argument_count) }
    };
    let result: Result<XiaoValue, i32> = (|| match operation.as_str() {
        "current_runtime" => expect_count(argument_count, 0).map(|()| {
            value_from_handle(box_object(ConcurrencyObject::Runtime(
                RuntimeHandle::current(),
            )))
        }),
        "runtime_create" => {
            expect_count(argument_count, 0)?;
            let parent = RuntimeHandle::current();
            let runtime = parent
                .in_context(RuntimeHandle::create)
                .map_err(map_exec_error)?;
            Ok(value_from_handle(box_object(ConcurrencyObject::Runtime(
                runtime,
            ))))
        }
        "runtime_close" => expect_count(argument_count, 1)
            .and_then(|()| runtime_argument(&arguments[0]))
            .and_then(|runtime| {
                RuntimeHandle::current()
                    .in_context(|| runtime.try_close())
                    .map_err(map_exec_error)?
                    .map_err(map_exec_error)
            })
            .map(|()| XiaoValue::none()),
        "pool_create" => {
            if argument_count != 2 {
                Err(XiaoAbiStatus::InvalidArgument.code())
            } else {
                let runtime = runtime_argument(&arguments[0])?;
                let capacity = match arguments[1].tag {
                    XiaoValueTag::Int => unsafe { arguments[1].payload.i64_value },
                    XiaoValueTag::Sint => i64::from(unsafe { arguments[1].payload.i32_value }),
                    _ => return Err(XiaoAbiStatus::InvalidArgument.code()),
                };
                let capacity =
                    usize::try_from(capacity).map_err(|_| XiaoAbiStatus::InvalidArgument.code())?;
                RuntimeHandle::current()
                    .in_context(|| TaskPool::create(runtime, capacity))
                    .map_err(map_exec_error)?
                    .map(|pool| value_from_handle(box_object(ConcurrencyObject::Pool(pool))))
                    .map_err(map_exec_error)
            }
        }
        "shutdown" => expect_count(argument_count, 1)
            .and_then(|()| pool_argument(&arguments[0]))
            .and_then(|pool| {
                RuntimeHandle::current()
                    .in_context(|| pool.shutdown())
                    .map_err(map_exec_error)?
                    .map_err(map_exec_error)
            })
            .map(|()| XiaoValue::none()),
        "shared_read" | "shared_mut" => {
            if argument_count != 1 {
                Err(XiaoAbiStatus::InvalidArgument.code())
            } else {
                // SAFETY: `arguments` 已由入口检查为有效的只读值数组。
                let value = unsafe { super::value_to_runtime(&arguments[0]) }?;
                let runtime_id = RuntimeHandle::current().id();
                let transfer =
                    VmTransferValue::encode(&value, runtime_id).map_err(set_transfer_error)?;
                let tag = if operation == "shared_read" {
                    SharedTag::Read
                } else {
                    SharedTag::Mut
                };
                let shared = SharedHandle::try_new(transfer, runtime_id, tag)
                    .map_err(|_| XiaoAbiStatus::RuntimeError.code())?;
                let object = if operation == "shared_read" {
                    ConcurrencyObject::Shared(shared)
                } else {
                    ConcurrencyObject::SharedMut(shared)
                };
                Ok(value_from_handle(box_object(object)))
            }
        }
        "read" => {
            if argument_count != 1 {
                Err(XiaoAbiStatus::InvalidArgument.code())
            } else {
                let ConcurrencyObject::Shared(shared) =
                    clone_object(handle_from_value(&arguments[0])?)?
                else {
                    return Err(XiaoAbiStatus::InvalidHandle.code());
                };
                let value = shared
                    .get()
                    .map_err(|_| XiaoAbiStatus::InvalidHandle.code())?
                    .decode(shared.runtime_id())
                    .map_err(set_transfer_error)?;
                super::runtime_to_value(&value)
            }
        }
        "lock" => {
            expect_count(argument_count, 1)?;
            let ConcurrencyObject::SharedMut(shared) =
                clone_object(handle_from_value(&arguments[0])?)?
            else {
                return Err(XiaoAbiStatus::InvalidHandle.code());
            };
            let cancellation = crate::current_cancellation_token();
            let guard = shared
                .lock_owned(&cancellation, || crate::cancellation_safe_point().is_err())
                .map_err(|error| match error {
                    crate::SharedLockError::Cancelled => {
                        set_concurrency_error(crate::CONCURRENCY_CANCELLED_CODE, error.to_string())
                    }
                    crate::SharedLockError::InvalidTag => XiaoAbiStatus::InvalidHandle.code(),
                })?;
            Ok(value_from_handle(box_object(ConcurrencyObject::LockGuard(
                Rc::new(RefCell::new(guard)),
            ))))
        }
        "lock_read" => {
            expect_count(argument_count, 1)?;
            let ConcurrencyObject::LockGuard(guard) =
                clone_object(handle_from_value(&arguments[0])?)?
            else {
                return Err(XiaoAbiStatus::InvalidHandle.code());
            };
            let guard_ref = guard
                .try_borrow()
                .map_err(|_| XiaoAbiStatus::RuntimeError.code())?;
            let value = guard_ref
                .decode(guard_ref.runtime_id())
                .map_err(set_transfer_error)?;
            super::runtime_to_value(&value)
        }
        "lock_write" => {
            expect_count(argument_count, 2)?;
            let ConcurrencyObject::LockGuard(guard) =
                clone_object(handle_from_value(&arguments[0])?)?
            else {
                return Err(XiaoAbiStatus::InvalidHandle.code());
            };
            let value = unsafe { super::value_to_runtime(&arguments[1]) }?;
            let runtime_id = transfer_runtime_id_from_guard(&guard)?;
            let transfer =
                VmTransferValue::encode(&value, runtime_id).map_err(set_transfer_error)?;
            **guard
                .try_borrow_mut()
                .map_err(|_| XiaoAbiStatus::RuntimeError.code())? = transfer;
            Ok(XiaoValue::none())
        }
        "spawn" => {
            let Some(entry) = entry else {
                return Err(set_runtime_error("spawn 缺少 trampoline"));
            };
            if argument_count < 1 {
                return Err(XiaoAbiStatus::InvalidArgument.code());
            }
            let runtime = runtime_argument(&arguments[0])?;
            let runtime_id = runtime.id();
            let values = native_arguments(&arguments[1..], runtime_id)?;
            let context = context as usize;
            let handle = RuntimeHandle::current()
                .in_context(|| {
                    crate::scheduler::spawn_cooperative(&runtime, move |_| {
                        invoke_entry(entry, context, values, runtime_id)
                    })
                })
                .map_err(map_exec_error)?
                .map_err(map_exec_error)?;
            let cancellation = handle.cancellation();
            Ok(value_from_handle(box_object(ConcurrencyObject::Thread(
                Arc::new(NativeThreadState {
                    handle: Mutex::new(Some(handle)),
                    cancellation,
                    runtime,
                }),
            ))))
        }
        "join" => {
            expect_count(argument_count, 1)?;
            let object = clone_object(handle_from_value(&arguments[0])?)?;
            let ConcurrencyObject::Thread(state) = object else {
                return Err(XiaoAbiStatus::InvalidHandle.code());
            };
            let handle = state
                .handle
                .lock()
                .map_err(|_| XiaoAbiStatus::RuntimeError.code())?
                .take()
                .ok_or(XiaoAbiStatus::InvalidHandle.code())?;
            let runtime_id = state.runtime.id();
            let value = handle
                .join_interruptible_with_report(
                    || crate::cancellation_safe_point().is_err(),
                    move |result| record_native_detached(runtime_id, result),
                )
                .map_err(map_exec_error)?;
            native_outcome_to_value(value, runtime_id)
        }
        "detach" => {
            expect_count(argument_count, 1)?;
            let object = clone_object(handle_from_value(&arguments[0])?)?;
            match object {
                ConcurrencyObject::Thread(state) => {
                    let handle = state
                        .handle
                        .lock()
                        .map_err(|_| XiaoAbiStatus::RuntimeError.code())?
                        .take()
                        .ok_or(XiaoAbiStatus::InvalidHandle.code())?;
                    let runtime_id = state.runtime.id();
                    handle.detach_with(move |result| record_native_detached(runtime_id, result));
                    Ok(XiaoValue::none())
                }
                ConcurrencyObject::Task(state) => {
                    let handle = state
                        .handle
                        .lock()
                        .map_err(|_| XiaoAbiStatus::RuntimeError.code())?
                        .take()
                        .ok_or(XiaoAbiStatus::InvalidHandle.code())?;
                    let runtime_id = state.runtime.id();
                    handle.detach_with(move |result| record_native_detached(runtime_id, result));
                    Ok(XiaoValue::none())
                }
                _ => Err(XiaoAbiStatus::InvalidHandle.code()),
            }
        }
        "cancel" => {
            expect_count(argument_count, 1)?;
            let object = clone_object(handle_from_value(&arguments[0])?)?;
            match object {
                ConcurrencyObject::Thread(state) => {
                    state.cancellation.cancel();
                    Ok(XiaoValue::none())
                }
                ConcurrencyObject::Task(state) => {
                    state.cancellation.cancel();
                    Ok(XiaoValue::none())
                }
                _ => Err(XiaoAbiStatus::InvalidHandle.code()),
            }
        }
        "submit" => {
            let Some(entry) = entry else {
                return Err(set_runtime_error("submit 缺少 trampoline"));
            };
            if argument_count < 1 {
                return Err(XiaoAbiStatus::InvalidArgument.code());
            }
            let pool = pool_argument(&arguments[0])?;
            let runtime = pool.runtime();
            let runtime_id = runtime.id();
            let values = native_arguments(&arguments[1..], runtime_id)?;
            let context = context as usize;
            let cancellation = crate::current_cancellation_token();
            let handle = RuntimeHandle::current()
                .in_context(|| {
                    pool.submit_cooperative(
                        cancellation,
                        || false,
                        move |_| invoke_entry(entry, context, values, runtime_id),
                    )
                })
                .map_err(map_exec_error)?
                .map_err(map_exec_error)?;
            let cancellation = handle.cancellation();
            Ok(value_from_handle(box_object(ConcurrencyObject::Task(
                Arc::new(NativeTaskState {
                    handle: Mutex::new(Some(handle)),
                    cancellation,
                    runtime,
                }),
            ))))
        }
        "wait" => {
            expect_count(argument_count, 1)?;
            let object = clone_object(handle_from_value(&arguments[0])?)?;
            let ConcurrencyObject::Task(state) = object else {
                return Err(XiaoAbiStatus::InvalidHandle.code());
            };
            let handle = state
                .handle
                .lock()
                .map_err(|_| XiaoAbiStatus::RuntimeError.code())?
                .take()
                .ok_or(XiaoAbiStatus::InvalidHandle.code())?;
            let outcome = handle
                .wait_interruptible(|| crate::cancellation_safe_point().is_err())
                .map_err(map_exec_error)?;
            native_outcome_to_value(outcome, state.runtime.id())
        }
        _ => Err(set_runtime_error(format!("未知原生并发入口 {operation}"))),
    })();
    match result {
        Ok(value) => {
            // SAFETY: `out` 已检查，写入函数会释放输出槽原有拥有值。
            unsafe { super::write_value(out, value) }
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
        }
        Err(error) => error,
    }
}

fn transfer_runtime_id_from_guard(
    guard: &Rc<RefCell<SharedOwnedGuard<VmTransferValue>>>,
) -> Result<u64, i32> {
    guard
        .try_borrow()
        .map(|guard| guard.runtime_id())
        .map_err(|_| XiaoAbiStatus::RuntimeError.code())
}

/// 保留并发不透明句柄。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_retain(
    handle: XiaoConcurrencyHandle,
) -> XiaoConcurrencyHandle {
    retain_handle(handle)
}

/// 释放并发不透明句柄。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_release(handle: XiaoConcurrencyHandle) {
    release_handle(handle);
}

/// 返回并发句柄种类，失效句柄返回零。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_kind(handle: XiaoConcurrencyHandle) -> u32 {
    handle_kind(handle)
}

/// 在 Runtime 线程局部取消上下文中消费一个安全点。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_cancellation_check(operation: XiaoAbiBytes) -> i32 {
    let operation = match bytes_to_operation(operation) {
        Ok(operation) => operation,
        Err(error) => return error,
    };
    if !matches!(operation.as_str(), "safe_point" | "wait" | "lock_wait") {
        return set_runtime_error(format!("未知并发取消检查 {operation}"));
    }
    crate::scheduler::cancellation_safe_point()
        .map_or_else(map_exec_error, |_| XiaoAbiStatus::Ok.code())
}

/// 从有界进程通道中取得一个指定 Runtime 的 detached 错误。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_detached_error_take(
    runtime_ids: *const u64,
    runtime_id_count: usize,
    runtime_id_out: *mut u64,
    error_out: *mut XiaoValue,
) -> i32 {
    if runtime_id_out.is_null() || error_out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    if runtime_id_count != 0 && runtime_ids.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let ids = if runtime_id_count == 0 {
        &[]
    } else {
        // SAFETY: pointer and length are validated and borrowed for this call.
        unsafe { std::slice::from_raw_parts(runtime_ids, runtime_id_count) }
    };
    let Some((runtime_id, packet)) = take_native_detached_error_for(ids) else {
        return XiaoAbiStatus::OutOfBounds.code();
    };
    unsafe { *runtime_id_out = runtime_id };
    let value = match packet {
        NativeErrorPacket::Recoverable(error) => super::value_from_owned_error(error),
        NativeErrorPacket::Fatal(error) => {
            super::set_pending_error(super::PendingError::Fatal(error));
            return XiaoAbiStatus::Fatal.code();
        }
    };
    // SAFETY: output was checked; this replaces and releases its previous owned value.
    unsafe { super::write_value(error_out, value) }
        .map_or_else(|status| status, |_| XiaoAbiStatus::Ok.code())
}

/// 查询并清零 detached 错误通道的有界容量丢弃计数。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_detached_error_drops_take(out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = NATIVE_DETACHED_ERROR_DROPS.swap(0, Ordering::AcqRel) };
    XiaoAbiStatus::Ok.code()
}

/// 创建 Runtime 的稳定 C 入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_runtime_create(out: *mut XiaoValue) -> i32 {
    call_named("runtime_create", std::ptr::null(), 0, out)
}

/// 取得当前 Runtime 的稳定 C 入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_current_runtime(out: *mut XiaoValue) -> i32 {
    call_named("current_runtime", std::ptr::null(), 0, out)
}

/// 关闭 Runtime 的稳定 C 入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_runtime_close(
    runtime: *const XiaoValue,
    out: *mut XiaoValue,
) -> i32 {
    call_named("runtime_close", runtime, 1, out)
}

/// 创建任务池的稳定 C 入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_pool_create(
    runtime: *const XiaoValue,
    capacity: *const XiaoValue,
    out: *mut XiaoValue,
) -> i32 {
    if runtime.is_null() || capacity.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let mut arguments = [XiaoValue::none(), XiaoValue::none()];
    let first_status = crate::abi::xiao_runtime_value_copy(runtime, &mut arguments[0]);
    if first_status != XiaoAbiStatus::Ok.code() {
        return first_status;
    }
    let second_status = crate::abi::xiao_runtime_value_copy(capacity, &mut arguments[1]);
    if second_status != XiaoAbiStatus::Ok.code() {
        crate::abi::xiao_runtime_value_release(&mut arguments[0]);
        return second_status;
    }
    let status = call_named("pool_create", arguments.as_ptr(), arguments.len(), out);
    crate::abi::xiao_runtime_value_release(&mut arguments[0]);
    crate::abi::xiao_runtime_value_release(&mut arguments[1]);
    status
}

/// 关闭任务池的稳定 C 入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_shutdown(
    pool: *const XiaoValue,
    out: *mut XiaoValue,
) -> i32 {
    call_named("shutdown", pool, 1, out)
}

/// 已登记但尚未由 LLVM trampoline 接入的线程入口，显式返回结构化拒绝。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_spawn(
    runtime: XiaoConcurrencyHandle,
    entry: Option<xiao_runtime_abi::XiaoConcurrencyEntry>,
    context: *mut c_void,
    arguments: *const XiaoValue,
    argument_count: usize,
    out: *mut XiaoValue,
) -> i32 {
    let Some(retained) = (if runtime.is_null() {
        None
    } else {
        Some(retain_handle(runtime))
    })
    .filter(|handle| !handle.is_null()) else {
        return XiaoAbiStatus::InvalidHandle.code();
    };
    if argument_count != 0 && arguments.is_null() {
        release_handle(retained);
        return XiaoAbiStatus::Null.code();
    }
    let mut values = vec![value_from_handle(retained)];
    for index in 0..argument_count {
        let value = unsafe { &*arguments.add(index) };
        let mut copy = XiaoValue::none();
        let status = crate::abi::xiao_runtime_value_copy(value, &mut copy);
        if status != XiaoAbiStatus::Ok.code() {
            for value in &mut values {
                crate::abi::xiao_runtime_value_release(value);
            }
            return status;
        }
        values.push(copy);
    }
    let status = call_named_with_entry("spawn", entry, context, values.as_ptr(), values.len(), out);
    for value in &mut values {
        crate::abi::xiao_runtime_value_release(value);
    }
    status
}

/// 已登记但尚未由 LLVM trampoline 接入的任务池入口，显式返回结构化拒绝。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_submit(
    pool: XiaoConcurrencyHandle,
    entry: Option<xiao_runtime_abi::XiaoConcurrencyEntry>,
    context: *mut c_void,
    arguments: *const XiaoValue,
    argument_count: usize,
    out: *mut XiaoValue,
) -> i32 {
    let Some(retained) = (if pool.is_null() {
        None
    } else {
        Some(retain_handle(pool))
    })
    .filter(|handle| !handle.is_null()) else {
        return XiaoAbiStatus::InvalidHandle.code();
    };
    if argument_count != 0 && arguments.is_null() {
        release_handle(retained);
        return XiaoAbiStatus::Null.code();
    }
    let mut values = vec![value_from_handle(retained)];
    for index in 0..argument_count {
        let value = unsafe { &*arguments.add(index) };
        let mut copy = XiaoValue::none();
        let status = crate::abi::xiao_runtime_value_copy(value, &mut copy);
        if status != XiaoAbiStatus::Ok.code() {
            for value in &mut values {
                crate::abi::xiao_runtime_value_release(value);
            }
            return status;
        }
        values.push(copy);
    }
    let status =
        call_named_with_entry("submit", entry, context, values.as_ptr(), values.len(), out);
    for value in &mut values {
        crate::abi::xiao_runtime_value_release(value);
    }
    status
}

macro_rules! unary_concurrency_wrapper {
    ($name:ident, $operation:literal) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(value: *const XiaoValue, out: *mut XiaoValue) -> i32 {
            call_named($operation, value, 1, out)
        }
    };
}

unary_concurrency_wrapper!(xiao_runtime_concurrency_join, "join");
unary_concurrency_wrapper!(xiao_runtime_concurrency_detach, "detach");
unary_concurrency_wrapper!(xiao_runtime_concurrency_cancel, "cancel");
unary_concurrency_wrapper!(xiao_runtime_concurrency_read, "read");
unary_concurrency_wrapper!(xiao_runtime_concurrency_shared_read, "shared_read");
unary_concurrency_wrapper!(xiao_runtime_concurrency_lock, "lock");
unary_concurrency_wrapper!(xiao_runtime_concurrency_lock_read, "lock_read");
unary_concurrency_wrapper!(xiao_runtime_concurrency_wait, "wait");

/// 创建共享可变句柄。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_shared_mut_create(
    value: *const XiaoValue,
    out: *mut XiaoValue,
) -> i32 {
    call_named("shared_mut", value, 1, out)
}

/// 写入锁守卫载荷。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_lock_write(
    guard: *const XiaoValue,
    value: *const XiaoValue,
    out: *mut XiaoValue,
) -> i32 {
    if guard.is_null() || value.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let mut arguments = [XiaoValue::none(), XiaoValue::none()];
    let first_status = crate::abi::xiao_runtime_value_copy(guard, &mut arguments[0]);
    if first_status != XiaoAbiStatus::Ok.code() {
        return first_status;
    }
    let second_status = crate::abi::xiao_runtime_value_copy(value, &mut arguments[1]);
    if second_status != XiaoAbiStatus::Ok.code() {
        crate::abi::xiao_runtime_value_release(&mut arguments[0]);
        return second_status;
    }
    let status = call_named("lock_write", arguments.as_ptr(), arguments.len(), out);
    crate::abi::xiao_runtime_value_release(&mut arguments[0]);
    crate::abi::xiao_runtime_value_release(&mut arguments[1]);
    status
}

/// 让 LLVM/宿主可以显式安装一次 trampoline；当前统一入口保留该 ABI 形状供后续
/// 线程 lowering 使用，`context` 的所有权仍由调用方负责。
#[allow(dead_code)]
fn _assert_entry_abi(
    _entry: Option<xiao_runtime_abi::XiaoConcurrencyEntry>,
    _context: *mut c_void,
) {
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(name: &str) -> XiaoAbiBytes {
        XiaoAbiBytes {
            ptr: name.as_ptr(),
            len: name.len(),
        }
    }

    #[test]
    fn runtime_and_pool_values_use_opaque_concurrency_tag() {
        let mut output = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(
                operation("runtime_create"),
                std::ptr::null(),
                0,
                &mut output
            ),
            0
        );
        assert_eq!(output.tag, XiaoValueTag::Concurrency);
        assert_eq!(
            xiao_runtime_concurrency_kind(unsafe { output.payload.handle }.cast()),
            1
        );
        let retained = xiao_runtime_concurrency_retain(unsafe { output.payload.handle }.cast());
        assert!(!retained.is_null());
        xiao_runtime_concurrency_release(retained);
        crate::abi::xiao_runtime_value_release(&mut output);
    }

    #[test]
    fn unsupported_operation_is_explicitly_rejected() {
        let mut output = XiaoValue::none();
        let status =
            xiao_runtime_concurrency_call(operation("spawn"), std::ptr::null(), 0, &mut output);
        assert_ne!(status, XiaoAbiStatus::Ok.code());
    }

    #[test]
    fn native_trampoline_spawn_join_round_trips_scalar_result() {
        unsafe extern "C" fn entry(
            _context: *mut c_void,
            _arguments: *const XiaoValue,
            _argument_count: usize,
            out: *mut XiaoValue,
        ) -> i32 {
            if out.is_null() {
                return XiaoAbiStatus::Null.code();
            }
            unsafe { *out = XiaoValue::int(7) };
            XiaoAbiStatus::Ok.code()
        }

        let mut runtime = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(
                operation("runtime_create"),
                std::ptr::null(),
                0,
                &mut runtime,
            ),
            0
        );
        let mut thread = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call_with_entry(
                operation("spawn"),
                Some(entry),
                std::ptr::null_mut(),
                &runtime,
                1,
                &mut thread,
            ),
            0
        );
        let mut result = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(operation("join"), &thread, 1, &mut result,),
            0
        );
        assert_eq!(result.tag, XiaoValueTag::Int);
        assert_eq!(unsafe { result.payload.i64_value }, 7);
        crate::abi::xiao_runtime_value_release(&mut result);
        crate::abi::xiao_runtime_value_release(&mut thread);
        crate::abi::xiao_runtime_value_release(&mut runtime);
    }

    #[test]
    fn native_shared_mut_lock_round_trips_and_writes_value() {
        let mut input = XiaoValue::int(1);
        let mut shared = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(operation("shared_mut"), &input, 1, &mut shared,),
            0
        );
        let mut guard = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(operation("lock"), &shared, 1, &mut guard),
            0
        );
        let mut read = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(operation("lock_read"), &guard, 1, &mut read),
            0
        );
        assert_eq!(unsafe { read.payload.i64_value }, 1);
        let updated = XiaoValue::int(2);
        // 该入口只借用参数，数组中的值不拥有额外引用。
        let arguments = [unsafe { std::ptr::read(&guard) }, unsafe {
            std::ptr::read(&updated)
        }];
        let mut none = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(
                operation("lock_write"),
                arguments.as_ptr(),
                arguments.len(),
                &mut none,
            ),
            0
        );
        crate::abi::xiao_runtime_value_release(&mut input);
        crate::abi::xiao_runtime_value_release(&mut shared);
        crate::abi::xiao_runtime_value_release(&mut guard);
        crate::abi::xiao_runtime_value_release(&mut read);
        crate::abi::xiao_runtime_value_release(&mut none);
    }

    #[test]
    fn current_runtime_is_stable_and_child_uses_bound_runtime_context() {
        let mut first = XiaoValue::none();
        let mut second = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(
                operation("current_runtime"),
                std::ptr::null(),
                0,
                &mut first,
            ),
            XiaoAbiStatus::Ok.code()
        );
        assert_eq!(
            xiao_runtime_concurrency_call(
                operation("current_runtime"),
                std::ptr::null(),
                0,
                &mut second,
            ),
            XiaoAbiStatus::Ok.code()
        );
        let first_id = runtime_argument(&first).unwrap().id();
        let second_id = runtime_argument(&second).unwrap().id();
        assert_eq!(first_id, second_id);

        unsafe extern "C" fn current_id(
            context: *mut c_void,
            _arguments: *const XiaoValue,
            _argument_count: usize,
            out: *mut XiaoValue,
        ) -> i32 {
            if out.is_null() {
                return XiaoAbiStatus::Null.code();
            }
            let expected = context as usize as u64;
            let observed = RuntimeHandle::current().id();
            unsafe { *out = XiaoValue::int(observed as i64) };
            if expected == observed {
                XiaoAbiStatus::Ok.code()
            } else {
                XiaoAbiStatus::RuntimeError.code()
            }
        }

        let runtime = runtime_argument(&first).unwrap();
        let mut thread = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call_with_entry(
                operation("spawn"),
                Some(current_id),
                first_id as usize as *mut c_void,
                &first,
                1,
                &mut thread,
            ),
            XiaoAbiStatus::Ok.code()
        );
        let mut result = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(operation("join"), &thread, 1, &mut result),
            XiaoAbiStatus::Ok.code()
        );
        assert_eq!(
            unsafe { result.payload.i64_value },
            i64::try_from(runtime.id()).unwrap()
        );
        crate::abi::xiao_runtime_value_release(&mut result);
        crate::abi::xiao_runtime_value_release(&mut thread);
        crate::abi::xiao_runtime_value_release(&mut first);
        crate::abi::xiao_runtime_value_release(&mut second);
    }

    #[test]
    fn trampoline_error_identity_returns_to_joining_thread() {
        unsafe extern "C" fn entry(
            _context: *mut c_void,
            _arguments: *const XiaoValue,
            _argument_count: usize,
            _out: *mut XiaoValue,
        ) -> i32 {
            set_concurrency_error(crate::CONCURRENCY_CANCELLED_CODE, "cancelled in child")
        }

        let mut runtime = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(
                operation("runtime_create"),
                std::ptr::null(),
                0,
                &mut runtime
            ),
            XiaoAbiStatus::Ok.code()
        );
        let mut thread = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call_with_entry(
                operation("spawn"),
                Some(entry),
                std::ptr::null_mut(),
                &runtime,
                1,
                &mut thread,
            ),
            XiaoAbiStatus::Ok.code()
        );
        let mut result = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(operation("join"), &thread, 1, &mut result),
            XiaoAbiStatus::RuntimeError.code()
        );
        let mut snapshot = xiao_runtime_abi::XiaoAbiErrorSnapshot {
            class: xiao_runtime_abi::XiaoErrorClass::None,
            kind: 0,
            error_id: 0,
            code: XiaoAbiBytes {
                ptr: std::ptr::null(),
                len: 0,
            },
            message_id: XiaoAbiBytes {
                ptr: std::ptr::null(),
                len: 0,
            },
            location: xiao_runtime_abi::XiaoAbiErrorLocation::none(),
            exit_code: 0,
            stack_depth: 0,
            param_count: 0,
        };
        assert_eq!(
            crate::abi::xiao_runtime_error_snapshot(&mut snapshot),
            XiaoAbiStatus::Ok.code()
        );
        let code = unsafe { std::slice::from_raw_parts(snapshot.code.ptr, snapshot.code.len) };
        assert_eq!(code, crate::CONCURRENCY_CANCELLED_CODE.as_bytes());
        let catch_type = XiaoAbiBytes {
            ptr: b"Error".as_ptr(),
            len: b"Error".len(),
        };
        assert_eq!(crate::abi::xiao_runtime_error_matches(catch_type), 0);
        crate::abi::xiao_runtime_error_clear();
        crate::abi::xiao_runtime_value_release(&mut result);
        crate::abi::xiao_runtime_value_release(&mut thread);
        crate::abi::xiao_runtime_value_release(&mut runtime);
    }

    #[test]
    fn detached_trampoline_failure_keeps_error_identity_and_runtime_id() {
        unsafe extern "C" fn entry(
            _context: *mut c_void,
            _arguments: *const XiaoValue,
            _argument_count: usize,
            _out: *mut XiaoValue,
        ) -> i32 {
            set_concurrency_error(crate::CONCURRENCY_DETACHED_FAILURE_CODE, "detached failure")
        }

        let mut runtime = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(
                operation("runtime_create"),
                std::ptr::null(),
                0,
                &mut runtime,
            ),
            XiaoAbiStatus::Ok.code()
        );
        let runtime_id = runtime_argument(&runtime).unwrap().id();
        let mut thread = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call_with_entry(
                operation("spawn"),
                Some(entry),
                std::ptr::null_mut(),
                &runtime,
                1,
                &mut thread,
            ),
            XiaoAbiStatus::Ok.code()
        );
        let mut none = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(operation("detach"), &thread, 1, &mut none),
            XiaoAbiStatus::Ok.code()
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let mut observed_runtime = 0_u64;
            let mut error = XiaoValue::none();
            let status = xiao_runtime_concurrency_detached_error_take(
                &runtime_id,
                1,
                &mut observed_runtime,
                &mut error,
            );
            if status == XiaoAbiStatus::Ok.code() {
                assert_eq!(observed_runtime, runtime_id);
                assert_eq!(error.tag, XiaoValueTag::Error);
                assert_eq!(
                    unsafe { super::super::clone_error(error.payload.handle) }
                        .expect("detached error handle")
                        .code(),
                    crate::CONCURRENCY_DETACHED_FAILURE_CODE
                );
                crate::abi::xiao_runtime_value_release(&mut error);
                break;
            }
            assert_eq!(status, XiaoAbiStatus::OutOfBounds.code());
            assert!(
                std::time::Instant::now() < deadline,
                "detached 回调必须完成"
            );
            std::thread::yield_now();
        }
        crate::abi::xiao_runtime_value_release(&mut none);
        crate::abi::xiao_runtime_value_release(&mut thread);
        crate::abi::xiao_runtime_value_release(&mut runtime);
    }
}
