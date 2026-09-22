//! 10A 前端到 LLVM 内部驱动器规格。

use xiao_codegen_llvm::{CodegenError, TargetDescription, Toolchain};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendNativeDriver, FrontendRequest,
    NativeBuildRequest, NativeDriverError,
};
use xiao_ir::{IrStatementKind, IrType};

/// 读取环境依赖原生测试使用的固定目标三元组。
fn configured_native_target() -> TargetDescription {
    let triple = std::env::var("XIAO_TARGET_TRIPLE")
        .expect("显式运行 --ignored 时 XIAO_TARGET_TRIPLE 必须已设置；准备方式见 10D §4");
    TargetDescription::new(
        triple,
        64,
        xiao_codegen_llvm::Endian::Little,
        xiao_codegen_llvm::ObjectFormat::Coff,
    )
    .expect("XIAO_TARGET_TRIPLE 必须是有效的 64 位 COFF 目标")
}

#[test]
/// 后端请求必须先经过真实 FrontendCompiler，再在工具链边界返回结构化错误。
fn compiles_frontend_before_toolchain() {
    let request = NativeBuildRequest::new(
        FrontendRequest::from_text("value = 1 + 2\n"),
        TargetDescription::host(),
        Toolchain::new("xiao-clang-not-installed"),
        std::env::temp_dir().join("xiao-native-driver-missing.exe"),
    );
    let error = FrontendNativeDriver::new()
        .build(&request)
        .expect_err("工具链应拒绝");
    assert!(matches!(
        error,
        NativeDriverError::Backend(CodegenError::ToolchainUnavailable { .. })
    ));
}

#[test]
/// 前端错误在进入 LLVM 后端前原样保留。
fn preserves_frontend_rejection() {
    let request = NativeBuildRequest::new(
        FrontendRequest::from_text("if 1\n    value = 1\n"),
        TargetDescription::host(),
        Toolchain::new("unused"),
        std::env::temp_dir().join("xiao-native-driver-invalid.exe"),
    );
    assert!(matches!(
        FrontendNativeDriver::new().build(&request),
        Err(NativeDriverError::Frontend(_))
    ));
}

#[test]
/// 未注解函数的最终参数/返回签名必须从类型阶段进入共享 IR。
fn carries_inferred_function_signature_into_ir() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "def count(limit)\n    total = 0\n    while total != limit\n        total = total + 1\n    return total\nresult = count(3)\n",
        ))
        .expect("类型阶段应推断函数签名");
    let function = artifact
        .ir
        .body
        .iter()
        .find_map(|statement| match &statement.kind {
            IrStatementKind::Function {
                name,
                parameters,
                return_type,
                ..
            } if name.text == "count" => Some((parameters, return_type)),
            _ => None,
        })
        .expect("应找到 count 函数");
    assert_eq!(
        function.0[0].ty,
        IrType::Scalar {
            name: "int".to_owned()
        }
    );
    assert_eq!(
        function.1,
        &IrType::Scalar {
            name: "int".to_owned()
        }
    );
}

#[test]
#[ignore = "需要 XIAO_CLANG 与 XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
/// 提供工具链时，真实 Xiao 源码必须完整走前端再生成原生程序。
fn optional_real_frontend_to_native_round_trip() {
    let clang = std::env::var_os("XIAO_CLANG")
        .expect("显式运行 --ignored 时 XIAO_CLANG 必须已设置；准备方式见 10D §4");
    let root = std::env::temp_dir().join(format!("xiao-driver-n0-a-{}", std::process::id()));
    let output = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let target = configured_native_target();
    let request = NativeBuildRequest::new(
        FrontendRequest::from_text(
            "def count(limit)\n    total = 0\n    while total != limit\n        total = total + 1\n    return total\nresult = count(3)\n",
        ),
        target,
        Toolchain::new(clang),
        &output,
    );
    let result = FrontendNativeDriver::new()
        .build(&request)
        .expect("原生构建");
    assert!(result.native.executable.exists());
    let run = xiao_codegen_llvm::NativeRun::new(&result.native)
        .run()
        .expect("启动");
    assert_eq!(run.status, Some(0), "stderr: {}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[ignore = "需要 XIAO_CLANG 与 XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
/// VM 与原生后端必须消费同一份前端产物，并在静态标量入口上得到同一结果。
fn optional_frontend_artifact_differential_round_trip() {
    let clang = std::env::var_os("XIAO_CLANG")
        .expect("显式运行 --ignored 时 XIAO_CLANG 必须已设置；准备方式见 10D §4");
    let source = "[main]\nvalue = 1 + 2\n";
    let frontend_request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new()
        .compile(&frontend_request)
        .expect("真实 Xiao 源码应先通过前端");
    let vm_request = DriverRequest::new(frontend_request.clone());
    let vm = xiao_driver::FrontendVmDriver::new().run_artifact(&artifact, &vm_request);
    let DriverOutcome::Executed(execution) = vm else {
        panic!("同一份前端产物应进入 VM");
    };
    assert!(execution.outcome.result.is_success());

    let root = std::env::temp_dir().join(format!("xiao-driver-n0-a-diff-{}", std::process::id()));
    let output = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let target = configured_native_target();
    let request = NativeBuildRequest::new(
        frontend_request,
        target.clone(),
        Toolchain::new(clang),
        &output,
    )
    .with_codegen_options(xiao_codegen_llvm::CodegenOptions::for_target(target));
    let native = FrontendNativeDriver::new()
        .build_artifact(&artifact, &request)
        .expect("同一份前端产物应能构建原生程序");
    assert_eq!(native.frontend.ir, artifact.ir);
    let run = xiao_codegen_llvm::NativeRun::new(&native.native)
        .run()
        .expect("启动原生程序");
    assert_eq!(run.status, Some(0), "stderr: {}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
}
