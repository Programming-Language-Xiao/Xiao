//! B6-1语言级资源/错误/原子性回归；ABI状态与正式MPFR向量由Runtime测试覆盖。

use std::time::Duration;

use xiao_driver::{DriverRequest, FrontendCompiler, FrontendRequest, FrontendVmDriver};
use xiao_vm::VmEvent;

const CMATH_LIBRARY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../tests/fixtures/docs/numeric/cmath-library.xiao"
));
const CMATH_LIBRARY_GOLDEN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../tests/fixtures/docs/numeric/cmath-library.stdout"
));

const NUMERIC_BOUNDARIES: &str = r#"
one = lfloat("1.00")
three = lfloat("3")
huge = lfloat("1e100000")
tiny = lfloat("1e-100000")
odd_tiny = lfloat("3e-100000")
try
    one / three
    print("unexpected")
catch failure as ArithmeticError
    print(error_code(failure))
finally
    print("exact-divide-clean")
try
    one / lfloat("0")
    print("unexpected")
catch failure as ArithmeticError
    print(error_code(failure))
try
    huge // tiny
    print("unexpected")
catch failure as ResourceError
    print(error_code(failure))
print(huge % tiny)
print(huge % odd_tiny)
try
    lfloat("1e100001")
    print("unexpected")
catch failure as ResourceError
    print(error_code(failure))
print(lfloat_divide(one, three, 1, "half_even"))
print(one)
print(three)
print("b6-1-continued")
"#;

fn output(events: &[VmEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
/// 两条官方导入路径都贯通数值错误、精确/显式除法、独立余数与finally清理。
fn b6_numeric_and_cmath_errors_are_catchable_and_atomic_on_both_aliases() {
    let expected_tail = concat!(
        "exact-divide-clean\n",
        "X06-RUNTIME-034\n",
        "X06-RUNTIME-013\n",
        "X06-RUNTIME-033\n",
        "0\n",
        "1e-100000\n",
        "X06-RUNTIME-033\n",
        "0.3\n",
        "1.00\n",
        "3\n",
        "b6-1-continued\n",
    );
    for prefix in ["std", "stdlib"] {
        let source =
            CMATH_LIBRARY.replace("std.cmath", &format!("{prefix}.cmath")) + NUMERIC_BOUNDARIES;
        let request = FrontendRequest::from_text(source);
        let artifact = FrontendCompiler::new().compile(&request).unwrap();
        let outcome = FrontendVmDriver::new().run_artifact(
            &artifact,
            &DriverRequest::new(request)
                .with_timeout(Duration::from_secs(30))
                .with_event_capacity(8192),
        );
        assert!(outcome.is_success(), "{prefix}: {outcome:?}");
        let actual = output(outcome.as_executed().unwrap().events());
        let expected = format!(
            "{}{}",
            CMATH_LIBRARY_GOLDEN.replace("\r\n", "\n"),
            expected_tail
        );
        assert_eq!(actual, expected, "{prefix}");
        println!(
            "B6-1-LANGUAGE module={prefix}.cmath math=15 lfloat_errors=009,013,033,034 finally=passed input_unchanged=true result=passed"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
/// 诊断产物组件误报时，绕过FrontendNativeDriver二次门禁核对实际语料/符号/执行结果。
fn b6_diagnoses_linux_native_cmath_runtime_component_observation() {
    use std::path::PathBuf;
    use xiao_codegen_llvm::{CodegenOptions, TargetDescription, Toolchain};

    let mut source = CMATH_LIBRARY.to_owned();
    source = source.replace("import std.cmath as math\n", "");
    for (index, &(primitive, _, _)) in xiao_intrinsics::CMATH_DECLARATIONS.iter().enumerate() {
        let name = primitive.strip_prefix("cmath_").unwrap();
        source = source.replace(&format!("math.{name}("), &format!("f{index}("));
    }
    let request = FrontendRequest::from_text(source);
    let artifact = FrontendCompiler::new().compile(&request).unwrap();
    let target = TargetDescription::host();
    let module =
        xiao_codegen_llvm::lower_program(&artifact.ir, &CodegenOptions::for_target(target.clone()))
            .unwrap();
    let clang = PathBuf::from(std::env::var_os("XIAO_CLANG").unwrap());
    let llvm_as = PathBuf::from(std::env::var_os("XIAO_LLVM_AS").unwrap());
    let runtime = PathBuf::from(std::env::var_os("XIAO_RUNTIME_LIBRARY").unwrap());
    let toolchain = Toolchain::new(clang)
        .with_llvm_as(llvm_as)
        .with_runtime_library(&runtime)
        .probe_native_static_libraries(
            std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            &target,
        )
        .unwrap();
    let output =
        std::env::temp_dir().join(format!("b6-component-diagnostic-{}", std::process::id()));
    let executable = toolchain
        .compile_with_runtime_components_at_level(
            &module.text,
            &target,
            &output,
            Some(&runtime),
            &module.runtime_components,
            0,
        )
        .unwrap();
    let inspection = xiao_codegen_llvm::inspect_artifact(&executable, &target).unwrap();
    let array_symbols: Vec<_> = inspection
        .symbols
        .iter()
        .filter(|symbol| symbol.starts_with("xiao_runtime_array_"))
        .cloned()
        .collect();
    println!(
        "B6-NATIVE-DIAGNOSTIC declared={:?} array_symbols={array_symbols:?}",
        module.runtime_components
    );
    let child = std::process::Command::new(&executable).output().unwrap();
    assert!(child.status.success(), "{:?}", child.status);
    assert!(
        child.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    assert_eq!(
        String::from_utf8(child.stdout)
            .unwrap()
            .replace("\r\n", "\n"),
        CMATH_LIBRARY_GOLDEN.replace("\r\n", "\n")
    );
    let _ = std::fs::remove_file(executable);
}
