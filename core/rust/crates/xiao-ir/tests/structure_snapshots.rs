//! 08 类型化 IR 结构快照入口。

use serde::Deserialize;
use xiao_ir::{IrExpressionKind, IrProgram, IrSelectionPlan, IrSelector, IrSelectorItem};
use xiao_ir::{IrStatementKind, IrValidator, from_json, lower_program, to_json};
use xiao_lifetime::analyze as analyze_lifetime;
use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::check;

/// 一条 IR 结构快照用例。
#[derive(Debug, Deserialize)]
struct SnapshotCase {
    /// 用例名称，用于失败定位。
    name: String,
    /// Xiao 源码。
    source: String,
    /// 期望状态。
    expect: String,
    /// 期望 IR 结构摘要。
    ir: IrExpectation,
}

/// 一条不依赖后端实现细节的 IR 结构摘要。
#[derive(Debug, Deserialize)]
struct IrExpectation {
    /// IR 版本。
    version: u32,
    /// 目标描述。
    target: String,
    /// 入口模式。
    entry_mode: String,
    /// 顶层语句形态。
    body_kinds: Vec<String>,
    /// 表签名数量。
    table_signatures: usize,
    /// 选择计划数量。
    selection_plans: usize,
    /// 是否必须能完成 JSON 往返。
    round_trip: bool,
    /// 选择器语法项的稳定类别；非选择器用例为空。
    #[serde(default)]
    selector_items: Vec<String>,
    /// 规范化选择计划的静态路径数量。
    #[serde(default)]
    selected_path_count: Option<usize>,
    /// 规范化选择计划的静态步长；没有步长时为空。
    #[serde(default)]
    step_value: Option<i128>,
    /// 选择计划是否使用放回随机；非随机用例不填写。
    #[serde(default)]
    with_replacement: Option<bool>,
}

/// 一份 IR 结构快照。
#[derive(Debug, Deserialize)]
struct SnapshotFile {
    /// 阶段标识。
    stage: String,
    /// 快照状态。
    status: String,
    /// 用例列表。
    cases: Vec<SnapshotCase>,
}

/// 将一个 IR 语句映射为稳定结构名称。
fn statement_kind(statement: &xiao_ir::IrStatement) -> &'static str {
    match statement.kind {
        IrStatementKind::Expression { .. } => "expression",
        IrStatementKind::Assignment { .. } => "assignment",
        IrStatementKind::ExtendedAssignment { .. } => "extended-assignment",
        IrStatementKind::Declaration { .. } => "declaration",
        IrStatementKind::ConstDeclaration { .. } => "const",
        IrStatementKind::Import { .. } => "import",
        IrStatementKind::Table { .. } => "table",
        IrStatementKind::Function { .. } => "function",
        IrStatementKind::If { .. } => "if",
        IrStatementKind::For { .. } => "for",
        IrStatementKind::While { .. } => "while",
        IrStatementKind::Return { .. } => "return",
        IrStatementKind::Break => "break",
        IrStatementKind::Continue => "continue",
        IrStatementKind::Try { .. } => "try",
        IrStatementKind::Raise { .. } => "raise",
    }
}

/// 返回顶层第一个选择表达式及其规范化计划。
fn selector_summary(ir: &IrProgram) -> Option<(&IrSelector, &IrSelectionPlan)> {
    for statement in &ir.body {
        let IrStatementKind::Assignment { value, .. } = &statement.kind else {
            continue;
        };
        let IrExpressionKind::Selector {
            selector,
            selection_plan: Some(plan_id),
            ..
        } = &value.kind
        else {
            continue;
        };
        return Some((selector, ir.selection_plans.get(*plan_id as usize)?));
    }
    None
}

/// 将语法选择项转换为快照中的稳定类别。
fn selector_item_kind(item: &IrSelectorItem) -> &'static str {
    match item {
        IrSelectorItem::Exact { .. } => "exact",
        IrSelectorItem::Range { .. } => "range",
        IrSelectorItem::OpenRange { .. } => "open-range",
        IrSelectorItem::All { .. } => "all",
        IrSelectorItem::Random { .. } => "random",
    }
}

/// 将一个源码样本降低为类型化 IR。
fn lower(source_text: &str) -> xiao_ir::IrProgram {
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
    lower_program(&source, &program, &typed, &lifetime, None)
}

/// 读取并执行一份 IR 结构快照。
fn assert_snapshot(raw: &str) {
    let snapshot: SnapshotFile = serde_json::from_str(raw).expect("IR 快照 JSON 必须有效");
    assert_eq!(snapshot.stage, "08");
    assert_eq!(snapshot.status, "verified-static");
    assert!(!snapshot.cases.is_empty(), "IR 快照不应为空");

    for case in snapshot.cases {
        assert_eq!(case.expect, "success", "IR 结构快照只收录成功降低样本");
        let ir = lower(&case.source);
        assert!(
            IrValidator::new().validate(&ir).is_success(),
            "快照用例: {}",
            case.name
        );
        assert_eq!(ir.version, case.ir.version, "快照用例: {}", case.name);
        assert_eq!(ir.target, case.ir.target, "快照用例: {}", case.name);
        let entry_mode = match ir.entry_mode {
            xiao_ir::IrEntryMode::Script => "script",
            xiao_ir::IrEntryMode::Project { .. } => "project",
        };
        assert_eq!(entry_mode, case.ir.entry_mode, "快照用例: {}", case.name);
        let body_kinds = ir.body.iter().map(statement_kind).collect::<Vec<_>>();
        assert_eq!(body_kinds, case.ir.body_kinds, "快照用例: {}", case.name);
        assert_eq!(
            ir.table_signatures.len(),
            case.ir.table_signatures,
            "快照用例: {}",
            case.name
        );
        assert_eq!(
            ir.selection_plans.len(),
            case.ir.selection_plans,
            "快照用例: {}",
            case.name
        );
        if !case.ir.selector_items.is_empty() {
            let (selector, plan) = selector_summary(&ir).expect("选择器快照必须有规范计划");
            let actual_items = selector
                .items
                .iter()
                .map(selector_item_kind)
                .collect::<Vec<_>>();
            assert_eq!(
                actual_items, case.ir.selector_items,
                "快照用例: {}",
                case.name
            );
            if let Some(expected) = case.ir.selected_path_count {
                assert_eq!(
                    plan.selected_paths.len(),
                    expected,
                    "快照用例: {}",
                    case.name
                );
            }
            if let Some(expected) = case.ir.step_value {
                assert_eq!(
                    plan.step.as_ref().and_then(|step| step.value),
                    Some(expected),
                    "快照用例: {}",
                    case.name
                );
            } else {
                assert!(plan.step.is_none(), "快照用例: {} 不应带步长", case.name);
            }
            if let Some(expected) = case.ir.with_replacement {
                assert_eq!(plan.with_replacement, expected, "快照用例: {}", case.name);
            }
        }
        if case.ir.round_trip {
            let restored = from_json(&to_json(&ir).expect("IR 快照编码")).expect("IR 快照解码");
            assert_eq!(restored, ir, "快照用例: {}", case.name);
        }
    }
}

#[test]
/// 08 的 IR 结构摘要必须由真实降低、验证和快照入口读取。
fn ir_structure_snapshots_are_executed() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/08-ir/structure.json"
    )));
}
