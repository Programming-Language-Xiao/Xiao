//! 在两个源码版本上以同一工具、Runtime、目标和 IR 比较 env 降低。

use std::{env, fs, path::PathBuf, process::ExitCode};

use xiao_codegen_llvm::{
    BuildRequest, CODEGEN_VERSION, CodegenOptions, NativeBuild, NativeRuntimeSupport,
    TargetDescription, Toolchain, validate_native_metadata_file,
};
use xiao_driver::{FrontendCompiler, FrontendNativeDriver, FrontendRequest, NativeBuildRequest};
use xiao_intrinsics::{CAPABILITY_ENVIRONMENT, CAPABILITY_STDOUT};
use xiao_ir::{IrArrayShape, IrExpressionKind, IrStatementKind, IrType};

fn main() -> ExitCode {
    let mut argv = env::args_os().skip(1);
    let mode = argv.next().unwrap();
    let path = PathBuf::from(argv.next().unwrap());
    let target = TargetDescription::host();
    assert_eq!(target.triple, env::var("XIAO_TARGET_TRIPLE").unwrap());
    if mode == "verify" {
        let support = NativeRuntimeSupport::default();
        let result = validate_native_metadata_file(&path, &target, &support, 0, true);
        println!("校验器上限 {CODEGEN_VERSION}，产物 {}", path.display());
        return match result {
            Ok(Some(metadata)) => {
                println!("接纳生成器 {}", metadata.generator_version);
                ExitCode::SUCCESS
            }
            Err(error) => {
                println!("拒绝 {error}");
                ExitCode::from(2)
            }
            Ok(None) => unreachable!("需要原生元数据"),
        };
    }

    let required = |name: &str| PathBuf::from(env::var_os(name).expect(name));
    let toolchain = Toolchain::new(required("XIAO_CLANG"))
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_llc(required("XIAO_LLC"))
        .with_runtime_library(required("XIAO_RUNTIME_LIBRARY"))
        .probe_native_static_libraries("rustc", &target)
        .unwrap()
        .probe_versions()
        .unwrap();
    println!("Runtime {}", required("XIAO_RUNTIME_LIBRARY").display());
    println!(
        "目标 {}，生成器 {CODEGEN_VERSION}，工具指纹 {}",
        target.triple,
        toolchain.fingerprint(&target, CODEGEN_VERSION)
    );

    let frontend = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "print(\"XIAO_B3_TEST_ENV\")\nprint(\"XIAO_B3_TEST_ENV\", \"probe\")\nprint(\"XIAO_B3_TEST_ENV\")\n",
        ))
        .unwrap();
    let mut program = frontend.ir;
    let string = IrType::Scalar { name: "str".into() };
    let array = IrType::Array {
        shape: IrArrayShape::Homogeneous {
            element: Box::new(string),
            length: None,
        },
    };
    let boolean = IrType::Scalar {
        name: "bool".into(),
    };
    for (statement, (env_id, result)) in
        program
            .body
            .iter_mut()
            .zip([(60, array), (61, IrType::None), (62, boolean)])
    {
        let IrStatementKind::Expression { value } = &mut statement.kind else {
            panic!("探针语句须包含表达式");
        };
        let IrExpressionKind::IntrinsicCall { id, .. } = &mut value.kind else {
            panic!("探针须包含 intrinsic");
        };
        *id = env_id;
        value.ty = result;
    }
    let builder = NativeBuild::new();
    let mut unsupported = false;
    for level in [0, 3] {
        let opts = CodegenOptions::for_target(target.clone())
            .with_optimization_level(level)
            .unwrap();
        let control = NativeBuildRequest::new(
            FrontendRequest::from_text(
                "from std.args import program_name\nprint(program_name())\n",
            ),
            target.clone(),
            toolchain.clone(),
            path.join(format!("control-o{level}{}", env::consts::EXE_SUFFIX)),
        )
        .with_codegen_options(opts.clone())
        .with_llvm_ir_output(path.join(format!("control-o{level}.ll")));
        let control = FrontendNativeDriver::new().build(&control).unwrap().native;
        println!(
            "O{level} 对照生成指纹 {}",
            control.module.codegen_fingerprint
        );
        let opts = opts
            .with_capabilities(CAPABILITY_ENVIRONMENT | CAPABILITY_STDOUT)
            .unwrap();
        match builder.lower(&program, &opts) {
            Ok(module) => {
                println!(
                    "O{level} 环境生成指纹 {}，组件 {:?}",
                    module.codegen_fingerprint, module.runtime_components
                );
                fs::write(path.join(format!("env-o{level}.ll")), &module.text).unwrap();
                let request = BuildRequest::new(
                    program.clone(),
                    target.clone(),
                    toolchain.clone(),
                    path.join(format!("env-o{level}{}", env::consts::EXE_SUFFIX)),
                )
                .with_options(opts);
                let artifact = builder.build(&request).unwrap();
                println!(
                    "O{level} env 产物级核对 {:?}",
                    artifact.artifact_runtime.verification
                );
            }
            Err(error) => {
                unsupported = true;
                println!("O{level} 环境降低拒绝 {error}");
            }
        }
    }
    if unsupported {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
