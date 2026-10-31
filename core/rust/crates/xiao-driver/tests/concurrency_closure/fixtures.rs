//! 22D-FIX.1 黄金矩阵的真实源码；标签同时作为失败诊断和证据中的稳定用例名。

/// 共享标志的读取和写入均通过作用域守卫完成，握手不依赖固定睡眠。
const FLAGS: &str = r#"def read_flag(shared_mut<int> flag) -> int
    guard = lock(flag)
    return lock_read(guard)
def write_flag(shared_mut<int> flag, int value) -> none
    guard = lock(flag)
    lock_write(guard, value)
"#;

/// C-1/C-2/C-5/C-6/C-9 的成功源码及逐字黄金输出。
pub(super) fn success_cases() -> Vec<(&'static str, String, String)> {
    vec![
        (
            "spawn-join-str-multiargument",
            r#"from std.concurrent import current_runtime, spawn, join
def work(int number, str text) -> str
    print(number)
    return text
rt = current_runtime()
thread = spawn(rt, work, 7, "thread:")
print(join(thread))
"#
            .to_owned(),
            "7\nthread:\n".to_owned(),
        ),
        (
            "spawn-join-tuple",
            r#"from std.concurrent import current_runtime, spawn, join
def identity<T>(T value) -> T
    return value
rt = current_runtime()
argument = (7, 9)
thread = spawn(rt, identity, argument)
result = join(thread)
print(result[0])
print(result[1])
"#
            .to_owned(),
            "7\n9\n".to_owned(),
        ),
        (
            "detach-success",
            r#"from std.concurrent import current_runtime, spawn, detach
def work() -> none
    print("detached")
rt = current_runtime()
thread = spawn(rt, work)
detach(thread)
print("main")
"#
            .to_owned(),
            "detached\nmain\n".to_owned(),
        ),
        (
            "multi-runtime-parallel",
            r#"from std.concurrent import runtime_create, spawn, join, runtime_close
def work(int value) -> int
    return value
first = runtime_create()
second = runtime_create()
left = spawn(first, work, 11)
right = spawn(second, work, 22)
print(join(left))
print(join(right))
runtime_close(second)
runtime_close(first)
"#
            .to_owned(),
            "11\n22\n".to_owned(),
        ),
        (
            "lock-guard-scope-release",
            r#"from std.concurrent import shared_mut, lock, lock_read, lock_write
def write(shared_mut<int> value) -> none
    guard = lock(value)
    lock_write(guard, 9)
def read(shared_mut<int> value) -> int
    guard = lock(value)
    return lock_read(guard)
value = shared_mut(0)
write(value)
print(read(value))
"#
            .to_owned(),
            "9\n".to_owned(),
        ),
        (
            "worker-loop-bounded-stack",
            r#"from std.concurrent import current_runtime, spawn, join
def work(int limit) -> int
    count = 0
    while count < limit
        count = count + 1
    return count
rt = current_runtime()
thread = spawn(rt, work, 100000)
print(join(thread))
"#
            .to_owned(),
            "100000\n".to_owned(),
        ),
        (
            "xcon-bounded-string-success",
            bounded_string_source(),
            format!("{}\n", "x".repeat(64 * 1024)),
        ),
    ]
}

/// C-9 逐字核对 64 KiB 的 XCON 字节往返；递归深度由 native Runtime 回归覆盖。
fn bounded_string_source() -> String {
    format!(
        "from std.concurrent import current_runtime, spawn, join\ndef identity(str value) -> str\n    return value\nrt = current_runtime()\nthread = spawn(rt, identity, \"{}\")\nprint(join(thread))\n",
        "x".repeat(64 * 1024)
    )
}

/// 巨型字面量隔离边界大小检查，不混入字符串拼接临时对象的实现差异。
fn oversized_source() -> String {
    format!(
        "from std.concurrent import current_runtime, spawn, join\ndef work(str value) -> int\n    print(\"must-not-start\")\n    return 1\nrt = current_runtime()\nthread = spawn(rt, work, \"{}\")\njoin(thread)\n",
        "x".repeat(16 * 1024 * 1024 + 1)
    )
}

/// 错误场景的源码、主错误身份、输出和可选 suppressed 身份。
pub(super) type ErrorCase = (
    &'static str,
    String,
    &'static str,
    &'static str,
    Option<&'static str>,
);

/// C-3/C-4/C-7/C-8/C-9：错误身份、清理输出及单个 suppressed 黄金身份。
pub(super) fn error_cases() -> Vec<ErrorCase> {
    vec![
        (
            "pool-capacity-zero-032",
            "from std.concurrent import current_runtime, pool_create\nrt = current_runtime()\npool = pool_create(rt, 0)\nprint(\"must-not-run\")\n".to_owned(),
            "X06-RUNTIME-032", "", None,
        ),
        (
            "pool-submit-after-shutdown-030",
            "from std.concurrent import current_runtime, pool_create, submit, shutdown\ndef work(int value, str text) -> int\n    print(\"must-not-run\")\n    return value\nrt = current_runtime()\npool = pool_create(rt, 1)\nshutdown(pool)\nsubmit(pool, work, 1, \"payload\")\n".to_owned(),
            "X06-RUNTIME-030", "", None,
        ),
        (
            "native-queued-cancellation", queued_source(),
            "X06-RUNTIME-027", "queued-finally\n", None,
        ),
        (
            "native-waiting-cancellation", lock_wait_source(false),
            "X06-RUNTIME-027", "lock-finally\n", None,
        ),
        (
            "native-wait-027", pool_wait_source(),
            "X06-RUNTIME-027", "wait-finally\nwait-drop\n", Some("wait-cleanup"),
        ),
        (
            "native-lock-wait-027", lock_wait_source(true),
            "X06-RUNTIME-027", "lock-finally\n", Some("lock-cleanup"),
        ),
        (
            "xcon-oversized-029",
            oversized_source(),
            "X06-RUNTIME-029", "", None,
        ),
    ]
}

/// C-7 先占用唯一 worker；被取消任务只在 gate 释放后才有机会启动。
fn queued_source() -> String {
    format!(
        r#"from std.concurrent import current_runtime, shared_mut, spawn, join, pool_create, submit, wait, cancel, lock, lock_read, lock_write
{FLAGS}def release_after_wait(shared_mut<int> ready, shared_mut<int> gate) -> int
    while read_flag(ready) == 0
        value = 1
    write_flag(gate, 1)
    return 0
def blocker(shared_mut<int> gate) -> int
    while read_flag(gate) == 0
        value = 1
    return 1
def queued(str value) -> int
    print("must-not-run")
    return 0
rt = current_runtime()
ready = shared_mut(0)
gate = shared_mut(0)
release_gate = gate
pool = pool_create(rt, 1)
release = spawn(rt, release_after_wait, ready, release_gate)
first = submit(pool, blocker, gate)
second = submit(pool, queued, "payload")
cancel(second)
try
    wait(second)
catch err as Error
    print("must-not-catch")
finally
    write_flag(ready, 1)
    wait(first)
    join(release)
    print("queued-finally")
"#
    )
}

/// C-7/C-8 目标锁由主执行单元持有，握手保证子线程进入 try 后才取消。
fn lock_wait_source(cleanup_error: bool) -> String {
    let cleanup = if cleanup_error {
        "        raise TypeError(code = \"lock-cleanup\")\n"
    } else {
        ""
    };
    format!(
        r#"from std.concurrent import current_runtime, shared_mut, spawn, join, cancel, lock, lock_read, lock_write
{FLAGS}def worker(shared_mut<int> gate, shared_mut<int> ready) -> int
    try
        write_flag(ready, 1)
        guard = lock(gate)
        return lock_read(guard)
    finally
        print("lock-finally")
{cleanup}    return 0
rt = current_runtime()
gate = shared_mut(7)
ready = shared_mut(0)
held = lock(gate)
thread = spawn(rt, worker, gate, ready)
while read_flag(ready) == 0
    value = 1
cancel(thread)
try
    join(thread)
catch err as Error
    print("must-not-catch")
"#
    )
}

/// C-8 运行中的 pool task 取消经 wait 返回 027，清理错误仍为 suppressed。
fn pool_wait_source() -> String {
    format!(
        r#"[[Item]]
    value = 1
    def drop(self) -> none
        print("wait-drop")
from std.concurrent import current_runtime, shared_mut, pool_create, submit, wait, cancel, lock, lock_read, lock_write
{FLAGS}def worker(shared_mut<int> ready) -> int
    item = new Item()
    try
        write_flag(ready, 1)
        while true
            value = 1
    finally
        print("wait-finally")
        raise TypeError(code = "wait-cleanup")
    return 0
rt = current_runtime()
ready = shared_mut(0)
pool = pool_create(rt, 1)
task = submit(pool, worker, ready)
while read_flag(ready) == 0
    value = 1
cancel(task)
try
    wait(task)
catch err as Error
    print("must-not-catch")
"#
    )
}
