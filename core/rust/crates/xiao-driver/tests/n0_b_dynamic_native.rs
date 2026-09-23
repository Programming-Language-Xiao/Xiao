//! N0-B 动态值经真实前端、LLVM 和 Runtime 静态库的闭环测试。

use std::path::PathBuf;

use xiao_codegen_llvm::{CodegenOptions, NativeBuild, TargetDescription, Toolchain};
use xiao_driver::{FrontendCompiler, FrontendNativeDriver, FrontendRequest, NativeBuildRequest};

#[test]
#[ignore = "需要 XIAO_CLANG / XIAO_LLVM_AS / XIAO_RUNTIME_LIBRARY / XIAO_TARGET_TRIPLE；准备方式见 10D §4"]
/// 同一份 Xiao 字符串源码必须能经前端生成动态 LLVM 并链接 Runtime。
fn optional_dynamic_string_native_round_trip() {
    let clang = std::env::var_os("XIAO_CLANG")
        .expect("显式运行 --ignored 时 XIAO_CLANG 必须已设置；准备方式见 10D §4");
    let llvm_as = std::env::var_os("XIAO_LLVM_AS")
        .expect("显式运行 --ignored 时 XIAO_LLVM_AS 必须已设置；准备方式见 10D §4");
    let runtime_text = std::env::var_os("XIAO_RUNTIME_LIBRARY")
        .expect("显式运行 --ignored 时 XIAO_RUNTIME_LIBRARY 必须已设置；准备方式见 10D §4");
    let runtime = PathBuf::from(runtime_text);
    assert!(
        runtime.is_file(),
        "XIAO_RUNTIME_LIBRARY 必须指向已构建的 Runtime staticlib: {}",
        runtime.display()
    );
    let configured_triple = std::env::var("XIAO_TARGET_TRIPLE")
        .expect("显式运行 --ignored 时 XIAO_TARGET_TRIPLE 必须已设置；准备方式见 10D §4");
    let target = TargetDescription::host();
    assert_eq!(
        configured_triple, target.triple,
        "XIAO_TARGET_TRIPLE 必须与当前 Rust 编译目标一致"
    );
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| std::ffi::OsString::from("rustc"));
    let toolchain = Toolchain::new(clang)
        .with_llvm_as(llvm_as)
        .with_runtime_library(runtime)
        .probe_native_static_libraries(rustc, &target)
        .expect("rustc 应报告 Runtime staticlib 的原生库清单");
    let root = std::env::temp_dir().join(format!("xiao-n0-b-dynamic-{}", std::process::id()));
    let output = root.join(if cfg!(windows) {
        "dynamic.exe"
    } else {
        "dynamic"
    });
    let request = NativeBuildRequest::new(
        FrontendRequest::from_text("value = \"heap\"\n"),
        target,
        toolchain,
        &output,
    );
    let result = FrontendNativeDriver::new()
        .build(&request)
        .expect("动态源码应完成原生构建");
    let run = xiao_codegen_llvm::NativeRun::new(&result.native)
        .run()
        .expect("动态原生程序应可启动");
    assert_eq!(run.status, Some(0), "stderr: {}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
/// 真实前端生成的动态布尔分支必须能进入 LLVM 验证器并保留 ABI 释放路径。
fn frontend_dynamic_branch_lowers_without_reparsing() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "value = \"root\"\nflag = true\nif flag\n    marker = 1\nelse\n    marker = 2\n",
        ))
        .expect("动态分支源码应通过前端");
    let options = CodegenOptions::default();
    let module = NativeBuild::new()
        .lower(&artifact.ir, &options)
        .expect("同一份 IR 应降低");
    assert!(module.uses_runtime);
    assert!(module.text.contains("dynamic.if.then"));
    if let Some(llvm_as) = std::env::var_os("XIAO_LLVM_AS") {
        NativeBuild::new()
            .validate_llvm(
                &module.text,
                &TargetDescription::host(),
                &Toolchain::new("unused").with_llvm_as(llvm_as),
            )
            .expect("llvm-as 应接受前端动态分支模块");
    }
}

#[test]
/// 分支局部拥有值在 N0-B 尚无块级展开时必须被结构化拒绝，不能静默泄漏。
fn frontend_dynamic_branch_with_owned_local_is_rejected() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "flag = true\nif flag\n    value = \"then\"\nelse\n    value = \"else\"\n",
        ))
        .expect("动态分支源码应通过前端");
    let error = NativeBuild::new()
        .lower(&artifact.ir, &CodegenOptions::default())
        .expect_err("嵌套释放计划不能被静默忽略");
    assert!(error.to_string().contains("嵌套作用域释放计划"));
}

#[test]
/// 前端生命周期值名带稳定前缀时，动态降低仍应能找到根释放槽。
fn frontend_dynamic_string_lowers_with_release_plan() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text("value = \"heap\"\n"))
        .expect("动态字符串源码应通过前端");
    let module = NativeBuild::new()
        .lower(&artifact.ir, &CodegenOptions::default())
        .expect("动态字符串 IR 应降低");
    assert!(module.uses_runtime);
    assert!(
        module
            .text
            .contains("call void @xiao_runtime_value_release(ptr %slot0)")
    );
}

#[test]
/// 同一份真实源码的字符串、数组、表字段默认值和成员读取必须沿同一份 IR 进入原生降低。
fn frontend_dynamic_table_initializer_and_member_lower() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "[[Box]]\n    value = 7\ntext = \"xiao\"\nvalues = [1, 2]\nitem = new Box()\ncode = item.value\n",
        ))
        .expect("真实表源码应通过前端");
    let module = NativeBuild::new()
        .lower(
            &artifact.ir,
            &CodegenOptions::default()
                .with_entry_observation(xiao_codegen_llvm::EntryObservation::ExitCode),
        )
        .expect("表初始化和成员读取应降低");
    assert!(module.uses_runtime);
    assert!(module.text.contains("@xiao_runtime_value_str"));
    assert!(module.text.contains("@xiao_runtime_array_new"));
    assert!(module.text.contains("@xiao_runtime_table_set"));
    assert!(module.text.contains("@xiao_runtime_table_get"));
    assert!(module.text.contains("c\"ascii:value\\00\""));
    if let Some(llvm_as) = std::env::var_os("XIAO_LLVM_AS") {
        NativeBuild::new()
            .validate_llvm(
                &module.text,
                &TargetDescription::host(),
                &Toolchain::new("unused").with_llvm_as(llvm_as),
            )
            .expect("llvm-as 应接受真实表初始化模块");
    }
}

#[test]
/// 动态根绑定被嵌套显式声明遮蔽时，降低器必须拒绝槽位冲突而不是覆盖根值。
fn frontend_dynamic_shadowed_slot_is_rejected() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "value = \"root\"\nif true\n    int value = 1\n",
        ))
        .expect("遮蔽源码应通过前端");
    let error = NativeBuild::new()
        .lower(&artifact.ir, &CodegenOptions::default())
        .expect_err("动态槽名称冲突不能静默覆盖根值");
    assert!(error.to_string().contains("动态槽") || error.to_string().contains("遮蔽"));
}

#[test]
/// 纯静态祖先绑定被分支内同名声明遮蔽时，也必须拒绝共享槽位覆盖。
fn frontend_static_shadowed_slot_is_rejected() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "root = \"runtime\"\nint value = 1\nif true\n    int value = 2\ncode = value\n",
        ))
        .expect("静态遮蔽源码应通过前端");
    let error = NativeBuild::new()
        .lower(&artifact.ir, &CodegenOptions::default())
        .expect_err("纯静态遮蔽不能覆盖祖先槽位");
    assert!(error.to_string().contains("动态槽") || error.to_string().contains("遮蔽"));
}
