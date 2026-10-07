//! A1 表回调边界、元数据所有权和构造/析构错误链回归。

use super::*;
use xiao_runtime_abi::{XiaoFieldType, XiaoTableMethodDescriptor};

thread_local! {
    static EVENTS: RefCell<Vec<i64>> = const { RefCell::new(Vec::new()) };
    static ESCAPED_VIEW: RefCell<XiaoValue> = const { RefCell::new(XiaoValue::none()) };
}

/// 字段初始化必须发生在 init 之前，并允许经编译字段入口写入。
unsafe extern "C" fn fields(
    receiver: *const XiaoValue,
    _: *const XiaoValue,
    _: usize,
    _: *mut XiaoValue,
) -> i32 {
    xiao_runtime_table_set_value(receiver, abi_bytes("ascii:value"), &XiaoValue::int(7))
}

/// init 读取已初始化字段并改变最终值，便于识别执行次序错误。
unsafe extern "C" fn init(
    receiver: *const XiaoValue,
    _: *const XiaoValue,
    _: usize,
    _: *mut XiaoValue,
) -> i32 {
    let mut value = XiaoValue::none();
    let code = xiao_runtime_table_get_value(receiver, abi_bytes("ascii:value"), &mut value);
    if code != 0 {
        return code;
    }
    let number = unsafe { value.payload.i64_value };
    EVENTS.with(|events| events.borrow_mut().push(number));
    xiao_runtime_table_set_value(
        receiver,
        abi_bytes("ascii:value"),
        &XiaoValue::int(number + 2),
    )
}

/// 方法使用接收者字段和实际参数；不复制或消费输入所有权。
unsafe extern "C" fn add(
    receiver: *const XiaoValue,
    arguments: *const XiaoValue,
    _: usize,
    out: *mut XiaoValue,
) -> i32 {
    let mut value = XiaoValue::none();
    let code = xiao_runtime_table_get_value(receiver, abi_bytes("ascii:value"), &mut value);
    if code != 0 {
        return code;
    }
    unsafe {
        *out = XiaoValue::int(value.payload.i64_value + (*arguments).payload.i64_value);
    }
    0
}

/// 析构只读字段并观察调用次数。
unsafe extern "C" fn drop_read(
    receiver: *const XiaoValue,
    _: *const XiaoValue,
    _: usize,
    _: *mut XiaoValue,
) -> i32 {
    let mut value = XiaoValue::none();
    let code = xiao_runtime_table_get_value(receiver, abi_bytes("ascii:value"), &mut value);
    if code != 0 {
        return code;
    }
    EVENTS.with(|events| events.borrow_mut().push(unsafe { value.payload.i64_value }));
    0
}

/// 故意保留弱视图以验证回调外失效，同时尝试写入和复活必须失败。
unsafe extern "C" fn drop_probe(
    receiver: *const XiaoValue,
    _: *const XiaoValue,
    _: usize,
    _: *mut XiaoValue,
) -> i32 {
    assert_ne!(
        xiao_runtime_table_set_value(receiver, abi_bytes("ascii:value"), &XiaoValue::int(100)),
        0
    );
    assert!(xiao_runtime_weak_upgrade(unsafe { (*receiver).payload.weak_handle }).is_null());
    ESCAPED_VIEW.with(|slot| {
        assert_eq!(
            xiao_runtime_value_copy(receiver, &mut *slot.borrow_mut()),
            0
        )
    });
    unsafe { drop_read(receiver, std::ptr::null(), 0, std::ptr::null_mut()) }
}

/// 错误 init 用于检查状态机包装和失败输出。
unsafe extern "C" fn init_fail(
    _: *const XiaoValue,
    _: *const XiaoValue,
    _: usize,
    _: *mut XiaoValue,
) -> i32 {
    status_from_error(&RuntimeError::invalid_value("init failure"))
}

/// 错误 drop 用于检查析构主因及构造失败的清理抑制链。
unsafe extern "C" fn drop_fail(
    _: *const XiaoValue,
    _: *const XiaoValue,
    _: usize,
    _: *mut XiaoValue,
) -> i32 {
    EVENTS.with(|events| events.borrow_mut().push(99));
    status_from_error(&RuntimeError::invalid_value("drop failure"))
}

/// 构造中的 Fatal 必须阻止普通 drop 回调。
unsafe extern "C" fn init_fatal(
    _: *const XiaoValue,
    _: *const XiaoValue,
    _: usize,
    _: *mut XiaoValue,
) -> i32 {
    set_pending_error(PendingError::Fatal(FatalError::internal("fatal init")));
    XiaoAbiStatus::RuntimeError.code()
}

/// 签名声明为 int，但返回字符串；Runtime 必须回收错误输出而不是交付它。
unsafe extern "C" fn wrong_result(
    _: *const XiaoValue,
    _: *const XiaoValue,
    _: usize,
    out: *mut XiaoValue,
) -> i32 {
    unsafe {
        *out = runtime_to_value(&RuntimeValue::Str(StringHandle::new("wrong").unwrap())).unwrap();
    }
    0
}

/// 本地元数据夹具，指针仅在 construct 内借用。
fn construct(
    init_callback: XiaoTableCallback,
    drop_callback: XiaoTableCallback,
    method_callback: XiaoTableCallback,
) -> Result<XiaoValue, i32> {
    let field = XiaoTableFieldDescriptor {
        name: abi_bytes("ascii:value"),
        ty: XiaoFieldType::Int,
        public: 1,
    };
    let parameters = [XiaoValueTag::Int.raw()];
    let methods = [
        method(
            "ascii:add",
            &parameters,
            XiaoValueTag::Int.raw(),
            method_callback,
        ),
        method("ascii:init", &[], XiaoValueTag::None.raw(), init_callback),
        method("ascii:drop", &[], XiaoValueTag::None.raw(), drop_callback),
    ];
    let descriptor = XiaoTableDescriptorV2 {
        struct_size: std::mem::size_of::<XiaoTableDescriptorV2>() as u32,
        version: TABLE_DESCRIPTOR_VERSION,
        fields: XiaoTableDescriptor {
            name: abi_bytes("Item"),
            kind: 1,
            fields: &field,
            field_count: 1,
        },
        methods: methods.as_ptr(),
        method_count: methods.len(),
        initialize_fields: Some(fields),
        init: Some(init_callback),
        drop: Some(drop_callback),
    };
    let mut handle = std::ptr::null_mut();
    let status = xiao_runtime_table_new_v2(&descriptor, &mut handle);
    if status != 0 {
        assert!(handle.is_null(), "构造失败不得交付部分实例");
        return Err(status);
    }
    Ok(xiao_runtime_value_table_owned(handle))
}

/// 生成签名一致的方法描述。
fn method(
    name: &str,
    parameters: &[u32],
    result: u32,
    callback: XiaoTableCallback,
) -> XiaoTableMethodDescriptor {
    XiaoTableMethodDescriptor {
        name: abi_bytes(name),
        signature_id: table_method_signature_id(parameters, result),
        parameter_types: parameters.as_ptr(),
        parameter_count: parameters.len(),
        return_type: result,
        public: 1,
        callback: Some(callback),
    }
}

/// 在不增加引用的情况下读取实际 Runtime 强计数。
fn strong_count(value: &XiaoValue) -> usize {
    let handle = unsafe { strong_ref(value.payload.handle) }.unwrap();
    handle.inners.borrow().last().unwrap().strong_count()
}

#[test]
/// 元数据离开作用域后仍可调用；别名归还后最后强引用恰好触发一次 drop。
fn native_methods_copy_metadata_and_balance_borrowed_receiver() {
    xiao_runtime_error_clear();
    EVENTS.with(|events| events.borrow_mut().clear());
    let mut value = construct(init, drop_read, add).unwrap();
    assert_eq!(strong_count(&value), 1);
    let mut alias = XiaoValue::none();
    assert_eq!(xiao_runtime_value_copy(&value, &mut alias), 0);
    assert_eq!(strong_count(&value), 2);
    let weak = xiao_runtime_weak(unsafe { value.payload.handle });
    let mut result = XiaoValue::none();
    let signature = table_method_signature_id(&[XiaoValueTag::Int.raw()], XiaoValueTag::Int.raw());
    assert_eq!(
        xiao_runtime_table_call(
            &value,
            abi_bytes("ascii:add"),
            signature,
            &XiaoValue::int(2),
            1,
            &mut result
        ),
        0
    );
    assert_eq!(unsafe { result.payload.i64_value }, 11);
    assert_eq!(strong_count(&value), 2, "借用调用不留额外强引用");
    xiao_runtime_value_release_strong(&mut value);
    EVENTS.with(|events| assert_eq!(*events.borrow(), [7]));
    xiao_runtime_value_release_strong(&mut alias);
    EVENTS.with(|events| assert_eq!(*events.borrow(), [7, 9]));
    assert!(xiao_runtime_weak_upgrade(weak).is_null());
    xiao_runtime_weak_release(weak);
    assert_eq!(pending_class(), XiaoErrorClass::None);
}

#[test]
/// 析构视图不能写、不能升级，复制到回调外后字段和方法均不可使用。
fn native_drop_view_is_readonly_and_expires() {
    xiao_runtime_error_clear();
    let mut value = construct(init, drop_probe, add).unwrap();
    xiao_runtime_value_release_strong(&mut value);
    ESCAPED_VIEW.with(|slot| {
        let mut view = slot.borrow_mut();
        let mut output = XiaoValue::none();
        assert_ne!(
            xiao_runtime_table_get_value(&*view, abi_bytes("ascii:value"), &mut output),
            0
        );
        xiao_runtime_error_clear();
        let signature =
            table_method_signature_id(&[XiaoValueTag::Int.raw()], XiaoValueTag::Int.raw());
        assert_ne!(
            xiao_runtime_table_call(
                &*view,
                abi_bytes("ascii:add"),
                signature,
                &XiaoValue::int(2),
                1,
                &mut output
            ),
            0
        );
        assert_eq!(output.tag, XiaoValueTag::None);
        xiao_runtime_value_release_any(&mut *view);
    });
    xiao_runtime_error_clear();
}

#[test]
/// 初始错误为主因，回滚 drop 错误进入抑制链，不能被传播槽反转或重复追加。
fn native_init_failure_preserves_cause_and_cleanup_error() {
    xiao_runtime_error_clear();
    EVENTS.with(|events| events.borrow_mut().clear());
    assert!(construct(init_fail, drop_fail, add).is_err());
    PENDING_ERROR.with(|slot| {
        let slot = slot.borrow();
        let Some(PendingError::Recoverable(error)) = slot.as_ref() else {
            panic!("缺少初始化错误");
        };
        assert_eq!(error.code(), RuntimeError::table_init("").code());
        assert_eq!(error.cause().unwrap().message(), "init failure");
        assert_eq!(error.suppressed().len(), 1);
        assert_eq!(
            error.suppressed()[0].cause().unwrap().message(),
            "drop failure"
        );
    });
    EVENTS.with(|events| assert_eq!(*events.borrow(), [99]));
    xiao_runtime_error_clear();
}

#[test]
/// 构造 Fatal 不执行普通 drop；普通析构错误则必须跨过 Rust Drop 返回 ABI 边界。
fn native_fatal_skips_drop_and_ordinary_drop_error_is_reported() {
    xiao_runtime_error_clear();
    EVENTS.with(|events| events.borrow_mut().clear());
    assert!(construct(init_fatal, drop_fail, add).is_err());
    assert_eq!(pending_class(), XiaoErrorClass::Fatal);
    EVENTS.with(|events| assert!(events.borrow().is_empty()));
    xiao_runtime_error_clear();
    let mut value = construct(init, drop_fail, add).unwrap();
    xiao_runtime_value_release_strong(&mut value);
    PENDING_ERROR.with(|slot| {
        let slot = slot.borrow();
        let Some(PendingError::Recoverable(error)) = slot.as_ref() else {
            panic!("析构错误丢失");
        };
        assert_eq!(error.code(), RuntimeError::table_drop("").code());
        assert_eq!(error.cause().unwrap().message(), "drop failure");
    });
    xiao_runtime_error_clear();
}

#[test]
/// 坏签名、参数和回调错误结果不得调用成功或泄露输出句柄。
fn native_method_rejects_wrong_signature_arguments_and_result() {
    xiao_runtime_error_clear();
    let mut value = construct(init, drop_read, wrong_result).unwrap();
    let signature = table_method_signature_id(&[XiaoValueTag::Int.raw()], XiaoValueTag::Int.raw());
    let mut output = XiaoValue::none();
    for (signature, argument) in [
        (signature ^ 1, XiaoValue::int(2)),
        (signature, XiaoValue::bool(true)),
    ] {
        assert_ne!(
            xiao_runtime_table_call(
                &value,
                abi_bytes("ascii:add"),
                signature,
                &argument,
                1,
                &mut output
            ),
            0
        );
        assert_eq!(output.tag, XiaoValueTag::None);
    }
    let _trace = crate::memory::start_release_trace();
    assert_ne!(
        xiao_runtime_table_call(
            &value,
            abi_bytes("ascii:add"),
            signature,
            &XiaoValue::int(2),
            1,
            &mut output
        ),
        0
    );
    assert_eq!(output.tag, XiaoValueTag::None);
    assert_eq!(
        crate::memory::take_release_events()
            .iter()
            .filter(|event| event.action == crate::memory::ReleaseAction::Destroy)
            .count(),
        1
    );
    xiao_runtime_error_clear();
    xiao_runtime_value_release_strong(&mut value);
}

#[test]
/// 描述符版本、生命周期映射、重复成员和签名校验必须先于对象分配。
fn native_descriptor_rejects_invalid_metadata_before_allocation() {
    let mut item = method("ascii:add", &[], XiaoValueTag::Int.raw(), wrong_result);
    let mut descriptor = XiaoTableDescriptorV2 {
        struct_size: std::mem::size_of::<XiaoTableDescriptorV2>() as u32,
        version: TABLE_DESCRIPTOR_VERSION,
        fields: XiaoTableDescriptor {
            name: abi_bytes("Item"),
            kind: 1,
            fields: std::ptr::null(),
            field_count: 0,
        },
        methods: &item,
        method_count: 1,
        initialize_fields: None,
        init: None,
        drop: None,
    };
    let _trace = crate::memory::start_release_trace();
    let mut handle = std::ptr::null_mut();
    descriptor.version += 1;
    assert_ne!(xiao_runtime_table_new_v2(&descriptor, &mut handle), 0);
    descriptor.version = TABLE_DESCRIPTOR_VERSION;
    descriptor.struct_size -= 8;
    assert_ne!(xiao_runtime_table_new_v2(&descriptor, &mut handle), 0);
    descriptor.struct_size += 8;
    descriptor.drop = Some(drop_read);
    assert_ne!(xiao_runtime_table_new_v2(&descriptor, &mut handle), 0);
    descriptor.drop = None;
    item.signature_id ^= 1;
    assert_ne!(xiao_runtime_table_new_v2(&descriptor, &mut handle), 0);
    item.signature_id ^= 1;
    item.callback = None;
    assert_ne!(xiao_runtime_table_new_v2(&descriptor, &mut handle), 0);
    item.callback = Some(wrong_result);
    let duplicates = [item, item];
    descriptor.methods = duplicates.as_ptr();
    descriptor.method_count = duplicates.len();
    assert_ne!(xiao_runtime_table_new_v2(&descriptor, &mut handle), 0);
    assert!(handle.is_null());
    assert!(
        crate::memory::take_release_events().is_empty(),
        "无效元数据不得分配表对象"
    );
}

#[test]
/// 外层传播错误与 drop 错误分别保存，回调不能看见或覆盖外层主因。
fn native_drop_failure_is_suppressed_by_existing_primary() {
    xiao_runtime_error_clear();
    let mut value = construct(init, drop_fail, add).unwrap();
    let primary = RuntimeError::invalid_value("outer primary");
    set_pending_runtime_error(primary.clone());
    xiao_runtime_value_release_strong(&mut value);
    PENDING_ERROR.with(|pending| {
        let pending = pending.borrow();
        let Some(PendingError::Recoverable(error)) = pending.as_ref() else {
            panic!("主因丢失");
        };
        assert_eq!(error.message(), primary.message());
        assert_eq!(error.suppressed().len(), 1);
        assert_eq!(
            error.suppressed()[0].cause().unwrap().message(),
            "drop failure"
        );
    });
    xiao_runtime_error_clear();
}
