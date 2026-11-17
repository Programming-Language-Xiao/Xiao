//! B4-4真实模块的原生差分；只使用调用方指定的本任务Runtime与LLVM工具。
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use xiao_codegen_llvm::{
    ArtifactVerification, CodegenOptions, ObjectFormat, TargetDescription, Toolchain,
};
use xiao_driver::{FrontendCompiler, FrontendNativeDriver, FrontendRequest, NativeBuildRequest};

/// 用两条固定官方选择导入路径实际构建O0/O3，逐字核对共享黄金及错误恢复。
pub fn run() {
    let required =
        |key| PathBuf::from(std::env::var_os(key).unwrap_or_else(|| panic!("缺少{key}")));
    let target = TargetDescription::host();
    assert_eq!(std::env::var("XIAO_TARGET_TRIPLE").unwrap(), target.triple);
    let runtime = required("XIAO_RUNTIME_LIBRARY");
    assert!(runtime.is_file());
    let tools = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(&runtime)
        .probe_native_static_libraries("rustc", &target)
        .unwrap();
    let root = super::workspace();
    for prefix in ["std", "stdlib"] {
        let (mut source, golden) = super::fixture(prefix);
        // B1已登记原生模块值/限定调用后置；同一语料改用已支持的选择导入别名，
        // 仍实际执行全部15项的位置/命名调用。VM/xiaoc/xar保留原始模块别名语料。
        source = source.replace(&format!("import {prefix}.cmath as math\n"), "");
        for (index, &(primitive, _, _)) in xiao_intrinsics::CMATH_DECLARATIONS.iter().enumerate() {
            let name = primitive.strip_prefix("cmath_").unwrap();
            source = source.replace(&format!("math.{name}("), &format!("f{index}("));
        }
        assert!(!source.contains("math."));
        let request = FrontendRequest::from_text(source);
        let artifact = FrontendCompiler::new().compile(&request).unwrap();
        for level in [0, 3] {
            let path = root.join(format!("{prefix}-o{level}{}", std::env::consts::EXE_SUFFIX));
            let options = CodegenOptions::for_target(target.clone())
                .with_optimization_level(level)
                .unwrap();
            let build =
                NativeBuildRequest::new(request.clone(), target.clone(), tools.clone(), &path)
                    .with_codegen_options(options);
            let result = FrontendNativeDriver::new()
                .build_artifact(&artifact, &build)
                .unwrap();
            assert!(result.native.module.uses_runtime);
            assert!(
                result
                    .native
                    .module
                    .runtime_components
                    .iter()
                    .any(|component| component == "containers"),
                "{prefix}.cmath O{level} missing emitted container ABI: {:?}",
                result.native.module.runtime_components
            );
            match result.native.artifact_runtime.verification {
                ArtifactVerification::Complete => assert_eq!(
                    result.native.artifact_runtime.declared_components,
                    result.native.artifact_runtime.observed_components
                ),
                ArtifactVerification::UnverifiedCoffExports => {
                    assert_eq!(
                        result.native.artifact_runtime.object_format,
                        ObjectFormat::Coff
                    );
                }
            }
            fs::write(path.with_extension("ll"), &result.native.module.text).unwrap();
            execute(&path, &golden);
            println!(
                "B4-4-NATIVE path={prefix}.cmath optimization={level} executable={} verification={:?} declared={:?} observed={:?} stdout=golden errors=009 result=passed",
                path.display(),
                result.native.artifact_runtime.verification,
                result.native.artifact_runtime.declared_components,
                result.native.artifact_runtime.observed_components
            );
        }
    }
}

fn execute(path: &Path, golden: &str) {
    let mut child = Command::new(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let read = |mut pipe: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).unwrap();
            bytes
        })
    };
    let out = read(Box::new(child.stdout.take().unwrap()));
    let err = read(Box::new(child.stderr.take().unwrap()));
    let begin = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if begin.elapsed() > Duration::from_secs(15) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("B4-4原生回归超时");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = out.join().unwrap();
    let stderr = err.join().unwrap();
    assert!(
        status.success(),
        "{path:?}: {status:?} {}",
        String::from_utf8_lossy(&stderr)
    );
    assert!(stderr.is_empty(), "{}", String::from_utf8_lossy(&stderr));
    assert_eq!(
        String::from_utf8(stdout).unwrap().replace("\r\n", "\n"),
        golden
    );
}
