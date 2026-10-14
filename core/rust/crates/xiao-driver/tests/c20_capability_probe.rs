//! 20C §3.1 官方 Xiao 库能力探测。
//!
//! 夹具只测现有语言、Runtime 和 LLVM 后端能力，不实现标准库，不添加 intrinsic。

use std::fs;
use std::path::{Path, PathBuf};

use xiao_bytecode::{TacOp, lower_program};
use xiao_codegen_llvm::{
    CodegenError, CodegenOptions, NativeBuild, NativeRun, TargetDescription, Toolchain,
};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendCompiler, FrontendNativeDriver, FrontendRequest,
    NativeBuildRequest, NativeDriverError,
};
use xiao_syntax::{BinaryOperator, UnaryOperator};
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
        ProbeExpectation::Success("true\n"),
    ),
    (
        "second-set-membership",
        "second-set-membership.xiao",
        ProbeExpectation::Success("ok\n"),
    ),
];

/// 算子守卫的来源枚举；新增算子时这里的穷举匹配会强制同步最小夹具。
/// 按算子拼写生成跨平台安全的产物文件名片段。
fn safe_operator_stem(operator: &str) -> String {
    let mut stem = String::new();
    for character in operator.chars() {
        match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => stem.push(character),
            ' ' => stem.push('_'),
            '+' => stem.push_str("plus"),
            '-' => stem.push_str("minus"),
            '*' => stem.push_str("star"),
            '/' => stem.push_str("slash"),
            '%' => stem.push_str("percent"),
            '<' => stem.push_str("lt"),
            '>' => stem.push_str("gt"),
            '=' => stem.push_str("eq"),
            '&' => stem.push_str("amp"),
            '^' => stem.push_str("caret"),
            other => stem.push_str(&format!("u{:02x}", other as u32)),
        }
    }
    stem
}

/// 为每个二元算子提供独立、最小且可执行的源码与期望形状。
fn binary_operator_probe(operator: BinaryOperator) -> (&'static str, ProbeExpectation) {
    match operator {
        BinaryOperator::Power => ("value = 2 ** 3\n", ProbeExpectation::Success("")),
        BinaryOperator::Multiply => ("value = 2 * 3\n", ProbeExpectation::Success("")),
        BinaryOperator::Divide => ("value = 6 / 3\n", ProbeExpectation::Success("")),
        BinaryOperator::FloorDivide => ("value = 6 // 3\n", ProbeExpectation::Success("")),
        BinaryOperator::Remainder => ("value = 6 % 3\n", ProbeExpectation::Success("")),
        BinaryOperator::Add => ("value = 2 + 3\n", ProbeExpectation::Success("")),
        BinaryOperator::Subtract => ("value = 3 - 2\n", ProbeExpectation::Success("")),
        BinaryOperator::Intersect => ("value = {1} & {1}\n", ProbeExpectation::Success("")),
        BinaryOperator::SymmetricDifference => {
            ("value = {1} ^ {2}\n", ProbeExpectation::Success(""))
        }
        BinaryOperator::Less => ("value = 1 < 2\n", ProbeExpectation::Success("")),
        BinaryOperator::LessEqual => ("value = 1 <= 2\n", ProbeExpectation::Success("")),
        BinaryOperator::Greater => ("value = 2 > 1\n", ProbeExpectation::Success("")),
        BinaryOperator::GreaterEqual => ("value = 2 >= 1\n", ProbeExpectation::Success("")),
        BinaryOperator::Equal => ("value = 1 == 1\n", ProbeExpectation::Success("")),
        BinaryOperator::NotEqual => ("value = 1 != 2\n", ProbeExpectation::Success("")),
        BinaryOperator::In => ("value = 1 in {1}\n", ProbeExpectation::Success("")),
        BinaryOperator::NotIn => ("value = 2 not in {1}\n", ProbeExpectation::Success("")),
        BinaryOperator::Is => (
            "value = 1 is 1\n",
            ProbeExpectation::Frontend("X02-TYPE-005"),
        ),
        BinaryOperator::IsNot => (
            "value = 1 is not 1\n",
            ProbeExpectation::Frontend("X02-TYPE-005"),
        ),
        BinaryOperator::And => (
            "def explode(int divisor) -> bool\n    return (1 // divisor) == 0\nvalue = false and explode(0)\n",
            ProbeExpectation::Success(""),
        ),
        BinaryOperator::Or => (
            "def explode(int divisor) -> bool\n    return (1 // divisor) == 0\nvalue = true or explode(0)\n",
            ProbeExpectation::Success(""),
        ),
    }
}

/// 为每个一元算子提供独立、最小且可执行的源码与期望形状。
fn unary_operator_probe(operator: UnaryOperator) -> (&'static str, ProbeExpectation) {
    match operator {
        UnaryOperator::Plus => ("value = +1\n", ProbeExpectation::Success("")),
        UnaryOperator::Minus => ("value = -1\n", ProbeExpectation::Success("")),
        UnaryOperator::Not => ("value = not true\n", ProbeExpectation::Success("")),
    }
}

/// 返回算子守卫使用的穷举用例；期望由 `BinaryOperator`/`UnaryOperator` 的无兜底
/// 匹配派生，避免只改手写文件清单而漏掉执行点。
fn operator_probes() -> Vec<(String, String, ProbeExpectation)> {
    let mut probes = Vec::with_capacity(BinaryOperator::ALL.len() + UnaryOperator::ALL.len() + 1);
    for operator in BinaryOperator::ALL {
        let spelling = operator.as_str();
        let (source, expectation) = binary_operator_probe(operator);
        probes.push((
            format!("binary-{}", safe_operator_stem(spelling)),
            source.to_owned(),
            expectation,
        ));
    }
    for operator in UnaryOperator::ALL {
        let spelling = operator.as_str();
        let (source, expectation) = unary_operator_probe(operator);
        probes.push((
            format!("unary-{}", safe_operator_stem(spelling)),
            source.to_owned(),
            expectation,
        ));
    }
    probes.push((
        "binary-power-output".to_owned(),
        "value = 2 ** 3\nprint(value)\n".to_owned(),
        ProbeExpectation::Success("8\n"),
    ));
    probes.push((
        "backend-negative-member-value".to_owned(),
        "[[Item]]\n    def read(self) -> int\n        return 1\nitem = new Item()\nvalue = item.read\n"
            .to_owned(),
        ProbeExpectation::Backend("动态表方法值"),
    ));
    probes
}

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

#[test]
/// 所有已接受算子必须在 VM 生产入口成功；身份算子必须在类型层先拒绝。
fn operator_coverage_guard_vm() {
    let compiler = FrontendCompiler::new();
    let mut failures = Vec::new();
    for (id, source, expectation) in operator_probes() {
        let request = FrontendRequest::from_text(source);
        let artifact = match compiler.compile(&request) {
            Ok(artifact) => artifact,
            Err(error) => {
                match expectation {
                    ProbeExpectation::Frontend(expected)
                        if first_frontend_code(&error) == expected => {}
                    ProbeExpectation::Frontend(expected) => failures.push(format!(
                        "{id}: 前端码 {:?}，期望 {expected}",
                        first_frontend_code(&error)
                    )),
                    ProbeExpectation::Success(_) => {
                        failures.push(format!("{id}: 已接受算子在前端拒绝：{error:?}"));
                    }
                    ProbeExpectation::Backend(_) => {
                        failures.push(format!("{id}: 后端负例在前端提前拒绝：{error:?}"));
                    }
                }
                continue;
            }
        };
        let outcome = xiao_driver::FrontendVmDriver::new()
            .run_artifact(&artifact, &DriverRequest::new(request));
        match expectation {
            ProbeExpectation::Success(expected_output) => {
                if !outcome.is_success() {
                    failures.push(format!("{id}: VM 执行失败：{outcome:?}"));
                } else if vm_output(&outcome) != expected_output {
                    failures.push(format!(
                        "{id}: VM 输出 {:?}，期望 {expected_output:?}",
                        vm_output(&outcome)
                    ));
                }
            }
            ProbeExpectation::Frontend(expected) => {
                failures.push(format!("{id}: 前端期望 {expected}，却进入 VM：{outcome:?}"));
            }
            ProbeExpectation::Backend(_) => {
                if outcome.code() != Some("X09-BYTECODE-001") {
                    failures.push(format!(
                        "{id}: VM 码 {:?}，期望 X09-BYTECODE-001",
                        outcome.code()
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "算子守卫一次性失败：\n{}",
        failures.join("\n")
    );
}

#[test]
/// 短路右侧未执行时，两个后端的释放序列都比去掉右侧的程序多出对应临时值。
fn logical_short_circuit_keeps_shared_release_plans() {
    let source = "right_text = \"ab\"\nresult = false and (\"a\" + \"b\" == right_text)\n";
    let without_right = "right_text = \"ab\"\nresult = false\n";
    let compiler = FrontendCompiler::new();
    let vm_artifact = compiler
        .compile(&FrontendRequest::from_text(source))
        .expect("短路夹具");
    let without_artifact = compiler
        .compile(&FrontendRequest::from_text(without_right))
        .expect("无右侧对照夹具");
    let with_tac = lower_program(vm_artifact.ir());
    let without_tac = lower_program(without_artifact.ir());
    let count_tac_releases = |program: &xiao_bytecode::TacProgram| {
        program
            .functions
            .iter()
            .flat_map(|function| function.blocks.iter())
            .flat_map(|block| block.instructions.iter())
            .filter(|instruction| matches!(instruction.op, TacOp::Release { .. }))
            .count()
    };
    assert!(
        count_tac_releases(&with_tac) > count_tac_releases(&without_tac),
        "去掉右侧后 TAC 释放序列应减少"
    );
    let with_native = NativeBuild::new()
        .lower(vm_artifact.ir(), &CodegenOptions::default())
        .expect("原生短路夹具");
    let without_native = NativeBuild::new()
        .lower(without_artifact.ir(), &CodegenOptions::default())
        .expect("原生无右侧对照夹具");
    let count_native_releases = |text: &str| text.matches("xiao_runtime_value_release_").count();
    assert!(
        count_native_releases(&with_native.text) > count_native_releases(&without_native.text),
        "去掉右侧后 LLVM 释放序列应减少"
    );

    let function = with_tac.functions.first().expect("脚本入口函数");
    let short_block = function
        .blocks
        .iter()
        .find_map(|block| {
            block.instructions.iter().find_map(|instruction| {
                if let TacOp::BranchIf { if_false, .. } = &instruction.op {
                    Some(*if_false)
                } else {
                    None
                }
            })
        })
        .and_then(|block| {
            function
                .blocks
                .iter()
                .find(|candidate| candidate.id == block)
        })
        .expect("and 的短路块");
    let short_release_count = short_block
        .instructions
        .iter()
        .filter(|instruction| matches!(&instruction.op, TacOp::Release { .. }))
        .count();
    assert_eq!(
        short_release_count, 0,
        "短路块不能释放未求值右侧的临时值"
    );

    let right_start = with_native
        .text
        .find("\ndynamic.logical.right")
        .expect("LLVM 右侧短路块");
    let short_start = with_native
        .text
        .find("\ndynamic.logical.short")
        .expect("LLVM 短路块");
    let merge_start = with_native
        .text
        .find("\ndynamic.logical.merge")
        .expect("LLVM 合流块");
    assert!(
        short_start < right_start && right_start < merge_start,
        "LLVM 短路、右侧和合流标签顺序必须稳定"
    );
    assert!(
        with_native.text[short_start..right_start]
            .find("xiao_runtime_value_release_")
            .is_none(),
        "LLVM 短路块不能释放未求值右侧的临时值"
    );
    assert!(
        with_native.text[right_start..merge_start].contains("xiao_runtime_value_release_"),
        "LLVM 右侧释放必须位于 logical.right 之后"
    );

    let outcome = xiao_driver::FrontendVmDriver::new().run_artifact(
        &vm_artifact,
        &DriverRequest::new(FrontendRequest::from_text(source)),
    );
    assert!(outcome.is_success(), "短路执行失败：{outcome:?}");
    let side_effect_source = "def right() -> bool\n    text = \"not evaluated\"\n    return true\nresult = false and right()\n";
    let side_effect_request = FrontendRequest::from_text(side_effect_source);
    let side_effect_artifact = compiler
        .compile(&side_effect_request)
        .expect("短路副作用夹具");
    let side_effect_outcome = xiao_driver::FrontendVmDriver::new().run_artifact(
        &side_effect_artifact,
        &DriverRequest::new(side_effect_request),
    );
    assert!(side_effect_outcome.is_success());
    assert!(
        !side_effect_outcome
            .as_executed()
            .into_iter()
            .flat_map(|execution| execution.events())
            .any(|event| matches!(
                event,
                VmEvent::FunctionEntered { function, .. } if function == "right"
            ))
    );
}

#[test]
/// VM 观察值固定住整数幂的结果，和原生守卫的顶层退出码形成双路径对照。
fn integer_power_result_matches_vm() {
    let request = FrontendRequest::from_text("value = 2 ** 3\nprint(value)\n");
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("整数幂夹具");
    let outcome =
        xiao_driver::FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(request));
    assert!(outcome.is_success(), "VM 整数幂失败：{outcome:?}");
    assert_eq!(vm_output(&outcome), "8\n");
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
/// 原生算子守卫必须真的构建并运行每个已接受变体；文件名使用安全映射。
fn native_operator_coverage_guard() {
    let (target, toolchain) = configured_native_toolchain();
    let compiler = FrontendCompiler::new();
    let root = std::env::temp_dir().join(format!("xiao-20c-operator-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建原生算子守卫目录");
    let mut failures = Vec::new();
    for (id, source, expectation) in operator_probes() {
        let request = FrontendRequest::from_text(source);
        let artifact = match compiler.compile(&request) {
            Ok(artifact) => artifact,
            Err(error) => {
                match expectation {
                    ProbeExpectation::Frontend(expected)
                        if first_frontend_code(&error) == expected => {}
                    ProbeExpectation::Frontend(expected) => failures.push(format!(
                        "{id}: 原生前端码 {:?}，期望 {expected}",
                        first_frontend_code(&error)
                    )),
                    ProbeExpectation::Success(_) => {
                        failures.push(format!("{id}: 已接受算子在原生前端拒绝：{error:?}"));
                    }
                    ProbeExpectation::Backend(_) => {
                        failures.push(format!("{id}: 后端负例在前端提前拒绝：{error:?}"));
                    }
                }
                continue;
            }
        };
        let output = root.join(if cfg!(windows) {
            format!("{id}.exe")
        } else {
            id.clone()
        });
        let mut codegen_options = xiao_codegen_llvm::CodegenOptions::for_target(target.clone());
        let observe_power = id == "binary-starstar";
        if observe_power {
            codegen_options = codegen_options
                .with_entry_observation(xiao_codegen_llvm::EntryObservation::ExitCode);
        }
        let native_request =
            NativeBuildRequest::new(request, target.clone(), toolchain.clone(), output)
                .with_codegen_options(codegen_options);
        let native = match FrontendNativeDriver::new().build_artifact(&artifact, &native_request) {
            Ok(native) => native,
            Err(error) => match expectation {
                ProbeExpectation::Backend(fragment) => {
                    match error {
                        NativeDriverError::Backend(CodegenError::Unsupported {
                            feature, ..
                        }) if feature.contains(fragment) => {}
                        other => failures.push(format!("{id}: 后端拒绝形态/片段不一致：{other}")),
                    }
                    continue;
                }
                ProbeExpectation::Success(_) => {
                    failures.push(format!("{id}: 已接受算子原生构建失败：{error}"));
                    continue;
                }
                ProbeExpectation::Frontend(expected) => {
                    failures.push(format!(
                        "{id}: 身份算子已应在前端拒绝 {expected}，却进入原生：{error}"
                    ));
                    continue;
                }
            },
        };
        let ProbeExpectation::Success(expected_output) = expectation else {
            failures.push(format!("{id}: 拒绝期望却生成了原生产物"));
            continue;
        };
        let run = NativeRun::new(&native.native)
            .run()
            .map_err(|error| format!("{id}: 已链接产物启动失败：{error}"));
        match run {
            Ok(run) => {
                let expected_status = if observe_power { Some(8) } else { Some(0) };
                if run.status != expected_status {
                    failures.push(format!(
                        "{id}: 原生状态 {:?}，期望 {:?}，stderr {:?}",
                        run.status, expected_status, run.stderr
                    ));
                }
                if run.stdout != expected_output {
                    failures.push(format!(
                        "{id}: 原生输出 {:?}，期望 {expected_output:?}",
                        run.stdout
                    ));
                }
                if !run.stderr.is_empty() {
                    failures.push(format!("{id}: 成功路径 stderr {:?}", run.stderr));
                }
            }
            Err(error) => failures.push(error),
        }
    }
    let _ = fs::remove_dir_all(root);
    assert!(
        failures.is_empty(),
        "原生算子守卫一次性失败：\n{}",
        failures.join("\n")
    );
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
