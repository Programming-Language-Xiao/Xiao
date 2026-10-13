//! 20C §3.1 官方 Xiao 库能力探测。
//!
//! 夹具只测现有语言、Runtime 和 LLVM 后端能力，不实现标准库，不添加 intrinsic。

use std::fs;
use std::path::{Path, PathBuf};

use xiao_codegen_llvm::{CodegenError, NativeRun, TargetDescription, Toolchain};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendNativeDriver, FrontendRequest,
    NativeBuildRequest, NativeDriverError,
};
use xiao_vm::VmEvent;

/// 探测夹具的穷尽结果形状；不提供“任意失败”兜底，避免前端提前拒绝遮住后端缺口。
#[derive(Clone, Copy, Debug)]
enum ProbeExpectation {
    /// VM 与原生都必须成功并输出指定文本。
    Success(&'static str),
    /// 两条路径都必须在前端以指定码拒绝。
    Frontend(&'static str),
    /// 前端必须通过；VM 进入稳定字节码拒绝，原生进入指定后端拒绝。
    Backend(&'static str),
}

const PROBES: [(&str, &str, Option<&str>); 2] = [
    ("strings-path", "strings-path.xiao", Some("lib/core\n")),
    ("arithmetic-error", "arithmetic-error.xiao", None),
];

const REPRESENTATIVE_PROBES: [(&str, &str, ProbeExpectation); 12] = [
    (
        "strings-path-representative",
        "strings-path-representative.xiao",
        ProbeExpectation::Success("/\n"),
    ),
    (
        "strings-index-boundary",
        "strings-index-boundary.xiao",
        ProbeExpectation::Frontend("X03-TYPE-003"),
    ),
    (
        "arithmetic-representative",
        "arithmetic-representative.xiao",
        ProbeExpectation::Frontend("X02-TYPE-006"),
    ),
    (
        "second-string-comparison",
        "second-string-comparison.xiao",
        ProbeExpectation::Frontend("X02-TYPE-005"),
    ),
    (
        "second-string-comparison-parameters",
        "second-string-comparison-parameters.xiao",
        ProbeExpectation::Success("ok\n"),
    ),
    (
        "second-count-length",
        "second-count-length.xiao",
        ProbeExpectation::Success("2\n"),
    ),
    (
        "second-string-accumulate",
        "second-string-accumulate.xiao",
        ProbeExpectation::Success("ok\n"),
    ),
    (
        "second-substring-lookahead",
        "second-substring-lookahead.xiao",
        ProbeExpectation::Frontend("X01-PARSE-007"),
    ),
    (
        "second-split-array",
        "second-split-array.xiao",
        ProbeExpectation::Frontend("X02-TYPE-005"),
    ),
    (
        "second-case-conversion",
        "second-case-conversion.xiao",
        ProbeExpectation::Frontend("X02-TYPE-006"),
    ),
    (
        "second-case-conversion-logic",
        "second-case-conversion-logic.xiao",
        ProbeExpectation::Backend("动态二元运算 and"),
    ),
    (
        "second-set-membership",
        "second-set-membership.xiao",
        ProbeExpectation::Success("ok\n"),
    ),
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("xiao-driver crate 应位于仓库 core/rust/crates 下")
        .to_path_buf()
}

fn fixture_path(file: &str) -> PathBuf {
    repository_root().join("tests/fixtures/20c").join(file)
}

fn source_request(path: &Path) -> FrontendRequest {
    FrontendRequest::from_text_at(
        fs::read_to_string(path).expect("读取 20C 能力探测夹具"),
        path,
    )
}

fn vm_output(outcome: &DriverOutcome) -> String {
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

fn first_frontend_code(error: &xiao_driver::FrontendError) -> &str {
    error
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.is_error())
        .or_else(|| error.diagnostics().first())
        .map_or("<none>", |diagnostic| diagnostic.code())
}

#[test]
/// 首批与第二批候选夹具必须给出可复核的 VM 结果或前端错误码。
fn vm_capability_probe_records_each_module() {
    let compiler = FrontendCompiler::new();
    for (id, file, expected_output) in PROBES {
        let path = fixture_path(file);
        let request = source_request(&path);
        let artifact = compiler
            .compile(&request)
            .unwrap_or_else(|error| panic!("{id} 前端拒绝：{:?}", error.diagnostics()));
        let outcome = xiao_driver::FrontendVmDriver::new()
            .run_artifact(&artifact, &DriverRequest::new(request));
        match id {
            "strings-path" => {
                assert!(outcome.is_success(), "字符串/路径探测失败：{outcome:?}");
                assert_eq!(
                    vm_output(&outcome),
                    expected_output.expect("成功夹具应有输出")
                );
                println!("20C-PROBE module={id} path=vm result=success output=lib/core");
            }
            "arithmetic-error" => {
                assert_eq!(
                    outcome.code(),
                    Some("X06-RUNTIME-009"),
                    "算术/错误探测必须保留稳定溢出错误码：{outcome:?}"
                );
                println!("20C-PROBE module={id} path=vm layer=runtime code=X06-RUNTIME-009");
            }
            _ => unreachable!("探测清单含未知模块"),
        }
    }
    for (id, file, expectation) in REPRESENTATIVE_PROBES {
        let path = fixture_path(file);
        let request = source_request(&path);
        let artifact = match compiler.compile(&request) {
            Ok(artifact) => artifact,
            Err(error) => {
                match expectation {
                    ProbeExpectation::Frontend(expected) => assert_eq!(
                        first_frontend_code(&error),
                        expected,
                        "代表性探测的前端错误身份不一致：{id}"
                    ),
                    ProbeExpectation::Success(_) => {
                        panic!("成功探测不得在前端拒绝：{id} {error:?}")
                    }
                    ProbeExpectation::Backend(_) => {
                        panic!("后端拒绝探测必须先通过前端：{id} {error:?}")
                    }
                }
                println!(
                    "20C-PROBE module={id} path=vm layer=frontend code={}",
                    first_frontend_code(&error)
                );
                continue;
            }
        };
        let outcome = xiao_driver::FrontendVmDriver::new()
            .run_artifact(&artifact, &DriverRequest::new(request));
        match expectation {
            ProbeExpectation::Success(expected_output) => {
                assert!(outcome.is_success(), "代表性探测失败：{id} {outcome:?}");
                assert_eq!(vm_output(&outcome), expected_output);
                println!("20C-PROBE module={id} path=vm result=success output={expected_output:?}");
            }
            ProbeExpectation::Frontend(expected) => {
                panic!("前端应提前拒绝 {id}，却进入 VM：{expected} {outcome:?}")
            }
            ProbeExpectation::Backend(_) => {
                assert_eq!(
                    outcome.code(),
                    Some("X09-BYTECODE-001"),
                    "后端探测 VM 错误身份不一致：{id} {outcome:?}"
                );
                println!("20C-PROBE module={id} path=vm layer=bytecode code=X09-BYTECODE-001");
            }
        }
    }
}

fn configured_native_toolchain() -> (TargetDescription, Toolchain) {
    let target = TargetDescription::host();
    let configured_triple = std::env::var("XIAO_TARGET_TRIPLE")
        .expect("原生探测需要显式 XIAO_TARGET_TRIPLE；准备方式见 10D §4");
    assert_eq!(configured_triple, target.triple);
    let runtime = PathBuf::from(
        std::env::var_os("XIAO_RUNTIME_LIBRARY")
            .expect("原生探测需要 XIAO_RUNTIME_LIBRARY；准备方式见 10D §4"),
    );
    assert!(
        runtime.is_file(),
        "Runtime staticlib 不存在：{}",
        runtime.display()
    );
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain = Toolchain::new(
        std::env::var_os("XIAO_CLANG").expect("原生探测需要 XIAO_CLANG；准备方式见 10D §4"),
    )
    .with_runtime_library(runtime)
    .probe_native_static_libraries(rustc, &target)
    .expect("应能读取 Runtime 原生库清单");
    (target, toolchain)
}

#[test]
#[ignore = "需要受控 LLVM 工具链、Runtime staticlib 和目标三元组；准备方式见 10D §4"]
/// 用同一份夹具跑 LLVM 原生；失败只记录真实层级，不把缺口改写成通过。
fn native_capability_probe_records_each_module() {
    let (target, toolchain) = configured_native_toolchain();
    let compiler = FrontendCompiler::new();
    let root = std::env::temp_dir().join(format!("xiao-20c-probe-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建原生探测目录");
    for (id, file, _) in PROBES {
        let path = fixture_path(file);
        let request = source_request(&path);
        let artifact = match compiler.compile(&request) {
            Ok(artifact) => artifact,
            Err(error) => {
                let code = error
                    .diagnostics()
                    .iter()
                    .find(|diagnostic| diagnostic.is_error())
                    .or_else(|| error.diagnostics().first())
                    .map_or("<none>", |diagnostic| diagnostic.code());
                println!("20C-PROBE module={id} path=llvm-native layer=frontend code={code}");
                continue;
            }
        };
        let output = root.join(if cfg!(windows) {
            format!("{id}.exe")
        } else {
            id.to_owned()
        });
        let native_request =
            NativeBuildRequest::new(request, target.clone(), toolchain.clone(), output);
        let native = match FrontendNativeDriver::new().build_artifact(&artifact, &native_request) {
            Ok(native) => native,
            Err(error) => {
                match error {
                    NativeDriverError::Frontend(frontend) => {
                        let code = frontend
                            .diagnostics()
                            .iter()
                            .find(|diagnostic| diagnostic.is_error())
                            .or_else(|| frontend.diagnostics().first())
                            .map_or("<none>", |diagnostic| diagnostic.code());
                        println!(
                            "20C-PROBE module={id} path=llvm-native layer=frontend code={code}"
                        );
                    }
                    NativeDriverError::Backend(backend) => {
                        println!(
                            "20C-PROBE module={id} path=llvm-native layer=backend detail={backend}"
                        );
                    }
                }
                continue;
            }
        };
        let run = NativeRun::new(&native.native)
            .run()
            .expect("已链接的原生产物应能启动");
        match id {
            "strings-path" => {
                assert_eq!(run.status, Some(0), "字符串/路径原生进程失败：{run:?}");
                assert_eq!(run.stdout, "lib/core\n");
                assert!(
                    run.stderr.is_empty(),
                    "成功路径不应输出 stderr：{}",
                    run.stderr
                );
                println!(
                    "20C-PROBE module={id} path=llvm-native layer=process status={:?} stdout={:?} stderr={:?}",
                    run.status, run.stdout, run.stderr
                );
            }
            "arithmetic-error" => {
                assert_eq!(run.status, Some(3), "算术/错误原生进程状态不一致：{run:?}");
                assert!(
                    run.stderr.contains("X06-RUNTIME-009"),
                    "原生 stderr 缺少稳定溢出错误码：{}",
                    run.stderr
                );
                println!(
                    "20C-PROBE module={id} path=llvm-native layer=process status={:?} stderr={:?}",
                    run.status, run.stderr
                );
            }
            _ => unreachable!("探测清单含未知模块"),
        }
    }
    for (id, file, expectation) in REPRESENTATIVE_PROBES {
        let path = fixture_path(file);
        let request = source_request(&path);
        let artifact = match compiler.compile(&request) {
            Ok(artifact) => artifact,
            Err(error) => {
                match expectation {
                    ProbeExpectation::Frontend(expected) => assert_eq!(
                        first_frontend_code(&error),
                        expected,
                        "代表性原生探测错误身份不一致：{id}"
                    ),
                    ProbeExpectation::Success(_) => {
                        panic!("成功原生探测不得在前端拒绝：{id} {error:?}")
                    }
                    ProbeExpectation::Backend(_) => {
                        panic!("后端拒绝探测必须先通过前端：{id} {error:?}")
                    }
                }
                println!(
                    "20C-PROBE module={id} path=llvm-native layer=frontend code={}",
                    first_frontend_code(&error)
                );
                continue;
            }
        };
        let output = root.join(if cfg!(windows) {
            format!("{id}.exe")
        } else {
            id.to_owned()
        });
        let native_request =
            NativeBuildRequest::new(request, target.clone(), toolchain.clone(), output);
        let native = match FrontendNativeDriver::new().build_artifact(&artifact, &native_request) {
            Ok(native) => native,
            Err(error) => match expectation {
                ProbeExpectation::Backend(fragment) => {
                    let NativeDriverError::Backend(CodegenError::Unsupported { feature, .. }) =
                        error
                    else {
                        panic!("后端拒绝探测的错误形态不一致：{id} {error}");
                    };
                    assert!(
                        feature.contains(fragment),
                        "原生拒绝信息不含期望片段：{id} {feature}"
                    );
                    println!(
                        "20C-PROBE module={id} path=llvm-native layer=backend detail={feature}"
                    );
                    continue;
                }
                ProbeExpectation::Success(_) => panic!("成功代表性原生构建失败：{id} {error}"),
                ProbeExpectation::Frontend(expected) => {
                    panic!("前端已应拒绝 {id}，却进入原生构建：{expected} {error}")
                }
            },
        };
        let ProbeExpectation::Success(expected_output) = expectation else {
            panic!("前端/后端拒绝探测不应生成原生产物：{id}");
        };
        let run = NativeRun::new(&native.native)
            .run()
            .expect("已链接的代表性原生产物应能启动");
        assert_eq!(run.status, Some(0), "代表性原生进程失败：{id} {run:?}");
        assert_eq!(run.stdout, expected_output);
        assert!(
            run.stderr.is_empty(),
            "代表性成功路径不应输出 stderr：{run:?}"
        );
        println!(
            "20C-PROBE module={id} path=llvm-native layer=process status={:?} stdout={:?} stderr={:?}",
            run.status, run.stdout, run.stderr
        );
    }
    let _ = fs::remove_dir_all(root);
}
