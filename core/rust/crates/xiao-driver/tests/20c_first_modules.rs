//! 20C 首批官方库模块的四件套回归。
//!
//! 测试通过保留的 `stdlib` 命名空间加载规范源码，避免把官方库伪装成普通包；
//! 原生测试使用同一份调用者源码，在受控工具链缺失时保持 ignored。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use xiao_codegen_llvm::{CodegenError, TargetDescription, Toolchain};
use xiao_driver::{
    DriverOutcome, DriverRequest, FrontendArtifact, FrontendCompiler, FrontendNativeDriver,
    FrontendRequest, FrontendVmDriver, NativeBuildRequest, NativeDriverError,
};
use xiao_ir::IrStatementKind;
use xiao_runtime::{ReleaseEvent, start_release_trace, take_release_events};
use xiao_types::ExternalNamespaces;
use xiao_vm::VmEvent;

/// 避免并行模块回归复用相同临时项目路径。
static PROJECT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 为单个模块调用用例建立隔离项目，并在用例结束后安全清理。
struct TestProject(PathBuf);

impl TestProject {
    /// 创建项目根并写入其唯一入口源码。
    fn new(source: &str) -> Self {
        let sequence = PROJECT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xiao-20c-project-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("创建隔离项目目录");
        fs::write(root.join("main.xiao"), source).expect("写入隔离项目入口");
        Self(root)
    }

    /// 返回项目入口路径。
    fn main_path(&self) -> PathBuf {
        self.0.join("main.xiao")
    }

    /// 从入口源码和项目根建立完整前端请求。
    fn request(&self, source: &str) -> FrontendRequest {
        let mut context = xiao_driver::FrontendContext::host();
        context.project_root = Some(self.0.clone());
        request(&self.main_path(), source).with_context(context)
    }

    /// 在临时项目中创建辅助模块或冲突夹具。
    fn write(&self, relative: &str, source: &str) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("创建临时模块父目录");
        }
        fs::write(path, source).expect("写入临时项目模块");
    }
}

impl Drop for TestProject {
    /// 仅清理本测试创建、且仍位于系统临时目录下的项目树。
    fn drop(&mut self) {
        if let (Ok(path), Ok(temp)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize()) {
            if path.starts_with(temp)
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("xiao-20c-project-"))
            {
                let _ = fs::remove_dir_all(path);
            }
        }
    }
}

/// 定位仓库根目录，以便测试读取与正式发布相同的 `stdlib/` 源码。
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("xiao-driver crate 应位于仓库 core/rust/crates 下")
        .to_path_buf()
}

/// 读取指定首批标准库模块，并返回其稳定的源码路径与文本。
fn module_source(module: &str) -> (PathBuf, String) {
    let path = repository_root()
        .join("stdlib")
        .join(format!("{module}.xiao"));
    let source = fs::read_to_string(&path).expect("读取官方库模块源码");
    (path, source)
}

/// 用模块路径构造前端请求，保持 VM 与原生测试的输入完全一致。
fn request(path: &Path, source: &str) -> FrontendRequest {
    FrontendRequest::from_text_at(source, path)
}

/// 提取成功执行结果中的标准输出事件，供 VM/原生结果对账使用。
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

/// 判断执行事件中是否出现指定标准库模块的加载记录。
fn loaded(outcome: &DriverOutcome, identity: &str) -> bool {
    outcome
        .as_executed()
        .into_iter()
        .flat_map(|execution| execution.events())
        .any(|event| matches!(event, VmEvent::ModuleLoaded { module } if module == identity))
}

/// 统计 VM 侧 Runtime 对象最终销毁数。
fn vm_destroy_count(events: &[ReleaseEvent]) -> usize {
    events
        .iter()
        .filter(|event| event.action.as_str() == "destroy")
        .count()
}

/// 统计原生产物输出的 Runtime 对象最终销毁数。
fn native_destroy_count(path: &Path) -> usize {
    let contents = fs::read_to_string(path).expect("原生产物应写出 Runtime 释放轨迹");
    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| {
            let mut fields = line.split('\t');
            let _sequence = fields.next().expect("释放轨迹缺少序号");
            let _identity = fields.next().expect("释放轨迹缺少对象身份");
            let action = fields.next().expect("释放轨迹缺少动作");
            assert!(fields.next().is_none(), "释放轨迹字段过多：{line}");
            action == "destroy"
        })
        .count()
}

/// 创建 LLVM 与 VM 差分共用的标准库静态链接产物。
fn static_standard_library_artifact(artifact: &FrontendArtifact) -> FrontendArtifact {
    let mut linked = artifact.clone();
    linked.ir.body.retain(|statement| {
        !matches!(
            &statement.kind,
            IrStatementKind::Import { items, .. }
                if !items.is_empty()
                    && items.iter().all(|item| {
                        item.module.starts_with("stdlib.") && item.selected.is_some()
                    })
        )
    });
    linked
}

/// 通过工具链标准库加载器执行真实导入回归。
fn run_vm(module: &str, caller: &str) -> DriverOutcome {
    let project = TestProject::new(caller);
    let frontend_request = project.request(caller);
    let artifact = FrontendCompiler::new()
        .compile(&frontend_request)
        .unwrap_or_else(|error| panic!("{module} 模块 VM 编译失败：{error:?}"));
    FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(frontend_request))
}

/// 编译负例并返回首个错误诊断码，用于验证模块移除和类型边界。
fn first_error_code(source_path: &Path, source: &str) -> Option<String> {
    let request = request(source_path, source);
    FrontendCompiler::new()
        .compile(&request)
        .err()
        .and_then(|error| {
            error
                .diagnostics()
                .iter()
                .find(|diagnostic| diagnostic.is_error())
                .or_else(|| error.diagnostics().first())
                .map(|diagnostic| diagnostic.code().to_owned())
        })
}

#[test]
/// 验证 text 模块的 VM 结果、类型拒绝和移除边界。
fn text_module_vm_result_and_removal_boundary_are_stable() {
    let caller = "from stdlib.text import contains_char\nfrom stdlib.text import length\nfrom stdlib.text import is_ascii_upper\nif not contains_char(\"lib/core\", \"/\")\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nif contains_char(\"lib/core\", \"x\")\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nif length(\"小雪\") != 2\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nif not is_ascii_upper(\"A\")\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nprint(\"ok\")\n";
    let outcome = run_vm("text", caller);
    assert!(outcome.is_success(), "text VM 调用失败：{outcome:?}");
    assert_eq!(output(&outcome), "ok\n");
    assert!(loaded(&outcome, "stdlib.text"));

    let path = repository_root().join("stdlib/text.xiao");
    let (_, source) = module_source("text");
    let caller_without_module = "print(contains_char(\"lib/core\", \"/\"))\n";
    assert!(
        first_error_code(&path, caller_without_module).is_some(),
        "移除 text 模块实现后，调用者必须失败"
    );
    let wrong_type = format!("{source}\nprint(contains_char(1, \"/\"))\n");
    assert!(
        first_error_code(&path, &wrong_type).is_some(),
        "text 参数类型错误必须在前端拒绝"
    );
    let imported_wrong_type = "from stdlib.text import contains_char\ncontains_char(1, \"/\")\n";
    assert_eq!(
        first_error_code(
            &repository_root().join("tests/20c_text_wrong_type.xiao"),
            imported_wrong_type
        )
        .as_deref(),
        Some("X04-TYPE-002"),
        "专用标准库签名必须保留稳定的调用参数诊断"
    );
    assert_eq!(
        first_error_code(
            &repository_root().join("tests/20c_text_length_wrong_type.xiao"),
            "from stdlib.text import length\nlength(1)\n",
        )
        .as_deref(),
        Some("X04-TYPE-002"),
        "text.length 导入签名必须保留稳定的调用参数诊断"
    );
    assert!(
        !source.contains('[') && !source.contains("dict") && !source.contains("serialize"),
        "text 模块不得构造容器、序列化对象或字典键"
    );

    assert!(
        first_error_code(
            &repository_root().join("tests/20c_missing_standard_module.xiao"),
            "from stdlib.removed import value\nprint(value)\n"
        )
        .is_some(),
        "未知标准库模块必须由前端以稳定模块诊断拒绝"
    );

    let missing_source = "from stdlib.removed import value\nprint(value)\n";
    let missing_project = TestProject::new(missing_source);
    let missing_error = FrontendCompiler::new()
        .compile(&missing_project.request(missing_source))
        .expect_err("未知标准库模块必须被拒绝");
    assert_eq!(
        missing_error
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == "X05-MODULE-004")
            .count(),
        1,
        "项目索引与标准库索引不得为同一未知标准库模块重复报告诊断"
    );
}

#[test]
/// 项目不得通过同名路径覆盖工具链保留的标准库命名空间。
fn project_cannot_shadow_the_reserved_standard_library_namespace() {
    let source = "from stdlib.text import contains_char\nprint(contains_char(\"x\", \"x\"))\n";
    let project = TestProject::new(source);
    project.write(
        "stdlib/text.xiao",
        "def contains_char(str text, str needle) -> bool\n    return false\n",
    );
    let error = FrontendCompiler::new()
        .compile(&project.request(source))
        .expect_err("项目声明的 stdlib 路径必须被拒绝");
    assert!(error.diagnostics.iter().any(|item| {
        item.code() == "X05-MODULE-003"
            && item.message().contains("保留的官方标准库命名空间 stdlib")
    }));

    let plain_source = "print(\"project\")\n";
    let plain_project = TestProject::new(plain_source);
    plain_project.write("stdlib/placeholder.xiao", "value = 1\n");
    let plain_error = FrontendCompiler::new()
        .compile(&plain_project.request(plain_source))
        .expect_err("即使入口不导入 stdlib，项目覆盖保留目录也必须被拒绝");
    assert!(plain_error.diagnostics.iter().any(|item| {
        item.code() == "X05-MODULE-003"
            && item.message().contains("保留的官方标准库命名空间 stdlib")
    }));

    let mut package_members = std::collections::BTreeMap::new();
    package_members.insert("stdlib".to_owned(), BTreeSet::new());
    let mut package_context = xiao_driver::FrontendContext::host();
    package_context.package_namespaces = Some(ExternalNamespaces {
        members: package_members,
    });
    let package_error = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(plain_source).with_context(package_context))
        .expect_err("普通包不得占用 stdlib 保留根");
    assert!(package_error.diagnostics.iter().any(|item| {
        item.code() == "X05-MODULE-003"
            && item.message().contains("保留的官方标准库命名空间 stdlib")
    }));
}

#[test]
/// 标准库导入也必须能从项目辅助模块进入 VM/原生共享的项目索引。
fn helper_module_can_import_standard_library() {
    let source = "from helper import has_separator\nif not has_separator(\"lib/core\")\n    raise ArithmeticError(code = \"HELPER_TEXT\")\n";
    let project = TestProject::new(source);
    project.write(
        "helper.xiao",
        "from stdlib.text import contains_char\ndef has_separator(str value) -> bool\n    return contains_char(value, \"/\")\n",
    );
    let artifact = FrontendCompiler::new()
        .compile(&project.request(source))
        .unwrap_or_else(|error| panic!("辅助模块导入标准库失败：{error:?}"));
    assert!(
        artifact
            .ir
            .modules
            .iter()
            .any(|module| module.name == "stdlib.text"),
        "项目辅助模块引用标准库时，标准库模块必须进入共享 IR 索引"
    );
    let outcome = FrontendVmDriver::new()
        .run_artifact(&artifact, &DriverRequest::new(project.request(source)));
    assert!(outcome.is_success(), "辅助模块标准库调用失败：{outcome:?}");
    assert!(loaded(&outcome, "project:helper"));
    assert!(loaded(&outcome, "stdlib.text"));

    project.write("helper.xiao", "from stdlib.removed import value\n");
    let missing_error = FrontendCompiler::new()
        .compile(&project.request(source))
        .expect_err("辅助模块的未知标准库导入不得被项目镜像过滤吞掉");
    assert_eq!(
        missing_error
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == "X05-MODULE-004")
            .count(),
        1,
        "辅助模块未知标准库导入必须产生唯一的稳定模块错误"
    );
}

#[test]
/// 不经 VM 加载绑定直接运行时，即使前端保留静态函数也必须失败。
fn standard_library_import_requires_vm_loader_binding() {
    for source in [
        "from stdlib.text import contains_char\nprint(contains_char(\"x\", \"x\"))\n",
        "from stdlib.path import join\nprint(join(\"lib\", \"core\"))\n",
    ] {
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .expect("标准库导入前端成功");
        let program = xiao_bytecode::lower_program(&artifact.ir);
        let outcome = xiao_vm::run_request(&xiao_vm::RunRequest::new(&artifact.ir, &program));
        let xiao_vm::RunResult::Error(error) = outcome.result else {
            panic!("移除 VM 加载绑定后必须失败：{outcome:?}");
        };
        assert_eq!(error.code(), "X06-RUNTIME-012");
        assert!(error.message().contains("未注册模块编译器"));
    }
}

#[test]
#[ignore = "需要受控 LLVM 工具链、Runtime staticlib 和目标三元组；准备方式见 10D §4"]
/// 删除 LLVM 标准库函数体的静态绑定后，原生构建必须在后端失败。
fn removing_llvm_standard_library_function_binding_rejects_native_build() {
    let (target, toolchain) = configured_native_toolchain();
    let caller = "from stdlib.path import join\nprint(join(\"lib\", \"core\"))\n";
    let project = TestProject::new(caller);
    let request = project.request(caller);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("标准库原生移除夹具前端应通过");
    let mut broken = artifact.clone();
    broken.ir.body.retain(|statement| {
        !matches!(
            &statement.kind,
            IrStatementKind::Function { name, .. } if name.text == "join"
        )
    });
    broken
        .ir
        .modules
        .iter_mut()
        .find(|module| module.name == "stdlib.path")
        .expect("前端产物应包含 stdlib.path 模块")
        .body
        .clear();
    let output = std::env::temp_dir().join(format!(
        "xiao-20c-removed-native-binding-{}.exe",
        std::process::id()
    ));
    let result = FrontendNativeDriver::new().build_artifact(
        &broken,
        &NativeBuildRequest::new(request, target, toolchain, &output),
    );
    let _ = fs::remove_file(&output);
    let failed_as_expected = matches!(
        &result,
        Err(NativeDriverError::Backend(CodegenError::Unsupported { .. }))
    );
    assert!(
        failed_as_expected,
        "删除 LLVM 标准库函数绑定后必须在原生后端拒绝：{result:?}"
    );
}

#[test]
/// 验证 path 模块的 VM 结果、类型拒绝和移除边界。
fn path_module_vm_result_and_removal_boundary_are_stable() {
    let outcome = run_vm(
        "path",
        "from stdlib.path import join\nprint(join(\"lib\", \"core\"))\n",
    );
    assert!(outcome.is_success(), "path VM 调用失败：{outcome:?}");
    assert_eq!(output(&outcome), "lib/core\n");
    assert!(loaded(&outcome, "stdlib.path"));

    let (path, source) = module_source("path");
    let caller_without_module = "print(join(\"lib\", \"core\"))\n";
    assert!(
        first_error_code(&path, caller_without_module).is_some(),
        "移除 path 模块实现后，调用者必须失败"
    );
    let wrong_type = "from stdlib.path import join\nprint(join(\"lib\", 1))\n";
    assert_eq!(
        first_error_code(
            &repository_root().join("tests/20c_path_wrong_type.xiao"),
            wrong_type
        )
        .as_deref(),
        Some("X04-TYPE-002"),
        "path 导入签名必须保留稳定的调用参数诊断"
    );
    assert!(
        !source.contains('[') && !source.contains("dict") && !source.contains("serialize"),
        "path 模块不得构造容器、序列化对象或字典键"
    );
}

/// 从受控环境变量加载并校验首批模块原生测试所需工具链。
fn configured_native_toolchain() -> (TargetDescription, Toolchain) {
    let target = TargetDescription::host();
    let configured_triple = std::env::var("XIAO_TARGET_TRIPLE")
        .expect("原生首批模块测试需要 XIAO_TARGET_TRIPLE；准备方式见 10D §4");
    assert_eq!(configured_triple, target.triple);
    let runtime = PathBuf::from(
        std::env::var_os("XIAO_RUNTIME_LIBRARY")
            .expect("原生首批模块测试需要 XIAO_RUNTIME_LIBRARY；准备方式见 10D §4"),
    );
    assert!(
        runtime.is_file(),
        "Runtime staticlib 不存在：{}",
        runtime.display()
    );
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let toolchain = Toolchain::new(
        std::env::var_os("XIAO_CLANG").expect("原生首批模块测试需要 XIAO_CLANG；准备方式见 10D §4"),
    )
    .with_llvm_as(
        std::env::var_os("XIAO_LLVM_AS")
            .expect("原生首批模块测试需要 XIAO_LLVM_AS；准备方式见 10D §4"),
    )
    .with_runtime_library(runtime)
    .probe_native_static_libraries(rustc, &target)
    .expect("应能读取 Runtime 原生库清单");
    (target, toolchain)
}

#[test]
#[ignore = "需要受控 LLVM 工具链、Runtime staticlib 和目标三元组；准备方式见 10D §4"]
/// 在受控原生工具链上核对两个首批模块的构建与运行输出。
fn first_modules_match_native_results() {
    let (target, toolchain) = configured_native_toolchain();
    let root = std::env::temp_dir().join(format!("xiao-20c-first-modules-{}", std::process::id()));
    fs::create_dir_all(&root).expect("创建首批模块原生测试目录");
    let cases = [
        (
            "path",
            "from stdlib.path import join\nprint(join(\"lib\", \"core\"))\n",
            "lib/core\n",
        ),
        (
            "text",
            "from stdlib.text import contains_char\nfrom stdlib.text import length\nfrom stdlib.text import is_ascii_upper\nif not contains_char(\"lib/core\", \"/\")\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nif contains_char(\"lib/core\", \"x\")\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nif length(\"小雪\") != 2\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nif not is_ascii_upper(\"A\")\n    raise ArithmeticError(code = \"TEXT_MODULE\")\nprint(\"ok\")\n",
            "ok\n",
        ),
        (
            "set-alias",
            "from stdlib.set import size as count\nprint(count({\"a\", \"b\"}))\n",
            "2\n",
        ),
        (
            "text-alias",
            "from stdlib.text import length as len\nprint(len(\"小雪\"))\n",
            "2\n",
        ),
        (
            "set-mutation",
            "from stdlib.set import add, remove, contains, union, intersection, difference\nvalues = {1, 2}\nalias = values\nadd(alias, 3)\nadd(values, 3)\nfor value in union(values, {3, 4})\n    print(value)\nfor value in intersection(values, {2, 4, 3})\n    print(value)\nfor value in difference(values, {1})\n    print(value)\nremove(alias, 2)\nprint(contains(values, 2))\n",
            "1\n2\n3\n4\n2\n3\n2\n3\nfalse\n",
        ),
        (
            "set-snapshot",
            "from stdlib.set import add, remove, size\nvalues = {1, 2, 3}\nfor value in values\n    remove(values, value)\n    add(values, value + 10)\n    print(value)\nprint(size(values))\nfor value in values\n    print(value)\n",
            "1\n2\n3\n3\n11\n12\n13\n",
        ),
        (
            "set-return",
            "def first(set<int> values) -> int\n    for value in values\n        return value\n    return 0\nprint(first({1, 2}))\n",
            "1\n",
        ),
        (
            "set-methods",
            "values = {1, 2}\nalias = values\nalias.add(3)\nvalues.add(3)\nfor value in values.union({3, 4})\n    print(value)\nalias.remove(2)\nprint(values.contains(2))\nprint(values.size())\n",
            "1\n2\n3\n4\nfalse\n2\n",
        ),
    ];
    for (module, caller, expected_stdout) in cases {
        let source = caller;
        let project = TestProject::new(source);
        let frontend_request = project.request(source);
        let artifact = FrontendCompiler::new()
            .compile(&frontend_request)
            .unwrap_or_else(|error| panic!("{module} 原生前端失败：{error:?}"));
        let output_path = root.join(if cfg!(windows) {
            format!("{module}.exe")
        } else {
            module.to_owned()
        });
        let build_request = NativeBuildRequest::new(
            frontend_request.clone(),
            target.clone(),
            toolchain.clone(),
            output_path,
        );
        let native = FrontendNativeDriver::new()
            .build_artifact(&artifact, &build_request)
            .unwrap_or_else(|error| match error {
                NativeDriverError::Frontend(frontend) => {
                    panic!("{module} 原生前端失败：{frontend}")
                }
                NativeDriverError::Backend(backend) => panic!("{module} 原生后端失败：{backend}"),
            });
        let trace_path = root.join(format!("{module}.release-trace.tsv"));
        let trace = start_release_trace();
        let linked_artifact = if module.ends_with("-alias") {
            // 别名导入本身就是原生路径的回归对象；保留导入让 VM 与原生都走
            // 同一份绑定，避免静态链接夹具把别名绑定一并删掉。
            artifact.clone()
        } else {
            static_standard_library_artifact(&artifact)
        };
        let vm = FrontendVmDriver::new()
            .run_artifact(&linked_artifact, &DriverRequest::new(frontend_request));
        let vm_releases = take_release_events();
        drop(trace);
        assert!(vm.is_success(), "{module} VM 执行失败：{vm:?}");
        assert_eq!(output(&vm), expected_stdout, "{module} VM 输出不符");

        let run = Command::new(&native.native.executable)
            .env("XIAO_RUNTIME_RELEASE_TRACE_PATH", &trace_path)
            .output()
            .unwrap_or_else(|error| panic!("{module} 原生产物启动失败：{error}"));
        let stdout = String::from_utf8_lossy(&run.stdout);
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert_eq!(
            run.status.code(),
            Some(0),
            "{module} 原生退出状态不符：{stderr}"
        );
        assert_eq!(stdout, expected_stdout, "{module} VM/原生输出不一致");
        assert!(
            stderr.is_empty(),
            "{module} 成功路径不应输出 stderr：{stderr}"
        );
        assert_eq!(
            linked_artifact.ir.ownership.release_plans, artifact.ir.ownership.release_plans,
            "{module} 的静态链接不得改变前端冻结的释放动作顺序"
        );
        assert_eq!(
            native.frontend.ir.ownership.release_plans, artifact.ir.ownership.release_plans,
            "{module} LLVM 构建必须保留前端冻结的释放动作顺序"
        );
        assert_eq!(
            native_destroy_count(&trace_path),
            vm_destroy_count(&vm_releases),
            "{module} VM/原生 Runtime 最终对象销毁数不一致"
        );
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
#[ignore = "需要受控 LLVM 工具链、Runtime staticlib 和目标三元组；准备方式见 10D §4"]
/// 未捕获的集合删除缺失错误在 VM 与原生边界保留相同错误码。
fn set_remove_missing_native_matches_vm() {
    let (target, toolchain) = configured_native_toolchain();
    let source = "from stdlib.set import remove\nremove({1}, 2)\n";
    let project = TestProject::new(source);
    let request = project.request(source);
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("集合缺失错误原生夹具前端应通过");
    let linked = static_standard_library_artifact(&artifact);
    let vm = FrontendVmDriver::new().run_artifact(&linked, &DriverRequest::new(request.clone()));
    assert_eq!(vm.code(), Some("SET_ELEMENT_MISSING"), "{vm:?}");
    let output = std::env::temp_dir().join(format!(
        "xiao-20g-set-remove-missing-{}.exe",
        std::process::id()
    ));
    let native = FrontendNativeDriver::new()
        .build_artifact(
            &artifact,
            &NativeBuildRequest::new(request, target, toolchain, &output),
        )
        .expect("集合缺失错误原生构建应通过");
    let run = Command::new(&native.native.executable)
        .output()
        .expect("运行集合缺失错误原生夹具");
    let _ = fs::remove_file(&output);
    assert_eq!(run.status.code(), Some(3), "原生退出码：{run:?}");
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("SET_ELEMENT_MISSING"),
        "原生 stderr：{}",
        String::from_utf8_lossy(&run.stderr)
    );
}
