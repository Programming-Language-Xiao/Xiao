//! B0-A 生产入口与研究兼容别名的 API 回归。

use xiao_bytecode::{
    TAC_INTERNAL_CONSISTENCY_CODE, lower_program, verify_for_execution, verify_production,
};
use xiao_driver::{FrontendCompiler, FrontendRequest};

#[test]
/// 根路径与兼容路径必须指向同一生产验证语义。
fn root_and_research_paths_share_the_production_verifier() {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text("value = 1\n"))
        .expect("前端应成功");
    let ir = artifact.ir;
    let mut tac = lower_program(&ir);
    assert!(tac.unsupported.is_empty());

    tac.unsupported.push("测试用未降低构造".to_owned());
    let root_error = verify_production(&ir, &tac).expect_err("生产入口必须拒绝");
    assert_eq!(root_error.code, TAC_INTERNAL_CONSISTENCY_CODE);
    let alias_error = xiao_bytecode::research::verify_production(&ir, &tac)
        .expect_err("兼容入口必须保留同一拒绝语义");
    assert_eq!(alias_error.code, TAC_INTERNAL_CONSISTENCY_CODE);
    let execution_error = verify_for_execution(&ir, &tac).expect_err("执行前入口必须拒绝");
    assert_eq!(execution_error.code, TAC_INTERNAL_CONSISTENCY_CODE);
}
