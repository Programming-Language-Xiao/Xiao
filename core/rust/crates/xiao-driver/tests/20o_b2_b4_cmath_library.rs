//! B4-4：真实官方模块的双路径、归档、签名/移除与原生O0/O3。
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use xiao_driver::{
    ArchiveBuildRequest, CancellationToken, DriverRequest, FrontendCompiler, FrontendRequest,
    FrontendVmDriver, ProtocolRequest, ProtocolResponse, RunOptions, build_archive, dispatch,
};
use xiao_vm::VmEvent;

#[path = "support/b4_cmath_library_native.rs"]
mod native;

/// 从测试crate定位共享夹具；不依赖当前工作目录。
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .join("tests/fixtures/docs/numeric")
}

/// 双路径使用同一源码/独立黄金，不从Runtime实现生成期望值。
fn fixture(prefix: &str) -> (String, String) {
    let root = fixture_root();
    (
        fs::read_to_string(root.join("cmath-library.xiao"))
            .unwrap()
            .replace("std.cmath", &format!("{prefix}.cmath")),
        fs::read_to_string(root.join("cmath-library.stdout"))
            .unwrap()
            .replace("\r\n", "\n"),
    )
}

/// 完整教程及其独立黄金原样进入已有精确入口。
fn tutorial_fixture() -> (String, String) {
    let root = fixture_root();
    (
        fs::read_to_string(root.join("cmath-tutorial.xiao")).unwrap(),
        fs::read_to_string(root.join("cmath-tutorial.stdout"))
            .unwrap()
            .replace("\r\n", "\n"),
    )
}

/// 每个测试独立创建临时目录，防止旧可执行文件或归档复用。
fn workspace() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "b4-4-library-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    path
}

fn output(events: &[VmEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// 核对真实协议运行响应，不仅验证编码或索引。
fn protocol_output(response: ProtocolResponse) -> String {
    let ProtocolResponse::Result {
        exit_code, events, ..
    } = response
    else {
        panic!("{response:?}")
    };
    assert_eq!(exit_code, 0, "{events:?}");
    events
        .iter()
        .filter(|event| event.kind == "intrinsic_output")
        .filter_map(|event| event.data.get("text").and_then(serde_json::Value::as_str))
        .collect()
}

#[test]
/// 15项包装的位置/逆序命名、模块别名、同名导出、错误清理经源码、xiaoc和xar执行。
fn aliases_replay_from_xiaoc_and_xar_after_application_source_removal() {
    for prefix in ["std", "stdlib"] {
        let (source, golden) = fixture(prefix);
        replay(prefix, &source, &golden, true);
    }
    let (source, golden) = tutorial_fixture();
    // 选择导入由宿主加载官方模块，应用字节码通过官方模块身份调用包装。
    replay("tutorial", &source, &golden, false);
    println!("B2-C-TUTORIAL vm,xiaoc,xar=passed source_removed=true");
}

/// 真前端编码保留直接34～48调用，同时归档仅包含应用模块、官方代码从宿主加载。
fn replay(prefix: &str, source: &str, golden: &str, check_ids: bool) {
    let root = workspace();
    let input = root.join("main.xiao");
    fs::write(&input, source).unwrap();
    let request = FrontendRequest::from_file(&input).unwrap();
    let artifact = FrontendCompiler::new().compile(&request).unwrap();
    let outcome = FrontendVmDriver::new().run_artifact(
        &artifact,
        &DriverRequest::new(request)
            .with_event_capacity(8192)
            .with_timeout(Duration::from_secs(15)),
    );
    assert!(outcome.is_success(), "{outcome:?}");
    assert_eq!(output(outcome.as_executed().unwrap().events()), golden);
    let tac = xiao_bytecode::lower_program(&artifact.ir);
    let bytes = xiao_bytecode::encode_xiaoc(&tac, Default::default()).unwrap();
    let file = xiao_bytecode::decode_xiaoc(&bytes).unwrap();
    if check_ids {
        let ids: std::collections::BTreeSet<_> = file
            .program
            .signatures
            .iter()
            .filter_map(|s| s.intrinsic_id())
            .filter(|id| (34..=48).contains(id))
            .collect();
        assert_eq!(ids, (34..=48).collect());
    }
    let xiaoc = root.join("main.xiaoc");
    fs::write(&xiaoc, &bytes).unwrap();
    let mut build = ArchiveBuildRequest::new(&input);
    build.cache_root = Some(root.join("cache"));
    build.output = Some(root.join("main.xar"));
    let archive = build_archive(&build, &CancellationToken::new()).unwrap();
    assert!(
        archive
            .index
            .stdlib_modules
            .contains(&"stdlib.cmath".to_owned())
    );
    assert!(
        archive
            .index
            .stdlib_modules
            .iter()
            .all(|name| !name.starts_with("std."))
    );
    assert!(
        archive
            .index
            .entries
            .iter()
            .all(|entry| !entry.module.starts_with("stdlib.") && !entry.module.starts_with("std."))
    );
    assert_eq!(archive.index.stdlib_interface_version, 1);
    let raw = fs::read(&archive.output).unwrap();
    let container = xiao_xar::decode_xar(&raw).unwrap();
    assert!(xiao_bytecode::decode_xiaoc(&container.read_entry().unwrap()).is_ok());
    fs::remove_file(input).unwrap();
    let options = RunOptions {
        timeout_ms: Some(15_000),
        ..Default::default()
    };
    let response = dispatch(ProtocolRequest::RunXiaoc {
        request_id: "b4-4-xiaoc".to_owned(),
        protocol_version: xiao_driver::PROTOCOL_VERSION,
        core_version: xiao_driver::CORE_VERSION,
        locale: None,
        path: xiaoc.display().to_string(),
        options: options.clone(),
        diagnostics: None,
        debug: false,
    });
    assert_eq!(protocol_output(response), golden);
    let response = dispatch(ProtocolRequest::RunArchive {
        request_id: "b4-4-xar".to_owned(),
        protocol_version: xiao_driver::PROTOCOL_VERSION,
        core_version: xiao_driver::CORE_VERSION,
        locale: None,
        path: archive.output.display().to_string(),
        options,
        diagnostics: None,
        debug: false,
    });
    assert_eq!(protocol_output(response), golden);
    let result = xiao_driver::run_archive(&raw, Default::default()).unwrap();
    assert!(result.result.is_success(), "{result:?}");
    assert_eq!(output(&result.events), golden);
    println!(
        "B4-4-REPLAY path={prefix}.cmath source,xiaoc,xar=passed source_removed=true official_bundled=false ids_checked={check_ids}"
    );
}

#[test]
/// 每个公开入口的参数类型、缺参、额外参数和未知命名仍由真实前端拒绝。
fn imported_signatures_reject_invalid_types_and_argument_shapes() {
    for prefix in ["std", "stdlib"] {
        for (index, &(primitive, _, _)) in xiao_intrinsics::CMATH_DECLARATIONS.iter().enumerate() {
            let name = primitive.strip_prefix("cmath_").unwrap();
            let parameters = xiao_intrinsics::cmath_parameters(
                xiao_intrinsics::IntrinsicId::new(34 + index as u32).unwrap(),
            )
            .unwrap();
            for bad in [
                "1",
                "sint(1)",
                "lint(1)",
                "lfloat(1)",
                "sfloat(1.0)",
                "true",
                "\"bad\"",
                "[1]",
            ] {
                for position in 0..parameters.len() {
                    let mut args = vec!["1.0"; parameters.len()];
                    args[position] = bad;
                    reject(&format!(
                        "from {prefix}.cmath import {name}\n{name}({})\n",
                        args.join(",")
                    ));
                }
            }
            for args in [
                "".to_owned(),
                "1.0,1.0,1.0".to_owned(),
                format!(
                    "typo=1.0{}",
                    if parameters.len() == 2 {
                        ",value=1.0"
                    } else {
                        ""
                    }
                ),
            ] {
                reject(&format!(
                    "from {prefix}.cmath import {name}\n{name}({args})\n"
                ));
            }
        }
    }
    println!(
        "B4-4-SIGNATURES functions=15 paths=2 invalid_types=8 each_parameter=checked shapes=checked result=passed"
    );
}

fn reject(source: &str) {
    let error = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap_err();
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.is_error() && d.code() == "X04-TYPE-002"),
        "{source}: {error:?}"
    );
}

#[test]
/// 逐真实模块函数体移除后LLVM导入失败；不修改产品文件。
fn native_module_removal_cannot_reuse_preloaded_function_bindings() {
    use xiao_ir::IrStatementKind;
    for prefix in ["std", "stdlib"] {
        for &(primitive, _, _) in &xiao_intrinsics::CMATH_DECLARATIONS {
            let name = primitive.strip_prefix("cmath_").unwrap();
            let source = format!("from {prefix}.cmath import {name}\nprint(7)\n");
            let mut ir = FrontendCompiler::new()
                .compile(&FrontendRequest::from_text(source))
                .unwrap()
                .ir;
            let module = ir
                .modules
                .iter_mut()
                .find(|m| m.name == "stdlib.cmath")
                .unwrap();
            let original = module.body.len();
            module.body.retain(|statement|!matches!(&statement.kind,IrStatementKind::Function {name:entry,..} if entry.text==name));
            assert_eq!(module.body.len() + 1, original);
            let error = xiao_codegen_llvm::lower_program(&ir, &Default::default()).unwrap_err();
            let xiao_codegen_llvm::CodegenError::Unsupported { feature, span } = error else {
                panic!("{error:?}")
            };
            assert_eq!(feature, format!("官方函数 stdlib.cmath.{name} 未静态链接"));
            assert!(span.is_some());
        }
    }
    println!(
        "B4-4-LLVM-REMOVAL functions=15 paths=2 source_body=removed fallback=rejected result=passed"
    );
}

#[test]
/// 模块函数抛出009不退化为加载错误；catch/release仍沿已冻结通道。
fn imported_domain_and_overflow_errors_preserve_numeric_identity() {
    for prefix in ["std", "stdlib"] {
        for expression in [
            "sqrt(-1.0)",
            "ln(0.0)",
            "log10(-1.0)",
            "pow(-2.0,0.5)",
            "pow(0.0,-1.0)",
            "exp(1000.0)",
        ] {
            let (name, args) = expression.split_once('(').unwrap();
            let source = format!("from {prefix}.cmath import {name}\n{name}({args}\n");
            let request = FrontendRequest::from_text(&source);
            let outcome = xiao_driver::run(&DriverRequest::new(request));
            assert_eq!(outcome.code(), Some("X06-RUNTIME-009"), "{outcome:?}");
            assert!(outcome.report().unwrap().location.is_some());
        }
    }
}

#[test]
/// 子VM经float的既有XCON复制保留负零和数值，归档也使用相同闭包身份。
fn child_float_values_and_text_survive_portable_replay() {
    for prefix in ["std", "stdlib"] {
        let source = format!(
            "from std.concurrent import current_runtime, spawn, join\ndef work(float value) -> float\n    from {prefix}.cmath import sqrt\n    return sqrt(value)\nrt=current_runtime()\na=spawn(rt,work,-0.0)\nb=spawn(rt,work,4.0)\nprint(join(a))\nprint(join(b))\n"
        );
        replay(prefix, &source, "-0\n2\n", false);
    }
    use xiao_runtime::{XconValue, decode_xcon, encode_xcon};
    for value in [
        xiao_runtime::cmath_sqrt(-0.0).unwrap(),
        xiao_runtime::cmath_sqrt(4.0).unwrap(),
    ] {
        let bytes = encode_xcon(&XconValue::Float(value)).unwrap();
        assert_eq!(&bytes[..4], b"XCON");
        let XconValue::Float(decoded) = decode_xcon(&bytes).unwrap() else {
            panic!("float标签丢失")
        };
        assert_eq!(value.to_bits(), decoded.to_bits());
    }
}

#[test]
#[ignore = "B4-4需要独立冷构建Runtime和显式LLVM工具链；固定任务精确执行"]
/// 双路径模块包装、位置/命名参数、错误及冲突导出在原生O0/O3逐字核对。
fn cmath_library_matches_native_o0_o3() {
    native::run();
}
