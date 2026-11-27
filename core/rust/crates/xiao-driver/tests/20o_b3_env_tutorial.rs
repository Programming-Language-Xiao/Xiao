//! B3-4 的冻结 env 教程经生产 VM 与原生入口执行。

use std::fs;
use std::process::Command;

use xiao_codegen_llvm::{CodegenOptions, TargetDescription};
use xiao_driver::{
    DriverRequest, FrontendNativeDriver, FrontendRequest, FrontendVmDriver, NativeBuildRequest,
    RunEnvironment, RunOptions,
};
use xiao_intrinsics::{CAPABILITY_ENVIRONMENT, CAPABILITY_STDOUT};
use xiao_vm::VmEvent;

#[path = "support/b3_env_artifact.rs"]
mod native_artifact;

const SOURCE: &str = include_str!("../../../../../tests/fixtures/docs/env/tutorial.xiao");
const GOLDEN: &[u8] = include_bytes!("../../../../../tests/fixtures/docs/env/tutorial.stdout");
const INPUT: &str = include_str!("../../../../../tests/fixtures/docs/env/tutorial.input.json");
const CAPABILITIES: u64 = CAPABILITY_ENVIRONMENT | CAPABILITY_STDOUT;

#[derive(serde::Deserialize)]
struct TutorialInput {
    environment: RunEnvironment,
}

fn input() -> RunEnvironment {
    serde_json::from_str::<TutorialInput>(INPUT)
        .unwrap()
        .environment
}

fn sources() -> [(&'static str, String); 2] {
    [
        ("std", SOURCE.to_owned()),
        (
            "stdlib",
            SOURCE.replacen("from std.env import ", "from stdlib.env import ", 1),
        ),
    ]
}

#[test]
fn env_tutorial_matches_vm_golden() {
    let options = RunOptions {
        environment: Some(input()),
        capabilities: CAPABILITIES,
        ..RunOptions::default()
    };
    assert!(GOLDEN.ends_with(b"\n"));
    for (alias, source) in sources() {
        let request = DriverRequest::new(FrontendRequest::from_text(&source))
            .with_capabilities(options.capabilities)
            .unwrap();
        let snapshot = options.environment_snapshot().unwrap();
        let result = FrontendVmDriver::new().run_with_environment(&request, snapshot);
        let exit_code = result.exit_code().as_process_code();
        assert_eq!(exit_code, 0, "{alias}: {result:?}");
        let events = result.as_executed().unwrap().events();
        assert!(events.iter().any(|event| matches!(
            event,
            VmEvent::ModuleLoaded { module } if module == "stdlib.env"
        )));
        let stdout = events
            .iter()
            .filter_map(|event| match event {
                VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(stdout.as_bytes(), GOLDEN, "{alias}");
        println!("env VM {alias}.env → stdlib.env，退出 {exit_code}，{stdout:?}");
    }
}

#[test]
#[ignore = "需要固定目标、发布 Runtime 和 LLVM 工具链"]
fn env_tutorial_matches_native_golden() {
    let target = TargetDescription::host();
    let tools = native_artifact::toolchain(&target);
    let root = native_artifact::test_dir();
    let driver = FrontendNativeDriver::new();
    let environment = input();
    let payload = serde_json::to_vec(&environment).unwrap();
    assert!(GOLDEN.ends_with(b"\n"));

    for (alias, source) in sources() {
        for optimization in [0, 3] {
            let case = format!("env-{alias}-o{optimization}");
            let task = root.join(&case);
            fs::create_dir(&task).unwrap();
            let executable = task.join(format!("{case}{}", std::env::consts::EXE_SUFFIX));
            let request = NativeBuildRequest::new(
                FrontendRequest::from_text(&source),
                target.clone(),
                tools.clone(),
                &executable,
            )
            .with_codegen_options(
                CodegenOptions::for_target(target.clone())
                    .with_optimization_level(optimization)
                    .unwrap()
                    .with_capabilities(CAPABILITIES)
                    .unwrap(),
            );
            let result = driver
                .build(&request)
                .unwrap_or_else(|error| panic!("{case}: {error}"));
            assert!(
                result
                    .frontend
                    .ir()
                    .modules
                    .iter()
                    .any(|module| module.name == "stdlib.env"),
                "{case}"
            );
            let mut command = Command::new(&result.native.executable);
            command.current_dir(&task).env_clear();
            for (name, _) in &environment.0 {
                command.env(name, "synthetic-host");
            }
            let output = native_artifact::run_injected(&mut command, &payload);
            fs::write(task.join(format!("{case}.stdout")), &output.stdout).unwrap();
            fs::write(task.join(format!("{case}.stderr")), &output.stderr).unwrap();
            native_artifact::record(&case, &source, &request, &result.native, &output);
            assert_eq!(output.status.code(), Some(0), "{case}: {output:?}");
            assert_eq!(output.stdout, GOLDEN, "{case}");
            assert!(output.stderr.is_empty(), "{case}: {output:?}");
        }
    }
}
