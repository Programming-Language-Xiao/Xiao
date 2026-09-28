//! 本地模块与环境包首次引用、失败重试的运行规格。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use xiao_driver::PackageRegistry;
use xiao_driver::protocol::{
    CORE_VERSION, OptimizationConfig, PROTOCOL_VERSION, ProtocolRequest, ProtocolResponse,
    ProtocolTarget, RunOptions, SourceIdentity, dispatch,
};
use xiao_driver::{DriverOutcome, DriverRequest, FrontendContext, FrontendRequest, run};
use xiao_package::{CacheLayout, ENVIRONMENT_METADATA_FILE};
use xiao_vm::VmEvent;

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("xiao-i4a2-{}-{stamp}", std::process::id()));
        fs::create_dir(&path).expect("workspace");
        Self(path)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(path, text).expect("module source");
    }

    fn run(&self, source: &str) -> DriverOutcome {
        self.write("main.xiao", source);
        let mut context = FrontendContext::host();
        context.project_root = Some(self.0.clone());
        run(&DriverRequest::new(
            FrontendRequest::from_text_at(source, self.0.join("main.xiao")).with_context(context),
        ))
    }

    fn package_environment(
        &self,
        module_source: &str,
        auxiliary: Option<&str>,
    ) -> (CacheLayout, PathBuf) {
        let layout =
            CacheLayout::from_xiao_home(Some(&self.0.join("home")), &self.0).expect("layout");
        let environment = self.0.join("active");
        fs::create_dir_all(&environment).expect("environment");
        let digest = "a".repeat(64);
        let mappings = json!([{
            "package": { "name": "lib", "version": "1.0.0",
                "source": { "source_id": "path:/lib/1", "alias": null, "display_name": "lib" } },
            "object": { "digest": digest, "object_kind": "source" }
        }]);
        fs::write(
            environment.join(ENVIRONMENT_METADATA_FILE),
            json!({
                "metadata_version": 2, "logical_name": "active", "directory_name": "active",
                "config_fingerprint": "config", "toolchain_fingerprint": "toolchain",
                "target_fingerprint": "target", "environment_fingerprint": "environment",
                "lockfile_summary": null, "package_mappings": mappings
            })
            .to_string(),
        )
        .expect("metadata");
        let source = layout.source_object_path(&digest).expect("source object");
        fs::create_dir_all(&source).expect("source directory");
        fs::write(source.join("api.xiao"), module_source).expect("api source");
        if let Some(auxiliary) = auxiliary {
            fs::write(source.join("util.xiao"), auxiliary).expect("util source");
        }
        (layout, environment)
    }

    fn run_package(&self, layout: &CacheLayout, environment: &Path, source: &str) -> DriverOutcome {
        let mut context = FrontendContext::host();
        context.package_registry =
            Some(PackageRegistry::from_layout(layout, environment, false).expect("registry"));
        run(&DriverRequest::new(
            FrontendRequest::from_text(source).with_context(context),
        ))
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if let (Ok(path), Ok(temp)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize()) {
            if path.starts_with(temp)
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("xiao-i4a2-"))
            {
                let _ = fs::remove_dir_all(path);
            }
        }
    }
}

fn loaded(outcome: &DriverOutcome, name: &str) -> usize {
    let DriverOutcome::Executed(execution) = outcome else {
        panic!("not executed: {outcome:?}");
    };
    execution
        .outcome
        .events
        .iter()
        .filter(|event| {
            matches!(event,
        VmEvent::ModuleLoaded { module } if module == name)
        })
        .count()
}

#[test]
fn module_loading_spec_vectors() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/spec/11b-repl/module-loading.json"
    ))
    .expect("module loading vectors");
    for case in vectors["cases"].as_array().expect("cases") {
        let workspace = Workspace::new();
        for (path, content) in case["files"].as_object().expect("files") {
            workspace.write(path, content.as_str().expect("file content"));
        }
        let outcome = workspace.run(case["source"].as_str().expect("main source"));
        assert!(outcome.is_success(), "{}: {outcome:?}", case["name"]);
        for module in case["loaded"].as_array().expect("loaded modules") {
            assert_eq!(
                loaded(&outcome, module.as_str().expect("module name")),
                1,
                "{}: {outcome:?}",
                case["name"]
            );
        }
        let DriverOutcome::Executed(execution) = outcome else {
            panic!("expected execution");
        };
        let observed = execution
            .outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::ModuleLoaded { module } if module.starts_with("project:")))
            .count();
        assert_eq!(
            observed,
            case["loaded"].as_array().unwrap().len(),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn protocol_run_loads_a_project_import_from_the_source_path() {
    let workspace = Workspace::new();
    let source = "import helper\nvalue = helper.answer\n";
    workspace.write("main.xiao", source);
    workspace.write("helper.xiao", "answer = 42\n");
    let response = dispatch(ProtocolRequest::Run {
        request_id: "module-loading".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: OptimizationConfig::default(),
        source: SourceIdentity {
            module: "main".to_owned(),
            path: Some(workspace.0.join("main.xiao").display().to_string()),
            text: source.to_owned(),
        },
        options: RunOptions::default(),
    });
    let ProtocolResponse::Result {
        exit_code: 0,
        events,
        ..
    } = &response
    else {
        panic!("protocol run must succeed: {response:?}");
    };
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == "module_loaded"
                && event.data.get("module") == Some(&json!("project:helper")))
            .count(),
        1
    );
}

#[test]
fn import_executes_once_and_exposes_value_and_function() {
    let workspace = Workspace::new();
    workspace.write(
        "helper.xiao",
        "value = 7\ndef double(int input) -> int\n    return input * 2\n",
    );
    let outcome = workspace.run("import helper\nfrom helper import value as copy\nfrom helper import double\nanswer = double(helper.value)\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:helper"), 1);
}

#[test]
fn imported_binding_can_be_reassigned_and_read_back() {
    let workspace = Workspace::new();
    workspace.write("helper.xiao", "value = 7\n");
    let outcome = workspace.run(
        "from helper import value\nvalue = 11\nif value != 11\n    raise ArithmeticError(code = \"STALE_IMPORT\")\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:helper"), 1);
}

#[test]
fn nested_local_shadows_import_without_changing_outer_binding() {
    let workspace = Workspace::new();
    workspace.write("helper.xiao", "value = 7\n");
    let outcome = workspace.run(
        "from helper import value\nif true\n    int value = 11\n    if value != 11\n        raise ArithmeticError(code = \"INNER\")\nif value != 7\n    raise ArithmeticError(code = \"OUTER\")\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
}

#[test]
fn imports_in_separate_scopes_keep_their_own_bindings() {
    let workspace = Workspace::new();
    workspace.write("first.xiao", "value = 1\n");
    workspace.write("second.xiao", "value = 2\n");
    let outcome = workspace.run(
        "if true\n    from first import value\n    if value != 1\n        raise ArithmeticError(code = \"FIRST\")\nif true\n    from second import value\n    if value != 2\n        raise ArithmeticError(code = \"SECOND\")\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:first"), 1);
    assert_eq!(loaded(&outcome, "project:second"), 1);
}

#[test]
fn reexported_import_and_aliased_nested_module_are_executable() {
    let workspace = Workspace::new();
    workspace.write("base.xiao", "value = 21\n");
    workspace.write("bridge.xiao", "from base import value\n");
    workspace.write("app/http.xiao", "value = 42\n");
    let outcome = workspace.run(
        "from bridge import value as shared\nimport app.http as http\nif shared + http.value != 63\n    raise ArithmeticError(code = \"REEXPORT\")\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:base"), 1);
    assert_eq!(loaded(&outcome, "project:bridge"), 1);
    assert_eq!(loaded(&outcome, "project:app.http"), 1);
}

#[test]
fn imports_sharing_a_directory_root_are_merged() {
    let workspace = Workspace::new();
    workspace.write("app/http.xiao", "value = 20\n");
    workspace.write("app/models.xiao", "value = 22\n");
    let outcome = workspace.run(
        "import app.http\nimport app.models\nif app.http.value + app.models.value != 42\n    raise ArithmeticError(code = \"SHARED_ROOT\")\n",
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:app.http"), 1);
    assert_eq!(loaded(&outcome, "project:app.models"), 1);
}

#[test]
fn explicit_project_import_overrides_same_named_package_root() {
    let workspace = Workspace::new();
    workspace.write("project/lib.xiao", "answer = 42\n");
    let (layout, environment) = workspace.package_environment("answer = 7\n", None);
    let source =
        "import lib\nif lib.answer != 42\n    raise ArithmeticError(code = \"WRONG_ROOT\")\n";
    let mut context = FrontendContext::host();
    context.project_root = Some(workspace.0.join("project"));
    context.package_registry =
        Some(PackageRegistry::from_layout(&layout, &environment, false).expect("registry"));
    let outcome = run(&DriverRequest::new(
        FrontendRequest::from_text_at(source, workspace.0.join("project/main.xiao"))
            .with_context(context),
    ));
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:lib"), 1);
    assert_eq!(loaded(&outcome, "package:lib.api"), 0);
}

#[test]
fn importing_a_module_with_a_table_export_keeps_it_executable() {
    let workspace = Workspace::new();
    workspace.write(
        "helper.xiao",
        "[State]\n    count = 7\n[[Item]]\n    value = 7\n",
    );
    let outcome = workspace.run("import helper\nif helper.State.count != 7\n    raise ArithmeticError(code = \"SINGLETON\")\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:helper"), 1);
}

#[test]
fn singleton_table_ids_do_not_collide_across_modules() {
    let workspace = Workspace::new();
    workspace.write("helper.xiao", "[State]\n    count = 7\n");
    let outcome = workspace.run("[State]\n    count = 3\nimport helper\nif State.count != 3\n    raise ArithmeticError(code = \"LOCAL_TABLE\")\nif helper.State.count != 7\n    raise ArithmeticError(code = \"IMPORTED_TABLE\")\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:helper"), 1);
}

#[test]
fn module_table_instances_use_their_own_program() {
    let workspace = Workspace::new();
    workspace.write(
        "helper.xiao",
        "[[Item]]\n    value = 7\ndef make() -> int\n    item = new Item()\n    return item.value\n",
    );
    let outcome = workspace.run("import helper\nif helper.make() != 7\n    raise ArithmeticError(code = \"MODULE_TABLE\")\n");
    assert!(outcome.is_success(), "{outcome:?}");
}

#[test]
fn backticked_module_export_keeps_its_binding_key() {
    let workspace = Workspace::new();
    workspace.write("helper.xiao", "`显示名` = 7\n");
    let outcome = workspace.run("from helper import `显示名` as value\nif value != 7\n    raise ArithmeticError(code = \"BACKTICK_EXPORT\")\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:helper"), 1);
}

#[test]
fn imported_function_errors_keep_the_module_source_path() {
    let workspace = Workspace::new();
    workspace.write(
        "helper.xiao",
        "def fail() -> none\n    raise ArithmeticError(code = \"MODULE_FAILURE\")\n",
    );
    let outcome = workspace.run("import helper\nhelper.fail()\n");
    let DriverOutcome::Executed(execution) = outcome else {
        panic!("expected runtime result: {outcome:?}");
    };
    let xiao_vm::RunResult::Error(error) = execution.outcome.result else {
        panic!("expected function failure");
    };
    assert!(
        error.stack().iter().any(|frame| {
            frame.module == "project:helper"
                && frame
                    .source
                    .as_deref()
                    .is_some_and(|source| source.ends_with("helper.xiao"))
        }),
        "{error:?}"
    );
}

#[test]
fn instance_table_export_has_an_explicit_unsupported_diagnostic() {
    let workspace = Workspace::new();
    workspace.write("helper.xiao", "[[Item]]\n    value = 7\n");
    let outcome = workspace.run("import helper\nvalue = helper.Item\n");
    let DriverOutcome::Executed(execution) = outcome else {
        panic!("expected runtime result: {outcome:?}");
    };
    let xiao_vm::RunResult::Error(error) = execution.outcome.result else {
        panic!("expected unsupported table reference");
    };
    assert!(error.message().contains("暂不支持跨模块引用"));
}

#[test]
fn import_inside_module_function_waits_until_the_function_runs() {
    let workspace = Workspace::new();
    workspace.write("util.xiao", "answer = 15\n");
    workspace.write(
        "helper.xiao",
        "def read() -> int\n    import util\n    return util.answer\n",
    );
    let unused = workspace.run("import helper\n");
    assert!(unused.is_success(), "{unused:?}");
    assert_eq!(loaded(&unused, "project:helper"), 1);
    assert_eq!(loaded(&unused, "project:util"), 0);

    let outcome = workspace.run("import helper\nif helper.read() != 15\n    raise ArithmeticError(code = \"FUNCTION_IMPORT\")\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:helper"), 1);
    assert_eq!(loaded(&outcome, "project:util"), 1);
}

#[test]
fn skipped_import_does_not_initialize_module() {
    let workspace = Workspace::new();
    workspace.write(
        "helper.xiao",
        "raise ArithmeticError(code = \"MUST_NOT_RUN\")\n",
    );
    let outcome = workspace.run("if false\n    import helper\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "project:helper"), 0);
}

#[test]
fn package_loads_only_on_first_member_use_and_reuses_module() {
    let workspace = Workspace::new();
    let (layout, environment) = workspace.package_environment(
        "value = 7\ndef double(int input) -> int\n    return input * 2\n",
        None,
    );
    let unused = workspace.run_package(&layout, &environment, "unused = 1\n");
    assert!(unused.is_success(), "{unused:?}");
    assert_eq!(loaded(&unused, "package:lib.api"), 0);
    let outcome = workspace.run_package(&layout, &environment,
        "first = lib.api.value\nsecond = lib.api.double(first)\nif second != 14\n    raise ArithmeticError(code = \"BAD_VALUE\")\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "package:lib.api"), 1);
}

#[test]
fn package_internal_import_initializes_its_own_project_module() {
    let workspace = Workspace::new();
    let (layout, environment) =
        workspace.package_environment("import util\nvalue = util.answer\n", Some("answer = 9\n"));
    let outcome = workspace.run_package(&layout, &environment,
        "first = lib.api.value\nsecond = lib.api.value\nif second != 9\n    raise ArithmeticError(code = \"BAD_VALUE\")\n");
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(loaded(&outcome, "package:lib.api"), 1);
    assert_eq!(loaded(&outcome, "package:lib.util"), 1);
}
