//! Xiao 包源、依赖求解和锁定逻辑的 crate 入口。

/// 11A-D1 包粒度的稳定诊断编号。
mod diagnostics;
/// 项目环境布局、指纹和元数据物化。
mod environment;
/// 包身份、来源预留和确定性内存图模型。
mod model;
/// 本地路径包配置读取和递归依赖解析。
mod resolver;

/// 重导出包解析诊断编号。
pub use diagnostics::*;
/// 重导出项目环境接口。
pub use environment::{
    ENVIRONMENT_METADATA_FILE, EnvironmentCreationError, EnvironmentError, EnvironmentFingerprint,
    EnvironmentLayout, EnvironmentMetadata, build_environment_metadata, fingerprint_config,
    fingerprint_environment, fingerprint_target, materialize_environment,
};
/// 重导出包身份和图模型。
pub use model::{
    PackageDependency, PackageDiagnostic, PackageEdge, PackageGraph, PackageIdentity, PackageNode,
    PackageResolution, PackageSource, SourceIdentity, local_source_id,
};
/// 重导出本地路径解析入口。
pub use resolver::{PackageResolver, resolve_path_dependencies, resolve_project};
