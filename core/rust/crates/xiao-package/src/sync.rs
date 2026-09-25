//! 本地包图到锁文件和环境映射的单一编排入口。

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_config::ConfigDocument;

use crate::cache::{CacheLayout, CacheStore};
use crate::diagnostics::{SYNC_ENVIRONMENT_CODE, SYNC_INVALID_INPUT_CODE, SYNC_LOCK_REQUIRED_CODE};
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
use crate::resolver::resolve_project;

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
}

/// 一次包操作的可序列化摘要。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageOperationResult {
    /// 被写入的绝对环境路径。
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
    let active = active_environment
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
    let resolution = resolve_project(project_root);
    if !resolution.is_success() {
        let diagnostic = resolution
            .diagnostics
            .first()
            .expect("失败的包解析必须有诊断");
        return Err(failure(
            diagnostic.diagnostic.code(),
            diagnostic.diagnostic.message(),
        ));
    }
    let graph = resolution.graph;
    let lock_path = lockfile_path(project_root);
    let existing = match read_lockfile(&lock_path) {
        Ok(lock) => Some(lock),
        Err(crate::lockfile::LockfileError::Io { .. }) if !lock_path.exists() => None,
        Err(error) => return Err(failure(error.code(), error)),
    };
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
        Some(
            match status {
                LockFileWriteStatus::Created => "created",
                LockFileWriteStatus::Updated => "updated",
                LockFileWriteStatus::Reused => "reused",
            }
            .to_owned(),
        )
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
