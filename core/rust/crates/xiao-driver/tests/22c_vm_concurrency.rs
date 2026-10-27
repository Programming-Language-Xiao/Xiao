//! 22C-FIX.1：真实源码、TAC、产物和隔离 VM 的并发结果/错误/清理回归。

use std::sync::mpsc;
use std::time::Duration;
use xiao_driver::{DriverRequest, FrontendCompiler, FrontendRequest};
use xiao_vm::{VmEvent, VmOptions};

/// 记录一段真实源码运行产生的状态、错误和 VM 事件。
#[derive(Debug)]
struct Observation {
    success: bool,
    code: Option<String>,
    output: String,
    report: Option<xiao_diagnostics::ReportRecord>,
    events: Vec<VmEvent>,
}

/// 在线程保护下运行源码，以便超时回归能收回失控执行。
fn observe(source: &str) -> Observation {
    let source = source.to_owned();
    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let request = DriverRequest::new(FrontendRequest::from_text(source))
            .with_timeout(Duration::from_secs(3));
        let outcome = xiao_driver::run(&request);
        if matches!(outcome, xiao_driver::DriverOutcome::Frontend(_)) {
            eprintln!("{outcome:?}");
        }
        let output = outcome
            .as_executed()
            .map(|execution| outputs(execution.events()))
            .unwrap_or_default();
        let events = outcome
            .as_executed()
            .map(|execution| execution.events().to_vec())
            .unwrap_or_default();
        tx.send(Observation {
            success: outcome.is_success(),
            code: outcome.code().map(str::to_owned),
            output,
            report: outcome.report().cloned(),
            events,
        })
        .unwrap();
    });
    let observed = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("VM 并发执行必须在超时内返回");
    worker.join().unwrap();
    observed
}

/// 拼接结构化 VM 输出事件，供源码场景断言。
fn outputs(events: &[VmEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// 读取本次 VM 事件或有界 Runtime 错误通道中的 detached 报告。
fn detached_report(result: &Observation, operation: &str) -> (u64, xiao_diagnostics::ReportRecord) {
    if let Some((runtime_id, report)) = result.events.iter().find_map(|event| match event {
        VmEvent::DetachedFailure { runtime_id, report } => Some((*runtime_id, report.clone())),
        _ => None,
    }) {
        return (runtime_id, report);
    }
    let runtime_id = result
        .events
        .iter()
        .find_map(|event| match event {
            VmEvent::ConcurrencyObserved {
                operation: observed,
                runtime_id,
                ..
            } if observed == operation => Some(*runtime_id),
            _ => None,
        })
        .expect("detached 操作應保留 Runtime 身份");
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(report) = xiao_runtime::take_vm_detached_errors_for(&[runtime_id])
            .into_iter()
            .next()
        {
            return (
                report.runtime_id,
                xiao_diagnostics::ReportRecord::from_error(&report.error),
            );
        }
        assert!(std::time::Instant::now() < deadline, "detached 报告未到达");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// 验证 Runtime、spawn/join、任务池和跨边界值均可从源码执行。
#[test]
fn source_threads_pool_aliases_and_values_are_observable() {
    let result = observe(
        r#"from std.concurrent import current_runtime, runtime_create, spawn as launch, join, pool_create, submit, wait, shutdown, runtime_close
def work(int number, str text) -> str
    return text + (number as str)
root = current_runtime()
other = runtime_create()
t = launch(other, work, 7, "thread:")
pool = pool_create(root, 2)
job = submit(pool, work, 9, "task:")
print(join(t))
print(wait(job))
shutdown(pool)
runtime_close(other)
"#,
    );
    assert!(result.success, "{result:?}");
    assert_eq!(result.output, "thread:7\ntask:9\n");
}

/// 验证 child current Runtime 中共享可变值和 guard 别名正确释放。
#[test]
fn child_current_runtime_preserves_shared_mut_and_guard_release() {
    let result = observe(
        r#"from std.concurrent import current_runtime, shared_mut, spawn, join, lock, lock_read, lock_write
def add(shared_mut<int> counter) -> int
    guard = lock(counter)
    old = lock_read(guard)
    lock_write(guard, old + 1)
    return 1
rt = current_runtime()
counter = shared_mut(0)
other_counter = counter
first = spawn(rt, add, counter)
second = spawn(rt, add, other_counter)
join(first)
join(second)
guard = lock(counter)
print(lock_read(guard))
"#,
    );
    assert!(result.success, "{result:?}");
    assert_eq!(result.output, "2\n");
}

/// 验证只读 shared 可跨 Runtime，并在来源 Runtime 关闭后继续读取。
#[test]
fn shared_read_can_cross_runtime_and_survive_source_close() {
    let result = observe(
        r#"from std.concurrent import current_runtime, runtime_create, runtime_close, shared_read, spawn, join, read
def work(shared<str> value) -> str
    return read(value)
root = current_runtime()
value = shared_read("readable")
other = runtime_create()
t = spawn(other, work, value)
print(join(t))
runtime_close(root)
print(read(value))
runtime_close(other)
"#,
    );
    assert!(result.success, "{result:?}");
    assert_eq!(result.output, "readable\nreadable\n");
}

/// 验证 join/wait 返回 child 主错误并保留 finally 的 suppressed 错误。
#[test]
fn child_error_and_suppressed_survive_join_and_wait() {
    for (make, collect, extra) in [
        ("spawn(rt, work)", "join(task)", ""),
        (
            "submit(pool, work)",
            "wait(task)",
            "pool = pool_create(rt, 1)\n",
        ),
    ] {
        let source = format!(
            "from std.concurrent import current_runtime, spawn, join, pool_create, submit, wait\ndef work() -> int\n    try\n        raise TypeError(code = \"child-primary\")\n    finally\n        print(\"cleanup\")\n        raise TypeError(code = \"child-cleanup\")\n    return 0\nrt = current_runtime()\n{extra}task = {make}\n{collect}\n"
        );
        let result = observe(&source);
        assert_eq!(result.code.as_deref(), Some("child-primary"), "{result:?}");
        assert_eq!(result.output, "cleanup\n");
        let report = result.report.unwrap();
        assert_eq!(report.suppressed.len(), 1);
        assert_eq!(report.suppressed[0].code, "child-cleanup");
    }
}

/// 验证显式取消运行中 child 后 finally 恰好执行一次。
#[test]
fn cancellation_of_running_child_executes_finally_once() {
    let result = observe(
        r#"from std.concurrent import current_runtime, shared_mut, spawn, join, cancel, lock, lock_read, lock_write
def ready(shared_mut<int> flag) -> int
    guard = lock(flag)
    return lock_read(guard)
def signal(shared_mut<int> flag) -> none
    guard = lock(flag)
    lock_write(guard, 1)
def work(shared_mut<int> flag) -> int
    try
        signal(flag)
        while true
            value = 1
    finally
        print("finally-once")
    return 0
rt = current_runtime()
flag = shared_mut(0)
task = spawn(rt, work, flag)
while ready(flag) == 0
    value = 1
cancel(task)
join(task)
"#,
    );
    assert_eq!(
        result.code.as_deref(),
        Some("X06-RUNTIME-027"),
        "{result:?}"
    );
    assert_eq!(result.output, "finally-once\n");
}

/// 验证并发 027 不会进入用户 catch，也不会让取消后的 child 继续运行。
#[test]
fn concurrency_cancel_cannot_be_swallowed_by_catch() {
    let result = observe(
        r#"from std.concurrent import current_runtime, shared_mut, spawn, join, cancel, lock, lock_read, lock_write
def ready(shared_mut<int> flag) -> int
    guard = lock(flag)
    return lock_read(guard)
def signal(shared_mut<int> flag) -> none
    guard = lock(flag)
    lock_write(guard, 1)
def work(shared_mut<int> flag) -> int
    signal(flag)
    try
        while true
            value = 1
    catch err as Error
        print("caught")
        while true
            value = 2
    return 0
rt = current_runtime()
flag = shared_mut(0)
task = spawn(rt, work, flag)
while ready(flag) == 0
    value = 1
cancel(task)
join(task)
"#,
    );
    assert_eq!(
        result.code.as_deref(),
        Some("X06-RUNTIME-027"),
        "{result:?}"
    );
    assert!(!result.output.contains("caught"));
}

/// 验证外部超时能够中断 join 和子 VM 热循环。
#[test]
fn external_timeout_interrupts_join_and_child_vm() {
    let result = observe(
        r#"from std.concurrent import current_runtime, spawn, join
def work() -> int
    try
        while true
            value = 1
    finally
        print("timed-finally")
    return 0
rt = current_runtime()
task = spawn(rt, work)
join(task)
"#,
    );
    assert_eq!(
        result.code.as_deref(),
        Some(xiao_driver::DRIVER_TIMEOUT_CODE),
        "{result:?}"
    );
}

/// 验证关闭、池参数和 Runtime 上下文错误保持冻结编号。
#[test]
fn frozen_runtime_error_codes_remain_structured() {
    for (body, code) in [
        ("pool = pool_create(rt, 0)\n", "X06-RUNTIME-032"),
        (
            "pool = pool_create(rt, 1)\nshutdown(pool)\nt = submit(pool, work)\n",
            "X06-RUNTIME-030",
        ),
        (
            "runtime_close(rt)\nt = spawn(rt, work)\n",
            "X06-RUNTIME-031",
        ),
    ] {
        let source = format!(
            "from std.concurrent import current_runtime, runtime_close, spawn, pool_create, shutdown, submit\ndef work() -> int\n    return 1\nrt = current_runtime()\n{body}"
        );
        let result = observe(&source);
        assert_eq!(result.code.as_deref(), Some(code), "{result:?}");
    }
}

/// 验证 TAC 计划可往返编码，VM 执行不依赖成员名称。
#[test]
fn plans_round_trip_and_execute_without_module_function_names() {
    let source = "from std.concurrent import current_runtime, spawn, join\ndef work(int value) -> int\n    return value + 1\nrt = current_runtime()\nt = spawn(rt, work, 41)\nprint(join(t))\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap();
    let mut program = xiao_bytecode::lower_program(&artifact.ir);
    assert_eq!(program.concurrency_plans.len(), 3);
    for instruction in program
        .functions
        .iter_mut()
        .flat_map(|f| &mut f.blocks)
        .flat_map(|b| &mut b.instructions)
    {
        if let xiao_bytecode::TacOp::MemberGet { member, .. } = &mut instruction.op {
            *member = "ascii:unrelated".to_owned();
        }
    }
    let bytes =
        xiao_bytecode::encode_xiaoc(&program, xiao_bytecode::XiaocMetadata::new("main")).unwrap();
    let decoded = xiao_bytecode::decode_xiaoc(&bytes).unwrap();
    assert_eq!(decoded.program.concurrency_plans, program.concurrency_plans);
    let result = xiao_vm::run_xiaoc(&bytes, VmOptions::default()).unwrap();
    assert!(result.result.is_success(), "{result:?}");
    assert_eq!(outputs(&result.events), "42\n");
}

/// 验证真实源码构建的 `.xar` 在 feature 预检后执行。
#[test]
fn real_source_builds_and_runs_xar_with_preflight_features() {
    let root = std::env::temp_dir().join(format!("xiao-22c-vm-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let input = root.join("main.xiao");
    std::fs::write(&input, "from std.concurrent import runtime_create, spawn, join, runtime_close\ndef work() -> int\n    return 42\nrt = runtime_create()\nt = spawn(rt, work)\nprint(join(t))\nruntime_close(rt)\n").unwrap();
    let mut request = xiao_driver::ArchiveBuildRequest::new(&input);
    request.output = Some(root.join("main.xar"));
    request.cache_root = Some(root.join("cache"));
    let archive =
        xiao_driver::build_archive(&request, &xiao_driver::CancellationToken::new()).unwrap();
    let bytes = std::fs::read(&archive.output).unwrap();
    let result = xiao_xar::run_archive(&bytes, Default::default()).unwrap();
    assert!(result.result.is_success(), "{result:?}");
    assert_eq!(outputs(&result.events), "42\n");
    let options = xiao_xar::XarRunOptions {
        runtime_features: 1,
        ..Default::default()
    };
    assert_eq!(
        xiao_xar::run_archive(&bytes, options).unwrap_err().code(),
        "X17-XAR-007"
    );
}

/// 验证排队任务取消后 wait 不等 worker 空闲，并立即释放队列容量。
#[test]
fn queued_task_cancel_never_runs_body_and_wait_is_027() {
    let source =
        r#"from std.concurrent import current_runtime, shared_mut, spawn, pool_create, submit, wait, cancel, lock, lock_read, lock_write
def read_flag(shared_mut<int> flag) -> int
    guard = lock(flag)
    return lock_read(guard)
def write_flag(shared_mut<int> flag, int value) -> none
    guard = lock(flag)
    lock_write(guard, value)
def release_after_wait(shared_mut<int> ready, shared_mut<int> gate) -> int
    while read_flag(ready) == 0
        value = 1
    write_flag(gate, 1)
    return 0
def blocker(shared_mut<int> flag) -> int
    while read_flag(flag) == 0
        value = 1
    return 1
def queued(int value, str text) -> int
    print("must-not-run")
    return value
rt = current_runtime()
ready = shared_mut(0)
gate = shared_mut(0)
release_gate = gate
pool = pool_create(rt, 1)
release = spawn(rt, release_after_wait, ready, release_gate)
first = submit(pool, blocker, gate)
second = submit(pool, queued)
cancel(second)
try
    wait(second)
finally
    write_flag(ready, 1)
"#
        .replace("second = submit(pool, queued)", "second = submit(pool, queued, 2, \"payload\")");
    let request = DriverRequest::new(FrontendRequest::from_text(source.clone()));
    let artifact = FrontendCompiler::new().compile(&request.frontend).unwrap();
    let transfer = artifact
        .ir
        .ownership
        .boundary_transfers
        .iter()
        .rfind(|transfer| transfer.parameters.len() == 2)
        .expect("排队任务应包含两个边界参数");
    let expected_cleanup = transfer.cleanup_order.clone();
    let result = observe(&source);
    assert_eq!(
        result.code.as_deref(),
        Some("X06-RUNTIME-027"),
        "{result:?}"
    );
    assert!(!result.output.contains("must-not-run"));
    let released = result
        .events
        .iter()
        .filter_map(|event| match event {
            VmEvent::BoundaryCopyReleased { value } if expected_cleanup.contains(value) => {
                Some(*value)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(released, expected_cleanup);
}

/// 验证锁等待和已完成句柄取消仍遵循 027 语义。
#[test]
fn lock_wait_and_completed_handle_cancellation_are_027() {
    let result = observe(
        r#"from std.concurrent import current_runtime, shared_mut, spawn, join, cancel, lock, lock_read
def work(shared_mut<int> value) -> int
    try
        guard = lock(value)
        return lock_read(guard)
    finally
        print("lock-finally")
rt = current_runtime()
value = shared_mut(1)
guard = lock(value)
t = spawn(rt, work, value)
cancel(t)
join(t)
"#,
    );
    assert_eq!(
        result.code.as_deref(),
        Some("X06-RUNTIME-027"),
        "{result:?}"
    );
    assert!(result.output.is_empty() || result.output == "lock-finally\n");
    let completed = observe(
        r#"from std.concurrent import current_runtime, spawn, join, cancel, runtime_close
def work() -> int
    return 3
rt = current_runtime()
t = spawn(rt, work)
runtime_close(rt)
cancel(t)
join(t)
"#,
    );
    assert_eq!(
        completed.code.as_deref(),
        Some("X06-RUNTIME-027"),
        "{completed:?}"
    );
}

/// 验证取消清理失败进入 suppressed，且主错误保持为首要错误。
#[test]
fn cancellation_cleanup_error_stays_suppressed() {
    let source = r#"from std.concurrent import current_runtime, shared_mut, spawn, join, cancel, lock, lock_read, lock_write
def ready(shared_mut<int> flag) -> int
    guard = lock(flag)
    return lock_read(guard)
def signal(shared_mut<int> flag) -> none
    guard = lock(flag)
    lock_write(guard, 1)
def work(shared_mut<int> flag) -> int
    try
        signal(flag)
        while true
            value = 1
    finally
        raise TypeError(code = "cancel-cleanup")
    return 0
rt = current_runtime()
flag = shared_mut(0)
t = spawn(rt, work, flag)
while ready(flag) == 0
    value = 1
cancel(t)
join(t)
"#.to_owned();
    let result = observe(&source);
    assert_eq!(
        result.code.as_deref(),
        Some("X06-RUNTIME-027"),
        "{result:?}"
    );
    assert_eq!(result.report.unwrap().suppressed[0].code, "cancel-cleanup");
}

/// 验证子 Runtime 的实际身份隔离及其任务池执行。
#[test]
fn nested_runtime_context_is_distinct_and_child_pool_works() {
    let result = observe(
        r#"from std.concurrent import current_runtime, runtime_create, runtime_close, pool_create, submit, wait, spawn, join
def leaf() -> int
    return 42
def work() -> int
    child = runtime_create()
    pool = pool_create(child, 1)
    job = submit(pool, leaf)
    value = wait(job)
    runtime_close(child)
    return value
root = current_runtime()
t = spawn(root, work)
print(join(t))
"#,
    );
    assert!(result.success, "{result:?}");
    assert_eq!(result.output, "42\n");
    let ids = result
        .events
        .iter()
        .filter_map(|event| match event {
            VmEvent::ConcurrencyObserved {
                operation,
                runtime_id,
                ..
            } if operation == "current_runtime" || operation == "runtime_create" => {
                Some(*runtime_id)
            }
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 2);
}

/// 验证不支持的可变容器结果以 029 拒绝，不伪装成 none。
#[test]
fn result_serialization_rejects_mutable_container_instead_of_none() {
    let result = observe(
        r#"from std.concurrent import current_runtime, spawn, join
def work() -> [int]
    return [1, 2]
rt = current_runtime()
t = spawn(rt, work)
join(t)
"#,
    );
    assert_eq!(
        result.code.as_deref(),
        Some("X06-RUNTIME-029"),
        "{result:?}"
    );
}

/// 验证边界复制失败时按 IR 顺序清理，且不启动 child。
#[test]
fn boundary_failure_cleans_copies_in_ir_order_before_starting_child() {
    let mut source = String::from(
        "from std.concurrent import current_runtime, spawn, join\ndef work(str a, str b, str c) -> int\n    print(\"must-not-start\")\n    return 1\nrt = current_runtime()\nt = spawn(rt, work, \"a\", \"b\", \"",
    );
    source.push_str(&"x".repeat(16 * 1024 * 1024 + 1));
    source.push_str("\")\njoin(t)\n");
    // 巨型字面量用于边界限制，不让驱动器的常规 3 秒编译截止时间遮蔽 029。
    let request = DriverRequest::new(FrontendRequest::from_text(source));
    let artifact = FrontendCompiler::new().compile(&request.frontend).unwrap();
    let cleanup = &artifact.ir.ownership.boundary_transfers[0].cleanup_order;
    let outcome = xiao_driver::FrontendVmDriver::new().run_artifact(&artifact, &request);
    assert_eq!(outcome.code(), Some("X06-RUNTIME-029"), "{outcome:?}");
    let events = outcome.as_executed().unwrap().events();
    assert!(outputs(events).is_empty());
    let released = events
        .iter()
        .filter_map(|event| match event {
            VmEvent::BoundaryCopyReleased { value } => Some(*value),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(released, cleanup[1..]);
}

/// 验证边界复制成功但任务启动被拒绝时仍按 IR 顺序释放副本。
#[test]
fn startup_failure_cleans_successful_boundary_copies() {
    let source = "from std.concurrent import current_runtime, pool_create, submit, shutdown\ndef work(int value, str text) -> int\n    return value\nrt = current_runtime()\npool = pool_create(rt, 1)\nshutdown(pool)\nsubmit(pool, work, 1, \"x\")\n";
    let request = DriverRequest::new(FrontendRequest::from_text(source));
    let artifact = FrontendCompiler::new().compile(&request.frontend).unwrap();
    let cleanup = artifact.ir.ownership.boundary_transfers[0]
        .cleanup_order
        .clone();
    let result = observe(source);
    assert_eq!(
        result.code.as_deref(),
        Some("X06-RUNTIME-030"),
        "{result:?}"
    );
    let released = result
        .events
        .iter()
        .filter_map(|event| match event {
            VmEvent::BoundaryCopyReleased { value } => Some(*value),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(released, cleanup);
}

/// 验证损坏的并发计划在进入 VM 前被 verifier 拒绝。
#[test]
fn corrupt_plans_are_rejected_before_vm_execution() {
    let source = "from std.concurrent import current_runtime, spawn, join\ndef work(int value) -> int\n    return value\nrt = current_runtime()\nt = spawn(rt, work, 7)\njoin(t)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap();
    let base = xiao_bytecode::lower_program(&artifact.ir);
    for mutate in 0..7 {
        let mut program = base.clone();
        let plan = program
            .concurrency_plans
            .iter_mut()
            .find(|plan| plan.operation == "spawn")
            .unwrap();
        match mutate {
            0 => plan.operation = "invented".to_owned(),
            1 => plan.runtime_id = Some(0),
            2 => plan.transfer.as_mut().unwrap().cleanup_order.clear(),
            3 => plan.cancellation_checks.clear(),
            4 => plan.facts[0].thread_boundary = false,
            5 => plan.argument_types.clear(),
            _ => program.concurrency_plans.clear(),
        }
        assert!(
            xiao_bytecode::encode_xiaoc(&program, Default::default()).is_err(),
            "篡改 {mutate} 未拒绝"
        );
    }
}

/// 验证显式 detach 与未 wait 任务失败都以结构化报告保留且不改变主退出码。
#[test]
fn detached_failure_is_structured_and_does_not_change_exit_code() {
    let _ = xiao_runtime::take_vm_detached_errors();
    let result = observe(
        r#"from std.concurrent import current_runtime, spawn, detach, runtime_close
def fail() -> int
    raise TypeError(code = "detached-original")
    return 0
rt = current_runtime()
t = spawn(rt, fail)
detach(t)
runtime_close(rt)
print("continued")
"#,
    );
    assert!(result.success, "{result:?}");
    assert_eq!(result.output, "continued\n");
    assert!(result.events.iter().any(|event| matches!(event, VmEvent::ConcurrencyObserved {operation, ..} if operation == "detach")));
    let (runtime_id, report) = detached_report(&result, "detach");
    assert_ne!(runtime_id, 0);
    assert_eq!(report.code, "X06-RUNTIME-028");
    assert_eq!(report.cause.as_ref().unwrap().code, "detached-original");
}

/// 验证未等待任务在 VM 退出回收后也进入本次运行的结构化事件。
#[test]
fn dropped_unwaited_task_failure_is_reported_before_vm_returns() {
    let result = observe(
        r#"from std.concurrent import current_runtime, pool_create, submit
def fail() -> int
    raise TypeError(code = "unwaited-original")
    return 0
rt = current_runtime()
pool = pool_create(rt, 1)
job = submit(pool, fail)
print("done")
"#,
    );
    assert!(result.success, "{result:?}");
    assert_eq!(result.output, "done\n");
    let (runtime_id, report) = detached_report(&result, "submit");
    assert_ne!(runtime_id, 0);
    assert_eq!(report.code, "X06-RUNTIME-028");
    assert_eq!(report.cause.as_ref().unwrap().code, "unwaited-original");
}

/// 22A 关闭语义要求 VM 退出前 drain detached 活动；外部截止时间仍能收束无限 child。
#[test]
fn detached_activity_is_drained_before_timeout_returns() {
    let result = observe(
        r#"from std.concurrent import current_runtime, spawn, detach
def work() -> int
    while true
        value = 1
    return 0
rt = current_runtime()
task = spawn(rt, work)
detach(task)
print("done")
"#,
    );
    assert_eq!(
        result.code.as_deref(),
        Some(xiao_driver::DRIVER_TIMEOUT_CODE),
        "{result:?}"
    );
}

/// 验证隔离 child 可以载入只读项目模块和标准模块快照。
#[test]
fn isolated_child_loads_project_and_standard_modules() {
    let root = std::env::temp_dir().join(format!("xiao-22c-module-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("helper.xiao"), "def work(int value) -> str\n    from stdlib.text import lower_ascii\n    return lower_ascii(\"OK\") + (value as str)\n").unwrap();
    let input = root.join("main.xiao");
    std::fs::write(&input, "from std.concurrent import current_runtime, spawn, join\ndef entry(int value) -> str\n    from helper import work\n    return work(value)\nrt = current_runtime()\nt = spawn(rt, entry, 7)\nprint(join(t))\n").unwrap();
    let mut frontend = FrontendRequest::from_file(&input).unwrap();
    frontend.context.project_root = Some(root);
    let result =
        xiao_driver::run(&DriverRequest::new(frontend).with_timeout(Duration::from_secs(3)));
    assert!(result.is_success(), "{result:?}");
    assert_eq!(outputs(result.as_executed().unwrap().events()), "ok7\n");
}

/// 验证优化级别变化不会丢失并发值和 TAC 计划。
#[test]
fn source_values_and_plans_survive_all_optimization_levels() {
    for level in [
        xiao_optimizer::OptimizationLevel::O0,
        xiao_optimizer::OptimizationLevel::O1,
        xiao_optimizer::OptimizationLevel::O2,
        xiao_optimizer::OptimizationLevel::O3,
    ] {
        let source = "from std.concurrent import current_runtime, spawn, join\ndef work(str text) -> str\n    return text + \"!\"\nrt = current_runtime()\nt = spawn(rt, work, \"done\")\nprint(join(t))\n";
        let result = xiao_driver::run(
            &DriverRequest::new(FrontendRequest::from_text(source)).with_optimization_level(level),
        );
        assert!(result.is_success(), "{level:?}: {result:?}");
        assert_eq!(outputs(result.as_executed().unwrap().events()), "done!\n");
    }
}

/// 验证宿主缺少并发 feature 时 `.xiaoc` 在执行前被拒绝。
#[test]
fn xiaoc_host_missing_features_is_rejected_before_execution() {
    let source = "from std.concurrent import current_runtime\nrt = current_runtime()\nprint(\"must-not-run\")\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap();
    let program = xiao_bytecode::lower_program(&artifact.ir);
    let bytes = xiao_bytecode::encode_xiaoc(&program, Default::default()).unwrap();
    assert!(matches!(
        xiao_vm::run_xiaoc_with_runtime_features(&bytes, VmOptions::default(), 0),
        Err(xiao_bytecode::XiaocError::UnknownRequiredFeature(_))
    ));
}
