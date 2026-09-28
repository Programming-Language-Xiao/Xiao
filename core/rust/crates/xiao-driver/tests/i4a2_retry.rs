//! VM 内加载失败、诊断来源与单次运行内重试。

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use xiao_bytecode::lower_program;
use xiao_driver::{FrontendCompiler, FrontendContext, FrontendRequest};
use xiao_types::ExternalNamespaces;
use xiao_vm::{CompiledModule, ModuleLoader, RunRequest, RunResult, VmEvent, run_request};

#[derive(Debug)]
struct FlakyLoader {
    attempts: Cell<usize>,
    fail_initialization: bool,
    fail_compilation: bool,
}

impl ModuleLoader for FlakyLoader {
    fn contains(&self, identity: &str) -> bool {
        identity == "package:lib.api"
    }

    fn compile(&self, identity: &str) -> Result<Option<CompiledModule>, String> {
        assert_eq!(identity, "package:lib.api");
        let attempts = self.attempts.get() + 1;
        self.attempts.set(attempts);
        if attempts == 1 && self.fail_compilation {
            return Err("源文件暂不可读".to_owned());
        }
        let mut context = FrontendContext::host();
        context.module_exports = vec!["value".to_owned()];
        let source = if attempts == 1 && self.fail_initialization {
            "raise ArithmeticError(code = \"BROKEN\")\n"
        } else {
            "value = 9\n"
        };
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source).with_context(context))
            .expect("module frontend");
        Ok(Some(CompiledModule {
            program: lower_program(&artifact.ir),
            ir: artifact.ir,
            source_name: "/isolated/cache/api.xiao".to_owned(),
        }))
    }

    fn environment(&self, _identity: &str) -> Option<&str> {
        Some("/isolated/active")
    }
}

fn execute(source: &str, loader: &FlakyLoader) -> xiao_vm::RunOutcome {
    let mut context = FrontendContext::host();
    context.package_namespaces = Some(ExternalNamespaces {
        members: BTreeMap::from([
            ("lib".to_owned(), BTreeSet::from(["api".to_owned()])),
            ("lib.api".to_owned(), BTreeSet::from(["value".to_owned()])),
        ]),
    });
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source).with_context(context))
        .expect("main frontend");
    let program = lower_program(&artifact.ir);
    run_request(&RunRequest::new(&artifact.ir, &program).with_module_loader(loader))
}

#[test]
fn successful_module_is_compiled_and_initialized_only_once_per_run() {
    let loader = FlakyLoader {
        attempts: Cell::new(0),
        fail_initialization: false,
        fail_compilation: false,
    };
    let outcome = execute("first = lib.api.value\nsecond = lib.api.value\n", &loader);
    assert!(matches!(outcome.result, RunResult::Success), "{outcome:?}");
    assert_eq!(loader.attempts.get(), 1);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event, VmEvent::ModuleLoaded { module } if module == "package:lib.api"))
            .count(),
        1
    );
}

#[test]
fn failed_compilation_is_not_cached_and_later_reference_succeeds() {
    let loader = FlakyLoader {
        attempts: Cell::new(0),
        fail_initialization: false,
        fail_compilation: true,
    };
    let outcome = execute(
        "try\n    first = lib.api\ncatch error as Error\n    recovered = true\nsecond = lib.api.value\n",
        &loader,
    );
    assert!(matches!(outcome.result, RunResult::Success), "{outcome:?}");
    assert_eq!(loader.attempts.get(), 2);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|event| matches!(event,
        VmEvent::ModuleLoaded { module } if module == "package:lib.api"))
            .count(),
        1
    );
}

#[test]
fn failed_initialization_is_not_cached() {
    let loader = FlakyLoader {
        attempts: Cell::new(0),
        fail_initialization: true,
        fail_compilation: false,
    };
    let outcome = execute(
        "try\n    first = lib.api\ncatch error as Error\n    recovered = true\nsecond = lib.api.value\n",
        &loader,
    );
    assert!(matches!(outcome.result, RunResult::Success), "{outcome:?}");
    assert_eq!(loader.attempts.get(), 2);
}

#[test]
fn package_failure_reports_package_environment_and_cause() {
    let loader = FlakyLoader {
        attempts: Cell::new(0),
        fail_initialization: false,
        fail_compilation: true,
    };
    let outcome = execute("first = lib.api\n", &loader);
    let RunResult::Error(error) = outcome.result else {
        panic!("expected recoverable failure: {outcome:?}");
    };
    assert!(error.message().contains("lib"));
    assert!(error.message().contains("/isolated/active"));
    assert!(error.message().contains("源文件暂不可读"));
    assert!(error.params().contains_key("package"));
    assert!(error.params().contains_key("environment"));
}
