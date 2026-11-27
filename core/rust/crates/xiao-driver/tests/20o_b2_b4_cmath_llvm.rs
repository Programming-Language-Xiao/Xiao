//! B4-3：15项cmath静态/动态 lowering、共享pow与显式原生O0/O3验收。
#[path = "support/b4_cmath_native.rs"]
mod native;
#[path = "20o_b2_a_support/mod.rs"]
#[allow(dead_code)]
mod numeric_support;
use xiao_codegen_llvm::{CodegenOptions, EntryObservation};
use xiao_driver::{FrontendCompiler, FrontendRequest};

/// 不经标准库导入，按冻结15项顺序定义直接原语探针。
pub fn probes(dynamic: bool) -> xiao_ir::IrProgram {
    let mut source = String::new();
    for (index, &(primitive, _, _)) in xiao_intrinsics::CMATH_DECLARATIONS.iter().enumerate() {
        let parameters = xiao_intrinsics::cmath_parameters(
            xiao_intrinsics::IntrinsicId::new(34 + index as u32).unwrap(),
        )
        .unwrap();
        let (signature, args) = if parameters.len() == 1 {
            ("float a", "value = a".to_owned())
        } else {
            (
                "float a, float b",
                format!("{} = b, {} = a", parameters[1].0, parameters[0].0),
            )
        };
        source.push_str(&format!(
            "def probe{index}({signature}) -> float\n    return {primitive}({args})\n"
        ));
    }
    source.push_str("def probe15(float a, float b) -> float\n    return a ** b\n");
    if dynamic {
        source.push_str("force_dynamic = \"owned\"\ndynamic_root_probe = cmath_sqrt(4.0)\n");
    }
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap()
        .ir
}

#[test]
/// 两种降低都发射15个既有C符号；静态仍为标量函数，动态仍为拥有值槽。
fn both_lowerings_use_fifteen_shared_abi_symbols_and_current_version() {
    for dynamic in [false, true] {
        let module =
            xiao_codegen_llvm::lower_program(&probes(dynamic), &CodegenOptions::default()).unwrap();
        assert!(module.uses_runtime);
        assert!(module.runtime_components.contains(&"value".to_owned()));
        assert_eq!(
            module.runtime_abi_version,
            Some(xiao_runtime_abi::ABI_ENCODED_VERSION)
        );
        assert!(module.codegen_fingerprint.starts_with(&format!(
            "xiao-codegen-{}-",
            xiao_codegen_llvm::CODEGEN_VERSION
        )));
        for &(primitive, _, _) in xiao_intrinsics::CMATH_DECLARATIONS.iter() {
            let symbol = format!("@xiao_runtime_{primitive}(");
            assert!(
                module.text.contains(&format!("call i32 {symbol}")),
                "{primitive}"
            );
        }
        assert!(!module.text.contains("llvm.pow.f64"));
        assert!(
            module.text.contains("switch i32") && module.text.contains("i32 6, label %cmath.error")
        );
        if dynamic {
            assert!(module.text.contains("define void @xiao.fn."));
        } else {
            assert!(module.text.contains("define double @xiao_fn_0_probe0"));
        }
    }
    assert_eq!(xiao_codegen_llvm::CODEGEN_VERSION, 27);
    assert_eq!(xiao_runtime_abi::ABI_MINOR_VERSION, 20);
}

#[test]
/// Float幂与cmath共核，Sfloat幂保持llvm.pow.f32；拥有scratch不能积累于静态循环。
fn float_power_uses_runtime_and_sfloat_keeps_existing_intrinsic() {
    let request = FrontendRequest::from_text(
        "def p(float a, float b) -> float\n    return a ** b\nvalue = p(2.0, 3.0)\n",
    );
    let ir = FrontendCompiler::new().compile(&request).unwrap().ir;
    let module = xiao_codegen_llvm::lower_program(&ir, &CodegenOptions::default()).unwrap();
    assert!(module.uses_runtime && module.text.contains("@xiao_runtime_cmath_pow"));
    assert!(
        module.text.contains("@llvm.stacksave()") && module.text.contains("@llvm.stackrestore(ptr")
    );
    assert!(!module.text.contains("llvm.pow.f64"));
    let ir = FrontendCompiler::new().compile(&FrontendRequest::from_text("def p(sfloat a, sfloat b) -> sfloat\n    return a ** b\nvalue = p(sfloat(2.0), sfloat(3.0))\n")).unwrap().ir;
    let module = xiao_codegen_llvm::lower_program(&ir, &CodegenOptions::default()).unwrap();
    assert!(module.text.contains("@llvm.pow.f32"));
    assert!(!module.text.contains("@xiao_runtime_cmath_pow"));
}

#[test]
/// 静态cmath保持uses_runtime事实，入口观察仍可经普通标量返回。
fn static_scalar_calls_keep_runtime_link_and_explicit_observation() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(
            "result = int(cmath_pow(exponent = 3.0, base = 2.0))\n",
        ))
        .unwrap();
    let module = xiao_codegen_llvm::lower_program(
        &artifact.ir,
        &CodegenOptions::default().with_entry_observation(EntryObservation::ExitCode),
    )
    .unwrap();
    assert!(module.uses_runtime && module.text.contains("define i64 @xiao_entry"));
}

#[test]
/// B3归档的1075组float幂差异逐条对应迁移后的正式输入和共享pow新黄金。
fn historical_float_power_diff_is_bound_to_formal_cmath_vectors() {
    use std::collections::HashMap;
    let vectors = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/fixtures/docs/cmath/reference-vectors.tsv"
    ));
    let pow: HashMap<_, _> = vectors
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            (columns[1] == "pow").then_some((columns[0], (columns[2], columns[3], columns[4])))
        })
        .collect();
    assert_eq!(pow.len(), 1_075);
    let differences = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/fixtures/docs/cmath/pow-legacy-diff.tsv"
    ));
    let mut counts = (0, 0, 0);
    let mut seen = HashMap::new();
    for line in differences.lines().filter(|line| !line.starts_with('#')) {
        let columns: Vec<_> = line.split('\t').collect();
        assert_eq!(columns.len(), 7, "{line}");
        assert!(
            seen.insert(columns[0], ()).is_none(),
            "重复差异向量：{}",
            columns[0]
        );
        assert_eq!(
            pow.get(columns[0]),
            Some(&(columns[1], columns[2], columns[5])),
            "输入/参考漂移：{}",
            columns[0]
        );
        let a = f64::from_bits(u64::from_str_radix(columns[1], 16).unwrap());
        let b = f64::from_bits(u64::from_str_radix(columns[2], 16).unwrap());
        let current = xiao_runtime::cmath_pow(a, b)
            .map(f64::to_bits)
            .map(|bits| format!("{bits:016x}"))
            .unwrap_or_else(|error| error.code().to_owned());
        assert_eq!(
            current, columns[4],
            "当前共享pow实现偏离B3差异表：{}",
            columns[0]
        );
        let same = columns[3] == columns[4];
        assert_eq!(columns[6], if same { "true" } else { "false" });
        if same {
            counts.0 += 1;
        } else if columns[3].starts_with("X06-") || columns[4].starts_with("X06-") {
            counts.1 += 1;
        } else {
            counts.2 += 1;
        }
    }
    assert_eq!(seen.len(), 1_075);
    assert_eq!(counts, (1_011, 40, 24));
    println!(
        "B4-3-POW-DIFF rows=1075 same=1011 error_boundary=40 value_bits=24 reference_inputs=exact result=passed"
    );
}

#[test]
#[ignore = "需要本任务全新Runtime和显式clang/llvm-as；B4-3独立取证精确执行"]
/// 实际O0/O3执行静态/动态15项及全部1075组pow输入，逐位或逐错误核对。
fn native_cmath_static_dynamic_o0_o3_and_shared_pow() {
    native::run();
}
