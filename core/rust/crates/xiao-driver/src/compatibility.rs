//! 19B 兼容矩阵：把现有版本边界集中成可测试、可报告的数据。

use serde::{Deserialize, Serialize};

/// 矩阵中一个版本轴的稳定名称。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CompatibilityAxis {
    /// `.xiaoc` 文件格式。
    XiaocFormat,
    /// 内容寻址索引 Schema。
    IndexSchema,
    /// `.xar` 容器格式。
    XarFormat,
    /// Runtime ABI 区间。
    RuntimeAbi,
    /// 类型化 IR 版本。
    IrVersion,
    /// Xiao 语言语义版本。
    LanguageVersion,
    /// 优化配置指纹。
    OptimizationFingerprint,
    /// 目标平台约束。
    TargetPlatform,
}

impl CompatibilityAxis {
    /// 返回文档和报告中使用的稳定名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::XiaocFormat => "xiaoc_format",
            Self::IndexSchema => "index_schema",
            Self::XarFormat => "xar_format",
            Self::RuntimeAbi => "runtime_abi",
            Self::IrVersion => "ir_version",
            Self::LanguageVersion => "language_version",
            Self::OptimizationFingerprint => "optimization_fingerprint",
            Self::TargetPlatform => "target_platform",
        }
    }
}

/// 旧产物面对当前核心时的冻结行为。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CompatibilityAction {
    /// 当前核心直接消费产物。
    Direct,
    /// 当前核心丢弃旧对象并重新生成。
    Regenerate,
    /// 当前核心先执行迁移再消费。
    Migrate,
    /// 当前核心拒绝产物，并返回稳定错误码。
    Reject {
        /// 稳定拒绝码。
        code: String,
    },
}

/// 兼容矩阵中的一格。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompatibilityCell {
    /// 版本轴。
    pub axis: CompatibilityAxis,
    /// 产物侧标签。
    pub artifact: String,
    /// 当前核心侧标签。
    pub runtime: String,
    /// 当前实现的行为。
    pub action: CompatibilityAction,
}

/// 当前核心版本清单，供矩阵、发布报告和文档生成复用。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompatibilityVersions {
    /// 语言语义版本。
    pub language: String,
    /// IR 版本。
    pub ir: u32,
    /// `.xiaoc` 格式主/次版本。
    pub xiaoc: (u16, u16),
    /// 内容寻址索引主/次版本。
    pub index_schema: (u32, u32),
    /// `.xar` 格式主/次版本。
    pub xar: (u16, u16),
    /// Runtime ABI 编码版本。
    pub runtime_abi: u64,
    /// 目标平台标签。
    pub target: String,
}

/// 返回由当前 crate 常量生成的版本清单。
#[must_use]
pub fn current_compatibility_versions() -> CompatibilityVersions {
    CompatibilityVersions {
        language: "0.1.0".to_owned(),
        ir: xiao_ir::IR_VERSION,
        xiaoc: (
            xiao_bytecode::XIAOC_FORMAT_MAJOR,
            xiao_bytecode::XIAOC_FORMAT_MINOR,
        ),
        index_schema: (
            xiao_artifacts::INDEX_SCHEMA_MAJOR,
            xiao_artifacts::INDEX_SCHEMA_MINOR,
        ),
        xar: (xiao_xar::XAR_FORMAT_MAJOR, xiao_xar::XAR_FORMAT_MINOR),
        runtime_abi: xiao_runtime_abi::ABI_ENCODED_VERSION,
        target: "portable".to_owned(),
    }
}

/// 生成当前实现的完整兼容矩阵。
///
/// 这里只登记已有行为：`.xiaoc`/索引的版本漂移和 ABI/IR/平台不匹配会拒绝，
/// 支持范围内的次版本直接运行，缓存指纹变化重新生成。当前 `.xar` 没有独立写入
/// 的容器版本字段，只有索引 Schema/Runtime ABI 会执行版本拒绝；语言版本也沿用
/// 现有“由请求上下文决定”的无门控行为。迁移窗口尚未冻结，因此不虚构 `Migrate` 格子。
#[must_use]
pub fn compatibility_matrix() -> Vec<CompatibilityCell> {
    let current = current_compatibility_versions();
    vec![
        cell(
            CompatibilityAxis::XiaocFormat,
            format!("{}.{}", current.xiaoc.0, current.xiaoc.1),
            format!("{}.{}", current.xiaoc.0, current.xiaoc.1),
            CompatibilityAction::Direct,
        ),
        cell(
            CompatibilityAxis::XiaocFormat,
            format!("{}.{}", current.xiaoc.0, current.xiaoc.1 + 1),
            format!("{}.{}", current.xiaoc.0, current.xiaoc.1),
            CompatibilityAction::Reject {
                code: "XIAOC-003".to_owned(),
            },
        ),
        cell(
            CompatibilityAxis::XiaocFormat,
            format!("{}.{}", current.xiaoc.0 + 1, current.xiaoc.1),
            format!("{}.{}", current.xiaoc.0, current.xiaoc.1),
            CompatibilityAction::Reject {
                code: "XIAOC-002".to_owned(),
            },
        ),
        cell(
            CompatibilityAxis::IndexSchema,
            format!("{}.{}", current.index_schema.0, current.index_schema.1),
            format!("{}.{}", current.index_schema.0, current.index_schema.1),
            CompatibilityAction::Direct,
        ),
        cell(
            CompatibilityAxis::IndexSchema,
            format!("{}.{}", current.index_schema.0, current.index_schema.1 + 1),
            format!("{}.{}", current.index_schema.0, current.index_schema.1),
            CompatibilityAction::Reject {
                code: "X17-XAR-007".to_owned(),
            },
        ),
        cell(
            CompatibilityAxis::IndexSchema,
            format!("{}.{}", current.index_schema.0 + 1, current.index_schema.1),
            format!("{}.{}", current.index_schema.0, current.index_schema.1),
            CompatibilityAction::Reject {
                code: "X17-XAR-007".to_owned(),
            },
        ),
        cell(
            CompatibilityAxis::XarFormat,
            "implicit-v1".to_owned(),
            format!("{}.{}", current.xar.0, current.xar.1),
            CompatibilityAction::Direct,
        ),
        cell(
            CompatibilityAxis::XarFormat,
            "zip64-v1".to_owned(),
            format!("{}.{}", current.xar.0, current.xar.1),
            CompatibilityAction::Direct,
        ),
        cell(
            CompatibilityAxis::RuntimeAbi,
            current.runtime_abi.to_string(),
            current.runtime_abi.to_string(),
            CompatibilityAction::Direct,
        ),
        cell(
            CompatibilityAxis::RuntimeAbi,
            "outside-supported-range".to_owned(),
            current.runtime_abi.to_string(),
            CompatibilityAction::Reject {
                code: "X17-XAR-007".to_owned(),
            },
        ),
        cell(
            CompatibilityAxis::IrVersion,
            current.ir.to_string(),
            current.ir.to_string(),
            CompatibilityAction::Direct,
        ),
        cell(
            CompatibilityAxis::IrVersion,
            (current.ir + 1).to_string(),
            current.ir.to_string(),
            CompatibilityAction::Reject {
                code: "X08-IR-002".to_owned(),
            },
        ),
        cell(
            CompatibilityAxis::LanguageVersion,
            current.language.clone(),
            current.language.clone(),
            CompatibilityAction::Direct,
        ),
        cell(
            CompatibilityAxis::LanguageVersion,
            "0.2.0".to_owned(),
            current.language.clone(),
            CompatibilityAction::Direct,
        ),
        cell(
            CompatibilityAxis::OptimizationFingerprint,
            "same".to_owned(),
            "same".to_owned(),
            CompatibilityAction::Direct,
        ),
        cell(
            CompatibilityAxis::OptimizationFingerprint,
            "different".to_owned(),
            "current".to_owned(),
            CompatibilityAction::Regenerate,
        ),
        cell(
            CompatibilityAxis::TargetPlatform,
            current.target.clone(),
            current.target,
            CompatibilityAction::Direct,
        ),
        cell(
            CompatibilityAxis::TargetPlatform,
            "x86_64-unknown-linux-gnu".to_owned(),
            "x86_64-pc-windows-msvc".to_owned(),
            CompatibilityAction::Reject {
                code: "X17-XAR-008".to_owned(),
            },
        ),
    ]
}

/// 创建一格兼容矩阵数据。
fn cell(
    axis: CompatibilityAxis,
    artifact: String,
    runtime: String,
    action: CompatibilityAction,
) -> CompatibilityCell {
    CompatibilityCell {
        axis,
        artifact,
        runtime,
        action,
    }
}

#[cfg(test)]
/// 兼容矩阵自身的逐轴不变量测试。
mod tests {
    use super::{
        CompatibilityAction, CompatibilityAxis, compatibility_matrix,
        current_compatibility_versions,
    };

    #[test]
    /// 当前版本必须至少有一个直接消费格。
    fn matrix_is_generated_from_current_versions_and_has_every_axis() {
        let current = current_compatibility_versions();
        let cells = compatibility_matrix();
        assert!(!cells.is_empty());
        for axis in [
            CompatibilityAxis::XiaocFormat,
            CompatibilityAxis::IndexSchema,
            CompatibilityAxis::XarFormat,
            CompatibilityAxis::RuntimeAbi,
            CompatibilityAxis::IrVersion,
            CompatibilityAxis::LanguageVersion,
            CompatibilityAxis::OptimizationFingerprint,
            CompatibilityAxis::TargetPlatform,
        ] {
            assert!(cells.iter().any(|cell| cell.axis == axis), "缺少 {axis:?}");
        }
        let current_xiaoc = format!("{}.{}", current.xiaoc.0, current.xiaoc.1);
        assert!(cells.iter().any(|cell| {
            cell.axis == CompatibilityAxis::XiaocFormat
                && cell.artifact == current_xiaoc
                && cell.runtime == current_xiaoc
                && cell.action == CompatibilityAction::Direct
        }));
    }

    #[test]
    /// 未冻结迁移窗口不能被矩阵误报为已支持。
    fn matrix_contains_no_unfrozen_migration_claim() {
        assert!(
            compatibility_matrix()
                .iter()
                .all(|cell| !matches!(cell.action, CompatibilityAction::Migrate))
        );
    }
}
