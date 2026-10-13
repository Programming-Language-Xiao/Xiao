//! 20C 首批官方库模块的四件套回归。
//!
//! 测试直接读取 `stdlib/` 规范源码并追加最小调用者，避免把官方库伪装成普通包；
//! 原生测试使用同一份源码和调用者，在受控工具链缺失时保持 ignored。

use std::fs;
use std::path::{Path, PathBuf};

use xiao_codegen_llvm::{NativeRun, TargetDescription, Toolchain};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendNativeDriver, FrontendRequest,
    FrontendVmDriver, NativeBuildRequest, NativeDriverError,
};
use xiao_vm::VmEvent;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("xiao-driver crate 应位于仓库 core/rust/crates 下")
        .to_path_buf()
}

fn module_source(module: &str) -> (PathBuf, String) {
    let path = repository_root()
        .join("stdlib")
        .join(format!("{module}.xiao"));
    let source = fs::read_to_string(&path).expect("读取官方库模块源码");
    (path, source)
}

fn request(path: &Path, source: &str) -> FrontendRequest {
    FrontendRequest::from_text_at(source, path)
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

fn run_vm(module: &str, caller: &str) -> DriverOutcome {
    let (path, module_source) = module_source(module);
    let source = format!("{module_source}\n{caller}");
    let frontend_request = request(&path, &source);
    let artifact = FrontendCompiler::new()
        .compile(&frontend_request)
        .unwrap_or_else(|error| panic!("{module} 模块 VM 编译失败：{error:?}"));
    FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(frontend_request))
}

fn first_error_code(source_path: &Path, source: &str) -> Option<String> {
    let request = request(source_path, source);
    FrontendCompiler::new()
        .compile(&request)
        .err()
        .and_then(|error| {
            error
                .diagnostics()
                .iter()
                .find(|diagnostic| diagnostic.is_error())
                .or_else(|| error.diagnostics().first())
                .map(|diagnostic| diagnostic.code().to_owned())
        })
}

#[test]
fn text_module_vm_result_and_removal_boundary_are_stable() {
    let caller = "if not contains_char(\"lib/core\", \"/\")\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nif contains_char(\"lib/core\", \"x\")\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nprint(\"ok\")\n";
    let outcome = run_vm("text", caller);
    assert!(outcome.is_success(), "text VM 调用失败：{outcome:?}");
    assert_eq!(output(&outcome), "ok\n");

    let (path, source) = module_source("text");
    let caller_without_module = "print(contains_char(\"lib/core\", \"/\"))\n";
    assert!(
        first_error_code(&path, caller_without_module).is_some(),
        "移除 text 模块实现后，调用者必须失败"
    );
    let wrong_type = format!("{source}\nprint(contains_char(1, \"/\"))\n");
    assert!(
        first_error_code(&path, &wrong_type).is_some(),
        "text 参数类型错误必须在前端拒绝"
    );
    assert!(
        !source.contains('[') && !source.contains("dict") && !source.contains("serialize"),
        "text 模块不得构造容器、序列化对象或字典键"
    );
}

#[test]
fn path_module_vm_result_and_removal_boundary_are_stable() {
    let outcome = run_vm("path", "print(join(\"lib\", \"core\"))\n");
    assert!(outcome.is_success(), "path VM 调用失败：{outcome:?}");
    assert_eq!(output(&outcome), "lib/core\n");

    let (path, source) = module_source("path");
    let caller_without_module = "print(join(\"lib\", \"core\"))\n";
    assert!(
        first_error_code(&path, caller_without_module).is_some(),
        "移除 path 模块实现后，调用者必须失败"
    );
    let wrong_type = format!("{source}\nprint(join(\"lib\", 1))\n");
    assert!(
        first_error_code(&path, &wrong_type).is_some(),
        "path 参数类型错误必须在前端拒绝"
    );
    assert!(
        !source.contains('[') && !source.contains("dict") && !source.contains("serialize"),
        "path 模块不得构造容器、序列化对象或字典键"
    );
}

fn configured_native_toolchain() -> (TargetDescription, Toolchain) {
    let target = TargetDescription::host();
    let configured_triple = std::env::var("XIAO_TARGET_TRIPLE")
        .expect("原生首批模块测试需要 XIAO_TARGET_TRIPLE；准备方式见 10D §4");
    assert_eq!(configured_triple, target.triple);
    let runtime = PathBuf::from(
        std::env::var_os("XIAO_RUNTIME_LIBRARY")
            .expect("原生首批模块测试需要 XIAO_RUNTIME_LIBRARY；准备方式见 10D §4"),
    );
    assert!(
        runtime.is_file(),
        "Runtime staticlib 不存在：{}",
        runtime.display()
    );
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain = Toolchain::new(
        std::env::var_os("XIAO_CLANG").expect("原生首批模块测试需要 XIAO_CLANG；准备方式见 10D §4"),
    )
    .with_llvm_as(
        std::env::var_os("XIAO_LLVM_AS")
            .expect("原生首批模块测试需要 XIAO_LLVM_AS；准备方式见 10D §4"),
    )
    .with_runtime_library(runtime)
    .probe_native_static_libraries(rustc, &target)
    .expect("应能读取 Runtime 原生库清单");
    (target, toolchain)
}

#[test]
#[ignore = "需要受控 LLVM 工具链、Runtime staticlib 和目标三元组；准备方式见 10D §4"]
fn first_modules_match_native_results() {
    let (target, toolchain) = configured_native_toolchain();
    let root = std::env::temp_dir().join(format!("xiao-20c-first-modules-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建首批模块原生测试目录");
    let cases = [
        (
            "text",
            "if not contains_char(\"lib/core\", \"/\")\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nif contains_char(\"lib/core\", \"x\")\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nprint(\"ok\")\n",
            "ok\n",
        ),
        ("path", "print(join(\"lib\", \"core\"))\n", "lib/core\n"),
    ];
    for (module, caller, expected_stdout) in cases {
        let (path, module_text) = module_source(module);
        let source = format!("{module_text}\n{caller}");
        let frontend_request = request(&path, &source);
        let artifact = FrontendCompiler::new()
            .compile(&frontend_request)
            .unwrap_or_else(|error| panic!("{module} 原生前端失败：{error:?}"));
        let output_path = root.join(if cfg!(windows) {
            format!("{module}.exe")
        } else {
            module.to_owned()
        });
        let build_request = NativeBuildRequest::new(
            frontend_request,
            target.clone(),
            toolchain.clone(),
            output_path,
        );
        let native = FrontendNativeDriver::new()
            .build_artifact(&artifact, &build_request)
            .unwrap_or_else(|error| match error {
                NativeDriverError::Frontend(frontend) => {
                    panic!("{module} 原生前端失败：{frontend}")
                }
                NativeDriverError::Backend(backend) => panic!("{module} 原生后端失败：{backend}"),
            });
        let run = NativeRun::new(&native.native)
            .run()
            .unwrap_or_else(|error| panic!("{module} 原生产物启动失败：{error}"));
        assert_eq!(run.status, Some(0), "{module} 原生退出状态不符：{run:?}");
        assert_eq!(run.stdout, expected_stdout, "{module} VM/原生输出不一致");
        assert!(
            run.stderr.is_empty(),
            "{module} 成功路径不应输出 stderr：{}",
            run.stderr
        );
    }
    let _ = fs::remove_dir_all(root);
}
