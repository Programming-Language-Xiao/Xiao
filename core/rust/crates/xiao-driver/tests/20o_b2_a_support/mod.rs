//! A5完整语言语料的共用原生执行点；所有结果逐字核对。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use xiao_codegen_llvm::{CodegenOptions, TargetDescription, Toolchain};
use xiao_driver::{FrontendCompiler, FrontendNativeDriver, FrontendRequest, NativeBuildRequest};

/// 同一源码/黄金供VM及原生使用，原生输出路径绑定PID和独立时间戳。
pub fn native_fixtures(names: &[&str]) {
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
        "xiao-b2-a5-native-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    for name in names {
        let path = repository
            .join("tests/fixtures/docs/numeric")
            .join(format!("{name}.xiao"));
        let golden = fs::read(path.with_extension("stdout")).unwrap();
        let request = FrontendRequest::from_text_at(fs::read_to_string(&path).unwrap(), path);
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
            assert_eq!(output.stdout, golden, "{name} O{optimization}");
            println!(
                "B2-A5-LANGUAGE case={name} backend=native optimization={optimization} artifact={} result=success",
                built.native.executable.display()
            );
        }
    }
}

/// 有界子进程和并发排空管道，失败不被背压或永久等待掩盖。
fn native_output(path: &Path) -> Output {
    let mut child = Command::new(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let read = |mut stream: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).unwrap();
            bytes
        })
    };
    let stdout = read(Box::new(child.stdout.take().unwrap()));
    let stderr = read(Box::new(child.stderr.take().unwrap()));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(10) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("A5原生示例超过10秒");
        }
        thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: stdout.join().unwrap(),
        stderr: stderr.join().unwrap(),
    }
}
