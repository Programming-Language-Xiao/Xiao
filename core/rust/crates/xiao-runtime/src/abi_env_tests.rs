//! env 原生入口、注入边界和运行控制安全点测试。

use super::*;
use std::sync::mpsc;
use xiao_intrinsics::CAPABILITY_ENVIRONMENT as ENV;

/// 拥有一个 ABI 值并在析构时释放的测试包装。
struct Value(XiaoValue);

impl Value {
    /// 创建拥有文本值的测试包装。
    fn text(text: &str) -> Self {
        Self(runtime_to_value(&RuntimeValue::new_string(text).unwrap()).unwrap())
    }

    /// 转换回可检查的 Runtime 值。
    fn runtime(&self) -> RuntimeValue {
        unsafe { value_to_runtime(&self.0) }.unwrap()
    }

    /// 断言当前值为数组并返回句柄。
    fn array(&self) -> crate::ArrayHandle {
        let RuntimeValue::Array(array) = self.runtime() else {
            panic!("预期环境数组");
        };
        array
    }
}

impl Drop for Value {
    /// 释放包装持有的强引用。
    fn drop(&mut self) {
        xiao_runtime_value_release_strong(&mut self.0);
    }
}

/// 为快照安装本次测试的原生环境上下文。
fn environment(snapshot: EnvironmentSnapshot) -> Arc<NativeEnvironment> {
    Arc::new(NativeEnvironment {
        snapshot,
        capabilities: ENV,
        control: NativeControl::default(),
    })
}

/// 读取当前挂起的可恢复错误身份。
fn pending_code() -> String {
    PENDING_ERROR.with_borrow(|pending| match pending {
        Some(PendingError::Recoverable(error)) => error.code().to_owned(),
        _ => panic!("预期可恢复错误"),
    })
}

#[test]
/// 验证状态码、输出槽所有权和借读契约。
fn env_abi_status_output_and_ownership() {
    let _entry = enter_environment(Some(environment(EnvironmentSnapshot::default())));
    let name = Value::text("Name");
    let empty = Value::text("");
    let mut out = Value::text("旧值");
    let invalid_tag = XiaoValue {
        tag: XiaoValueTag::from_raw(255),
        payload: XiaoValuePayload { raw: 0 },
    };
    let invalid_handle = XiaoValue {
        tag: XiaoValueTag::Str,
        payload: XiaoValuePayload { raw: 0 },
    };
    let old = out.runtime();
    let alias = &raw mut out.0;
    for read in [xiao_runtime_env_get, xiao_runtime_env_unset] {
        assert_eq!(read(std::ptr::null(), &mut out.0, ENV), 1);
        assert_eq!(read(&name.0, std::ptr::null_mut(), ENV), 1);
        assert_eq!(read(&invalid_tag, &mut out.0, ENV), 2);
        assert_eq!(read(alias, alias, ENV), 2);
        assert_eq!(read(&name.0, &mut out.0, ENV | 0x100), 2);
        assert_eq!(read(&invalid_handle, &mut out.0, ENV), 3);
        let mut invalid_out = XiaoValue {
            tag: XiaoValueTag::Str,
            payload: XiaoValuePayload { raw: 0 },
        };
        assert_eq!(read(&name.0, &mut invalid_out, ENV), 3);
        assert_eq!(read(&XiaoValue::int(0), &mut out.0, 0), 6);
        assert_eq!(pending_code(), "X06-RUNTIME-002");
        xiao_runtime_error_clear();
        assert_eq!(read(&empty.0, &mut out.0, 0), 6);
        assert_eq!(pending_code(), "X06-RUNTIME-026");
        xiao_runtime_error_clear();
        for invalid in ["".to_owned(), "a=b".into(), "a\0b".into(), "x".repeat(257)] {
            let invalid = Value::text(&invalid);
            assert_eq!(read(&invalid.0, &mut out.0, ENV), 6);
            assert_eq!(pending_code(), "X06-RUNTIME-012");
            xiao_runtime_error_clear();
            assert_eq!(out.runtime(), old);
        }
    }
    assert_eq!(xiao_runtime_env_set(&name.0, std::ptr::null(), ENV), 1);
    assert_eq!(xiao_runtime_env_set(&invalid_tag, &name.0, ENV), 2);
    assert_eq!(xiao_runtime_env_set(&name.0, &empty.0, ENV | 0x100), 2);
    assert_eq!(xiao_runtime_env_set(&name.0, &name.0, ENV), 2);
    assert_eq!(xiao_runtime_env_set(&name.0, &invalid_handle, ENV), 3);
    assert_eq!(xiao_runtime_env_set(&name.0, &XiaoValue::int(0), 0), 6);
    assert_eq!(pending_code(), "X06-RUNTIME-002");
    xiao_runtime_error_clear();
    assert_eq!(xiao_runtime_env_set(&empty.0, &name.0, 0), 6);
    assert_eq!(pending_code(), "X06-RUNTIME-026");
    xiao_runtime_error_clear();
    let invalid = Value::text("\0");
    assert_eq!(xiao_runtime_env_set(&name.0, &invalid.0, ENV), 6);
    assert_eq!(pending_code(), "X06-RUNTIME-012");
    xiao_runtime_error_clear();
    let oversized = Value::text(&"x".repeat(65_537));
    assert_eq!(xiao_runtime_env_set(&name.0, &oversized.0, ENV), 6);
    assert_eq!(pending_code(), "ENV_LIMIT_EXCEEDED");
    xiao_runtime_error_clear();

    assert_eq!(xiao_runtime_env_get(&name.0, &mut out.0, ENV), 0);
    assert!(out.array().is_empty());
    assert_eq!(xiao_runtime_env_set(&name.0, &empty.0, ENV), 0);
    assert_eq!(xiao_runtime_env_get(&name.0, &mut out.0, ENV), 0);
    let owned = out.array();
    assert_eq!(owned.len(), 1);
    assert_eq!(owned.element(0).unwrap().unwrap(), empty.runtime());
    assert_eq!(xiao_runtime_env_unset(&name.0, &mut out.0, ENV), 0);
    assert_eq!(out.runtime(), RuntimeValue::Bool(true));
    assert_eq!(xiao_runtime_env_unset(&name.0, &mut out.0, ENV), 0);
    assert_eq!(out.runtime(), RuntimeValue::Bool(false));
    assert_eq!(owned.element(0).unwrap().unwrap(), empty.runtime());
    println!("env C ABI 状态 0/1/2/3/6，独立拥有结果，失败保留输出槽");
}

#[test]
/// 验证 fatal 不展开且不替换既有输出槽。
fn env_abi_fatal_does_not_unwind_or_replace_the_output() {
    let environment = environment(EnvironmentSnapshot::default());
    let name = Value::text("Name");
    let name_value = name.runtime();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = environment
            .snapshot
            .execute::<()>(Operation::Unset, &[&name_value], ENV, || {
                panic!("模拟内部故障");
            });
    }));
    let _entry = enter_environment(Some(environment));
    let value = Value::text("value");
    let mut out = Value::text("preserved");
    let old = out.runtime();
    assert_eq!(xiao_runtime_env_get(&name.0, &mut out.0, ENV), 8);
    assert_eq!(xiao_runtime_env_set(&name.0, &value.0, ENV), 8);
    assert_eq!(xiao_runtime_env_unset(&name.0, &mut out.0, ENV), 8);
    assert_eq!(out.runtime(), old);
    assert_eq!(pending_class(), xiao_runtime_abi::XiaoErrorClass::Fatal);
    xiao_runtime_error_clear();
    println!("env C ABI 三入口均交付状态 8，失败保留输出槽");
}

#[test]
/// 验证任务取消时写删不改变共享视图。
fn env_abi_cancelled_writes_and_removals_preserve_the_shared_view() {
    let snapshot = environment(
        EnvironmentSnapshot::from_pairs(&[("Name".to_owned(), "original".to_owned())]).unwrap(),
    );
    let runtime = crate::RuntimeHandle::create();
    for replace in [true, false] {
        let snapshot = Arc::clone(&snapshot);
        let (ready, started) = mpsc::channel();
        let (resume, blocked) = mpsc::channel();
        let task = crate::scheduler::spawn_cooperative(&runtime, move |_| {
            super::super::abi_concurrency::reset_cancellation_delivery();
            let _entry = enter_environment(Some(snapshot));
            let name = Value::text("Name");
            let mut out = Value(XiaoValue::int(5));
            ready.send(()).unwrap();
            blocked.recv().unwrap();
            let status = if replace {
                xiao_runtime_env_set(&name.0, &Value::text("discarded").0, ENV)
            } else {
                xiao_runtime_env_unset(&name.0, &mut out.0, ENV)
            };
            assert_eq!(out.runtime(), RuntimeValue::Int(5));
            let code = pending_code();
            xiao_runtime_error_clear();
            (status, code)
        })
        .unwrap();
        started
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        task.cancel();
        resume.send(()).unwrap();
        assert_eq!(task.join().unwrap(), (6, "X06-RUNTIME-027".to_owned()));
    }
    let _entry = enter_environment(Some(snapshot));
    let mut out = Value(XiaoValue::none());
    assert_eq!(
        xiao_runtime_env_get(&Value::text("Name").0, &mut out.0, ENV),
        0
    );
    assert_eq!(
        out.array().element(0).unwrap().unwrap(),
        Value::text("original").runtime()
    );
    runtime.close();
    println!("env C ABI 提交前取消返回 6/027，视图和输出槽保留");
}

#[test]
/// 验证省略、空数组和显式字节三种注入来源。
fn env_native_injection_distinguishes_omission_empty_and_explicit_bytes() {
    extern "C" fn inspect() -> i32 {
        let name = Value::text("Name");
        let mut out = Value(XiaoValue::none());
        let status = xiao_runtime_env_get(&name.0, &mut out.0, ENV);
        if status != 0 {
            return -status;
        }
        out.array().len() as i32 + 10
    }
    let invoke = |data: &[u8], capabilities| unsafe {
        xiao_runtime_env_run_injected(Some(inspect), data.as_ptr(), data.len(), capabilities)
    };
    let outer = environment(
        EnvironmentSnapshot::from_pairs(&[("Name".to_owned(), "outer".to_owned())]).unwrap(),
    );
    let _outer = enter_environment(Some(outer));
    assert_eq!(
        unsafe { xiao_runtime_env_run_injected(Some(inspect), std::ptr::null(), 0, ENV) },
        10
    );
    assert_eq!(invoke(b"[]", ENV), 10);
    assert_eq!(invoke(br#"[["Name",""]]"#, ENV), 11);
    assert_eq!(invoke(br#"[["Name",""]]"#, 0), -6);
    assert_eq!(pending_code(), "X06-RUNTIME-026");
    xiao_runtime_error_clear();
    assert_eq!(invoke(b"\xff", ENV), 2);
    assert_eq!(invoke(br#"[["Name","\ud800"]]"#, ENV), 2);
    assert_eq!(invoke(br#"[["=C:","drive"]]"#, ENV), 2);
    assert_eq!(invoke(b"[]", ENV | 0x100), 2);
    assert_eq!(inspect(), 11);
}

/// 验证原始载荷上限在解码前生效，边界值仍可接纳。
#[test]
fn env_injection_bounds_raw_bytes_before_decoding() {
    extern "C" fn entered() -> i32 {
        17
    }
    let pairs: Vec<_> = (0..16)
        .map(|n| (format!("E{n:04}"), "\u{1}".repeat(65_531)))
        .collect();
    let mut data = serde_json::to_vec(&pairs).unwrap();
    assert!(data.len() <= MAX_INJECTED_BYTES);
    data.resize(MAX_INJECTED_BYTES, b' ');
    let invoke = |data: &[u8]| unsafe {
        xiao_runtime_env_run_injected(Some(entered), data.as_ptr(), data.len(), ENV)
    };
    let snapshot = read_injected_environment(std::io::Cursor::new(&data)).unwrap();
    assert_eq!(snapshot.usage(), (16, 1_048_576));
    assert_eq!(invoke(&data), 17);

    data.push(b' ');
    assert_eq!(
        read_injected_environment(std::io::Cursor::new(&data)).unwrap_err(),
        SnapshotError::LimitExceeded
    );
    assert_eq!(invoke(&data), 2);
    // 超限长度必须在借用指针前拒绝，即使该指针未借出任何字节。
    for (ptr, len) in [
        (std::ptr::dangling(), MAX_INJECTED_BYTES + 1),
        (std::ptr::dangling(), usize::MAX),
        (std::ptr::null(), usize::MAX),
        (std::ptr::null(), 1),
    ] {
        assert_eq!(
            unsafe { xiao_runtime_env_run_injected(Some(entered), ptr, len, ENV) },
            2
        );
    }

    let mut over = pairs;
    over[15].1.push('x');
    let over = serde_json::to_vec(&over).unwrap();
    assert!(over.len() <= MAX_INJECTED_BYTES);
    assert_eq!(
        read_injected_environment(std::io::Cursor::new(&over)).unwrap_err(),
        SnapshotError::LimitExceeded
    );
    assert_eq!(invoke(&over), 2);
}

/// 验证运行截止在 env 操作前生效并保留旧视图。
#[test]
fn env_abi_runtime_deadline_preserves_value_entries_and_bytes() {
    let environment = Arc::new(NativeEnvironment {
        snapshot: EnvironmentSnapshot::from_pairs(&[("Name".into(), "original".into())]).unwrap(),
        capabilities: ENV,
        control: NativeControl {
            deadline_ns: Some(0),
            cancellation: None,
        },
    });
    let _entry = enter_environment(Some(Arc::clone(&environment)));
    let name = Value::text("Name");
    let mut out = Value(XiaoValue::int(5));
    let replacement = Value::text("replacement");
    for operation in [Operation::Get, Operation::Set, Operation::Unset] {
        let status = match operation {
            Operation::Get => xiao_runtime_env_get(&name.0, &mut out.0, ENV),
            Operation::Set => xiao_runtime_env_set(&name.0, &replacement.0, ENV),
            Operation::Unset => xiao_runtime_env_unset(&name.0, &mut out.0, ENV),
        };
        assert_eq!(status, XiaoAbiStatus::Fatal.code());
        PENDING_ERROR.with_borrow(|pending| {
            let Some(PendingError::Fatal(error)) = pending else {
                panic!("预期不可捕获的运行截止");
            };
            assert_eq!(error.code(), "X09-DRIVER-002");
        });
        assert_eq!(pending_exit_code(), 2);
        assert_eq!(out.runtime(), RuntimeValue::Int(5));
        assert_eq!(environment.snapshot.usage(), (1, 12));
        let result = environment
            .snapshot
            .execute(Operation::Get, &[&name.runtime()], ENV, || Ok::<_, ()>(()));
        let RuntimeValue::Array(value) = result.unwrap() else {
            panic!("预期环境数组");
        };
        assert_eq!(
            value.element(0).unwrap().unwrap(),
            Value::text("original").runtime()
        );
    }
    xiao_runtime_error_clear();
}
