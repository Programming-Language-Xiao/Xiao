//! 20D 进程 API：静态种类、能力、退出码缓存、分离生命周期与 VM/LLVM 差分。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use xiao_codegen_llvm::{CodegenOptions, TargetDescription, Toolchain};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendArtifact, FrontendCompiler, FrontendContext,
    FrontendNativeDriver, FrontendRequest, FrontendVmDriver, NativeBuildRequest,
};
use xiao_intrinsics::{CAPABILITY_PROCESS, KNOWN_CAPABILITY_MASK};

/// 并行测试使用独立的目录和子进程控制文件。
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 测试工程；释放时先允许仍在运行的测试子进程结束。
struct Project(PathBuf);

impl Project {
    /// 创建不会与其它用例冲突的临时根。
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "xiao-20d-process-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("创建进程夹具目录");
        Self(root)
    }

    /// 给真实前端注入项目根。
    fn request(&self, source: &str) -> FrontendRequest {
        let mut context = FrontendContext::host();
        context.project_root = Some(self.0.clone());
        FrontendRequest::from_text_at(source, self.0.join("main.xiao")).with_context(context)
    }

    /// 通过统一前端编译，不执行任何进程入口。
    fn compile(&self, source: &str) -> FrontendArtifact {
        FrontendCompiler::new()
            .compile(&self.request(source))
            .unwrap_or_else(|error| panic!("{source}: {error:?}"))
    }

    /// 实跑 VM，并保留结构化结果供负例校验。
    fn run(&self, source: &str, capabilities: u64) -> DriverOutcome {
        FrontendVmDriver::new().run(
            &DriverRequest::new(self.request(source))
                .with_capabilities(capabilities)
                .expect("有效能力位"),
        )
    }
}

impl Drop for Project {
    /// 即使断言失败，也先放行测试子程序，避免留下被测试协议挂住的进程。
    fn drop(&mut self) {
        let _ = fs::write(self.0.join("release"), "release");
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 将宿主路径嵌入 Xiao 字符串；夹具路径不含用户输入。
fn source_path(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .to_string_lossy()
        .replace('\\', "/")
        .replace('"', "\\\"")
}

/// 只收集 Xiao 的输出事件，保留 stdout 继承测试的宿主输出渠道。
fn vm_stdout(outcome: &DriverOutcome) -> String {
    outcome
        .as_executed()
        .expect("应进入 VM")
        .events()
        .iter()
        .filter_map(|event| match event {
            xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// 首批公共签名均要求一个位置参数。
const IMPORT: &str = "from stdlib.os import spawn, wait, close_process\n";

#[test]
/// 赋值、传参、返回和容器持有共享句柄；重复 wait 使用同一退出码缓存。
fn process_vm_wait_caches_exit_code_and_preserves_kind() {
    let project = Project::new();
    let child = source_path(env!("CARGO_BIN_EXE_process_child_exit7"));
    let source = format!(
        "{IMPORT}def relay(process value) -> process\n    return value\nprocess handle = spawn(\"{child}\")\nalias = relay(handle)\nvalues = [handle, alias]\npair = (handle, 1)\nprint(handle)\nprint(wait(handle))\nprint(wait(alias))\nclose_process(handle)\nclose_process(alias)\n"
    );
    let outcome = project.run(
        &source,
        CAPABILITY_PROCESS | xiao_intrinsics::CAPABILITY_STDOUT,
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(vm_stdout(&outcome), "resource<process>\n7\n7\n");
}

#[test]
/// process 字段的 IR 类型复原与 Runtime 表值匹配不退回 dynamic。
fn process_table_fields_preserve_resource_kind() {
    let project = Project::new();
    let child = source_path(env!("CARGO_BIN_EXE_process_child_exit7"));
    let source = format!(
        "{IMPORT}[[Task]]\n    process handle\n    def init(self, process value) -> none\n        self.handle = value\n    def finish(self) -> int\n        return wait(self.handle)\nhandle = spawn(\"{child}\")\ntask = new Task(handle)\nprint(task.finish())\nclose_process(handle)\n"
    );
    let outcome = project.run(&source, KNOWN_CAPABILITY_MASK);
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(vm_stdout(&outcome), "7\n");
}

#[test]
/// 启动失败带宿主原因，关闭后别名等待使用生命周期错误。
fn process_vm_error_identities_and_closed_alias_are_stable() {
    let project = Project::new();
    let missing = source_path(project.0.join("missing-program"));
    let source = format!("{IMPORT}handle = spawn(\"{missing}\")\n");
    let outcome = project.run(&source, KNOWN_CAPABILITY_MASK);
    assert_eq!(outcome.code(), Some("X06-RUNTIME-012"));
    assert!(format!("{outcome:?}").contains("reason"));
    let child = source_path(env!("CARGO_BIN_EXE_process_child_exit7"));
    let closed = format!(
        "{IMPORT}handle = spawn(\"{child}\")\nalias = handle\nwait(handle)\nclose_process(alias)\nclose_process(handle)\nwait(handle)\n"
    );
    assert_eq!(
        project.run(&closed, KNOWN_CAPABILITY_MASK).code(),
        Some("X06-RUNTIME-025")
    );
}

#[test]
/// 直接入口、公开函数、别名和限定调用共享进程类型与参数形状边界。
fn process_types_and_call_shapes_are_rejected_before_execution() {
    let project = Project::new();
    let imports = [
        (
            "from stdlib.os import spawn, wait, close_process, close, read_text, open_file\n",
            "",
        ),
        (
            "from stdlib.os import spawn as os_spawn, wait as os_wait, close_process as os_close_process, close as os_close, read_text as os_read_text, open_file as os_open_file\n",
            "os_",
        ),
        ("import stdlib.os as platform\n", "platform."),
        ("import stdlib.os\n", "stdlib.os."),
        ("from stdlib import os\n", "os."),
    ];
    for body in [
        "api.wait(api.open_file(\"unused\"))",
        "api.read_text(api.spawn(\"unused\"))",
        "api.spawn(1)",
        "api.close(api.spawn(\"unused\"))",
        "api.close_process(api.open_file(\"unused\"))",
        "resource parent = api.spawn(\"unused\")\napi.close(parent)",
        "api.spawn(program = \"unused\")",
        "api.wait(value = api.spawn(\"unused\"))",
        "api.close_process(value = api.spawn(\"unused\"))",
        "api.spawn(*[\"unused\"])",
        "api.spawn(**{program = \"unused\"})",
    ] {
        for (import, prefix) in imports {
            let source = format!("{import}{}\n", body.replace("api.", prefix));
            let error = FrontendCompiler::new()
                .compile(&project.request(&source))
                .expect_err("进程错误种类或参数形状必须在编译期拒绝");
            assert!(
                error
                    .diagnostics()
                    .iter()
                    .any(|d| d.is_error() && d.code().contains("TYPE")),
                "{source}: {error:?}"
            );
        }
    }
    for source in [
        "os_wait(os_open_file(\"unused\"))",
        "os_close(os_spawn(\"unused\"))",
        "os_close_process(os_open_file(\"unused\"))",
    ] {
        assert!(
            FrontendCompiler::new()
                .compile(&project.request(source))
                .is_err(),
            "{source}"
        );
    }
}

#[test]
/// 进程沿用资源的不可哈希、常量拒绝及路径约束规则。
fn process_container_and_constant_boundaries_are_executable() {
    let project = Project::new();
    for (source, code) in [
        ("const value = os_spawn(\"unused\")", "X02-TYPE-008"),
        ("const process value = os_spawn(\"unused\")", "X02-TYPE-008"),
        (
            "[Config]\n    const value = os_spawn(\"unused\")",
            "X02-TYPE-008",
        ),
        (
            "[[Config]]\n    const value = [os_spawn(\"unused\")]",
            "X02-TYPE-008",
        ),
        ("values = {os_spawn(\"unused\")}", "X03-TYPE-016"),
        ("set<process> values = set()", "X03-TYPE-016"),
        ("process values[1]", "X03-TYPE-006"),
    ] {
        let error = FrontendCompiler::new()
            .compile(&project.request(source))
            .expect_err("进程资源边界必须拒绝");
        assert!(
            error.diagnostics().iter().any(|d| d.code() == code),
            "{source}: {error:?}"
        );
    }
    let mut artifact = project.compile("handle = os_spawn(\"unused\")\n");
    let statement = artifact.ir.body.first_mut().expect("资源创建赋值");
    let xiao_ir::IrStatementKind::Assignment { target, value } = &statement.kind else {
        panic!("赋值形状");
    };
    statement.kind = xiao_ir::IrStatementKind::ConstDeclaration {
        target: target.clone(),
        declared_type: Some("process".to_owned()),
        value: value.clone(),
    };
    let tac = xiao_bytecode::lower_program(&artifact.ir);
    assert_eq!(
        xiao_bytecode::verify_production(&artifact.ir, &tac)
            .expect_err("降低后的进程常量必须拒绝")
            .code,
        "X07-FATAL-002"
    );
    let error = xiao_bytecode::encode(&tac, xiao_bytecode::OperandWidth::Leb128)
        .expect_err("进程常量不能进入字节码产物");
    assert!(error.to_string().contains("X07-FATAL-002"));
}

#[test]
/// Unix 实际信号与 Windows 异常状态均经同一 Runtime 退出码入口归一。
fn process_vm_maps_child_signal_or_exception_exit() {
    let project = Project::new();
    let child = source_path(env!("CARGO_BIN_EXE_process_child_abort"));
    let source = format!(
        "{IMPORT}handle = spawn(\"{child}\")\nprint(wait(handle))\nclose_process(handle)\n"
    );
    let outcome = project.run(&source, KNOWN_CAPABILITY_MASK);
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(
        vm_stdout(&outcome),
        if cfg!(windows) { "139\n" } else { "143\n" }
    );
}

/// 给隔离 VM 测试子进程提供输入和继承上下文，不修改测试进程的全局环境或 cwd。
fn isolated_vm_command(project: &Project, mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("测试程序"));
    command
        .args(["--exact", "process_vm_child", "--ignored", "--nocapture"])
        .current_dir(&project.0)
        .env("XIAO_PROCESS_DRIVER_MODE", mode)
        .env("XIAO_PROCESS_DRIVER_ROOT", &project.0);
    command
}

/// 在子程序等待放行期间确认父程序先完成；随后放行并读取继承管道，避免管道持有导致假阻塞。
fn observe_detached_child(project: &Project, mut command: Command, expect_started: bool) -> Output {
    command
        .env("XIAO_PROCESS_TEST_CONTROL", &project.0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut parent = command.spawn().expect("启动隔离生命周期夹具");
    let deadline = Instant::now() + Duration::from_secs(4);
    let mut returned = false;
    while Instant::now() < deadline {
        if parent.try_wait().expect("读取父程序状态").is_some() {
            returned = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if expect_started && returned {
        let deadline = Instant::now() + Duration::from_secs(4);
        while !project.0.join("started").is_file() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let started = project.0.join("started").is_file();
    let completed_before_release = project.0.join("completed").is_file();
    fs::write(project.0.join("release"), "release").expect("放行子进程");
    let output = parent.wait_with_output().expect("回收测试父程序及其输出");
    assert!(returned, "close/drop 不得等待尚未退出的子进程：{output:?}");
    assert!(
        !completed_before_release,
        "测试子进程必须仍在等待测试端放行"
    );
    if expect_started {
        assert!(started, "分离不能终止子进程：{output:?}");
        assert!(
            project.0.join("completed").is_file(),
            "子进程应在放行后自然结束"
        );
    } else {
        assert!(
            !project.0.join("started").is_file(),
            "缺能力时不能启动子进程"
        );
    }
    output
}

#[test]
/// 通过有界握手验证 VM 关闭不等待、不终止，最后引用释放也只分离。
fn process_close_and_drop_detach_without_waiting_or_killing() {
    for mode in ["close", "drop"] {
        let project = Project::new();
        let output = observe_detached_child(&project, isolated_vm_command(&project, mode), true);
        assert!(output.status.success(), "{mode}: {output:?}");
    }
}

#[test]
/// 能力缺失使用 process 位而非 filesystem 位，并且没有子进程启动副作用。
fn process_missing_capability_does_not_start_a_child() {
    let project = Project::new();
    let output = observe_detached_child(&project, isolated_vm_command(&project, "denied"), false);
    assert!(output.status.success(), "{output:?}");
}

#[test]
/// PATH、环境、cwd 与三个标准流都从父进程继承，不由 Xiao API 增加参数。
fn process_spawn_inherits_path_environment_cwd_and_standard_streams() {
    let project = Project::new();
    fs::write(project.0.join("inherited-cwd"), "cwd").expect("cwd 标记");
    let helper = Path::new(env!("CARGO_BIN_EXE_process_child_exit7"));
    let mut paths = vec![helper.parent().expect("子程序目录").to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let mut command = isolated_vm_command(&project, "inherit");
    command
        .env("PATH", std::env::join_paths(paths).expect("PATH"))
        .env(
            "XIAO_PROCESS_TEST_PROGRAM",
            helper.file_name().expect("程序名"),
        )
        .env("XIAO_PROCESS_TEST_INHERIT", project.0.join("inherited.txt"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("继承测试父程序");
    child
        .stdin
        .take()
        .expect("stdin 管道")
        .write_all(b"inherited-input\n")
        .expect("输入");
    let output = child.wait_with_output().expect("等待继承夹具");
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("child-stdout"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("child-stderr"));
    assert_eq!(
        fs::read_to_string(project.0.join("inherited.txt")).expect("继承结果"),
        "true\ninherited-input\n"
    );
}

#[test]
#[ignore = "仅由父测试在隔离进程与 cwd 下调用"]
/// 承接测试环境，避免默认并行夹具共享进程环境变量。
fn process_vm_child() {
    let Ok(mode) = std::env::var("XIAO_PROCESS_DRIVER_MODE") else {
        return;
    };
    let root = PathBuf::from(std::env::var_os("XIAO_PROCESS_DRIVER_ROOT").expect("测试根"));
    let project = Project(root);
    let program = std::env::var_os("XIAO_PROCESS_TEST_PROGRAM")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_process_child_exit7")));
    let program = source_path(program);
    if mode == "drop" {
        let value = xiao_runtime::RuntimeValue::new_string(program).expect("路径");
        let resource = xiao_runtime::resources::spawn(&value, CAPABILITY_PROCESS).expect("启动");
        drop(resource);
    } else {
        let body = match mode.as_str() {
            "close" => format!(
                "handle = spawn(\"{program}\")\nalias = handle\nclose_process(handle)\nclose_process(alias)\nwait(alias)\n"
            ),
            "denied" => format!("handle = spawn(\"{program}\")\n"),
            "inherit" => format!(
                "handle = spawn(\"{program}\")\nprint(wait(handle))\nclose_process(handle)\n"
            ),
            _ => panic!("未知测试模式"),
        };
        let capabilities = if mode == "denied" {
            KNOWN_CAPABILITY_MASK & !CAPABILITY_PROCESS
        } else {
            KNOWN_CAPABILITY_MASK
        };
        let outcome = project.run(&format!("{IMPORT}{body}"), capabilities);
        if mode == "inherit" {
            assert!(outcome.is_success(), "{outcome:?}");
        } else {
            assert_eq!(
                outcome.code(),
                Some(if mode == "denied" {
                    "X06-RUNTIME-026"
                } else {
                    "X06-RUNTIME-025"
                })
            );
        }
    }
    // 测试目录由外层父进程管理；此处必须保留控制文件供父进程完成观察。
    std::mem::forget(project);
}

/// 原生测试显式要求完整工具链；缺环境不会退化为跳过。
fn native_toolchain() -> (TargetDescription, Toolchain) {
    let target = TargetDescription::host();
    assert_eq!(
        std::env::var("XIAO_TARGET_TRIPLE").expect("原生目标三元组"),
        target.triple
    );
    let runtime =
        PathBuf::from(std::env::var_os("XIAO_RUNTIME_LIBRARY").expect("Runtime staticlib"));
    assert!(runtime.is_file(), "Runtime staticlib 必须存在");
    let toolchain = Toolchain::new(std::env::var_os("XIAO_CLANG").expect("XIAO_CLANG"))
        .with_llvm_as(std::env::var_os("XIAO_LLVM_AS").expect("XIAO_LLVM_AS"))
        .with_runtime_library(runtime)
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .expect("Runtime 原生库清单");
    (target, toolchain)
}

/// 从同一前端产物构建原生程序，能力掩码作为编译选项固化。
fn build_native(
    project: &Project,
    source: &str,
    label: &str,
    capabilities: u64,
    target: &TargetDescription,
    toolchain: &Toolchain,
) -> PathBuf {
    let artifact = project.compile(source);
    let output = project.0.join(format!(
        "{label}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
    FrontendNativeDriver::new()
        .build_artifact(
            &artifact,
            &NativeBuildRequest::new(
                project.request(source),
                target.clone(),
                toolchain.clone(),
                &output,
            )
            .with_codegen_options(
                CodegenOptions::for_target(target.clone())
                    .with_capabilities(capabilities)
                    .expect("有效掩码"),
            ),
        )
        .unwrap_or_else(|error| panic!("{label} 原生构建失败：{error:?}"));
    output
}

#[test]
#[ignore = "需要受控 LLVM 工具链、Runtime staticlib 和目标三元组"]
/// 三平台使用同一子程序，逐条比较 VM/LLVM 输出、错误身份与进程能力。
fn native_process_api_matches_vm() {
    let (target, toolchain) = native_toolchain();
    let project = Project::new();
    let child = source_path(env!("CARGO_BIN_EXE_process_child_exit7"));
    let abort = source_path(env!("CARGO_BIN_EXE_process_child_abort"));
    let missing = source_path(project.0.join("missing-program"));
    let signal_stdout = if cfg!(windows) { "139\n" } else { "143\n" };
    for (label, body, capabilities, expected_stdout, expected_code) in [
        (
            "cached",
            format!(
                "handle = spawn(\"{child}\")\nalias = handle\nprint(handle)\nprint(wait(handle))\nprint(wait(alias))\nclose_process(handle)\nclose_process(alias)\n"
            ),
            KNOWN_CAPABILITY_MASK,
            "resource<process>\n7\n7\n",
            None,
        ),
        (
            "closed",
            format!(
                "handle = spawn(\"{child}\")\nwait(handle)\nalias = handle\nclose_process(handle)\nclose_process(alias)\nwait(alias)\n"
            ),
            KNOWN_CAPABILITY_MASK,
            "",
            Some("X06-RUNTIME-025"),
        ),
        (
            "missing",
            format!("handle = spawn(\"{missing}\")\n"),
            KNOWN_CAPABILITY_MASK,
            "",
            Some("X06-RUNTIME-012"),
        ),
        (
            "denied",
            format!("handle = spawn(\"{child}\")\n"),
            KNOWN_CAPABILITY_MASK & !CAPABILITY_PROCESS,
            "",
            Some("X06-RUNTIME-026"),
        ),
        (
            "signal",
            format!("handle = spawn(\"{abort}\")\nprint(wait(handle))\nclose_process(handle)\n"),
            KNOWN_CAPABILITY_MASK,
            signal_stdout,
            None,
        ),
    ] {
        let source = format!("{IMPORT}{body}");
        let vm = project.run(&source, capabilities);
        assert_eq!(vm.code(), expected_code, "{label}: {vm:?}");
        assert_eq!(vm_stdout(&vm), expected_stdout, "{label}");
        let executable = build_native(&project, &source, label, capabilities, &target, &toolchain);
        let native = Command::new(executable).output().expect("运行原生进程夹具");
        assert_eq!(
            native.status.code(),
            Some(if expected_code.is_some() { 3 } else { 0 }),
            "{label}: {native:?}"
        );
        let stdout = String::from_utf8(native.stdout).expect("UTF-8 输出");
        assert_eq!(stdout, expected_stdout, "{label}");
        let stderr = String::from_utf8_lossy(&native.stderr);
        if let Some(code) = expected_code {
            assert!(
                stderr.contains(&format!("code={code} ")),
                "{label}: {stderr}"
            );
        } else {
            assert!(stderr.is_empty(), "{label}: {stderr}");
        }
        println!("20D-PROCESS {label}: stdout={expected_stdout:?} code={expected_code:?}");
    }
}

#[test]
#[ignore = "需要受控 LLVM 工具链、Runtime staticlib 和目标三元组"]
/// 原生关闭不等待、不 kill；缺 process 能力的程序不启动子进程。
fn native_process_lifecycle_and_capability_have_no_hidden_host_effects() {
    let (target, toolchain) = native_toolchain();
    let child = source_path(env!("CARGO_BIN_EXE_process_child_exit7"));
    for denied in [false, true] {
        let project = Project::new();
        let source = format!(
            "{IMPORT}handle = spawn(\"{child}\")\nclose_process(handle)\nclose_process(handle)\nwait(handle)\n"
        );
        let capabilities = if denied {
            KNOWN_CAPABILITY_MASK & !CAPABILITY_PROCESS
        } else {
            KNOWN_CAPABILITY_MASK
        };
        let executable = build_native(
            &project,
            &source,
            "lifecycle",
            capabilities,
            &target,
            &toolchain,
        );
        let output = observe_detached_child(&project, Command::new(executable), !denied);
        assert_eq!(output.status.code(), Some(3), "{output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(if denied {
                "X06-RUNTIME-026"
            } else {
                "X06-RUNTIME-025"
            }),
            "{stderr}"
        );
    }
}
