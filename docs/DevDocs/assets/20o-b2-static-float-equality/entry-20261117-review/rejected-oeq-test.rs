//! 已撤回的未提交 oeq 测试草案，仅供审阅；没有执行或验收，不属于产品回归。

use xiao_codegen_llvm::{CodegenOptions, NativeBuild, TargetDescription, Toolchain};
use xiao_driver::{FrontendCompiler, FrontendRequest};

#[test]
/// 真实参数不被常量折叠，Float 与 Sfloat 均使用有序浮点谓词。
fn static_float_equality_from_real_parameters_uses_ordered_predicates() {
    for (source, llvm_ty) in [
        (
            r#"def equals(float left, float right) -> bool
    return left == right
"#,
            "double",
        ),
        (
            r#"def equals(sfloat left, sfloat right) -> bool
    return left == right
"#,
            "float",
        ),
    ] {
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .expect("真实参数比较源码应通过前端");
        let module = xiao_codegen_llvm::lower_program(
            &artifact.ir,
            &CodegenOptions::default(),
        )
        .expect("真实参数比较应降低");
        assert!(module.text.contains(&format!("fcmp oeq {llvm_ty}")));
        assert!(!module.text.contains("fcmp eq "));
        assert!(!module.uses_runtime);
        if let Some(llvm_as) = std::env::var_os("XIAO_LLVM_AS") {
            NativeBuild::new()
                .validate_llvm(
                    &module.text,
                    &TargetDescription::host(),
                    &Toolchain::new("unused").with_llvm_as(llvm_as),
                )
                .expect("真实参数比较 LLVM 文本应可汇编");
        }
    }
}
