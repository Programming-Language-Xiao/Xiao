//! 22B 并发静态接线回归。

use xiao_driver::{FrontendCompiler, FrontendRequest};

#[test]
fn concurrent_source_reaches_ir_facts_and_cancellation_checks() {
    let source = "from std.concurrent import runtime_create, spawn, join\ndef work(int value) -> int\n    return value\nrt = runtime_create()\nthread = spawn(rt, work, 1)\nresult = join(thread)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap_or_else(|error| panic!("并发源码编译失败：{error:?}"));
    assert_eq!(artifact.ir.ownership.cross_thread_facts.len(), 1);
    assert_eq!(artifact.ir.ownership.boundary_transfers.len(), 1);
    assert_eq!(
        artifact
            .ir
            .ownership
            .cancellation_checks
            .iter()
            .map(|check| check.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["safe_point", "wait"]
    );
}

#[test]
fn submit_uses_pool_owner_runtime_and_wait_lowering() {
    let source = "from std.concurrent import current_runtime, pool_create, submit, wait\ndef work(int value) -> int\n    return value\nrt = current_runtime()\npool = pool_create(rt, 1)\ntask = submit(pool, work, 1)\nresult = wait(task)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("submit/wait 的静态接线应通过");
    assert_eq!(artifact.ir.ownership.cross_thread_facts.len(), 1);
    assert_ne!(artifact.ir.ownership.cross_thread_facts[0].runtime_id, 0);
    assert_eq!(
        artifact
            .ir
            .ownership
            .cancellation_checks
            .iter()
            .map(|check| check.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["safe_point", "wait"]
    );
}

#[test]
fn parameterized_spawn_checks_mixed_boundary_types() {
    let valid = "from std.concurrent import runtime_create, spawn, join\ndef work(int id, str text) -> int\n    return id\nrt = runtime_create()\nthread = spawn(rt, work, 1, \"x\")\nresult = join(thread)\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(valid))
        .expect("带参入口和混合边界值应通过");

    let invalid = "from std.concurrent import runtime_create, spawn\ndef work(int id, str text) -> int\n    return id\nrt = runtime_create()\nthread = spawn(rt, work, \"bad\", 1)\n";
    assert!(error_codes(invalid).contains(&"X04-TYPE-002".to_owned()));

    let extra = "from std.concurrent import runtime_create, spawn\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work, 1)\n";
    assert!(error_codes(extra).contains(&"X04-TYPE-002".to_owned()));
}

#[test]
fn concurrency_consumption_is_scoped_and_reassignment_resets_it() {
    let same_name = "from std.concurrent import runtime_create, spawn, join\ndef work(int value) -> int\n    return value\nrt = runtime_create()\ndef first() -> none\n    thread = spawn(rt, work, 1)\n    join(thread)\ndef second() -> none\n    thread = spawn(rt, work, 1)\n    join(thread)\nfirst()\nsecond()\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(same_name))
        .expect("不同函数中的同名句柄状态不能互相污染");

    let reassigned = "from std.concurrent import runtime_create, spawn, join\ndef work(int value) -> int\n    return value\nrt = runtime_create()\nthread = spawn(rt, work, 1)\njoin(thread)\nthread = spawn(rt, work, 1)\njoin(thread)\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(reassigned))
        .expect("重新赋值新句柄应重置消费状态");
}

#[test]
fn concurrency_consumption_flows_out_of_control_flow_scopes() {
    let consumed_in_branch = "from std.concurrent import runtime_create, spawn, join\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work)\nif true\n    join(thread)\njoin(thread)\n";
    assert!(error_codes(consumed_in_branch).contains(&"X04-TYPE-002".to_owned()));

    let mutually_exclusive_branches = "from std.concurrent import runtime_create, spawn, join\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work)\nif true\n    join(thread)\nelse\n    join(thread)\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(mutually_exclusive_branches))
        .expect("互斥分支可各自消费同一外层句柄");

    let consumed_in_loop = "from std.concurrent import runtime_create, spawn, join\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work)\nwhile false\n    join(thread)\njoin(thread)\n";
    assert!(error_codes(consumed_in_loop).contains(&"X04-TYPE-002".to_owned()));

    let consumed_repeatedly_in_loop = "from std.concurrent import runtime_create, spawn, join\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work)\nwhile false\n    join(thread)\n";
    assert!(error_codes(consumed_repeatedly_in_loop).contains(&"X04-TYPE-002".to_owned()));

    let fresh_handle_per_iteration = "from std.concurrent import runtime_create, spawn, join\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work)\nwhile false\n    thread = spawn(rt, work)\n    join(thread)\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(fresh_handle_per_iteration))
        .expect("每次循环先创建新句柄时可以消费该迭代的句柄");

    let consumed_shared_value_in_loop =
        "from std.concurrent import shared_read\nvalue = 1\nwhile false\n    shared_read(value)\n";
    assert!(error_codes(consumed_shared_value_in_loop).contains(&"X04-TYPE-002".to_owned()));

    let refreshed_shared_value_in_loop = "from std.concurrent import shared_read\nvalue = 1\nwhile false\n    value = 1\n    shared_read(value)\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(refreshed_shared_value_in_loop))
        .expect("每次循环先写入新值时可以消费该迭代的值");

    let consumed_in_try = "from std.concurrent import runtime_create, spawn, join\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work)\ntry\n    join(thread)\ncatch err as ArithmeticError\n    ignored = 0\njoin(thread)\n";
    assert!(error_codes(consumed_in_try).contains(&"X04-TYPE-002".to_owned()));
}

#[test]
fn concurrency_consumption_merges_return_break_and_continue_exits() {
    let valid_return = "from std.concurrent import runtime_create, spawn, join\nrt = runtime_create()\ndef work() -> int\n    return 1\ndef return_case() -> none\n    thread = spawn(rt, work)\n    if true\n        join(thread)\n        return\n    join(thread)\nreturn_case()\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(valid_return))
        .expect("return 分支退出后，未退出路径的消费状态应独立合流");

    let valid_break = "from std.concurrent import runtime_create, spawn, join\nrt = runtime_create()\ndef work() -> int\n    return 1\nthread = spawn(rt, work)\nwhile false\n    break\n    join(thread)\njoin(thread)\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(valid_break))
        .expect("break 后的不可达消费不能污染循环出口");

    let valid_continue = "from std.concurrent import runtime_create, spawn, join\nrt = runtime_create()\ndef work() -> int\n    return 1\nthread = spawn(rt, work)\nwhile false\n    continue\n    join(thread)\njoin(thread)\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(valid_continue))
        .expect("continue 后的不可达消费不能污染循环出口");

    let invalid_return = "from std.concurrent import runtime_create, spawn, join\nrt = runtime_create()\ndef work() -> int\n    return 1\ndef return_case() -> none\n    thread = spawn(rt, work)\n    if true\n        join(thread)\n        return\n    join(thread)\n    join(thread)\nreturn_case()\n";
    assert!(error_codes(invalid_return).contains(&"X04-TYPE-002".to_owned()));

    let invalid_break = "from std.concurrent import runtime_create, spawn, join\nrt = runtime_create()\ndef work() -> int\n    return 1\nthread = spawn(rt, work)\nwhile false\n    join(thread)\n    break\njoin(thread)\n";
    assert!(error_codes(invalid_break).contains(&"X04-TYPE-002".to_owned()));

    let invalid_continue = "from std.concurrent import runtime_create, spawn, join\nrt = runtime_create()\ndef work() -> int\n    return 1\nthread = spawn(rt, work)\nwhile false\n    join(thread)\n    continue\njoin(thread)\n";
    assert!(error_codes(invalid_continue).contains(&"X04-TYPE-002".to_owned()));
}

#[test]
fn raise_error_exit_and_unreachable_type_checks_are_isolated() {
    let raise_then_catch = "from std.concurrent import runtime_create, spawn, join\nrt = runtime_create()\ndef work() -> int\n    return 1\nthread = spawn(rt, work)\ntry\n    join(thread)\n    raise ArithmeticError(code = \"boom\")\ncatch err as Error\n    join(thread)\n";
    assert!(error_codes(raise_then_catch).contains(&"X04-TYPE-002".to_owned()));

    let return_type_error = "def bad() -> none\n    return\n    missing = unknown_name\nbad()\n";
    assert!(error_codes(return_type_error).contains(&"X02-TYPE-001".to_owned()));

    let break_type_error = "while true\n    break\n    missing = unknown_name\n";
    assert!(error_codes(break_type_error).contains(&"X02-TYPE-001".to_owned()));

    let continue_type_error = "while true\n    continue\n    missing = unknown_name\n";
    assert!(error_codes(continue_type_error).contains(&"X02-TYPE-001".to_owned()));
}

#[test]
fn aliases_share_thread_and_task_consumption_state() {
    let thread_alias = "from std.concurrent import runtime_create, spawn, join\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work)\nalias = thread\njoin(thread)\njoin(alias)\n";
    assert!(error_codes(thread_alias).contains(&"X04-TYPE-002".to_owned()));

    let task_alias = "from std.concurrent import current_runtime, pool_create, submit, wait\ndef work() -> int\n    return 1\nrt = current_runtime()\npool = pool_create(rt, 1)\ntask = submit(pool, work)\nalias = task\nwait(task)\nwait(alias)\n";
    assert!(error_codes(task_alias).contains(&"X04-TYPE-002".to_owned()));
}

fn error_codes(source: &str) -> Vec<String> {
    match FrontendCompiler::new().compile(&FrontendRequest::from_text(source)) {
        Ok(_) => Vec::new(),
        Err(error) => error
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_owned())
            .collect(),
    }
}

#[test]
fn concurrency_type_contract_emits_handle_and_send_diagnostics() {
    let wrong_handle = "from std.concurrent import cancel\ncancel(1)\n";
    assert!(error_codes(wrong_handle).contains(&"X03-TYPE-032".to_owned()));

    let dynamic_value = "from std.concurrent import runtime_create, spawn\ndef work([int] value) -> int\n    return 1\nrt = runtime_create()\nvalue = [1]\nthread = spawn(rt, work, value)\n";
    assert!(error_codes(dynamic_value).contains(&"X03-TYPE-030".to_owned()));

    let ordinary_handle = "from std.concurrent import runtime_create, spawn\ndef work() -> int\n    return 1\nrt = runtime_create()\nfirst = spawn(rt, work, 1)\nsecond = spawn(rt, work, first)\n";
    assert!(error_codes(ordinary_handle).contains(&"X03-TYPE-031".to_owned()));

    let resource_value = "from stdlib.os import open_file\nfrom std.concurrent import runtime_create, spawn as concurrent_spawn\ndef work(file value) -> file\n    return value\nrt = runtime_create()\nhandle = open_file(\"unused\")\nthread = concurrent_spawn(rt, work, handle)\n";
    let resource_codes = error_codes(resource_value);
    assert!(resource_codes.contains(&"X03-TYPE-030".to_owned()));
}

#[test]
fn consumed_handle_and_boundary_limit_are_rejected() {
    let consumed = "from std.concurrent import runtime_create, spawn, join, cancel\ndef work(int value) -> int\n    return value\nrt = runtime_create()\nthread = spawn(rt, work, 1)\njoin(thread)\ncancel(thread)\n";
    assert!(error_codes(consumed).contains(&"X04-TYPE-002".to_owned()));

    let values = (0..65).map(|_| "1").collect::<Vec<_>>().join(", ");
    let source = format!(
        "from std.concurrent import runtime_create, spawn\ndef work([int] value) -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work, {values})\n"
    );
    assert!(error_codes(&source).contains(&"X04-TYPE-002".to_owned()));
}

#[test]
fn cancel_does_not_consume_a_handle() {
    let source = "from std.concurrent import runtime_create, spawn, cancel, join\ndef work(int value) -> int\n    return value\nrt = runtime_create()\nthread = spawn(rt, work, 1)\ncancel(thread)\ncancel(thread)\nresult = join(thread)\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("cancel 只借用句柄，之后仍可 cancel/join");
}

#[test]
fn shared_mut_cross_runtime_is_rejected_by_lifetime_lowering() {
    let source = "from std.concurrent import runtime_create, shared_mut, spawn\ndef work(shared_mut<int> value) -> int\n    return 1\nrt = runtime_create()\nvalue = shared_mut(1)\nthread = spawn(rt, work, value)\n";
    assert!(error_codes(source).contains(&"X06-LIFETIME-008".to_owned()));

    let same_runtime = "from std.concurrent import current_runtime, shared_mut, spawn\ndef work(shared_mut<int> value) -> int\n    return 1\nrt = current_runtime()\nvalue = shared_mut(1)\nthread = spawn(rt, work, value)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(same_runtime))
        .expect("current_runtime 下的 shared_mut 应保持同一 Runtime");
    assert_eq!(artifact.ir.ownership.cross_thread_facts.len(), 1);
    assert_ne!(artifact.ir.ownership.cross_thread_facts[0].runtime_id, 0);
    assert_eq!(
        artifact.ir.ownership.cross_thread_facts[0].source_runtime_id,
        Some(artifact.ir.ownership.cross_thread_facts[0].runtime_id)
    );
}

#[test]
fn lock_wait_is_lowered_and_lock_guard_escape_is_rejected() {
    let source = "from std.concurrent import shared_mut, lock, lock_read\nvalue = shared_mut(1)\nguard = lock(value)\nresult = lock_read(guard)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("同一作用域内的锁读取应通过静态检查");
    assert_eq!(
        artifact.ir.ownership.cancellation_checks[0].kind,
        "lock_wait"
    );

    let escaping = "def leak(lock_guard<int> guard) -> lock_guard<int>\n    return guard\n";
    assert!(error_codes(escaping).contains(&"X06-LIFETIME-006".to_owned()));
}

#[test]
fn shared_operations_consume_sources_and_check_capabilities() {
    let shared_read = "from std.concurrent import shared_read\nvalue = 1\nshared_read(value)\nresult = value + 1\n";
    let shared_read_codes = error_codes(shared_read);
    assert!(shared_read_codes.contains(&"X04-TYPE-002".to_owned()));

    let shared_mut =
        "from std.concurrent import shared_mut\nvalue = 1\nshared_mut(value)\nresult = value + 1\n";
    assert!(error_codes(shared_mut).contains(&"X04-TYPE-002".to_owned()));

    let aliased_shared_source = "from std.concurrent import shared_read\nvalue = 1\nalias = value\nshared_read(value)\nresult = alias + 1\n";
    assert!(error_codes(aliased_shared_source).contains(&"X04-TYPE-002".to_owned()));

    let lock_write = "from std.concurrent import shared_mut, lock, lock_write\nvalue = shared_mut(1)\nguard = lock(value)\nnew_value = 2\nlock_write(guard, new_value)\nresult = new_value + 1\n";
    assert!(error_codes(lock_write).contains(&"X04-TYPE-002".to_owned()));

    let bad_shared =
        "from std.concurrent import shared_read, shared_mut\nshared_read([1])\nshared_mut([1])\n";
    let codes = error_codes(bad_shared);
    assert!(
        codes
            .iter()
            .filter(|code| code.as_str() == "X03-TYPE-030")
            .count()
            >= 2
    );
}
