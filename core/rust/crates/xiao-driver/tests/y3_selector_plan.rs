//! Y3 选择计划生产契约回归。

use xiao_driver::{FrontendCompiler, FrontendRequest};

/// 合法的高级选择器必须为每个选择表达式登记一个规范计划引用。
#[test]
fn legal_selector_forms_always_carry_a_plan_reference() {
    let source = "values = [1, 2, 3, 4]\npart = values[1~2]\nrandom_part = values[?2]\nall_part = values[=]\nopen_part = values[<2]\nmulti_part = values[0, 2]\n";
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("合法选择器应通过前端");
    let debug = format!("{:?}", artifact.ir);
    let references = debug.matches("selection_plan: Some(").count();
    println!(
        "selection_plans={} selector_plan_references={references}",
        artifact.ir.selection_plans.len()
    );
    assert_eq!(artifact.ir.selection_plans.len(), 5);
    assert_eq!(references, 5);
}
