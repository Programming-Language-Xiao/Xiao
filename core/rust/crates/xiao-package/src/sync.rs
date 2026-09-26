//! 本地包图到锁文件和环境映射的单一编排入口。

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_config::{ConfigDocument, parse_config_project};
use xiao_source::SourceFile;

use crate::cache::{CacheLayout, CacheStore};
use crate::config_edit::{DependencyEdit, edit_dependency, write_config_edit};
use crate::diagnostics::{SYNC_ENVIRONMENT_CODE, SYNC_INVALID_INPUT_CODE, SYNC_LOCK_REQUIRED_CODE};
use crate::entry_lock::EntryLock;
use crate::environment::{
    ENVIRONMENT_METADATA_FILE, EnvironmentLayout, build_environment_metadata_with_mappings,
    materialize_environment_with_mappings, read_environment_metadata, update_environment_metadata,
};
use crate::lockfile::{
    LockFileWriteStatus, build_lockfile, compare_lockfile, lockfile_path, read_lockfile,
    write_lockfile,
};
use crate::mapping::{
    PackageObjectMapping, materialize_package_mappings, validate_package_mappings,
};
use crate::resolver::{resolve_project, resolve_project_with_document};
use crate::version::{Version, VersionRequirement};

/// 用户选择的包操作。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageOperation {
    /// 同步本地依赖并激活环境。
    Sync {
        /// 保留目标环境额外映射。
        keep_extra: bool,
        /// 要求锁文件与当前图一致。
        locked: bool,
        /// 仅消费现有锁文件。
        frozen: bool,
    },
    /// 仅消费已有锁文件，保持多余映射。
    Install,
    /// 首次建立锁文件；存在锁时只验证而不升级。
    Lock,
    /// 显式重解配置并改写锁文件，不物化环境。
    Update,
}

/// 一次包操作的可序列化摘要。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageOperationResult {
    /// 目标环境绝对路径；锁操作不写入环境。
    pub environment_path: PathBuf,
    /// 是否新建项目环境或全局映射容器。
    pub created: bool,
    /// 实际映射是否发生变化。
    pub changed: bool,
    /// 是否需由父 Shell 激活。
    pub activate: bool,
    /// 锁文件状态；安装时为空。
    pub lock_status: Option<String>,
}

/// 包操作的稳定诊断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageSyncError {
    /// 稳定诊断编号。
    pub code: String,
    /// 用户可读的原因。
    pub message: String,
}

impl std::fmt::Display for PackageSyncError {
    /// 输出带有稳定诊断编号的包操作错误。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PackageSyncError {}

/// 将下层错误保留为可交给驱动器的包操作诊断。
fn failure(code: &str, message: impl ToString) -> PackageSyncError {
    PackageSyncError {
        code: code.to_owned(),
        message: message.to_string(),
    }
}

/// 仅枚举已保存的映射；不加载或求值包源码。
pub fn environment_package_view(
    path: impl AsRef<Path>,
) -> Result<Vec<PackageObjectMapping>, PackageSyncError> {
    let metadata = read_environment_metadata(path.as_ref().join(ENVIRONMENT_METADATA_FILE))
        .map_err(|error| failure(error.code(), error))?;
    validate_package_mappings(&metadata.package_mappings)
        .map_err(|error| failure(error.code(), error))?;
    Ok(metadata.package_mappings)
}

/// 在完成静态解析及本地图预校验后提交依赖配置，并重解、更新锁文件。
pub fn apply_dependency_edit(
    project_root: &Path,
    document: &ConfigDocument,
    original: &str,
    edit: &DependencyEdit,
    cache_layout: CacheLayout,
) -> Result<PackageOperationResult, PackageSyncError> {
    if !project_root.is_absolute() {
        return Err(failure(SYNC_INVALID_INPUT_CODE, "项目根目录必须是绝对路径"));
    }
    let config_path = project_root.join("config.xiao");
    let _guard = acquire_operation_lock(project_root, &cache_layout)?;
    let edited = edit_dependency(document, original, edit)?;
    let next_document = parse_config_project(&SourceFile::from_text(&edited))
        .map_err(|_| failure(SYNC_INVALID_INPUT_CODE, "拟写回配置不合法"))?;
    let resolution = resolve_project_with_document(project_root, &next_document);
    let graph = valid_resolution(resolution)?;
    validate_local_versions(&graph)?;
    let lock_path = lockfile_path(project_root);
    if lock_path.exists() {
        read_lockfile(&lock_path).map_err(|error| failure(error.code(), error))?;
    }
    let cache = CacheStore::open(cache_layout).map_err(|error| failure(error.code(), error))?;
    write_config_edit(&config_path, original, &edited)?;
    let write_result = (|| {
        let candidate = build_lockfile(&graph, &next_document, &cache)
            .map_err(|error| failure(error.code(), error))?;
        if fs::read_to_string(&config_path).ok().as_deref() != Some(&edited) {
            return Err(failure(
                SYNC_INVALID_INPUT_CODE,
                "config.xiao 在求解后发生变化",
            ));
        }
        write_lockfile(&lock_path, &candidate).map_err(|error| failure(error.code(), error))
    })();
    let status = match write_result {
        Ok(status) => status,
        Err(error) => {
            if let Err(rollback) = write_config_edit(&config_path, &edited, original) {
                return Err(failure(
                    rollback.code.as_str(),
                    "锁定失败且配置回滚失败；请手动检查 config.xiao 与 xiao.lock.json",
                ));
            }
            return Err(error);
        }
    };
    Ok(PackageOperationResult {
        environment_path: EnvironmentLayout::default_for_project(project_root).path,
        created: false,
        changed: false,
        activate: false,
        lock_status: Some(lock_status_name(status).to_owned()),
    })
}

fn acquire_operation_lock(
    project_root: &Path,
    cache_layout: &CacheLayout,
) -> Result<EntryLock, PackageSyncError> {
    if !project_root.is_dir() {
        return Err(failure(SYNC_INVALID_INPUT_CODE, "项目根目录不存在"));
    }
    let canonical = fs::canonicalize(project_root)
        .map_err(|_| failure(SYNC_INVALID_INPUT_CODE, "项目根目录不可用"))?;
    let digest = Sha256::digest(canonical.to_string_lossy().as_bytes());
    let lock_path = cache_layout
        .cache_root()
        .join("locks/package-operations")
        .join(format!("{digest:x}.lock"));
    EntryLock::acquire(&lock_path).map_err(|error| failure(error.code, "无法获取项目包操作锁"))
}

fn valid_resolution(
    resolution: crate::model::PackageResolution,
) -> Result<crate::model::PackageGraph, PackageSyncError> {
    if let Some(diagnostic) = resolution.diagnostics.first() {
        return Err(failure(
            diagnostic.diagnostic.code(),
            diagnostic.diagnostic.message(),
        ));
    }
    Ok(resolution.graph)
}

fn validate_local_versions(graph: &crate::model::PackageGraph) -> Result<(), PackageSyncError> {
    for node in graph.nodes.values() {
        for dependency in node.dependencies.values() {
            if let Some(reference) = &dependency.source {
                let target = dependency
                    .target
                    .as_ref()
                    .ok_or_else(|| failure(SYNC_INVALID_INPUT_CODE, "本地依赖尚未解析完成"))?;
                if reference != &target.source.source_id
                    && target.source.alias.as_ref() != Some(reference)
                {
                    return Err(failure(
                        crate::diagnostics::SOURCE_UNKNOWN_REFERENCE_CODE,
                        format!("本地依赖 {:?} 的显式源与目标来源不符", dependency.name),
                    ));
                }
            }
            if let Some(constraint) = &dependency.version {
                let requirement = VersionRequirement::parse(constraint)
                    .map_err(|error| failure(error.code, error))?;
                let target = dependency
                    .target
                    .as_ref()
                    .ok_or_else(|| failure(SYNC_INVALID_INPUT_CODE, "本地依赖尚未解析完成"))?;
                let version =
                    Version::parse(&target.version).map_err(|error| failure(error.code, error))?;
                if !requirement.matches(&version) {
                    return Err(failure(
                        crate::diagnostics::VERSION_UNSATISFIED_CODE,
                        format!("本地依赖 {:?} 的版本不满足约束", dependency.name),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// 求解、锁定并更新项目或全局环境；目标选择只在这里发生。
#[allow(clippy::too_many_arguments)]
pub fn apply_packages(
    project_root: &Path,
    active_environment: Option<&Path>,
    document: &ConfigDocument,
    toolchain: &Toolchain,
    target: &TargetDescription,
    operation: PackageOperation,
    cache_layout: CacheLayout,
) -> Result<PackageOperationResult, PackageSyncError> {
    if matches!(
        operation,
        PackageOperation::Sync {
            locked: true,
            frozen: true,
            ..
        }
    ) {
        return Err(failure(
            SYNC_INVALID_INPUT_CODE,
            "--locked 与 --frozen 不可同时使用",
        ));
    }
    if !project_root.is_absolute() {
        return Err(failure(SYNC_INVALID_INPUT_CODE, "项目根目录必须是绝对路径"));
    }
    let _guard = acquire_operation_lock(project_root, &cache_layout)?;
    let config_path = project_root.join("config.xiao");
    let current = fs::read_to_string(&config_path)
        .map_err(|_| failure(SYNC_INVALID_INPUT_CODE, "读取 config.xiao 失败"))?;
    let current_document = parse_config_project(&SourceFile::from_text(&current))
        .map_err(|_| failure(SYNC_INVALID_INPUT_CODE, "config.xiao 未通过静态校验"))?;
    if &current_document != document {
        return Err(failure(
            SYNC_INVALID_INPUT_CODE,
            "config.xiao 已被修改，请重新执行包操作",
        ));
    }
    let active = active_environment
        .filter(|_| !matches!(operation, PackageOperation::Lock | PackageOperation::Update))
        .map(|path| {
            if !path.is_absolute()
                || path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                Err(failure(
                    SYNC_INVALID_INPUT_CODE,
                    "XIAO_ACTIVE_ENV 必须是规范的绝对路径",
                ))
            } else {
                Ok(path.to_path_buf())
            }
        })
        .transpose()?;
    let has_active_environment = active.is_some();
    let default = EnvironmentLayout::default_for_project(project_root).path;
    let environment_path = match operation {
        PackageOperation::Sync { .. } => active.unwrap_or_else(|| default.clone()),
        PackageOperation::Install => match active {
            Some(path) => path,
            None => cache_layout
                .global_environment_path("global")
                .map_err(|error| failure(error.code(), error))?,
        },
        PackageOperation::Lock | PackageOperation::Update => default.clone(),
    };
    if matches!(operation, PackageOperation::Install)
        && has_active_environment
        && !environment_path.exists()
    {
        return Err(failure(
            SYNC_ENVIRONMENT_CODE,
            "激活环境不存在；install 不创建项目环境",
        ));
    }
    let graph = valid_resolution(resolve_project(project_root))?;
    validate_local_versions(&graph)?;
    let lock_path = lockfile_path(project_root);
    let existing = match read_lockfile(&lock_path) {
        Ok(lock) => Some(lock),
        Err(crate::lockfile::LockfileError::Io { .. }) if !lock_path.exists() => None,
        Err(error) => return Err(failure(error.code(), error)),
    };
    if matches!(operation, PackageOperation::Lock | PackageOperation::Update) {
        let cache = CacheStore::open(cache_layout).map_err(|error| failure(error.code(), error))?;
        let status = if let (PackageOperation::Lock, Some(lock)) = (operation, existing.as_ref()) {
            compare_lockfile(lock, &graph, document)
                .map_err(|error| failure(error.code(), error))?;
            LockFileWriteStatus::Reused
        } else {
            let candidate = build_lockfile(&graph, document, &cache)
                .map_err(|error| failure(error.code(), error))?;
            write_lockfile(&lock_path, &candidate).map_err(|error| failure(error.code(), error))?
        };
        return Ok(PackageOperationResult {
            environment_path,
            created: false,
            changed: false,
            activate: false,
            lock_status: Some(lock_status_name(status).to_owned()),
        });
    }
    let must_match = !matches!(
        operation,
        PackageOperation::Sync {
            locked: false,
            frozen: false,
            ..
        }
    );
    if must_match {
        let lock = existing
            .as_ref()
            .ok_or_else(|| failure(SYNC_LOCK_REQUIRED_CODE, "锁文件缺失；请先执行 xiao sync"))?;
        compare_lockfile(lock, &graph, document).map_err(|error| failure(error.code(), error))?;
    }
    let cache = CacheStore::open(cache_layout).map_err(|error| failure(error.code(), error))?;
    let lock_status = if must_match {
        None
    } else {
        let candidate = build_lockfile(&graph, document, &cache)
            .map_err(|error| failure(error.code(), error))?;
        let status =
            write_lockfile(&lock_path, &candidate).map_err(|error| failure(error.code(), error))?;
        Some(lock_status_name(status).to_owned())
    };
    let mappings = materialize_package_mappings(&graph, &cache)
        .map_err(|error| failure(error.code(), error))?;
    let metadata_path = environment_path.join(ENVIRONMENT_METADATA_FILE);
    let created = !environment_path.exists();
    if created {
        if matches!(operation, PackageOperation::Install) {
            fs::create_dir_all(environment_path.parent().expect("全局环境有父目录"))
                .map_err(|error| failure(SYNC_ENVIRONMENT_CODE, error))?;
        }
        let layout = if environment_path == default {
            EnvironmentLayout::default_for_project(project_root)
        } else {
            let name = environment_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| failure(SYNC_INVALID_INPUT_CODE, "环境名称不可用"))?;
            EnvironmentLayout::named_for_project(
                environment_path.parent().expect("绝对环境有父目录"),
                name,
            )
            .map_err(|error| failure(SYNC_INVALID_INPUT_CODE, error))?
        };
        materialize_environment_with_mappings(
            layout.path.parent().expect("环境有父目录"),
            if layout.directory_name == ".venv" {
                None
            } else {
                Some(&layout.logical_name)
            },
            document,
            toolchain,
            target,
            mappings.clone(),
        )
        .map_err(|error| failure(SYNC_ENVIRONMENT_CODE, error))?;
    }
    let current = read_environment_metadata(&metadata_path)
        .map_err(|error| failure(SYNC_ENVIRONMENT_CODE, error))?;
    let mut merged = mappings;
    if matches!(
        operation,
        PackageOperation::Install
            | PackageOperation::Sync {
                keep_extra: true,
                ..
            }
    ) {
        for mapping in &current.package_mappings {
            if !merged.iter().any(|entry| entry.package == mapping.package) {
                merged.push(mapping.clone());
            }
        }
        merged.sort_by(|left, right| left.package.cmp(&right.package));
    }
    validate_package_mappings(&merged).map_err(|error| failure(error.code(), error))?;
    let layout = EnvironmentLayout {
        logical_name: current.logical_name.clone(),
        directory_name: current.directory_name.clone(),
        path: environment_path.clone(),
    };
    let mut next =
        build_environment_metadata_with_mappings(&layout, document, toolchain, target, merged);
    next.lockfile_summary = current.lockfile_summary.clone();
    let changed = created || current != next;
    if !created && changed {
        update_environment_metadata(&metadata_path, &next)
            .map_err(|error| failure(SYNC_ENVIRONMENT_CODE, error))?;
    }
    Ok(PackageOperationResult {
        environment_path,
        created,
        changed,
        activate: matches!(operation, PackageOperation::Sync { .. }),
        lock_status,
    })
}

fn lock_status_name(status: LockFileWriteStatus) -> &'static str {
    match status {
        LockFileWriteStatus::Created => "created",
        LockFileWriteStatus::Updated => "updated",
        LockFileWriteStatus::Reused => "reused",
    }
}
