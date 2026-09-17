//! 09R2 后端释放序列对账规格。
//!
//! 验证器只能看到释放计划本身，而计划是「每个作用域 × 全部退出边」的无条件
//! 笛卡尔积，因此覆盖类断言恒真。这些用例锁定真正有校验价值的部分：实际发出
//! 的释放序列必须与冻结计划逐条相等。

use xiao_ir::{
    IR_RELEASE_MISMATCH_CODE, IrProgram, IrReleaseAction, ObservedRelease, reconcile_release_plans,
};
use xiao_lifetime::analyze as analyze_lifetime;
use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::check;

/// 解析、类型检查、生命周期分析并降低一个无错误源码样例。
fn lower(source_text: &str) -> IrProgram {
    let source = SourceFile::from_text(source_text);
    let parsed = Parser::new(&source).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "语法诊断: {:?}",
        parsed.diagnostics
    );
    let program = parsed.program.expect("程序");
    let typed = check(&source, &program);
    assert!(
        typed.diagnostics.is_empty(),
        "类型诊断: {:?}",
        typed.diagnostics
    );
    let lifetime = analyze_lifetime(&source, &program, &typed);
    assert!(
        lifetime.diagnostics.is_empty(),
        "生命周期诊断: {:?}",
        lifetime.diagnostics
    );
    xiao_ir::lower_program(&source, &program, &typed, &lifetime, None)
}

/// 找出第一个带释放动作的计划。
fn first_non_empty_plan(ir: &IrProgram) -> (u32, String, Vec<IrReleaseAction>) {
    let plan = ir
        .ownership
        .release_plans
        .iter()
        .find(|plan| !plan.actions.is_empty())
        .expect("至少应有一个非空释放计划");
    (plan.scope, plan.exit.clone(), plan.actions.clone())
}

#[test]
/// 原样回放冻结计划时对账通过。
fn accepts_replayed_plan() {
    let ir = lower("value = \"x\"\n");
    let (scope, exit, actions) = first_non_empty_plan(&ir);
    let observed = vec![ObservedRelease::new(scope, exit, actions)];
    let result = reconcile_release_plans(&ir, &observed);
    assert!(result.is_success(), "对账错误: {:?}", result.errors());
}

#[test]
/// 释放序列按 order 排序比较，不受观测向量下标顺序影响。
fn compares_by_order_not_by_index() {
    let ir = lower("first = \"a\"\nsecond = \"b\"\n");
    let (scope, exit, mut actions) = first_non_empty_plan(&ir);
    assert!(actions.len() >= 2, "需要至少两个动作才能验证顺序无关");
    actions.reverse();
    let observed = vec![ObservedRelease::new(scope, exit, actions)];
    let result = reconcile_release_plans(&ir, &observed);
    assert!(result.is_success(), "对账错误: {:?}", result.errors());
}

#[test]
/// 后端按不同顺序发出释放时必须报 X08-IR-003，而不是静默通过。
///
/// 这里连 `order` 一起改写，模拟「真的换了释放次序并据此重新编号」；
/// 只交换 `Vec` 下标属于下标无关的情况，由 `compares_by_order_not_by_index`
/// 断言应当通过。
fn rejects_reordered_release() {
    let ir = lower("first = \"a\"\nsecond = \"b\"\n");
    let (scope, exit, mut actions) = first_non_empty_plan(&ir);
    assert!(actions.len() >= 2, "需要至少两个动作才能验证重排");
    actions.swap(0, 1);
    for (index, action) in actions.iter_mut().enumerate() {
        action.order = index;
    }
    let observed = vec![ObservedRelease::new(scope, exit, actions)];
    let result = reconcile_release_plans(&ir, &observed);
    assert!(!result.is_success());
    assert_eq!(result.errors()[0].code, IR_RELEASE_MISMATCH_CODE);
}

#[test]
/// 少放一个值时必须报错，漏放不会被当成宽容通过。
fn rejects_missing_release() {
    let ir = lower("first = \"a\"\nsecond = \"b\"\n");
    let (scope, exit, mut actions) = first_non_empty_plan(&ir);
    assert!(actions.len() >= 2, "需要至少两个动作才能验证漏放");
    actions.pop();
    let observed = vec![ObservedRelease::new(scope, exit, actions)];
    assert!(!reconcile_release_plans(&ir, &observed).is_success());
}

#[test]
/// 动作类别被改写时必须报错，强释放不能悄悄变成弱释放。
fn rejects_changed_action_kind() {
    let ir = lower("value = \"x\"\n");
    let (scope, exit, mut actions) = first_non_empty_plan(&ir);
    actions[0].kind = "weak".to_owned();
    let observed = vec![ObservedRelease::new(scope, exit, actions)];
    assert!(!reconcile_release_plans(&ir, &observed).is_success());
}

#[test]
/// 指向不存在的作用域时必须报错。
fn rejects_unknown_scope() {
    let ir = lower("value = \"x\"\n");
    let observed = vec![ObservedRelease::new(9_999, "normal", Vec::new())];
    let result = reconcile_release_plans(&ir, &observed);
    assert!(!result.is_success());
    assert_eq!(result.errors()[0].code, IR_RELEASE_MISMATCH_CODE);
}

#[test]
/// 退出边名称不在冻结拼写集合内时必须报错。
fn rejects_unknown_exit_name() {
    let ir = lower("value = \"x\"\n");
    let observed = vec![ObservedRelease::new(0, "Normal", Vec::new())];
    let result = reconcile_release_plans(&ir, &observed);
    assert!(!result.is_success());
    assert_eq!(result.errors()[0].code, IR_RELEASE_MISMATCH_CODE);
}

#[test]
/// 空计划与空观测一致，不会被误判为漏放。
fn accepts_empty_plan_for_unused_exit() {
    let ir = lower("value = 1\n");
    let empty = ir
        .ownership
        .release_plans
        .iter()
        .find(|plan| plan.actions.is_empty())
        .expect("标量程序应有空计划");
    let observed = vec![ObservedRelease::new(
        empty.scope,
        empty.exit.clone(),
        Vec::new(),
    )];
    assert!(reconcile_release_plans(&ir, &observed).is_success());
}
