//! 05-B 本地模块发现、导出和依赖图规格测试。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use xiao_modules::{
    BindingKind, ExportOrigin, ImportEdgeKind, ModuleKind, ModuleName, analyze_project,
};

/// 为并行测试 fixture 提供进程内唯一的递增后缀。
static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);

/// 一个测试用隔离项目；测试结束后递归清理自身目录。
struct TempProject {
    path: PathBuf,
}

impl TempProject {
    /// 创建一个不会与其他测试共享的临时项目根。
    fn new() -> Self {
        let base = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let path = base.join(format!("xiao-modules-{stamp}-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).expect("create temporary project");
        Self { path }
    }

    /// 写入一个相对项目根的 UTF-8 Xiao 文件。
    fn write(&self, relative: impl AsRef<Path>, text: &str) {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture directory");
        }
        fs::write(path, text).expect("write fixture");
    }
}

impl Drop for TempProject {
    /// 删除测试项目及其全部 fixture。
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// 从项目结果中提取稳定的诊断编号。
fn diagnostic_codes(result: &xiao_modules::ProjectModuleResult) -> Vec<&str> {
    result
        .diagnostics
        .iter()
        .map(|entry| entry.diagnostic.code())
        .collect()
}

/// 在依赖序列中返回模块位置。
fn order_index(result: &xiao_modules::ProjectModuleResult, name: &str) -> usize {
    result
        .graph
        .initialization_order
        .iter()
        .position(|module| module.to_string() == name)
        .expect("module should be in initialization order")
}

#[test]
/// 文件、目录命名空间、根配置、嵌套配置边界和点目录规则保持确定。
fn discovers_files_and_namespaces_with_project_boundaries() {
    let project = TempProject::new();
    project.write("config.xiao", "[project]\nname = \"demo\"\n");
    project.write("main.xiao", "print(1)\n");
    project.write("app/user.xiao", "name = \"user\"\n");
    project.write("app/http/client.xiao", "name = \"client\"\n");
    project.write("target/generated.xiao", "value = 1\n");
    project.write("node_modules/vendor.xiao", "value = 2\n");
    project.write(".hidden/secret.xiao", "value = 3\n");
    project.write("nested/config.xiao", "[package]\nname = \"nested\"\n");
    project.write("nested/ignored.xiao", "value = 4\n");

    let result = analyze_project(&project.path);
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    let names = result
        .modules
        .keys()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "app.http.client",
            "app.user",
            "main",
            "node_modules.vendor",
            "target.generated",
        ]
    );
    assert!(
        !result
            .modules
            .contains_key(&ModuleName::new(vec!["config".to_owned()]))
    );
    assert!(
        !result
            .modules
            .keys()
            .any(|name| name.to_string().starts_with("nested"))
    );
    assert!(
        result
            .namespaces
            .contains_key(&ModuleName::new(vec!["app".to_owned()]))
    );
    assert!(
        result
            .namespaces
            .contains_key(&ModuleName::new(vec!["app".to_owned(), "http".to_owned()]))
    );
    assert!(
        result
            .namespaces
            .contains_key(&ModuleName::new(vec!["target".to_owned()]))
    );
    assert_eq!(
        result.modules[&ModuleName::new(vec!["app".to_owned(), "user".to_owned()])].kind,
        ModuleKind::File
    );
}

#[test]
/// 文件模块与目录命名空间同名时报告冲突而不静默选择一方。
fn diagnoses_file_namespace_conflict() {
    let project = TempProject::new();
    project.write("app.xiao", "value = 1\n");
    project.write("app/user.xiao", "value = 2\n");

    let result = analyze_project(&project.path);
    assert!(diagnostic_codes(&result).contains(&"X05-MODULE-003"));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|entry| entry.diagnostic.message_id() == "x05.module.file_namespace_conflict")
    );
}

#[test]
/// 逻辑名称大小写折叠后冲突，导入解析仍保持大小写精确匹配。
fn diagnoses_case_fold_conflicts_and_preserves_case_sensitivity() {
    let conflict = TempProject::new();
    // Windows 文件系统不允许仅大小写不同的同名文件，因此用文件与
    // 大小写不同的目录命名空间构造同一个逻辑冲突。
    conflict.write("App.xiao", "value = 1\n");
    conflict.write("app/user.xiao", "value = 2\n");
    let result = analyze_project(&conflict.path);
    assert!(diagnostic_codes(&result).contains(&"X05-MODULE-003"));

    let exact = TempProject::new();
    exact.write("App.xiao", "value = 1\n");
    exact.write("main.xiao", "import app\n");
    let result = analyze_project(&exact.path);
    assert!(diagnostic_codes(&result).contains(&"X05-MODULE-004"));
}

#[test]
/// 直接导入、块内导入和选择导入都会进入依赖图；初始化顺序为依赖优先。
fn resolves_imports_bindings_and_initialization_order() {
    let project = TempProject::new();
    project.write("main.xiao", "import app.user\nimport app.http\nfrom util import value as imported\ndef load()\n    import lazy.mod\n    return imported\n");
    project.write("app/user.xiao", "user_value = 1\n");
    project.write("app/http/client.xiao", "client_value = 2\n");
    project.write("util.xiao", "value = 3\n");
    project.write("lazy/mod.xiao", "lazy_value = 4\n");

    let result = analyze_project(&project.path);
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    let main = ModuleName::new(vec!["main".to_owned()]);
    let edges = &result.graph.edges[&main];
    assert!(edges.iter().any(|edge| {
        edge.target == ModuleName::new(vec!["app".to_owned(), "user".to_owned()])
            && edge.kind == ImportEdgeKind::Import
    }));
    assert!(edges.iter().any(|edge| {
        edge.target == ModuleName::new(vec!["lazy".to_owned(), "mod".to_owned()])
            && edge.kind == ImportEdgeKind::Import
    }));
    assert!(order_index(&result, "app.user") < order_index(&result, "main"));
    assert!(order_index(&result, "util") < order_index(&result, "main"));
    assert!(order_index(&result, "lazy.mod") < order_index(&result, "main"));

    let imported_binding = result
        .bindings
        .iter()
        .find(|binding| binding.module == main && binding.local_name == "imported")
        .expect("selected import binding");
    assert!(matches!(imported_binding.kind, BindingKind::Value { .. }));
    let app_bindings = result
        .bindings
        .iter()
        .filter(|binding| binding.module == main && binding.local_name == "app")
        .collect::<Vec<_>>();
    assert_eq!(app_bindings.len(), 2, "two namespace imports should merge");
    assert!(app_bindings.iter().all(|binding| {
        matches!(
            binding.kind,
            BindingKind::Qualifier {
                kind: ModuleKind::Namespace,
                ..
            }
        )
    }));
}

#[test]
/// 块内导入只在自身作用域生效，不会成为模块导出接口。
fn keeps_block_imports_local() {
    let project = TempProject::new();
    project.write(
        "main.xiao",
        "def load()\n    import worker\n    return worker.value\n",
    );
    project.write("worker.xiao", "value = 1\n");

    let result = analyze_project(&project.path);
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    let main = &result.modules[&ModuleName::new(vec!["main".to_owned()])];
    assert!(!main.symbols.contains_key("worker"));
    let worker_binding = result
        .bindings
        .iter()
        .find(|binding| binding.local_name == "worker")
        .expect("block import binding");
    assert_ne!(worker_binding.scope.0, 0);
}

#[test]
/// 顶层选择导入可以逐层再导出，并保留最初符号来源。
fn propagates_top_level_reexports() {
    let project = TempProject::new();
    project.write("base.xiao", "value = 1\n");
    project.write("bridge.xiao", "from base import value\n");
    project.write("main.xiao", "from bridge import value\n");

    let result = analyze_project(&project.path);
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    for module_name in ["bridge", "main"] {
        let module = &result.modules[&ModuleName::new(vec![module_name.to_owned()])];
        let symbol = module.symbols.get("value").expect("re-exported value");
        assert_eq!(
            symbol.origin,
            ExportOrigin::Reexport {
                module: ModuleName::new(vec!["base".to_owned()]),
                name: "value".to_owned(),
            }
        );
    }
    assert!(order_index(&result, "base") < order_index(&result, "bridge"));
    assert!(order_index(&result, "bridge") < order_index(&result, "main"));
}

#[test]
/// 缺失模块、文件符号和命名空间子模块分别给出稳定诊断，且不重复报告。
fn diagnoses_missing_targets_and_symbols_once() {
    let project = TempProject::new();
    project.write(
        "main.xiao",
        "import missing\nfrom base import absent\nfrom ns import absent\n",
    );
    project.write("base.xiao", "value = 1\n");
    project.write("ns/child.xiao", "value = 2\n");

    let result = analyze_project(&project.path);
    let codes = diagnostic_codes(&result);
    assert_eq!(
        codes
            .iter()
            .filter(|code| **code == "X05-MODULE-004")
            .count(),
        2
    );
    assert_eq!(
        codes
            .iter()
            .filter(|code| **code == "X05-MODULE-005")
            .count(),
        1
    );
}

#[test]
/// 循环依赖立即报错，并且不会向后端暴露部分初始化顺序。
fn rejects_cycles_and_clears_initialization_order() {
    let project = TempProject::new();
    project.write("a.xiao", "import b\n");
    project.write("b.xiao", "import a\n");

    let result = analyze_project(&project.path);
    let codes = diagnostic_codes(&result);
    assert_eq!(
        codes
            .iter()
            .filter(|code| **code == "X05-MODULE-007")
            .count(),
        1
    );
    assert!(result.graph.initialization_order.is_empty());
}

#[test]
/// 命名空间限定访问按需登记具体文件边，限定符本身不是普通值。
fn records_qualified_namespace_use() {
    let project = TempProject::new();
    project.write("main.xiao", "import app\nvalue = app.user.value\n");
    project.write("app/user.xiao", "value = 1\n");

    let result = analyze_project(&project.path);
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    let main = ModuleName::new(vec!["main".to_owned()]);
    let edge = result.graph.edges[&main]
        .iter()
        .find(|edge| edge.target == ModuleName::new(vec!["app".to_owned(), "user".to_owned()]))
        .expect("qualified edge");
    assert_eq!(edge.kind, ImportEdgeKind::QualifiedUse);
}

#[test]
/// 命名空间限定访问缺少子模块时报告缺失目标，而不是误报限定符误用。
fn diagnoses_missing_qualified_namespace_target() {
    let project = TempProject::new();
    project.write("main.xiao", "import app\nvalue = app.missing.value\n");
    project.write("app/available.xiao", "value = 1\n");

    let result = analyze_project(&project.path);
    let codes = diagnostic_codes(&result);
    assert_eq!(
        codes
            .iter()
            .filter(|code| **code == "X05-MODULE-004")
            .count(),
        1
    );
    assert!(!codes.contains(&"X05-MODULE-008"));
}

#[test]
/// 限定符不能被赋值或单独作为普通表达式使用。
fn diagnoses_invalid_qualifier_use_and_binding_conflict() {
    let project = TempProject::new();
    project.write(
        "main.xiao",
        "import worker\nworker = 1\nimport worker as other\nimport worker as other\n",
    );
    project.write("worker.xiao", "value = 1\n");

    let result = analyze_project(&project.path);
    let codes = diagnostic_codes(&result);
    assert!(codes.contains(&"X05-MODULE-008"));
    assert!(codes.contains(&"X05-MODULE-006"));
}
