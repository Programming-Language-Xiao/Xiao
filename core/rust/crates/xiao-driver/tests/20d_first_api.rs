//! 20D 首批 `os` API 的 VM 路径夹具：公开模块、资源类型和能力边界必须同时成立。

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use xiao_codegen_llvm::{CodegenOptions, TargetDescription, Toolchain};
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
/// 资源集合注解先解析，再由类型层统一拒绝不可哈希成员。
fn resource_set_annotations_are_rejected_by_type_check() {
    let project = Project::new();
    for kind in ["file", "directory", "resource", "int | file"] {
        for initializer in ["", " = set()"] {
            let source = format!("set<{kind}> values{initializer}\n");
            for source in [source.clone(), format!("[ResourceSet]\n    {source}")] {
                let error = FrontendCompiler::new()
                    .compile(&project.request(&source))
                    .expect_err("资源类型不能成为集合成员");
                assert!(
                    error
                        .diagnostics()
                        .iter()
                        .any(|d| d.code() == "X03-TYPE-016"),
                    "{source}: {error:?}"
                );
                assert!(
                    !error
                        .diagnostics()
                        .iter()
                        .any(|d| d.code().contains("PARSE")),
                    "{error:?}"
                );
            }
        }
    }
}

#[test]
/// 内部入口和公开模块都在前端拒绝关键字/展开实参。
fn resource_api_rejects_keyword_and_expanded_arguments_before_execution() {
    let project = Project::new();
    for call in [
        "open_file(path = \"x\")",
        "open_directory(path = \".\")",
        "close(value = os_open_file(\"x\"))",
        "read_text(value = os_open_file(\"x\"))",
        "os_open_file(path = \"x\")",
        "os_open_directory(path = \".\")",
        "os_close(value = os_open_file(\"x\"))",
        "os_read_text(value = os_open_file(\"x\"))",
        "os_open_file(*[\"x\"])",
        "os_open_file(**{path = \"x\"})",
    ] {
        let source =
            format!("from stdlib.os import open_file, open_directory, close, read_text\n{call}\n");
        let error = FrontendCompiler::new()
            .compile(&project.request(&source))
            .expect_err("资源入口不接受关键字/展开参数");
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|d| matches!(d.code(), "X02-TYPE-005" | "X04-TYPE-002")),
            "{call}: {error:?}"
        );
    }
}

#[test]
/// 在隔离子进程设置 cwd，避免并行测试修改当前进程工作目录。
fn vm_relative_paths_use_process_cwd() {
    let project = Project::new();
    fs::create_dir(project.root.join("data")).expect("创建相对路径目录");
    fs::write(project.root.join("data/relative.txt"), "relative-ok").expect("写夹具");
    let child = Command::new(std::env::current_exe().expect("测试程序"))
        .args([
            "--exact",
            "vm_relative_path_child",
            "--ignored",
            "--nocapture",
        ])
        .env("XIAO_20D_RELATIVE_PATH_CHILD", "1")
        .current_dir(&project.root)
        .output()
        .expect("运行相对路径子进程");
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
}

#[test]
#[ignore = "由 vm_relative_paths_use_process_cwd 在隔离 cwd 中调用"]
fn vm_relative_path_child() {
    if std::env::var_os("XIAO_20D_RELATIVE_PATH_CHILD").is_none() {
        return;
    }
    let source = "from stdlib.os import open_file, read_text, close\nhandle = open_file(\"data/relative.txt\")\nprint(read_text(handle))\nclose(handle)\n";
    let request = FrontendRequest::from_text(source);
    let outcome = FrontendVmDriver::new().run(&DriverRequest::new(request));
    assert!(outcome.is_success(), "{outcome:?}");
    let output: String = outcome
        .as_executed()
        .expect("进入执行")
        .events()
        .iter()
        .filter_map(|e| {
            if let xiao_vm::VmEvent::IntrinsicOutput { text } = e {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(output, "relative-ok\n");
}

#[test]
fn os_module_runtime_error_identities_are_stable_in_vm() {
    let project = Project::new();
    let missing = project
        .root
        .join("missing.txt")
        .to_string_lossy()
        .replace('\\', "/");
    let missing_source =
        format!("from stdlib.os import open_file\nhandle = open_file(\"{missing}\")\n");
    let missing_artifact = compile(&project, &missing_source);
    let missing_outcome = FrontendVmDriver::new().run_artifact(
        &missing_artifact,
        &DriverRequest::new(project.request(&missing_source)),
    );
    assert_eq!(missing_outcome.code(), Some("X06-RUNTIME-012"));

    let file = project.root.join("closed.txt");
    fs::write(&file, "closed\n").expect("写入关闭测试文件");
    let closed_path = file.to_string_lossy().replace('\\', "/");
    let closed_source = format!(
        "from stdlib.os import open_file, read_text, close\nhandle = open_file(\"{closed_path}\")\nclose(handle)\ntext = read_text(handle)\n"
    );
    let closed_artifact = compile(&project, &closed_source);
    let closed_outcome = FrontendVmDriver::new().run_artifact(
        &closed_artifact,
        &DriverRequest::new(project.request(&closed_source)),
    );
    assert_eq!(closed_outcome.code(), Some("X06-RUNTIME-025"));
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
            &NativeBuildRequest::new(
                frontend_request.clone(),
                target.clone(),
                toolchain.clone(),
                output_path,
            ),
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

    let denied_output = project.root.join(if cfg!(windows) {
        "20d-first-api-native-denied.exe"
    } else {
        "20d-first-api-native-denied"
    });
    let denied_options = CodegenOptions::for_target(target.clone())
        .with_capabilities(0)
        .expect("零能力掩码应被接受");
    let denied_native = FrontendNativeDriver::new()
        .build_artifact(
            &artifact,
            &NativeBuildRequest::new(
                project.request(&source),
                target.clone(),
                toolchain.clone(),
                denied_output,
            )
            .with_codegen_options(denied_options),
        )
        .unwrap_or_else(|error| match error {
            NativeDriverError::Frontend(error) => panic!("原生缺能力前端失败：{error}"),
            NativeDriverError::Backend(error) => panic!("原生缺能力后端失败：{error}"),
        });
    let denied_run = Command::new(&denied_native.native.executable)
        .output()
        .expect("启动原生缺能力夹具");
    assert_eq!(denied_run.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&denied_run.stderr).contains("X06-RUNTIME-026"));

    fs::create_dir(project.root.join("data")).expect("相对路径目录");
    fs::write(project.root.join("data/relative.txt"), "relative-ok").expect("相对路径夹具");
    fs::write(project.root.join("binary.dat"), [0xff_u8, 0xfe_u8]).expect("二进制夹具");
    for (label, source, expected_stdout, expected_code) in [
        (
            "relative",
            "from stdlib.os import open_file, read_text, close\nhandle = open_file(\"data/relative.txt\")\nprint(read_text(handle))\nclose(handle)\n",
            "relative-ok\n",
            None,
        ),
        (
            "directory",
            "from stdlib.os import open_directory, close\nhandle = open_directory(\"data\")\nclose(handle)\nclose(handle)\nprint(handle)\n",
            "resource<directory>\n",
            None,
        ),
        (
            "missing",
            "from stdlib.os import open_file\nhandle = open_file(\"data/missing.txt\")\n",
            "",
            Some("X06-RUNTIME-012"),
        ),
        (
            "utf8",
            "from stdlib.os import open_file, read_text\nhandle = open_file(\"binary.dat\")\ntext = read_text(handle)\n",
            "",
            Some("X06-RUNTIME-012"),
        ),
        (
            "closed",
            "from stdlib.os import open_file, read_text, close\nhandle = open_file(\"data/relative.txt\")\nalias = handle\nclose(alias)\nclose(handle)\ntext = read_text(handle)\n",
            "",
            Some("X06-RUNTIME-025"),
        ),
    ] {
        let artifact = compile(&project, source);
        let output = project.root.join(format!(
            "20d-{label}{}",
            if cfg!(windows) { ".exe" } else { "" }
        ));
        let native = FrontendNativeDriver::new()
            .build_artifact(
                &artifact,
                &NativeBuildRequest::new(
                    project.request(source),
                    target.clone(),
                    toolchain.clone(),
                    output,
                ),
            )
            .unwrap_or_else(|error| panic!("{label} 原生构建失败：{error}"));
        let run = Command::new(&native.native.executable)
            .current_dir(&project.root)
            .output()
            .expect("启动原生夹具");
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert_eq!(
            run.status.code(),
            Some(if expected_code.is_some() { 3 } else { 0 }),
            "{label}: {stderr}"
        );
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            expected_stdout,
            "{label}"
        );
        match expected_code {
            Some(code) => assert!(
                stderr.contains(&format!("code={code} ")),
                "{label}: {stderr}"
            ),
            None => assert!(stderr.is_empty(), "{label}: {stderr}"),
        }
        println!("20D-NATIVE {label}: stdout={expected_stdout:?} code={expected_code:?}");
    }
}
