//! 22D 原生并发 C ABI 的边界、错误和生命周期回归。

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
        xiao_runtime_concurrency_call(operation("shared_read"), &XiaoValue::int(7), 1, &mut shared),
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
/// XCON 恶意 tag、超深和超大输入必须在启动 trampoline 前以 029 拒绝。
fn malformed_xcon_never_enters_native_trampoline() {
    static ENTRY_CALLS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn entry(
        _context: *mut c_void,
        _arguments: *const XiaoValue,
        _argument_count: usize,
        _out: *mut XiaoValue,
    ) -> i32 {
        ENTRY_CALLS.fetch_add(1, Ordering::SeqCst);
        XiaoAbiStatus::Ok.code()
    }

    fn node(tag: u8, payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12 + payload.len());
        bytes.extend_from_slice(b"XCON");
        bytes.extend_from_slice(&crate::XCON_VERSION.to_be_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        bytes.push(tag);
        bytes.push(0);
        bytes.extend_from_slice(payload);
        bytes
    }

    let mut cases = Vec::new();
    for tag in [13, 14] {
        cases.push((format!("reserved tag {tag}"), node(tag, &[])));
    }

    let mut deep = node(0, &[]);
    for _ in 0..=crate::XCON_MAX_DEPTH {
        let mut payload = 1_u32.to_be_bytes().to_vec();
        payload.extend_from_slice(&deep);
        deep = node(10, &payload);
    }
    cases.push(("depth limit".to_owned(), deep));

    // 完整且小于单值字节上限的帧，只有总节点数超过冻结上限。
    let leaf = node(0, &[]);
    let mut many_nodes = (crate::XCON_MAX_NODES as u32).to_be_bytes().to_vec();
    for _ in 0..crate::XCON_MAX_NODES {
        many_nodes.extend_from_slice(&leaf);
    }
    cases.push(("node limit".to_owned(), node(10, &many_nodes)));

    let mut oversized = Vec::new();
    oversized.extend_from_slice(b"XCON");
    oversized.extend_from_slice(&crate::XCON_VERSION.to_be_bytes());
    oversized.extend_from_slice(
        &u32::try_from(crate::XCON_MAX_VALUE_BYTES + 1)
            .expect("XCON 大小限制可由 u32 表示")
            .to_be_bytes(),
    );
    oversized.extend_from_slice(&[8, 0]);
    cases.push(("value size limit".to_owned(), oversized));

    ENTRY_CALLS.store(0, Ordering::SeqCst);
    for (label, bytes) in cases {
        crate::abi::xiao_runtime_error_clear();
        let outcome = invoke_entry(
            entry,
            0,
            vec![NativeRawValue {
                transfer: VmTransferValue::Xcon(bytes),
            }],
            RuntimeHandle::current().id(),
            RuntimeHandle::current().id(),
            "en-US".to_owned(),
        );
        assert!(
            matches!(outcome, NativeEntryOutcome::Error(NativeErrorPacket::Recoverable(error))
                if error.code() == crate::CONCURRENCY_SERIALIZATION_CODE),
            "{label}: 必须返回结构化 029"
        );
        assert_eq!(ENTRY_CALLS.load(Ordering::SeqCst), 0, "{label}");
    }
}

#[test]
/// C-9 在 native trampoline 中往返冻结上限的 64 层 tuple，输出信封字节保持一致。
fn native_xcon_maximum_depth_round_trips_through_trampoline() {
    /// 返回一份拥有的 ABI 参数副本，让输入与返回都经过真实信封编码。
    unsafe extern "C" fn identity(
        _context: *mut c_void,
        arguments: *const XiaoValue,
        argument_count: usize,
        out: *mut XiaoValue,
    ) -> i32 {
        if argument_count != 1 || arguments.is_null() || out.is_null() {
            return XiaoAbiStatus::InvalidArgument.code();
        }
        crate::abi::xiao_runtime_value_copy(arguments, out)
    }
    let runtime_id = RuntimeHandle::current().id();
    let mut value = RuntimeValue::Int(7);
    for _ in 0..crate::XCON_MAX_DEPTH {
        value = RuntimeValue::Tuple(crate::TupleHandle::new(vec![value]).unwrap());
    }
    let transfer = VmTransferValue::encode(&value, runtime_id).unwrap();
    let VmTransferValue::Xcon(expected) = transfer.clone() else {
        panic!("普通 tuple 必须使用 XCON");
    };
    let outcome = invoke_entry(
        identity,
        0,
        vec![NativeRawValue { transfer }],
        runtime_id,
        runtime_id,
        "en-US".to_owned(),
    );
    let NativeEntryOutcome::Value(NativeRawValue {
        transfer: VmTransferValue::Xcon(actual),
    }) = outcome
    else {
        panic!("边界内最大深度应往返成功");
    };
    assert_eq!(actual, expected, "native 输入和结果信封逐字节一致");
}

#[test]
/// native 函数错误展开按内到外次序保留统一用户栈帧和源码区间。
fn native_error_frames_are_appended_during_unwind() {
    crate::abi::xiao_runtime_error_clear();
    crate::abi::set_pending_runtime_error(crate::RuntimeError::new(
        crate::XiaoErrorKind::Other,
        "frame-test",
        "runtime.user_error",
        "frame test",
    ));
    for (function, start, end) in [("inner", 8, 15), ("outer", 0, 20)] {
        crate::abi::xiao_runtime_error_push_native_frame(
            XiaoAbiBytes {
                ptr: b"app".as_ptr(),
                len: 3,
            },
            XiaoAbiBytes {
                ptr: function.as_ptr(),
                len: function.len(),
            },
            start,
            end,
        );
    }
    assert!(matches!(
        take_native_error_packet(),
        Some(NativeErrorPacket::Recoverable(error))
            if error.stack().iter().map(|frame| frame.function.as_str()).collect::<Vec<_>>()
                == ["inner", "outer"]
                && error.stack()[0].span.is_some_and(|span| span.start() == 8 && span.end() == 15)
    ));
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
/// 复用的 pool worker 在每个 trampoline 间隔离错误和 finally checkpoint。
fn native_trampoline_resets_worker_checkpoint_and_inherits_locale() {
    unsafe extern "C" fn entry(
        _context: *mut c_void,
        _arguments: *const XiaoValue,
        _argument_count: usize,
        out: *mut XiaoValue,
    ) -> i32 {
        if out.is_null() {
            return XiaoAbiStatus::Null.code();
        }
        unsafe { *out = XiaoValue::int(9) };
        XiaoAbiStatus::Ok.code()
    }

    let runtime = RuntimeHandle::current();
    let pool = TaskPool::create(runtime.clone(), 1).expect("单 worker 测试池");
    pool.submit(|_| {
        crate::abi::set_pending_runtime_error(crate::RuntimeError::new(
            crate::XiaoErrorKind::Other,
            "stale-worker-error",
            "runtime.user_error",
            "stale",
        ));
        crate::abi::xiao_runtime_error_checkpoint_begin();
    })
    .expect("写入 worker 遗留状态")
    .wait()
    .expect("脏状态任务完成");

    let runtime_id = runtime.id();
    let (is_value, locale, error_is_clear, checkpoints_are_clear) = pool
        .submit(move |_| {
            let outcome = invoke_entry(
                entry,
                0,
                Vec::new(),
                runtime_id,
                runtime_id,
                "en-US".to_owned(),
            );
            (
                matches!(outcome, NativeEntryOutcome::Value(_)),
                super::super::current_locale(),
                super::super::PENDING_ERROR.with(|pending| pending.borrow().is_none()),
                super::super::PENDING_ERROR_CHECKPOINTS
                    .with(|checkpoints| checkpoints.borrow().is_empty()),
            )
        })
        .expect("提交清理 worker 状态的 trampoline")
        .wait()
        .expect("trampoline 任务完成");
    assert!(is_value);
    assert_eq!(locale, "en-US");
    assert!(error_is_clear);
    assert!(checkpoints_are_clear);
    pool.shutdown().expect("关闭单 worker 测试池");
}

#[test]
/// 并发 C ABI 的内部生命周期、边界或测试辅助实现。
fn unsupported_operation_is_explicitly_rejected() {
    let mut output = XiaoValue::none();
    let status =
        xiao_runtime_concurrency_call(operation("spawn"), std::ptr::null(), 0, &mut output);
    assert_ne!(status, XiaoAbiStatus::Ok.code());

    crate::abi::xiao_runtime_error_clear();
    let status =
        xiao_runtime_concurrency_call(operation("future_api"), std::ptr::null(), 0, &mut output);
    assert_eq!(status, XiaoAbiStatus::RuntimeError.code());
    assert!(matches!(
        take_native_error_packet(),
        Some(NativeErrorPacket::Recoverable(error))
            if error.code() == crate::INVALID_VALUE_CODE
    ));
}

#[test]
/// native close drain 失败不能替换入口已经存在的主错误。
fn native_finish_close_error_is_suppressed_under_primary() {
    crate::abi::xiao_runtime_error_clear();
    crate::abi::set_pending_runtime_error(crate::RuntimeError::new(
        crate::XiaoErrorKind::Other,
        "entry-primary",
        "runtime.user_error",
        "entry failed",
    ));
    assert_eq!(
        native_finish_status(Some(RuntimeExecError::RuntimeClosed)),
        XiaoAbiStatus::RuntimeError.code()
    );
    assert_eq!(crate::abi::xiao_runtime_error_exit_code(), 3);
    assert!(matches!(
        take_native_error_packet(),
        Some(NativeErrorPacket::Recoverable(error))
            if error.code() == "entry-primary"
                && error.suppressed().iter().any(|cleanup| {
                    cleanup.code() == crate::CONCURRENCY_RUNTIME_CLOSED_CODE
                })
    ));
}

#[test]
/// C-10 直接驱动真实 finish：错误的 owner 登记产生 031，保留主错误与 suppressed 顺序。
fn entry_close_drain_failure_preserves_primary_and_suppressed() {
    std::thread::spawn(|| {
        let root = RuntimeHandle::current();
        let owner = RuntimeHandle::create();
        let child = RuntimeHandle::create();
        // 正常语言入口不会构造错误 owner；仅在测试中注入登记，走真实上下文检查和 finish。
        native_owned_runtimes()
            .lock()
            .unwrap()
            .push((child.clone(), owner.clone()));
        crate::abi::xiao_runtime_error_clear();
        for code in ["entry-primary", "before-close"] {
            crate::abi::set_pending_runtime_error(crate::RuntimeError::new(
                crate::XiaoErrorKind::Other,
                code,
                "runtime.user_error",
                code,
            ));
        }
        assert_eq!(
            xiao_runtime_concurrency_finish(),
            XiaoAbiStatus::RuntimeError.code()
        );
        assert_eq!(crate::abi::xiao_runtime_error_exit_code(), 3);
        let Some(NativeErrorPacket::Recoverable(error)) = take_native_error_packet() else {
            panic!("真实 finish 必须保留结构化主错误");
        };
        assert_eq!(error.code(), "entry-primary");
        assert_eq!(
            error
                .suppressed()
                .iter()
                .map(|error| error.code())
                .collect::<Vec<_>>(),
            ["before-close", crate::CONCURRENCY_RUNTIME_CLOSED_CODE]
        );
        assert!(root.is_closed(), "finish 仍应关闭当前 Runtime");
        assert!(!child.is_closed(), "错误 owner 不得关闭无关 Runtime");
        assert!(native_owned_runtimes().lock().unwrap().is_empty());
        child.try_close().unwrap();
        owner.try_close().unwrap();
    })
    .join()
    .expect("隔离入口测试线程正常返回");
}

#[test]
/// C-8 宿主取消真实线程令牌后，三个原生 C ABI 检查入口都返回不可捕获的 027。
fn native_external_cancellation_checks_safe_point_wait_and_lock_wait() {
    let runtime = RuntimeHandle::create();
    for point in ["safe_point", "wait", "lock_wait"] {
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let thread = crate::scheduler::spawn_cooperative(&runtime, move |_| {
            started_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap();
            assert_eq!(
                xiao_runtime_concurrency_cancellation_check(operation(point)),
                XiaoAbiStatus::RuntimeError.code()
            );
            assert_eq!(error_matches("Error"), 0, "027 不进入普通 catch");
            assert!(
                matches!(take_native_error_packet(), Some(NativeErrorPacket::Recoverable(error))
                if error.code() == crate::CONCURRENCY_CANCELLED_CODE)
            );
        })
        .unwrap();
        started_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        thread.cancel();
        release_tx.send(()).unwrap();
        assert!(thread.is_cancelled(), "宿主已取消执行单元令牌");
        thread.join().expect("C ABI 检查在 worker 内完成");
    }
    runtime.try_close().unwrap();
}

#[test]
/// C-7/C-8 观察真实等待位后取消宿主令牌，验证原生 lock C ABI 从阻塞中返回 027。
fn native_blocked_lock_wait_is_cancelled_by_host_token() {
    let runtime = RuntimeHandle::create();
    let shared = SharedHandle::new(
        VmTransferValue::encode(&RuntimeValue::Int(7), runtime.id()).unwrap(),
        runtime.id(),
        SharedTag::Mut,
    );
    let held = shared
        .lock_owned(&RuntimeCancellation::default(), || false)
        .unwrap();
    let child_shared = shared.clone();
    let child = crate::scheduler::spawn_cooperative(&runtime, move |_| {
        let mut value = value_from_handle(box_object(ConcurrencyObject::SharedMut(child_shared)));
        let mut guard = XiaoValue::none();
        let status = xiao_runtime_concurrency_call(operation("lock"), &value, 1, &mut guard);
        assert_eq!(status, XiaoAbiStatus::RuntimeError.code());
        assert_eq!(guard.tag, XiaoValueTag::None, "阻塞取消不得产生守卫");
        assert_eq!(error_matches("Error"), 0);
        assert!(
            matches!(take_native_error_packet(), Some(NativeErrorPacket::Recoverable(error))
            if error.code() == crate::CONCURRENCY_CANCELLED_CODE)
        );
        crate::abi::xiao_runtime_value_release(&mut value);
    })
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !shared.has_waiters_for_test() {
        assert!(
            std::time::Instant::now() < deadline,
            "C ABI 必须先进入真实锁等待"
        );
        std::thread::yield_now();
    }
    child.cancel();
    child.join().unwrap();
    drop(held);
    runtime.try_close().unwrap();
}

#[test]
/// finally/drop 中的普通 helper 共享清理 checkpoint，不重复触发取消或丢失主/次错误。
fn native_cleanup_checkpoint_defers_repeated_cancellation() {
    let runtime = RuntimeHandle::create();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let child = crate::scheduler::spawn_cooperative(&runtime, move |_| {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        assert_eq!(
            xiao_runtime_concurrency_cancellation_check(operation("safe_point")),
            XiaoAbiStatus::RuntimeError.code()
        );
        crate::abi::xiao_runtime_error_checkpoint_begin();
        for point in ["safe_point", "wait", "lock_wait"] {
            assert_eq!(
                xiao_runtime_concurrency_cancellation_check(operation(point)),
                XiaoAbiStatus::Ok.code()
            );
        }
        crate::abi::set_pending_runtime_error(crate::RuntimeError::new(
            crate::XiaoErrorKind::Other,
            "cleanup",
            "runtime.user_error",
            "cleanup",
        ));
        crate::abi::xiao_runtime_error_checkpoint_finish();
        let Some(NativeErrorPacket::Recoverable(error)) = take_native_error_packet() else {
            panic!("清理后必须恢复 027");
        };
        assert_eq!(error.code(), crate::CONCURRENCY_CANCELLED_CODE);
        assert_eq!(
            error
                .suppressed()
                .iter()
                .map(|error| error.code())
                .collect::<Vec<_>>(),
            ["cleanup"]
        );
        assert!(!crate::abi::error_cleanup_is_active());
        assert_eq!(
            xiao_runtime_concurrency_cancellation_check(operation("safe_point")),
            XiaoAbiStatus::Ok.code()
        );
        crate::abi::xiao_runtime_error_checkpoint_reset();
        assert_eq!(
            xiao_runtime_concurrency_cancellation_check(operation("safe_point")),
            XiaoAbiStatus::RuntimeError.code()
        );
        let _ = take_native_error_packet();
    })
    .unwrap();
    started_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    child.cancel();
    release_tx.send(()).unwrap();
    child.join().unwrap();
    runtime.try_close().unwrap();
}

#[test]
/// E-0 首次取消在 finally checkpoint 内到达时必须投递，只有已投递取消才抑制。
fn native_first_cancellation_inside_finally_is_delivered() {
    let runtime = RuntimeHandle::create();
    let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(0);
    let (cancelled_tx, cancelled_rx) = std::sync::mpsc::sync_channel(0);
    let child = crate::scheduler::spawn_cooperative(&runtime, move |_| {
        crate::abi::xiao_runtime_error_checkpoint_reset();
        crate::abi::xiao_runtime_error_checkpoint_begin();
        entered_tx.send(()).unwrap();
        cancelled_rx.recv().unwrap();
        assert_eq!(xiao_runtime_concurrency_cancellation_check(operation("safe_point")), XiaoAbiStatus::RuntimeError.code());
        crate::abi::xiao_runtime_error_checkpoint_finish();
        assert!(matches!(take_native_error_packet(), Some(NativeErrorPacket::Recoverable(error)) if error.code() == crate::CONCURRENCY_CANCELLED_CODE));
        assert_eq!(xiao_runtime_concurrency_cancellation_check(operation("safe_point")), XiaoAbiStatus::Ok.code());
    }).unwrap();
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    child.cancel();
    cancelled_tx.send(()).unwrap();
    child.join().unwrap();
    runtime.try_close().unwrap();
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
/// native shared 边界的参数副本在 join 后全部释放并销毁共享对象。
fn native_shared_boundary_releases_every_transfer_alias() {
    let _serial = crate::memory::native_release_trace_test_lock()
        .lock()
        .expect("原生追踪测试串行锁未中毒");

    unsafe extern "C" fn entry(
        _context: *mut c_void,
        arguments: *const XiaoValue,
        argument_count: usize,
        out: *mut XiaoValue,
    ) -> i32 {
        if arguments.is_null() || argument_count != 1 || out.is_null() {
            return XiaoAbiStatus::InvalidArgument.code();
        }
        let value = match unsafe { super::super::value_to_runtime(&*arguments) } {
            Ok(RuntimeValue::Concurrency(VmConcurrencyValue::Shared(shared))) => shared,
            _ => return XiaoAbiStatus::InvalidHandle.code(),
        };
        let decoded = match value.get() {
            Ok(transfer) => match transfer.decode(RuntimeHandle::current().id()) {
                Ok(value) => value,
                Err(_) => return XiaoAbiStatus::RuntimeError.code(),
            },
            Err(_) => return XiaoAbiStatus::RuntimeError.code(),
        };
        match super::super::runtime_to_value(&decoded) {
            Ok(output) => {
                unsafe { *out = output };
                XiaoAbiStatus::Ok.code()
            }
            Err(status) => status,
        }
    }

    crate::memory::start_native_release_trace_for_test();
    let runtime_id = RuntimeHandle::current().id();
    let transfer = VmTransferValue::encode(&RuntimeValue::Int(23), runtime_id).unwrap();
    let shared = SharedHandle::new(transfer, runtime_id, SharedTag::Read);
    let weak = shared.downgrade();
    let mut shared_value = value_from_handle(box_object(ConcurrencyObject::Shared(shared)));
    let mut runtime_value = XiaoValue::none();
    assert_eq!(
        xiao_runtime_concurrency_call(
            operation("current_runtime"),
            std::ptr::null(),
            0,
            &mut runtime_value,
        ),
        XiaoAbiStatus::Ok.code()
    );
    let mut arguments = [XiaoValue::none(), XiaoValue::none()];
    assert_eq!(
        crate::abi::xiao_runtime_value_copy(&runtime_value, &mut arguments[0]),
        XiaoAbiStatus::Ok.code()
    );
    assert_eq!(
        crate::abi::xiao_runtime_value_copy(&shared_value, &mut arguments[1]),
        XiaoAbiStatus::Ok.code()
    );
    let mut thread_value = XiaoValue::none();
    assert_eq!(
        xiao_runtime_concurrency_call_with_entry(
            operation("spawn"),
            Some(entry),
            std::ptr::null_mut(),
            arguments.as_ptr(),
            arguments.len(),
            &mut thread_value,
        ),
        XiaoAbiStatus::Ok.code()
    );
    for argument in arguments.iter_mut().rev() {
        crate::abi::xiao_runtime_value_release(argument);
    }
    let mut output = XiaoValue::none();
    assert_eq!(
        xiao_runtime_concurrency_call(operation("join"), &thread_value, 1, &mut output),
        XiaoAbiStatus::Ok.code()
    );
    crate::abi::xiao_runtime_value_release(&mut output);
    crate::abi::xiao_runtime_value_release(&mut thread_value);
    crate::abi::xiao_runtime_value_release(&mut shared_value);
    crate::abi::xiao_runtime_value_release(&mut runtime_value);

    assert!(weak.upgrade().is_none(), "所有 transfer 别名都应归还");
    assert!(
        crate::memory::take_native_release_events_for_test()
            .iter()
            .any(|event| event.action == crate::memory::ReleaseAction::SharedDestroy)
    );
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
        xiao_runtime_concurrency_call(operation("shared_mut"), &XiaoValue::int(1), 1, &mut shared,),
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
