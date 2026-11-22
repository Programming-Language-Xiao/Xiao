//! B3-4包装源码契约及B4-4双路径可执行回归；旧planned拒绝由固定历史证据保留。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use xiao_driver::{FrontendCompiler, FrontendContext, FrontendRequest};
use xiao_intrinsics::{CMATH_DECLARATIONS, IntrinsicId, cmath_parameters};
use xiao_source::SourceFile;
use xiao_syntax::{
    CallArgumentKind, Expression, FunctionParameterKind, FunctionTypeAnnotation, ScalarType,
    Statement,
};

/// 独立列出F-5冻结函数与形参，防止仅根据产品声明表自证导出集合。
const FROZEN: [(&str, &[&str]); 15] = [
    ("sqrt", &["value"]),
    ("floor", &["value"]),
    ("ceil", &["value"]),
    ("round", &["value"]),
    ("trunc", &["value"]),
    ("abs", &["value"]),
    ("min", &["left", "right"]),
    ("max", &["left", "right"]),
    ("pow", &["base", "exponent"]),
    ("sin", &["value"]),
    ("cos", &["value"]),
    ("tan", &["value"]),
    ("exp", &["value"]),
    ("ln", &["value"]),
    ("log10", &["value"]),
];

/// 从编译期crate位置固定源码根，避免测试继承调用者目录。
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
}

/// 读取真正的标准库包装源码，不用手工接口清单代替。
fn wrapper() -> SourceFile {
    SourceFile::from_text(&fs::read_to_string(root().join("stdlib/cmath.xiao")).unwrap())
}

/// 解析真实声明和转发调用，独立核对F-5参数与15个公开函数集合。
fn interface(source: &SourceFile) -> Result<BTreeSet<String>, String> {
    let parsed = xiao_syntax::parse(source);
    if !parsed.is_success() {
        return Err(format!("语法错误：{:?}", parsed.diagnostics));
    }
    let mut names = BTreeSet::new();
    let program = parsed.program.unwrap();
    for statement in program.statements {
        let Statement::Function {
            name,
            type_parameters,
            parameters,
            return_type,
            body,
            ..
        } = statement
        else {
            return Err("包装含函数声明之外的顶层语句".into());
        };
        let name = name.text(source);
        let (index, &(_, frozen_parameters)) = FROZEN
            .iter()
            .enumerate()
            .find(|(_, declaration)| declaration.0 == name)
            .ok_or_else(|| format!("未冻结导出：{name}"))?;
        let (primitive, _, _) = CMATH_DECLARATIONS[index];
        if primitive != format!("cmath_{name}") {
            return Err(format!("原语声明漂移：{name}"));
        }
        let expected = cmath_parameters(IntrinsicId::new(34 + index as u32).unwrap()).unwrap();
        if expected.iter().map(|p| p.0).collect::<Vec<_>>() != frozen_parameters {
            return Err(format!("原语形参漂移：{name}"));
        }
        if !type_parameters.is_empty()
            || parameters.len() != expected.len()
            || return_type != Some(FunctionTypeAnnotation::Scalar(ScalarType::Float))
        {
            return Err(format!("签名不符：{name}"));
        }
        for (parameter, &(expected_name, _)) in parameters.iter().zip(expected) {
            if parameter.name.text(source) != expected_name
                || parameter.kind != FunctionParameterKind::PositionalOrKeyword
                || parameter.annotation != Some(FunctionTypeAnnotation::Scalar(ScalarType::Float))
                || parameter.default.is_some()
            {
                return Err(format!("形参不符：{name}"));
            }
        }
        let [
            Statement::Return {
                value:
                    Some(Expression::Call {
                        callee, arguments, ..
                    }),
                ..
            },
        ] = body.as_slice()
        else {
            return Err(format!("包装不是单一原语转发：{name}"));
        };
        if !matches!(callee.as_ref(), Expression::Name(n) if n.text(source) == primitive)
            || arguments.len() != expected.len()
        {
            return Err(format!("原语不符：{name}"));
        }
        for (argument, &(expected_name, _)) in arguments.iter().zip(expected) {
            if argument.kind != CallArgumentKind::Positional
                || argument.name.is_some()
                || !matches!(&argument.value, Expression::Name(n) if n.text(source) == expected_name)
            {
                return Err(format!("转发参数不符：{name}"));
            }
        }
        if !names.insert(name.to_owned()) {
            return Err(format!("重复导出：{name}"));
        }
    }
    let expected = CMATH_DECLARATIONS
        .iter()
        .map(|d| d.0.strip_prefix("cmath_").unwrap().to_owned())
        .collect();
    if names != expected {
        return Err("公开函数集合不完整".into());
    }
    Ok(names)
}

#[test]
/// AST确认仅导出15个冻结的float原语转发包装。
fn source_contract_exports_exactly_fifteen_frozen_forwarders() {
    assert_eq!(interface(&wrapper()).unwrap().len(), 15);
    for (index, &(name, parameters)) in FROZEN.iter().enumerate() {
        let row = serde_json::json!({"function":name,"parameters":parameters,"parameter_type":"float",
            "return_type":"float","primitive":format!("cmath_{name}"),"intrinsic_id":34+index});
        println!("B3-WRAPPER-EXPORT\t{row}");
    }
    println!("B3-CMATH-WRAPPER functions=15 validation=AST-interface result=success");
}

#[test]
/// 逐个删除真实函数声明，必须破坏完整导出契约。
fn source_contract_removing_each_wrapper_invalidates_exports() {
    let source = wrapper();
    let program = xiao_syntax::parse(&source).program.unwrap();
    for statement in program.statements {
        let Statement::Function { name, span, .. } = statement else {
            panic!("仅函数声明")
        };
        let mut text = source.text().to_owned();
        text.replace_range(span.start()..span.end(), "");
        assert!(
            interface(&SourceFile::from_text(&text)).is_err(),
            "删除{}后集合仍通过",
            name.text(&source)
        );
    }
    println!("B3-CMATH-WRAPPER removals=15 validation=AST-interface result=success");
}

#[test]
/// 两条官方路径共享身份，普通包不得覆盖该命名空间。
fn source_contract_aliases_share_identity_and_reject_package_override() {
    assert_eq!(
        xiao_standard_library::canonical_module_identity("std.cmath"),
        Some("stdlib.cmath".into())
    );
    assert_eq!(
        xiao_standard_library::canonical_module_identity("stdlib.cmath"),
        Some("stdlib.cmath".into())
    );
    for prefix in ["std", "stdlib"] {
        let mut context = FrontendContext::host();
        context.package_namespaces = Some(xiao_types::ExternalNamespaces {
            members: BTreeMap::from([("stdlib".into(), BTreeSet::from(["cmath".into()]))]),
        });
        let request = FrontendRequest::from_text(format!(
            "from {prefix}.cmath import sqrt\nprint(sqrt(4.0))\n"
        ))
        .with_context(context);
        let error = FrontendCompiler::new().compile(&request).unwrap_err();
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == "X05-MODULE-003"),
            "{prefix}: {error:?}"
        );
    }
    println!("B3-CMATH-WRAPPER paths=std,stdlib ordinary-package=rejected result=success");
}

#[test]
/// 教程与registry状态一致，未来verified时还须登记代码和测试。
fn source_contract_tutorial_tracks_registry_and_requires_verified_evidence() {
    let registry: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root().join("docs/module-registry.json")).unwrap(),
    )
    .unwrap();
    let entry = registry["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == "stdlib.cmath")
        .unwrap();
    let tutorial = fs::read_to_string(root().join("docs/UseDocs/library/cmath.md"))
        .unwrap()
        .replace("\r\n", "\n");
    let front = tutorial
        .strip_prefix("---\n")
        .unwrap()
        .split_once("\n---\n")
        .unwrap()
        .0;
    let statuses: Vec<_> = front
        .lines()
        .filter_map(|line| line.strip_prefix("status: "))
        .collect();
    assert_eq!(entry["stage"], "20O-B2-C");
    assert_eq!(entry["status"], "verified");
    assert_eq!(statuses, ["verified"]);
    assert!(
        entry["code"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p == "stdlib/cmath.xiao")
    );
    assert!(!entry["tests"].as_array().unwrap().is_empty());
    let examples = root().join("tests/fixtures/docs/numeric");
    for (language, file) in [
        ("xiao", "cmath-tutorial.xiao"),
        ("text", "cmath-tutorial.stdout"),
    ] {
        let example = fs::read_to_string(examples.join(file))
            .unwrap()
            .replace("\r\n", "\n");
        assert!(tutorial.contains(&format!("```{language}\n{example}```")));
    }
}

#[test]
/// 最小回归：无关标准库包装不得破坏既有hex原生降低。
fn existing_hex_import_keeps_native_lowering() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "from stdlib.hex import encode\nprint(encode([0, 255]))\n",
        ))
        .unwrap();
    let modules: Vec<_> = artifact
        .ir
        .modules
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    println!("B3-CMATH-ISOLATION modules={modules:?}");
    let module = xiao_codegen_llvm::NativeBuild::new()
        .lower(&artifact.ir, &Default::default())
        .unwrap();
    assert!(
        !module.text.contains("cmath_abs"),
        "未导入的 planned cmath 不得进入 hex 原生模块：{}",
        module.text
    );
    println!("B3-CMATH-ISOLATION native=success imported=stdlib.hex preloaded=stdlib.cmath:false");
}

#[test]
/// 未导入的path模块即使引用真实planned cmath包装，也不得破坏hex原生降低。
fn unimported_module_cannot_pull_cmath_into_hex() {
    let mut artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "from stdlib.hex import encode\nprint(encode([0, 255]))\n",
        ))
        .unwrap();
    let mut dependency = artifact
        .ir
        .body
        .iter()
        .find(|statement| matches!(statement.kind, xiao_ir::IrStatementKind::Import { .. }))
        .unwrap()
        .clone();
    let xiao_ir::IrStatementKind::Import { items, .. } = &mut dependency.kind else {
        unreachable!();
    };
    for item in items {
        item.module = "std.cmath".to_owned();
        item.selected.as_mut().unwrap().text = "abs".to_owned();
        item.binding.text = "planned_abs".to_owned();
    }
    let unused = artifact
        .ir
        .modules
        .iter_mut()
        .find(|module| module.name == "stdlib.path")
        .unwrap();
    unused.body.insert(0, dependency);
    let module = xiao_codegen_llvm::NativeBuild::new()
        .lower(&artifact.ir, &Default::default())
        .expect("不可达path模块不能把cmath原语带入hex降低");
    assert!(!module.text.contains("cmath_abs"));
    println!(
        "B3-CMATH-REACHABILITY root=stdlib.hex unreachable=stdlib.path dependency=stdlib.cmath result=success"
    );
}

#[test]
/// 非冻结导出、错误形参/类型/默认值和数学实现必须破坏包装转发契约。
fn source_contract_rejects_extra_exports_and_wrong_forwarders() {
    let text = wrapper().text().to_owned();
    let mutants = [
        text.replace("def sqrt(float value)", "def sqrt(int value)"),
        text.replacen("-> float", "-> int", 1),
        text.replace("def sqrt(float value)", "def sqrt(float value = 1.0)"),
        text.replace("def sqrt(float value)", "def sqrt(float other)"),
        text.replace("return cmath_sqrt(value)", "return cmath_ceil(value)"),
        text.replace("return cmath_sqrt(value)", "return value + 0.0"),
        text.replace(
            "return cmath_min(left, right)",
            "return cmath_min(right, left)",
        ),
        format!("{text}\ndef helper(float value) -> float\n    return value\n"),
        format!("{text}\ndef sqrt(float value) -> float\n    return cmath_sqrt(value)\n"),
        format!("state = 0\n{text}"),
    ];
    for (index, mutant) in mutants.iter().enumerate() {
        assert_ne!(mutant, &text, "变异没有修改源码：{index}");
        assert!(
            interface(&SourceFile::from_text(mutant)).is_err(),
            "变异错误未拒绝：{index}"
        );
    }
    println!("B3-WRAPPER-CONTRACT mutation_cases=10 result=success");
}

#[test]
/// B4-4显式迁移30条路径为可执行；B3历史planned证据仍由固定Git对象证明。
fn b4_handoff_executes_all_thirty_language_imports() {
    use xiao_driver::{DriverRequest, FrontendVmDriver};
    use xiao_vm::VmEvent;
    let goldens = [
        ("sqrt", "4.0", "2"),
        ("floor", "1.5", "1"),
        ("ceil", "1.5", "2"),
        ("round", "2.5", "2"),
        ("trunc", "-1.5", "-1"),
        ("abs", "-3.0", "3"),
        ("min", "7.0, 2.0", "2"),
        ("max", "7.0, 2.0", "7"),
        ("pow", "2.0, 3.0", "8"),
        ("sin", "0.0", "0"),
        ("cos", "0.0", "1"),
        ("tan", "0.0", "0"),
        ("exp", "0.0", "1"),
        ("ln", "1.0", "0"),
        ("log10", "1.0", "0"),
    ];
    assert_eq!(
        goldens.iter().map(|r| r.0).collect::<Vec<_>>(),
        FROZEN.iter().map(|r| r.0).collect::<Vec<_>>()
    );
    for prefix in ["std", "stdlib"] {
        for (name, arguments, expected) in goldens {
            let request = FrontendRequest::from_text(format!(
                "from {prefix}.cmath import {name}\nprint({name}({arguments}))\n"
            ));
            let artifact = FrontendCompiler::new().compile(&request).unwrap();
            let outcome =
                FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(request));
            assert!(outcome.is_success(), "{prefix}.{name}: {outcome:?}");
            let events = outcome.as_executed().unwrap().events();
            let output: String = events
                .iter()
                .filter_map(|event| match event {
                    VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(output, format!("{expected}\n"));
            assert_eq!(events.iter().filter(|event|matches!(event, VmEvent::ModuleLoaded {module} if module == "stdlib.cmath")).count(), 1);
            println!(
                "B4-4-IMPORT path={prefix}.cmath function={name} identity=stdlib.cmath executable=true result=passed"
            );
        }
    }
}
