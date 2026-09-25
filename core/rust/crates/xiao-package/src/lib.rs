//! Xiao 包源、依赖求解和锁定逻辑的 crate 入口。

/// 离线目录源与传输适配器接口。
mod adapters;
/// 不可变源码对象缓存和摘要校验。
mod cache;
/// 11A-D1 包粒度的稳定诊断编号。
mod diagnostics;
/// 缓存条目的跨进程排他创建与陈旧锁回收。
mod entry_lock;
/// 项目环境布局、指纹和元数据物化。
mod environment;
/// 离线快速路径、三态回退与有界包源并行。
mod fastpath;
/// 包源快照及联邦索引。
mod federation;
/// 联邦索引覆盖缓存与跨源去重的包元数据。
mod federation_cache;
/// RFC 8785 JSON 规范化。
mod jcs;
/// 11A-E2A 锁文件、过期判据和跨平台原子文件提交。
mod lockfile;
/// 环境到缓存对象的逻辑映射。
mod mapping;
/// 包身份、来源预留和确定性内存图模型。
mod model;
/// 本地路径包配置读取和递归依赖解析。
mod resolver;
/// 跨源选择的确定性纯逻辑。
mod selection;
/// 按源身份存放不可变快照与当前指向。
mod snapshot_store;
/// 离线源身份、声明与清单展开。
mod source;
/// E2B 本地依赖同步、安装与只读包视图。
mod sync;

/// 重导出统一索引与正文读取边界。
pub use adapters::{IndexSnapshot, LocalDirectoryAdapter, PackageSourceAdapter};
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
    update_environment_mappings, update_environment_metadata,
};
/// 重导出多源缓存解析编排入口与状态报告。
pub use fastpath::{MAX_PARALLEL_SOURCES, SourceResolution, SourceResolver, SourceStatusReport};
/// 重导出离线快照与合并契约。
pub use federation::{
    ArtifactReference, FederatedRecord, FederationKey, IndexDependency, IndexPackage, PackageShard,
    SnapshotManifest, SnapshotStatus, SourceListFingerprint, SourceSequenceEntry, SourceSnapshot,
    federate, source_list_fingerprint,
};
/// 重导出可审计的联邦索引与包元数据缓存接口。
pub use federation_cache::{FederationCache, FederationIndex, MetadataCache};
/// 重导出统一 JSON 规范化入口。
pub use jcs::{canonicalize_json, jcs_digest};
/// 重导出锁文件和原子更新接口。
pub use lockfile::{
    LOCKFILE_NAME, LOCKFILE_VERSION, LockFile, LockFileWriteStatus, LockedDependency,
    LockedPackage, LockedSourceSnapshot, LockfileError, LockfileMismatch, build_lockfile,
    compare_lockfile, generate_or_reuse_lockfile, lockfile_path, read_lockfile, validate_lockfile,
    write_lockfile,
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
/// 重导出跨源选择入口。
pub use selection::select_source;
/// 重导出已验证的可复用源快照存储。
pub use snapshot_store::{SnapshotStore, StoredSnapshot};
/// 重导出离线源契约。
pub use source::{
    ConfiguredSource, SOURCE_PROTOCOL_VERSION, SourceDeclaration, SourceDescriptor, SourceError,
    SourceList, SourceListImport, expand_source_lists, source_declarations, source_id,
};
/// 重导出同步与安装编排入口。
pub use sync::{
    PackageOperation, PackageOperationResult, PackageSyncError, apply_packages,
    environment_package_view,
};
