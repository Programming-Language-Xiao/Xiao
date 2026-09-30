//! `xiao-runtime-abi` 的唯一符号实现。
//!
//! ABI 句柄指向本模块自己的盒子，而不是直接指向 Runtime 对象头。这样既保持对象头
//! 私有，也让 C 调用方无法依赖 Rust 的分配布局；盒子内部只保存一个已经由 06B
//! 实现验证过的 `StrongHandle` 或 `WeakHandle`。
//!
//! 这些句柄继承 06B 的单线程、非原子引用计数约束；调用方不得跨线程传递或并发调用
//! 同一个 ABI 句柄。并发/原子实现需要单独的 ABI 版本与生命周期契约。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use xiao_diagnostics::window::{
    DIAGNOSTIC_PROTOCOL_VERSION, DiagnosticEvent, DiagnosticFrameError, DiagnosticMessage,
    DiagnosticMetrics, default_level, read_message, write_message,
};
use xiao_diagnostics::{ReportRecord, render_builtin_localized_text, validated_locale_tag};
use xiao_runtime_abi::{
    ABI_MAJOR_VERSION, ABI_MINOR_VERSION, XiaoAbiBytes, XiaoAbiDiagnosticEvent,
    XiaoAbiErrorLocation, XiaoAbiErrorParam, XiaoAbiErrorSnapshot, XiaoAbiMutBytes, XiaoAbiSpan,
    XiaoAbiStackFrame, XiaoAbiStatus, XiaoErrorClass, XiaoErrorParamKind, XiaoFieldType,
    XiaoHandle, XiaoOpaqueHandle, XiaoOpaqueWeakHandle, XiaoTableDescriptor,
    XiaoTableFieldDescriptor, XiaoValue, XiaoValuePayload, XiaoValueTag, XiaoWeakHandle,
};
use xiao_source::SourceSpan;
use xiao_syntax::TableKind;
use xiao_types::{TableMemberSignature, TableSignature, Type, Visibility};

use crate::containers::{ArrayHandle, DictHandle, DictKind, SetHandle, TupleHandle};
use crate::errors::{
    CONTAINER_INDEX_CODE, CONTAINER_KEY_CODE, DiagnosticParam, FatalError, FatalKind, FrameKind,
    INVALID_HANDLE_CODE, RuntimeError, RuntimeResult, USE_AFTER_RELEASE_CODE, WEAK_UPGRADE_CODE,
    XiaoErrorKind,
};
use crate::memory::{RuntimeTypeTag, StrongHandle, WeakHandle};
use crate::tables::{TableDefinition, TableInstance};
use crate::value::{RuntimeValue, StringHandle};

/// 当前线程尚未交给原生控制流消费的错误。
enum PendingError {
    /// 可由 Xiao `catch` 消费的错误。
    Recoverable(RuntimeError),
    /// 不得进入 Xiao `catch` 的致命故障。
    Fatal(FatalError),
}

/// 原生调试产物与独立诊断进程之间的单进程会话。
struct RuntimeDiagnosticSession {
    listener: Option<TcpListener>,
    stream: Option<TcpStream>,
    token: String,
    started: Instant,
    metrics: DiagnosticMetrics,
}

/// 原生调试会话只允许有一个拥有者；普通产物不会创建它。
fn diagnostic_session() -> &'static Mutex<Option<RuntimeDiagnosticSession>> {
    static SESSION: OnceLock<Mutex<Option<RuntimeDiagnosticSession>>> = OnceLock::new();
    SESSION.get_or_init(|| Mutex::new(None))
}

/// 原生启动桥和 Runtime 之间约定的失败退出码。
const DIAGNOSTIC_START_EXIT_CODE: i32 = 70;

/// 生成不会依赖目录、本地化或外部进程的会话令牌。
fn diagnostic_session_token() -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(1);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("xiao-native-{nanos:032x}-{sequence:016x}")
}

/// 输出启动阶段的机器可见失败，不把窗口失败降级成普通运行。
fn diagnostic_start_failure(message: impl std::fmt::Display) -> i32 {
    eprintln!("X11-DIAGNOSTIC-START-001: {message}");
    DIAGNOSTIC_START_EXIT_CODE
}

/// 向已握手的诊断会话发送一帧；通信中断只关闭输出，不污染用户错误槽。
fn send_diagnostic_message(
    session: &mut RuntimeDiagnosticSession,
    message: &DiagnosticMessage,
) -> Result<(), DiagnosticFrameError> {
    let Some(stream) = session.stream.as_mut() else {
        return Err(DiagnosticFrameError::Json("诊断会话尚未握手".to_owned()));
    };
    if let Err(error) = write_message(stream, message) {
        session.stream = None;
        return Err(error);
    }
    Ok(())
}

thread_local! {
    /// N0-C 原生错误的单线程传播槽；它不跨线程，也不替代 Runtime 的错误对象。
    static PENDING_ERROR: RefCell<Option<PendingError>> = const { RefCell::new(None) };
    /// 当前原生入口的不可变语言上下文。
    static CURRENT_LOCALE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// ABI 盒子的魔数；用于在仍可读取的盒子中拒绝明显类型错配。
///
/// 入口先读取候选地址再比较魔数，因此它不是任意外部指针或释放后悬空指针的安全
/// 探测器；调用方仍必须遵守“只传 Runtime 返回且尚未归还的 live 句柄”契约。
const ABI_HANDLE_MAGIC: u64 = 0x5849_414F_4142_4931;
/// ABI 强句柄盒子的种类标记。
const ABI_KIND_STRONG: u32 = 1;
/// ABI 弱句柄盒子的种类标记。
const ABI_KIND_WEAK: u32 = 2;
/// ABI 错误句柄盒子的种类标记。
const ABI_KIND_ERROR: u32 = 3;

/// ABI 强句柄的内部盒子；其地址就是 C 侧不透明句柄地址。
///
/// ABI 的 retain 契约要求返回同一地址，因此盒子保存每一次 retain 对应的 Runtime
/// 强句柄。最后一次 ABI release 才回收盒子本身。
#[repr(C)]
struct AbiStrong {
    magic: u64,
    kind: u32,
    _reserved: u32,
    inners: RefCell<Vec<StrongHandle>>,
}

/// ABI 弱句柄的内部盒子；它不拥有目标载荷。
#[repr(C)]
struct AbiWeak {
    magic: u64,
    kind: u32,
    _reserved: u32,
    inners: RefCell<Vec<WeakHandle>>,
}

/// ABI 错误句柄盒子；错误对象不参与 Runtime 对象头的强/弱引用计数。
#[repr(C)]
struct AbiError {
    magic: u64,
    kind: u32,
    _reserved: u32,
    inners: RefCell<Vec<RuntimeError>>,
}

/// 将错误放入当前线程传播槽。
fn set_pending_error(error: PendingError) {
    PENDING_ERROR.with(|pending| {
        *pending.borrow_mut() = Some(error);
    });
}

/// 将可恢复错误放入当前线程传播槽。
fn set_pending_runtime_error(error: RuntimeError) {
    PENDING_ERROR.with(|pending| {
        let mut pending = pending.borrow_mut();
        match pending.as_mut() {
            None => *pending = Some(PendingError::Recoverable(error)),
            Some(PendingError::Recoverable(primary)) => primary.push_suppressed(error),
            Some(PendingError::Fatal(_)) => {}
        }
    });
}

/// 返回当前线程的错误类别。
fn pending_class() -> XiaoErrorClass {
    PENDING_ERROR.with(|pending| match pending.borrow().as_ref() {
        None => XiaoErrorClass::None,
        Some(PendingError::Recoverable(_)) => XiaoErrorClass::Recoverable,
        Some(PendingError::Fatal(_)) => XiaoErrorClass::Fatal,
    })
}

/// 把 ABI 的源码位置转换为 Runtime 源码位置。
fn source_span(location: XiaoAbiErrorLocation) -> Option<SourceSpan> {
    (location.present != 0)
        .then(|| SourceSpan::new(location.span.start as usize, location.span.end as usize))
        .flatten()
}

/// 把 Runtime 源码位置转换为 ABI 固定布局。
fn abi_location(location: Option<SourceSpan>) -> XiaoAbiErrorLocation {
    location.map_or_else(XiaoAbiErrorLocation::none, |span| {
        XiaoAbiErrorLocation::from_span(XiaoAbiSpan {
            start: span.start() as u64,
            end: span.end() as u64,
        })
    })
}

/// 把字符串借用为 ABI 字节视图。
fn abi_bytes(value: &str) -> XiaoAbiBytes {
    XiaoAbiBytes {
        ptr: value.as_ptr(),
        len: value.len(),
    }
}

/// 把 Runtime 错误类别编码为稳定 ABI 数值。
fn error_kind_code(kind: XiaoErrorKind) -> u32 {
    match kind {
        XiaoErrorKind::Memory => 1,
        XiaoErrorKind::Type => 2,
        XiaoErrorKind::Arithmetic => 3,
        XiaoErrorKind::Table => 4,
        XiaoErrorKind::Concurrency => 5,
        XiaoErrorKind::Resource => 6,
        XiaoErrorKind::Other => 7,
    }
}

/// 将当前线程状态的失败码映射到稳定宿主退出码。
fn pending_exit_code() -> i32 {
    match pending_class() {
        XiaoErrorClass::None => 0,
        XiaoErrorClass::Recoverable => 3,
        XiaoErrorClass::Fatal => 4,
        _ => 4,
    }
}

/// 把 Runtime 错误映射到 ABI 稳定状态码。
fn status_from_error(error: &RuntimeError) -> i32 {
    let status = match error.code() {
        INVALID_HANDLE_CODE | USE_AFTER_RELEASE_CODE | WEAK_UPGRADE_CODE => {
            XiaoAbiStatus::InvalidHandle.code()
        }
        CONTAINER_INDEX_CODE | CONTAINER_KEY_CODE => XiaoAbiStatus::OutOfBounds.code(),
        _ => XiaoAbiStatus::RuntimeError.code(),
    };
    set_pending_runtime_error(error.clone());
    status
}

/// 将任意 Runtime 结果映射为 ABI 状态码。
fn status<T>(result: RuntimeResult<T>) -> Result<T, i32> {
    result.map_err(|error| status_from_error(&error))
}

/// 从 ABI 字节视图借用一个切片；零长度视图允许空指针。
/// 从不拥有内存的字节视图借用切片，供 ABI 字符串/键入口统一校验指针和长度。
unsafe fn bytes<'a>(view: XiaoAbiBytes) -> Result<&'a [u8], i32> {
    if view.len == 0 {
        return Ok(&[]);
    }
    if view.ptr.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    Ok(unsafe { slice::from_raw_parts(view.ptr, view.len) })
}

/// 从 ABI 可变字节视图借用一个输出切片；零容量允许空指针。
unsafe fn mut_bytes<'a>(view: XiaoAbiMutBytes) -> Result<&'a mut [u8], i32> {
    if view.capacity == 0 {
        return Ok(&mut []);
    }
    if view.ptr.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    Ok(unsafe { slice::from_raw_parts_mut(view.ptr, view.capacity) })
}

/// 把一个 Runtime 强句柄装进 ABI 盒子。
fn box_strong(inner: StrongHandle) -> XiaoHandle {
    Box::into_raw(Box::new(AbiStrong {
        magic: ABI_HANDLE_MAGIC,
        kind: ABI_KIND_STRONG,
        _reserved: 0,
        inners: RefCell::new(vec![inner]),
    }))
    .cast::<XiaoOpaqueHandle>()
}

/// 把一个 Runtime 弱句柄装进 ABI 盒子。
fn box_weak(inner: WeakHandle) -> XiaoWeakHandle {
    Box::into_raw(Box::new(AbiWeak {
        magic: ABI_HANDLE_MAGIC,
        kind: ABI_KIND_WEAK,
        _reserved: 0,
        inners: RefCell::new(vec![inner]),
    }))
    .cast::<XiaoOpaqueWeakHandle>()
}

/// 把一个可恢复错误装进 ABI 错误盒子。
fn box_error(inner: RuntimeError) -> XiaoHandle {
    Box::into_raw(Box::new(AbiError {
        magic: ABI_HANDLE_MAGIC,
        kind: ABI_KIND_ERROR,
        _reserved: 0,
        inners: RefCell::new(vec![inner]),
    }))
    .cast::<XiaoOpaqueHandle>()
}

/// 借用 ABI 强句柄盒子；调用方必须传入 Runtime 返回且尚未 release 的有效地址。
///
/// 魔数只能拦截仍可读取的明显类型错误；释放后的悬空裸指针不属于 ABI 合法输入。
unsafe fn strong_ref<'a>(handle: XiaoHandle) -> Result<&'a AbiStrong, i32> {
    if handle.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    let strong = unsafe { &*handle.cast::<AbiStrong>() };
    if strong.magic != ABI_HANDLE_MAGIC || strong.kind != ABI_KIND_STRONG {
        return Err(XiaoAbiStatus::InvalidHandle.code());
    }
    Ok(strong)
}

/// 借用 ABI 弱句柄盒子；调用方必须传入 Runtime 返回且尚未 weak_release 的有效地址。
///
/// 魔数只能拦截仍可读取的明显类型错误；释放后的悬空裸指针不属于 ABI 合法输入。
unsafe fn weak_ref<'a>(handle: XiaoWeakHandle) -> Result<&'a AbiWeak, i32> {
    if handle.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    let weak = unsafe { &*handle.cast::<AbiWeak>() };
    if weak.magic != ABI_HANDLE_MAGIC || weak.kind != ABI_KIND_WEAK {
        return Err(XiaoAbiStatus::InvalidHandle.code());
    }
    Ok(weak)
}

/// 借用 ABI 错误盒子；调用方必须传入 Runtime 返回且尚未归还的错误句柄。
unsafe fn error_ref<'a>(handle: XiaoHandle) -> Result<&'a AbiError, i32> {
    if handle.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    let error = unsafe { &*handle.cast::<AbiError>() };
    if error.magic != ABI_HANDLE_MAGIC || error.kind != ABI_KIND_ERROR {
        return Err(XiaoAbiStatus::InvalidHandle.code());
    }
    Ok(error)
}

/// 克隆 ABI 错误盒子中的一个错误对象。
unsafe fn clone_error(handle: XiaoHandle) -> Result<RuntimeError, i32> {
    let error = unsafe { error_ref(handle) }?;
    error
        .inners
        .borrow()
        .last()
        .cloned()
        .ok_or(XiaoAbiStatus::InvalidHandle.code())
}

/// 增加 ABI 错误盒子的一份拥有引用。
unsafe fn retain_error(handle: XiaoHandle) -> Result<XiaoHandle, i32> {
    let error = unsafe { error_ref(handle) }?;
    let clone = error
        .inners
        .borrow()
        .last()
        .cloned()
        .ok_or(XiaoAbiStatus::InvalidHandle.code())?;
    error.inners.borrow_mut().push(clone);
    Ok(handle)
}

/// 释放 ABI 错误盒子的一份拥有引用。
unsafe fn release_error(handle: XiaoHandle) {
    if handle.is_null() {
        return;
    }
    let should_drop = {
        let Ok(error) = (unsafe { error_ref(handle) }) else {
            return;
        };
        let mut inners = error.inners.borrow_mut();
        if inners.len() > 1 {
            let _ = inners.pop();
            false
        } else if inners.len() == 1 {
            let _ = inners.pop();
            true
        } else {
            false
        }
    };
    if should_drop {
        unsafe { drop(Box::from_raw(handle.cast::<AbiError>())) };
    }
}

/// 克隆 ABI 强句柄内部计数并返回新的 Runtime 句柄。
unsafe fn clone_strong(handle: XiaoHandle) -> Result<StrongHandle, i32> {
    let strong = unsafe { strong_ref(handle) }?;
    let inner = strong
        .inners
        .borrow()
        .last()
        .ok_or(XiaoAbiStatus::InvalidHandle.code())?
        .try_clone();
    status(inner)
}

/// 检查强句柄的对象标签。
unsafe fn expect_strong(handle: XiaoHandle, expected: RuntimeTypeTag) -> Result<StrongHandle, i32> {
    let strong = unsafe { clone_strong(handle) }?;
    if strong.type_tag() != expected {
        return Err(XiaoAbiStatus::InvalidHandle.code());
    }
    Ok(strong)
}

/// 消费 ABI 强句柄盒子中的最后一份强引用，不增加 Runtime 引用计数。
///
/// 该入口供原生降低器把 `string_new`/容器构造返回的拥有句柄直接转移进
/// `XiaoValue`；它避免“先复制值、再释放原句柄”在 VM/原生差分中形成额外
/// 的运行期释放事件。若盒子中还有 `retain` 产生的其他引用，只移出最新一份，
/// 盒子本身继续由剩余引用持有。
unsafe fn take_strong(handle: XiaoHandle, expected: RuntimeTypeTag) -> Result<StrongHandle, i32> {
    let strong_ptr = handle.cast::<AbiStrong>();
    let (inner, empty) = {
        let strong = unsafe { strong_ref(handle) }?;
        let mut inners = strong.inners.borrow_mut();
        let current = inners.last().ok_or(XiaoAbiStatus::InvalidHandle.code())?;
        if current.type_tag() != expected {
            return Err(XiaoAbiStatus::InvalidHandle.code());
        }
        let inner = inners.pop().ok_or(XiaoAbiStatus::InvalidHandle.code())?;
        (inner, inners.is_empty())
    };
    if empty {
        unsafe { drop(Box::from_raw(strong_ptr)) };
    }
    Ok(inner)
}

/// 把 ABI 字节视图解码为拥有的 UTF-8 字符串。
unsafe fn utf8(view: XiaoAbiBytes) -> Result<String, i32> {
    let bytes = unsafe { bytes(view) }?;
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| XiaoAbiStatus::InvalidUtf8.code())
}

/// 把 ABI 值借用转换为 Runtime 值；其中的句柄会各自增加一次强引用。
unsafe fn value_to_runtime(value: &XiaoValue) -> Result<RuntimeValue, i32> {
    if !value.tag.is_known() {
        return Err(XiaoAbiStatus::InvalidArgument.code());
    }
    let payload = value.payload;
    match value.tag {
        XiaoValueTag::None => Ok(RuntimeValue::None),
        XiaoValueTag::Bool => {
            let raw = unsafe { payload.bool_value };
            match raw {
                0 => Ok(RuntimeValue::Bool(false)),
                1 => Ok(RuntimeValue::Bool(true)),
                _ => Err(XiaoAbiStatus::InvalidArgument.code()),
            }
        }
        XiaoValueTag::Int => Ok(RuntimeValue::Int(unsafe { payload.i64_value })),
        XiaoValueTag::Sint => Ok(RuntimeValue::Sint(unsafe { payload.i32_value })),
        XiaoValueTag::Float => Ok(RuntimeValue::Float(unsafe { payload.f64_value })),
        XiaoValueTag::Sfloat => Ok(RuntimeValue::Sfloat(unsafe { payload.f32_value })),
        XiaoValueTag::Lint | XiaoValueTag::Lfloat | XiaoValueTag::Str => {
            let handle = unsafe { expect_strong(payload.handle, RuntimeTypeTag::String) }?;
            let string = status(StringHandle::from_strong_handle(handle))?;
            let text = status(string.to_string())?;
            match value.tag {
                XiaoValueTag::Lint => Ok(RuntimeValue::Lint(text)),
                XiaoValueTag::Lfloat => Ok(RuntimeValue::Lfloat(text)),
                XiaoValueTag::Str => Ok(RuntimeValue::Str(string)),
                _ => unreachable!(),
            }
        }
        XiaoValueTag::Table => {
            let handle = unsafe { expect_strong(payload.handle, RuntimeTypeTag::Table) }?;
            Ok(RuntimeValue::Table(status(
                TableInstance::from_strong_handle(handle),
            )?))
        }
        XiaoValueTag::Array => {
            let handle = unsafe { expect_strong(payload.handle, RuntimeTypeTag::Array) }?;
            Ok(RuntimeValue::Array(status(
                ArrayHandle::from_strong_handle(handle),
            )?))
        }
        XiaoValueTag::Tuple => {
            let handle = unsafe { expect_strong(payload.handle, RuntimeTypeTag::Tuple) }?;
            Ok(RuntimeValue::Tuple(status(
                TupleHandle::from_strong_handle(handle),
            )?))
        }
        XiaoValueTag::DictTable | XiaoValueTag::DictColumn => {
            let expected = if value.tag == XiaoValueTag::DictTable {
                RuntimeTypeTag::DictTable
            } else {
                RuntimeTypeTag::DictColumn
            };
            let handle = unsafe { expect_strong(payload.handle, expected) }?;
            let dictionary = status(DictHandle::from_strong_handle(handle))?;
            if (value.tag == XiaoValueTag::DictTable && dictionary.kind() != DictKind::Table)
                || (value.tag == XiaoValueTag::DictColumn && dictionary.kind() != DictKind::Column)
            {
                return Err(XiaoAbiStatus::InvalidHandle.code());
            }
            Ok(if value.tag == XiaoValueTag::DictTable {
                RuntimeValue::DictTable(dictionary)
            } else {
                RuntimeValue::DictColumn(dictionary)
            })
        }
        XiaoValueTag::Set => {
            let handle = unsafe { expect_strong(payload.handle, RuntimeTypeTag::Set) }?;
            Ok(RuntimeValue::Set(status(SetHandle::from_strong_handle(
                handle,
            ))?))
        }
        XiaoValueTag::Error => Ok(RuntimeValue::error(unsafe { clone_error(payload.handle) }?)),
        XiaoValueTag::TableDropView => Err(XiaoAbiStatus::InvalidArgument.code()),
        _ => Err(XiaoAbiStatus::InvalidArgument.code()),
    }
}

/// 把 Runtime 值转换为拥有句柄的 ABI 值。
fn runtime_to_value(value: &RuntimeValue) -> Result<XiaoValue, i32> {
    match value {
        RuntimeValue::None => Ok(XiaoValue::none()),
        RuntimeValue::Bool(value) => Ok(XiaoValue::bool(*value)),
        RuntimeValue::Int(value) => Ok(XiaoValue::int(*value)),
        RuntimeValue::Sint(value) => Ok(XiaoValue::sint(*value)),
        RuntimeValue::Float(value) => Ok(XiaoValue::float(*value)),
        RuntimeValue::Sfloat(value) => Ok(XiaoValue::sfloat(*value)),
        RuntimeValue::Lint(value) => {
            let handle = status(StringHandle::new(value.clone()))?;
            Ok(value_from_owned_handle(
                XiaoValueTag::Lint,
                handle.into_strong_handle(),
            ))
        }
        RuntimeValue::Lfloat(value) => {
            let handle = status(StringHandle::new(value.clone()))?;
            Ok(value_from_owned_handle(
                XiaoValueTag::Lfloat,
                handle.into_strong_handle(),
            ))
        }
        RuntimeValue::Str(value) => Ok(value_from_owned_handle(
            XiaoValueTag::Str,
            value.clone().into_strong_handle(),
        )),
        RuntimeValue::Table(value) => Ok(value_from_owned_handle(
            XiaoValueTag::Table,
            value.clone().into_strong_handle(),
        )),
        RuntimeValue::Array(value) => Ok(value_from_owned_handle(
            XiaoValueTag::Array,
            value.clone().into_strong_handle(),
        )),
        RuntimeValue::Tuple(value) => Ok(value_from_owned_handle(
            XiaoValueTag::Tuple,
            value.clone().into_strong_handle(),
        )),
        RuntimeValue::DictTable(value) => Ok(value_from_owned_handle(
            XiaoValueTag::DictTable,
            value.clone().into_strong_handle(),
        )),
        RuntimeValue::DictColumn(value) => Ok(value_from_owned_handle(
            XiaoValueTag::DictColumn,
            value.clone().into_strong_handle(),
        )),
        RuntimeValue::Set(value) => Ok(value_from_owned_handle(
            XiaoValueTag::Set,
            value.clone().into_strong_handle(),
        )),
        RuntimeValue::Error(error) => Ok(value_from_owned_error((**error).clone())),
        RuntimeValue::TableDropView(_)
        | RuntimeValue::Module(_)
        | RuntimeValue::ModuleFunction(_, _) => Err(XiaoAbiStatus::InvalidArgument.code()),
    }
}

/// 用已经拥有的强句柄构造 ABI 值；所有权转移到 ABI 盒子。
fn value_from_owned_handle(tag: XiaoValueTag, handle: StrongHandle) -> XiaoValue {
    XiaoValue {
        tag,
        payload: XiaoValuePayload {
            handle: box_strong(handle),
        },
    }
}

/// 用已经拥有的可恢复错误构造 ABI 值；所有权转移到错误盒子。
fn value_from_owned_error(error: RuntimeError) -> XiaoValue {
    XiaoValue {
        tag: XiaoValueTag::Error,
        payload: XiaoValuePayload {
            handle: box_error(error),
        },
    }
}

/// 将句柄替换到调用方输出槽，并释放槽中原有的强句柄。
///
/// 调用方必须先把槽初始化为空指针或有效的 ABI 强句柄；失败时新句柄会被回收，旧
/// 槽位保持不变。
unsafe fn write_handle(out: *mut XiaoHandle, handle: XiaoHandle) -> Result<(), i32> {
    if out.is_null() {
        if !handle.is_null() {
            xiao_runtime_release(handle);
        }
        return Err(XiaoAbiStatus::Null.code());
    }
    let previous = unsafe { *out };
    if !previous.is_null() {
        xiao_runtime_release(previous);
    }
    unsafe { *out = handle };
    Ok(())
}

/// 将值替换到调用方输出槽，并释放槽中原有的拥有值。
///
/// 调用方必须先把槽初始化为有效的 `XiaoValue`（通常是 `none`）；这样 Runtime 才能在
/// 覆盖前正确归还旧句柄。失败时新值由本函数回收，旧值保持不变。
unsafe fn write_value(out: *mut XiaoValue, value: XiaoValue) -> Result<(), i32> {
    if out.is_null() {
        let mut value = value;
        xiao_runtime_value_release(&mut value);
        return Err(XiaoAbiStatus::Null.code());
    }
    xiao_runtime_value_release(out);
    unsafe { *out = value };
    Ok(())
}

/// 读取表字段类型描述。
fn field_type(ty: XiaoFieldType) -> Type {
    match ty {
        XiaoFieldType::Dynamic => Type::Dynamic,
        XiaoFieldType::Int => Type::scalar(xiao_syntax::ScalarType::Int),
        XiaoFieldType::Sint => Type::scalar(xiao_syntax::ScalarType::Sint),
        XiaoFieldType::Float => Type::scalar(xiao_syntax::ScalarType::Float),
        XiaoFieldType::Sfloat => Type::scalar(xiao_syntax::ScalarType::Sfloat),
        XiaoFieldType::Bool => Type::scalar(xiao_syntax::ScalarType::Bool),
        XiaoFieldType::Str => Type::scalar(xiao_syntax::ScalarType::Str),
        _ => Type::Dynamic,
    }
}

/// 把 ABI 字段类型转换为 Runtime 类型；未知原始值不能静默降级为 dynamic。
fn checked_field_type(ty: XiaoFieldType) -> Result<Type, i32> {
    if !ty.is_known() {
        return Err(XiaoAbiStatus::InvalidArgument.code());
    }
    Ok(field_type(ty))
}

/// 从 ABI 表描述复制一个 Runtime 表定义。
unsafe fn table_definition(descriptor: *const XiaoTableDescriptor) -> Result<TableDefinition, i32> {
    if descriptor.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    let descriptor = unsafe { &*descriptor };
    let name = unsafe { utf8(descriptor.name) }?;
    if name.is_empty() {
        return Err(XiaoAbiStatus::InvalidArgument.code());
    }
    let kind = match descriptor.kind {
        0 => TableKind::Singleton,
        1 => TableKind::Instance,
        _ => return Err(XiaoAbiStatus::InvalidArgument.code()),
    };
    let span = SourceSpan::new(0, 0).expect("零长度表描述区间有效");
    let mut signature = TableSignature::new(name, kind, span);
    let fields = if descriptor.field_count == 0 {
        &[]
    } else {
        if descriptor.fields.is_null() {
            return Err(XiaoAbiStatus::Null.code());
        }
        unsafe { slice::from_raw_parts(descriptor.fields, descriptor.field_count) }
    };
    for field in fields {
        let field = unsafe { &*(field as *const XiaoTableFieldDescriptor) };
        let name = unsafe { utf8(field.name) }?;
        if name.is_empty() {
            return Err(XiaoAbiStatus::InvalidArgument.code());
        }
        let mut member =
            TableMemberSignature::field(name.clone(), checked_field_type(field.ty)?, span);
        member.visibility = match field.public {
            0 => Visibility::Private,
            1 => Visibility::Public,
            _ => return Err(XiaoAbiStatus::InvalidArgument.code()),
        };
        if signature.members.insert(name, member).is_some() {
            return Err(XiaoAbiStatus::InvalidArgument.code());
        }
    }
    Ok(TableDefinition::new(signature))
}

/// 读取一个值数组并提升为 Runtime 值列表。
unsafe fn value_slice(values: *const XiaoValue, length: usize) -> Result<Vec<RuntimeValue>, i32> {
    if length == 0 {
        return Ok(Vec::new());
    }
    if values.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    unsafe { slice::from_raw_parts(values, length) }
        .iter()
        .map(|value| unsafe { value_to_runtime(value) })
        .collect()
}

/// ABI 版本主入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_abi_version() -> u32 {
    ABI_MAJOR_VERSION
}

/// ABI 版本次入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_abi_minor_version() -> u32 {
    ABI_MINOR_VERSION
}

/// 检查生成代码所需的主/次版本是否由当前 Runtime 满足。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_abi_is_compatible(required_major: u32, required_minor: u32) -> i32 {
    crate::crash::install_platform_failure_reporter();
    i32::from(required_major == ABI_MAJOR_VERSION && required_minor <= ABI_MINOR_VERSION)
}

/// 清除当前线程挂起的原生错误。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_clear() {
    PENDING_ERROR.with(|pending| {
        *pending.borrow_mut() = None;
    });
}

/// 设置当前原生入口的不可变语言上下文；实际目录查找和插值仍由统一诊断渲染器完成。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_language_context_set(locale: XiaoAbiBytes) -> i32 {
    let locale = match unsafe { utf8(locale) } {
        Ok(locale) => locale,
        Err(status) => return status,
    };
    let locale = match validated_locale_tag(&locale) {
        Ok(locale) => locale,
        Err(_) => {
            set_pending_runtime_error(RuntimeError::invalid_value("语言上下文无效"));
            return XiaoAbiStatus::InvalidArgument.code();
        }
    };
    CURRENT_LOCALE.with(|current| *current.borrow_mut() = Some(locale));
    XiaoAbiStatus::Ok.code()
}

/// 开启原生运行期释放事件追踪。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_release_trace_begin() {
    crate::memory::start_release_trace_from_env();
}

/// 刷新原生运行期释放事件追踪文件。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_release_trace_flush() {
    crate::memory::flush_release_trace();
}

/// 返回当前线程挂起错误的机器类别。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_class() -> u32 {
    pending_class().raw()
}

/// 为挂起错误附加源码位置；没有挂起错误时建立一个确定性的可恢复 Runtime 错误。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_attach_span(start: u64, end: u64) -> i32 {
    let Some(span) = SourceSpan::new(start as usize, end as usize) else {
        return XiaoAbiStatus::InvalidArgument.code();
    };
    PENDING_ERROR.with(|pending| {
        let mut pending = pending.borrow_mut();
        if pending.is_none() {
            *pending = Some(PendingError::Recoverable(RuntimeError::invalid_value(
                "Runtime ABI 调用失败",
            )));
        }
        match pending.as_mut() {
            Some(PendingError::Recoverable(error)) => {
                *error = error.clone().with_location(span);
            }
            Some(PendingError::Fatal(error)) => {
                *error = error.clone().with_location(span);
            }
            None => unreachable!(),
        }
    });
    XiaoAbiStatus::Ok.code()
}

/// 创建一个拥有错误对象的 ABI 值。
///
/// 该入口只负责构造可恢复错误；`FatalError` 和未知名称进入独立的 Fatal 槽，
/// 由原生控制流统一终止，不能被普通 `catch` 伪装成成功值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_new(
    type_name: XiaoAbiBytes,
    code: XiaoAbiBytes,
    message: XiaoAbiBytes,
    location: *const XiaoAbiErrorLocation,
) -> XiaoValue {
    let type_name = match unsafe { utf8(type_name) } {
        Ok(value) if !value.is_empty() => value,
        Ok(_) => {
            set_pending_runtime_error(RuntimeError::invalid_value("错误类型名称不能为空"));
            return XiaoValue::none();
        }
        Err(_) => {
            set_pending_runtime_error(RuntimeError::invalid_value("错误类型名称不是有效 UTF-8"));
            return XiaoValue::none();
        }
    };
    let code = match unsafe { utf8(code) } {
        Ok(value) if !value.is_empty() => Some(value),
        Ok(_) => None,
        Err(_) => {
            set_pending_runtime_error(RuntimeError::invalid_value("错误码不是有效 UTF-8"));
            return XiaoValue::none();
        }
    };
    let message = match unsafe { utf8(message) } {
        Ok(value) if !value.is_empty() => Some(value),
        Ok(_) => None,
        Err(_) => {
            set_pending_runtime_error(RuntimeError::invalid_value("错误消息不是有效 UTF-8"));
            return XiaoValue::none();
        }
    };
    let location = if location.is_null() {
        None
    } else {
        let location = unsafe { &*location };
        if location.present == 0 {
            None
        } else {
            let Some(span) =
                SourceSpan::new(location.span.start as usize, location.span.end as usize)
            else {
                set_pending_runtime_error(RuntimeError::invalid_value("错误源码位置无效"));
                return XiaoValue::none();
            };
            Some(span)
        }
    };
    let Some(mut error) =
        RuntimeError::from_type_name(&type_name, code.as_deref(), message.as_deref())
    else {
        set_pending_error(PendingError::Fatal(FatalError::internal(format!(
            "未知或不可恢复的错误类型 {type_name}"
        ))));
        return XiaoValue::none();
    };
    if let Some(location) = location {
        error = error.with_location(location);
    }
    value_from_owned_error(error)
}

#[unsafe(no_mangle)]
/// 从 ABI 字符串值构造错误对象，并保留传入源码位置。
pub extern "C" fn xiao_runtime_error_new_values(
    type_name: XiaoAbiBytes,
    code: *const XiaoValue,
    message: *const XiaoValue,
    location: *const XiaoAbiErrorLocation,
) -> XiaoValue {
    let code = match unsafe { error_text_value(code) } {
        Ok(value) => value,
        Err(()) => return XiaoValue::none(),
    };
    let message = match unsafe { error_text_value(message) } {
        Ok(value) => value,
        Err(()) => return XiaoValue::none(),
    };
    xiao_runtime_error_new(
        type_name,
        abi_bytes(code.as_deref().unwrap_or_default()),
        abi_bytes(message.as_deref().unwrap_or_default()),
        location,
    )
}

unsafe fn error_text_value(value: *const XiaoValue) -> Result<Option<String>, ()> {
    if value.is_null() {
        return Ok(None);
    }
    let runtime = unsafe { value_to_runtime(&*value) }.map_err(|_| {
        set_pending_runtime_error(RuntimeError::invalid_value("错误文本值句柄无效"));
    })?;
    let text = match runtime {
        RuntimeValue::Str(handle) => status(handle.to_string()).map_err(|_| ())?,
        RuntimeValue::Lint(text) | RuntimeValue::Lfloat(text) => text,
        other => {
            set_pending_runtime_error(RuntimeError::type_mismatch("str", other.type_name()));
            return Err(());
        }
    };
    Ok(Some(text))
}

/// 创建并挂起一个语言层可恢复错误。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_raise_type(
    type_name: XiaoAbiBytes,
    code: XiaoAbiBytes,
    message: XiaoAbiBytes,
    location: XiaoAbiErrorLocation,
) -> i32 {
    let type_name = match unsafe { utf8(type_name) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let code = match unsafe { utf8(code) } {
        Ok(value) if !value.is_empty() => Some(value),
        Ok(_) => None,
        Err(status) => return status,
    };
    let message = match unsafe { utf8(message) } {
        Ok(value) if !value.is_empty() => Some(value),
        Ok(_) => None,
        Err(status) => return status,
    };
    let Some(mut error) =
        RuntimeError::from_type_name(&type_name, code.as_deref(), message.as_deref())
    else {
        set_pending_error(PendingError::Fatal(FatalError::internal(format!(
            "未知或不可恢复的错误类型 {type_name}"
        ))));
        return XiaoAbiStatus::Fatal.code();
    };
    if let Some(span) = source_span(location) {
        error = error.with_location(span);
    }
    set_pending_runtime_error(error);
    XiaoAbiStatus::Ok.code()
}

/// 将 `XiaoValueTag::Error` 值复制为挂起的可恢复错误。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_raise_value(
    value: *const XiaoValue,
    location: XiaoAbiErrorLocation,
) -> i32 {
    if value.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let value = unsafe { &*value };
    let Ok(RuntimeValue::Error(error)) = (unsafe { value_to_runtime(value) }) else {
        set_pending_runtime_error(RuntimeError::type_mismatch(
            "error",
            format!("ABI tag {}", value.tag.raw()),
        ));
        return XiaoAbiStatus::RuntimeError.code();
    };
    let mut error = *error;
    if let Some(span) = source_span(location).filter(|_| error.location().is_none()) {
        error = error.with_location(span);
    }
    set_pending_runtime_error(error);
    XiaoAbiStatus::Ok.code()
}

/// 判断当前挂起的可恢复错误是否匹配语言层错误类型名。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_matches(error_type: XiaoAbiBytes) -> i32 {
    let Ok(error_type) = (unsafe { utf8(error_type) }) else {
        return 0;
    };
    PENDING_ERROR.with(|pending| match pending.borrow().as_ref() {
        Some(PendingError::Recoverable(error)) => i32::from(
            xiao_diagnostics::error_kind_of(&error_type)
                .is_some_and(|kind| kind.matches(error.kind())),
        ),
        Some(PendingError::Fatal(_)) | None => 0,
    })
}

/// 把挂起的可恢复错误取成拥有的错误值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_take(out: *mut XiaoValue) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let error = PENDING_ERROR.with(|pending| pending.borrow_mut().take());
    match error {
        Some(PendingError::Recoverable(error)) => unsafe {
            write_value(out, value_from_owned_error(error))
                .map_or_else(|status| status, |_| XiaoAbiStatus::Ok.code())
        },
        Some(PendingError::Fatal(error)) => {
            set_pending_error(PendingError::Fatal(error));
            XiaoAbiStatus::Fatal.code()
        }
        None => XiaoAbiStatus::InvalidArgument.code(),
    }
}

/// 从 Runtime 错误类别返回固定宽度数值。
fn error_kind_snapshot(kind: XiaoErrorKind) -> u32 {
    error_kind_code(kind)
}

/// 从 Fatal 类别返回固定宽度数值。
fn fatal_kind_snapshot(kind: FatalKind) -> u32 {
    match kind {
        FatalKind::RuntimeInvariant => 1,
        FatalKind::CorruptArtifact => 2,
        FatalKind::OutOfMemory => 3,
        FatalKind::StackOverflow => 4,
        FatalKind::Hardware => 5,
        FatalKind::Internal => 6,
    }
}

/// 填充当前线程错误的固定宽度快照。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_snapshot(out: *mut XiaoAbiErrorSnapshot) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let empty = XiaoAbiBytes {
        ptr: std::ptr::null(),
        len: 0,
    };
    let snapshot = PENDING_ERROR.with(|pending| match pending.borrow().as_ref() {
        Some(PendingError::Recoverable(error)) => XiaoAbiErrorSnapshot {
            class: XiaoErrorClass::Recoverable,
            kind: error_kind_snapshot(error.kind()),
            error_id: error.error_id(),
            code: abi_bytes(error.code()),
            message_id: abi_bytes(error.message_id()),
            location: abi_location(error.location()),
            exit_code: 3,
            stack_depth: error.stack().len() as u32,
            param_count: error.params().len() as u32,
        },
        Some(PendingError::Fatal(error)) => XiaoAbiErrorSnapshot {
            class: XiaoErrorClass::Fatal,
            kind: fatal_kind_snapshot(error.kind()),
            error_id: error.error_id(),
            code: abi_bytes(error.code()),
            message_id: abi_bytes(error.message_id()),
            location: abi_location(error.location()),
            exit_code: 4,
            stack_depth: error.stack().len() as u32,
            param_count: error.params().len() as u32,
        },
        None => XiaoAbiErrorSnapshot {
            class: XiaoErrorClass::None,
            kind: 0,
            error_id: 0,
            code: empty,
            message_id: empty,
            location: XiaoAbiErrorLocation::none(),
            exit_code: 0,
            stack_depth: 0,
            param_count: 0,
        },
    });
    unsafe { *out = snapshot };
    XiaoAbiStatus::Ok.code()
}

/// 填充当前线程错误的第一个结构化参数。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_param(index: usize, out: *mut XiaoAbiErrorParam) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let empty = XiaoAbiBytes {
        ptr: std::ptr::null(),
        len: 0,
    };
    let parameter = PENDING_ERROR.with(|pending| {
        let pending = pending.borrow();
        let params = pending.as_ref().map(|error| match error {
            PendingError::Recoverable(error) => error.params(),
            PendingError::Fatal(error) => error.params(),
        })?;
        params.iter().nth(index).map(|(key, value)| {
            let (kind, integer, text) = match value {
                DiagnosticParam::Text(value) => (XiaoErrorParamKind::Text, 0, abi_bytes(value)),
                DiagnosticParam::Integer(value) => {
                    (XiaoErrorParamKind::Integer, *value as i64, empty)
                }
                DiagnosticParam::Boolean(value) => {
                    (XiaoErrorParamKind::Boolean, i64::from(*value), empty)
                }
            };
            XiaoAbiErrorParam {
                key: abi_bytes(key),
                kind,
                integer,
                text,
            }
        })
    });
    let Some(parameter) = parameter else {
        return XiaoAbiStatus::OutOfBounds.code();
    };
    unsafe { *out = parameter };
    XiaoAbiStatus::Ok.code()
}

/// 填充当前线程错误的第一个统一堆栈帧。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_stack_frame(index: usize, out: *mut XiaoAbiStackFrame) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let frame = PENDING_ERROR.with(|pending| {
        let pending = pending.borrow();
        let frames = pending.as_ref().map(|error| match error {
            PendingError::Recoverable(error) => error.stack(),
            PendingError::Fatal(error) => error.stack(),
        });
        frames.and_then(|frames| frames.get(index).cloned())
    });
    let Some(frame) = frame else {
        return XiaoAbiStatus::OutOfBounds.code();
    };
    let empty = XiaoAbiBytes {
        ptr: std::ptr::null(),
        len: 0,
    };
    let backend = frame.backend;
    let output = XiaoAbiStackFrame {
        module: abi_bytes(&frame.module),
        function: abi_bytes(&frame.function),
        source: frame.source.as_deref().map_or(empty, abi_bytes),
        location: abi_location(frame.span),
        bytecode_offset: backend.bytecode_offset.map_or(-1, |value| value as i64),
        native_address: backend.native_address.map_or(-1, |value| value as i64),
        inline_depth: backend.inline_depth.map_or(-1, |value| value as i32),
        frame_kind: u32::from(matches!(frame.kind, FrameKind::Runtime)),
    };
    unsafe { *out = output };
    XiaoAbiStatus::Ok.code()
}

/// 返回当前线程挂起错误的稳定宿主退出码。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_exit_code() -> i32 {
    pending_exit_code()
}

/// 把当前挂起错误写成不依赖语言目录的机器诊断摘要。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_report() {
    let report = PENDING_ERROR.with(|pending| {
        pending.borrow().as_ref().map(|error| match error {
            PendingError::Recoverable(error) => ("recoverable", error.report()),
            PendingError::Fatal(error) => ("fatal", error.report()),
        })
    });
    let Some((class, report)) = report else {
        return;
    };
    write_runtime_report(class, &report, current_locale(), pending_exit_code());
}

/// 为动态原生产物建立本机诊断监听端点；不查找目录，也不执行本地化。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_diagnostic_prepare() -> i32 {
    let listener = match TcpListener::bind(("127.0.0.1", 0)) {
        Ok(listener) => listener,
        Err(error) => return diagnostic_start_failure(format!("无法创建本机诊断端点：{error}")),
    };
    if let Err(error) = listener.set_nonblocking(true) {
        return diagnostic_start_failure(format!("无法配置本机诊断端点：{error}"));
    }
    let endpoint = match listener.local_addr() {
        Ok(endpoint) => endpoint.to_string(),
        Err(error) => return diagnostic_start_failure(format!("无法读取本机诊断端点：{error}")),
    };
    let token = diagnostic_session_token();
    let mut session = match diagnostic_session().lock() {
        Ok(session) => session,
        Err(_) => return diagnostic_start_failure("诊断会话状态已损坏"),
    };
    if session.is_some() {
        return diagnostic_start_failure("诊断会话已经存在");
    }
    unsafe {
        std::env::set_var("XIAO_DIAGNOSTICS_ENDPOINT", &endpoint);
        std::env::set_var("XIAO_DIAGNOSTICS_TOKEN", &token);
    }
    *session = Some(RuntimeDiagnosticSession {
        listener: Some(listener),
        stream: None,
        token,
        started: Instant::now(),
        metrics: DiagnosticMetrics::default(),
    });
    XiaoAbiStatus::Ok.code()
}

/// 等待诊断窗口完成令牌握手；用户代码只能在此成功后开始执行。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_diagnostic_ready() -> i32 {
    let mut session = match diagnostic_session().lock() {
        Ok(session) => session,
        Err(_) => return diagnostic_start_failure("诊断会话状态已损坏"),
    };
    let Some(session) = session.as_mut() else {
        return diagnostic_start_failure("诊断会话尚未准备");
    };
    let Some(listener) = session.listener.take() else {
        return if session.stream.is_some() {
            XiaoAbiStatus::Ok.code()
        } else {
            diagnostic_start_failure("诊断监听端点已经被消费")
        };
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                if let Err(error) = stream.set_read_timeout(Some(Duration::from_secs(2))) {
                    return diagnostic_start_failure(format!("无法配置诊断握手：{error}"));
                }
                let message = match read_message(&mut stream) {
                    Ok(Some(message)) => message,
                    Ok(None) => return diagnostic_start_failure("诊断进程在握手前退出"),
                    Err(error) => {
                        return diagnostic_start_failure(format!("诊断握手读取失败：{error}"));
                    }
                };
                let DiagnosticMessage::Hello {
                    protocol_version,
                    token,
                    ..
                } = message
                else {
                    return diagnostic_start_failure("诊断进程未发送 hello 握手");
                };
                if protocol_version != DIAGNOSTIC_PROTOCOL_VERSION || token != session.token {
                    return diagnostic_start_failure("诊断进程握手版本或令牌不匹配");
                }
                if let Err(error) = write_message(
                    &mut stream,
                    &DiagnosticMessage::Ready {
                        session_id: session.token.clone(),
                    },
                ) {
                    return diagnostic_start_failure(format!("诊断窗口握手确认失败：{error}"));
                }
                let _ = stream.set_write_timeout(Some(Duration::from_millis(20)));
                let _ = stream.set_read_timeout(None);
                session.stream = Some(stream);
                return XiaoAbiStatus::Ok.code();
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return diagnostic_start_failure("诊断进程未在 10 秒内连接");
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                return diagnostic_start_failure(format!("等待诊断进程连接失败：{error}"));
            }
        }
    }
}

/// 记录一个结构化诊断事件；机器字段原样透传，不查目录，也不执行本地化。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_diagnostic_event(event: *const XiaoAbiDiagnosticEvent) -> i32 {
    if event.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let event = unsafe { &*event };
    let event_type = match unsafe { utf8(event.event_type) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let code = match unsafe { utf8(event.code) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let message_id = match unsafe { utf8(event.message_id) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let mut session = match diagnostic_session().lock() {
        Ok(session) => session,
        Err(_) => return XiaoAbiStatus::RuntimeError.code(),
    };
    let Some(session) = session.as_mut() else {
        eprintln!("X11-DIAGNOSTIC-CHANNEL-001: 诊断事件没有已建立的会话");
        return XiaoAbiStatus::RuntimeError.code();
    };
    if session.stream.is_none() {
        return XiaoAbiStatus::RuntimeError.code();
    }
    let location = if event.location.present != 0 {
        json!({
            "start": event.location.span.start,
            "end": event.location.span.end,
        })
    } else {
        Value::Null
    };
    let mut payload = BTreeMap::new();
    payload.insert("code".to_owned(), Value::String(code.clone()));
    payload.insert("message_id".to_owned(), Value::String(message_id.clone()));
    payload.insert("location".to_owned(), location);
    let diagnostic = DiagnosticEvent {
        monotonic_ns: session
            .started
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64,
        level: default_level(&event_type).to_owned(),
        event_type: event_type.clone(),
        module: None,
        source: None,
        node: Some(event_type.clone()),
        function: None,
        error_id: None,
        locale: Some(current_locale()),
        message_id: (!message_id.is_empty()).then_some(message_id),
        params: BTreeMap::new(),
        text: None,
        payload,
    };
    session.metrics.error_count = session.metrics.error_count.saturating_add(u64::from(
        event_type.contains("error") || event_type.contains("fatal"),
    ));
    session.metrics.hook_count = session.metrics.hook_count.saturating_add(u64::from(
        event_type.starts_with("handler_") || event_type.contains("hook"),
    ));
    send_diagnostic_message(
        session,
        &DiagnosticMessage::Event {
            event: Box::new(diagnostic),
        },
    )
    .map_or(XiaoAbiStatus::RuntimeError.code(), |_| {
        XiaoAbiStatus::Ok.code()
    })
}

/// 发送最终指标并关闭原生诊断会话；普通产物调用不到此入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_diagnostic_finish() {
    let Ok(mut session) = diagnostic_session().lock() else {
        return;
    };
    let Some(mut session) = session.take() else {
        return;
    };
    session.metrics.elapsed_ms = session
        .started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let metrics = session.metrics.clone();
    let _ = send_diagnostic_message(&mut session, &DiagnosticMessage::Final { metrics });
    let _ = send_diagnostic_message(
        &mut session,
        &DiagnosticMessage::Close {
            reason: "运行完成".to_owned(),
        },
    );
    unsafe {
        std::env::remove_var("XIAO_DIAGNOSTICS_ENDPOINT");
        std::env::remove_var("XIAO_DIAGNOSTICS_TOKEN");
    }
}

/// 处理 ABI 边界致命故障；该入口永远不会返回。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_fatal_abi() -> ! {
    let error = FatalError::internal("Runtime ABI 边界失败");
    let report = error.report();
    write_runtime_report("fatal", &report, current_locale(), 4);
    std::process::exit(4)
}

/// 返回当前线程的规范语言标签；未设置时使用默认语言。
fn current_locale() -> String {
    CURRENT_LOCALE.with(|current| {
        current
            .borrow()
            .clone()
            .unwrap_or_else(|| "zh-CN".to_owned())
    })
}

/// 通过统一诊断报告记录输出 ABI 的机器摘要与人类可读文本。
fn write_runtime_report(class: &str, report: &ReportRecord, locale: String, exit_code: i32) {
    let location = report.location.map_or_else(
        || "<none>".to_owned(),
        |span| format!("{}..{}", span.start(), span.end()),
    );
    let text = render_builtin_localized_text(report, &locale);
    let line = format!(
        "xiao-error class={class} code={} message_id={} params={:?} span={location} exit_code={exit_code}\n{text}",
        report.code, report.message_id, report.params,
    );
    let event_type = if class == "fatal" {
        "fatal_raised"
    } else {
        "error_raised"
    };
    let event = XiaoAbiDiagnosticEvent {
        event_type: XiaoAbiBytes {
            ptr: event_type.as_ptr(),
            len: event_type.len(),
        },
        code: XiaoAbiBytes {
            ptr: report.code.as_ptr(),
            len: report.code.len(),
        },
        message_id: XiaoAbiBytes {
            ptr: report.message_id.as_ptr(),
            len: report.message_id.len(),
        },
        location: report
            .location
            .map_or_else(XiaoAbiErrorLocation::none, |span| {
                XiaoAbiErrorLocation::from_span(XiaoAbiSpan {
                    start: span.start() as u64,
                    end: span.end() as u64,
                })
            }),
    };
    let _ = xiao_runtime_diagnostic_event(&event);
    let _ = std::io::stderr().write_all(line.as_bytes());
}

/// 保留强句柄；空指针安全地返回空指针。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_retain(handle: XiaoHandle) -> XiaoHandle {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(strong) = (unsafe { strong_ref(handle) }) else {
        return std::ptr::null_mut();
    };
    let Ok(clone) = strong
        .inners
        .borrow()
        .last()
        .ok_or(XiaoAbiStatus::InvalidHandle.code())
        .and_then(|inner| status(inner.try_clone()))
    else {
        return std::ptr::null_mut();
    };
    strong.inners.borrow_mut().push(clone);
    handle
}

/// 释放强句柄；空指针安全，盒子析构会减少 Runtime 强计数。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_release(handle: XiaoHandle) {
    if handle.is_null() {
        return;
    }
    let should_drop = {
        let Ok(strong) = (unsafe { strong_ref(handle) }) else {
            return;
        };
        let mut inners = strong.inners.borrow_mut();
        if inners.len() > 1 {
            let _ = inners.pop();
            false
        } else if inners.len() == 1 {
            let _ = inners.pop();
            true
        } else {
            false
        }
    };
    if should_drop {
        unsafe { drop(Box::from_raw(handle.cast::<AbiStrong>())) };
    }
}

/// 从强句柄建立弱句柄；失败时返回空指针。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_weak(handle: XiaoHandle) -> XiaoWeakHandle {
    let Ok(strong) = (unsafe { strong_ref(handle) }) else {
        return std::ptr::null_mut();
    };
    let Ok(inner) = strong
        .inners
        .borrow()
        .last()
        .ok_or(XiaoAbiStatus::InvalidHandle.code())
        .and_then(|inner| status(inner.try_downgrade()))
    else {
        return std::ptr::null_mut();
    };
    box_weak(inner)
}

/// 保留弱句柄；空指针安全地返回空指针。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_weak_retain(handle: XiaoWeakHandle) -> XiaoWeakHandle {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(weak) = (unsafe { weak_ref(handle) }) else {
        return std::ptr::null_mut();
    };
    let Ok(clone) = weak
        .inners
        .borrow()
        .last()
        .ok_or(XiaoAbiStatus::InvalidHandle.code())
        .and_then(|inner| status(inner.try_clone()))
    else {
        return std::ptr::null_mut();
    };
    weak.inners.borrow_mut().push(clone);
    handle
}

/// 释放弱句柄；它不拥有目标载荷。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_weak_release(handle: XiaoWeakHandle) {
    if handle.is_null() {
        return;
    }
    let should_drop = {
        let Ok(weak) = (unsafe { weak_ref(handle) }) else {
            return;
        };
        let mut inners = weak.inners.borrow_mut();
        if inners.len() > 1 {
            let _ = inners.pop();
            false
        } else if inners.len() == 1 {
            let _ = inners.pop();
            true
        } else {
            false
        }
    };
    if should_drop {
        unsafe { drop(Box::from_raw(handle.cast::<AbiWeak>())) };
    }
}

/// 尝试升级弱句柄；目标已销毁时返回空指针。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_weak_upgrade(handle: XiaoWeakHandle) -> XiaoHandle {
    let Ok(weak) = (unsafe { weak_ref(handle) }) else {
        return std::ptr::null_mut();
    };
    weak.inners
        .borrow()
        .last()
        .and_then(|inner| inner.upgrade().ok())
        .map(box_strong)
        .unwrap_or(std::ptr::null_mut())
}

/// 复制 ABI 值并增加其中句柄的引用计数。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_copy(value: *const XiaoValue, out: *mut XiaoValue) -> i32 {
    if value.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    if std::ptr::eq(value, out.cast_const()) {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    let value = unsafe { &*value };
    let mut copied = XiaoValue {
        tag: value.tag,
        payload: XiaoValuePayload { raw: 0 },
    };
    let result = match value.tag {
        XiaoValueTag::None
        | XiaoValueTag::Bool
        | XiaoValueTag::Int
        | XiaoValueTag::Sint
        | XiaoValueTag::Float
        | XiaoValueTag::Sfloat => {
            if value.tag == XiaoValueTag::Bool
                && !matches!(unsafe { value.payload.bool_value }, 0 | 1)
            {
                return XiaoAbiStatus::InvalidArgument.code();
            }
            copied.payload = value.payload;
            Ok(())
        }
        XiaoValueTag::Str
        | XiaoValueTag::Lint
        | XiaoValueTag::Lfloat
        | XiaoValueTag::Table
        | XiaoValueTag::Array
        | XiaoValueTag::Tuple
        | XiaoValueTag::DictTable
        | XiaoValueTag::DictColumn
        | XiaoValueTag::Set => {
            let handle = xiao_runtime_retain(unsafe { value.payload.handle });
            if handle.is_null() {
                Err(XiaoAbiStatus::InvalidHandle.code())
            } else {
                copied.payload = XiaoValuePayload { handle };
                Ok(())
            }
        }
        XiaoValueTag::Error => match unsafe { retain_error(value.payload.handle) } {
            Ok(handle) => {
                copied.payload = XiaoValuePayload { handle };
                Ok(())
            }
            Err(error) => Err(error),
        },
        XiaoValueTag::TableDropView => {
            let handle = xiao_runtime_weak_retain(unsafe { value.payload.weak_handle });
            if handle.is_null() {
                Err(XiaoAbiStatus::InvalidHandle.code())
            } else {
                copied.payload = XiaoValuePayload {
                    weak_handle: handle,
                };
                Ok(())
            }
        }
        _ => Err(XiaoAbiStatus::InvalidArgument.code()),
    };
    match result {
        Ok(()) => unsafe { write_value(out, copied) }
            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code()),
        Err(error) => error,
    }
}

/// 释放 ABI 值中的句柄并把槽位重置为空值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_release(value: *mut XiaoValue) {
    if value.is_null() {
        return;
    }
    let value = unsafe { &mut *value };
    match value.tag {
        XiaoValueTag::Str
        | XiaoValueTag::Lint
        | XiaoValueTag::Lfloat
        | XiaoValueTag::Table
        | XiaoValueTag::Array
        | XiaoValueTag::Tuple
        | XiaoValueTag::DictTable
        | XiaoValueTag::DictColumn
        | XiaoValueTag::Set => xiao_runtime_release(unsafe { value.payload.handle }),
        XiaoValueTag::Error => unsafe { release_error(value.payload.handle) },
        XiaoValueTag::TableDropView => {
            xiao_runtime_weak_release(unsafe { value.payload.weak_handle });
        }
        _ => {}
    }
    *value = XiaoValue::none();
}

/// 只释放 ABI 值中的弱句柄；这是释放计划 `weak` 动作的窄入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_release_weak(value: *mut XiaoValue) -> i32 {
    if value.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let value = unsafe { &mut *value };
    if value.tag != XiaoValueTag::TableDropView {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    xiao_runtime_weak_release(unsafe { value.payload.weak_handle });
    *value = XiaoValue::none();
    XiaoAbiStatus::Ok.code()
}

/// 从弱句柄构造析构观察值并增加一次弱引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_weak(handle: XiaoWeakHandle) -> XiaoValue {
    if handle.is_null() {
        return XiaoValue::none();
    }
    let Ok(weak) = (unsafe { weak_ref(handle) }) else {
        return XiaoValue::none();
    };
    let is_table = weak
        .inners
        .borrow()
        .last()
        .is_some_and(|inner| inner.type_tag() == RuntimeTypeTag::Table);
    if !is_table {
        return XiaoValue::none();
    }
    let retained = xiao_runtime_weak_retain(handle);
    if retained.is_null() {
        XiaoValue::none()
    } else {
        XiaoValue {
            tag: XiaoValueTag::TableDropView,
            payload: XiaoValuePayload {
                weak_handle: retained,
            },
        }
    }
}

/// 从错误强句柄构造错误值；调用方将句柄所有权转移给返回值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_error(handle: XiaoHandle) -> XiaoValue {
    if unsafe { error_ref(handle) }.is_err() {
        return XiaoValue::none();
    }
    XiaoValue {
        tag: XiaoValueTag::Error,
        payload: XiaoValuePayload { handle },
    }
}

/// 构造内联 64 位整数 ABI 值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_int(value: i64) -> XiaoValue {
    XiaoValue::int(value)
}

/// 构造内联 32 位整数 ABI 值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_sint(value: i32) -> XiaoValue {
    XiaoValue::sint(value)
}

/// 构造内联 64 位浮点 ABI 值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_float(value: f64) -> XiaoValue {
    XiaoValue::float(value)
}

/// 构造内联 32 位浮点 ABI 值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_sfloat(value: f32) -> XiaoValue {
    XiaoValue::sfloat(value)
}

/// 构造内联布尔 ABI 值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_bool(value: u8) -> XiaoValue {
    XiaoValue::bool(value != 0)
}

/// 构造空 ABI 值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_none() -> XiaoValue {
    XiaoValue::none()
}

/// 从 UTF-8 字节构造字符串强句柄。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_string_new(input: XiaoAbiBytes, out: *mut XiaoHandle) -> i32 {
    let text = match unsafe { utf8(input) } {
        Ok(text) => text,
        Err(error) => return error,
    };
    let handle = match status(StringHandle::new(text)) {
        Ok(handle) => box_strong(handle.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 查询字符串 Unicode 标量长度。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_string_len(handle: XiaoHandle, out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::String) } {
        Ok(handle) => handle,
        Err(error) => return error,
    };
    let result = status(StringHandle::from_strong_handle(handle)).map(|handle| handle.len());
    match result {
        Ok(length) => {
            unsafe { *out = length };
            XiaoAbiStatus::Ok.code()
        }
        Err(error) => error,
    }
}

/// 复制字符串 UTF-8 字节到调用方缓冲区。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_string_copy(
    handle: XiaoHandle,
    buffer: XiaoAbiMutBytes,
    written: *mut usize,
) -> i32 {
    if written.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *written = 0 };
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::String) } {
        Ok(handle) => match status(StringHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    let text = match status(handle.to_string()) {
        Ok(text) => text,
        Err(error) => return error,
    };
    let bytes = text.as_bytes();
    unsafe { *written = bytes.len() };
    let output = match unsafe { mut_bytes(buffer) } {
        Ok(output) => output,
        Err(error) => return error,
    };
    if output.len() < bytes.len() {
        return XiaoAbiStatus::OutOfBounds.code();
    }
    output[..bytes.len()].copy_from_slice(bytes);
    XiaoAbiStatus::Ok.code()
}

/// 将字符串强句柄包装为 ABI 字符串值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_str(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::String) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Str, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 消费字符串强句柄并包装为 ABI 字符串值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_str_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::String) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Str, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 将字符串强句柄包装为任意精度整数文本值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_lint(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::String) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Lint, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 消费字符串强句柄并包装为任意精度整数文本值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_lint_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::String) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Lint, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 将字符串强句柄包装为任意精度浮点文本值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_lfloat(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::String) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Lfloat, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 消费字符串强句柄并包装为任意精度浮点文本值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_lfloat_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::String) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Lfloat, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 构造数组对象。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_array_new(
    values: *const XiaoValue,
    length: usize,
    out: *mut XiaoHandle,
) -> i32 {
    let values = match unsafe { value_slice(values, length) } {
        Ok(values) => values,
        Err(error) => return error,
    };
    let handle = match status(ArrayHandle::new(values)) {
        Ok(handle) => box_strong(handle.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 查询数组长度。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_array_len(handle: XiaoHandle, out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::Array) } {
        Ok(handle) => match status(ArrayHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    unsafe { *out = handle.len() };
    XiaoAbiStatus::Ok.code()
}

/// 复制数组元素。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_array_get(
    handle: XiaoHandle,
    index: usize,
    out: *mut XiaoValue,
) -> i32 {
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::Array) } {
        Ok(handle) => match status(ArrayHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    let value = match status(handle.element(index)) {
        Ok(Some(value)) => value,
        Ok(None) => return XiaoAbiStatus::OutOfBounds.code(),
        Err(error) => return error,
    };
    let value = match runtime_to_value(&value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    unsafe { write_value(out, value) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 将数组强句柄包装为 ABI 数组值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_array(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::Array) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Array, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

#[unsafe(no_mangle)]
/// 消费数组强句柄并将其所有权转移到 ABI 数组值，不额外增加引用。
pub extern "C" fn xiao_runtime_value_array_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::Array) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Array, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 构造元组对象。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_tuple_new(
    values: *const XiaoValue,
    length: usize,
    out: *mut XiaoHandle,
) -> i32 {
    let values = match unsafe { value_slice(values, length) } {
        Ok(values) => values,
        Err(error) => return error,
    };
    let handle = match status(TupleHandle::new(values)) {
        Ok(handle) => box_strong(handle.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 查询元组长度。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_tuple_len(handle: XiaoHandle, out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::Tuple) } {
        Ok(handle) => match status(TupleHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    unsafe { *out = handle.len() };
    XiaoAbiStatus::Ok.code()
}

/// 复制元组元素。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_tuple_get(
    handle: XiaoHandle,
    index: usize,
    out: *mut XiaoValue,
) -> i32 {
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::Tuple) } {
        Ok(handle) => match status(TupleHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    let value = match status(handle.element(index)) {
        Ok(Some(value)) => value,
        Ok(None) => return XiaoAbiStatus::OutOfBounds.code(),
        Err(error) => return error,
    };
    let value = match runtime_to_value(&value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    unsafe { write_value(out, value) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 将元组强句柄包装为 ABI 元组值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_tuple(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::Tuple) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Tuple, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

#[unsafe(no_mangle)]
/// 消费元组强句柄并将其所有权转移到 ABI 元组值，不额外增加引用。
pub extern "C" fn xiao_runtime_value_tuple_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::Tuple) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Tuple, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 构造字典表或字典列对象。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_dict_new(
    kind: u32,
    keys: *const XiaoAbiBytes,
    values: *const XiaoValue,
    length: usize,
    out: *mut XiaoHandle,
) -> i32 {
    if length != 0 && (keys.is_null() || values.is_null()) {
        return XiaoAbiStatus::Null.code();
    }
    let keys = if length == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(keys, length) }
    };
    let values = match unsafe { value_slice(values, length) } {
        Ok(values) => values,
        Err(error) => return error,
    };
    let kind = match kind {
        0 => DictKind::Table,
        1 => DictKind::Column,
        _ => return XiaoAbiStatus::InvalidArgument.code(),
    };
    let mut entries = Vec::with_capacity(length);
    let mut seen_keys = std::collections::BTreeSet::new();
    for (key, value) in keys.iter().zip(values) {
        let key = match unsafe { utf8(*key) } {
            Ok(key) => key,
            Err(error) => return error,
        };
        if !seen_keys.insert(key.clone()) {
            return XiaoAbiStatus::InvalidArgument.code();
        }
        entries.push((key, value));
    }
    let handle = match status(DictHandle::new(kind, entries)) {
        Ok(handle) => box_strong(handle.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 查询字典条目数。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_dict_len(handle: XiaoHandle, out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(DictHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    unsafe { *out = handle.len() };
    XiaoAbiStatus::Ok.code()
}

/// 按 UTF-8 键复制字典值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_dict_get(
    handle: XiaoHandle,
    key: XiaoAbiBytes,
    out: *mut XiaoValue,
) -> i32 {
    let key = match unsafe { utf8(key) } {
        Ok(key) => key,
        Err(error) => return error,
    };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(DictHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    let value = match status(handle.value(&key)) {
        Ok(Some(value)) => value,
        Ok(None) => return XiaoAbiStatus::OutOfBounds.code(),
        Err(error) => return error,
    };
    let value = match runtime_to_value(&value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    unsafe { write_value(out, value) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 将字典强句柄包装为 ABI 值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_dict(handle: XiaoHandle, kind: u32) -> XiaoValue {
    let (tag, expected) = match kind {
        0 => (XiaoValueTag::DictTable, RuntimeTypeTag::DictTable),
        1 => (XiaoValueTag::DictColumn, RuntimeTypeTag::DictColumn),
        _ => return XiaoValue::none(),
    };
    unsafe { expect_strong(handle, expected) }
        .map(|handle| value_from_owned_handle(tag, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

#[unsafe(no_mangle)]
/// 消费字典强句柄并将其所有权转移到对应的 ABI 字典值，不额外增加引用。
pub extern "C" fn xiao_runtime_value_dict_owned(handle: XiaoHandle, kind: u32) -> XiaoValue {
    let (tag, expected) = match kind {
        0 => (XiaoValueTag::DictTable, RuntimeTypeTag::DictTable),
        1 => (XiaoValueTag::DictColumn, RuntimeTypeTag::DictColumn),
        _ => return XiaoValue::none(),
    };
    unsafe { take_strong(handle, expected) }
        .map(|handle| value_from_owned_handle(tag, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 构造集合对象。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_set_new(
    values: *const XiaoValue,
    length: usize,
    out: *mut XiaoHandle,
) -> i32 {
    let values = match unsafe { value_slice(values, length) } {
        Ok(values) => values,
        Err(error) => return error,
    };
    let handle = match status(SetHandle::new(values)) {
        Ok(handle) => box_strong(handle.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 查询集合长度。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_set_len(handle: XiaoHandle, out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(SetHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    unsafe { *out = handle.len() };
    XiaoAbiStatus::Ok.code()
}

/// 判断集合是否包含一个 ABI 值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_set_contains(
    handle: XiaoHandle,
    value: *const XiaoValue,
    out: *mut u8,
) -> i32 {
    if value.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(SetHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    match status(handle.contains(&value)) {
        Ok(found) => {
            unsafe { *out = u8::from(found) };
            XiaoAbiStatus::Ok.code()
        }
        Err(error) => error,
    }
}

/// 将集合强句柄包装为 ABI 集合值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_set(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::Set) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Set, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

#[unsafe(no_mangle)]
/// 消费集合强句柄并将其所有权转移到 ABI 集合值，不额外增加引用。
pub extern "C" fn xiao_runtime_value_set_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::Set) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Set, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 按字段描述构造表对象。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_table_new(
    descriptor: *const XiaoTableDescriptor,
    out: *mut XiaoHandle,
) -> i32 {
    let definition = match unsafe { table_definition(descriptor) } {
        Ok(definition) => definition,
        Err(error) => return error,
    };
    let instance = if definition.is_instantiable() {
        TableInstance::new(definition)
    } else {
        TableInstance::singleton(definition)
    };
    let handle = match status(instance) {
        Ok(instance) => box_strong(instance.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 按字段名读取表字段。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_table_get(
    handle: XiaoHandle,
    field: XiaoAbiBytes,
    out: *mut XiaoValue,
) -> i32 {
    let field = match unsafe { utf8(field) } {
        Ok(field) => field,
        Err(error) => return error,
    };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(TableInstance::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    let value = match status(handle.get_compiled_field(&field)) {
        Ok(Some(value)) => value,
        Ok(None) => return XiaoAbiStatus::OutOfBounds.code(),
        Err(error) => return error,
    };
    let value = match runtime_to_value(&value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    unsafe { write_value(out, value) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 按字段名写入表字段。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_table_set(
    handle: XiaoHandle,
    field: XiaoAbiBytes,
    value: *const XiaoValue,
) -> i32 {
    if value.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let field = match unsafe { utf8(field) } {
        Ok(field) => field,
        Err(error) => return error,
    };
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(TableInstance::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    status(handle.set_compiled_field(&field, value))
        .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 将表强句柄包装为 ABI 表值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_table(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::Table) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Table, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

#[unsafe(no_mangle)]
/// 消费表强句柄并将其所有权转移到 ABI 表值，不额外增加引用。
pub extern "C" fn xiao_runtime_value_table_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::Table) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Table, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 保留 N0-A 的最小整数输出入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_write_i64(value: i64) -> i32 {
    use std::io::{self, Write};
    if writeln!(io::stdout(), "{value}").is_ok() {
        XiaoAbiStatus::Ok.code()
    } else {
        XiaoAbiStatus::RuntimeError.code()
    }
}

/// ABI 句柄、值复制和弱引用的回归测试。
#[cfg(test)]
#[path = "abi_tests.rs"]
mod tests;
