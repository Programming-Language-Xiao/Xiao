//! A1 注册式表方法 ABI；复用 tables 的初始化、回滚与析构状态机。

use super::*;
use crate::tables::{TableDropView, TableObject};
use std::collections::BTreeMap;
use std::rc::Rc;
use xiao_runtime_abi::{
    TABLE_DESCRIPTOR_VERSION, TABLE_METHOD_DYNAMIC_TYPE, XiaoTableCallback, XiaoTableDescriptorV2,
    table_method_signature_id,
};
use xiao_types::TableMemberKind;

#[cfg(test)]
#[path = "abi_table_tests.rs"]
mod tests;

/// Runtime 拥有的方法元数据，源描述符和参数数组可以在构造后立即销毁。
struct NativeMethod {
    signature_id: u64,
    parameter_types: Vec<u32>,
    return_type: u32,
    callback: XiaoTableCallback,
}

/// 每个定义的不可变方法集合；不保存借用的描述符指针。
struct NativeTableMetadata {
    methods: BTreeMap<String, NativeMethod>,
    initialize_fields: Option<XiaoTableCallback>,
    init: Option<XiaoTableCallback>,
    drop: Option<XiaoTableCallback>,
}

/// 析构观察盒子的独立类别；仍使用旧弱句柄不透明指针和 TableDropView 值标签。
const ABI_KIND_DROP_VIEW: u32 = 4;

/// 只拥有 TableDropView，绝不保存表强句柄；布局对 C 调用方始终不透明。
#[repr(C)]
struct AbiDropView {
    magic: u64,
    kind: u32,
    _reserved: u32,
    views: RefCell<Vec<TableDropView>>,
    metadata: Rc<NativeTableMetadata>,
}

/// 识别仍有效的 Runtime 句柄地址；悬空或伪造地址不属于 ABI 合法输入。
unsafe fn drop_view_ref<'a>(handle: XiaoWeakHandle) -> Option<&'a AbiDropView> {
    if handle.is_null() {
        return None;
    }
    let header = handle.cast::<AbiDropView>();
    if unsafe { (*header).magic != ABI_HANDLE_MAGIC || (*header).kind != ABI_KIND_DROP_VIEW } {
        return None;
    }
    Some(unsafe { &*header })
}

/// 保留析构观察盒子；仅复制弱只读视图，不复活对象。
pub(super) unsafe fn retain_drop_view(handle: XiaoWeakHandle) -> bool {
    let Some(view) = (unsafe { drop_view_ref(handle) }) else {
        return false;
    };
    let copied = view.views.borrow().last().cloned();
    if let Some(copied) = copied {
        view.views.borrow_mut().push(copied);
    }
    true
}

/// 归还析构观察盒子；最后一份视图消失时释放盒子和共享元数据。
pub(super) unsafe fn release_drop_view(handle: XiaoWeakHandle) -> bool {
    let Some(view) = (unsafe { drop_view_ref(handle) }) else {
        return false;
    };
    let empty = {
        let mut views = view.views.borrow_mut();
        views.pop();
        views.is_empty()
    };
    if empty {
        unsafe { drop(Box::from_raw(handle.cast::<AbiDropView>())) };
    }
    true
}

/// 校验元数据数组长度后借用；不对零长度空指针建立 Rust 切片。
unsafe fn metadata_slice<'a, T>(ptr: *const T, len: usize) -> Result<&'a [T], i32> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    if len > isize::MAX as usize / std::mem::size_of::<T>() {
        return Err(XiaoAbiStatus::InvalidArgument.code());
    }
    Ok(unsafe { slice::from_raw_parts(ptr, len) })
}

/// 方法只接受普通值标签或动态通配标识，析构视图不得作为实参或结果逃逸。
fn valid_method_type(tag: u32) -> bool {
    tag == TABLE_METHOD_DYNAMIC_TYPE
        || (XiaoValueTag::from_raw(tag).is_known() && tag != XiaoValueTag::TableDropView.raw())
}

/// 根据唯一的 ABI 标签校验参数/结果；不克隆借用参数，避免制造释放事件。
fn matches_type(value: &XiaoValue, expected: u32) -> bool {
    value.tag.is_known()
        && value.tag != XiaoValueTag::TableDropView
        && (expected == TABLE_METHOD_DYNAMIC_TYPE || expected == value.tag.raw())
}

/// 复制并验证描述符、方法表以及生命周期入口的一致性。
unsafe fn copy_definition(
    ptr: *const XiaoTableDescriptorV2,
) -> Result<(TableDefinition, Rc<NativeTableMetadata>), i32> {
    if ptr.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    // 先检查固定头，再建立完整描述符引用，拒绝声明为截短布局的输入。
    if unsafe { (*ptr).struct_size } as usize != std::mem::size_of::<XiaoTableDescriptorV2>()
        || unsafe { (*ptr).version } != TABLE_DESCRIPTOR_VERSION
    {
        return Err(XiaoAbiStatus::InvalidArgument.code());
    }
    let descriptor = unsafe { &*ptr };
    unsafe { metadata_slice(descriptor.fields.fields, descriptor.fields.field_count) }?;
    let fields = unsafe { table_definition(&descriptor.fields) }?;
    let mut signature = fields.signature().clone();
    let mut methods = BTreeMap::new();
    for item in unsafe { metadata_slice(descriptor.methods, descriptor.method_count) }? {
        let name = unsafe { utf8(item.name) }?;
        let callback = item.callback.ok_or(XiaoAbiStatus::InvalidArgument.code())?;
        let parameters =
            unsafe { metadata_slice(item.parameter_types, item.parameter_count) }?.to_vec();
        if name.is_empty()
            || item.public > 1
            || !valid_method_type(item.return_type)
            || parameters.iter().any(|tag| !valid_method_type(*tag))
            || item.signature_id != table_method_signature_id(&parameters, item.return_type)
            || signature.members.contains_key(&name)
        {
            return Err(XiaoAbiStatus::InvalidArgument.code());
        }
        let span = signature.span;
        signature.members.insert(
            name.clone(),
            TableMemberSignature {
                name: name.clone(),
                kind: TableMemberKind::Method,
                visibility: if item.public == 1 {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                ty: Type::Function {
                    parameters: vec![Type::Dynamic; parameters.len()],
                    return_type: Box::new(Type::Dynamic),
                },
                function: None,
                span,
            },
        );
        methods.insert(
            name,
            NativeMethod {
                signature_id: item.signature_id,
                parameter_types: parameters,
                return_type: item.return_type,
                callback,
            },
        );
    }
    for (name, callback) in [
        ("ascii:init", descriptor.init),
        ("ascii:drop", descriptor.drop),
    ] {
        match (methods.get(name), callback) {
            (None, None) => {}
            (Some(method), Some(callback))
                if method.parameter_types.is_empty()
                    && method.return_type == XiaoValueTag::None.raw()
                    && std::ptr::fn_addr_eq(method.callback, callback) => {}
            _ => return Err(XiaoAbiStatus::InvalidArgument.code()),
        }
    }
    let metadata = Rc::new(NativeTableMetadata {
        methods,
        initialize_fields: descriptor.initialize_fields,
        init: descriptor.init,
        drop: descriptor.drop,
    });
    let mut definition =
        TableDefinition::new(signature).with_execution_metadata(Rc::clone(&metadata));
    if let Some(callback) = metadata.drop {
        let metadata = Rc::clone(&metadata);
        definition = definition.with_drop_executor(move |object| {
            if pending_class() == XiaoErrorClass::Fatal {
                return Ok(());
            }
            let handle = Box::into_raw(Box::new(AbiDropView {
                magic: ABI_HANDLE_MAGIC,
                kind: ABI_KIND_DROP_VIEW,
                _reserved: 0,
                views: RefCell::new(vec![object.drop_view()]),
                metadata: Rc::clone(&metadata),
            }))
            .cast::<XiaoOpaqueWeakHandle>();
            let mut receiver = XiaoValue {
                tag: XiaoValueTag::TableDropView,
                payload: XiaoValuePayload {
                    weak_handle: handle,
                },
            };
            let result = invoke_callback(callback, &receiver, &[], XiaoValueTag::None.raw());
            release_value_slot(&mut receiver);
            match result {
                Ok(mut value) => {
                    release_value_slot(&mut value);
                    Ok(())
                }
                Err(error) => {
                    // 隐式 StrongHandle::drop 无法返回错误，必须把析构错误交回原生执行边界。
                    set_pending_runtime_error(
                        RuntimeError::table_drop("表 drop 钩子失败").with_cause(error.clone()),
                    );
                    Err(error)
                }
            }
        });
    }
    Ok((definition, metadata))
}

/// 隔离外层错误，让回调执行自己的错误/清理链，结束后恢复外层主因。
fn invoke_callback(
    callback: XiaoTableCallback,
    receiver: &XiaoValue,
    arguments: &[XiaoValue],
    result_type: u32,
) -> RuntimeResult<XiaoValue> {
    let previous = PENDING_ERROR.with(|pending| pending.borrow_mut().take());
    let mut output = XiaoValue::none();
    let status = unsafe { callback(receiver, arguments.as_ptr(), arguments.len(), &mut output) };
    let pending = PENDING_ERROR.with(|pending| pending.borrow_mut().take());
    match pending {
        Some(PendingError::Fatal(fatal)) => {
            set_pending_error(PendingError::Fatal(fatal));
            release_value_slot(&mut output);
            Err(RuntimeError::invalid_value("表回调被致命故障中断"))
        }
        pending => {
            PENDING_ERROR.with(|slot| *slot.borrow_mut() = previous);
            if let Some(PendingError::Recoverable(error)) = pending {
                release_value_slot(&mut output);
                return Err(error);
            }
            if status != XiaoAbiStatus::Ok.code() || !matches_type(&output, result_type) {
                release_value_slot(&mut output);
                return Err(RuntimeError::invalid_value(
                    "表回调状态或返回类型不符合签名",
                ));
            }
            Ok(output)
        }
    }
}

/// 从借用接收者取得共享方法元数据，进入回调前解除全部 RefCell 借用。
unsafe fn receiver_metadata(receiver: &XiaoValue) -> Result<Rc<NativeTableMetadata>, i32> {
    if receiver.tag == XiaoValueTag::TableDropView {
        let view = unsafe { drop_view_ref(receiver.payload.weak_handle) }
            .ok_or(XiaoAbiStatus::InvalidHandle.code())?;
        let views = view.views.borrow();
        let receiver = views.last().ok_or(XiaoAbiStatus::InvalidHandle.code())?;
        status(receiver.validate_callback_borrow())?;
        return Ok(Rc::clone(&view.metadata));
    }
    if receiver.tag != XiaoValueTag::Table {
        return Err(XiaoAbiStatus::InvalidArgument.code());
    }
    let strong = unsafe { strong_ref(receiver.payload.handle) }?;
    let inners = strong.inners.borrow();
    let handle = inners.last().ok_or(XiaoAbiStatus::InvalidHandle.code())?;
    status(
        handle.with_payload(RuntimeTypeTag::Table, |object: &TableObject| {
            object
                .definition()
                .execution_metadata::<NativeTableMetadata>()
        }),
    )?
    .ok_or(XiaoAbiStatus::InvalidArgument.code())
}

/// 新表构造入口；只在构造完整成功后交付唯一拥有的强句柄。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_table_new_v2(
    descriptor: *const XiaoTableDescriptorV2,
    out: *mut XiaoHandle,
) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    if !unsafe { *out }.is_null() {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    let (definition, metadata) = match unsafe { copy_definition(descriptor) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let previous = PENDING_ERROR.with(|pending| pending.borrow_mut().take());
    let instance = TableInstance::with_initializer(definition, |instance| {
        let mut receiver =
            value_from_owned_handle(XiaoValueTag::Table, instance.clone().into_strong_handle());
        let result = (|| {
            for callback in [metadata.initialize_fields, metadata.init]
                .into_iter()
                .flatten()
            {
                let mut value =
                    invoke_callback(callback, &receiver, &[], XiaoValueTag::None.raw())?;
                release_value_slot(&mut value);
            }
            Ok(())
        })();
        release_value_slot(&mut receiver);
        result
    });
    // 初始化失败的 primary/cause/suppressed 由 TableInstance 已有状态机组装，
    // 不能让隐式析构传播槽里的同一错误反客为主或重复追加。
    let pending = PENDING_ERROR.with(|pending| pending.borrow_mut().take());
    PENDING_ERROR.with(|slot| *slot.borrow_mut() = previous);
    if let Some(PendingError::Fatal(fatal)) = pending {
        set_pending_error(PendingError::Fatal(fatal));
    }
    match status(instance) {
        Ok(instance) => {
            unsafe {
                *out = box_strong(instance.into_strong_handle());
            }
            XiaoAbiStatus::Ok.code()
        }
        Err(error) => error,
    }
}

/// 校验方法签名并调用注册回调；不会消费调用方的 receiver/参数。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_table_call(
    receiver: *const XiaoValue,
    method: XiaoAbiBytes,
    signature_id: u64,
    arguments: *const XiaoValue,
    argument_count: usize,
    out: *mut XiaoValue,
) -> i32 {
    if out.is_null() || receiver.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    if std::ptr::eq(receiver, out.cast_const()) {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    // 输出槽由调用方初始化为 none，禁止与借用输入别名。
    if unsafe { (*out).tag } != XiaoValueTag::None {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    let operation = || -> Result<XiaoValue, i32> {
        let name = unsafe { utf8(method) }?;
        if name == "ascii:drop" || name == "ascii:init" {
            return Err(XiaoAbiStatus::InvalidArgument.code());
        }
        let metadata = unsafe { receiver_metadata(&*receiver) }?;
        let method = metadata
            .methods
            .get(&name)
            .ok_or(XiaoAbiStatus::InvalidArgument.code())?;
        let arguments = unsafe { metadata_slice(arguments, argument_count) }?;
        if method.signature_id != signature_id
            || arguments.len() != method.parameter_types.len()
            || arguments
                .iter()
                .zip(&method.parameter_types)
                .any(|(value, expected)| !matches_type(value, *expected))
            || arguments
                .iter()
                .any(|value| std::ptr::eq(value, out.cast_const()))
        {
            return Err(XiaoAbiStatus::InvalidArgument.code());
        }
        status(invoke_callback(
            method.callback,
            unsafe { &*receiver },
            arguments,
            method.return_type,
        ))
    };
    match operation() {
        Ok(value) => {
            unsafe {
                *out = value;
            }
            XiaoAbiStatus::Ok.code()
        }
        Err(error) => error,
    }
}

/// 从强表值或当前有效的 TableDropView 读取已检查字段。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_table_get_value(
    receiver: *const XiaoValue,
    field: XiaoAbiBytes,
    out: *mut XiaoValue,
) -> i32 {
    if receiver.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let receiver = unsafe { &*receiver };
    if !matches!(
        receiver.tag,
        XiaoValueTag::Table | XiaoValueTag::TableDropView
    ) {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    let operation = || -> Result<XiaoValue, i32> {
        let field = unsafe { utf8(field) }?;
        if receiver.tag == XiaoValueTag::Table {
            let handle = unsafe { clone_strong(receiver.payload.handle) }?;
            let instance = status(TableInstance::from_strong_handle(handle))?;
            let value = status(instance.get_compiled_field(&field))?
                .ok_or(XiaoAbiStatus::OutOfBounds.code())?;
            return runtime_into_value(value);
        }
        let view = unsafe { drop_view_ref(receiver.payload.weak_handle) }
            .ok_or(XiaoAbiStatus::InvalidHandle.code())?;
        let views = view.views.borrow();
        let view = views.last().ok_or(XiaoAbiStatus::InvalidHandle.code())?;
        let value =
            status(view.get_compiled_field(&field))?.ok_or(XiaoAbiStatus::OutOfBounds.code())?;
        runtime_into_value(value)
    };
    match operation() {
        Ok(value) => unsafe { write_value(out, value) }
            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code()),
        Err(error) => error,
    }
}

/// 字段读取已经返回一个拥有值，直接转移进 ABI，避免再次克隆后马上归还。
fn runtime_into_value(value: RuntimeValue) -> Result<XiaoValue, i32> {
    let (tag, handle) = match value {
        RuntimeValue::Str(value) => (XiaoValueTag::Str, value.into_strong_handle()),
        RuntimeValue::Table(value) => (XiaoValueTag::Table, value.into_strong_handle()),
        RuntimeValue::Array(value) => (XiaoValueTag::Array, value.into_strong_handle()),
        RuntimeValue::Tuple(value) => (XiaoValueTag::Tuple, value.into_strong_handle()),
        RuntimeValue::DictTable(value) => (XiaoValueTag::DictTable, value.into_strong_handle()),
        RuntimeValue::DictColumn(value) => (XiaoValueTag::DictColumn, value.into_strong_handle()),
        RuntimeValue::Set(value) => (XiaoValueTag::Set, value.into_strong_handle()),
        other => return runtime_to_value(&other),
    };
    Ok(value_from_owned_handle(tag, handle))
}

/// 普通表值字段写入；析构视图不拥有可写或可升级的表句柄。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_table_set_value(
    receiver: *const XiaoValue,
    field: XiaoAbiBytes,
    value: *const XiaoValue,
) -> i32 {
    if receiver.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    if unsafe { (*receiver).tag } != XiaoValueTag::Table {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    xiao_runtime_table_set(unsafe { (*receiver).payload.handle }, field, value)
}

/// 统一归还回调临时值的强或弱所有权；不改变旧 strong/weak 窄入口。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_release_any(value: *mut XiaoValue) {
    if !value.is_null() {
        release_value_slot(unsafe { &mut *value });
    }
}
