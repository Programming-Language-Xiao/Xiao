//! 22D 原生并发 lowering 的同源 IR 与 fail-closed 回归。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use xiao_codegen_llvm::{CodegenOptions, NativeArtifact, lower_program, native_concurrency_plan};
use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_driver::{
    DriverRequest, FrontendCompiler, FrontendContext, FrontendNativeDriver, FrontendRequest,
    FrontendVmDriver, NativeBuildRequest,
};

#[path = "concurrency_closure/fixtures.rs"]
/// 由同一前端馈给 VM 与 LLVM 的并发黄金源码。
mod closure_fixtures;

#[path = "concurrency_closure/release_trace.rs"]
/// 黄金夹具共享的 TSV 校验和释放差分口径。
mod release_trace;

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
fn run_native_with_timeout(artifact: &NativeArtifact, label: &str, trace: &Path) -> Output {
    let output = run_traced_executable(&artifact.executable, label, trace);
    println!(
        "22D-NATIVE {label} exit={:?} verification={}",
        output.status.code(),
        artifact.artifact_runtime.verification.as_str(),
    );
    output
}

/// 将轨迹路径随环境传给子进程并排空输出管道。
fn run_traced_executable(executable: &Path, label: &str, trace: &Path) -> Output {
    let child = Command::new(executable)
        .env("XIAO_RUNTIME_RELEASE_TRACE_PATH", trace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("{label}: 启动 native 程序失败：{error}"));
    wait_child_with_timeout(child, label, Duration::from_secs(10))
}

/// 将 F1 的取消固定在 worker 已复制 lock 实参、尚未进 Runtime 锁入口的时点。
fn with_controlled_lock_cancellation(text: &str, source: &str) -> String {
    let expression = "lock(gate)";
    let start = source.find(expression).expect("F1 worker 的锁表达式");
    let end = start + expression.len();
    let span = format!("xiao_runtime_error_attach_span(i64 {start}, i64 {end})");
    let check = " = call i32 @xiao_runtime_concurrency_cancellation_check(";
    let mut lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
    let sites = lines
        .windows(5)
        .enumerate()
        .filter(|(_, window)| window[0].contains(check) && window[4].contains(&span))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let &[site] = sites.as_slice() else {
        panic!("F1 worker 的 lock 取消检查应有唯一调用点：{sites:?}");
    };
    let call = &lines[site];
    let (status, _) = call.trim_start().split_once(check).expect("F1 取消调用");
    lines[site] = format!(
        "  br label %test.lock.cancel\n\
         test.lock.cancel:\n{call}\n  %test.lock.cancelled = icmp ne i32 {status}, 0\n  \
         br i1 %test.lock.cancelled, label %test.lock.cancel.done, label %test.lock.cancel\n\
         test.lock.cancel.done:"
    );
    lines.join("\n") + "\n"
}

/// 收集退出码和两个输出流，达到既有期限时回收进程。
fn wait_child_with_timeout(mut child: Child, label: &str, timeout: Duration) -> Output {
    use std::io::Read;
    let read_stream = |mut stream: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).expect("读取子进程输出");
            bytes
        })
    };
    let stdout = child
        .stdout
        .take()
        .map(|stream| read_stream(Box::new(stream)));
    let stderr = child
        .stderr
        .take()
        .map(|stream| read_stream(Box::new(stream)));
    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    loop {
        if child
            .try_wait()
            .unwrap_or_else(|error| panic!("{label}: 轮询子进程失败：{error}"))
            .is_some()
        {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            timed_out = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let status = child
        .wait()
        .unwrap_or_else(|error| panic!("{label}: 回收子进程失败：{error}"));
    let output = Output {
        status,
        stdout: stdout
            .map(|reader| reader.join().expect("stdout 读取线程"))
            .unwrap_or_default(),
        stderr: stderr
            .map(|reader| reader.join().expect("stderr 读取线程"))
            .unwrap_or_default(),
    };
    assert!(
        !timed_out,
        "{label}: 程序超过 {timeout:?}，stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// 管理 VM 内跨线程追踪所需的进程环境，并在退出时恢复原会话。
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

fn detached_message_projection(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let line = line.trim_start();
            !line.starts_with("at bytes ") && !line.starts_with("at ::")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// detached 成功夹具声明两条输出可交换，其余源码保持逐字黄金断言。
fn assert_case_stdout(label: &str, actual: &str, expected: &str) {
    if label == "detach-success" {
        assert!(actual.ends_with('\n'), "{label}: print 输出保留换行");
        let mut actual = actual.lines().collect::<Vec<_>>();
        let mut expected = expected.lines().collect::<Vec<_>>();
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected, "{label}: stdout 多重集合");
    } else {
        assert_eq!(actual, expected, "{label}: stdout 黄金输出");
    }
}

/// 比较主错误、cause/suppressed 的类别、身份和嵌套顺序，语言文案不参与身份断言。
fn error_identity_projection(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let (class, rest) = if let Some(rest) = trimmed.strip_prefix("error [") {
                ("recoverable", rest)
            } else if let Some(rest) = trimmed.strip_prefix("fatal [") {
                ("fatal", rest)
            } else if matches!(trimmed, "caused by:" | "suppressed:") {
                return Some((line.len() - trimmed.len(), trimmed.to_owned()));
            } else {
                return None;
            };
            let (code, _) = rest.split_once(']')?;
            Some((line.len() - trimmed.len(), format!("{class}:{code}")))
        })
        .collect()
}

#[test]
/// 新黄金场景必须使用真实、可通过统一前端及 LLVM lowering 的源码。
fn closure_golden_sources_compile_and_lower() {
    let successes = closure_fixtures::success_cases();
    let errors = closure_fixtures::error_cases();
    for (label, source) in successes
        .iter()
        .map(|(label, source, _)| (*label, source))
        .chain(errors.iter().map(|(label, source, ..)| (*label, source)))
    {
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source.clone()))
            .unwrap_or_else(|error| panic!("{label}: 真实源码应通过前端：{error:?}"));
        lower_program(
            &artifact.ir,
            &CodegenOptions::default().with_internal_concurrency(),
        )
        .unwrap_or_else(|error| panic!("{label}: 源码应可 native lowering：{error}"));
    }
}

#[test]
#[ignore = "需要受控 XIAO_CLANG 和 XIAO_TARGET_TRIPLE；三平台原生作业精确执行"]
/// C-11 用同一 C 探针生成真实 COFF/ELF/Mach-O 对象，并对照 ABI crate 的 Rust 布局。
fn native_concurrency_shared_c_layout_probe() {
    use std::mem::{align_of, offset_of, size_of};
    use xiao_runtime_abi::{XiaoSharedHandle, XiaoSharedHeader};
    let target = TargetDescription::host();
    assert_eq!(
        std::env::var("XIAO_TARGET_TRIPLE").expect("XIAO_TARGET_TRIPLE"),
        target.triple
    );
    let header = [
        size_of::<XiaoSharedHeader>(),
        align_of::<XiaoSharedHeader>(),
        offset_of!(XiaoSharedHeader, strong),
        offset_of!(XiaoSharedHeader, weak),
        offset_of!(XiaoSharedHeader, tag),
        offset_of!(XiaoSharedHeader, lock_state),
        offset_of!(XiaoSharedHeader, reserved),
    ];
    let handle = [
        size_of::<XiaoSharedHandle>(),
        align_of::<XiaoSharedHandle>(),
        offset_of!(XiaoSharedHandle, runtime_id),
        offset_of!(XiaoSharedHandle, object_ptr),
    ];
    assert_eq!(header, [32, 8, 0, 8, 16, 24, 28]);
    assert_eq!(handle, [16, 8, 0, 8]);
    let root = std::env::temp_dir().join(format!("xiao-22d-layout-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建 C 布局探针目录");
    let source = root.join("layout.c");
    let object = root.join(if cfg!(windows) {
        "layout.obj"
    } else {
        "layout.o"
    });
    fs::write(&source, include_str!("concurrency_closure/layout.c")).expect("写入同源 C 探针");
    let child = Command::new(std::env::var_os("XIAO_CLANG").expect("XIAO_CLANG"))
        .args(["-target", &target.triple, "-std=c11", "-c"])
        .arg(&source)
        .arg("-o")
        .arg(&object)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("启动目标 clang");
    let output = wait_child_with_timeout(child, "shared C layout", Duration::from_secs(30));
    assert!(
        output.status.success(),
        "C 布局探针编译失败：{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(&object).expect("clang 必须生成目标对象");
    assert!(bytes.len() > 64, "目标对象不能为空");
    println!(
        "22D-LAYOUT target={} header={header:?} handle={handle:?}",
        target.triple
    );
    fs::remove_dir_all(root).expect("回收 C 探针目录");
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
/// 22E 的公开构建与内部构建使用同一套嵌入元数据，不依赖内部开关绕过验收。
fn public_native_build_emits_required_concurrency_metadata() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "from std.concurrent import current_runtime\nrt = current_runtime()\n",
        ))
        .expect("并发源码的前端语义有效");
    let module =
        lower_program(&artifact.ir, &CodegenOptions::default()).expect("公开构建已接入元数据验收");
    assert!(module.text.contains("@xiao_native_metadata ="));
    assert!(module.text.contains("@llvm.used"));
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
    let trace_parent = std::env::var_os("XIAO_22D_TRACE_DIR").map(PathBuf::from);
    let selected_case = std::env::var_os("XIAO_22D_CASE")
        .map(|case| case.into_string().expect("XIAO_22D_CASE 必须是 UTF-8"));
    if let Some(label) = &selected_case {
        assert!(
            trace_parent.is_some(),
            "单夹具执行必须指定 XIAO_22D_TRACE_DIR"
        );
        assert!(
            matches!(
                label.as_str(),
                "native-lock-wait-027"
                    | "xcon-bounded-string-success"
                    | "spawn-join-str-multiargument"
            ),
            "无法选择 22D 单夹具 {label}"
        );
    }
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
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "xiao-22d-native-{}-{timestamp}",
        std::process::id()
    ));
    fs::create_dir(&root).expect("创建原生并发临时目录");
    let trace_root = if let Some(parent) = trace_parent {
        let directory = parent.join(root.file_name().expect("22D 临时目录名"));
        fs::create_dir(&directory).expect("创建独立的 22D 轨迹目录");
        directory
    } else {
        root.clone()
    };
    println!("22D-TRACE-DIR {}", trace_root.display());
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
    let closure_successes = closure_fixtures::success_cases();
    for (label, source, expected_output) in cases.into_iter().chain(
        closure_successes
            .iter()
            .map(|(label, source, stdout)| (*label, source.as_str(), stdout.as_str())),
    ) {
        if selected_case.as_deref().is_some_and(|case| case != label) {
            continue;
        }
        println!("22D-CASE {label}: start");
        let frontend_request = FrontendRequest::from_text(source);
        let artifact = FrontendCompiler::new()
            .compile(&frontend_request)
            .unwrap_or_else(|error| panic!("{label}: 差分源码应通过统一前端：{error:?}"));
        let vm_trace = trace_root.join(format!("{label}.vm-release-trace.tsv"));
        release_trace_environment.set_path(&vm_trace);
        xiao_runtime::start_release_trace_from_env();
        let vm = FrontendVmDriver::new().run_artifact(
            &artifact,
            &DriverRequest::new(frontend_request.clone()).with_timeout(Duration::from_secs(10)),
        );
        assert!(vm.is_success(), "{label}: VM 场景必须成功：{vm:?}");
        assert!(
            !vm.as_executed()
                .unwrap()
                .events()
                .iter()
                .any(|event| matches!(event, xiao_vm::VmEvent::DetachedFailure { .. })),
            "{label}: VM 成功场景不得有 detached 错误"
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
        xiao_runtime::flush_release_trace();
        release_trace_environment.reset_runtime_trace();
        assert_case_stdout(label, &vm_output, expected_output);
        if label == "multi-runtime-parallel" {
            let ids = vm
                .as_executed()
                .unwrap()
                .events()
                .iter()
                .filter_map(|event| match event {
                    xiao_vm::VmEvent::ConcurrencyObserved {
                        operation,
                        runtime_id,
                        ..
                    } if operation == "spawn" => Some(*runtime_id),
                    _ => None,
                })
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(ids.len(), 2, "{label}: spawn 必须属于两个不同 Runtime");
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
        let native_trace = trace_root.join(format!("{label}.native-release-trace.tsv"));
        let execution = run_native_with_timeout(&native.native, label, &native_trace);
        assert_eq!(execution.status.code(), Some(0), "{label}");
        assert!(
            execution.stderr.is_empty(),
            "{label}: 成功场景不得写入 stderr：{}",
            String::from_utf8_lossy(&execution.stderr)
        );
        let native_output = String::from_utf8(execution.stdout)
            .expect("原生 stdout 为 UTF-8")
            .replace("\r\n", "\n");
        assert_case_stdout(label, &native_output, &vm_output);
        assert_case_stdout(label, &native_output, expected_output);
        release_trace::assert_case(label, &native_trace, &vm_trace);
        println!("22D-CASE {label}: ok");
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
    let closure_errors = closure_fixtures::error_cases();
    for (label, source, expected_code, expected_output, expected_suppressed) in
        error_cases.into_iter().chain(closure_errors.iter().map(
            |(label, source, code, stdout, suppressed)| {
                (*label, source.as_str(), *code, *stdout, *suppressed)
            },
        ))
    {
        if selected_case.as_deref().is_some_and(|case| case != label) {
            continue;
        }
        println!("22D-CASE {label}: start");
        let frontend_request = FrontendRequest::from_text(source);
        let artifact = FrontendCompiler::new()
            .compile(&frontend_request)
            .unwrap_or_else(|error| panic!("{label}: 负例源码应通过前端：{error:?}"));
        let vm_trace = trace_root.join(format!("{label}.vm-release-trace.tsv"));
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
        let native_trace = trace_root.join(format!("{label}.native-release-trace.tsv"));
        let execution = run_native_with_timeout(&native.native, label, &native_trace);
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
        let vm_report_text = xiao_diagnostics::render_builtin_localized_text(report, "zh-CN");
        assert_eq!(
            error_identity_projection(&stderr),
            error_identity_projection(&vm_report_text),
            "{label}: VM/native 主错误与 cause/suppressed 结构"
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
        release_trace::assert_case(label, &native_trace, &vm_trace);
        if label == release_trace::LOCK_WAIT && selected_case.is_none() {
            let label = release_trace::LOCK_WAIT_PRECHECK;
            let controlled_ir =
                with_controlled_lock_cancellation(&native.native.module.text, source);
            let executable = toolchain
                .compile_with_runtime_components(
                    &controlled_ir,
                    &target,
                    root.join(format!("{label}.{suffix}")),
                    None,
                    &native.native.module.runtime_components,
                )
                .unwrap_or_else(|error| panic!("{label}: 原生构建失败：{error}"));
            let trace = trace_root.join(format!("{label}.native-release-trace.tsv"));
            let execution = run_traced_executable(&executable, label, &trace);
            println!("22D-PRECHECK {label} exit={:?}", execution.status.code());
            assert_eq!(execution.status.code(), Some(3), "{label}: native 退出码");
            let stdout = String::from_utf8(execution.stdout).expect("native stdout 为 UTF-8");
            assert_eq!(stdout.replace("\r\n", "\n"), expected_output, "{label}");
            let controlled_error =
                String::from_utf8(execution.stderr).expect("native stderr 为 UTF-8");
            assert_eq!(
                error_identity_projection(&controlled_error),
                error_identity_projection(&stderr),
                "{label}"
            );
            release_trace::assert_case(label, &trace, &vm_trace);
            println!("22D-REGRESSION {label}: ok");
        }
        if matches!(
            label,
            "native-queued-cancellation" | "xcon-oversized-029" | "pool-submit-after-shutdown-030"
        ) {
            assert!(!vm.as_executed().unwrap().events().iter().any(|event| matches!(event,
                xiao_vm::VmEvent::FunctionEntered { function, .. } if function == "queued" || function == "work")),
                "{label}: VM 不得启动目标入口");
        }
        println!("22D-CASE {label}: ok");
    }

    if selected_case.is_none() {
        assert_detached_failure(
            &root,
            &trace_root,
            &release_trace_environment,
            target,
            toolchain,
            suffix,
        );
    }
    let _ = fs::remove_dir_all(root);
}

/// 对照入口收尾阶段的 detached 报告、退出码和释放动作。
fn assert_detached_failure(
    root: &Path,
    trace_root: &Path,
    release_trace_environment: &ReleaseTraceEnvironment,
    target: TargetDescription,
    toolchain: Toolchain,
    suffix: &str,
) {
    println!("22D-CASE detached-failure: start");
    let detached_source = "from std.concurrent import current_runtime, spawn, join, detach\ndef grandchild() -> int\n    raise ArithmeticError(code = \"detached-child\")\n    return 0\ndef child() -> int\n    rt = current_runtime()\n    nested = spawn(rt, grandchild)\n    detach(nested)\n    return 1\nrt = current_runtime()\nthread = spawn(rt, child)\njoin(thread)\nprint(\"main\")\n";
    let detached_request = FrontendRequest::from_text(detached_source)
        .with_context(FrontendContext::host().with_locale("en-US"));
    let detached_artifact = FrontendCompiler::new()
        .compile(&detached_request)
        .expect("detached 负例应通过前端");
    let detached_vm_trace = trace_root.join("detached-failure.vm-release-trace.tsv");
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
    let detached_native_trace = trace_root.join("detached-failure.native-release-trace.tsv");
    let detached_output = run_native_with_timeout(
        &detached_native.native,
        "detached-failure",
        &detached_native_trace,
    );
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
    release_trace::assert_case(
        "detached-failure",
        &detached_native_trace,
        &detached_vm_trace,
    );
    println!("22D-CASE detached-failure: ok");
}
