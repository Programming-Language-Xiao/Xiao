//! 20M 候选探测及冻结接口的同源 VM/LLVM、加载移除与归档重放回归。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use xiao_codegen_llvm::{CodegenError, CodegenOptions, NativeBuild, TargetDescription, Toolchain};
use xiao_driver::{
    ArchiveBuildRequest, CancellationToken, DriverRequest, FrontendCompiler, FrontendNativeDriver,
    FrontendRequest, FrontendVmDriver, NativeBuildRequest, ProtocolRequest, ProtocolResponse,
    RunOptions, build_archive, dispatch,
};
use xiao_ir::IrStatementKind;
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

/// 冻结的七个入口；组合剔除项不成为产品 API。
const PRODUCTS: [(&str, &str, &str); 7] = [
    ("trim", "text", "trim"),
    ("replace-all", "text", "replace_all"),
    ("repeat", "text", "repeat"),
    ("upper-ascii", "text", "upper_ascii"),
    ("basename", "path", "basename"),
    ("dirname", "path", "dirname"),
    ("extension", "path", "extension"),
];

/// 函数别名、跨模块组合及补充路径边界的独立黄金值。
const INTEGRATION_OUTPUT: &str = "dir/小雪-A.TXT\n小雪-A.TXT\ntrue\ntrue\n[|a/b|]\n[b|/a|]\n[name|dir.x|]\n[.config.json||.json]\n[a..b||.b]\n[|dir.x|]\n1\n";

/// 将本测试创建的临时目录与工程源码隔离，失败时也回收产物。
struct TestDirectory(PathBuf);

impl TestDirectory {
    /// 按测试用途创建固定前缀、当前进程独占的目录。
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!("xiao-20m-{label}-{}", std::process::id()));
        fs::create_dir_all(&root).expect("创建隔离目录");
        Self(root)
    }
}

impl Drop for TestDirectory {
    /// 解析绝对路径后，只回收临时根目录下本测试拥有的直接子目录。
    fn drop(&mut self) {
        if let (Ok(root), Ok(temp)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize())
            && root.parent() == Some(temp.as_path())
            && root
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("xiao-20m-"))
        {
            let _ = fs::remove_dir_all(root);
        }
    }
}

/// 定位与正式工具链使用同一份标准库的仓库根目录。
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("xiao-driver 应位于 core/rust/crates 下")
        .to_path_buf()
}

/// 读取真实文件，保留源码路径和导入环境。
fn fixture_request(id: &str) -> FrontendRequest {
    let path = repository_root().join(format!("tests/fixtures/20m/{id}.xiao"));
    FrontendRequest::from_text_at(fs::read_to_string(&path).expect("读取夹具"), path)
}

/// 产品调用与探测共享独立黄金值，另加函数别名、组合调用与目录点边界。
fn product_cases() -> Vec<(&'static str, FrontendRequest, &'static str)> {
    PRODUCTS
        .iter()
        .map(|&(id, _, _)| {
            let expected = PROBES
                .iter()
                .find(|&&(probe, _)| probe == id)
                .expect("候选黄金值")
                .1;
            (id, fixture_request(id), expected)
        })
        .chain(std::iter::once((
            "integration",
            fixture_request("integration"),
            INTEGRATION_OUTPUT,
        )))
        .collect()
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
        let request = fixture_request(&format!("probe-{id}"));
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{id} 前端探测失败：{error:?}"));
        assert_vm_output(&request, &artifact, expected);
        println!("20M-PROBE candidate={id} path=vm result=success");
    }
}

#[test]
/// 七项公开接口及组合调用逐字满足已冻结的边界。
fn text_path_interfaces_have_vm_golden_outputs() {
    for (id, request, expected) in product_cases() {
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{id} 前端失败：{error:?}"));
        assert_vm_output(&request, &artifact, expected);
    }
}

#[test]
/// 每项调用验证 VM 的模块级加载失败；LLVM 侧精确核对缺失函数名并要求携带诊断区间。
fn text_path_interfaces_require_loader_and_native_function_binding() {
    for (id, module, function) in PRODUCTS {
        let request = fixture_request(id);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .expect("移除夹具前端通过");
        let program = xiao_bytecode::lower_program(&artifact.ir);
        let without_loader =
            xiao_vm::run_request(&xiao_vm::RunRequest::new(&artifact.ir, &program));
        let xiao_vm::RunResult::Error(error) = without_loader.result else {
            panic!("{id} 移除加载器后必须拒绝：{without_loader:?}");
        };
        assert_eq!(error.code(), "X06-RUNTIME-012", "{id}");
        assert!(
            error.message().contains("未注册模块编译器"),
            "{id}: {error:?}"
        );
        assert!(
            error
                .message()
                .contains(&format!("官方标准库模块 {module}")),
            "{id} 应在对应模块加载时失败：{error:?}"
        );
        let mut broken = artifact.ir.clone();
        let keep = |statement: &xiao_ir::IrStatement| !matches!(&statement.kind, IrStatementKind::Function { name, .. } if name.text == function);
        broken.body.retain(keep);
        for module in &mut broken.modules {
            module.body.retain(keep);
        }
        let result = NativeBuild::new().lower(&broken, &CodegenOptions::default());
        let Err(CodegenError::Unsupported { feature, span }) = result else {
            panic!("{id} 移除函数体后须在 LLVM lowering 拒绝：{result:?}");
        };
        assert_eq!(
            feature,
            format!("动态函数调用 {function}（未找到静态函数绑定）"),
            "{id} 的拒绝必须指向被删除的函数"
        );
        assert!(span.is_some(), "{id} 的缺失绑定诊断必须带源码区间");
        println!(
            "20M-REMOVAL function={function} vm_module=stdlib.{module} native_feature={feature} span={span:?}"
        );
    }
}

#[test]
/// 真实归档仅记录实际官方导入、保持接口版本 1，删除应用源码后仍通过宿主加载器重放。
fn text_path_archives_replay_with_exact_host_module_requirements() {
    let root = TestDirectory::new("archives");
    for (id, request, expected) in product_cases() {
        let source = root.0.join(format!("{}.xiao", id.replace('-', "_")));
        fs::write(&source, request.source.text()).expect("写入应用源码");
        let mut build = ArchiveBuildRequest::new(&source);
        build.cache_root = Some(root.0.join("cache"));
        let archive = build_archive(&build, &CancellationToken::new()).expect("真实归档构建");
        let modules: Vec<_> = if id == "integration" {
            vec!["stdlib.path", "stdlib.text"]
        } else {
            let module = PRODUCTS.iter().find(|&&(name, _, _)| name == id).unwrap().1;
            vec![if module == "text" {
                "stdlib.text"
            } else {
                "stdlib.path"
            }]
        };
        assert_eq!(archive.index.stdlib_modules, modules, "{id}");
        assert_eq!(archive.index.stdlib_interface_version, 1);
        assert_eq!(
            archive.index.required_capabilities,
            xiao_intrinsics::CAPABILITY_STDOUT
        );
        assert!(
            archive
                .index
                .entries
                .iter()
                .all(|entry| !entry.module.starts_with("stdlib."))
        );
        let decoded = xiao_xar::decode_xar(&fs::read(&archive.output).unwrap()).unwrap();
        let entry = xiao_bytecode::decode_xiaoc(&decoded.read_entry().unwrap()).unwrap();
        assert!(
            entry
                .program
                .functions
                .iter()
                .all(|function| !PRODUCTS.iter().any(|&(_, _, name)| function.name == name)),
            "官方函数不能复制进入口对象"
        );
        fs::remove_file(source).expect("删除应用源码，验证独立重放");
        let response = dispatch(ProtocolRequest::RunArchive {
            request_id: format!("20m-{id}"),
            protocol_version: xiao_driver::PROTOCOL_VERSION,
            core_version: xiao_driver::CORE_VERSION,
            locale: None,
            path: archive.output.display().to_string(),
            options: RunOptions {
                timeout_ms: Some(10_000),
                ..Default::default()
            },
            diagnostics: None,
            debug: false,
        });
        let ProtocolResponse::Result {
            exit_code, events, ..
        } = response
        else {
            panic!("{id} 归档重放失败：{response:?}");
        };
        assert_eq!(exit_code, 0, "{id}: {events:?}");
        let output: String = events
            .iter()
            .filter(|event| event.kind == "intrinsic_output")
            .filter_map(|event| event.data.get("text").and_then(serde_json::Value::as_str))
            .collect();
        assert_eq!(output, expected, "{id} 归档黄金输出");
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

/// 实际链接并运行，同步核对退出码、stdout 和 stderr。
fn assert_native_case(
    id: &str,
    artifact: &xiao_driver::FrontendArtifact,
    request: NativeBuildRequest,
    expected: &str,
) {
    let native = FrontendNativeDriver::new()
        .build_artifact(artifact, &request)
        .unwrap_or_else(|error| panic!("{id} 原生构建失败：{error}"));
    let output = native_output(&native.native.executable);
    assert_eq!(output.status.code(), Some(0), "{id}: {output:?}");
    assert_eq!(output.stdout, expected.as_bytes(), "{id}: 原生黄金输出");
    assert!(output.stderr.is_empty(), "{id}: {output:?}");
}

#[test]
#[ignore = "需要受控 LLVM、Runtime staticlib 和目标三元组；维护回归三平台执行"]
/// 九项候选复用同一份前端 IR，VM/原生均须逐字匹配黄金值且 stderr 为空。
fn text_path_candidates_match_native_golden_outputs() {
    let (target, toolchain) = configured_native_toolchain();
    let root = TestDirectory::new("probes");
    for (id, expected) in PROBES {
        let request = fixture_request(&format!("probe-{id}"));
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{id} 前端探测失败：{error:?}"));
        assert_vm_output(&request, &artifact, expected);
        let executable = root.0.join(format!("{id}{}", std::env::consts::EXE_SUFFIX));
        assert_native_case(
            id,
            &artifact,
            NativeBuildRequest::new(request, target.clone(), toolchain.clone(), executable),
            expected,
        );
        println!("20M-PROBE candidate={id} path=vm/native result=success");
    }
}

#[test]
#[ignore = "需要受控 LLVM、Runtime staticlib 和目标三元组；维护回归三平台执行"]
/// 产品公开入口复用同源 IR，逐字通过七项边界及函数别名/跨模块组合场景。
fn text_path_interfaces_match_native_golden_outputs() {
    let (target, toolchain) = configured_native_toolchain();
    let root = TestDirectory::new("interfaces");
    for (id, request, expected) in product_cases() {
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{id} 前端失败：{error:?}"));
        assert_vm_output(&request, &artifact, expected);
        let executable = root.0.join(format!("{id}{}", std::env::consts::EXE_SUFFIX));
        assert_native_case(
            id,
            &artifact,
            NativeBuildRequest::new(request, target.clone(), toolchain.clone(), executable),
            expected,
        );
        println!("20M-INTERFACE case={id} path=vm/native result=success");
    }
}
