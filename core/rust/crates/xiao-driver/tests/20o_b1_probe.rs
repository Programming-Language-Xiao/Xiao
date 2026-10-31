//! 20O-B0 的 B1 能力探测；只执行夹具，不注册或实现任何官方库接口。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_driver::{
    DriverRequest, FrontendArtifact, FrontendCompiler, FrontendNativeDriver, FrontendRequest,
    FrontendVmDriver, NativeBuildRequest,
};
use xiao_vm::VmEvent;

/// 黄金值独立于 Xiao 算法保存；含字节边界、Unicode、空值和错误恢复标签。
const CASES: [(&str, &str); 7] = [
    (
        "math",
        "6\n6\n0\n1024\n1\ncaught-negative-exponent\ncaught-overflow\n",
    ),
    ("hex", "00010f107f80ff\n255\n-1\ncaught-byte-range\n"),
    (
        "base64",
        "Zm9v\nZg==\nZm8=\n102\n111\n111\n-1\ntrue\nfalse\n",
    ),
    (
        "url",
        "%E9%9B%AA\n%F0%9F%98%80\n%20%2B%2F\n雪\n😀\n-1\n38634\n",
    ),
    (
        "csv",
        "[a]\n[b,c]\n[d\"e]\n[]\n[]\n[雪\n😀]\n[x]\ncaught-unclosed-quote\n",
    ),
    (
        "fmt",
        "..雪😀\nlong\n000\nff\n-9223372036854775808\ncaught-fill-width\n",
    ),
    (
        "statistics",
        "10\n-2\n9\n5\n0\ncaught-empty-min\ncaught-empty-max\ncaught-overflow\n",
    ),
];

/// 依据 crate 固定位置找到仓库，保留真实源码路径及官方导入环境。
fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("仓库目录布局")
}

/// 教程与测试消费同一个完整探测文件，不在 Rust 字符串中另写算法。
fn fixture_request(module: &str) -> FrontendRequest {
    let path = repository_root().join(format!("tests/fixtures/docs/{module}/probe.xiao"));
    FrontendRequest::from_text_at(fs::read_to_string(&path).expect("读取完整示例"), path)
}

/// VM 必须成功退出，并逐字满足独立黄金值；超时属于失败。
fn assert_vm(request: &FrontendRequest, artifact: &FrontendArtifact, expected: &str) {
    let outcome = FrontendVmDriver::new().run_artifact(
        artifact,
        &DriverRequest::new(request.clone()).with_timeout(Duration::from_secs(10)),
    );
    assert_eq!(outcome.exit_code().as_process_code(), 0, "{outcome:?}");
    let stdout: String = outcome
        .as_executed()
        .expect("必须实际执行")
        .events()
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(stdout.as_bytes(), expected.as_bytes(), "VM 黄金输出");
}

#[test]
/// 七项真实源码均通过完整前端和生产 VM，不能将编译成功当作执行成功。
fn b1_capabilities_have_vm_golden_outputs() {
    for (module, expected) in CASES {
        let request = fixture_request(module);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{module}: {error:?}"));
        assert_vm(&request, &artifact, expected);
        println!("20O-B0-PROBE module={module} backend=vm result=success");
    }
}

#[test]
/// 全文 CSV 的嵌套数组签名目前在共同前端被拒绝，不能用内部数组能力冒充公开 API。
fn b1_csv_nested_signature_records_frontend_boundary() {
    let path = repository_root().join("tests/fixtures/20o-b1/csv-nested-signature.xiao");
    let request = FrontendRequest::from_text_at(fs::read_to_string(&path).expect("读取边界"), path);
    let error = FrontendCompiler::new()
        .compile(&request)
        .expect_err("嵌套数组注解尚未开放");
    let codes: Vec<_> = error
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .map(|diagnostic| diagnostic.code())
        .collect();
    assert!(codes.contains(&"X04-PARSE-005"), "{error:?}");
    println!("20O-B0-BOUNDARY csv_nested_signature=X04-PARSE-005 backends=not_entered");
}

#[test]
/// 从驱动器读取真实错误码；另存直接访问 error.code 的既有运行时边界。
fn b1_error_identities_and_member_access_boundary_are_recorded() {
    for (fixture, code) in [
        ("numeric-overflow", "X06-RUNTIME-009"),
        ("explicit-error", "B0_PROBE"),
        ("error-code-access", "X06-RUNTIME-002"),
    ] {
        let path = repository_root().join(format!("tests/fixtures/20o-b1/{fixture}.xiao"));
        let request =
            FrontendRequest::from_text_at(fs::read_to_string(&path).expect("读取边界"), path);
        let outcome = FrontendVmDriver::new()
            .run(&DriverRequest::new(request).with_timeout(Duration::from_secs(10)));
        assert_eq!(
            outcome.exit_code().as_process_code(),
            3,
            "{fixture}: {outcome:?}"
        );
        assert_eq!(outcome.code(), Some(code), "{fixture}: {outcome:?}");
        println!("20O-B0-BOUNDARY fixture={fixture} code={code} backend=vm");
    }
}

#[test]
/// 教程的完整示例与执行源码逐字一致，避免修改页面后 CI 仍测试旧副本。
fn b1_tutorials_embed_the_executed_sources() {
    for (module, _) in CASES {
        let page =
            fs::read_to_string(repository_root().join(format!("docs/UseDocs/library/{module}.md")))
                .expect("每库必须有教程");
        let page = page.replace("\r\n", "\n");
        assert!(page.lines().any(|line| line == "status: planned"));
        let complete = page
            .split_once("<!-- b0-executable-example -->\n```xiao\n")
            .expect("完整示例标记")
            .1
            .split_once("```")
            .expect("完整示例结束")
            .0;
        let source = fs::read_to_string(
            repository_root().join(format!("tests/fixtures/docs/{module}/probe.xiao")),
        )
        .expect("完整示例源码");
        assert_eq!(complete, source, "{module} 教程与执行文件漂移");
    }
}

/// 临时原生产物不落在源码树中；结束后只回收当前进程拥有的直接子目录。
struct ProbeDirectory(PathBuf);

impl ProbeDirectory {
    /// 创建与其他测试隔离的目录。
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("xiao-20o-b0-{}", std::process::id()));
        fs::create_dir_all(&root).expect("创建原生探测目录");
        Self(root)
    }
}

impl Drop for ProbeDirectory {
    /// 在规范绝对路径通过父级与前缀检查后回收产物。
    fn drop(&mut self) {
        if let (Ok(root), Ok(temp)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize())
            && root.parent() == Some(temp.as_path())
            && root.file_name().is_some_and(|name| {
                name.to_string_lossy() == format!("xiao-20o-b0-{}", std::process::id())
            })
        {
            let _ = fs::remove_dir_all(root);
        }
    }
}

/// 显式配置缺失就失败，不能把未执行的原生路径登记为通过。
fn native_toolchain() -> (TargetDescription, Toolchain) {
    let target = TargetDescription::host();
    assert_eq!(
        std::env::var("XIAO_TARGET_TRIPLE").expect("需要 XIAO_TARGET_TRIPLE"),
        target.triple
    );
    let required = |name| {
        let path = PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("需要 {name}")));
        assert!(path.is_file(), "{name} 文件不存在：{path:?}");
        path
    };
    let toolchain = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .expect("探测 Runtime 链接依赖");
    (target, toolchain)
}

/// 同时读取输出管道，原生进程超过十秒即终止并回收，不静默挂起 CI。
fn native_output(executable: &Path) -> Output {
    let mut child = Command::new(executable)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("执行原生探测");
    let read = |mut stream: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).expect("读取原生输出");
            bytes
        })
    };
    let stdout = read(Box::new(child.stdout.take().expect("stdout")));
    let stderr = read(Box::new(child.stderr.take().expect("stderr")));
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
#[ignore = "需要显式 LLVM/MSVC 或平台链接器、Runtime staticlib；维护回归三平台精确执行并校验一项通过"]
/// 七项同源 IR 在 VM/native 分别匹配独立黄金值，原生退出零且 stderr 为空。
fn b1_capabilities_match_native_golden_outputs() {
    let (target, toolchain) = native_toolchain();
    let root = ProbeDirectory::new();
    for (module, expected) in CASES {
        let request = fixture_request(module);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{module}: {error:?}"));
        assert_vm(&request, &artifact, expected);
        let executable = root
            .0
            .join(format!("{module}{}", std::env::consts::EXE_SUFFIX));
        let build = NativeBuildRequest::new(request, target.clone(), toolchain.clone(), executable);
        let native = FrontendNativeDriver::new()
            .build_artifact(&artifact, &build)
            .unwrap_or_else(|error| panic!("{module} 原生构建：{error}"));
        let output = native_output(&native.native.executable);
        assert_eq!(output.status.code(), Some(0), "{module}: {output:?}");
        assert_eq!(output.stdout, expected.as_bytes(), "{module} 原生黄金输出");
        assert!(output.stderr.is_empty(), "{module}: {output:?}");
        println!("20O-B0-PROBE module={module} backend=vm/native result=success");
    }
}
