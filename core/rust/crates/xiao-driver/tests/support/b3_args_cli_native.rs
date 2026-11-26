//! 使用指定 Runtime 与工具链复跑 B3-2 原生验收。

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use xiao_codegen_llvm::{
    ArtifactVerification, CodegenOptions, NativeRuntimeSupport, ObjectFormat, TargetDescription,
    Toolchain,
};
use xiao_driver::{FrontendNativeDriver, NativeBuildRequest};

use super::*;

/// 限制测试程序的运行时间，保留它自己的原始 stdout 与 stderr。
fn run(executable: &Path, argv: &[OsString]) -> Output {
    let mut child = Command::new(executable)
        .args(argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!(
                "{} 超时：{:?}",
                executable.display(),
                child.wait_with_output()
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    child.wait_with_output().unwrap()
}

fn assert_output(output: Output, expected: &[u8]) {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        std::str::from_utf8(expected).unwrap()
    );
}

/// 宿主参数无法无损构造为 Unicode 的单项。
fn invalid_os_argument() -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(vec![0xff])
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        OsString::from_wide(&[0xd800])
    }
}

#[test]
#[ignore = "需要独立任务根、发布版 Runtime 和指定 LLVM 工具链"]
fn args_cli_matches_native_goldens_and_boundaries() {
    let required = |key: &str| PathBuf::from(std::env::var_os(key).expect(key));
    let target = TargetDescription::host();
    assert_eq!(std::env::var("XIAO_TARGET_TRIPLE").unwrap(), target.triple);
    let toolchain = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries("rustc", &target)
        .unwrap()
        .probe_versions()
        .unwrap();
    let root = std::env::temp_dir().join(format!("xiao-b3-2-native-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let previous = required("XIAO_B3_1_NATIVE_ARTIFACT");
    let previous = xiao_codegen_llvm::read_native_metadata(&previous, &target)
        .unwrap()
        .unwrap();
    assert_eq!(previous.generator_version, 23);
    previous.validate(&target, &Default::default(), 0).unwrap();
    let previous_fingerprint = toolchain.fingerprint(&target, 23);
    let current_fingerprint = toolchain.fingerprint(&target, xiao_codegen_llvm::CODEGEN_VERSION);
    assert_ne!(previous_fingerprint, current_fingerprint);
    println!(
        "B3-2-GENERATOR {}",
        serde_json::json!({"baseline": 23, "current": xiao_codegen_llvm::CODEGEN_VERSION, "older_metadata": "accepted",
            "baseline_fingerprint": previous_fingerprint.to_string(),
            "current_fingerprint": current_fingerprint.to_string()})
    );
    let build = |case: &str, source: String, level, capabilities| {
        let executable = root.join(format!("{case}{}", std::env::consts::EXE_SUFFIX));
        let options = CodegenOptions::for_target(target.clone())
            .with_optimization_level(level)
            .unwrap()
            .with_capabilities(capabilities)
            .unwrap();
        let result = FrontendNativeDriver::new()
            .build(
                &NativeBuildRequest::new(
                    FrontendRequest::from_text(source),
                    target.clone(),
                    toolchain.clone(),
                    &executable,
                )
                .with_codegen_options(options),
            )
            .unwrap_or_else(|error| panic!("{case} O{level}：{error:?}"));
        let composition = &result.native.artifact_runtime;
        assert_eq!(composition.verification, ArtifactVerification::Complete);
        assert_eq!(
            composition.declared_components,
            composition.observed_components
        );
        assert!(composition.unclassified_runtime_symbols.is_empty());
        let metadata = result.native.native_metadata.unwrap();
        assert_eq!(
            metadata.generator_version,
            xiao_codegen_llvm::CODEGEN_VERSION
        );
        assert_eq!(
            metadata.runtime_abi_min,
            xiao_runtime_abi::ABI_ENCODED_VERSION
        );
        assert_eq!(metadata.runtime_abi_max, metadata.runtime_abi_min);
        assert_eq!(metadata.stdlib.stdlib_interface_version, 1);
        let previous = NativeRuntimeSupport {
            generator_version: 23,
            ..Default::default()
        };
        assert_eq!(
            metadata.validate(&target, &previous, 0).unwrap_err().code,
            "X17-XAR-007"
        );
        let coff = composition.coff_link_identity.as_ref();
        let digest =
            |path: &Path| xiao_artifacts::Digest256::of_bytes(&fs::read(path).unwrap()).as_hex();
        let artifact_sha256 = digest(&executable);
        if target.object_format == ObjectFormat::Coff {
            let binding = coff.expect("MSVC 原生构建必需映像绑定的链接 map");
            assert_eq!(binding.artifact_sha256, artifact_sha256);
            assert_eq!(binding.map_sha256, digest(&binding.map_path));
        }
        println!(
            "B3-2-ARTIFACT {}",
            serde_json::json!({"case": case, "target": target.triple, "optimization": level,
                "path": executable, "sha256": artifact_sha256,
                "coff_map": coff.map(|binding| &binding.map_path),
                "coff_map_sha256": coff.map(|binding| &binding.map_sha256),
                "components": composition.declared_components, "observed": composition.observed_components,
                "unclassified": composition.unclassified_runtime_symbols, "verification": "complete",
                "generator": metadata.generator_version, "old_verifier": "X17-XAR-007"})
        );
        executable
    };
    let input: RunArguments = serde_json::from_str(ARGS_INPUT).unwrap();
    let argv: Vec<OsString> = input.argv.into_iter().map(OsString::from).collect();
    let capabilities = CAPABILITY_PROCESS | CAPABILITY_STDOUT;
    let mut native_args = None;
    for alias in ["std", "stdlib"] {
        for level in [0, 3] {
            let source = |text: &str| text.replacen("from std.", &format!("from {alias}."), 1);
            let args = build(
                &format!("args-{alias}-o{level}"),
                source(ARGS_SOURCE),
                level,
                capabilities,
            );
            let expected = std::str::from_utf8(ARGS_GOLDEN).unwrap().replacen(
                &input.program_name,
                args.to_str().unwrap(),
                1,
            );
            assert_output(run(&args, &argv), expected.as_bytes());
            native_args.get_or_insert(args);
            let cli = build(
                &format!("cli-{alias}-o{level}"),
                source(CLI_SOURCE),
                level,
                CAPABILITY_STDOUT,
            );
            assert_output(run(&cli, &[]), CLI_GOLDEN);
        }
    }

    let dynamic_cli = build(
        "cli-types",
        cli_dynamic_types("from std.cli import split_terminator"),
        0,
        CAPABILITY_STDOUT,
    );
    assert_output(
        run(&dynamic_cli, &[]),
        b"X06-RUNTIME-002\nX06-RUNTIME-002\nok\n",
    );
    let limited_cli = build("cli-limits", cli_limits_source(), 3, CAPABILITY_STDOUT);
    assert_output(run(&limited_cli, &[]), CLI_LIMITS_GOLDEN.as_bytes());
    let denied = build(
        "args-denied",
        r#"from stdlib.args import program_name, argv
try
    program_name()
catch failure as ResourceError
    print(error_code(failure))
try
    argv()
catch failure as ResourceError
    print(error_code(failure))
"#
        .to_owned(),
        0,
        CAPABILITY_STDOUT,
    );
    assert_output(run(&denied, &argv), b"X06-RUNTIME-026\nX06-RUNTIME-026\n");

    let children = build(
        "args-children",
        r#"from std.args import program_name, argv
from std.concurrent import current_runtime, spawn, join, pool_create, submit, wait, shutdown
def child() -> str
    copy = argv()
    result = program_name() + "[" + copy[0] + "]"
    array_push(copy, "local")
    return result
root = current_runtime()
print(join(spawn(root, child)))
pool = pool_create(root, 1)
print(wait(submit(pool, child)))
print(wait(submit(pool, child)))
shutdown(pool)
print(child())
"#
        .to_owned(),
        3,
        capabilities,
    );
    let line = format!("{}[\u{fffd} \"quote\" -- ]\n", children.display());
    assert_output(
        run(&children, &["\u{fffd} \"quote\" -- ".into()]),
        line.repeat(4).as_bytes(),
    );

    let native_args = native_args.unwrap();
    let invalid = invalid_os_argument();
    let long: OsString = "a".repeat(4097).into();
    let mut too_many = vec![OsString::new(); 1025];
    too_many[0] = invalid.clone();
    let too_many_bytes = vec![OsString::from("\u{800}".repeat(1365)); 17];
    for (label, arguments, code) in [
        ("invalid", vec![invalid.clone()], "X09-DRIVER-004"),
        ("single", vec![long, invalid], "X09-DRIVER-005"),
        ("count", too_many, "X09-DRIVER-005"),
        ("total", too_many_bytes, "X09-DRIVER-005"),
    ] {
        let result = run(&native_args, &arguments);
        assert_eq!(result.status.code(), Some(2), "{label}: {result:?}");
        assert!(result.stdout.is_empty(), "{label}: {result:?}");
        let diagnostic = String::from_utf8(result.stderr).unwrap();
        assert!(diagnostic.starts_with(code), "{label}: {diagnostic}");
        println!("B3-2-NATIVE-REJECTION case={label} exit=2 code={code}");
    }
}
