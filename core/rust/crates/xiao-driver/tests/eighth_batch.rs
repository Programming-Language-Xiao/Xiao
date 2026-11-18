//! 20C 第八批：动态标量转换、`lint`/`lfloat` 形参与整数族边界。

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use xiao_codegen_llvm::{CodegenOptions, NativeRun, NativeRunResult, TargetDescription, Toolchain};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendContext, FrontendNativeDriver,
    FrontendRequest, NativeBuildRequest,
};
use xiao_intrinsics::{CAPABILITY_STDOUT, KNOWN_CAPABILITY_MASK};
use xiao_vm::VmEvent;

static PROJECT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestProject {
    root: PathBuf,
}

impl TestProject {
    fn new(source: &str) -> Self {
        let sequence = PROJECT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("xiao-20c-eighth-{}-{sequence}", std::process::id()));
        fs::create_dir_all(&root).expect("创建第八批隔离项目");
        fs::write(root.join("main.xiao"), source).expect("写入第八批入口");
        Self { root }
    }

    fn write(&self, relative: &str, source: &str) {
        fs::write(self.root.join(relative), source).expect("写入第八批辅助模块");
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
        .unwrap_or_else(|error| panic!("第八批源码不应在前端拒绝：{error:?}"));
    xiao_driver::FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(request))
}

#[test]
/// 普通动态值的 `str`/`bool`/整数转换进入 Runtime；错误边界仍保留专门守卫。
fn vm_dynamic_scalar_conversions_align_with_runtime() {
    let source = "import helper\nprint(helper.value as str)\nprint(helper.flag as bool)\nprint(int(helper.number))\n";
    let project = TestProject::new(source);
    project.write("helper.xiao", "value = \"A\"\nflag = true\nnumber = 7\n");
    let outcome = vm_run(&project, source);
    assert!(outcome.is_success(), "动态标量转换失败：{outcome:?}");
    assert_eq!(output(&outcome), "A\ntrue\n7\n");

    let heterogeneous_source = "for value in [1, \"A\"]\n    print(value as str)\n    print(value as int)\nfor value in [true, \"true\"]\n    print(value as bool)\n";
    let heterogeneous_project = TestProject::new(heterogeneous_source);
    let heterogeneous = vm_run(&heterogeneous_project, heterogeneous_source);
    assert!(
        heterogeneous.is_success(),
        "异构动态标量转换失败：{heterogeneous:?}"
    );
    assert_eq!(output(&heterogeneous), "1\n1\nA\n65\ntrue\ntrue\n");

    let boundary_source = "import helper\nstr rendered = helper.value\nprint(rendered)\n";
    let boundary_project = TestProject::new(boundary_source);
    boundary_project.write("helper.xiao", "value = \"A\"\n");
    let boundary = vm_run(&boundary_project, boundary_source);
    assert_eq!(boundary.code(), Some("X06-RUNTIME-002"));
}

#[test]
/// `lint`/`lfloat` 形参不再把调用方寄存器误判为整数类别；整数族转换共用 Runtime 规则。
fn vm_accepts_high_precision_parameters_and_integer_conversions() {
    let source = "lint x = 42\nlfloat y = 1.5\ndef lint_to_int(lint value) -> int\n    return value as int\ndef lint_to_sint(lint value) -> sint\n    return value as sint\ndef lint_identity(lint value) -> lint\n    return value\ndef ignore_lfloat(lfloat value) -> str\n    return \"ok\"\nint small = 7\nprint(lint_to_int(x))\nprint(lint_to_sint(x))\nprint(lint_identity(x))\nprint(ignore_lfloat(y))\nprint(small as lint)\n";
    let project = TestProject::new(source);
    let outcome = vm_run(&project, source);
    assert!(
        outcome.is_success(),
        "高精度形参与整数转换失败：{outcome:?}"
    );
    assert_eq!(output(&outcome), "42\n42\n42\nok\n7\n");

    let large_source = "lint big = 9223372036854775808\nprint(big)\nprint(170141183460469231731687303715884105727)\n";
    let large_project = TestProject::new(large_source);
    let large = vm_run(&large_project, large_source);
    assert!(large.is_success(), "大 lint 字面量不应静默失败：{large:?}");
    assert_eq!(
        output(&large),
        "9223372036854775808\n170141183460469231731687303715884105727\n"
    );

    let invalid_source = "lint huge = 2147483648\nprint(huge as sint)\n";
    let invalid_project = TestProject::new(invalid_source);
    let invalid = vm_run(&invalid_project, invalid_source);
    assert_eq!(invalid.code(), Some("X06-RUNTIME-009"));
}

#[test]
/// `lint`/`lfloat` 一元负号必须保留文本精度，并按零值与已有符号规则归一化。
fn vm_supports_high_precision_unary_negation() {
    let source = "lint zero = 0
lint positive = 42
lint negative = -positive
lint negative_zero = -zero
lfloat precise = 1.234567890123456789
lfloat negative_precise = -precise
lfloat zero_float = 0.0
lfloat negative_zero_float = -zero_float
lfloat zero_float_precise = 0.00
print(negative)
print(negative_zero)
print(negative_precise)
print(zero_float)
print(negative_zero_float)
print(zero_float_precise)
";
    let project = TestProject::new(source);
    let outcome = vm_run(&project, source);
    assert!(outcome.is_success(), "高精度一元负号失败：{outcome:?}");
    assert_eq!(
        output(&outcome),
        "-42
0
-1.234567890123456789
0
0
0
"
    );
}

fn configured_native_toolchain() -> (TargetDescription, Toolchain) {
    let target = TargetDescription::host();
    let configured_triple =
        std::env::var("XIAO_TARGET_TRIPLE").expect("原生第八批测试需要 XIAO_TARGET_TRIPLE");
    assert_eq!(configured_triple, target.triple);
    let runtime = PathBuf::from(
        std::env::var_os("XIAO_RUNTIME_LIBRARY").expect("原生第八批测试需要 Runtime staticlib"),
    );
    assert!(
        runtime.is_file(),
        "Runtime staticlib 不存在：{}",
        runtime.display()
    );
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain =
        Toolchain::new(std::env::var_os("XIAO_CLANG").expect("原生第八批测试需要 XIAO_CLANG"))
            .with_llvm_as(
                std::env::var_os("XIAO_LLVM_AS").expect("原生第八批测试需要 XIAO_LLVM_AS"),
            )
            .with_llc(std::env::var_os("XIAO_LLC").expect("原生第八批测试需要 XIAO_LLC"))
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
        .unwrap_or_else(|error| panic!("原生第八批源码不应前端拒绝：{error:?}"));
    let output =
        std::env::temp_dir().join(format!("xiao-20c-eighth-{name}-{}.exe", std::process::id()));
    let native = FrontendNativeDriver::new()
        .build_artifact(
            &artifact,
            &NativeBuildRequest::new(request, target.clone(), toolchain.clone(), output.clone()),
        )
        .unwrap_or_else(|error| panic!("第八批原生构建失败：{error}"));
    let run = NativeRun::new(&native.native)
        .run()
        .unwrap_or_else(|error| panic!("第八批原生产物启动失败：{error}"));
    let _ = fs::remove_file(output);
    run
}

#[test]
#[ignore = "需要受控 LLVM 工具链、Runtime staticlib 和目标三元组"]
/// 原生路径必须与 VM 共享动态转换、形参修复和整数窄化身份。
fn native_matches_eighth_batch_boundaries() {
    let (target, toolchain) = configured_native_toolchain();
    let success = "lint x = 42\nlfloat y = 1.5\ndef lint_to_int(lint value) -> int\n    return value as int\ndef ignore_lfloat(lfloat value) -> str\n    return \"ok\"\nprint(lint_to_int(x))\nprint(ignore_lfloat(y))\n";
    let project = TestProject::new(success);
    let run = native_run(&project, success, &target, &toolchain, "success");
    assert_eq!(run.status, Some(0), "第八批原生成功夹具状态错误：{run:?}");
    assert_eq!(run.stdout, "42\nok\n");
    assert!(run.stderr.is_empty(), "成功夹具不应输出 stderr：{run:?}");

    let blocked = "print(\"blocked\")\n";
    let blocked_project = TestProject::new(blocked);
    let blocked_request = blocked_project.request(blocked);
    let blocked_artifact = FrontendCompiler::new()
        .compile(&blocked_request)
        .expect("能力负例源码应通过前端");
    let blocked_output = std::env::temp_dir().join(format!(
        "xiao-20c-eighth-capability-{}.exe",
        std::process::id()
    ));
    let blocked_options = CodegenOptions::for_target(target.clone())
        .with_capabilities(KNOWN_CAPABILITY_MASK & !CAPABILITY_STDOUT)
        .expect("能力掩码应有效");
    let blocked_native = FrontendNativeDriver::new()
        .build_artifact(
            &blocked_artifact,
            &NativeBuildRequest::new(
                blocked_request,
                target.clone(),
                toolchain.clone(),
                blocked_output.clone(),
            )
            .with_codegen_options(blocked_options),
        )
        .expect("能力负例应能完成原生构建");
    let blocked_run = NativeRun::new(&blocked_native.native)
        .run()
        .expect("能力负例原生产物应能启动");
    let _ = fs::remove_file(blocked_output);
    assert_eq!(
        blocked_run.status,
        Some(3),
        "缺少 stdout 能力应运行时失败：{blocked_run:?}"
    );
    assert!(
        blocked_run.stderr.contains("X06-RUNTIME-026"),
        "原生能力负例缺少稳定码：{blocked_run:?}"
    );

    let unary = "lint zero = 0\nlint positive = 42\nlint negative = -positive\nlfloat precise = 1.234567890123456789\nlfloat negative_precise = -precise\nlfloat zero_float = 0.0\nlfloat negative_zero_float = -zero_float\nlfloat zero_float_precise = 0.00\nlfloat zero_exponent = 0e5\nlfloat negative_zero_exponent = -zero_exponent\nlfloat zero_exponent_decimal = 0.0e0\nlfloat negative_zero_exponent_decimal = -zero_exponent_decimal\nlfloat zero_exponent_upper = 0E-3\nlfloat negative_zero_exponent_upper = -zero_exponent_upper\nlfloat nonzero_exponent = 1e10\nlfloat nonzero_decimal = 1.50\nprint(negative)\nprint(-zero)\nprint(negative_precise)\nprint(zero_float)\nprint(negative_zero_float)\nprint(zero_float_precise)\nprint(zero_exponent)\nprint(negative_zero_exponent)\nprint(zero_exponent_decimal)\nprint(negative_zero_exponent_decimal)\nprint(zero_exponent_upper)\nprint(negative_zero_exponent_upper)\nprint(nonzero_exponent)\nprint(nonzero_decimal)\n";
    let project = TestProject::new(unary);
    let run = native_run(&project, unary, &target, &toolchain, "unary");
    assert_eq!(run.status, Some(0), "第八批原生高精度取负状态错误：{run:?}");
    assert_eq!(
        run.stdout,
        "-42\n0\n-1.234567890123456789\n0\n0\n0\n0\n0\n0\n0\n0\n0\n1e10\n1.50\n"
    );
    assert!(run.stderr.is_empty(), "高精度取负不应输出 stderr：{run:?}");

    let dynamic = "for value in [1, \"A\"]\n    print(value as str)\n    print(value as int)\nfor value in [true, \"true\"]\n    print(value as bool)\n";
    let project = TestProject::new(dynamic);
    let run = native_run(&project, dynamic, &target, &toolchain, "dynamic");
    assert_eq!(run.status, Some(0), "第八批动态原生夹具状态错误：{run:?}");
    assert_eq!(run.stdout, "1\n1\nA\n65\ntrue\ntrue\n");
    assert!(
        run.stderr.is_empty(),
        "动态成功夹具不应输出 stderr：{run:?}"
    );

    let invalid = "lint huge = 2147483648\nprint(huge as sint)\n";
    let project = TestProject::new(invalid);
    let run = native_run(&project, invalid, &target, &toolchain, "range");
    assert_eq!(run.status, Some(3), "第八批窄化越界进程状态错误：{run:?}");
    assert!(
        run.stderr.contains("X06-RUNTIME-009"),
        "缺少范围错误码：{run:?}"
    );
}
