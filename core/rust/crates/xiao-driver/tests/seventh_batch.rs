//! 20C 第七批：整数码点去程、`chr` 回程与 ASCII 小写接口的双路径回归。

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use xiao_codegen_llvm::{NativeRun, NativeRunResult, TargetDescription, Toolchain};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendContext, FrontendNativeDriver,
    FrontendRequest, NativeBuildRequest,
};
use xiao_vm::VmEvent;

static PROJECT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestProject {
    root: PathBuf,
}

impl TestProject {
    fn new(source: &str) -> Self {
        let sequence = PROJECT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xiao-20c-seventh-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("创建第七批隔离项目");
        fs::write(root.join("main.xiao"), source).expect("写入第七批入口");
        Self { root }
    }

    fn write(&self, relative: &str, source: &str) {
        fs::write(self.root.join(relative), source).expect("写入第七批辅助模块");
    }

    fn request(&self, source: &str) -> FrontendRequest {
        let mut context = FrontendContext::host();
        context.project_root = Some(self.root.clone());
        FrontendRequest::from_text_at(source, self.root.join("main.xiao")).with_context(context)
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn output(outcome: &DriverOutcome) -> String {
    outcome
        .as_executed()
        .into_iter()
        .flat_map(|execution| execution.events())
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn vm_run(project: &TestProject, source: &str) -> DriverOutcome {
    let request = project.request(source);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .unwrap_or_else(|error| panic!("第七批源码不应在前端拒绝：{error:?}"));
    xiao_driver::FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(request))
}

#[test]
/// VM 必须同时支持三种整数宽度、单码点去程、`chr` 回程与 ASCII 小写接口。
fn vm_supports_codepoint_round_trip_and_ascii_lowering() {
    let source = "from stdlib.text import lower_ascii\nprint(int(\"A\"))\nprint(sint(\"小\"))\nprint(lint(\"雪\"))\nprint(chr(97))\nprint(chr(sint(128512)))\nprint(lower_ascii(\"ABCxyz-Az-9小\"))\n";
    let project = TestProject::new(source);
    let outcome = vm_run(&project, source);
    assert!(outcome.is_success(), "第七批 VM 成功夹具失败：{outcome:?}");
    assert_eq!(output(&outcome), "65\n23567\n38634\na\n😀\nabcxyz-az-9小\n");

    let dynamic_source = "import helper\nprint(int(helper.value))\n";
    let dynamic_project = TestProject::new(dynamic_source);
    dynamic_project.write("helper.xiao", "value = \"A\"\n");
    let dynamic_outcome = vm_run(&dynamic_project, dynamic_source);
    assert!(
        dynamic_outcome.is_success(),
        "动态 str 去程失败：{dynamic_outcome:?}"
    );
    assert_eq!(output(&dynamic_outcome), "65\n");
}

#[test]
/// 空串、多字符、数字文本和三个非法码点都必须由 Runtime 统一报 `X06-RUNTIME-012`。
fn vm_rejects_invalid_codepoint_inputs_with_one_runtime_code() {
    for source in [
        "print(int(\"\"))\n",
        "print(int(\"AB\"))\n",
        "print(int(\"42\"))\n",
        "print(chr(-1))\n",
        "print(chr(1114112))\n",
        "print(chr(55296))\n",
    ] {
        let project = TestProject::new(source);
        let outcome = vm_run(&project, source);
        assert_eq!(
            outcome.code(),
            Some("X06-RUNTIME-012"),
            "非法码点输入错误身份不一致：{source:?} {outcome:?}"
        );
    }
}

#[test]
/// `chr` 保持整数类型边界，字符串和浮点输入仍在类型层拒绝。
fn chr_keeps_scalar_type_boundary() {
    for source in ["print(chr(\"A\"))\n", "print(chr(1.0))\n"] {
        let project = TestProject::new(source);
        let request = project.request(source);
        let error = FrontendCompiler::new()
            .compile(&request)
            .expect_err("chr 的非整数输入必须在类型层拒绝");
        assert_eq!(
            error
                .diagnostics()
                .iter()
                .find(|diagnostic| diagnostic.is_error())
                .map(|diagnostic| diagnostic.code()),
            Some("X02-TYPE-006")
        );
    }
}

fn configured_native_toolchain() -> (TargetDescription, Toolchain) {
    let target = TargetDescription::host();
    let configured_triple =
        std::env::var("XIAO_TARGET_TRIPLE").expect("原生第七批测试需要 XIAO_TARGET_TRIPLE");
    assert_eq!(configured_triple, target.triple);
    let runtime = PathBuf::from(
        std::env::var_os("XIAO_RUNTIME_LIBRARY").expect("原生第七批测试需要 Runtime staticlib"),
    );
    assert!(
        runtime.is_file(),
        "Runtime staticlib 不存在：{}",
        runtime.display()
    );
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain =
        Toolchain::new(std::env::var_os("XIAO_CLANG").expect("原生第七批测试需要 XIAO_CLANG"))
            .with_llvm_as(
                std::env::var_os("XIAO_LLVM_AS").expect("原生第七批测试需要 XIAO_LLVM_AS"),
            )
            .with_runtime_library(runtime)
            .probe_native_static_libraries(rustc, &target)
            .expect("应能读取 Runtime 原生库清单");
    (target, toolchain)
}

fn native_run(
    project: &TestProject,
    source: &str,
    target: &TargetDescription,
    toolchain: &Toolchain,
    name: &str,
) -> NativeRunResult {
    let request = project.request(source);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .unwrap_or_else(|error| panic!("原生第七批源码不应前端拒绝：{error:?}"));
    let output = std::env::temp_dir().join(format!(
        "xiao-20c-seventh-{name}-{}.exe",
        std::process::id()
    ));
    let native = FrontendNativeDriver::new()
        .build_artifact(
            &artifact,
            &NativeBuildRequest::new(request, target.clone(), toolchain.clone(), output.clone()),
        )
        .unwrap_or_else(|error| panic!("第七批原生构建失败：{error}"));
    let run = NativeRun::new(&native.native)
        .run()
        .unwrap_or_else(|error| panic!("第七批原生产物启动失败：{error}"));
    let _ = fs::remove_file(output);
    run
}

#[test]
#[ignore = "需要受控 LLVM 工具链、Runtime staticlib 和目标三元组"]
/// Windows LLVM 原生路径必须与 VM 共享码点结果和错误身份。
fn native_matches_codepoint_round_trip_and_errors() {
    let (target, toolchain) = configured_native_toolchain();
    let success = "from stdlib.text import lower_ascii\nprint(int(\"A\"))\nprint(sint(\"小\"))\nprint(lint(\"雪\"))\nprint(chr(97))\nprint(chr(sint(128512)))\nprint(lower_ascii(\"ABCxyz-Az-9小\"))\n";
    let project = TestProject::new(success);
    let run = native_run(&project, success, &target, &toolchain, "success");
    assert_eq!(run.status, Some(0), "第七批原生成功夹具状态错误：{run:?}");
    assert_eq!(run.stdout, "65\n23567\n38634\na\n😀\nabcxyz-az-9小\n");
    assert!(run.stderr.is_empty(), "成功夹具不应输出 stderr：{run:?}");

    for (name, source) in [
        ("multi", "print(int(\"42\"))\n"),
        ("negative", "print(chr(-1))\n"),
        ("large", "print(chr(1114112))\n"),
        ("surrogate", "print(chr(55296))\n"),
    ] {
        let project = TestProject::new(source);
        let run = native_run(&project, source, &target, &toolchain, name);
        assert_eq!(run.status, Some(3), "非法输入进程状态错误：{name} {run:?}");
        assert!(
            run.stderr.contains("X06-RUNTIME-012"),
            "非法输入缺少稳定错误码：{name} {run:?}"
        );
    }
}
