//! 22C/22D Runtime、线程与任务池的调度和取消回归。

use super::*;

/// 串行化使用进程级 detached 错误通道的单元测试。
static DETACHED_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
/// Runtime 关闭、取消和 join 边界。
fn runtime_spawn_cancel_and_join_are_wired() {
    let runtime = RuntimeHandle::create();
    let handle = spawn(&runtime, |token| token.is_cancelled()).expect("spawn");
    handle.cancel();
    assert_eq!(handle.join(), Err(RuntimeExecError::Cancelled));
    runtime.close();
    assert!(matches!(
        spawn(&runtime, |_| 1_u32),
        Err(error) if error.code() == CONCURRENCY_RUNTIME_CLOSED_CODE
    ));
}

#[test]
/// 任务池容量、提交、等待和关闭边界。
fn task_pool_validates_capacity_and_waits_tasks() {
    let runtime = RuntimeHandle::create();
    assert!(matches!(
        TaskPool::create(runtime.clone(), 0),
        Err(error) if error.code() == CONCURRENCY_POOL_ARGUMENT_CODE
    ));
    let pool = TaskPool::create(runtime, 1).expect("pool");
    let task = pool.submit(|_| 42_u32).expect("submit");
    assert_eq!(task.wait().expect("wait"), 42);
    pool.shutdown().expect("池应成功关闭");
    assert!(matches!(
        pool.submit(|_| 1_u32),
        Err(error) if error.code() == CONCURRENCY_POOL_CLOSED_CODE
    ));
}

#[test]
/// 取消尚在队列中的任务会移除工作项，且任务闭包不运行。
fn cancellation_at_queued_position_skips_task_body() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime, 1).expect("单 worker 测试池");
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let blocker = pool
        .submit(move |_| {
            started_tx.send(()).expect("通知 worker 已占用");
            let _ = release_rx.recv();
        })
        .expect("占用 worker");
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("worker 应开始执行阻塞任务");

    let ran = Arc::new(AtomicBool::new(false));
    let task_ran = Arc::clone(&ran);
    let queued = pool
        .submit(move |_| task_ran.store(true, Ordering::Release))
        .expect("排入第二个任务");
    queued.cancel();
    release_tx.send(()).expect("释放 worker");
    assert_eq!(blocker.wait(), Ok(()));
    assert_eq!(queued.wait(), Err(RuntimeExecError::Cancelled));
    assert!(!ran.load(Ordering::Acquire));
    pool.shutdown().expect("关闭测试池");
}

#[test]
/// 取消正在运行的任务后，任务完成才向等待方返回 027。
fn cancellation_at_running_position_waits_for_task_exit() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime, 1).expect("单 worker 测试池");
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let task = pool
        .submit(move |_| {
            started_tx.send(()).expect("通知任务已运行");
            let _ = release_rx.recv();
            9_u32
        })
        .expect("提交任务");
    let cancellation = task.cancellation();
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("任务应开始运行");
    cancellation.cancel();
    release_tx.send(()).expect("允许任务退出");
    assert_eq!(task.wait(), Err(RuntimeExecError::Cancelled));
    pool.shutdown().expect("关闭测试池");
}

#[test]
/// 等待任务结果期间取消令牌，等待方仍等清理结束后返回 027。
fn cancellation_at_waiting_position_waits_for_task_exit() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime, 1).expect("单 worker 测试池");
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let task = pool
        .submit(move |_| {
            started_tx.send(()).expect("通知任务已运行");
            let _ = release_rx.recv();
            11_u32
        })
        .expect("提交任务");
    let cancellation = task.cancellation();
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("任务应开始运行");
    let (waiting_tx, waiting_rx) = mpsc::sync_channel(0);
    let waiter = std::thread::spawn(move || {
        let entered = AtomicBool::new(false);
        task.wait_interruptible(|| {
            if !entered.swap(true, Ordering::AcqRel) {
                waiting_tx.send(()).expect("通知等待方已进入 wait");
            }
            false
        })
    });
    waiting_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("等待方应在 wait 内部检查中断");
    cancellation.cancel();
    release_tx.send(()).expect("允许任务退出");
    assert_eq!(
        waiter.join().expect("等待线程不应 panic"),
        Err(RuntimeExecError::Cancelled)
    );
    pool.shutdown().expect("关闭测试池");
}

#[test]
/// 任务完成但结果尚未消费时取消，消费句柄仍返回 027。
fn cancellation_at_completed_unreclaimed_position_returns_027() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime, 1).expect("单 worker 测试池");
    let completed = pool.submit(|_| 13_u32).expect("提交目标任务");
    let cancellation = completed.cancellation();
    let marker = pool.submit(|_| ()).expect("提交完成屏障");
    marker.wait().expect("屏障完成代表前一个任务已结束");
    cancellation.cancel();
    assert_eq!(completed.wait(), Err(RuntimeExecError::Cancelled));
    pool.shutdown().expect("关闭测试池");
}

#[test]
/// Runtime 关闭后，公开 shutdown 按冻结语义返回 031。
fn pool_shutdown_after_runtime_close_returns_runtime_closed() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
    runtime.close();
    assert_eq!(pool.shutdown(), Err(RuntimeExecError::RuntimeClosed));
}

#[test]
/// 池 worker 内重入 Runtime close 不等待自身，也不 join 自身。
fn runtime_close_is_reentrant_from_pool_worker() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
    let task_runtime = runtime.clone();
    let task = pool
        .submit(move |_| {
            task_runtime.close();
            9_u32
        })
        .expect("submit");
    assert_eq!(task.wait(), Ok(9));
    assert!(runtime.is_closed());
    assert!(matches!(
        pool.submit(|_| 1_u32),
        Err(RuntimeExecError::RuntimeClosed)
    ));
}

#[test]
/// 两个 worker 同时关闭同一池时只有一个 closer join，不能互相等待。
fn two_pool_workers_can_close_same_pool_concurrently() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime, 2).expect("pool");
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let first_pool = pool.clone();
    let first_barrier = Arc::clone(&barrier);
    let first = pool
        .submit(move |_| {
            first_barrier.wait();
            first_pool.shutdown()
        })
        .expect("first close task");
    let second_pool = pool.clone();
    let second_barrier = Arc::clone(&barrier);
    let second = pool
        .submit(move |_| {
            second_barrier.wait();
            second_pool.shutdown()
        })
        .expect("second close task");
    assert_eq!(first.wait(), Ok(Ok(())));
    assert_eq!(second.wait(), Ok(Ok(())));
}

#[test]
/// 两个活动执行单元同时 close 时只有一个 Runtime drain owner，不能互相等待。
fn two_active_runtime_closers_do_not_deadlock() {
    let runtime = RuntimeHandle::create();
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let (done_tx, done_rx) = mpsc::channel();

    let first_runtime = runtime.clone();
    let first_barrier = Arc::clone(&barrier);
    let first_done = done_tx.clone();
    let first = spawn(&runtime, move |_| {
        first_barrier.wait();
        first_runtime.close();
        first_done.send(()).unwrap();
    })
    .expect("first closer");

    let second_runtime = runtime.clone();
    let second_barrier = Arc::clone(&barrier);
    let second_done = done_tx;
    let second = spawn(&runtime, move |_| {
        second_barrier.wait();
        second_runtime.close();
        second_done.send(()).unwrap();
    })
    .expect("second closer");

    done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("第一个活动 closer 必须返回");
    done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("第二个活动 closer 必须返回");
    assert_eq!(first.join(), Ok(()));
    assert_eq!(second.join(), Ok(()));
    assert!(runtime.is_closed());
}

#[test]
/// pool.shutdown 与 Runtime close 并发时，pool join 不得与 Runtime drain 互等。
fn pool_shutdown_and_runtime_close_do_not_deadlock() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 2).expect("pool");
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let (done_tx, done_rx) = mpsc::channel();

    let first_pool = pool.clone();
    let first_barrier = Arc::clone(&barrier);
    let first_done = done_tx.clone();
    let first = pool
        .submit(move |_| {
            first_barrier.wait();
            let result = first_pool.shutdown();
            first_done.send(()).unwrap();
            result
        })
        .expect("pool closer");

    let second_runtime = runtime.clone();
    let second_barrier = Arc::clone(&barrier);
    let second_done = done_tx;
    let second = pool
        .submit(move |_| {
            second_barrier.wait();
            second_runtime.close();
            second_done.send(()).unwrap();
        })
        .expect("runtime closer");

    done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("pool/runtime closer 必须返回");
    done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("pool/runtime closer 必须都返回");
    assert!(matches!(
        first.wait(),
        Ok(Ok(())) | Ok(Err(RuntimeExecError::RuntimeClosed))
    ));
    assert_eq!(second.wait(), Ok(()));
    assert!(runtime.is_closed());
}

#[test]
/// 两个 Rust pool worker 互相 shutdown 时，closing worker 不得被对方 join。
fn cross_pool_shutdowns_do_not_deadlock() {
    let runtime = RuntimeHandle::create();
    let pool_a = TaskPool::create(runtime.clone(), 1).expect("pool A");
    let pool_b = TaskPool::create(runtime.clone(), 1).expect("pool B");
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let (done_tx, done_rx) = mpsc::channel();

    let pool_b_for_a = pool_b.clone();
    let barrier_a = Arc::clone(&barrier);
    let done_a = done_tx.clone();
    let task_a = pool_a
        .submit(move |_| {
            barrier_a.wait();
            let result = pool_b_for_a.shutdown();
            done_a.send(()).unwrap();
            result
        })
        .expect("pool A worker");

    let pool_a_for_b = pool_a.clone();
    let barrier_b = Arc::clone(&barrier);
    let done_b = done_tx;
    let task_b = pool_b
        .submit(move |_| {
            barrier_b.wait();
            let result = pool_a_for_b.shutdown();
            done_b.send(()).unwrap();
            result
        })
        .expect("pool B worker");

    done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("第一个跨池 shutdown 必须返回");
    done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("第二个跨池 shutdown 必须返回");
    assert_eq!(task_a.wait(), Ok(Ok(())));
    assert_eq!(task_b.wait(), Ok(Ok(())));
    runtime.close();
}

#[test]
/// Rust API 不允许 Runtime worker 关闭另一个 Runtime 的 pool。
fn cross_runtime_pool_shutdown_is_rejected() {
    let runtime_a = RuntimeHandle::create();
    let runtime_b = RuntimeHandle::create();
    let pool_a = TaskPool::create(runtime_a.clone(), 1).expect("pool A");
    let pool_b = TaskPool::create(runtime_b.clone(), 1).expect("pool B");
    let target_pool = pool_b.clone();
    let task = pool_a
        .submit(move |_| target_pool.shutdown())
        .expect("跨 Runtime 负例任务");

    assert_eq!(
        task.wait(),
        Ok(Err(RuntimeExecError::RuntimeClosed)),
        "跨 Runtime pool shutdown 必须返回 031"
    );
    assert!(!pool_b.inner.closed.load(Ordering::Acquire));
    pool_a.shutdown().expect("pool A shutdown");
    pool_b.shutdown().expect("pool B shutdown");
    runtime_a.close();
    runtime_b.close();
}

#[test]
/// Runtime worker 不能跨 Runtime close 或 spawn；两者都返回 031。
fn cross_runtime_runtime_close_and_spawn_are_rejected() {
    let runtime_a = RuntimeHandle::create();
    let runtime_b = RuntimeHandle::create();
    let pool_a = TaskPool::create(runtime_a.clone(), 1).expect("pool A");
    let target_runtime = runtime_b.clone();
    let task = pool_a
        .submit(move |_| {
            let close_rejected = target_runtime.try_close().is_err();
            let spawn_rejected = spawn(&target_runtime, |_| 1_u32).is_err();
            (close_rejected, spawn_rejected)
        })
        .expect("跨 Runtime Runtime 负例任务");

    assert_eq!(task.wait(), Ok((true, true)));
    assert!(!runtime_b.is_closed());
    pool_a.shutdown().expect("pool A shutdown");
    runtime_a.close();
    runtime_b.close();
}

#[test]
/// Runtime 任务创建的子 Runtime 可在 owner 上下文中创建 pool 并执行任务。
fn task_owned_runtime_can_create_pool_and_spawn() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 1).expect("parent pool");
    let task = pool
        .submit(move |_| {
            let child = RuntimeHandle::create();
            let child_pool = TaskPool::create(child.clone(), 1).expect("child pool");
            let child_task = child_pool.submit(|_| 42_u32).expect("child task");
            let child_thread = spawn(&child, |_| 7_u32).expect("child thread");
            let values = (child_task.wait().expect("child wait"), child_thread.join());
            child.try_close().expect("child close");
            (values, child.is_closed())
        })
        .expect("owner task");

    assert_eq!(task.wait(), Ok(((42, Ok(7)), true)));
    pool.shutdown().expect("parent pool shutdown");
    runtime.close();
}

#[test]
/// 容量大于四时，唯一 close owner 仍能并行 drain 多个后续排队任务。
fn large_pool_close_drains_many_queued_tasks() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 8).expect("pool");
    let mut release_txs = Vec::new();
    let mut blocked_handles = Vec::new();
    for index in 0..3 {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        release_txs.push(release_tx);
        blocked_handles.push(
            pool.submit(move |_| {
                started_tx.send(index).unwrap();
                release_rx.recv().unwrap();
                index
            })
            .expect("阻塞 worker"),
        );
        assert_eq!(started_rx.recv_timeout(Duration::from_secs(2)), Ok(index));
    }

    let (closer_started_tx, closer_started_rx) = mpsc::channel();
    let (close_go_tx, close_go_rx) = mpsc::channel();
    let (close_returned_tx, close_returned_rx) = mpsc::channel();
    let closer_runtime = runtime.clone();
    let closer = pool
        .submit(move |_| {
            closer_started_tx.send(()).unwrap();
            close_go_rx.recv().unwrap();
            closer_runtime.close();
            close_returned_tx.send(()).unwrap();
            99_u32
        })
        .expect("close owner");
    closer_started_rx.recv().expect("close owner 已开始");

    let (queued_started_tx, queued_started_rx) = mpsc::channel();
    let mut queued = Vec::new();
    for index in 0..5_u32 {
        let queued_started_tx = queued_started_tx.clone();
        queued.push(
            pool.submit(move |_| {
                queued_started_tx.send(index).unwrap();
                index
            })
            .expect("排队任务"),
        );
    }
    drop(queued_started_tx);
    close_go_tx.send(()).unwrap();
    for release_tx in release_txs {
        release_tx.send(()).unwrap();
    }
    let mut started = Vec::new();
    for _ in 0..5 {
        started.push(
            queued_started_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("排队任务必须被 drain 槽执行"),
        );
    }
    started.sort_unstable();
    assert_eq!(started, vec![0, 1, 2, 3, 4]);
    close_returned_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("大容量 pool close 必须返回");
    for (index, task) in queued.into_iter().enumerate() {
        assert_eq!(task.wait(), Ok(index as u32));
    }
    for (index, task) in blocked_handles.into_iter().enumerate() {
        assert_eq!(task.wait(), Ok(index));
    }
    assert_eq!(closer.wait(), Ok(99));
}

#[test]
/// 同一 Runtime 的独立 pool shutdown 与 Runtime close 并发时保持可收敛。
fn runtime_close_and_independent_pool_shutdown_converge() {
    let runtime = RuntimeHandle::create();
    let pool_a = TaskPool::create(runtime.clone(), 1).expect("pool A");
    let pool_b = TaskPool::create(runtime.clone(), 1).expect("pool B");
    let (blocked_started_tx, blocked_started_rx) = mpsc::channel();
    let (blocked_release_tx, blocked_release_rx) = mpsc::channel();
    let blocked = pool_a
        .submit(move |_| {
            blocked_started_tx.send(()).unwrap();
            blocked_release_rx.recv().unwrap();
            1_u32
        })
        .expect("pool A task");
    blocked_started_rx.recv().expect("pool A task 已开始");

    let (shutdown_go_tx, shutdown_go_rx) = mpsc::channel();
    let (shutdown_done_tx, shutdown_done_rx) = mpsc::channel();
    let pool_b_for_task = pool_b.clone();
    let independent = pool_b
        .submit(move |_| {
            shutdown_go_rx.recv().unwrap();
            let result = pool_b_for_task.shutdown();
            shutdown_done_tx.send(result.clone()).unwrap();
            result
        })
        .expect("pool B task");

    let close_runtime = runtime.clone();
    let closer = thread::spawn(move || close_runtime.close());
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !runtime.is_closed() {
        assert!(std::time::Instant::now() < deadline, "Runtime close 未开始");
        thread::yield_now();
    }
    shutdown_go_tx.send(()).unwrap();
    blocked_release_tx.send(()).unwrap();

    let shutdown_result = shutdown_done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("独立 pool shutdown 必须返回");
    assert!(matches!(
        shutdown_result,
        Ok(()) | Err(RuntimeExecError::RuntimeClosed)
    ));
    assert_eq!(blocked.wait(), Ok(1));
    assert!(matches!(
        independent.wait(),
        Ok(Ok(())) | Ok(Err(RuntimeExecError::RuntimeClosed))
    ));
    closer.join().expect("Runtime close 完成");
}

#[test]
/// 外部 pool shutdown 已先接管 worker 后，运行中的 worker close 仍会 drain 排队任务。
fn external_pool_shutdown_and_worker_runtime_close_drain_queue() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
    let (running_started_tx, running_started_rx) = mpsc::channel();
    let (close_go_tx, close_go_rx) = mpsc::channel();
    let (close_returned_tx, close_returned_rx) = mpsc::channel();
    let worker_runtime = runtime.clone();
    let running = pool
        .submit(move |_| {
            running_started_tx.send(()).unwrap();
            close_go_rx.recv().unwrap();
            worker_runtime.close();
            close_returned_tx.send(()).unwrap();
            1_u32
        })
        .expect("运行任务");
    running_started_rx.recv().expect("运行任务已开始");

    let (queued_started_tx, queued_started_rx) = mpsc::channel();
    let queued = pool
        .submit(move |_| {
            queued_started_tx.send(()).unwrap();
            2_u32
        })
        .expect("排队任务");

    let shutdown_pool = pool.clone();
    let shutdown_done = thread::spawn(move || shutdown_pool.shutdown());
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !pool.inner.closed.load(Ordering::Acquire) {
        assert!(
            std::time::Instant::now() < deadline,
            "外部 shutdown 未停止池接纳"
        );
        thread::yield_now();
    }
    close_go_tx.send(()).unwrap();
    queued_started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("worker close 必须移交排队任务");
    close_returned_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("worker close 必须返回");
    assert_eq!(running.wait(), Ok(1));
    assert_eq!(queued.wait(), Ok(2));
    assert_eq!(
        shutdown_done.join().expect("外部 shutdown 完成"),
        Err(RuntimeExecError::RuntimeClosed)
    );
}

#[test]
/// 满队列提交可以由外部令牌取消，并返回 027。
fn full_queue_submission_has_cancellable_backpressure() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime, 1).expect("pool");
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let running = pool
        .submit(move |_| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            1_u32
        })
        .expect("running task");
    started_rx.recv().expect("worker 已开始");
    let queued = pool.submit(|_| 2_u32).expect("队列任务");

    let cancellation = RuntimeCancellation::default();
    let submit_cancellation = cancellation.clone();
    let submit_pool = pool.clone();
    let submitter =
        thread::spawn(move || submit_pool.submit_with_cancel(submit_cancellation, |_| 2_u32));
    thread::sleep(Duration::from_millis(10));
    cancellation.cancel();
    assert!(matches!(
        submitter.join().expect("提交线程完成"),
        Err(RuntimeExecError::Cancelled)
    ));
    release_tx.send(()).unwrap();
    assert_eq!(running.wait(), Ok(1));
    assert_eq!(queued.wait(), Ok(2));
}

#[test]
/// 普通 submit 从当前 worker 继承取消上下文，满队列不会永久阻塞。
fn default_submit_inherits_worker_cancellation_context() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime, 2).expect("pool");
    let (block_started_tx, block_started_rx) = mpsc::channel();
    let (block_release_tx, block_release_rx) = mpsc::channel();
    let blocker = pool
        .submit(move |_| {
            block_started_tx.send(()).unwrap();
            block_release_rx.recv().unwrap();
            1_u32
        })
        .expect("阻塞任务");
    block_started_rx.recv().expect("阻塞任务已开始");

    let (parent_started_tx, parent_started_rx) = mpsc::channel();
    let (parent_proceed_tx, parent_proceed_rx) = mpsc::channel();
    let parent_pool = pool.clone();
    let parent = pool
        .submit(move |_| {
            parent_started_tx.send(()).unwrap();
            parent_proceed_rx.recv().unwrap();
            let nested = parent_pool.submit(|_| 3_u32);
            assert!(matches!(nested, Err(RuntimeExecError::Cancelled)));
            2_u32
        })
        .expect("父任务");
    parent_started_rx.recv().expect("父任务已开始");

    let queued_a = pool.submit(|_| 4_u32).expect("填充队列 A");
    let queued_b = pool.submit(|_| 5_u32).expect("填充队列 B");
    parent.cancel();
    parent_proceed_tx.send(()).unwrap();
    assert_eq!(parent.wait(), Err(RuntimeExecError::Cancelled));
    block_release_tx.send(()).unwrap();
    assert_eq!(blocker.wait(), Ok(1));
    assert_eq!(queued_a.wait(), Ok(4));
    assert_eq!(queued_b.wait(), Ok(5));
}

#[test]
/// 无上下文入口线程的普通 submit 可由 Runtime 关闭终止满队列背压。
fn default_submit_from_external_thread_stops_on_runtime_close() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
    let (running_started_tx, running_started_rx) = mpsc::channel();
    let (running_release_tx, running_release_rx) = mpsc::channel();
    let running = pool
        .submit(move |_| {
            running_started_tx.send(()).unwrap();
            running_release_rx.recv().unwrap();
            1_u32
        })
        .expect("运行任务");
    running_started_rx.recv().expect("运行任务已开始");
    let queued = pool.submit(|_| 2_u32).expect("队列任务");

    let submit_pool = pool.clone();
    let submitter = thread::spawn(move || submit_pool.submit(|_| 3_u32));
    thread::sleep(Duration::from_millis(20));
    let close_runtime = runtime.clone();
    let closer = thread::spawn(move || close_runtime.close());
    for _ in 0..100 {
        if runtime.is_closed() {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(runtime.is_closed(), "关闭线程应先停止 Runtime 接纳");
    running_release_tx.send(()).unwrap();

    assert!(matches!(
        submitter.join().expect("外部提交线程完成"),
        Err(RuntimeExecError::RuntimeClosed)
    ));
    closer.join().expect("Runtime close 完成");
    assert_eq!(running.wait(), Ok(1));
    assert_eq!(queued.wait(), Ok(2));
}

#[test]
/// 排队任务取消后立即返回，并释放逻辑队列槽供后续提交使用。
fn queued_task_cancel_releases_slot_before_worker_is_free() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let blocker = pool
        .submit(move |_| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            1_u32
        })
        .expect("阻塞任务");
    started_rx.recv().expect("阻塞任务已开始");
    let queued = pool.submit(|_| 2_u32).expect("排队任务");
    queued.cancel();
    let started = std::time::Instant::now();
    assert_eq!(
        queued.wait_interruptible(|| false),
        Err(RuntimeExecError::Cancelled)
    );
    assert!(started.elapsed() < Duration::from_millis(100));
    assert_eq!(
        *runtime
            .inner
            .active
            .lock()
            .expect("Runtime 活动计数锁未中毒"),
        1,
        "取消排队任务应释放其 Runtime activity lease"
    );
    let replacement = pool.submit(|_| 3_u32).expect("取消后应能重新提交");
    assert_eq!(
        pool.inner
            .queue
            .state
            .lock()
            .expect("任务池队列锁未中毒")
            .messages
            .len(),
        1,
        "已取消任务必须从队列移除，只留下替代任务"
    );
    release_tx.send(()).unwrap();
    assert_eq!(blocker.wait(), Ok(1));
    assert_eq!(replacement.wait(), Ok(3));
    pool.shutdown().expect("pool shutdown");
}

#[test]
/// 公开 wait API 在令牌取消后即使任务体已返回仍返回 027。
fn completed_task_cancellation_is_observed_by_public_wait_apis() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 1).expect("pool");

    let waited = pool.submit(|_| 7_u32).expect("submit");
    thread::sleep(Duration::from_millis(10));
    waited.cancel();
    assert_eq!(waited.wait(), Err(RuntimeExecError::Cancelled));

    let interrupted = pool.submit(|_| 8_u32).expect("submit");
    thread::sleep(Duration::from_millis(10));
    interrupted.cancel();
    assert_eq!(
        interrupted.wait_interruptible(|| false),
        Err(RuntimeExecError::Cancelled)
    );
    pool.shutdown().expect("shutdown");
    runtime.close();
}

#[test]
/// 结果到达时才变为 true 的宿主中断仍覆盖已完成任务结果。
fn wait_interruptible_checks_host_interrupt_after_receiving_result() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
    let (release_tx, release_rx) = mpsc::channel();
    let task = pool
        .submit(move |_| {
            release_rx.recv().expect("host poll releases task");
            7_u32
        })
        .expect("task");
    let release_tx = Mutex::new(Some(release_tx));
    let polls = AtomicUsize::new(0);
    assert_eq!(
        task.wait_interruptible(|| {
            if polls.fetch_add(1, Ordering::AcqRel) == 0 {
                release_tx
                    .lock()
                    .expect("release sender lock")
                    .take()
                    .expect("release exactly once")
                    .send(())
                    .expect("release task");
                false
            } else {
                true
            }
        }),
        Err(RuntimeExecError::Cancelled)
    );
    pool.shutdown().expect("shutdown");
    runtime.close();
}

#[test]
/// 接收器断开时仍检查最后一次宿主中断，不能把 027 漏成 028。
fn wait_interruptible_checks_host_interrupt_on_disconnect() {
    let (sender, receiver) = mpsc::sync_channel::<Result<u32, RuntimeExecError>>(1);
    drop(sender);
    let cancellation = RuntimeCancellation::default();
    let task = TaskHandle {
        receiver,
        cancellation,
        cancel_hook: None,
        cancel_registration: None,
    };
    let polls = AtomicUsize::new(0);
    assert_eq!(
        task.wait_interruptible(|| polls.fetch_add(1, Ordering::AcqRel) != 0),
        Err(RuntimeExecError::Cancelled)
    );
}

#[test]
/// 任务 panic 转成 028，worker 保持存活并继续接收工作。
fn task_panic_is_reported_without_killing_pool_worker() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime, 1).expect("pool");
    let failed = pool
        .submit(|_| panic!("task panic regression"))
        .expect("submit");
    assert_eq!(failed.wait().unwrap_err().code(), "X06-RUNTIME-028");
    assert_eq!(pool.submit(|_| 42_u32).unwrap().wait(), Ok(42));
    pool.shutdown().expect("池应成功关闭");
}

#[test]
/// detached 线程 panic 进入进程级 028 通道。
fn detached_thread_panic_is_reported_to_process_channel() {
    let _test_lock = DETACHED_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("detached 测试锁未中毒");
    let _ = take_detached_errors();
    let runtime = RuntimeHandle::create();
    spawn(&runtime, |_| panic!("detached panic regression"))
        .expect("spawn")
        .detach();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let errors = loop {
        let errors = take_detached_errors();
        if !errors.is_empty() {
            break errors;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "detached 错误未进入通道"
        );
        thread::yield_now();
    };
    assert!(errors.iter().any(|error| error.code() == "X06-RUNTIME-028"));
    runtime.close();
}

#[test]
/// detached 取消进入 027，panic 进入 028；长任务不会阻塞后续错误收集。
fn detached_errors_are_classified_and_reaped_concurrently() {
    let _test_lock = DETACHED_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("detached 测试锁未中毒");
    let _ = take_detached_errors();
    let runtime = RuntimeHandle::create();
    let (release_tx, release_rx) = mpsc::channel();
    spawn(&runtime, move |_| {
        release_rx.recv().unwrap();
    })
    .expect("长 detached 任务")
    .detach();
    let cancelled = spawn(&runtime, |token| {
        while !token.is_cancelled() {
            thread::yield_now();
        }
        1_u32
    })
    .expect("可取消任务");
    cancelled.cancel();
    cancelled.detach();
    spawn(&runtime, |_| panic!("detached classification panic"))
        .expect("panic 任务")
        .detach();

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let mut errors = Vec::new();
    let errors = loop {
        errors.extend(take_detached_errors());
        if errors
            .iter()
            .any(|error| error.code() == CONCURRENCY_CANCELLED_CODE)
            && errors.iter().any(|error| error.code() == "X06-RUNTIME-028")
        {
            break errors;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "detached 错误未及时收集"
        );
        thread::yield_now();
    };
    assert!(
        errors
            .iter()
            .any(|error| error.code() == CONCURRENCY_CANCELLED_CODE)
    );
    assert!(errors.iter().any(|error| error.code() == "X06-RUNTIME-028"));
    release_tx.send(()).unwrap();
    runtime.close();
}

#[test]
/// Runtime 关闭拒绝新任务，并等待已经接纳的线程退出。
fn runtime_close_stops_admission_and_drains_threads() {
    let runtime = RuntimeHandle::create();
    let worker_runtime = runtime.clone();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = spawn(&runtime, move |_| {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
    })
    .expect("spawn");
    started_rx.recv().expect("线程开始");

    let (closed_tx, closed_rx) = mpsc::channel();
    let closer = thread::spawn(move || {
        worker_runtime.close();
        closed_tx.send(()).unwrap();
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !runtime.is_closed() {
        assert!(
            std::time::Instant::now() < deadline,
            "Runtime 未进入关闭状态"
        );
        thread::yield_now();
    }
    assert!(matches!(
        spawn(&runtime, |_| ()),
        Err(RuntimeExecError::RuntimeClosed)
    ));
    assert!(closed_rx.try_recv().is_err(), "关闭必须等待已接纳线程");
    release_tx.send(()).unwrap();
    assert_eq!(worker.join(), Ok(()));
    closed_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    closer.join().unwrap();
}

#[test]
/// 活动 Runtime 线程调用 close 时仍等待其他 activity，只豁免自身 lease。
fn active_thread_close_drains_other_threads_before_returning() {
    let runtime = RuntimeHandle::create();
    let (blocked_started_tx, blocked_started_rx) = mpsc::channel();
    let (blocked_release_tx, blocked_release_rx) = mpsc::channel();
    let blocked = spawn(&runtime, move |_| {
        blocked_started_tx.send(()).unwrap();
        blocked_release_rx.recv().unwrap();
    })
    .expect("blocked worker");
    blocked_started_rx.recv().expect("blocked worker started");

    let (close_started_tx, close_started_rx) = mpsc::channel();
    let (close_returned_tx, close_returned_rx) = mpsc::channel();
    let close_runtime = runtime.clone();
    let closer = spawn(&runtime, move |_| {
        close_started_tx.send(()).unwrap();
        close_runtime.close();
        close_returned_tx.send(()).unwrap();
    })
    .expect("active closer");
    close_started_rx.recv().expect("active closer started");
    assert!(close_returned_rx.try_recv().is_err(), "不能提前返回");
    blocked_release_tx.send(()).unwrap();
    assert_eq!(blocked.join(), Ok(()));
    assert_eq!(closer.join(), Ok(()));
    assert!(runtime.is_closed());
}

#[test]
/// 活动 follower 可快速返回，但唯一 owner 仍等待未进入关闭路径的 activity。
fn active_close_follower_leaves_owner_to_drain_other_activity() {
    let runtime = RuntimeHandle::create();
    let (blocked_started_tx, blocked_started_rx) = mpsc::channel();
    let (blocked_release_tx, blocked_release_rx) = mpsc::channel();
    let blocked = spawn(&runtime, move |_| {
        blocked_started_tx.send(()).unwrap();
        blocked_release_rx.recv().unwrap();
    })
    .expect("blocked worker");
    blocked_started_rx.recv().expect("blocked worker started");

    let (follower_started_tx, follower_started_rx) = mpsc::channel();
    let (follower_go_tx, follower_go_rx) = mpsc::channel();
    let (follower_returned_tx, follower_returned_rx) = mpsc::channel();
    let (follower_release_tx, follower_release_rx) = mpsc::channel();
    let follower_runtime = runtime.clone();
    let follower = spawn(&runtime, move |_| {
        follower_started_tx.send(()).unwrap();
        follower_go_rx.recv().unwrap();
        follower_runtime.close();
        follower_returned_tx.send(()).unwrap();
        follower_release_rx.recv().unwrap();
    })
    .expect("active follower");
    follower_started_rx.recv().expect("active follower started");

    let owner_runtime = runtime.clone();
    let owner_done = thread::spawn(move || owner_runtime.close());
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !runtime.is_closed() {
        assert!(
            std::time::Instant::now() < deadline,
            "Runtime owner 未开始 drain"
        );
        thread::yield_now();
    }
    follower_go_tx.send(()).unwrap();
    follower_returned_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("活动 follower 应在 owner drain 期间返回");
    assert!(!owner_done.is_finished(), "owner 不得绕过普通 activity");

    follower_release_tx.send(()).unwrap();
    blocked_release_tx.send(()).unwrap();
    owner_done.join().expect("Runtime owner 完成");
    assert_eq!(blocked.join(), Ok(()));
    assert_eq!(follower.join(), Ok(()));
}

#[test]
/// 活动 pool worker close 时也要等待尚未开始但已接纳的排队 activity。
fn active_close_waits_for_queued_pool_activity() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 2).expect("pool");
    let (blocked_started_tx, blocked_started_rx) = mpsc::channel();
    let (blocked_release_tx, blocked_release_rx) = mpsc::channel();
    let blocked = pool
        .submit(move |_| {
            blocked_started_tx.send(()).unwrap();
            blocked_release_rx.recv().unwrap();
            1_u32
        })
        .expect("阻塞任务");
    blocked_started_rx.recv().expect("阻塞任务已开始");

    let (closer_started_tx, closer_started_rx) = mpsc::channel();
    let (close_go_tx, close_go_rx) = mpsc::channel();
    let (close_returned_tx, close_returned_rx) = mpsc::channel();
    let closer_runtime = runtime.clone();
    let closer = pool
        .submit(move |_| {
            closer_started_tx.send(()).unwrap();
            close_go_rx.recv().unwrap();
            closer_runtime.close();
            close_returned_tx.send(()).unwrap();
            2_u32
        })
        .expect("Runtime closer");
    closer_started_rx.recv().expect("Runtime closer 已开始");

    let (queued_started_tx, queued_started_rx) = mpsc::channel();
    let (queued_release_tx, queued_release_rx) = mpsc::channel();
    let queued = pool
        .submit(move |_| {
            queued_started_tx.send(()).unwrap();
            queued_release_rx.recv().unwrap();
            3_u32
        })
        .expect("排队任务");
    close_go_tx.send(()).unwrap();
    assert!(
        close_returned_rx
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "close 不得绕过排队 activity"
    );

    blocked_release_tx.send(()).unwrap();
    queued_started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("释放 worker 后排队任务必须运行");
    assert!(
        close_returned_rx
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "排队任务仍在运行时 close 不得返回"
    );
    queued_release_tx.send(()).unwrap();

    close_returned_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("排队 activity 完成后 close 必须返回");
    assert_eq!(blocked.wait(), Ok(1));
    assert_eq!(queued.wait(), Ok(3));
    assert_eq!(closer.wait(), Ok(2));
    assert!(runtime.is_closed());
}

#[test]
/// 单 worker 也必须在 close 期间移交排队任务，避免 active 总数等待自锁。
fn active_close_drains_queued_activity_with_single_worker() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
    let (closer_started_tx, closer_started_rx) = mpsc::channel();
    let (close_go_tx, close_go_rx) = mpsc::channel();
    let (close_returned_tx, close_returned_rx) = mpsc::channel();
    let closer_runtime = runtime.clone();
    let closer = pool
        .submit(move |_| {
            closer_started_tx.send(()).unwrap();
            close_go_rx.recv().unwrap();
            closer_runtime.close();
            close_returned_tx.send(()).unwrap();
            1_u32
        })
        .expect("Runtime closer");
    closer_started_rx.recv().expect("Runtime closer 已开始");

    let (queued_started_tx, queued_started_rx) = mpsc::channel();
    let (queued_release_tx, queued_release_rx) = mpsc::channel();
    let queued = pool
        .submit(move |_| {
            queued_started_tx.send(()).unwrap();
            queued_release_rx.recv().unwrap();
            2_u32
        })
        .expect("排队任务");
    close_go_tx.send(()).unwrap();
    queued_started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("single worker close 必须移交排队任务");
    assert!(close_returned_rx.try_recv().is_err());
    queued_release_tx.send(()).unwrap();
    close_returned_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("排队任务完成后 close 必须返回");
    assert_eq!(queued.wait(), Ok(2));
    assert_eq!(closer.wait(), Ok(1));
}

#[test]
/// 当前 worker 的 drain helper 执行排队 close 时也不能重新形成关闭环。
fn shutdown_drain_helper_handles_queued_runtime_close() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 2).expect("pool");
    let (blocked_started_tx, blocked_started_rx) = mpsc::channel();
    let (blocked_release_tx, blocked_release_rx) = mpsc::channel();
    let blocked = pool
        .submit(move |_| {
            blocked_started_tx.send(()).unwrap();
            blocked_release_rx.recv().unwrap();
            0_u32
        })
        .expect("阻塞 worker");
    blocked_started_rx.recv().expect("阻塞 worker 已开始");

    let (closer_started_tx, closer_started_rx) = mpsc::channel();
    let (close_go_tx, close_go_rx) = mpsc::channel();
    let (close_returned_tx, close_returned_rx) = mpsc::channel();
    let closer_runtime = runtime.clone();
    let closer = pool
        .submit(move |_| {
            closer_started_tx.send(()).unwrap();
            close_go_rx.recv().unwrap();
            closer_runtime.close();
            close_returned_tx.send(()).unwrap();
            1_u32
        })
        .expect("Runtime closer");
    closer_started_rx.recv().expect("Runtime closer 已开始");

    let (queued_started_tx, queued_started_rx) = mpsc::channel();
    let (followup_result_tx, followup_result_rx) = mpsc::channel();
    let queued_runtime = runtime.clone();
    let queued = pool
        .submit(move |_| {
            queued_runtime.close();
            queued_started_tx.send(()).unwrap();
            followup_result_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("排队 close 任务必须取得后续任务结果");
            2_u32
        })
        .expect("排队 close 任务");
    let followup = pool
        .submit(move |_| {
            followup_result_tx.send(9_u32).unwrap();
            3_u32
        })
        .expect("排队后续任务");
    close_go_tx.send(()).unwrap();
    queued_started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("drain helper 必须执行排队 close 任务");
    assert_eq!(followup.wait(), Ok(3));
    assert_eq!(queued.wait(), Ok(2));
    blocked_release_tx.send(()).unwrap();
    close_returned_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("主 worker close 必须返回");
    assert_eq!(blocked.wait(), Ok(0));
    assert_eq!(closer.wait(), Ok(1));
    assert!(runtime.is_closed());
}

#[test]
/// detached 回收池忙碌时，扩展队列策略仍让 detach 发送立即返回。
fn detached_reaper_queue_does_not_block_when_workers_are_busy() {
    let _test_lock = DETACHED_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("detached 测试锁未中毒");
    let (started_tx, started_rx) = mpsc::channel();
    let mut release_txs = Vec::new();
    for _ in 0..4 {
        let started_tx = started_tx.clone();
        let (release_tx, release_rx) = mpsc::channel();
        release_txs.push(release_tx);
        enqueue_detached_reap(Box::new(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        }))
        .expect("发送 detached 回收任务");
    }
    drop(started_tx);
    for _ in 0..4 {
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("四个 detached worker 必须占用回收槽");
    }

    let baseline_depth = detached_reaper_queue_depth();
    let start = std::time::Instant::now();
    for _ in 0..8 {
        for _ in 0..512 {
            enqueue_detached_reap(Box::new(|| {})).expect("无界回收队列必须可继续接收");
        }
    }
    assert!(
        detached_reaper_queue_depth() >= baseline_depth + 4096,
        "持续积压必须可观测"
    );
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "回收队列满时发送不应阻塞调用方"
    );
    for release_tx in release_txs {
        release_tx.send(()).unwrap();
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while detached_reaper_queue_depth() > baseline_depth {
        assert!(
            std::time::Instant::now() < deadline,
            "detached 回收队列积压未能清空"
        );
        thread::yield_now();
    }
}

#[test]
/// Runtime 关闭会关闭所属任务池并 drain 已提交任务。
fn runtime_close_shuts_down_and_drains_owned_pools() {
    let runtime = RuntimeHandle::create();
    let pool = TaskPool::create(runtime.clone(), 1).expect("pool");
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let task = pool
        .submit(move |_| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            7
        })
        .expect("submit");
    started_rx.recv().expect("任务开始");
    let worker_runtime = runtime.clone();
    let closer = thread::spawn(move || worker_runtime.close());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !runtime.is_closed() {
        assert!(std::time::Instant::now() < deadline, "Runtime 未关闭");
        thread::yield_now();
    }
    release_tx.send(()).unwrap();
    assert_eq!(task.wait(), Ok(7));
    closer.join().unwrap();
    assert!(matches!(
        pool.submit(|_| 8),
        Err(RuntimeExecError::RuntimeClosed)
    ));
}
