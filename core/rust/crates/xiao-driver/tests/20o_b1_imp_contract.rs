//! 20O-B1-IMP：公开导入、真实导出、边界、移除、归档与教程的产品回归。

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use xiao_driver::{
    ArchiveBuildRequest, CancellationToken, DriverRequest, FrontendCompiler, FrontendRequest,
    FrontendVmDriver, ProtocolRequest, ProtocolResponse, RunOptions, build_archive, dispatch,
};
use xiao_vm::VmEvent;

/// 已落地模块的唯一公开导出清单；其余模块按实施顺序加入。
const PRODUCTS: &[(&str, &[&str])] = &[
    ("math", &["abs", "min", "max", "clamp", "gcd", "pow"]),
    ("statistics", &["sum", "min", "max"]),
    ("fmt", &["pad_left", "pad_right", "format_int"]),
];

/// 并行契约测试使用独立缓存和应用路径。
static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 本进程单个用例拥有的临时项目；归档重放不依赖应用源码继续存在。
struct Directory(PathBuf);

impl Directory {
    /// 建立固定安全前缀的临时项目。
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "xiao-b1-imp-{label}-{}-{}",
            std::process::id(),
            DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    /// 仅删除系统临时根下由本测试创建的直接子目录。
    fn drop(&mut self) {
        if let (Ok(path), Ok(temp)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize())
            && path.parent() == Some(temp.as_path())
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("xiao-b1-imp-"))
        {
            let _ = fs::remove_dir_all(path);
        }
    }
}

/// 逐函数移除使用最小合法调用，参数与公开签名独立登记。
fn arguments(module: &str, function: &str) -> &'static str {
    match (module, function) {
        ("math", "abs") => "-7",
        ("math", "min" | "max" | "gcd") => "4, 2",
        ("math", "clamp") => "1, 0, 2",
        ("math", "pow") => "2, 3",
        ("statistics", _) => "[1, 2]",
        ("fmt", "pad_left" | "pad_right") => "\"x\", 3, \".\"",
        ("fmt", "format_int") => "10, 16",
        ("hex", "encode") => "[0, 255]",
        ("hex", "decode") => "\"00ff\"",
        ("base64", "encode") => "[0, 255], false",
        ("base64", "decode") => "\"AP8=\", false",
        ("url", _) => "\"hello\"",
        ("csv", "parse_record") => "\"a,b\"",
        ("csv", "format_record") => "[\"a\", \"b\"]",
        _ => panic!("未登记的产品入口 {module}.{function}"),
    }
}

/// 从固定 crate 布局定位实际标准库和产品夹具。
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
}

/// 产品示例必须保留真实文件路径，使用正式标准库加载器。
fn product(module: &str) -> (FrontendRequest, String) {
    let directory = root().join(format!("tests/fixtures/20o-b1/imp/{module}"));
    let path = directory.join("product.xiao");
    let golden: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(directory.join("golden.json")).unwrap()).unwrap();
    (
        FrontendRequest::from_text_at(fs::read_to_string(&path).unwrap(), path),
        golden["stdout"].as_str().unwrap().to_owned(),
    )
}

/// 运行实际产品并核对完整 stdout 和成功退出，不从同一算法生成预期值。
fn assert_output(request: FrontendRequest, expected: &str) {
    let result = FrontendVmDriver::new()
        .run(&DriverRequest::new(request).with_timeout(Duration::from_secs(30)));
    assert!(
        result.is_success(),
        "code={:?}; report={:?}",
        result.code(),
        result.report()
    );
    let output: String = result
        .as_executed()
        .unwrap()
        .events()
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(output.as_bytes(), expected.as_bytes());
}

#[test]
/// 每个模块逐字执行黄金，并经真实模块接口核对完整公开名称和来源。
fn products_execute_and_export_only_the_frozen_names() {
    for &(module, names) in PRODUCTS {
        let (request, expected) = product(module);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("{module}: {error:?}"));
        let interface = artifact
            .ir
            .modules
            .iter()
            .find(|item| item.name == format!("stdlib.{module}"))
            .unwrap();
        let mut actual: Vec<_> = interface
            .symbols
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect();
        let mut expected_names = names.to_vec();
        actual.sort();
        expected_names.sort();
        assert_eq!(actual, expected_names, "{module} 实际导出集合");
        assert!(
            interface
                .symbols
                .iter()
                .all(|symbol| symbol.kind == "function" && symbol.origin == "local"),
            "{module}: {:?}",
            interface.symbols
        );
        let canonical = request.source.text().replace(
            &format!("from std.{module} import"),
            &format!("from stdlib.{module} import"),
        );
        assert_output(
            FrontendRequest::from_text_at(
                canonical,
                root().join(format!("tests/fixtures/20o-b1/imp/{module}/product.xiao")),
            ),
            &expected,
        );
    }
}

#[test]
/// 已有 text 和新库均须通过公开 std 别名，不能只证明 stdlib 规范路径可执行。
fn public_std_aliases_execute_through_the_official_loader() {
    assert_output(
        FrontendRequest::from_text("from std.text import length\nprint(length(\"ok\"))\n"),
        "2\n",
    );
    for &(module, _) in PRODUCTS {
        let (request, expected) = product(module);
        assert_output(request, &expected);
    }
}

#[test]
/// 每个公开函数分别证明需要加载器和真实静态函数体，拒绝必须指明函数及源码区间。
fn every_public_function_requires_its_real_implementation() {
    use xiao_codegen_llvm::{CodegenError, CodegenOptions, NativeBuild};
    use xiao_ir::IrStatementKind;
    for &(module, names) in PRODUCTS {
        for &function in names {
            let source = format!(
                "from stdlib.{module} import {function}\nprint({function}({}))\n",
                arguments(module, function)
            );
            let artifact = FrontendCompiler::new()
                .compile(&FrontendRequest::from_text(source))
                .unwrap();
            let program = xiao_bytecode::lower_program(&artifact.ir);
            let missing = xiao_vm::run_request(&xiao_vm::RunRequest::new(&artifact.ir, &program));
            let xiao_vm::RunResult::Error(error) = missing.result else {
                panic!("{module}.{function}: 缺加载器未拒绝");
            };
            assert_eq!(error.code(), "X06-RUNTIME-012");
            let mut broken = artifact.ir.clone();
            let keep = |statement: &xiao_ir::IrStatement| !matches!(&statement.kind, IrStatementKind::Function { name, .. } if name.text == function);
            broken.body.retain(keep);
            for imported in &mut broken.modules {
                imported.body.retain(keep);
            }
            let result = NativeBuild::new().lower(&broken, &CodegenOptions::default());
            let Err(CodegenError::Unsupported { feature, span }) = result else {
                panic!("{module}.{function}: 移除函数未明确拒绝：{result:?}");
            };
            assert!(feature.contains(function), "{module}.{function}: {feature}");
            assert!(span.is_some(), "{module}.{function}: 缺源码区间");
        }
    }
}

#[test]
/// 真实归档只记录宿主官方模块，不复制产品函数；删除应用源码后逐字重放。
fn products_replay_from_real_archives_without_bundled_library_bodies() {
    for &(module, names) in PRODUCTS {
        let (request, expected) = product(module);
        let directory = Directory::new(module);
        let path = directory.0.join("main.xiao");
        fs::write(&path, request.source.text()).unwrap();
        let mut build = ArchiveBuildRequest::new(&path);
        build.cache_root = Some(directory.0.join("cache"));
        let archive = build_archive(&build, &CancellationToken::new()).unwrap();
        assert_eq!(archive.index.stdlib_modules, [format!("stdlib.{module}")]);
        assert_eq!(archive.index.stdlib_interface_version, 1);
        assert!(
            archive
                .index
                .entries
                .iter()
                .all(|entry| !entry.module.starts_with("stdlib.")
                    && !entry.module.starts_with("std."))
        );
        let container = xiao_xar::decode_xar(&fs::read(&archive.output).unwrap()).unwrap();
        let entry = xiao_bytecode::decode_xiaoc(&container.read_entry().unwrap()).unwrap();
        assert!(
            entry
                .program
                .functions
                .iter()
                .all(|function| !names.contains(&function.name.as_str()))
        );
        fs::remove_file(&path).unwrap();
        let response = dispatch(ProtocolRequest::RunArchive {
            request_id: format!("b1-imp-{module}"),
            protocol_version: xiao_driver::PROTOCOL_VERSION,
            core_version: xiao_driver::CORE_VERSION,
            locale: None,
            path: archive.output.display().to_string(),
            options: RunOptions {
                timeout_ms: Some(60_000),
                ..Default::default()
            },
            diagnostics: None,
            debug: false,
        });
        let ProtocolResponse::Result {
            exit_code, events, ..
        } = response
        else {
            panic!("{module}: {response:?}");
        };
        assert_eq!(exit_code, 0, "{module}: {events:?}");
        let output: String = events
            .iter()
            .filter(|event| event.kind == "intrinsic_output")
            .filter_map(|event| event.data.get("text").and_then(serde_json::Value::as_str))
            .collect();
        assert_eq!(output, expected, "{module} 归档黄金");
    }
}

#[test]
/// 普通包接口即使提供同名产品，也不能占用 stdlib 保留根或替代官方源码。
fn products_keep_the_official_namespace_out_of_ordinary_packages() {
    use std::collections::{BTreeMap, BTreeSet};
    for &(module, names) in PRODUCTS {
        let mut context = xiao_driver::FrontendContext::host();
        context.package_namespaces = Some(xiao_types::ExternalNamespaces {
            members: BTreeMap::from([("stdlib".to_owned(), BTreeSet::from([module.to_owned()]))]),
        });
        let source = format!(
            "from stdlib.{module} import {}\nprint({}({}))\n",
            names[0],
            names[0],
            arguments(module, names[0])
        );
        let error = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source).with_context(context))
            .unwrap_err();
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == "X05-MODULE-003")
        );
    }
}

#[test]
/// 产品教程逐字绑定已执行样例，页面状态与对应登记项一致，允许 CLOSE 自然升级。
fn product_tutorials_embed_the_executed_sources_and_match_registry() {
    let registry: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root().join("docs/module-registry.json")).unwrap(),
    )
    .unwrap();
    for &(module, _) in PRODUCTS {
        let page = fs::read_to_string(root().join(format!("docs/UseDocs/library/{module}.md")))
            .unwrap()
            .replace("\r\n", "\n");
        let (request, _) = product(module);
        let source = request.source.text().replace("\r\n", "\n");
        let embedded = page
            .split_once("<!-- b1-imp-executable-example -->\n```xiao\n")
            .unwrap()
            .1
            .split_once("```")
            .unwrap()
            .0;
        assert_eq!(embedded, source, "{module} 产品教程必须逐字执行");
        let id = format!("stdlib.{module}");
        let record = registry["modules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"].as_str() == Some(id.as_str()))
            .unwrap();
        let front = page
            .strip_prefix("---\n")
            .unwrap()
            .split_once("\n---\n")
            .unwrap()
            .0;
        let statuses: Vec<_> = front
            .lines()
            .filter_map(|line| line.strip_prefix("status: "))
            .collect();
        assert_eq!(
            statuses,
            [record["status"].as_str().unwrap()],
            "{module} 状态漂移"
        );
        assert!(
            record["code"]
                .as_array()
                .unwrap()
                .iter()
                .any(|path| path.as_str() == Some(format!("stdlib/{module}.xiao").as_str()))
        );
    }
}
