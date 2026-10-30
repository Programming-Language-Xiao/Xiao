//! 原生并发值的 C ABI 适配。
//!
//! 这里的盒子只保存 Runtime 的公开句柄类型或显式共享句柄；LLVM 看到的始终是
//! `XiaoValueTag::Concurrency` 加不透明指针，不能依赖 `RuntimeHandle`、任务池或
//! 共享对象的 Rust 布局。

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};

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
            super::set_pending_runtime_error(error);
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
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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

/// 原生边界只传 XCON 数据和显式 shared 句柄；普通 ABI 盒子不会跨线程移动。
#[derive(Clone, Debug)]
struct NativeRawValue {
    transfer: VmTransferValue,
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
struct NativeThreadState {
    handle: Mutex<Option<ThreadHandle<NativeEntryOutcome>>>,
    cancellation: RuntimeCancellation,
    runtime: RuntimeHandle,
    locale: String,
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
struct NativeTaskState {
    handle: Mutex<Option<TaskHandle<NativeEntryOutcome>>>,
    cancellation: RuntimeCancellation,
    runtime: RuntimeHandle,
    locale: String,
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
enum NativeEntryOutcome {
    Value(NativeRawValue),
    Error(NativeErrorPacket),
    RuntimeFailure(RuntimeExecError),
}

impl NativeEntryOutcome {
    /// 只转换 Runtime 内部结果类型，不执行另一个 VM 或携带 VM 栈。
    fn into_execution_result(self) -> crate::VmExecutionResult {
        match self {
            Self::Value(value) => Ok(value.transfer),
            Self::Error(NativeErrorPacket::Recoverable(error))
                if error.code() == crate::CONCURRENCY_CANCELLED_CODE =>
            {
                Err(crate::VmExecutionError::ConcurrencyCancelled(error))
            }
            Self::Error(NativeErrorPacket::Recoverable(error)) => {
                Err(crate::VmExecutionError::Error(error))
            }
            Self::Error(NativeErrorPacket::Fatal(error)) => {
                Err(crate::VmExecutionError::Fatal(error))
            }
            Self::RuntimeFailure(error) => Err(crate::VmExecutionError::Error(
                crate::execution_error(error),
            )),
        }
    }

    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    fn from_execution_result(result: crate::VmExecutionResult) -> Self {
        match result {
            Ok(transfer) => Self::Value(NativeRawValue { transfer }),
            Err(
                crate::VmExecutionError::Error(error)
                | crate::VmExecutionError::ConcurrencyCancelled(error),
            ) => Self::Error(NativeErrorPacket::Recoverable(error)),
            Err(crate::VmExecutionError::Fatal(error)) => {
                Self::Error(NativeErrorPacket::Fatal(error))
            }
            Err(crate::VmExecutionError::Cancelled) => {
                Self::RuntimeFailure(RuntimeExecError::Cancelled)
            }
        }
    }
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
struct NativeDetachedError {
    runtime_id: u64,
    locale: String,
    packet: NativeErrorPacket,
}

fn native_detached_callbacks() -> &'static (Mutex<std::collections::HashMap<u64, usize>>, Condvar) {
    static CALLBACKS: OnceLock<(Mutex<std::collections::HashMap<u64, usize>>, Condvar)> =
        OnceLock::new();
    CALLBACKS.get_or_init(|| (Mutex::new(std::collections::HashMap::new()), Condvar::new()))
}

struct NativeDetachedCallbackGuard {
    runtime_id: u64,
}

impl NativeDetachedCallbackGuard {
    fn register(runtime_id: u64) -> Self {
        let mut callbacks = native_detached_callbacks()
            .0
            .lock()
            .expect("native detached callback 锁未中毒");
        *callbacks.entry(runtime_id).or_default() += 1;
        Self { runtime_id }
    }
}

impl Drop for NativeDetachedCallbackGuard {
    fn drop(&mut self) {
        let (callbacks, drained) = native_detached_callbacks();
        if let Ok(mut callbacks) = callbacks.lock() {
            if let Some(count) = callbacks.get_mut(&self.runtime_id) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    callbacks.remove(&self.runtime_id);
                }
            }
            drained.notify_all();
        }
    }
}

fn wait_native_detached_callbacks(runtime_ids: &[u64]) {
    let (callbacks, drained) = native_detached_callbacks();
    let Ok(mut callbacks) = callbacks.lock() else {
        return;
    };
    while runtime_ids
        .iter()
        .any(|runtime_id| callbacks.get(runtime_id).is_some_and(|count| *count != 0))
    {
        let Ok(guard) = drained.wait(callbacks) else {
            return;
        };
        callbacks = guard;
    }
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
const NATIVE_DETACHED_ERROR_CAPACITY: usize = 4096;
/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
static NATIVE_DETACHED_ERROR_DROPS: AtomicUsize = AtomicUsize::new(0);
static NATIVE_ENTRY_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn native_detached_errors() -> &'static Mutex<Vec<NativeDetachedError>> {
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    static ERRORS: OnceLock<Mutex<Vec<NativeDetachedError>>> = OnceLock::new();
    ERRORS.get_or_init(|| Mutex::new(Vec::new()))
}

/// 当前 native 入口创建的直接子 Runtime；入口收尾按逆序 drain。
fn native_owned_runtimes() -> &'static Mutex<Vec<(RuntimeHandle, RuntimeHandle)>> {
    static RUNTIMES: OnceLock<Mutex<Vec<(RuntimeHandle, RuntimeHandle)>>> = OnceLock::new();
    RUNTIMES.get_or_init(Default::default)
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn record_native_detached(
    runtime_id: u64,
    locale: &str,
    result: Result<NativeEntryOutcome, RuntimeExecError>,
) {
    if let Some(error) = crate::vm_concurrency::detached_execution_error(
        result.map(NativeEntryOutcome::into_execution_result),
    ) {
        // 先缓存在 Runtime 专属有界通道；入口 finish drain 后以创建时 locale
        // 输出，避免 detached worker 的线程局部 locale 回退为默认值。
        push_native_detached_error(
            runtime_id,
            locale.to_owned(),
            NativeErrorPacket::Recoverable(error),
        );
    }
}

fn native_detached_callback(
    runtime_id: u64,
    locale: String,
) -> impl FnOnce(Result<NativeEntryOutcome, RuntimeExecError>) + Send + 'static {
    let activity = NativeDetachedCallbackGuard::register(runtime_id);
    move |result| {
        let _activity = activity;
        record_native_detached(runtime_id, &locale, result);
    }
}

/// 释放一个并发盒子时记录与普通 Runtime 对象相同的强引用事件。
fn trace_concurrency_release(handle: XiaoConcurrencyHandle, kind: XiaoConcurrencyKind) {
    debug_assert_ne!(kind.raw(), 0);
    crate::memory::record_external_release(
        handle as usize,
        crate::memory::ReleaseAction::ConcurrencyRelease,
    );
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn push_native_detached_error(runtime_id: u64, locale: String, packet: NativeErrorPacket) {
    let Ok(mut errors) = native_detached_errors().lock() else {
        NATIVE_DETACHED_ERROR_DROPS.fetch_add(1, Ordering::AcqRel);
        return;
    };
    if errors.len() >= NATIVE_DETACHED_ERROR_CAPACITY {
        NATIVE_DETACHED_ERROR_DROPS.fetch_add(1, Ordering::AcqRel);
        return;
    }
    errors.push(NativeDetachedError {
        runtime_id,
        locale,
        packet,
    });
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn take_native_detached_error_for(runtime_ids: &[u64]) -> Option<NativeDetachedError> {
    let Ok(mut errors) = native_detached_errors().lock() else {
        return None;
    };
    let index = errors
        .iter()
        .position(|error| runtime_ids.contains(&error.runtime_id))?;
    Some(errors.remove(index))
}

impl Drop for NativeThreadState {
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    fn drop(&mut self) {
        let Ok(mut handle) = self.handle.lock() else {
            return;
        };
        if let Some(handle) = handle.take() {
            let runtime_id = self.runtime.id();
            handle.detach_with(native_detached_callback(runtime_id, self.locale.clone()));
        }
    }
}

impl Drop for NativeTaskState {
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    fn drop(&mut self) {
        let Ok(mut handle) = self.handle.lock() else {
            return;
        };
        if let Some(handle) = handle.take() {
            let runtime_id = self.runtime.id();
            handle.detach_with(native_detached_callback(runtime_id, self.locale.clone()));
        }
    }
}

/// 可复制的并发句柄盒子；每次 retain 在 `inners` 中增加一个 Runtime 对象别名。
#[repr(C)]
struct AbiConcurrency {
    magic: u64,
    kind: u32,
    _reserved: u32,
    owner_thread: std::thread::ThreadId,
    inners: Mutex<Vec<ConcurrencyObject>>,
}

// SAFETY: registry access serializes all operations. The only !Send payload is LockGuard;
// lookup rejects non-owner threads before the box contents can be cloned or released.
unsafe impl Send for AbiConcurrency {}
unsafe impl Sync for AbiConcurrency {}

struct ConcurrencyHandleRegistry {
    next_token: usize,
    objects: std::collections::HashMap<usize, Arc<AbiConcurrency>>,
}

/// 将 opaque token 查找与最后释放串行化；token 不直接编码可解引用地址。
fn concurrency_handle_registry() -> &'static Mutex<ConcurrencyHandleRegistry> {
    static REGISTRY: OnceLock<Mutex<ConcurrencyHandleRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        Mutex::new(ConcurrencyHandleRegistry {
            next_token: 16,
            objects: std::collections::HashMap::new(),
        })
    })
}

/// 在 process-wide registry 锁有效期间访问一个 live opaque handle。
struct ConcurrencyHandleAccess {
    registry: MutexGuard<'static, ConcurrencyHandleRegistry>,
    token: usize,
}

impl std::ops::Deref for ConcurrencyHandleAccess {
    type Target = AbiConcurrency;

    fn deref(&self) -> &Self::Target {
        self.registry
            .objects
            .get(&self.token)
            .expect("live concurrency token 必须仍在 registry 中")
            .as_ref()
    }
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn box_object(object: ConcurrencyObject) -> XiaoConcurrencyHandle {
    let kind = object.kind().raw();
    let object = Arc::new(AbiConcurrency {
        magic: CONCURRENCY_MAGIC,
        kind,
        _reserved: 0,
        owner_thread: std::thread::current().id(),
        inners: Mutex::new(vec![object]),
    });
    let mut registry = concurrency_handle_registry()
        .lock()
        .expect("并发 opaque registry 锁未中毒");
    let token = registry.next_token;
    registry.next_token = registry
        .next_token
        .checked_add(16)
        .expect("并发 opaque token 序列耗尽");
    registry.objects.insert(token, object);
    token as XiaoConcurrencyHandle
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn object_ref(handle: XiaoConcurrencyHandle) -> Result<ConcurrencyHandleAccess, i32> {
    if handle.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    let token = handle as usize;
    let registry = concurrency_handle_registry()
        .lock()
        .map_err(|_| XiaoAbiStatus::RuntimeError.code())?;
    let Some(object) = registry.objects.get(&token) else {
        return Err(XiaoAbiStatus::InvalidHandle.code());
    };
    if object.magic != CONCURRENCY_MAGIC || XiaoConcurrencyKind::from_raw(object.kind).is_none() {
        return Err(XiaoAbiStatus::InvalidHandle.code());
    }
    if object.kind == XiaoConcurrencyKind::LockGuard.raw()
        && object.owner_thread != std::thread::current().id()
    {
        return Err(map_exec_error(RuntimeExecError::RuntimeClosed));
    }
    Ok(ConcurrencyHandleAccess { registry, token })
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn object_kind(handle: XiaoConcurrencyHandle) -> Result<XiaoConcurrencyKind, i32> {
    let object = object_ref(handle)?;
    Ok(XiaoConcurrencyKind::from_raw(object.kind).expect("object_ref 已验证 concurrency kind"))
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn clone_object(handle: XiaoConcurrencyHandle) -> Result<ConcurrencyObject, i32> {
    let object = object_ref(handle)?;
    object
        .inners
        .lock()
        .map_err(|_| XiaoAbiStatus::RuntimeError.code())?
        .last()
        .cloned()
        .ok_or(XiaoAbiStatus::InvalidHandle.code())
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
pub(super) fn retain_handle(handle: XiaoConcurrencyHandle) -> XiaoConcurrencyHandle {
    let Ok(object) = object_ref(handle) else {
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

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
pub(super) fn release_handle(handle: XiaoConcurrencyHandle) {
    if handle.is_null() {
        return;
    }
    let Ok(mut object) = object_ref(handle) else {
        return;
    };
    let Ok(mut inners) = object.inners.lock() else {
        return;
    };
    let Some(released) = inners.pop() else {
        return;
    };
    let should_drop = inners.is_empty();
    let released_kind = released.kind();
    drop(inners);
    trace_concurrency_release(handle, released_kind);
    if should_drop {
        crate::memory::forget_external_release_object(handle as usize);
        object.registry.objects.remove(&(handle as usize));
    }
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
pub(super) fn handle_kind(handle: XiaoConcurrencyHandle) -> u32 {
    object_kind(handle).map_or(0, XiaoConcurrencyKind::raw)
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn raw_from_value(value: &XiaoValue, runtime_id: u64) -> Result<NativeRawValue, i32> {
    if value.tag == XiaoValueTag::Concurrency {
        let kind = object_kind(handle_from_value(value)?)?;
        if !matches!(
            kind,
            XiaoConcurrencyKind::Shared | XiaoConcurrencyKind::SharedMut
        ) {
            return Err(set_transfer_error(crate::transfer_error(
                "只有显式 shared/shared_mut 句柄可通过并发边界",
            )));
        }
    }
    // SAFETY: 调用方提供的 ABI 值在当前调用期间保持有效。
    let runtime_value = unsafe { super::value_to_runtime(value) }?;
    let transfer =
        crate::VmTransferValue::encode(&runtime_value, runtime_id).map_err(set_transfer_error)?;
    Ok(NativeRawValue { transfer })
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn set_transfer_error(error: crate::XiaoError) -> i32 {
    super::set_pending_runtime_error(error);
    XiaoAbiStatus::RuntimeError.code()
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn value_from_raw(value: NativeRawValue, runtime_id: u64) -> Result<XiaoValue, i32> {
    let decoded = value
        .transfer
        .decode(runtime_id)
        .map_err(set_transfer_error)?;
    super::runtime_to_value(&decoded)
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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
        let byte_len = value.transfer.byte_len();
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

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn invoke_entry(
    entry: xiao_runtime_abi::XiaoConcurrencyEntry,
    context: usize,
    arguments: Vec<NativeRawValue>,
    input_runtime_id: u64,
    output_runtime_id: u64,
) -> NativeEntryOutcome {
    let mut values = Vec::with_capacity(arguments.len());
    for argument in arguments {
        match value_from_raw(argument, input_runtime_id) {
            Ok(value) => values.push(value),
            Err(_) => {
                for value in values.iter_mut().rev() {
                    crate::abi::xiao_runtime_value_release(value);
                }
                return take_native_error_packet().map_or(
                    NativeEntryOutcome::RuntimeFailure(RuntimeExecError::JoinFailed),
                    NativeEntryOutcome::Error,
                );
            }
        }
    }
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
    for value in values.iter_mut().rev() {
        crate::abi::xiao_runtime_value_release(value);
    }
    if status != XiaoAbiStatus::Ok.code() {
        crate::abi::xiao_runtime_value_release(&mut output);
        return take_native_error_packet().map_or(
            NativeEntryOutcome::RuntimeFailure(RuntimeExecError::JoinFailed),
            NativeEntryOutcome::Error,
        );
    }
    let raw = raw_from_value(&output, output_runtime_id);
    crate::abi::xiao_runtime_value_release(&mut output);
    match raw {
        Ok(value) => NativeEntryOutcome::Value(value),
        // VM 的 spawn 通过 `entry_result.encode(return_runtime_id)` 验证子函数
        // 返回值的边界资格；此处即使编码失败也必须把结构化错误回传给 join/wait。
        Err(_) => take_native_error_packet().map_or_else(
            || NativeEntryOutcome::RuntimeFailure(RuntimeExecError::JoinFailed),
            NativeEntryOutcome::Error,
        ),
    }
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
pub(super) fn handle_from_value(value: &XiaoValue) -> Result<XiaoConcurrencyHandle, i32> {
    if value.tag != XiaoValueTag::Concurrency {
        return Err(XiaoAbiStatus::InvalidArgument.code());
    }
    let handle =
        unsafe { value.payload.handle }.cast::<xiao_runtime_abi::XiaoConcurrencyOpaqueHandle>();
    object_kind(handle).map(|_| handle)
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn runtime_argument(value: &XiaoValue) -> Result<RuntimeHandle, i32> {
    match clone_object(handle_from_value(value)?)? {
        ConcurrencyObject::Runtime(runtime) => Ok(runtime),
        _ => Err(XiaoAbiStatus::InvalidHandle.code()),
    }
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn pool_argument(value: &XiaoValue) -> Result<TaskPool, i32> {
    match clone_object(handle_from_value(value)?)? {
        ConcurrencyObject::Pool(pool) => Ok(pool),
        _ => Err(XiaoAbiStatus::InvalidHandle.code()),
    }
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn set_runtime_error(message: impl Into<String>) -> i32 {
    set_concurrency_error(crate::CONCURRENCY_RUNTIME_CLOSED_CODE, message)
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn set_concurrency_error(code: &str, message: impl Into<String>) -> i32 {
    super::set_pending_runtime_error(crate::RuntimeError::new(
        crate::XiaoErrorKind::Other,
        code,
        "runtime.concurrency",
        message,
    ));
    XiaoAbiStatus::RuntimeError.code()
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn map_exec_error(error: RuntimeExecError) -> i32 {
    super::set_pending_runtime_error(crate::execution_error(error));
    XiaoAbiStatus::RuntimeError.code()
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn bytes_to_operation(operation: XiaoAbiBytes) -> Result<String, i32> {
    // SAFETY: Runtime ABI 只在调用期间借用操作名，长度和指针由统一视图校验。
    unsafe { super::utf8(operation) }
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn expect_count(count: usize, expected: usize) -> Result<(), i32> {
    if count == expected {
        Ok(())
    } else {
        Err(XiaoAbiStatus::InvalidArgument.code())
    }
}

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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
            track_native_runtime(runtime.clone(), parent);
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
                    _ => return Err(map_exec_error(RuntimeExecError::PoolArgument)),
                };
                let capacity = usize::try_from(capacity)
                    .map_err(|_| map_exec_error(RuntimeExecError::PoolArgument))?;
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
            } else if RuntimeHandle::current().is_closed() {
                Err(map_exec_error(RuntimeExecError::RuntimeClosed))
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
                    .decode(RuntimeHandle::current().id())
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
            if shared.runtime_id() != RuntimeHandle::current().id()
                || RuntimeHandle::current().is_closed()
            {
                return Err(map_exec_error(RuntimeExecError::RuntimeClosed));
            }
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
            if guard_ref.runtime_id() != RuntimeHandle::current().id()
                || RuntimeHandle::current().is_closed()
            {
                return Err(map_exec_error(RuntimeExecError::RuntimeClosed));
            }
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
            if runtime_id != RuntimeHandle::current().id() || RuntimeHandle::current().is_closed() {
                return Err(map_exec_error(RuntimeExecError::RuntimeClosed));
            }
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
            let input_runtime_id = runtime.id();
            let output_runtime_id = RuntimeHandle::current().id();
            let values = native_arguments(&arguments[1..], input_runtime_id)?;
            let context = context as usize;
            let handle = RuntimeHandle::current()
                .in_context(|| {
                    crate::scheduler::spawn_cooperative(&runtime, move |_| {
                        invoke_entry(entry, context, values, input_runtime_id, output_runtime_id)
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
                    locale: super::current_locale(),
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
                    native_detached_callback(runtime_id, state.locale.clone()),
                )
                .map_err(map_exec_error)?;
            let result = crate::vm_concurrency::cancel_result(
                value.into_execution_result(),
                &state.cancellation,
            );
            native_outcome_to_value(
                NativeEntryOutcome::from_execution_result(result),
                RuntimeHandle::current().id(),
            )
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
                    handle.detach_with(native_detached_callback(runtime_id, state.locale.clone()));
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
                    handle.detach_with(native_detached_callback(runtime_id, state.locale.clone()));
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
            let input_runtime_id = runtime.id();
            let output_runtime_id = RuntimeHandle::current().id();
            let values = native_arguments(&arguments[1..], input_runtime_id)?;
            let context = context as usize;
            let submitting_cancellation = crate::current_cancellation_token();
            let task_cancellation = RuntimeCancellation::default();
            let task_cancellation_hook: Arc<dyn Fn() + Send + Sync> = {
                let task_cancellation = task_cancellation.clone();
                Arc::new(move || task_cancellation.cancel())
            };
            let parent_registration =
                submitting_cancellation.register_cancel_hook(&task_cancellation_hook);
            let queue_interrupted = submitting_cancellation.clone();
            let handle = RuntimeHandle::current()
                .in_context(|| {
                    pool.submit_cooperative(
                        task_cancellation,
                        move || queue_interrupted.is_cancelled(),
                        move |_| {
                            let _parent_registration = parent_registration;
                            let _parent_hook = task_cancellation_hook;
                            invoke_entry(
                                entry,
                                context,
                                values,
                                input_runtime_id,
                                output_runtime_id,
                            )
                        },
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
                    locale: super::current_locale(),
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
                .wait_interruptible_preserving_result(|| crate::cancellation_safe_point().is_err())
                .map(NativeEntryOutcome::into_execution_result);
            let result =
                crate::vm_concurrency::normalize_task_wait_result(outcome, &state.cancellation)
                    .map_err(map_exec_error)?;
            native_outcome_to_value(
                NativeEntryOutcome::from_execution_result(result),
                RuntimeHandle::current().id(),
            )
        }
        _ => Err(set_concurrency_error(
            crate::INVALID_VALUE_CODE,
            format!("未知原生并发入口 {operation}"),
        )),
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

/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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

/// 注册一个由当前 Runtime 直接拥有的 native 子 Runtime。
fn register_native_runtime(runtime: RuntimeHandle, owner: RuntimeHandle) {
    if !NATIVE_ENTRY_ACTIVE.load(Ordering::Acquire) {
        return;
    }
    native_owned_runtimes()
        .lock()
        .expect("native Runtime 所有权锁未中毒")
        .push((runtime, owner));
}

/// native 入口结束时关闭当前 Runtime，并消费已经抵达的 detached 记录。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_finish() -> i32 {
    let runtime = RuntimeHandle::current();
    let owned = std::mem::take(
        &mut *native_owned_runtimes()
            .lock()
            .expect("native Runtime 所有权锁未中毒"),
    );
    let mut runtime_ids = owned
        .iter()
        .map(|(runtime, _)| runtime.id())
        .collect::<Vec<_>>();
    let mut first_error = None;
    for (child, owner) in owned.into_iter().rev() {
        if child.is_closed() {
            continue;
        }
        match owner.in_context(|| child.try_close()) {
            Ok(Ok(())) => {}
            Ok(Err(error)) | Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    if !runtime.is_closed() {
        if let Err(error) = runtime.try_close() {
            first_error.get_or_insert(error);
        }
    }
    runtime_ids.push(runtime.id());
    wait_native_detached_callbacks(&runtime_ids);
    while let Some(detached) = take_native_detached_error_for(&runtime_ids) {
        match detached.packet {
            NativeErrorPacket::Recoverable(error) => {
                let report = error.report();
                super::write_runtime_report("recoverable", &report, detached.locale, 0);
            }
            NativeErrorPacket::Fatal(error) => {
                let report = error.report();
                super::write_runtime_report("fatal", &report, detached.locale, 0);
            }
        }
    }
    NATIVE_ENTRY_ACTIVE.store(false, Ordering::Release);
    first_error.map_or_else(|| XiaoAbiStatus::Ok.code(), map_exec_error)
}

/// 开启一个真实 native 入口的 Runtime 所有权收集窗口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_concurrency_entry_begin() {
    native_owned_runtimes()
        .lock()
        .expect("native Runtime 所有权锁未中毒")
        .clear();
    NATIVE_ENTRY_ACTIVE.store(true, Ordering::Release);
}

/// 将 runtime_create 产生的直接子 Runtime 纳入 native 入口自动收尾。
fn track_native_runtime(runtime: RuntimeHandle, owner: RuntimeHandle) {
    register_native_runtime(runtime, owner);
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
    let Some(NativeDetachedError {
        runtime_id, packet, ..
    }) = take_native_detached_error_for(ids)
    else {
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
/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
mod tests {
    use super::*;

    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    fn operation(name: &str) -> XiaoAbiBytes {
        XiaoAbiBytes {
            ptr: name.as_ptr(),
            len: name.len(),
        }
    }

    #[test]
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    fn public_release_reclaims_shared_box_and_boundaries_reject_thread_local_handles() {
        crate::abi::xiao_runtime_error_clear();
        let runtime = RuntimeHandle::current();
        let shared = SharedHandle::new(
            VmTransferValue::encode(&RuntimeValue::Int(1), runtime.id()).unwrap(),
            runtime.id(),
            SharedTag::Read,
        );
        let weak = shared.downgrade();
        let handle = box_object(ConcurrencyObject::Shared(shared));
        let mut value = value_from_handle(handle);
        let transfer = raw_from_value(&value, runtime.id()).expect("显式 shared 通过值通道");
        let mut copy = value_from_raw(transfer, runtime.id()).expect("接收线程重建自己的 ABI 盒子");
        assert_ne!(unsafe { value.payload.handle }, unsafe {
            copy.payload.handle
        });
        crate::abi::xiao_runtime_value_release(&mut value);
        assert!(weak.upgrade().is_some());
        crate::abi::xiao_runtime_value_release(&mut copy);
        assert!(weak.upgrade().is_none(), "普通值释放入口也必须回收并发盒子");

        let mut runtime_value = value_from_handle(box_object(ConcurrencyObject::Runtime(runtime)));
        assert!(raw_from_value(&runtime_value, RuntimeHandle::current().id()).is_err());
        assert!(
            matches!(take_native_error_packet(), Some(NativeErrorPacket::Recoverable(error))
            if error.code() == crate::CONCURRENCY_SERIALIZATION_CODE)
        );
        crate::abi::xiao_runtime_value_release(&mut runtime_value);
    }

    #[test]
    /// 并发 retain/release 同一 opaque alias 时，最后释放不能与句柄读取竞态。
    fn retain_release_race_is_serialized_until_box_reclamation() {
        use std::sync::Barrier;

        for _ in 0..128 {
            let first = box_object(ConcurrencyObject::Runtime(RuntimeHandle::current()));
            let racing_alias = retain_handle(first);
            assert!(!racing_alias.is_null());
            let barrier = Arc::new(Barrier::new(3));

            let retain_barrier = Arc::clone(&barrier);
            let alias = racing_alias as usize;
            let retain_thread = std::thread::spawn(move || {
                retain_barrier.wait();
                retain_handle(alias as XiaoConcurrencyHandle) as usize
            });
            let release_barrier = Arc::clone(&barrier);
            let alias = racing_alias as usize;
            let release_thread = std::thread::spawn(move || {
                release_barrier.wait();
                release_handle(alias as XiaoConcurrencyHandle);
            });
            barrier.wait();
            let retained =
                retain_thread.join().expect("retain 线程不应 panic") as XiaoConcurrencyHandle;
            release_thread.join().expect("release 线程不应 panic");
            if !retained.is_null() {
                release_handle(retained);
            }
            release_handle(first);
        }
    }

    #[test]
    /// VM 的显式释放追踪也必须看到共享句柄和并发盒子的释放动作。
    fn vm_release_trace_includes_concurrency_external_objects() {
        let _trace = crate::start_release_trace();
        let mut shared = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(
                operation("shared_read"),
                &XiaoValue::int(7),
                1,
                &mut shared
            ),
            XiaoAbiStatus::Ok.code()
        );
        crate::abi::xiao_runtime_value_release(&mut shared);
        let events = crate::take_release_events();
        assert!(
            events
                .iter()
                .any(|event| event.action == crate::memory::ReleaseAction::ConcurrencyRelease),
            "VM 释放追踪必须包含并发盒子释放"
        );
        assert!(
            events
                .iter()
                .any(|event| event.action == crate::memory::ReleaseAction::SharedRelease),
            "VM 释放追踪必须包含共享强句柄释放"
        );
    }

    #[test]
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    fn completed_child_cancellation_uses_vm_error_normalization_and_keeps_cleanup_error() {
        let cancellation = RuntimeCancellation::default();
        cancellation.cancel();
        let child_error = crate::RuntimeError::from_type_name(
            "ArithmeticError",
            Some("CHILD"),
            Some("child cleanup"),
        )
        .unwrap();
        let outcome = NativeEntryOutcome::Error(NativeErrorPacket::Recoverable(child_error));
        let result =
            crate::vm_concurrency::cancel_result(outcome.into_execution_result(), &cancellation);
        assert!(
            matches!(result, Err(crate::VmExecutionError::ConcurrencyCancelled(error))
            if error.code() == crate::CONCURRENCY_CANCELLED_CODE
                && error.suppressed().iter().any(|error| error.code() == "CHILD"))
        );
    }

    #[test]
    /// 原生 finally checkpoint 恢复主错误并保留清理错误为 suppressed。
    fn native_error_checkpoint_preserves_primary_and_suppressed() {
        crate::abi::xiao_runtime_error_clear();
        crate::abi::set_pending_runtime_error(crate::RuntimeError::new(
            crate::XiaoErrorKind::Other,
            "child-primary",
            "runtime.user_error",
            "primary",
        ));
        crate::abi::xiao_runtime_error_checkpoint_begin();
        // finally 内调用的 helper 显式返回，再继续产生清理错误。
        crate::abi::xiao_runtime_error_clear();
        crate::abi::set_pending_runtime_error(crate::RuntimeError::new(
            crate::XiaoErrorKind::Other,
            "child-cleanup",
            "runtime.user_error",
            "cleanup",
        ));
        assert_eq!(
            crate::abi::xiao_runtime_error_checkpoint_finish(),
            xiao_runtime_abi::XiaoErrorClass::Recoverable.raw()
        );
        assert!(matches!(
            take_native_error_packet(),
            Some(NativeErrorPacket::Recoverable(error))
                if error.code() == "child-primary"
                    && error.suppressed().iter().any(|cleanup| cleanup.code() == "child-cleanup")
        ));
    }

    #[test]
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    fn unsupported_operation_is_explicitly_rejected() {
        let mut output = XiaoValue::none();
        let status =
            xiao_runtime_concurrency_call(operation("spawn"), std::ptr::null(), 0, &mut output);
        assert_ne!(status, XiaoAbiStatus::Ok.code());

        crate::abi::xiao_runtime_error_clear();
        let status = xiao_runtime_concurrency_call(
            operation("future_api"),
            std::ptr::null(),
            0,
            &mut output,
        );
        assert_eq!(status, XiaoAbiStatus::RuntimeError.code());
        assert!(matches!(
            take_native_error_packet(),
            Some(NativeErrorPacket::Recoverable(error))
                if error.code() == crate::INVALID_VALUE_CODE
        ));
    }

    #[test]
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    fn native_trampoline_spawn_join_round_trips_scalar_result() {
        /// 22D 内部并发适配实现。
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
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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
    /// 关闭当前 Runtime 后，两个 shared 构造入口都返回 031。
    fn shared_creation_rejects_a_closed_current_runtime() {
        let runtime = RuntimeHandle::current()
            .in_context(RuntimeHandle::create)
            .expect("应能创建隔离测试 Runtime");
        runtime
            .in_context(|| {
                runtime.try_close().expect("当前 Runtime 可关闭");
                for operation_name in ["shared_read", "shared_mut"] {
                    crate::abi::xiao_runtime_error_clear();
                    let mut input = XiaoValue::int(7);
                    let mut output = XiaoValue::none();
                    let status = xiao_runtime_concurrency_call(
                        operation(operation_name),
                        &input,
                        1,
                        &mut output,
                    );
                    assert_eq!(status, XiaoAbiStatus::RuntimeError.code());
                    assert!(matches!(
                        take_native_error_packet(),
                        Some(NativeErrorPacket::Recoverable(error))
                            if error.code() == crate::CONCURRENCY_RUNTIME_CLOSED_CODE
                    ));
                    crate::abi::xiao_runtime_value_release(&mut input);
                    crate::abi::xiao_runtime_value_release(&mut output);
                }
            })
            .expect("关闭前可进入 Runtime 上下文");
    }

    #[test]
    /// child Runtime 返回 SharedMut 时按调用方 Runtime ID 验证并回传 031。
    fn join_rejects_shared_mut_result_owned_by_child_runtime() {
        /// 在 child Runtime 中创建并返回一个 SharedMut 句柄。
        unsafe extern "C" fn return_shared_mut(
            _context: *mut c_void,
            _arguments: *const XiaoValue,
            _argument_count: usize,
            out: *mut XiaoValue,
        ) -> i32 {
            if out.is_null() {
                return XiaoAbiStatus::Null.code();
            }
            let input = XiaoValue::int(1);
            xiao_runtime_concurrency_call(operation("shared_mut"), &input, 1, out)
        }

        crate::abi::xiao_runtime_error_clear();
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
                Some(return_shared_mut),
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
        assert!(matches!(
            take_native_error_packet(),
            Some(NativeErrorPacket::Recoverable(error))
                if error.code() == crate::CONCURRENCY_RUNTIME_CLOSED_CODE
        ));
        crate::abi::xiao_runtime_value_release(&mut result);
        crate::abi::xiao_runtime_value_release(&mut thread);
        crate::abi::xiao_runtime_value_release(&mut runtime);
    }

    #[test]
    /// wait 与 join 一样，按调用方 Runtime ID 校验子任务返回值。
    fn wait_rejects_shared_mut_result_owned_by_pool_runtime() {
        /// 在任务池 Runtime 中创建并返回一个 SharedMut 句柄。
        unsafe extern "C" fn return_shared_mut(
            _context: *mut c_void,
            _arguments: *const XiaoValue,
            _argument_count: usize,
            out: *mut XiaoValue,
        ) -> i32 {
            if out.is_null() {
                return XiaoAbiStatus::Null.code();
            }
            let input = XiaoValue::int(1);
            xiao_runtime_concurrency_call(operation("shared_mut"), &input, 1, out)
        }

        crate::abi::xiao_runtime_error_clear();
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
        let capacity = XiaoValue::int(1);
        let mut pool = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_pool_create(&runtime, &capacity, &mut pool),
            XiaoAbiStatus::Ok.code()
        );
        let mut task = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call_with_entry(
                operation("submit"),
                Some(return_shared_mut),
                std::ptr::null_mut(),
                &pool,
                1,
                &mut task,
            ),
            XiaoAbiStatus::Ok.code()
        );
        let mut result = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(operation("wait"), &task, 1, &mut result),
            XiaoAbiStatus::RuntimeError.code()
        );
        assert!(matches!(
            take_native_error_packet(),
            Some(NativeErrorPacket::Recoverable(error))
                if error.code() == crate::CONCURRENCY_RUNTIME_CLOSED_CODE
        ));
        crate::abi::xiao_runtime_value_release(&mut result);
        crate::abi::xiao_runtime_value_release(&mut task);
        crate::abi::xiao_runtime_value_release(&mut pool);
        crate::abi::xiao_runtime_value_release(&mut runtime);
    }

    #[test]
    /// LockGuard 跨 OS 线程操作时必须在 Runtime 侧返回 031。
    fn lock_guard_rejects_cross_thread_use() {
        let mut shared = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(
                operation("shared_mut"),
                &XiaoValue::int(1),
                1,
                &mut shared,
            ),
            XiaoAbiStatus::Ok.code()
        );
        let mut guard = XiaoValue::none();
        assert_eq!(
            xiao_runtime_concurrency_call(operation("lock"), &shared, 1, &mut guard),
            XiaoAbiStatus::Ok.code()
        );
        let handle = unsafe { guard.payload.handle } as usize;
        let status = std::thread::spawn(move || {
            crate::abi::xiao_runtime_error_clear();
            let foreign = XiaoValue {
                tag: XiaoValueTag::Concurrency,
                payload: xiao_runtime_abi::XiaoValuePayload {
                    handle: handle as *mut xiao_runtime_abi::XiaoOpaqueHandle,
                },
            };
            let mut output = XiaoValue::none();
            let status =
                xiao_runtime_concurrency_call(operation("lock_read"), &foreign, 1, &mut output);
            let is_031 = matches!(
                take_native_error_packet(),
                Some(NativeErrorPacket::Recoverable(error))
                    if error.code() == crate::CONCURRENCY_RUNTIME_CLOSED_CODE
            );
            (status, is_031)
        })
        .join()
        .expect("跨线程 LockGuard 负例不应 panic");
        assert_eq!(status.0, XiaoAbiStatus::RuntimeError.code());
        assert!(status.1);
        crate::abi::xiao_runtime_value_release(&mut guard);
        crate::abi::xiao_runtime_value_release(&mut shared);
    }

    #[test]
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
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

        /// 22D 内部并发适配实现。
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
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    fn trampoline_error_identity_returns_to_joining_thread() {
        /// 22D 内部并发适配实现。
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
    /// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
    fn detached_trampoline_failure_keeps_error_identity_and_runtime_id() {
        /// 22D 内部并发适配实现。
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
