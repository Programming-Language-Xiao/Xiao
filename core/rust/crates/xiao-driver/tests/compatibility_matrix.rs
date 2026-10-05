//! 19B 兼容矩阵逐格回归。

use xiao_driver::{
    CompatibilityAction, CompatibilityAxis, compatibility_matrix, current_compatibility_versions,
};

#[test]
/// 每格都必须携带版本标签和四选一行为。
fn every_generated_cell_has_an_explicit_action_and_stable_axis() {
    let cells = compatibility_matrix();
    assert_eq!(cells.len(), 18);
    for cell in &cells {
        assert!(!cell.artifact.is_empty());
        assert!(!cell.runtime.is_empty());
        assert!(!cell.axis.as_str().is_empty());
        match &cell.action {
            CompatibilityAction::Reject { code } => assert!(!code.is_empty()),
            CompatibilityAction::Direct
            | CompatibilityAction::Regenerate
            | CompatibilityAction::Migrate => {}
        }
    }
}

#[test]
/// 当前版本直接运行，优化指纹变化重新生成。
fn current_version_cells_are_direct_and_mismatches_are_explicit() {
    let versions = current_compatibility_versions();
    let xiaoc = format!("{}.{}", versions.xiaoc.0, versions.xiaoc.1);
    assert!(compatibility_matrix().iter().any(|cell| {
        cell.axis == CompatibilityAxis::XiaocFormat
            && cell.artifact == xiaoc
            && cell.runtime == xiaoc
            && cell.action == CompatibilityAction::Direct
    }));
    assert!(compatibility_matrix().iter().any(|cell| {
        cell.axis == CompatibilityAxis::OptimizationFingerprint
            && cell.action == CompatibilityAction::Regenerate
    }));
}
