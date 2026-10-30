//! 22D 原生并发 lowering 的同源 IR 与 fail-closed 回归。

use std::collections::BTreeMap;
use std::fs;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use xiao_codegen_llvm::{CodegenOptions, lower_program, native_concurrency_plan};
use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_driver::{
    DriverRequest, FrontendCompiler, FrontendContext, FrontendNativeDriver, FrontendRequest,
    FrontendVmDriver, NativeBuildRequest,
};

/// 把生成文本交给真实 LLVM 解析器；22D 的这组回归必须具备 `llvm-as`，不能用文本
/// 包含关系替代汇编器验证。
fn validate_with_llvm_as(text: &str, label: &str) {
    let llvm_as = std::env::var_os("XIAO_LLVM_AS")
        .or_else(|| {
            let command = if cfg!(windows) { "where.exe" } else { "which" };
            let output = Command::new(command).arg("llvm-as").output().ok()?;
            if !output.status.success() {
                return None;
            }
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()?
                .trim()
                .to_owned();
            (!path.is_empty()).then_some(path.into())
        })
        .unwrap_or_else(|| {
            panic!("{label}: 22D LLVM 回归必须提供 XIAO_LLVM_AS 或 PATH 中的 llvm-as")
        });
    Toolchain::new("unused")
        .with_llvm_as(llvm_as)
        .validate_llvm_ir(text, &TargetDescription::host())
        .unwrap_or_else(|error| panic!("{label}: llvm-as 应接受生成文本：{error}"));
}

/// 对 VM/native 场景统一施加超时，防止并发回归挂死 CI worker。
fn run_native_with_timeout(executable: &std::path::Path, label: &str) -> Output {
    let child = Command::new(executable)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("{label}: 启动 native 程序失败：{error}"));
    wait_child_with_timeout(child, label, Duration::from_secs(10))
}

fn wait_child_with_timeout(mut child: Child, label: &str, timeout: Duration) -> Output {
    let deadline = Instant::now() + timeout;
    loop {
        if child
            .try_wait()
            .unwrap_or_else(|error| panic!("{label}: 轮询子进程失败：{error}"))
            .is_some()
        {
            return child
                .wait_with_output()
                .unwrap_or_else(|error| panic!("{label}: 收集子进程输出失败：{error}"));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .unwrap_or_else(|error| panic!("{label}: 超时后回收子进程失败：{error}"));
            panic!(
                "{label}: native 程序超过 {timeout:?}，stdout={:?} stderr={:?}",
                output.stdout, output.stderr
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
}

struct ReleaseTraceEnvironment {
    previous: Option<std::ffi::OsString>,
}

impl ReleaseTraceEnvironment {
    fn new() -> Self {
        Self {
            previous: std::env::var_os("XIAO_RUNTIME_RELEASE_TRACE_PATH"),
        }
    }

    fn set_path(&self, path: &std::path::Path) {
        unsafe { std::env::set_var("XIAO_RUNTIME_RELEASE_TRACE_PATH", path) };
    }

    fn reset_runtime_trace(&self) {
        unsafe { std::env::remove_var("XIAO_RUNTIME_RELEASE_TRACE_PATH") };
        xiao_runtime::start_release_trace_from_env();
    }
}

impl Drop for ReleaseTraceEnvironment {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            unsafe { std::env::set_var("XIAO_RUNTIME_RELEASE_TRACE_PATH", previous) };
        } else {
            unsafe { std::env::remove_var("XIAO_RUNTIME_RELEASE_TRACE_PATH") };
        }
        xiao_runtime::start_release_trace_from_env();
    }
}

fn release_trace_sequence(path: &std::path::Path) -> Vec<(usize, String)> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("读取释放追踪 {} 失败：{error}", path.display()));
    let mut identities = BTreeMap::<u64, usize>::new();
    let mut events = Vec::new();
    for (expected_sequence, line) in text.lines().enumerate() {
        let mut fields = line.split('\t');
        let line_sequence = fields
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or_else(|| panic!("无效释放追踪序号：{line:?}"));
        assert_eq!(line_sequence, expected_sequence, "释放追踪序号必须连续");
        let object_id = fields
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_else(|| panic!("无效释放追踪对象身份：{line:?}"));
        let action = fields
            .next()
            .unwrap_or_else(|| panic!("释放追踪缺少动作：{line:?}"));
        assert!(fields.next().is_none(), "释放追踪多出字段：{line:?}");
        // VM 值不经过 C ABI 的不透明盒子；ConcurrencyRelease 是 native 包装层
        // 自身的引用归还，不对应语言值/共享对象的语义释放动作。
        if action == "concurrency_release" {
            continue;
        }
        let next_identity = identities.len();
        let normalized_identity = *identities.entry(object_id).or_insert(next_identity);
        let event = (normalized_identity, action.to_owned());
        // ABI value copies may add adjacent temporary aliases on one backend. Keep
        // transitions between actions and objects, while avoiding alias-count skew.
        if events.last() != Some(&event) {
            events.push(event);
        }
    }
    events
}

fn detached_message_projection(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let line = line.trim_start();
            !line.starts_with("at bytes ") && !line.starts_with("at ::")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
/// 比较同一 IR 降出的 Native/TAC 计划并检查 trampoline lowering。
fn native_lowering_consumes_concurrency_spans_and_emits_trampoline() {
    let source = "from std.concurrent import runtime_create, spawn, join\ndef work(int value) -> int\n    return value\nrt = runtime_create()\nthread = spawn(rt, work, 1)\nresult = join(thread)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("并发源码应先通过统一前端");
    let native_plan = native_concurrency_plan(&artifact.ir).expect("建立原生并发计划");
    let tac = xiao_bytecode::lower_program(&artifact.ir);
    assert_eq!(native_plan.calls().len(), tac.concurrency_plans.len());
    for (native, vm) in native_plan.calls().iter().zip(&tac.concurrency_plans) {
        assert_eq!(native.span, vm.span);
        assert_eq!(native.operation, vm.operation);
        assert_eq!(native.argument_spans, vm.argument_spans);
        assert_eq!(native.argument_types, vm.argument_types);
        assert_eq!(native.result_type, vm.result_type);
        assert_eq!(native.cross_thread_facts, vm.facts);
        assert_eq!(
            native.boundary_transfer.as_ref().map(|transfer| (
                transfer.copy_kind.as_str(),
                transfer.parameters.as_slice(),
                transfer.cleanup_order.as_slice(),
            )),
            vm.transfer.as_ref().map(|transfer| (
                transfer.copy_kind.as_str(),
                transfer.parameters.as_slice(),
                transfer.cleanup_order.as_slice(),
            ))
        );
        assert_eq!(
            native
                .cancellation_checks
                .iter()
                .map(|check| check.kind.as_str())
                .collect::<Vec<_>>(),
            vm.cancellation_checks
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
    }
    let module = lower_program(
        &artifact.ir,
        &CodegenOptions::default().with_internal_concurrency(),
    )
    .expect("同源并发 IR 应能生成 LLVM 文本");
    assert!(
        module
            .text
            .contains("@xiao_runtime_concurrency_call_with_entry")
    );
    assert!(module.text.contains("xiao.concurrent.trampoline."));
    assert!(
        module
            .text
            .contains("@xiao_runtime_concurrency_cancellation_check")
    );
    assert!(
        module
            .runtime_components
            .iter()
            .any(|component| component == "concurrency")
    );
}

#[test]
/// 确认未知并发操作在普通 lowering 前被拒绝。
fn unknown_native_concurrency_operation_is_rejected_before_lowering() {
    let source = "from std.concurrent import runtime_create\nrt = runtime_create()\n";
    let mut artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("源码应先通过统一前端");
    artifact.ir.concurrency_calls[0].operation = "future_api".to_owned();
    let error = lower_program(
        &artifact.ir,
        &CodegenOptions::default().with_internal_concurrency(),
    )
    .expect_err("未知并发入口必须 fail-closed");
    assert!(error.to_string().contains("原生并发入口 future_api"));
}

#[test]
/// 守住 22E 前公开构建不得开启并发产物。
fn public_native_build_keeps_concurrency_artifacts_closed_until_22e() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "from std.concurrent import current_runtime\nrt = current_runtime()\n",
        ))
        .expect("并发源码的前端语义有效");
    let error = lower_program(&artifact.ir, &CodegenOptions::default())
        .expect_err("公开原生构建不得提前发布并发产物");
    assert!(matches!(
        error,
        xiao_codegen_llvm::CodegenError::Unsupported { .. }
    ));
    assert!(error.to_string().contains("22E"));
}

#[test]
/// 检查 shared/lock lowering 保留取消检查与 Runtime ABI 路径。
fn native_shared_lock_plan_keeps_cancel_and_runtime_abi_paths() {
    let source = "from std.concurrent import shared_mut, lock, lock_read\nvalue = shared_mut(1)\nguard = lock(value)\nresult = lock_read(guard)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("shared/lock 源码应通过统一前端");
    let module = lower_program(
        &artifact.ir,
        &CodegenOptions::default().with_internal_concurrency(),
    )
    .expect("shared/lock 同源 IR 应能生成 LLVM 文本");
    assert!(module.text.contains("xiao_runtime_concurrency_call"));
    assert!(
        module
            .text
            .contains("xiao_runtime_concurrency_cancellation_check")
    );
    assert!(module.text.contains("xiao_runtime_release_trace_begin"));
}

#[test]
/// 真实普通项目模块的顶层并发调用也必须在 native lowering 前结构化拒绝。
fn native_project_module_concurrency_is_rejected_structurally() {
    static PROJECT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "xiao-22d-module-concurrency-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).expect("创建项目模块测试目录");
    let main_path = root.join("main.xiao");
    fs::write(
        root.join("bridge.xiao"),
        "from std.concurrent import runtime_create\ndef create_runtime() -> runtime\n    return runtime_create()\n",
    )
    .expect("写入并发函数 re-export 模块");
    fs::write(
        root.join("helper.xiao"),
        "from bridge import create_runtime\nrt = create_runtime()\n",
    )
    .expect("写入间接传递并发函数的项目模块");
    let source = "from helper import rt\nprint(1)\n";
    fs::write(&main_path, source).expect("写入项目入口");
    let mut context = xiao_driver::FrontendContext::host();
    context.project_root = Some(root.clone());
    let request = FrontendRequest::from_text_at(source, &main_path).with_context(context);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("普通项目模块并发源码应通过前端");
    assert!(
        artifact
            .ir
            .modules
            .iter()
            .any(|module| { module.name == "helper" && module.has_concurrency }),
        "间接导入并发函数的模块必须传播并发标记"
    );
    let vm = FrontendVmDriver::new().run_artifact(
        &artifact,
        &DriverRequest::new(request.clone()).with_timeout(Duration::from_secs(10)),
    );
    assert!(vm.is_success(), "VM 可执行普通模块并发：{vm:?}");
    let error = lower_program(
        &artifact.ir,
        &CodegenOptions::default().with_internal_concurrency(),
    )
    .expect_err("普通项目模块并发必须在 native lowering 前拒绝");
    assert!(matches!(
        error,
        xiao_codegen_llvm::CodegenError::Unsupported { feature, .. }
            if feature == "模块体并发暂不支持"
    ));
    let _ = fs::remove_dir_all(root);
}

#[test]
#[ignore = "需要受控 llvm-as；由三平台 native job 显式执行"]
/// 使用真实 llvm-as 校验根导入、函数别名、分组目标和局部别名四个边界。
fn native_concurrency_llvm_assembly_fixtures() {
    let fixtures = [
        (
            "根导入别名 + spawn/join",
            "from stdlib.text import length as mylen\nfrom std.concurrent import runtime_create, spawn, join\ndef work(int value) -> int\n    return value\nrt = runtime_create()\nt = spawn(rt, work, 1)\nprint(join(t))\n",
        ),
        (
            "别名函数目标",
            "from stdlib.text import length as mylen\nfrom std.concurrent import runtime_create, spawn, join\nrt = runtime_create()\nt = spawn(rt, mylen, \"abc\")\nprint(join(t))\n",
        ),
        (
            "括号分组函数目标",
            "from std.concurrent import runtime_create, spawn, join\ndef work(int value) -> int\n    return value\nrt = runtime_create()\nt = spawn(rt, (work), 1)\nprint(join(t))\n",
        ),
        (
            "函数体局部别名目标",
            "from std.concurrent import runtime_create, spawn, join\ndef outer() -> int\n    from stdlib.text import length as mylen\n    rt = runtime_create()\n    t = spawn(rt, mylen, \"abc\")\n    return join(t)\nprint(outer())\n",
        ),
    ];
    for (label, source) in fixtures {
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .unwrap_or_else(|error| panic!("{label}: 前端应接受合法并发源码：{error:?}"));
        let module = lower_program(
            &artifact.ir,
            &CodegenOptions::default().with_internal_concurrency(),
        )
        .unwrap_or_else(|error| panic!("{label}: lowering 应成功：{error}"));
        validate_with_llvm_as(&module.text, label);
    }
}

#[test]
#[ignore = "需要 XIAO_CLANG、XIAO_LLVM_AS、XIAO_LLC、XIAO_RUNTIME_LIBRARY 和 XIAO_TARGET_TRIPLE"]
/// 在目标原生工具链下比较线程/共享黄金输出和 Runtime 031 负例。
fn native_concurrency_vm_native_differential_matches_from_same_ir() {
    let target = TargetDescription::host();
    let configured_target = std::env::var("XIAO_TARGET_TRIPLE").expect("XIAO_TARGET_TRIPLE");
    assert_eq!(configured_target, target.triple);
    let toolchain = Toolchain::new(std::env::var_os("XIAO_CLANG").expect("XIAO_CLANG"))
        .with_llvm_as(std::env::var_os("XIAO_LLVM_AS").expect("XIAO_LLVM_AS"))
        .with_llc(std::env::var_os("XIAO_LLC").expect("XIAO_LLC"))
        .with_runtime_library(
            std::env::var_os("XIAO_RUNTIME_LIBRARY").expect("XIAO_RUNTIME_LIBRARY"),
        )
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .expect("Runtime 的目标系统库清单应能读取");
    let suffix = if cfg!(windows) { "exe" } else { "bin" };
    let root = std::env::temp_dir().join(format!("xiao-22d-native-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("创建原生并发临时目录");
    let release_trace_environment = ReleaseTraceEnvironment::new();
    let cases = [
        (
            "spawn-join",
            "from std.concurrent import runtime_create, spawn, join\ndef work(int value) -> int\n    return value + 6\nrt = runtime_create()\nthread = spawn(rt, work, 1)\nresult = join(thread)\nprint(result)\n",
            "7\n",
        ),
        (
            "pool-submit-wait",
            "from std.concurrent import current_runtime, pool_create, submit, wait, shutdown\ndef work(int value) -> int\n    return value + 5\nrt = current_runtime()\npool = pool_create(rt, 1)\ntask = submit(pool, work, 7)\nresult = wait(task)\nprint(result)\nshutdown(pool)\n",
            "12\n",
        ),
        (
            "shared-lock",
            "from std.concurrent import shared_mut, lock, lock_read, lock_write\nvalue = shared_mut(1)\nguard = lock(value)\nlock_write(guard, 9)\nresult = lock_read(guard)\nprint(result)\n",
            "9\n",
        ),
        (
            "shared-read-boundary",
            "from std.concurrent import runtime_create, shared_read, read, spawn, join\ndef work(shared<int> value) -> int\n    return read(value)\nrt = runtime_create()\nvalue = shared_read(4)\nthread = spawn(rt, work, value)\nresult = join(thread)\nprint(result)\n",
            "4\n",
        ),
        (
            "finally-return-overrides-child-error",
            "from std.concurrent import current_runtime, spawn, join\ndef work() -> none\n    try\n        raise ArithmeticError(code = \"overridden-child-error\")\n    finally\n        return none\nrt = current_runtime()\nthread = spawn(rt, work)\njoin(thread)\nprint(\"return-wins\")\n",
            "return-wins\n",
        ),
    ];
    for (label, source, expected_output) in cases {
        let frontend_request = FrontendRequest::from_text(source);
        let artifact = FrontendCompiler::new()
            .compile(&frontend_request)
            .unwrap_or_else(|error| panic!("{label}: 差分源码应通过统一前端：{error:?}"));
        let vm_trace = root.join(format!("{label}.vm-release-trace.tsv"));
        release_trace_environment.set_path(&vm_trace);
        xiao_runtime::start_release_trace_from_env();
        let vm = FrontendVmDriver::new().run_artifact(
            &artifact,
            &DriverRequest::new(frontend_request.clone()).with_timeout(Duration::from_secs(10)),
        );
        assert!(vm.is_success(), "{label}: VM 场景必须成功：{vm:?}");
        let vm_output = vm
            .as_executed()
            .expect("VM 返回执行结果")
            .events()
            .iter()
            .filter_map(|event| match event {
                xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>()
            .replace("\r\n", "\n");
        xiao_runtime::flush_release_trace();
        release_trace_environment.reset_runtime_trace();
        assert_eq!(vm_output, expected_output, "{label}: VM 应匹配黄金输出");
        let output = root.join(format!("{label}.{suffix}"));
        let request =
            NativeBuildRequest::new(frontend_request, target.clone(), toolchain.clone(), &output)
                .with_codegen_options(
                    CodegenOptions::for_target(target.clone()).with_internal_concurrency(),
                );
        let native = FrontendNativeDriver::new()
            .build_artifact(&artifact, &request)
            .unwrap_or_else(|error| panic!("{label}: 原生构建失败：{error}"));
        let native_trace = root.join(format!("{label}.native-release-trace.tsv"));
        release_trace_environment.set_path(&native_trace);
        let execution = run_native_with_timeout(&native.native.executable, label);
        assert_eq!(execution.status.code(), Some(0), "{label}");
        assert!(
            execution.stderr.is_empty(),
            "{label}: 成功场景不得写入 stderr：{}",
            String::from_utf8_lossy(&execution.stderr)
        );
        let native_output = String::from_utf8(execution.stdout)
            .expect("原生 stdout 为 UTF-8")
            .replace("\r\n", "\n");
        assert_eq!(native_output, vm_output, "{label}");
        assert_eq!(
            native_output, expected_output,
            "{label}: native 应匹配黄金输出"
        );
        assert_eq!(
            release_trace_sequence(&native_trace),
            release_trace_sequence(&vm_trace),
            "{label}: VM/native 完整释放对象顺序"
        );
    }

    let error_cases = [
        (
            "closed-runtime-shared-read",
            "from std.concurrent import current_runtime, runtime_close, shared_read, read\nrt = current_runtime()\nruntime_close(rt)\nvalue = shared_read(7)\nprint(read(value))\n",
            "X06-RUNTIME-031",
            "",
            None,
        ),
        (
            "child-shared-mut-result",
            "from std.concurrent import runtime_create, spawn, join, shared_mut\ndef work() -> shared_mut<int>\n    return shared_mut(1)\nrt = runtime_create()\nthread = spawn(rt, work)\nvalue = join(thread)\nprint(\"joined\")\n",
            "X06-RUNTIME-031",
            "",
            None,
        ),
        (
            "pool-shared-mut-result",
            "from std.concurrent import runtime_create, pool_create, submit, wait, shutdown, shared_mut\ndef work() -> shared_mut<int>\n    return shared_mut(1)\nrt = runtime_create()\npool = pool_create(rt, 1)\ntask = submit(pool, work)\nvalue = wait(task)\nprint(\"waited\")\nshutdown(pool)\n",
            "X06-RUNTIME-031",
            "",
            None,
        ),
        (
            "child-primary-and-finally-suppressed",
            "from std.concurrent import current_runtime, spawn, join\ndef work() -> none\n    try\n        raise TypeError(code = \"child-primary\")\n    finally\n        print(\"cleanup\")\n        raise TypeError(code = \"child-cleanup\")\n    return none\nrt = current_runtime()\nthread = spawn(rt, work)\njoin(thread)\n",
            "child-primary",
            "cleanup\n",
            Some("child-cleanup"),
        ),
        (
            "finally-helper-return-preserves-primary",
            "def helper() -> none\n    return none\ndef work() -> none\n    try\n        raise TypeError(code = \"primary\")\n    finally\n        helper()\n        print(\"cleanup\")\n    return none\nwork()\n",
            "primary",
            "cleanup\n",
            None,
        ),
        (
            "nested-finally-helper-return-preserves-primary-and-suppressed",
            "def helper() -> none\n    try\n        raise TypeError(code = \"helper-overridden\")\n    finally\n        return none\ndef work() -> none\n    try\n        raise TypeError(code = \"primary\")\n    finally\n        helper()\n        print(\"cleanup\")\n        raise TypeError(code = \"cleanup-error\")\n    return none\nwork()\n",
            "primary",
            "cleanup\n",
            Some("cleanup-error"),
        ),
        (
            "native-running-cancellation-finally",
            "from std.concurrent import current_runtime, shared_mut, spawn, join, cancel, lock, lock_read, lock_write\ndef read_flag(shared_mut<int> flag) -> int\n    guard = lock(flag)\n    return lock_read(guard)\ndef write_flag(shared_mut<int> flag, int value) -> none\n    guard = lock(flag)\n    lock_write(guard, value)\ndef worker(shared_mut<int> ready) -> int\n    try\n        write_flag(ready, 1)\n        while read_flag(ready) == 1\n            value = 1\n    finally\n        print(\"cancel-finally\")\n    return 0\nrt = current_runtime()\nready = shared_mut(0)\nthread = spawn(rt, worker, ready)\nwhile read_flag(ready) == 0\n    value = 1\ncancel(thread)\njoin(thread)\n",
            "X06-RUNTIME-027",
            "cancel-finally\n",
            None,
        ),
        (
            "native-completed-cancellation",
            "from std.concurrent import current_runtime, spawn, join, cancel, runtime_close\ndef worker() -> int\n    return 3\nrt = current_runtime()\nthread = spawn(rt, worker)\nruntime_close(rt)\ncancel(thread)\njoin(thread)\n",
            "X06-RUNTIME-027",
            "",
            None,
        ),
        (
            "native-stack-frames",
            "def inner() -> none\n    raise TypeError(code = \"stack-primary\")\ndef outer() -> none\n    inner()\nouter()\n",
            "stack-primary",
            "",
            None,
        ),
    ];
    for (label, source, expected_code, expected_output, expected_suppressed) in error_cases {
        let frontend_request = FrontendRequest::from_text(source);
        let artifact = FrontendCompiler::new()
            .compile(&frontend_request)
            .unwrap_or_else(|error| panic!("{label}: 负例源码应通过前端：{error:?}"));
        let vm_trace = root.join(format!("{label}.vm-release-trace.tsv"));
        release_trace_environment.set_path(&vm_trace);
        xiao_runtime::start_release_trace_from_env();
        let vm = FrontendVmDriver::new().run_artifact(
            &artifact,
            &DriverRequest::new(frontend_request.clone()).with_timeout(Duration::from_secs(10)),
        );
        xiao_runtime::flush_release_trace();
        release_trace_environment.reset_runtime_trace();
        assert_eq!(vm.code(), Some(expected_code), "{label}: VM 错误身份");
        assert_eq!(
            vm.exit_code().as_process_code(),
            3,
            "{label}: VM 运行时错误退出码"
        );
        let vm_output = vm
            .as_executed()
            .expect("VM 返回执行结果")
            .events()
            .iter()
            .filter_map(|event| match event {
                xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>()
            .replace("\r\n", "\n");
        assert_eq!(vm_output, expected_output, "{label}: VM stdout");
        let report = vm.report().expect("VM 错误应有结构化报告");
        assert_eq!(
            report
                .suppressed
                .iter()
                .map(|error| error.code.as_str())
                .collect::<Vec<_>>(),
            expected_suppressed.into_iter().collect::<Vec<_>>(),
            "{label}: VM suppressed 错误身份"
        );
        if label == "native-stack-frames" {
            let frames = report
                .stack
                .iter()
                .map(|frame| frame.function.as_str())
                .collect::<Vec<_>>();
            let inner = frames
                .iter()
                .position(|function| *function == "inner")
                .expect("VM 栈应包含 inner");
            let outer = frames
                .iter()
                .position(|function| *function == "outer")
                .expect("VM 栈应包含 outer");
            assert!(inner < outer, "VM 帧应按内到外排列：{frames:?}");
        }
        let output = root.join(format!("{label}.{suffix}"));
        let request =
            NativeBuildRequest::new(frontend_request, target.clone(), toolchain.clone(), &output)
                .with_codegen_options(
                    CodegenOptions::for_target(target.clone()).with_internal_concurrency(),
                );
        let native = FrontendNativeDriver::new()
            .build_artifact(&artifact, &request)
            .unwrap_or_else(|error| panic!("{label}: 原生构建失败：{error}"));
        let native_trace = root.join(format!("{label}.native-release-trace.tsv"));
        release_trace_environment.set_path(&native_trace);
        let execution = run_native_with_timeout(&native.native.executable, label);
        assert_eq!(
            execution.status.code(),
            Some(3),
            "{label}: native 退出码，stdout={:?} stderr={:?}",
            execution.stdout,
            execution.stderr
        );
        let native_output = String::from_utf8(execution.stdout)
            .expect("native stdout 为 UTF-8")
            .replace("\r\n", "\n");
        assert_eq!(native_output, expected_output, "{label}: native stdout");
        let stderr = String::from_utf8(execution.stderr).expect("native stderr 为 UTF-8");
        let expected_identity = format!("xiao-error class=recoverable code={expected_code} ");
        assert!(
            stderr
                .lines()
                .any(|line| line.starts_with(&expected_identity)),
            "{label}: native 主错误身份未匹配：{stderr}"
        );
        if label == "native-stack-frames" {
            let inner = stderr
                .find("at entry::inner (User)")
                .expect("native 报告应包含 inner 帧");
            let outer = stderr
                .find("at entry::outer (User)")
                .expect("native 报告应包含 outer 帧");
            assert!(inner < outer, "native 帧应按内到外排列：{stderr}");
        }
        if let Some(expected_suppressed) = expected_suppressed {
            assert!(
                stderr.contains(expected_suppressed),
                "{label}: suppressed 未投影：{stderr}"
            );
        }
        if label == "native-running-cancellation-finally" {
            // 锁轮询握手次数取决于调度时机，临时守卫的释放序列不适合作为差分基准。
            assert!(
                !release_trace_sequence(&native_trace).is_empty(),
                "{label}: native 应记录握手对象释放"
            );
            assert!(
                !release_trace_sequence(&vm_trace).is_empty(),
                "{label}: VM 应记录握手对象释放"
            );
        } else {
            assert_eq!(
                release_trace_sequence(&native_trace),
                release_trace_sequence(&vm_trace),
                "{label}: VM/native 完整释放对象顺序"
            );
        }
    }

    let detached_source = "from std.concurrent import current_runtime, spawn, join, detach\ndef grandchild() -> int\n    raise ArithmeticError(code = \"detached-child\")\n    return 0\ndef child() -> int\n    rt = current_runtime()\n    nested = spawn(rt, grandchild)\n    detach(nested)\n    return 1\nrt = current_runtime()\nthread = spawn(rt, child)\njoin(thread)\nprint(\"main\")\n";
    let detached_request = FrontendRequest::from_text(detached_source)
        .with_context(FrontendContext::host().with_locale("en-US"));
    let detached_artifact = FrontendCompiler::new()
        .compile(&detached_request)
        .expect("detached 负例应通过前端");
    let detached_vm_trace = root.join("detached-failure.vm-release-trace.tsv");
    release_trace_environment.set_path(&detached_vm_trace);
    xiao_runtime::start_release_trace_from_env();
    let vm_detached = FrontendVmDriver::new().run_artifact(
        &detached_artifact,
        &DriverRequest::new(detached_request.clone()).with_timeout(Duration::from_secs(10)),
    );
    xiao_runtime::flush_release_trace();
    release_trace_environment.reset_runtime_trace();
    assert!(vm_detached.is_success(), "detached 失败不得改变主退出码");
    let detached_report = vm_detached
        .as_executed()
        .unwrap()
        .events()
        .iter()
        .find_map(|event| match event {
            xiao_vm::VmEvent::DetachedFailure { report, .. } => Some(report),
            _ => None,
        })
        .expect("VM 记录 DetachedFailure 事件");
    assert_eq!(detached_report.code, "X06-RUNTIME-028");
    assert!(
        detached_report
            .cause
            .as_deref()
            .is_some_and(|cause| cause.code == "detached-child")
    );
    let vm_detached_projection =
        xiao_diagnostics::render_builtin_localized_text(detached_report, "en-US");
    let detached_path = root.join(format!("detached-failure.{suffix}"));
    let detached_native = FrontendNativeDriver::new()
        .build_artifact(
            &detached_artifact,
            &NativeBuildRequest::new(
                detached_request,
                target.clone(),
                toolchain.clone(),
                &detached_path,
            )
            .with_codegen_options(
                CodegenOptions::for_target(target.clone()).with_internal_concurrency(),
            ),
        )
        .expect("detached native 负例应构建");
    let detached_native_trace = root.join("detached-failure.native-release-trace.tsv");
    release_trace_environment.set_path(&detached_native_trace);
    let detached_output =
        run_native_with_timeout(&detached_native.native.executable, "detached-failure");
    assert_eq!(detached_output.status.code(), Some(0));
    assert_eq!(String::from_utf8(detached_output.stdout).unwrap(), "main\n");
    let detached_stderr = String::from_utf8(detached_output.stderr).unwrap();
    assert!(detached_stderr.contains("X06-RUNTIME-028"));
    assert!(detached_stderr.contains("detached-child"));
    assert!(
        detached_message_projection(&detached_stderr)
            .contains(&detached_message_projection(&vm_detached_projection)),
        "native detached 报告应匹配 VM 的错误身份、cause 与 locale 投影：expected={vm_detached_projection:?}, stderr={detached_stderr:?}"
    );
    assert_eq!(
        release_trace_sequence(&detached_native_trace),
        release_trace_sequence(&detached_vm_trace),
        "detached-failure: VM/native 完整释放对象顺序"
    );
    let _ = std::fs::remove_dir_all(root);
}
