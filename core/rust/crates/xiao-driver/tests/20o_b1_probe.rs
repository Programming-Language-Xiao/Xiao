//! 20O-B0/FIX-A 探测与 FIX-B error_code 前置回归；不实现七库接口。

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

/// FIX-A 单独保存独立黄金，不能用旧七项探测替代这些边界。
const FIX_A_CASES: [&str; 7] = [
    "math-extremes",
    "hex-all-bytes",
    "base64-tail-bits",
    "utf8-boundaries",
    "csv-budget",
    "fmt-int-min",
    "statistics-overflow",
];

/// FIX-B 黄金独立于七库探测；分配失败另由测试 feature 定向注入。
const FIX_B_CASES: [&str; 7] = [
    "default-and-custom",
    "nested-rethrow",
    "escape-lifetime",
    "pending-finally",
    "dynamic-types",
    "long-code",
    "abi-requirements",
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

/// 加载真实边界源码和独立固定输出；黄金项缺失或多出都会失败。
fn fix_a_cases() -> Vec<(&'static str, FrontendRequest, String)> {
    phase_cases("fix-a", &FIX_A_CASES)
}

/// 从同一固定目录加载 FIX-B 的源码与黄金，不混入 FIX-A 数值库输出。
fn fix_b_cases() -> Vec<(&'static str, FrontendRequest, String)> {
    phase_cases("fix-b", &FIX_B_CASES)
}

/// 读取指定阶段的完整黄金并严格核对场景集合数量。
fn phase_cases(phase: &str, ids: &[&'static str]) -> Vec<(&'static str, FrontendRequest, String)> {
    let root = repository_root().join(format!("tests/fixtures/20o-b1/{phase}"));
    let golden: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("golden.json")).expect("读取 FIX-A 黄金"),
    )
    .expect("FIX-A 黄金 JSON");
    let entries = golden["cases"].as_object().expect("FIX-A 黄金条目");
    assert_eq!(entries.len(), ids.len(), "{phase} 源码与黄金数量");
    ids.iter()
        .map(|&id| {
            let path = root.join(format!("{id}.xiao"));
            let request = FrontendRequest::from_text_at(
                fs::read_to_string(&path).expect("读取 FIX-A 真实源码"),
                path,
            );
            let expected = entries[id]["stdout"]
                .as_str()
                .expect("FIX-A 完整黄金 stdout")
                .to_owned();
            (id, request, expected)
        })
        .collect()
}

/// VM 必须成功退出，并逐字满足独立黄金值；超时属于失败。
fn assert_vm(
    request: &FrontendRequest,
    artifact: &FrontendArtifact,
    expected: &str,
    timeout: Duration,
) {
    let outcome = FrontendVmDriver::new().run_artifact(
        artifact,
        &DriverRequest::new(request.clone()).with_timeout(timeout),
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
/// 原七项与 FIX-A 七项真实源码均经完整前端/生产 VM，不把编译成功当执行成功。
fn b1_capabilities_have_vm_golden_outputs() {
    for (module, expected) in CASES {
        let request = fixture_request(module);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{module}: {error:?}"));
        assert_vm(&request, &artifact, expected, Duration::from_secs(10));
        println!("20O-B0-PROBE module={module} backend=vm result=success");
    }
    for (id, request, expected) in fix_a_cases() {
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("FIX-A {id}: {error:?}"));
        assert_vm(&request, &artifact, &expected, Duration::from_secs(60));
        println!("20O-B1-FIX-A-PROBE case={id} backend=vm result=success");
    }
    for (id, request, expected) in fix_b_cases() {
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("FIX-B {id}: {error:?}"));
        assert_vm(&request, &artifact, &expected, Duration::from_secs(10));
        println!("20O-B1-FIX-B-PROBE case={id} backend=vm result=success");
    }
}

#[test]
/// 参数形态和静态类型必须在执行前拒绝，Fatal 不变成可恢复读取。
fn error_code_rejects_static_arguments_and_preserves_fatal_boundaries() {
    for source in [
        "error_code()\n",
        "error_code(1)\n",
        "error_code(\"text\")\n",
        "error_code(none)\n",
        "error_code(Error(), Error())\n",
        "error_code(value = Error())\n",
        "error_code(*[])\n",
        "error_code(**{})\n",
    ] {
        let error = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .expect_err("静态非法读取必须拒绝");
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == "X04-TYPE-002"),
            "{source}: {error:?}"
        );
    }
    let fatal = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text("error_code(FatalError())\n"))
        .expect_err("Fatal 不可构造读取");
    assert!(
        fatal
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "X07-TYPE-003")
    );
    for (source, actual) in [("error_code(1)\n", "int"), ("error_code(none)\n", "none")] {
        let error = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .unwrap_err();
        let diagnostic = error
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.code() == "X04-TYPE-002")
            .unwrap();
        assert_eq!(
            diagnostic.params().get("expected"),
            Some(&xiao_diagnostics::DiagnosticParam::Text("Error".to_owned()))
        );
        assert_eq!(
            diagnostic.params().get("actual"),
            Some(&xiao_diagnostics::DiagnosticParam::Text(actual.to_owned()))
        );
    }
}

#[test]
/// 三目标文本锁定成功基本块中的 getter→复制→接管→释放，不借挂起槽模拟读取。
fn error_code_llvm_copies_the_view_before_releasing_the_input() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "original = Error()\ncode = error_code(original)\n",
        ))
        .unwrap();
    for target in [
        TargetDescription::windows_x86_64(),
        TargetDescription::linux_x86_64(),
        TargetDescription::macos_aarch64(),
    ] {
        // 最小 ABI 文件仅取 current_runtime，不直接调用值复制；泛型 Runtime 仍可达弱引用分支。
        let (_, request, _) = fix_b_cases()
            .into_iter()
            .find(|(id, _, _)| *id == "abi-requirements")
            .unwrap();
        let abi_artifact = FrontendCompiler::new().compile(&request).unwrap();
        let abi_module = xiao_codegen_llvm::NativeBuild::new()
            .lower(
                &abi_artifact.ir,
                &xiao_codegen_llvm::CodegenOptions::for_target(target.clone()),
            )
            .unwrap();
        assert!(
            abi_module
                .runtime_components
                .iter()
                .any(|component| component == "weak"),
            "ABI 夹具的 Runtime 传递依赖不能漏记弱引用组件"
        );
        let module = xiao_codegen_llvm::NativeBuild::new()
            .lower(
                &artifact.ir,
                &xiao_codegen_llvm::CodegenOptions::for_target(target),
            )
            .unwrap();
        let getter = module
            .text
            .find("call i32 @xiao_runtime_error_code(")
            .unwrap();
        let rest = &module.text[getter..];
        let success = rest
            .lines()
            .find(|line| line.starts_with("error.code.view.ok") && line.ends_with(':'))
            .unwrap();
        let copying = &rest[rest.find(success).unwrap()..];
        let copy = copying.find("@xiao_runtime_string_new(").unwrap();
        let owned = copying.find("@xiao_runtime_value_str_owned(").unwrap();
        let release = copying[owned..]
            .find("@xiao_runtime_value_release_")
            .unwrap()
            + owned;
        assert!(copy < owned && owned < release);
        for forbidden in [
            "@xiao_runtime_error_take(",
            "@xiao_runtime_error_snapshot(",
            "@xiao_runtime_error_clear(",
        ] {
            assert!(
                !copying[..release].contains(forbidden),
                "{forbidden} 被用于读取"
            );
        }
    }
}

#[test]
/// 无字符串常量的真实源码定向触发结果复制失败；源错误仍可再次读取。
fn error_code_vm_allocation_failure_preserves_source_and_recovers() {
    let path = repository_root().join("tests/fixtures/20o-b1/fix-b/allocation-failure.xiao");
    let request =
        FrontendRequest::from_text_at(fs::read_to_string(&path).expect("分配失败源码"), path);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("注入前编译");
    let fail = xiao_runtime::allocation_testing::StringAllocationFailure::after(0);
    assert_vm(
        &request,
        &artifact,
        "X06-RUNTIME-011\nX07-RUNTIME-ERROR\n",
        Duration::from_secs(10),
    );
    assert!(fail.was_triggered());
}

#[test]
/// 真实字节码/归档先验证所需 ABI，当前产物接受 1.18、拒绝 1.16/1.17 Runtime。
fn error_code_portable_artifacts_require_runtime_1_18() {
    let request = FrontendRequest::from_text("print(error_code(Error(code = \"portable\")))\n");
    let artifact = FrontendCompiler::new().compile(&request).unwrap();
    let program = xiao_bytecode::lower_program(&artifact.ir);
    let bytes =
        xiao_bytecode::encode_xiaoc(&program, xiao_bytecode::XiaocMetadata::new("fix-b-abi"))
            .unwrap();
    let file = xiao_bytecode::decode_xiaoc(&bytes).unwrap();
    assert_eq!(
        u64::from(file.header.runtime_abi_min),
        xiao_runtime_abi::encoded_abi_version(1, 18)
    );
    assert_eq!(file.header.runtime_abi_min, file.header.runtime_abi_max);
    xiao_bytecode::validate_runtime_requirements(
        &file.header,
        xiao_runtime_abi::encoded_abi_version(1, 18),
        xiao_runtime_abi::XIAOC_SUPPORTED_CONCURRENCY_FEATURES,
    )
    .unwrap();
    let index = xiao_artifacts::ArchiveIndex {
        schema_major: xiao_artifacts::INDEX_SCHEMA_MAJOR,
        schema_minor: xiao_artifacts::INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: Vec::new(),
        dependency_lock_digest: "0".repeat(64),
        runtime_abi_min: xiao_runtime_abi::ABI_ENCODED_VERSION,
        runtime_abi_max: xiao_runtime_abi::ABI_ENCODED_VERSION,
        platform: "portable".to_owned(),
        debug_activation: false,
        language_locale: "zh-CN".to_owned(),
        stdlib_interface_version: 1,
        language_version: "0.1.0".to_owned(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    };
    let mut builder = xiao_xar::XarBuilder::new(index);
    builder
        .add_xiaoc("main.xiaoc", "main", "portable", &bytes, Default::default())
        .unwrap();
    let archive = builder.build().unwrap();
    let current = xiao_xar::run_archive(&archive, Default::default()).unwrap();
    assert!(current.result.is_success());
    // 显式篡改版本声明的拒绝回归，不冒充旧编译器生成的真实产物。
    let previous_abi = xiao_runtime_abi::encoded_abi_version(1, 17);
    let mut previous_header = bytes.clone();
    previous_header[56..60].copy_from_slice(&(previous_abi as u32).to_le_bytes());
    previous_header[60..64].copy_from_slice(&(previous_abi as u32).to_le_bytes());
    assert!(matches!(
        xiao_bytecode::decode_xiaoc(&previous_header),
        Err(xiao_bytecode::XiaocError::InvalidField { ref field, .. }) if field == "runtime_abi"
    ));
    let decoded = xiao_xar::decode_xar(&archive).unwrap();
    let object = xiao_xar::XarObject::from_bytes(xiao_artifacts::ObjectKind::Xiaoc, bytes);
    let mut previous_index = decoded.index().clone();
    previous_index.runtime_abi_min = previous_abi;
    previous_index.runtime_abi_max = previous_abi;
    let previous_archive = xiao_xar::encode_xar(&previous_index, &[object]).unwrap();
    assert_eq!(
        xiao_xar::run_archive(&previous_archive, Default::default())
            .unwrap_err()
            .code(),
        "X17-XAR-007"
    );
    for minor in [16, 17] {
        assert!(
            xiao_bytecode::validate_runtime_requirements(
                &file.header,
                xiao_runtime_abi::encoded_abi_version(1, minor),
                xiao_runtime_abi::XIAOC_SUPPORTED_CONCURRENCY_FEATURES,
            )
            .is_err()
        );
        let old = xiao_xar::XarRunOptions {
            runtime_abi: xiao_runtime_abi::encoded_abi_version(1, minor),
            ..Default::default()
        };
        assert_eq!(
            xiao_xar::run_archive(&archive, old).unwrap_err().code(),
            "X17-XAR-007"
        );
    }
}

#[test]
/// 三种载体读取后重抛同一错误，保留 id、cause、suppressed、位置和上下文。
fn error_code_vm_read_then_raise_preserves_structured_identity() {
    use xiao_vm::research::{
        Carrier, HybridCarrier, RegisterCarrier, StackCarrier, VmOptions, run_with_values,
    };
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "original = Error()\nignored = error_code(original)\nraise original\n",
        ))
        .unwrap();
    let mut program = xiao_bytecode::lower_program(&artifact.ir);
    // 仅在语义核测试里用宿主提供的完整错误替换源码构造；不开放 Error/dynamic 参数语法。
    let parameter = xiao_bytecode::VReg::new(program.functions[0].categories.len() as u32);
    program.functions[0]
        .categories
        .insert(parameter, xiao_bytecode::RegisterClass::ObjHandle);
    program.functions[0].parameters = vec![parameter];
    let mut replaced = 0;
    for block in &mut program.functions[0].blocks {
        for instruction in &mut block.instructions {
            if let xiao_bytecode::TacOp::Call {
                callee, signature, ..
            } = &instruction.op
                && callee.get() == 0
                && program
                    .signatures
                    .get(*signature)
                    .and_then(|signature| signature.intrinsic_id())
                    == Some(10)
            {
                instruction.op = xiao_bytecode::TacOp::Copy(parameter);
                replaced += 1;
            }
        }
    }
    assert_eq!(replaced, 1, "只替换待验证错误的构造，不替换 getter/raise");
    program.categories = program.functions[0].categories.clone();
    let mut original = xiao_runtime::RuntimeError::invalid_value("original")
        .with_location(xiao_source::SourceSpan::new(7, 8).unwrap())
        .with_context(
            "owner",
            xiao_runtime::DiagnosticParam::Text("input".to_owned()),
        )
        .with_cause(xiao_runtime::RuntimeError::invalid_value("cause"));
    original.push_suppressed(xiao_runtime::RuntimeError::invalid_value("suppressed"));
    /// 在给定载体中读取后重抛，逐项比较宿主提供的完整错误身份。
    fn check<C: Carrier>(
        program: &xiao_bytecode::TacProgram,
        original: &xiao_runtime::RuntimeError,
    ) {
        let result = run_with_values::<C>(
            program,
            VmOptions::default(),
            &[xiao_runtime::RuntimeValue::error(original.clone())],
        );
        let xiao_vm::RunResult::Error(error) = result.result else {
            panic!("应重抛：{result:?}");
        };
        assert_eq!(error.error_id(), original.error_id());
        assert_eq!(error.code(), original.code());
        assert_eq!(error.cause(), original.cause());
        assert_eq!(error.suppressed(), original.suppressed());
        assert_eq!(error.location(), original.location());
        assert_eq!(error.context(), original.context());
    }
    check::<StackCarrier>(&program, &original);
    check::<RegisterCarrier>(&program, &original);
    check::<HybridCarrier>(&program, &original);
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
    let registry: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repository_root().join("docs/module-registry.json"))
            .expect("读取模块登记"),
    )
    .expect("模块登记 JSON");
    let modules = registry["modules"].as_array().expect("模块登记数组");
    for (module, _) in CASES {
        let page =
            fs::read_to_string(repository_root().join(format!("docs/UseDocs/library/{module}.md")))
                .expect("每库必须有教程");
        let page = page.replace("\r\n", "\n");
        let id = format!("stdlib.{module}");
        let status = modules
            .iter()
            .find(|entry| entry["id"].as_str() == Some(id.as_str()))
            .and_then(|entry| entry["status"].as_str())
            .expect("教程对应的模块状态");
        assert!(matches!(
            status,
            "planned" | "draft" | "verified" | "deprecated"
        ));
        assert_eq!(tutorial_status(&page), Some(status), "{module} 状态漂移");
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
    let page = fs::read_to_string(
        repository_root().join("docs/UseDocs/language/control-flow/error-handling.md"),
    )
    .unwrap()
    .replace("\r\n", "\n");
    let example = page
        .split_once("<!-- b1-error-code-executable-example -->\n```xiao\n")
        .expect("error_code 完整示例标记")
        .1
        .split_once("```")
        .unwrap()
        .0;
    let source = fs::read_to_string(
        repository_root().join("tests/fixtures/20o-b1/fix-b/default-and-custom.xiao"),
    )
    .unwrap()
    .replace("\r\n", "\n");
    assert_eq!(example, source, "error_code 教程必须逐字执行");
}

/// 只读取 front matter 的唯一状态，不让正文中的状态文字掩盖元数据不一致。
fn tutorial_status(page: &str) -> Option<&str> {
    let front_matter = page.strip_prefix("---\n")?.split_once("\n---\n")?.0;
    let mut statuses = front_matter
        .lines()
        .filter_map(|line| line.strip_prefix("status: "));
    let status = statuses.next()?.trim();
    if statuses.next().is_some() {
        return None;
    }
    Some(status)
}

#[test]
/// 状态随登记从 planned 升为 verified 时不必改测试；正文和重复字段不能伪造状态。
fn b1_tutorial_status_allows_verified_and_rejects_metadata_spoofing() {
    assert_eq!(
        tutorial_status("---\nstatus: planned\n---\n正文\n"),
        Some("planned")
    );
    assert_eq!(
        tutorial_status("---\nstatus: verified\n---\n正文\n"),
        Some("verified")
    );
    assert_eq!(
        tutorial_status("---\nstatus: planned\n---\nstatus: verified\n"),
        Some("planned")
    );
    assert_eq!(
        tutorial_status("---\nstatus: planned\nstatus: verified\n---\n"),
        None
    );
    assert_eq!(tutorial_status("正文\nstatus: verified\n"), None);
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

/// 同时读取输出管道，按调用方显式时限终止并回收，避免静默挂起 CI。
fn native_output(executable: &Path, timeout: Duration) -> Output {
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
    let deadline = Instant::now() + timeout;
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

/// 在测试可执行文件中改接一条 getter/复制调用；保留生成器原控制流与真实 Runtime。
fn instrument_error_code_failure(text: &str) -> String {
    let getter = text
        .find("call i32 @xiao_runtime_error_code(")
        .expect("实际 getter 调用");
    let view_ok = text[getter..]
        .lines()
        .find(|line| line.starts_with("error.code.view.ok") && line.ends_with(':'))
        .expect("getter 成功基本块");
    let view_start = getter + text[getter..].find(view_ok).unwrap();
    let copy = view_start
        + text[view_start..]
            .find("@xiao_runtime_string_new(")
            .expect("实际结果复制");
    let mut result = String::with_capacity(text.len() + 512);
    result.push_str(&text[..copy]);
    result.push_str(&text[copy..].replacen(
        "@xiao_runtime_string_new(",
        "@xiao_test_string_new(",
        1,
    ));
    result = result.replacen(
        "call i32 @xiao_runtime_error_code(",
        "call i32 @xiao_test_error_code(",
        1,
    );
    for (original, wrapper) in [
        ("xiao_runtime_value_copy", "xiao_test_value_copy"),
        (
            "xiao_runtime_value_release_strong",
            "xiao_test_release_strong",
        ),
        ("xiao_runtime_value_release_any", "xiao_test_release_any"),
    ] {
        result = result
            .lines()
            .map(|line| {
                if line.trim_start().starts_with("declare ") {
                    line.to_owned()
                } else {
                    line.replace(&format!("@{original}("), &format!("@{wrapper}("))
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    for (original, wrapper) in [
        ("xiao_runtime_error_code", "xiao_test_error_code"),
        ("xiao_runtime_string_new", "xiao_test_string_new"),
        ("xiao_runtime_value_copy", "xiao_test_value_copy"),
        (
            "xiao_runtime_value_release_strong",
            "xiao_test_release_strong",
        ),
        ("xiao_runtime_value_release_any", "xiao_test_release_any"),
    ] {
        let declaration = text
            .lines()
            .find(|line| line.contains("declare ") && line.contains(&format!("@{original}(")))
            .expect("已有 Runtime 声明");
        result.push('\n');
        result.push_str(&declaration.replace(original, wrapper));
    }
    result.push('\n');
    result
}

/// 链接期故障 shim 不进入生产 Runtime，实跑 getter 失败/复制失败及临时引用归还。
fn assert_native_error_code_failure_cleanup(
    target: &TargetDescription,
    toolchain: &Toolchain,
    root: &Path,
) {
    for (case, definition, expected) in [
        (
            "success-cleanup",
            "-DFIX_B_SUCCESS=1",
            "X07-RUNTIME-ERROR\nX07-RUNTIME-ERROR\n",
        ),
        (
            "allocation-failure",
            "-DFIX_B_COPY_FAILURE=1",
            "X06-RUNTIME-011\nX07-RUNTIME-ERROR\n",
        ),
        (
            "getter-failure",
            "-DFIX_B_GETTER_FAILURE=1",
            "X06-RUNTIME-004\nX07-RUNTIME-ERROR\n",
        ),
    ] {
        let path = repository_root().join(format!("tests/fixtures/20o-b1/fix-b/{case}.xiao"));
        let request =
            FrontendRequest::from_text_at(fs::read_to_string(&path).expect("故障源码"), path);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .expect("故障源码前端");
        let object = root.join(format!("{case}-shim.o"));
        let mut compiler = Command::new(&toolchain.clang);
        compiler
            .args(["-target", target.triple.as_str(), "-std=c11", "-c"])
            .arg(repository_root().join("tests/fixtures/20o-b1/fix-b/failure-shim.c"))
            .arg("-o")
            .arg(&object);
        compiler.arg(definition);
        let compiled = compiler.output().expect("编译仅测试 C shim");
        assert!(compiled.status.success(), "{case}: {compiled:?}");
        for level in [0, 3] {
            let options = xiao_codegen_llvm::CodegenOptions::for_target(target.clone())
                .with_optimization_level(level)
                .unwrap();
            let module = xiao_codegen_llvm::NativeBuild::new()
                .lower(&artifact.ir, &options)
                .expect("实际生成器 lowering");
            let text = instrument_error_code_failure(&module.text);
            let executable = root.join(format!(
                "injected-{case}-o{level}{}",
                std::env::consts::EXE_SUFFIX
            ));
            toolchain
                .validate_llvm_ir(&text, target)
                .expect("实际 llvm-as 验证故障控制流");
            let llvm_path = root.join(format!("injected-{case}-o{level}.ll"));
            fs::write(&llvm_path, &text).expect("写测试专用 LLVM");
            // 将 shim 对象置于静态库之前，避免 ELF 链接器的一次扫描漏掉它的 Runtime 引用。
            let linked = Command::new(&toolchain.clang)
                .args(["-target", target.triple.as_str(), &format!("-O{level}")])
                .arg(&llvm_path)
                .arg(&object)
                .arg(
                    toolchain
                        .runtime_library
                        .as_ref()
                        .expect("真实 Runtime staticlib"),
                )
                .args(&toolchain.native_static_libraries)
                .arg("-o")
                .arg(&executable)
                .output()
                .expect("链接测试 shim 与真实 Runtime");
            assert!(linked.status.success(), "{case} O{level} 链接：{linked:?}");
            let output = native_output(&executable, Duration::from_secs(10));
            assert_eq!(output.status.code(), Some(0), "{case} O{level}: {output:?}");
            assert_eq!(output.stdout, expected.as_bytes(), "{case} O{level}");
            assert!(output.stderr.is_empty(), "{case} O{level}: {output:?}");
            println!(
                "20O-B1-FIX-B-CLEANUP case={case} optimization={level} cleanup=once result=success"
            );
        }
    }
}

#[test]
#[ignore = "需要显式 LLVM/MSVC 或平台链接器、Runtime staticlib；维护回归三平台精确执行并校验一项通过"]
/// 原七项与 FIX-A 七项同源 IR 分别匹配 VM/native 黄金，原生退出零且 stderr 为空。
fn b1_capabilities_match_native_golden_outputs() {
    let (target, toolchain) = native_toolchain();
    let root = ProbeDirectory::new();
    for (module, expected) in CASES {
        let request = fixture_request(module);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{module}: {error:?}"));
        assert_vm(&request, &artifact, expected, Duration::from_secs(10));
        let executable = root
            .0
            .join(format!("{module}{}", std::env::consts::EXE_SUFFIX));
        let build = NativeBuildRequest::new(request, target.clone(), toolchain.clone(), executable);
        let native = FrontendNativeDriver::new()
            .build_artifact(&artifact, &build)
            .unwrap_or_else(|error| panic!("{module} 原生构建：{error}"));
        let output = native_output(&native.native.executable, Duration::from_secs(10));
        assert_eq!(output.status.code(), Some(0), "{module}: {output:?}");
        assert_eq!(output.stdout, expected.as_bytes(), "{module} 原生黄金输出");
        assert!(output.stderr.is_empty(), "{module}: {output:?}");
        println!("20O-B0-PROBE module={module} backend=vm/native result=success");
    }
    for (id, request, expected) in fix_a_cases() {
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("FIX-A {id}: {error:?}"));
        assert_vm(&request, &artifact, &expected, Duration::from_secs(60));
        let executable = root
            .0
            .join(format!("fix-a-{id}{}", std::env::consts::EXE_SUFFIX));
        let build = NativeBuildRequest::new(request, target.clone(), toolchain.clone(), executable);
        let native = FrontendNativeDriver::new()
            .build_artifact(&artifact, &build)
            .unwrap_or_else(|error| panic!("FIX-A {id} 原生构建：{error}"));
        let output = native_output(&native.native.executable, Duration::from_secs(60));
        assert_eq!(output.status.code(), Some(0), "FIX-A {id}: {output:?}");
        assert_eq!(
            output.stdout,
            expected.as_bytes(),
            "FIX-A {id} 原生黄金输出"
        );
        assert!(output.stderr.is_empty(), "FIX-A {id}: {output:?}");
        println!("20O-B1-FIX-A-PROBE case={id} backend=vm/native result=success");
    }
    for (id, request, expected) in fix_b_cases() {
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("FIX-B {id}: {error:?}"));
        assert_vm(&request, &artifact, &expected, Duration::from_secs(10));
        for level in [0, 3] {
            let options = xiao_codegen_llvm::CodegenOptions::for_target(target.clone())
                .with_optimization_level(level)
                .expect("O0/O3");
            let executable = root.0.join(format!(
                "fix-b-{id}-o{level}{}",
                std::env::consts::EXE_SUFFIX
            ));
            let build = NativeBuildRequest::new(
                request.clone(),
                target.clone(),
                toolchain.clone(),
                executable,
            )
            .with_codegen_options(options);
            let native = FrontendNativeDriver::new()
                .build_artifact(&artifact, &build)
                .unwrap_or_else(|error| panic!("FIX-B {id} O{level}: {error}"));
            if id == "abi-requirements" {
                let metadata = native
                    .native
                    .native_metadata
                    .as_ref()
                    .expect("真实原生产物元数据");
                assert_eq!(
                    metadata.runtime_abi_min,
                    xiao_runtime_abi::encoded_abi_version(1, 18)
                );
                let mut previous = metadata.clone();
                previous.runtime_abi_min = xiao_runtime_abi::encoded_abi_version(1, 17);
                previous.runtime_abi_max = previous.runtime_abi_min;
                assert_eq!(
                    previous
                        .validate(&target, &Default::default(), metadata.required_features)
                        .unwrap_err()
                        .code,
                    "X17-XAR-007"
                );
                for minor in [16, 17] {
                    let support = xiao_codegen_llvm::NativeRuntimeSupport {
                        runtime_abi: xiao_runtime_abi::encoded_abi_version(1, minor),
                        ..Default::default()
                    };
                    assert_eq!(
                        xiao_codegen_llvm::validate_native_metadata_file(
                            &native.native.executable,
                            &target,
                            &support,
                            metadata.required_features,
                            true,
                        )
                        .unwrap_err()
                        .code,
                        "X17-XAR-007"
                    );
                }
            }
            let output = native_output(&native.native.executable, Duration::from_secs(10));
            assert_eq!(
                output.status.code(),
                Some(0),
                "FIX-B {id} O{level}: {output:?}"
            );
            assert_eq!(output.stdout, expected.as_bytes(), "FIX-B {id} O{level}");
            assert!(output.stderr.is_empty(), "FIX-B {id} O{level}: {output:?}");
            println!(
                "20O-B1-FIX-B-PROBE case={id} backend=vm/native optimization={level} result=success"
            );
        }
    }
    assert_native_error_code_failure_cleanup(&target, &toolchain, &root.0);
}
