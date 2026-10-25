//! 22B 并发静态接线回归。

use xiao_driver::{FrontendCompiler, FrontendRequest};

#[test]
fn concurrent_source_reaches_ir_facts_and_cancellation_checks() {
    let source = "from std.concurrent import runtime_create, spawn, join\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work, 1)\nresult = join(thread)\n";
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
    let source = "from std.concurrent import current_runtime, pool_create, submit, wait\ndef work() -> int\n    return 1\nrt = current_runtime()\npool = pool_create(rt, 1)\ntask = submit(pool, work, 1)\nresult = wait(task)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("submit/wait 的静态接线应通过");
    assert_eq!(artifact.ir.ownership.cross_thread_facts.len(), 1);
    assert_eq!(artifact.ir.ownership.cross_thread_facts[0].runtime_id, 0);
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
}

#[test]
fn concurrency_consumption_is_scoped_and_reassignment_resets_it() {
    let same_name = "from std.concurrent import runtime_create, spawn, join\ndef work() -> int\n    return 1\nrt = runtime_create()\ndef first() -> none\n    thread = spawn(rt, work, 1)\n    join(thread)\ndef second() -> none\n    thread = spawn(rt, work, 1)\n    join(thread)\nfirst()\nsecond()\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(same_name))
        .expect("不同函数中的同名句柄状态不能互相污染");

    let reassigned = "from std.concurrent import runtime_create, spawn, join\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work, 1)\njoin(thread)\nthread = spawn(rt, work, 1)\njoin(thread)\n";
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(reassigned))
        .expect("重新赋值新句柄应重置消费状态");
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

    let dynamic_value = "from std.concurrent import runtime_create, spawn\ndef work() -> int\n    return 1\nrt = runtime_create()\nvalue = [1]\nthread = spawn(rt, work, value)\n";
    assert!(error_codes(dynamic_value).contains(&"X03-TYPE-030".to_owned()));

    let ordinary_handle = "from std.concurrent import runtime_create, spawn\ndef work() -> int\n    return 1\nrt = runtime_create()\nfirst = spawn(rt, work, 1)\nsecond = spawn(rt, work, first)\n";
    assert!(error_codes(ordinary_handle).contains(&"X03-TYPE-031".to_owned()));

    let resource_value = "from stdlib.os import open_file\nfrom std.concurrent import runtime_create, spawn as concurrent_spawn\ndef work() -> int\n    return 1\nrt = runtime_create()\nhandle = open_file(\"unused\")\nthread = concurrent_spawn(rt, work, handle)\n";
    let resource_codes = error_codes(resource_value);
    assert!(resource_codes.contains(&"X03-TYPE-030".to_owned()));
}

#[test]
fn consumed_handle_and_boundary_limit_are_rejected() {
    let consumed = "from std.concurrent import runtime_create, spawn, join, cancel\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work, 1)\njoin(thread)\ncancel(thread)\n";
    assert!(error_codes(consumed).contains(&"X04-TYPE-002".to_owned()));

    let values = (0..65).map(|_| "1").collect::<Vec<_>>().join(", ");
    let source = format!(
        "from std.concurrent import runtime_create, spawn\ndef work() -> int\n    return 1\nrt = runtime_create()\nthread = spawn(rt, work, {values})\n"
    );
    assert!(error_codes(&source).contains(&"X04-TYPE-002".to_owned()));
}

#[test]
fn shared_mut_cross_runtime_is_rejected_by_lifetime_lowering() {
    let source = "from std.concurrent import runtime_create, shared_mut, spawn\ndef work() -> int\n    return 1\nrt = runtime_create()\nvalue = shared_mut(1)\nthread = spawn(rt, work, value)\n";
    assert!(error_codes(source).contains(&"X06-LIFETIME-008".to_owned()));

    let same_runtime = "from std.concurrent import current_runtime, shared_mut, spawn\ndef work() -> int\n    return 1\nrt = current_runtime()\nvalue = shared_mut(1)\nthread = spawn(rt, work, value)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(same_runtime))
        .expect("current_runtime 下的 shared_mut 应保持同一 Runtime");
    assert_eq!(artifact.ir.ownership.cross_thread_facts.len(), 1);
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
