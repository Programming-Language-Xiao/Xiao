//! A1数值恢复与身份的完整源码示例；不测试未实施的算术或转换矩阵。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use xiao_codegen_llvm::{CodegenOptions, TargetDescription, Toolchain};
use xiao_driver::{
    DriverRequest, FrontendCompiler, FrontendNativeDriver, FrontendRequest, FrontendVmDriver,
    NativeBuildRequest,
};
use xiao_vm::VmEvent;

/// 同一个完整fixture和独立黄金供VM与原生消费。
fn fixture(name: &str) -> (FrontendRequest, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let directory = root.join("tests/fixtures/docs/numeric");
    let path = directory.join(format!("{name}.xiao"));
    let source = fs::read_to_string(&path).unwrap();
    let page = fs::read_to_string(root.join("docs/UseDocs/language/basics/types/conversions.md"))
        .unwrap()
        .replace("\r\n", "\n");
    assert!(
        page.contains(&format!("```xiao\n{source}```")),
        "A1完整示例必须与UseDocs逐字一致：{name}"
    );
    let request = FrontendRequest::from_text_at(source, path);
    let golden = fs::read_to_string(directory.join(format!("{name}.stdout"))).unwrap();
    (request, golden)
}

#[test]
/// 三份完整示例默认进入现有workspace测试，不用Rust字符串另写产品算法。
fn a1_numeric_values_and_errors_have_vm_golden_outputs() {
    for name in ["a1_identity", "a1_admission_error", "a1_pending_finally"] {
        let (request, golden) = fixture(name);
        let artifact = FrontendCompiler::new().compile(&request).unwrap();
        let outcome = FrontendVmDriver::new().run_artifact(
            &artifact,
            &DriverRequest::new(request).with_timeout(Duration::from_secs(10)),
        );
        assert!(outcome.is_success(), "{name}: {outcome:?}");
        let stdout: String = outcome
            .as_executed()
            .unwrap()
            .events()
            .iter()
            .filter_map(|event| match event {
                VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(stdout, golden, "{name}");
        println!("B2-A1-IDENTITY case={name} backend=vm result=success");
    }
}

/// 原生子进程有明确超时，两个管道并发读取，避免管道背压掩盖挂起。
fn native_output(path: &Path) -> Output {
    let mut child = Command::new(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let read = |mut stream: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).unwrap();
            bytes
        })
    };
    let stdout = read(Box::new(stdout));
    let stderr = read(Box::new(stderr));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(10) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("A1原生示例超过10秒");
        }
        thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: stdout.join().unwrap(),
        stderr: stderr.join().unwrap(),
    }
}

#[test]
#[ignore = "需要本次新构建的Runtime及显式LLVM/平台链接器；既有原生workspace入口可执行"]
/// 同一数值源码与错误在O0/O3逐字一致；失败必须发生在构造处并可继续执行。
fn a1_numeric_values_and_errors_match_native_golden_outputs() {
    let required =
        |key| PathBuf::from(std::env::var_os(key).unwrap_or_else(|| panic!("缺少{key}")));
    let target = TargetDescription::host();
    assert_eq!(std::env::var("XIAO_TARGET_TRIPLE").unwrap(), target.triple);
    let toolchain = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .unwrap();
    let root = std::env::temp_dir().join(format!(
        "xiao-b2-a1-native-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    for name in ["a1_identity", "a1_admission_error", "a1_pending_finally"] {
        let (request, golden) = fixture(name);
        let artifact = FrontendCompiler::new().compile(&request).unwrap();
        for optimization in [0, 3] {
            let executable = root.join(format!(
                "{name}-o{optimization}{}",
                std::env::consts::EXE_SUFFIX
            ));
            let options = CodegenOptions::for_target(target.clone())
                .with_optimization_level(optimization)
                .unwrap();
            let build = NativeBuildRequest::new(
                request.clone(),
                target.clone(),
                toolchain.clone(),
                executable,
            )
            .with_codegen_options(options);
            let built = FrontendNativeDriver::new()
                .build_artifact(&artifact, &build)
                .unwrap();
            let output = native_output(&built.native.executable);
            assert_eq!(
                output.status.code(),
                Some(0),
                "{name} O{optimization}: {output:?}"
            );
            assert!(
                output.stderr.is_empty(),
                "{name} O{optimization}: {output:?}"
            );
            assert_eq!(output.stdout, golden.as_bytes(), "{name} O{optimization}");
            println!(
                "B2-A1-IDENTITY case={name} backend=native optimization={optimization} artifact={} result=success",
                built.native.executable.display()
            );
        }
    }
}
