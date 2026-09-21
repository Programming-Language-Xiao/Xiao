//! `xiao-runtime-abi` 的唯一符号实现。
//!
//! ABI 句柄指向本模块自己的盒子，而不是直接指向 Runtime 对象头。这样既保持对象头
//! 私有，也让 C 调用方无法依赖 Rust 的分配布局；盒子内部只保存一个已经由 06B
//! 实现验证过的 `StrongHandle` 或 `WeakHandle`。
//!
//! 这些句柄继承 06B 的单线程、非原子引用计数约束；调用方不得跨线程传递或并发调用
//! 同一个 ABI 句柄。并发/原子实现需要单独的 ABI 版本与生命周期契约。

use std::cell::RefCell;
use std::slice;

use xiao_runtime_abi::{
    ABI_MAJOR_VERSION, ABI_MINOR_VERSION, XiaoAbiBytes, XiaoAbiMutBytes, XiaoAbiStatus,
    XiaoFieldType, XiaoHandle, XiaoOpaqueHandle, XiaoOpaqueWeakHandle, XiaoTableDescriptor,
    XiaoTableFieldDescriptor, XiaoValue, XiaoValuePayload, XiaoValueTag, XiaoWeakHandle,
};
use xiao_source::SourceSpan;
use xiao_syntax::TableKind;
use xiao_types::{TableMemberSignature, TableSignature, Type, Visibility};

use crate::containers::{ArrayHandle, DictHandle, DictKind, SetHandle, TupleHandle};
use crate::errors::{
    CONTAINER_INDEX_CODE, CONTAINER_KEY_CODE, INVALID_HANDLE_CODE, RuntimeError, RuntimeResult,
    USE_AFTER_RELEASE_CODE, WEAK_UPGRADE_CODE,
};
use crate::memory::{RuntimeTypeTag, StrongHandle, WeakHandle};
use crate::tables::{TableDefinition, TableInstance};
use crate::value::{RuntimeValue, StringHandle};

/// ABI 盒子的魔数；用于在仍可读取的盒子中拒绝明显类型错配。
///
/// 入口先读取候选地址再比较魔数，因此它不是任意外部指针或释放后悬空指针的安全
/// 探测器；调用方仍必须遵守“只传 Runtime 返回且尚未归还的 live 句柄”契约。
const ABI_HANDLE_MAGIC: u64 = 0x5849_414F_4142_4931;
/// ABI 强句柄盒子的种类标记。
const ABI_KIND_STRONG: u32 = 1;
/// ABI 弱句柄盒子的种类标记。
const ABI_KIND_WEAK: u32 = 2;

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

/// 把 Runtime 错误映射到 ABI 稳定状态码。
fn status_from_error(error: &RuntimeError) -> i32 {
    match error.code() {
        INVALID_HANDLE_CODE | USE_AFTER_RELEASE_CODE | WEAK_UPGRADE_CODE => {
            XiaoAbiStatus::InvalidHandle.code()
        }
        CONTAINER_INDEX_CODE | CONTAINER_KEY_CODE => XiaoAbiStatus::OutOfBounds.code(),
        _ => XiaoAbiStatus::RuntimeError.code(),
    }
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
        // 析构视图和错误展开属于 N0-C；它们不能被伪装成可拥有 ABI 值。
        XiaoValueTag::TableDropView | XiaoValueTag::Error => {
            Err(XiaoAbiStatus::InvalidArgument.code())
        }
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
        RuntimeValue::TableDropView(_) | RuntimeValue::Error(_) => {
            Err(XiaoAbiStatus::InvalidArgument.code())
        }
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
    i32::from(required_major == ABI_MAJOR_VERSION && required_minor <= ABI_MINOR_VERSION)
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

/// 将字符串强句柄包装为任意精度整数文本值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_lint(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::String) }
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

#[cfg(test)]
/// ABI 句柄、值复制和弱引用的回归测试。
mod tests {
    use super::*;

    /// 把测试字符串借用为 ABI UTF-8 字节视图。
    fn bytes(text: &str) -> XiaoAbiBytes {
        XiaoAbiBytes {
            ptr: text.as_ptr(),
            len: text.len(),
        }
    }

    #[test]
    /// 强句柄复制和显式释放必须保持对象存活直到最后一份引用归还。
    fn strong_value_copy_uses_runtime_counts() {
        let mut raw = std::ptr::null_mut();
        assert_eq!(xiao_runtime_string_new(bytes("abi"), &mut raw), 0);
        let mut value = xiao_runtime_value_str(raw);
        let mut copy = XiaoValue::none();
        assert_eq!(xiao_runtime_value_copy(&value, &mut copy), 0);
        xiao_runtime_value_release(&mut value);
        let mut output = [0_u8; 8];
        let mut written = 0;
        assert_eq!(
            xiao_runtime_string_copy(
                raw,
                XiaoAbiMutBytes {
                    ptr: output.as_mut_ptr(),
                    capacity: output.len(),
                },
                &mut written,
            ),
            0
        );
        assert_eq!(&output[..written], b"abi");
        xiao_runtime_value_release(&mut copy);
        xiao_runtime_release(raw);
    }

    #[test]
    /// 复制到已拥有值的输出槽时，旧句柄必须先释放而不能泄漏。
    fn value_copy_replaces_existing_owned_output() {
        let mut source_raw = std::ptr::null_mut();
        assert_eq!(xiao_runtime_string_new(bytes("source"), &mut source_raw), 0);
        let mut source = xiao_runtime_value_str(source_raw);
        xiao_runtime_release(source_raw);

        let mut old_raw = std::ptr::null_mut();
        assert_eq!(xiao_runtime_string_new(bytes("old"), &mut old_raw), 0);
        let old_weak = xiao_runtime_weak(old_raw);
        let mut output = xiao_runtime_value_str(old_raw);
        xiao_runtime_release(old_raw);

        assert_eq!(xiao_runtime_value_copy(&source, &mut output), 0);
        assert!(xiao_runtime_weak_upgrade(old_weak).is_null());

        xiao_runtime_value_release(&mut source);
        xiao_runtime_value_release(&mut output);
        xiao_runtime_weak_release(old_weak);
    }

    #[test]
    /// 弱句柄不阻止目标载荷在最后一个强句柄释放时销毁。
    fn weak_upgrade_fails_after_last_strong_release() {
        let mut raw = std::ptr::null_mut();
        assert_eq!(xiao_runtime_string_new(bytes("weak"), &mut raw), 0);
        let weak = xiao_runtime_weak(raw);
        assert!(!weak.is_null());
        xiao_runtime_release(raw);
        assert!(xiao_runtime_weak_upgrade(weak).is_null());
        xiao_runtime_weak_release(weak);
    }

    #[test]
    /// 数组 ABI 会复制动态值并允许按位置读取。
    fn array_round_trip_copies_values() {
        let values = [XiaoValue::int(4), XiaoValue::bool(true)];
        let mut raw = std::ptr::null_mut();
        assert_eq!(
            xiao_runtime_array_new(values.as_ptr(), values.len(), &mut raw),
            0
        );
        let mut length = 0;
        assert_eq!(xiao_runtime_array_len(raw, &mut length), 0);
        assert_eq!(length, 2);
        let mut item = XiaoValue::none();
        assert_eq!(xiao_runtime_array_get(raw, 1, &mut item), 0);
        assert_eq!(item.tag, XiaoValueTag::Bool);
        assert_eq!(unsafe { item.payload.bool_value }, 1);
        xiao_runtime_value_release(&mut item);
        xiao_runtime_release(raw);
    }

    #[test]
    /// ABI retain/release 在同一盒子上成对调用，且最后一次释放才销毁盒子。
    fn retain_keeps_same_box_and_balances_count() {
        let mut raw = std::ptr::null_mut();
        assert_eq!(xiao_runtime_string_new(bytes("retain"), &mut raw), 0);
        let retained = xiao_runtime_retain(raw);
        assert_eq!(retained, raw);
        xiao_runtime_release(raw);
        let mut output = [0_u8; 8];
        let mut written = 0;
        assert_eq!(
            xiao_runtime_string_copy(
                retained,
                XiaoAbiMutBytes {
                    ptr: output.as_mut_ptr(),
                    capacity: output.len(),
                },
                &mut written,
            ),
            0
        );
        assert_eq!(&output[..written], b"retain");
        xiao_runtime_release(retained);
    }

    #[test]
    /// 构造入口复用已有句柄槽时，旧对象必须在替换前释放。
    fn handle_outputs_replace_previous_owner() {
        let mut raw = std::ptr::null_mut();
        assert_eq!(xiao_runtime_string_new(bytes("old"), &mut raw), 0);
        let weak = xiao_runtime_weak(raw);
        assert_eq!(xiao_runtime_string_new(bytes("new"), &mut raw), 0);
        assert!(xiao_runtime_weak_upgrade(weak).is_null());

        let mut output = [0_u8; 4];
        let mut written = 0;
        assert_eq!(
            xiao_runtime_string_copy(
                raw,
                XiaoAbiMutBytes {
                    ptr: output.as_mut_ptr(),
                    capacity: output.len(),
                },
                &mut written,
            ),
            0
        );
        assert_eq!(&output[..written], b"new");
        xiao_runtime_weak_release(weak);
        xiao_runtime_release(raw);
    }

    #[test]
    /// 外部伪造的未知标签必须返回稳定错误，不能让 Rust 读取非法枚举判别值。
    fn rejects_unknown_value_tag() {
        let value = XiaoValue {
            tag: XiaoValueTag::from_raw(0xffff),
            payload: XiaoValuePayload { raw: 0 },
        };
        let mut output = XiaoValue::none();
        assert_eq!(
            xiao_runtime_value_copy(&value, &mut output),
            XiaoAbiStatus::InvalidArgument.code()
        );
    }

    #[test]
    /// 标量复制只复制固定载荷位，不应把整数或布尔值变成空值。
    fn scalar_value_copy_preserves_payload() {
        for value in [XiaoValue::int(-9), XiaoValue::bool(true)] {
            let mut copied = XiaoValue::none();
            assert_eq!(xiao_runtime_value_copy(&value, &mut copied), 0);
            assert_eq!(copied.tag, value.tag);
            match copied.tag {
                XiaoValueTag::Int => assert_eq!(unsafe { copied.payload.i64_value }, -9),
                XiaoValueTag::Bool => assert_eq!(unsafe { copied.payload.bool_value }, 1),
                _ => unreachable!(),
            }
            xiao_runtime_value_release(&mut copied);
        }
    }

    #[test]
    /// 复制非法布尔载荷时必须拒绝，而不能把非零位伪装成合法值。
    fn rejects_invalid_boolean_payload_on_copy() {
        let value = XiaoValue {
            tag: XiaoValueTag::Bool,
            payload: XiaoValuePayload { bool_value: 2 },
        };
        let mut output = XiaoValue::none();
        assert_eq!(
            xiao_runtime_value_copy(&value, &mut output),
            XiaoAbiStatus::InvalidArgument.code()
        );
        assert_eq!(output.tag, XiaoValueTag::None);
    }

    #[test]
    /// 弱值释放入口必须拒绝强值并保留原值，避免错误地把强句柄当弱句柄弹出。
    fn weak_release_rejects_strong_value() {
        let mut raw = std::ptr::null_mut();
        assert_eq!(xiao_runtime_string_new(bytes("strong"), &mut raw), 0);
        let mut value = xiao_runtime_value_str(raw);
        assert_eq!(
            xiao_runtime_value_release_weak(&mut value),
            XiaoAbiStatus::InvalidArgument.code()
        );
        assert_eq!(value.tag, XiaoValueTag::Str);
        xiao_runtime_value_release(&mut value);
        xiao_runtime_release(raw);
    }

    #[test]
    /// 非表强句柄不能伪造表析构观察值。
    fn weak_value_rejects_non_table_target() {
        let mut raw = std::ptr::null_mut();
        assert_eq!(xiao_runtime_string_new(bytes("string"), &mut raw), 0);
        let weak = xiao_runtime_weak(raw);
        assert!(!weak.is_null());
        let value = xiao_runtime_value_weak(weak);
        assert_eq!(value.tag, XiaoValueTag::None);
        xiao_runtime_weak_release(weak);
        xiao_runtime_release(raw);
    }

    #[test]
    /// 字典 ABI 拒绝重复键，避免查找顺序成为未定义语义。
    fn dictionary_rejects_duplicate_keys() {
        let keys = [bytes("duplicate"), bytes("duplicate")];
        let values = [XiaoValue::int(1), XiaoValue::int(2)];
        let mut raw = std::ptr::null_mut();
        assert_eq!(
            xiao_runtime_dict_new(0, keys.as_ptr(), values.as_ptr(), values.len(), &mut raw,),
            XiaoAbiStatus::InvalidArgument.code()
        );
        assert!(raw.is_null());
    }

    #[test]
    /// 表描述符驱动的 get/set 必须沿用字段类型和状态检查。
    fn table_descriptor_round_trip() {
        let field = XiaoTableFieldDescriptor {
            name: bytes("count"),
            ty: XiaoFieldType::Int,
            public: 1,
        };
        let descriptor = XiaoTableDescriptor {
            name: bytes("Counter"),
            kind: 1,
            fields: &field,
            field_count: 1,
        };
        let mut raw = std::ptr::null_mut();
        assert_eq!(xiao_runtime_table_new(&descriptor, &mut raw), 0);
        let input = XiaoValue::int(7);
        assert_eq!(xiao_runtime_table_set(raw, bytes("count"), &input), 0);
        let mut output = XiaoValue::none();
        assert_eq!(xiao_runtime_table_get(raw, bytes("count"), &mut output), 0);
        assert_eq!(output.tag, XiaoValueTag::Int);
        assert_eq!(unsafe { output.payload.i64_value }, 7);
        xiao_runtime_value_release(&mut output);
        xiao_runtime_release(raw);
    }

    #[test]
    /// 表描述符中的未知字段类型和可见性编码必须返回稳定参数错误。
    fn table_descriptor_rejects_unknown_metadata() {
        let bad_type = XiaoTableFieldDescriptor {
            name: bytes("value"),
            ty: XiaoFieldType::from_raw(99),
            public: 1,
        };
        let bad_public = XiaoTableFieldDescriptor {
            name: bytes("other"),
            ty: XiaoFieldType::Int,
            public: 2,
        };
        for field in [bad_type, bad_public] {
            let descriptor = XiaoTableDescriptor {
                name: bytes("Bad"),
                kind: 1,
                fields: &field,
                field_count: 1,
            };
            let mut raw = std::ptr::null_mut();
            assert_eq!(
                xiao_runtime_table_new(&descriptor, &mut raw),
                XiaoAbiStatus::InvalidArgument.code()
            );
            assert!(raw.is_null());
        }
    }

    #[test]
    /// 表名和字段名为空时必须拒绝，避免生成不可寻址的语言成员身份。
    fn table_descriptor_rejects_empty_names() {
        let field = XiaoTableFieldDescriptor {
            name: bytes(""),
            ty: XiaoFieldType::Int,
            public: 1,
        };
        let descriptor = XiaoTableDescriptor {
            name: bytes("Record"),
            kind: 1,
            fields: &field,
            field_count: 1,
        };
        let mut raw = std::ptr::null_mut();
        assert_eq!(
            xiao_runtime_table_new(&descriptor, &mut raw),
            XiaoAbiStatus::InvalidArgument.code()
        );
        assert!(raw.is_null());

        let descriptor = XiaoTableDescriptor {
            name: bytes(""),
            kind: 1,
            fields: std::ptr::null(),
            field_count: 0,
        };
        assert_eq!(
            xiao_runtime_table_new(&descriptor, &mut raw),
            XiaoAbiStatus::InvalidArgument.code()
        );
        assert!(raw.is_null());
    }

    #[test]
    /// 长度输出在无效句柄上先清零，避免调用方继续使用旧结果。
    fn length_outputs_are_zeroed_on_invalid_handle() {
        let mut length = 42;
        assert_eq!(
            xiao_runtime_string_len(std::ptr::null_mut(), &mut length),
            XiaoAbiStatus::Null.code()
        );
        assert_eq!(length, 0);
        length = 42;
        assert_eq!(
            xiao_runtime_array_len(std::ptr::null_mut(), &mut length),
            XiaoAbiStatus::Null.code()
        );
        assert_eq!(length, 0);
    }
}
