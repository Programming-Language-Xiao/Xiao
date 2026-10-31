//! 20M 纯 Xiao 文本/路径候选探测；冻结前不修改标准库函数体。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_driver::{
    DriverRequest, FrontendCompiler, FrontendNativeDriver, FrontendRequest, FrontendVmDriver,
    NativeBuildRequest,
};
use xiao_vm::VmEvent;

/// 九项独立源码及黄金输出；即使接口被裁掉，也保留其组合能力探测。
const PROBES: [(&str, &str); 9] = [
    (
        "trim",
        "[]\n[]\n[小😀]\n[小😀]\n[小😀]\n[小😀]\n[小😀]\n[小😀]\n[左]\n[右]\n[内 部\t空白]\n[ 　雪　 ]\n",
    ),
    ("starts-with", "true\ntrue\ntrue\nfalse\nfalse\nfalse\n"),
    ("ends-with", "true\ntrue\ntrue\nfalse\nfalse\nfalse\n"),
    (
        "replace-all",
        "[]\n[小😀]\n[bba]\n[aaaaaa]\n[]\n[雪雪]\n[abc]\n[abc]\n[xba]\n",
    ),
    ("repeat", "[]\n[]\n[小😀]\n[小😀小😀小😀]\n[]\n256\n"),
    (
        "upper-ascii",
        "[]\nABCDEFGHIJKLMNOPQRSTUVWXYZ\nAZ-9小雪😀éßı\n`A{Z~\n",
    ),
    (
        "basename",
        "[]\n[name]\n[]\n[name]\n[]\n[b]\n[b]\n[]\n[]\n[.env]\n[.env]\n[a.tar.gz]\n[file.]\n[.]\n[..]\n[b]\n[C:\\tmp\\file.txt]\n[小😀.x]\n",
    ),
    (
        "dirname",
        "[]\n[]\n[]\n[]\n[a]\n[a]\n[a/]\n[/]\n[a/]\n[]\n[dir.x]\n[]\n[]\n[]\n[]\n[a/..]\n[]\n[目录]\n",
    ),
    (
        "extension",
        "[]\n[]\n[]\n[]\n[]\n[]\n[]\n[]\n[]\n[.env]\n[.env]\n[.gz]\n[.]\n[.]\n[.]\n[]\n[.txt]\n[.x]\n",
    ),
];

/// 定位与正式工具链使用同一份标准库的仓库根目录。
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("xiao-driver 应位于 core/rust/crates 下")
        .to_path_buf()
}

/// 为候选读取真实文件，保留源码路径和导入环境。
fn probe_request(id: &str) -> FrontendRequest {
    let path = repository_root().join(format!("tests/fixtures/20m/probe-{id}.xiao"));
    FrontendRequest::from_text_at(fs::read_to_string(&path).expect("读取候选夹具"), path)
}

/// 从 VM 生产入口收集标准输出，并和独立黄金值对账。
fn assert_vm_output(
    request: &FrontendRequest,
    artifact: &xiao_driver::FrontendArtifact,
    expected: &str,
) {
    let outcome = FrontendVmDriver::new().run_artifact(
        artifact,
        &DriverRequest::new(request.clone()).with_timeout(Duration::from_secs(10)),
    );
    assert!(outcome.is_success(), "VM 探测失败：{outcome:?}");
    let output: String = outcome
        .as_executed()
        .into_iter()
        .flat_map(|execution| execution.events())
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(output, expected, "VM 黄金输出");
}

#[test]
/// 每项候选均须使用现有语言能力通过 VM，不允许因裁剪而撤掉探测。
fn text_path_candidates_have_vm_golden_outputs() {
    for (id, expected) in PROBES {
        let request = probe_request(id);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{id} 前端探测失败：{error:?}"));
        assert_vm_output(&request, &artifact, expected);
        println!("20M-PROBE candidate={id} path=vm result=success");
    }
}

/// 只接受显式配置的 LLVM、Runtime 和目标，避免无工具时虚报通过。
fn configured_native_toolchain() -> (TargetDescription, Toolchain) {
    let target = TargetDescription::host();
    assert_eq!(
        std::env::var("XIAO_TARGET_TRIPLE").expect("需要 XIAO_TARGET_TRIPLE"),
        target.triple
    );
    let runtime =
        PathBuf::from(std::env::var_os("XIAO_RUNTIME_LIBRARY").expect("需要 XIAO_RUNTIME_LIBRARY"));
    assert!(runtime.is_file(), "Runtime staticlib 不存在");
    let toolchain = Toolchain::new(std::env::var_os("XIAO_CLANG").expect("需要 XIAO_CLANG"))
        .with_llvm_as(std::env::var_os("XIAO_LLVM_AS").expect("需要 XIAO_LLVM_AS"))
        .with_runtime_library(runtime)
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .expect("读取 Runtime 原生链接库");
    (target, toolchain)
}

/// 并行读取两个输出管道，并给原生夹具施加与 VM 相同的十秒限制。
fn native_output(executable: &Path) -> Output {
    let mut child = Command::new(executable)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("启动原生夹具");
    let read_stream = |mut stream: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).expect("读取原生输出");
            bytes
        })
    };
    let stdout = read_stream(Box::new(child.stdout.take().expect("stdout 管道")));
    let stderr = read_stream(Box::new(child.stderr.take().expect("stderr 管道")));
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut timed_out = false;
    while child.try_wait().expect("轮询原生进程").is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            timed_out = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = Output {
        status: child.wait().expect("回收原生进程"),
        stdout: stdout.join().expect("stdout 读取线程"),
        stderr: stderr.join().expect("stderr 读取线程"),
    };
    assert!(!timed_out, "原生探测超时：{output:?}");
    output
}

#[test]
#[ignore = "需要受控 LLVM、Runtime staticlib 和目标三元组；维护回归三平台执行"]
/// 九项候选复用同一份前端 IR，VM/原生均须逐字匹配黄金值且 stderr 为空。
fn text_path_candidates_match_native_golden_outputs() {
    let (target, toolchain) = configured_native_toolchain();
    let root = std::env::temp_dir().join(format!("xiao-20m-probe-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建原生探测目录");
    for (id, expected) in PROBES {
        let request = probe_request(id);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{id} 前端探测失败：{error:?}"));
        assert_vm_output(&request, &artifact, expected);
        let executable = root.join(format!("{id}{}", std::env::consts::EXE_SUFFIX));
        let native = FrontendNativeDriver::new()
            .build_artifact(
                &artifact,
                &NativeBuildRequest::new(request, target.clone(), toolchain.clone(), executable),
            )
            .unwrap_or_else(|error| panic!("{id} 原生探测构建失败：{error}"));
        let output = native_output(&native.native.executable);
        assert_eq!(output.status.code(), Some(0), "{id}: {output:?}");
        assert_eq!(output.stdout, expected.as_bytes(), "{id}: 原生黄金输出");
        assert!(output.stderr.is_empty(), "{id}: {output:?}");
        println!("20M-PROBE candidate={id} path=vm/native result=success");
    }
    let resolved = root.canonicalize().expect("解析原生探测目录");
    let temp = std::env::temp_dir().canonicalize().expect("解析临时根目录");
    assert!(resolved.starts_with(temp) && resolved != repository_root());
    fs::remove_dir_all(resolved).expect("清理原生探测目录");
}
