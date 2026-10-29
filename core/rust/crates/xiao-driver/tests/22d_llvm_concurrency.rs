//! 22D 原生并发 lowering 的同源 IR 与 fail-closed 回归。

use std::process::Command;
use xiao_codegen_llvm::{CodegenOptions, lower_program, native_concurrency_plan};
use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_driver::{
    DriverRequest, FrontendCompiler, FrontendNativeDriver, FrontendRequest, FrontendVmDriver,
    NativeBuildRequest,
};

#[test]
/// 比较同一 IR 降出的 Native/TAC 计划并检查 trampoline lowering。
fn native_lowering_consumes_concurrency_spans_and_emits_trampoline() {
    let source = "from std.concurrent import runtime_create, spawn, join\ndef work(int value) -> int\n    return value\nrt = runtime_create()\nthread = spawn(rt, work, 1)\nresult = join(thread)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("并发源码应先通过统一前端");
    let native_plan = native_concurrency_plan(&artifact.ir).expect("建立原生并发计划");
    let tac = xiao_bytecode::lower_program(&artifact.ir);
    assert_eq!(native_plan.calls().len(), tac.concurrency_plans.len());
    for (native, vm) in native_plan.calls().iter().zip(&tac.concurrency_plans) {
        assert_eq!(native.span, vm.span);
        assert_eq!(native.operation, vm.operation);
        assert_eq!(native.argument_spans, vm.argument_spans);
        assert_eq!(native.argument_types, vm.argument_types);
        assert_eq!(native.result_type, vm.result_type);
        assert_eq!(native.cross_thread_facts, vm.facts);
        assert_eq!(
            native.boundary_transfer.as_ref().map(|transfer| (
                transfer.copy_kind.as_str(),
                transfer.parameters.as_slice(),
                transfer.cleanup_order.as_slice(),
            )),
            vm.transfer.as_ref().map(|transfer| (
                transfer.copy_kind.as_str(),
                transfer.parameters.as_slice(),
                transfer.cleanup_order.as_slice(),
            ))
        );
        assert_eq!(
            native
                .cancellation_checks
                .iter()
                .map(|check| check.kind.as_str())
                .collect::<Vec<_>>(),
            vm.cancellation_checks
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
    }
    let module = lower_program(
        &artifact.ir,
        &CodegenOptions::default().with_internal_concurrency(),
    )
    .expect("同源并发 IR 应能生成 LLVM 文本");
    assert!(
        module
            .text
            .contains("@xiao_runtime_concurrency_call_with_entry")
    );
    assert!(module.text.contains("xiao.concurrent.trampoline."));
    assert!(
        module
            .text
            .contains("@xiao_runtime_concurrency_cancellation_check")
    );
    assert!(
        module
            .runtime_components
            .iter()
            .any(|component| component == "concurrency")
    );
}

#[test]
/// 确认未知并发操作在普通 lowering 前被拒绝。
fn unknown_native_concurrency_operation_is_rejected_before_lowering() {
    let source = "from std.concurrent import runtime_create\nrt = runtime_create()\n";
    let mut artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("源码应先通过统一前端");
    artifact.ir.concurrency_calls[0].operation = "future_api".to_owned();
    let error = lower_program(
        &artifact.ir,
        &CodegenOptions::default().with_internal_concurrency(),
    )
    .expect_err("未知并发入口必须 fail-closed");
    assert!(error.to_string().contains("原生并发入口 future_api"));
}

#[test]
/// 守住 22E 前公开构建不得开启并发产物。
fn public_native_build_keeps_concurrency_artifacts_closed_until_22e() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "from std.concurrent import current_runtime\nrt = current_runtime()\n",
        ))
        .expect("并发源码的前端语义有效");
    let error = lower_program(&artifact.ir, &CodegenOptions::default())
        .expect_err("公开原生构建不得提前发布并发产物");
    assert!(matches!(
        error,
        xiao_codegen_llvm::CodegenError::Unsupported { .. }
    ));
    assert!(error.to_string().contains("22E"));
}

#[test]
/// 检查 shared/lock lowering 保留取消检查与 Runtime ABI 路径。
fn native_shared_lock_plan_keeps_cancel_and_runtime_abi_paths() {
    let source = "from std.concurrent import shared_mut, lock, lock_read\nvalue = shared_mut(1)\nguard = lock(value)\nresult = lock_read(guard)\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("shared/lock 源码应通过统一前端");
    let module = lower_program(
        &artifact.ir,
        &CodegenOptions::default().with_internal_concurrency(),
    )
    .expect("shared/lock 同源 IR 应能生成 LLVM 文本");
    assert!(module.text.contains("xiao_runtime_concurrency_call"));
    assert!(
        module
            .text
            .contains("xiao_runtime_concurrency_cancellation_check")
    );
    assert!(module.text.contains("xiao_runtime_release_trace_begin"));
}

#[test]
#[ignore = "需要 XIAO_CLANG、XIAO_LLVM_AS、XIAO_LLC、XIAO_RUNTIME_LIBRARY 和 XIAO_TARGET_TRIPLE"]
/// 在目标原生工具链下比较线程、任务池和共享场景的 VM/native stdout。
fn native_concurrency_vm_native_stdout_matches_from_same_ir() {
    let target = TargetDescription::host();
    let configured_target = std::env::var("XIAO_TARGET_TRIPLE").expect("XIAO_TARGET_TRIPLE");
    assert_eq!(configured_target, target.triple);
    let toolchain = Toolchain::new(std::env::var_os("XIAO_CLANG").expect("XIAO_CLANG"))
        .with_llvm_as(std::env::var_os("XIAO_LLVM_AS").expect("XIAO_LLVM_AS"))
        .with_llc(std::env::var_os("XIAO_LLC").expect("XIAO_LLC"))
        .with_runtime_library(
            std::env::var_os("XIAO_RUNTIME_LIBRARY").expect("XIAO_RUNTIME_LIBRARY"),
        )
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .expect("Runtime 的目标系统库清单应能读取");
    let suffix = if cfg!(windows) { "exe" } else { "bin" };
    let root = std::env::temp_dir().join(format!("xiao-22d-native-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("创建原生并发临时目录");
    let cases = [
        (
            "spawn-join",
            "from std.concurrent import runtime_create, spawn, join\ndef work(int value) -> int\n    return value + 6\nrt = runtime_create()\nthread = spawn(rt, work, 1)\nresult = join(thread)\nprint(result)\n",
        ),
        (
            "pool-submit-wait",
            "from std.concurrent import current_runtime, pool_create, submit, wait, shutdown\ndef work(int value) -> int\n    return value + 5\nrt = current_runtime()\npool = pool_create(rt, 1)\ntask = submit(pool, work, 7)\nresult = wait(task)\nprint(result)\nshutdown(pool)\n",
        ),
        (
            "shared-lock",
            "from std.concurrent import shared_mut, lock, lock_read, lock_write\nvalue = shared_mut(1)\nguard = lock(value)\nlock_write(guard, 9)\nresult = lock_read(guard)\nprint(result)\n",
        ),
        (
            "shared-read-boundary",
            "from std.concurrent import runtime_create, shared_read, read, spawn, join\ndef work(shared<int> value) -> int\n    return read(value)\nrt = runtime_create()\nvalue = shared_read(4)\nthread = spawn(rt, work, value)\nresult = join(thread)\nprint(result)\n",
        ),
    ];
    for (label, source) in cases {
        let frontend_request = FrontendRequest::from_text(source);
        let artifact = FrontendCompiler::new()
            .compile(&frontend_request)
            .unwrap_or_else(|error| panic!("{label}: 差分源码应通过统一前端：{error:?}"));
        let vm = FrontendVmDriver::new()
            .run_artifact(&artifact, &DriverRequest::new(frontend_request.clone()));
        assert!(vm.is_success(), "{label}: VM 场景必须成功：{vm:?}");
        let vm_output = vm
            .as_executed()
            .expect("VM 返回执行结果")
            .events()
            .iter()
            .filter_map(|event| match event {
                xiao_vm::VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>()
            .replace("\r\n", "\n");
        let output = root.join(format!("{label}.{suffix}"));
        let request =
            NativeBuildRequest::new(frontend_request, target.clone(), toolchain.clone(), &output)
                .with_codegen_options(
                    CodegenOptions::for_target(target.clone()).with_internal_concurrency(),
                );
        let native = FrontendNativeDriver::new()
            .build_artifact(&artifact, &request)
            .unwrap_or_else(|error| panic!("{label}: 原生构建失败：{error}"));
        let execution = Command::new(&native.native.executable)
            .output()
            .unwrap_or_else(|error| panic!("{label}: 原生并发产物启动失败：{error}"));
        assert_eq!(execution.status.code(), Some(0), "{label}");
        let native_output = String::from_utf8(execution.stdout)
            .expect("原生 stdout 为 UTF-8")
            .replace("\r\n", "\n");
        assert_eq!(native_output, vm_output, "{label}");
    }
    let _ = std::fs::remove_dir_all(root);
}
