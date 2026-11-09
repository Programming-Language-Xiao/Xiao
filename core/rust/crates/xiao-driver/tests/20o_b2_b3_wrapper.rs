//! B3-4包装的源码契约、移除和语言未接线探测；原生隔离回归独立运行。

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
    println!(
        "B3-CMATH-WRAPPER functions=15 validation=AST-interface status=planned result=success"
    );
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
    assert_eq!(statuses, [entry["status"].as_str().unwrap()]);
    if entry["status"] == "verified" {
        assert!(
            entry["code"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p == "stdlib/cmath.xiao")
        );
        assert!(!entry["tests"].as_array().unwrap().is_empty());
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
/// 探测真实15个函数的两条导入路径；当前状态由B3证据核验，后续批次可记录新状态。
fn b3_handoff_records_language_import_state() {
    use std::time::Duration;
    use xiao_driver::{DriverRequest, FrontendVmDriver};
    let mut checked = 0;
    for prefix in ["std", "stdlib"] {
        for &(name, parameters) in &FROZEN {
            let arguments = if parameters.len() == 1 {
                "1.0"
            } else {
                "1.0, 2.0"
            };
            let request = FrontendRequest::from_text(format!(
                "from {prefix}.cmath import {name}\nprint({name}({arguments}))\n"
            ));
            match FrontendCompiler::new().compile(&request) {
                Ok(artifact) => {
                    let outcome = FrontendVmDriver::new().run_artifact(
                        &artifact,
                        &DriverRequest::new(request).with_timeout(Duration::from_secs(10)),
                    );
                    let executable = outcome.is_success();
                    if !executable {
                        assert_eq!(
                            outcome.code(),
                            Some("X06-RUNTIME-012"),
                            "{prefix}.{name}: {outcome:?}"
                        );
                        let detail = format!("{outcome:?}");
                        assert!(
                            detail.contains(&format!("cmath_{name}"))
                                || detail.contains("尚未绑定 VM"),
                            "{detail}"
                        );
                    }
                    println!(
                        "B3-WRAPPER-IMPORT path={prefix}.cmath function={name} stage=vm code={} executable={executable} result=observed",
                        outcome.code().unwrap_or("none")
                    );
                }
                Err(error) => {
                    let primitive = format!("cmath_{name}");
                    let diagnostic = error
                        .diagnostics()
                        .iter()
                        .find(|d| {
                            d.is_error()
                                && d.code() == "X02-TYPE-001"
                                && d.message().contains(&primitive)
                        })
                        .unwrap_or_else(|| panic!("失败未指明未接线原语：{error:?}"));
                    println!(
                        "B3-WRAPPER-IMPORT path={prefix}.cmath function={name} stage=frontend code={} primitive={primitive} executable=false result=observed",
                        diagnostic.code()
                    );
                }
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 30);
}
