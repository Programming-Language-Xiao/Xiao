//! 10A 前端到 LLVM 内部驱动器规格。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use xiao_codegen_llvm::{CodegenError, TargetDescription, Toolchain};
use xiao_diagnostics::ReportClass;
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendArtifact, FrontendCompiler, FrontendNativeDriver,
    FrontendRequest, NativeBuildRequest, NativeBuildResult, NativeDriverError,
};
use xiao_ir::{IrStatementKind, IrType};
use xiao_runtime::{ReleaseEvent, start_release_trace, take_release_events};

/// 读取环境依赖原生测试使用的固定目标三元组。
fn configured_native_target() -> TargetDescription {
    let configured_triple = std::env::var("XIAO_TARGET_TRIPLE")
        .expect("显式运行 --ignored 时 XIAO_TARGET_TRIPLE 必须已设置；准备方式见 10D §4");
    let target = TargetDescription::host();
    assert_eq!(
        configured_triple, target.triple,
        "XIAO_TARGET_TRIPLE 必须与当前 Rust 编译目标一致"
    );
    target
}

/// 读取并验证动态 Runtime 原生回环所需的工具链。
fn configured_dynamic_toolchain() -> (TargetDescription, Toolchain) {
    let clang = std::env::var_os("XIAO_CLANG")
        .expect("显式运行 --ignored 时 XIAO_CLANG 必须已设置；准备方式见 10D §4");
    let runtime = PathBuf::from(
        std::env::var_os("XIAO_RUNTIME_LIBRARY")
            .expect("显式运行 --ignored 时 XIAO_RUNTIME_LIBRARY 必须已设置；准备方式见 10D §4"),
    );
    assert!(
        runtime.is_file(),
        "XIAO_RUNTIME_LIBRARY 必须指向已构建的 Runtime staticlib: {}",
        runtime.display()
    );
    let target = configured_native_target();
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain = Toolchain::new(clang)
        .with_runtime_library(runtime)
        .probe_native_static_libraries(rustc, &target)
        .expect("rustc 应报告 Runtime staticlib 的原生库清单");
    (target, toolchain)
}

/// 使用同一份源码产出前端产物和原生可执行文件。
fn build_native_case(
    source: &str,
    label: &str,
    target: &TargetDescription,
    toolchain: &Toolchain,
) -> (
    FrontendRequest,
    FrontendArtifact,
    NativeBuildResult,
    PathBuf,
) {
    let frontend_request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new()
        .compile(&frontend_request)
        .expect("真实 Xiao 源码应先通过前端");
    let root =
        std::env::temp_dir().join(format!("xiao-driver-n0-c2-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("应能创建原生测试目录");
    let output = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let request = NativeBuildRequest::new(
        frontend_request.clone(),
        target.clone(),
        toolchain.clone(),
        &output,
    );
    let native = FrontendNativeDriver::new()
        .build_artifact(&artifact, &request)
        .unwrap_or_else(|error| panic!("同一份前端产物应能构建原生程序：{error:?}"));
    (frontend_request, artifact, native, root)
}

/// 在原生产物子进程中启用共享 Runtime 释放追踪。
fn run_native_with_trace(executable: &Path, trace_path: &Path) -> Output {
    Command::new(executable)
        .env("XIAO_RUNTIME_RELEASE_TRACE_PATH", trace_path)
        .output()
        .expect("应能启动原生程序")
}

/// 解析原生产物第一行机器错误摘要。
fn parse_machine_error(stderr: &str) -> Option<(&str, &str, &str, &str, &str, i32)> {
    let line = stderr
        .lines()
        .find(|line| line.starts_with("xiao-error "))?
        .strip_prefix("xiao-error ")?;
    let (class, rest) = line.split_once(" code=")?;
    let class = class.strip_prefix("class=")?;
    let (code, rest) = rest.split_once(" message_id=")?;
    let (message_id, rest) = rest.split_once(" params=")?;
    let (params, rest) = rest.split_once(" span=")?;
    let (span, exit_code) = rest.split_once(" exit_code=")?;
    Some((
        class,
        code,
        message_id,
        params,
        span,
        exit_code.parse().ok()?,
    ))
}

/// 对照 VM 与原生的机器错误字段和标准输出。
fn assert_native_matches_vm(outcome: &DriverOutcome, native: &Output) {
    let stdout = String::from_utf8_lossy(&native.stdout);
    let stderr = String::from_utf8_lossy(&native.stderr);
    assert_eq!(stdout, "", "本用例没有用户输出");
    assert_eq!(
        native.status.code(),
        Some(i32::from(outcome.exit_code().as_process_code())),
        "VM 与原生退出码不一致；stderr: {stderr}"
    );
    let Some(report) = outcome.report() else {
        assert!(outcome.is_success(), "无报告结果必须是成功");
        assert!(
            stderr.trim().is_empty(),
            "成功路径不应输出错误报告：{stderr}"
        );
        return;
    };
    let (class, code, message_id, params, span, exit_code) =
        parse_machine_error(&stderr).unwrap_or_else(|| panic!("缺少机器错误摘要：{stderr}"));
    let expected_class = match report.class {
        ReportClass::Recoverable => "recoverable",
        ReportClass::Fatal => "fatal",
    };
    let expected_span = report.location.map_or_else(
        || "<none>".to_owned(),
        |location| format!("{}..{}", location.start(), location.end()),
    );
    assert_eq!(class, expected_class);
    assert_eq!(code, report.code);
    assert_eq!(message_id, report.message_id);
    assert_eq!(params, format!("{:?}", report.params));
    assert_eq!(span, expected_span);
    assert_eq!(exit_code, i32::from(outcome.exit_code().as_process_code()));
}

/// 解析原生产物写出的运行期释放事件。
fn read_native_release_events(path: &Path) -> Vec<(u64, u64, String)> {
    std::fs::read_to_string(path)
        .expect("原生产物应写出释放追踪文件")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut fields = line.split('\t');
            let sequence = fields
                .next()
                .expect("释放事件缺少序号")
                .parse()
                .expect("释放事件序号应为整数");
            let object_id = fields
                .next()
                .expect("释放事件缺少对象身份")
                .parse()
                .expect("释放事件对象身份应为整数");
            let action = fields.next().expect("释放事件缺少动作").to_owned();
            assert!(fields.next().is_none(), "释放事件字段过多：{line}");
            (sequence, object_id, action)
        })
        .collect()
}

/// 将 VM 共享 Runtime 事件转换成跨进程可比较的稳定三元组。
fn release_event_tuples(events: &[ReleaseEvent]) -> Vec<(u64, u64, String)> {
    events
        .iter()
        .map(|event| {
            (
                event.sequence,
                event.object_id,
                event.action.as_str().to_owned(),
            )
        })
        .collect()
}

#[test]
/// 后端请求必须先经过真实 FrontendCompiler，再在工具链边界返回结构化错误。
fn compiles_frontend_before_toolchain() {
    let request = NativeBuildRequest::new(
        FrontendRequest::from_text("value = 1 + 2\n"),
        TargetDescription::host(),
        Toolchain::new("xiao-clang-not-installed"),
        std::env::temp_dir().join("xiao-native-driver-missing.exe"),
    );
    let error = FrontendNativeDriver::new()
        .build(&request)
        .expect_err("工具链应拒绝");
    assert!(matches!(
        error,
        NativeDriverError::Backend(CodegenError::ToolchainUnavailable { .. })
    ));
}

#[test]
/// 前端错误在进入 LLVM 后端前原样保留。
fn preserves_frontend_rejection() {
    let request = NativeBuildRequest::new(
        FrontendRequest::from_text("if 1\n    value = 1\n"),
        TargetDescription::host(),
        Toolchain::new("unused"),
        std::env::temp_dir().join("xiao-native-driver-invalid.exe"),
    );
    assert!(matches!(
        FrontendNativeDriver::new().build(&request),
        Err(NativeDriverError::Frontend(_))
    ));
}

#[test]
/// 未注解函数的最终参数/返回签名必须从类型阶段进入共享 IR。
fn carries_inferred_function_signature_into_ir() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "def count(limit)\n    total = 0\n    while total != limit\n        total = total + 1\n    return total\nresult = count(3)\n",
        ))
        .expect("类型阶段应推断函数签名");
    let function = artifact
        .ir
        .body
        .iter()
        .find_map(|statement| match &statement.kind {
            IrStatementKind::Function {
                name,
                parameters,
                return_type,
                ..
            } if name.text == "count" => Some((parameters, return_type)),
            _ => None,
        })
        .expect("应找到 count 函数");
    assert_eq!(
        function.0[0].ty,
        IrType::Scalar {
            name: "int".to_owned()
        }
    );
    assert_eq!(
        function.1,
        &IrType::Scalar {
            name: "int".to_owned()
        }
    );
}

#[test]
#[ignore = "需要 XIAO_CLANG 与 XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
/// 提供工具链时，真实 Xiao 源码必须完整走前端再生成原生程序。
fn optional_real_frontend_to_native_round_trip() {
    let clang = std::env::var_os("XIAO_CLANG")
        .expect("显式运行 --ignored 时 XIAO_CLANG 必须已设置；准备方式见 10D §4");
    let root = std::env::temp_dir().join(format!("xiao-driver-n0-a-{}", std::process::id()));
    let output = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let target = configured_native_target();
    let request = NativeBuildRequest::new(
        FrontendRequest::from_text(
            "def count(limit)\n    total = 0\n    while total != limit\n        total = total + 1\n    return total\nresult = count(3)\n",
        ),
        target,
        Toolchain::new(clang),
        &output,
    );
    let result = FrontendNativeDriver::new()
        .build(&request)
        .expect("原生构建");
    assert!(result.native.executable.exists());
    let run = xiao_codegen_llvm::NativeRun::new(&result.native)
        .run()
        .expect("启动");
    assert_eq!(run.status, Some(0), "stderr: {}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[ignore = "需要 XIAO_CLANG、XIAO_RUNTIME_LIBRARY 与 XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
/// VM 与原生后端必须消费同一份前端产物，并逐项对照错误、输出和释放事件。
fn optional_frontend_artifact_differential_round_trip() {
    let (target, toolchain) = configured_dynamic_toolchain();
    let cases = [
        ("dynamic-string", "payload = \"held\"\n"),
        ("dynamic-array", "values = [1, 2]\n"),
        (
            "caught",
            "payload = \"held\"\ntry\n    raise ArithmeticError(code = \"CAUGHT\")\ncatch err as ArithmeticError\n    handled = \"yes\"\nfinally\n    cleanup = \"done\"\n",
        ),
        (
            "unmatched",
            "payload = \"held\"\nraise ArithmeticError(code = \"UNMATCHED\")\n",
        ),
    ];
    for (label, source) in cases {
        let (frontend_request, artifact, native, root) =
            build_native_case(source, label, &target, &toolchain);
        let request = DriverRequest::new(frontend_request);
        let trace = start_release_trace();
        let vm = xiao_driver::FrontendVmDriver::new().run_artifact(&artifact, &request);
        let vm_events = take_release_events();
        drop(trace);

        assert_eq!(native.frontend.ir, artifact.ir);
        let trace_path = root.join("release-events.tsv");
        let native_output = run_native_with_trace(&native.native.executable, &trace_path);
        assert_native_matches_vm(&vm, &native_output);
        assert_eq!(
            read_native_release_events(&trace_path),
            release_event_tuples(&vm_events),
            "{label} 用例的 Runtime 释放序列不一致"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}

#[test]
#[ignore = "需要 XIAO_CLANG、XIAO_RUNTIME_LIBRARY 与 XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
/// 可恢复错误走原生路径时必须被 `catch` 消费，不能终止宿主进程。
fn optional_native_catch_does_not_terminate() {
    let (target, toolchain) = configured_dynamic_toolchain();
    let source = "try\n    raise ArithmeticError(code = \"NATIVE_CATCH\")\ncatch err as ArithmeticError\n    handled = true\n";
    let (frontend_request, artifact, native, root) =
        build_native_case(source, "catch-boundary", &target, &toolchain);
    assert_eq!(native.frontend.ir, artifact.ir);
    let run = xiao_codegen_llvm::NativeRun::new(&native.native)
        .run()
        .expect("应能启动原生 catch 程序");
    assert_eq!(run.status, Some(0), "catch 未消费错误：{}", run.stderr);
    assert!(
        run.stderr.trim().is_empty(),
        "成功路径不应报告错误：{}",
        run.stderr
    );
    let _ = frontend_request;
    let _ = std::fs::remove_dir_all(root);
}
