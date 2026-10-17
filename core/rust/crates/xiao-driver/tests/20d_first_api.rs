//! 20D 首批 `os` API 的 VM 路径夹具：公开模块、资源类型和能力边界必须同时成立。

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendContext, FrontendNativeDriver,
    FrontendRequest, FrontendVmDriver, NativeBuildRequest, NativeDriverError,
};
use xiao_intrinsics::CAPABILITY_FILESYSTEM;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Project {
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "xiao-20d-first-api-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("创建 20D 测试目录");
        Self { root }
    }

    fn request(&self, source: &str) -> FrontendRequest {
        let mut context = FrontendContext::host();
        context.project_root = Some(self.root.clone());
        FrontendRequest::from_text_at(source, self.root.join("main.xiao")).with_context(context)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn compile(project: &Project, source: &str) -> xiao_driver::FrontendArtifact {
    FrontendCompiler::new()
        .compile(&project.request(source))
        .unwrap_or_else(|error| panic!("20D 首批源码编译失败：{error:?}"))
}

#[test]
fn os_module_opens_reads_and_closes_file_in_vm() {
    let project = Project::new();
    let file = project.root.join("fixture.txt");
    fs::write(&file, "20D\n").expect("写入文件夹具");
    let path = file.to_string_lossy().replace('\\', "/");
    let source = format!(
        "from stdlib.os import open_file, read_text, close\nhandle = open_file(\"{path}\")\ntext = read_text(handle)\nclose(handle)\nprint(text)\n"
    );
    let artifact = compile(&project, &source);
    let request = DriverRequest::new(project.request(&source));
    let outcome = FrontendVmDriver::new().run_artifact(&artifact, &request);
    assert!(outcome.is_success(), "首批 os VM 调用失败：{outcome:?}");
    let output = outcome
        .as_executed()
        .expect("应进入 VM")
        .events()
        .iter()
        .filter_map(|event| match event {
            xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(output, "20D\n\n");
}

#[test]
fn os_module_requires_filesystem_capability_before_open() {
    let project = Project::new();
    let path = project
        .root
        .join("missing.txt")
        .to_string_lossy()
        .replace('\\', "/");
    let source = format!("from stdlib.os import open_file\nhandle = open_file(\"{path}\")\n");
    let artifact = compile(&project, &source);
    let request = DriverRequest::new(project.request(&source))
        .with_capabilities(0)
        .expect("零能力掩码有效");
    let outcome = FrontendVmDriver::new().run_artifact(&artifact, &request);
    assert_eq!(outcome.code(), Some("X06-RUNTIME-026"));
    assert!(matches!(outcome, DriverOutcome::Executed(_)));
}

#[test]
fn directory_cannot_be_passed_to_read_text_at_type_check() {
    let project = Project::new();
    let source = "from stdlib.os import open_directory, read_text\nhandle = open_directory(\".\")\ntext = read_text(handle)\n";
    let error = FrontendCompiler::new()
        .compile(&project.request(source))
        .expect_err("directory -> read_text 必须在类型层拒绝");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X04-TYPE-002"),
        "应得到类型层诊断：{:?}",
        error.diagnostics()
    );
}

#[test]
fn resource_const_is_rejected_with_non_constant_diagnostic() {
    let project = Project::new();
    let source = "const file handle = os_open_file(\"missing.txt\")\n";
    let error = FrontendCompiler::new()
        .compile(&project.request(source))
        .expect_err("资源值不能进入 const");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X02-TYPE-008")
    );
}

#[test]
fn filesystem_capability_bit_is_the_frozen_bit_zero() {
    assert_eq!(CAPABILITY_FILESYSTEM, 1);
}

#[test]
#[ignore = "需要受控 LLVM 工具链、Runtime staticlib 和目标三元组；准备方式见 10D §4"]
fn native_os_module_matches_vm_output() {
    let clang =
        std::env::var_os("XIAO_CLANG").expect("原生 20D 夹具需要 XIAO_CLANG；准备方式见 10D §4");
    let llvm_as = std::env::var_os("XIAO_LLVM_AS")
        .expect("原生 20D 夹具需要 XIAO_LLVM_AS；准备方式见 10D §4");
    let runtime = PathBuf::from(
        std::env::var_os("XIAO_RUNTIME_LIBRARY")
            .expect("原生 20D 夹具需要 XIAO_RUNTIME_LIBRARY；准备方式见 10D §4"),
    );
    let target = TargetDescription::host();
    assert_eq!(
        std::env::var("XIAO_TARGET_TRIPLE").expect("原生目标三元组"),
        target.triple
    );
    assert!(
        runtime.is_file(),
        "Runtime staticlib 不存在：{}",
        runtime.display()
    );
    let toolchain = Toolchain::new(clang)
        .with_llvm_as(llvm_as)
        .with_runtime_library(runtime)
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .expect("原生 Runtime 库清单应可读取");

    let project = Project::new();
    let fixture = project.root.join("fixture.txt");
    fs::write(&fixture, "20D-native\n").expect("写入原生文件夹具");
    let path = fixture.to_string_lossy().replace('\\', "/");
    let source = format!(
        "from stdlib.os import open_file, read_text, close\nhandle = open_file(\"{path}\")\ntext = read_text(handle)\nclose(handle)\nprint(text)\n"
    );
    let frontend_request = project.request(&source);
    let artifact = compile(&project, &source);
    let output_path = project.root.join(if cfg!(windows) {
        "20d-first-api-native.exe"
    } else {
        "20d-first-api-native"
    });
    let native = FrontendNativeDriver::new()
        .build_artifact(
            &artifact,
            &NativeBuildRequest::new(frontend_request.clone(), target, toolchain, output_path),
        )
        .unwrap_or_else(|error| match error {
            NativeDriverError::Frontend(error) => panic!("原生前端失败：{error}"),
            NativeDriverError::Backend(error) => panic!("原生后端失败：{error}"),
        });
    let vm = FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(frontend_request));
    assert!(vm.is_success(), "20D VM 执行失败：{vm:?}");
    let vm_output = vm
        .as_executed()
        .expect("VM 应进入执行阶段")
        .events()
        .iter()
        .filter_map(|event| match event {
            xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    let run = Command::new(&native.native.executable)
        .output()
        .expect("启动原生 20D 夹具");
    assert_eq!(run.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&run.stdout), vm_output);
    assert!(run.stderr.is_empty(), "原生成功路径不应有 stderr");
}
