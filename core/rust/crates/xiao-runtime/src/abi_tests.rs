//! ABI 句柄、值复制和原生诊断会话的回归测试。
use super::*;

/// 把测试字符串借用为 ABI UTF-8 字节视图。
fn bytes(text: &str) -> XiaoAbiBytes {
    XiaoAbiBytes {
        ptr: text.as_ptr(),
        len: text.len(),
    }
}

#[test]
fn pending_runtime_error_preserves_primary_and_suppresses_secondary() {
    PENDING_ERROR.with(|pending| *pending.borrow_mut() = None);
    let primary = RuntimeError::invalid_value("primary");
    let secondary = RuntimeError::invalid_value("secondary");
    set_pending_runtime_error(primary.clone());
    set_pending_runtime_error(secondary.clone());
    PENDING_ERROR.with(|pending| {
        let pending = pending.borrow();
        let Some(PendingError::Recoverable(error)) = pending.as_ref() else {
            panic!("应保留可恢复主错误");
        };
        assert_eq!(error.code(), primary.code());
        assert_eq!(error.message_id(), primary.message_id());
        assert_eq!(error.suppressed(), &[secondary]);
    });
    PENDING_ERROR.with(|pending| *pending.borrow_mut() = None);
}

#[test]
fn pending_fatal_error_is_not_replaced_by_runtime_error() {
    PENDING_ERROR.with(|pending| {
        *pending.borrow_mut() = Some(PendingError::Fatal(FatalError::internal("fatal")))
    });
    set_pending_runtime_error(RuntimeError::invalid_value("secondary"));
    assert_eq!(pending_class(), XiaoErrorClass::Fatal);
    PENDING_ERROR.with(|pending| *pending.borrow_mut() = None);
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

#[test]
/// 错误构造入口必须保留机器字段和源码位置，并返回拥有的错误值。
fn error_value_constructor_preserves_identity() {
    xiao_runtime_error_clear();
    let location = XiaoAbiErrorLocation::from_span(XiaoAbiSpan { start: 4, end: 9 });
    let mut value = xiao_runtime_error_new(
        bytes("ArithmeticError"),
        bytes("N0-C"),
        bytes("failed"),
        &location,
    );
    assert_eq!(value.tag, XiaoValueTag::Error);
    let RuntimeValue::Error(error) = (unsafe { value_to_runtime(&value) }).expect("错误值")
    else {
        panic!("错误构造器返回了非错误值");
    };
    assert_eq!(error.code(), "N0-C");
    assert_eq!(error.message_id(), "runtime.user_error");
    assert_eq!(error.location(), SourceSpan::new(4, 9));
    xiao_runtime_value_release(&mut value);
}

#[test]
/// `FatalError` 构造必须进入 Fatal 槽，而不能产生可捕获错误值。
fn fatal_error_constructor_never_returns_recoverable_value() {
    xiao_runtime_error_clear();
    let location = XiaoAbiErrorLocation::from_span(XiaoAbiSpan { start: 1, end: 2 });
    let value = xiao_runtime_error_new(
        bytes("FatalError"),
        bytes("fatal"),
        bytes("fatal"),
        &location,
    );
    assert_eq!(value.tag, XiaoValueTag::None);
    let mut snapshot = XiaoAbiErrorSnapshot {
        class: XiaoErrorClass::None,
        kind: 0,
        error_id: 0,
        code: bytes(""),
        message_id: bytes(""),
        location: XiaoAbiErrorLocation::none(),
        exit_code: 0,
        stack_depth: 0,
        param_count: 0,
    };
    assert_eq!(
        xiao_runtime_error_snapshot(&mut snapshot),
        XiaoAbiStatus::Ok.code()
    );
    assert_eq!(snapshot.class, XiaoErrorClass::Fatal);
    xiao_runtime_error_clear();
}

#[test]
/// 诊断 ABI 必须完成握手、透传事件字段并发送 Final/Close 生命周期消息。
fn diagnostic_session_forwards_events_and_closes() {
    xiao_runtime_diagnostic_finish();
    assert_eq!(
        xiao_runtime_language_context_set(bytes("en-US")),
        XiaoAbiStatus::Ok.code()
    );
    assert_eq!(xiao_runtime_diagnostic_prepare(), XiaoAbiStatus::Ok.code());
    let endpoint = std::env::var("XIAO_DIAGNOSTICS_ENDPOINT").expect("诊断端点");
    let token = std::env::var("XIAO_DIAGNOSTICS_TOKEN").expect("诊断令牌");
    let mut stream = TcpStream::connect(endpoint).expect("连接诊断端点");
    write_message(
        &mut stream,
        &DiagnosticMessage::Hello {
            protocol_version: DIAGNOSTIC_PROTOCOL_VERSION,
            token,
            locale: "en-US".to_owned(),
            renderer: "runtime-test".to_owned(),
        },
    )
    .expect("发送诊断握手");
    assert_eq!(xiao_runtime_diagnostic_ready(), XiaoAbiStatus::Ok.code());
    assert!(matches!(
        read_message(&mut stream).expect("读取握手确认"),
        Some(DiagnosticMessage::Ready { .. })
    ));

    let event_type = "error_raised_错误";
    let code = "X11-测试-001";
    let message_id = "xiao.测试.消息";
    let abi_event = XiaoAbiDiagnosticEvent {
        event_type: bytes(event_type),
        code: bytes(code),
        message_id: bytes(message_id),
        location: XiaoAbiErrorLocation::from_span(XiaoAbiSpan { start: 13, end: 21 }),
    };
    assert_eq!(
        xiao_runtime_diagnostic_event(&abi_event),
        XiaoAbiStatus::Ok.code()
    );
    let Some(DiagnosticMessage::Event {
        event: diagnostic_event,
    }) = read_message(&mut stream).expect("读取诊断事件")
    else {
        panic!("应收到诊断事件");
    };
    assert_eq!(diagnostic_event.event_type, event_type);
    assert_eq!(diagnostic_event.level, "error");
    assert_eq!(diagnostic_event.locale.as_deref(), Some("en-US"));
    assert_eq!(diagnostic_event.message_id.as_deref(), Some(message_id));
    assert_eq!(
        diagnostic_event.payload["code"],
        Value::String(code.to_owned())
    );
    assert_eq!(
        diagnostic_event.payload["location"],
        json!({"start": 13, "end": 21})
    );

    xiao_runtime_diagnostic_finish();
    assert!(matches!(
        read_message(&mut stream).expect("读取最终指标"),
        Some(DiagnosticMessage::Final { metrics }) if metrics.error_count == 1
    ));
    assert!(matches!(
        read_message(&mut stream).expect("读取关闭消息"),
        Some(DiagnosticMessage::Close { .. })
    ));
    assert!(std::env::var_os("XIAO_DIAGNOSTICS_ENDPOINT").is_none());

    assert_eq!(
        xiao_runtime_diagnostic_event(&abi_event),
        XiaoAbiStatus::RuntimeError.code()
    );
}
