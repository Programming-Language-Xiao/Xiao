//! Xiao 包源、依赖求解和锁定逻辑的 crate 入口。

/// 不可变源码对象缓存和摘要校验。
mod cache;
/// 11A-D1 包粒度的稳定诊断编号。
mod diagnostics;
/// 项目环境布局、指纹和元数据物化。
mod environment;
/// 环境到缓存对象的逻辑映射。
mod mapping;
/// 包身份、来源预留和确定性内存图模型。
mod model;
/// 本地路径包配置读取和递归依赖解析。
mod resolver;

/// 重导出本地源码缓存接口。
pub use cache::{
    CacheError, CacheLayout, CacheObject, CacheObjectKind, CacheObjectReference, CacheStore,
    SOURCE_OBJECT_ALGORITHM, SOURCE_OBJECT_KIND, XIAO_HOME_ENV, source_directory_digest,
};
/// 重导出包解析诊断编号。
pub use diagnostics::*;
/// 重导出项目环境接口。
pub use environment::{
    ENVIRONMENT_METADATA_FILE, ENVIRONMENT_METADATA_VERSION, EnvironmentCreationError,
    EnvironmentError, EnvironmentFingerprint, EnvironmentLayout, EnvironmentMetadata,
    EnvironmentPackageError, build_environment_metadata, build_environment_metadata_with_mappings,
    fingerprint_config, fingerprint_environment, fingerprint_target, materialize_environment,
    materialize_environment_from_graph, materialize_environment_with_mappings,
    materialize_global_environment_from_graph, read_environment_metadata,
};
/// 重导出环境映射接口。
pub use mapping::{
    MappingError, PackageObjectMapping, materialize_package_mappings, resolve_package_object,
    validate_package_mappings,
};
/// 重导出包身份和图模型。
pub use model::{
    PackageDependency, PackageDiagnostic, PackageEdge, PackageGraph, PackageIdentity, PackageNode,
    PackageResolution, PackageSource, SourceIdentity, local_source_id,
};
/// 重导出本地路径解析入口。
pub use resolver::{PackageResolver, resolve_path_dependencies, resolve_project};
