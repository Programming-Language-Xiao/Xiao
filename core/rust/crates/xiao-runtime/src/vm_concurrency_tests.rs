//! VM 传递信封、锁守卫寿命和消费身份的 Runtime 回归。

use super::*;

static DETACHED_REPORT_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
/// 验证普通跨边界值经历 XCON 编码后成为独立副本。
fn transfer_bytes_are_validated_and_copy_values() {
    let input = RuntimeValue::Tuple(
        TupleHandle::new(vec![
            RuntimeValue::new_string("copy").unwrap(),
            RuntimeValue::Int(7),
        ])
        .unwrap(),
    );
    let encoded = VmTransferValue::encode(&input, 1).unwrap();
    let output = encoded.decode(2).unwrap();
    let (RuntimeValue::Tuple(source), RuntimeValue::Tuple(copy)) = (&input, &output) else {
        panic!("元组")
    };
    assert!(!source.same_object(copy));
    assert_eq!(copy.element(1).unwrap(), Some(RuntimeValue::Int(7)));
    let VmTransferValue::Xcon(mut bytes) = encoded else {
        panic!("普通值必须编码为 XCON")
    };
    bytes[10] = 13;
    assert_eq!(
        VmTransferValue::Xcon(bytes).decode(2).unwrap_err().code(),
        "X06-RUNTIME-029"
    );
}

#[test]
/// 验证 owned guard 持有共享分配并在最后一个别名释放后解锁。
fn owned_guard_keeps_allocation_alive_and_unlocks_after_alias_drop() {
    let value = VmTransferValue::encode(&RuntimeValue::Int(1), 1).unwrap();
    let shared = SharedHandle::new(value, 1, crate::SharedTag::Mut);
    let weak = shared.downgrade();
    let guard = shared
        .lock_owned(&RuntimeCancellation::default(), || false)
        .unwrap();
    let guard = VmConcurrencyValue::LockGuard(Rc::new(RefCell::new(guard)));
    let alias = guard.clone();
    drop(shared);
    drop(guard);
    assert!(weak.upgrade().is_some());
    drop(alias);
    assert!(weak.upgrade().is_none());
}

#[test]
/// 验证超过嵌套上限的递归值在无界复制前被拒绝。
fn excessive_recursive_values_fail_before_unbounded_copy() {
    let mut value = RuntimeValue::Int(1);
    for _ in 0..66 {
        value = RuntimeValue::Tuple(TupleHandle::new(vec![value]).unwrap());
    }
    assert_eq!(
        VmTransferValue::encode(&value, 1).unwrap_err().code(),
        "X06-RUNTIME-029"
    );
}

#[test]
/// 验证线程句柄别名共享消费状态，且不能用作容器键。
fn thread_consumption_is_shared_but_handles_are_not_hashable() {
    let runtime = RuntimeHandle::create();
    let thread = crate::scheduler::spawn_cooperative(&runtime, |_| {
        Ok(VmTransferValue::Xcon(
            encode_xcon(&XconValue::Int(7)).unwrap(),
        ))
    })
    .unwrap();
    let handle = VmConcurrencyValue::thread(thread, runtime.clone());
    let alias = handle.clone();
    assert!(!crate::is_hashable(&RuntimeValue::Concurrency(
        alias.clone()
    )));
    assert!(handle.join_thread(|| false).unwrap().is_ok());
    assert!(matches!(
        alias.join_thread(|| false),
        Err(RuntimeExecError::JoinFailed)
    ));
    runtime.try_close().unwrap();
}

#[test]
/// 验证 VM detached 报告通道有界，并公开报告溢出数量。
fn vm_detached_report_channel_is_bounded_and_counts_drops() {
    let _guard = DETACHED_REPORT_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("VM detached 测试锁未中毒");
    let _ = take_vm_detached_errors();
    let _ = take_vm_detached_error_drops();
    let result = Ok(Err(VmExecutionError::Error(execution_error(
        RuntimeExecError::JoinFailed,
    ))));
    for runtime_id in 0..=VM_DETACHED_ERROR_CAPACITY as u64 {
        report_detached(runtime_id, result.clone());
    }
    let reports = take_vm_detached_errors();
    assert_eq!(reports.len(), VM_DETACHED_ERROR_CAPACITY);
    assert_eq!(take_vm_detached_error_drops(), 1);
}

#[test]
/// 已完成执行单元在 join 前取消时，合成 027 仍使用不可捕获出口。
fn completed_child_cancellation_keeps_noncatchable_variant() {
    let cancellation = RuntimeCancellation::default();
    cancellation.cancel();
    let result = cancel_result(Ok(VmTransferValue::Xcon(vec![0])), &cancellation);
    assert!(
        matches!(result, Err(VmExecutionError::ConcurrencyCancelled(error))
        if error.code() == RuntimeExecError::Cancelled.code())
    );

    let result = cancel_result(
        Err(VmExecutionError::Error(execution_error(
            RuntimeExecError::Cancelled,
        ))),
        &cancellation,
    );
    assert!(
        matches!(result, Err(VmExecutionError::ConcurrencyCancelled(error))
        if error.code() == RuntimeExecError::Cancelled.code())
    );
}

#[test]
/// VM 任务令牌取消与结果通道断连同时发生时，取消 027 优先并保留 028 cause。
fn cancelled_task_disconnect_is_classified_as_027() {
    let cancellation = RuntimeCancellation::default();
    cancellation.cancel();
    let result = normalize_task_wait_result(Err(RuntimeExecError::JoinFailed), &cancellation)
        .expect("内部等待应返回 VM 错误值");
    assert!(matches!(
        result,
        Err(VmExecutionError::ConcurrencyCancelled(error))
            if error.code() == RuntimeExecError::Cancelled.code()
                && error.suppressed().iter().any(|suppressed|
                    suppressed.code() == RuntimeExecError::JoinFailed.code())
    ));
}

#[test]
/// 未取消任务的 channel disconnect 仍保持普通 028。
fn task_disconnect_without_cancellation_remains_028() {
    let cancellation = RuntimeCancellation::default();
    assert!(matches!(
        normalize_task_wait_result(Err(RuntimeExecError::JoinFailed), &cancellation),
        Err(RuntimeExecError::JoinFailed)
    ));
}
