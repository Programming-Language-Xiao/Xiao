//! 环境到全局源码对象的只读逻辑映射。
//!
//! 映射只保存包逻辑身份和对象引用，不在项目环境目录创建符号链接、硬链接或源码副本。
//! 包身份沿用 D1 的 [`crate::model::PackageIdentity`]，对象内容由 [`crate::cache::CacheStore`]
//! 负责导入和校验。

use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::cache::{CacheError, CacheObject, CacheObjectKind, CacheObjectReference, CacheStore};
use crate::environment::EnvironmentMetadata;
use crate::model::{PackageGraph, PackageIdentity};

/// 一个包逻辑身份到不可变源码对象的映射项。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageObjectMapping {
    /// D1 的完整包身份，不能退化成包名。
    pub package: PackageIdentity,
    /// 全局缓存对象引用。
    pub object: CacheObjectReference,
}

/// 一次环境映射构造或读取失败。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MappingError {
    /// 包图中出现重复身份。
    DuplicatePackage {
        /// 重复的包身份。
        package: PackageIdentity,
    },
    /// 环境映射中找不到请求的包身份。
    MissingPackage {
        /// 请求的包身份。
        package: PackageIdentity,
    },
    /// 映射引用了当前批次不支持的对象类型。
    UnsupportedObjectKind {
        /// 相关包身份。
        package: PackageIdentity,
        /// 不支持的对象类型。
        object_kind: String,
    },
    /// 映射中的包身份没有按确定顺序排列。
    UnstableOrder,
    /// 底层缓存导入或校验失败。
    Cache {
        /// 底层缓存错误。
        error: CacheError,
    },
}

impl MappingError {
    /// 返回底层缓存错误，其他映射错误返回自身的稳定名称。
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Cache { error } => error.code(),
            Self::DuplicatePackage { .. } => "X05-CACHE-005",
            Self::MissingPackage { .. } => "X05-CACHE-006",
            Self::UnsupportedObjectKind { .. } => "X05-CACHE-007",
            Self::UnstableOrder => "X05-CACHE-008",
        }
    }
}

impl Display for MappingError {
    /// 输出带稳定编号的映射诊断。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePackage { package } => {
                write!(
                    formatter,
                    "{}: 环境映射包含重复包身份：{package}",
                    self.code()
                )
            }
            Self::MissingPackage { package } => {
                write!(formatter, "{}: 环境映射缺少包身份：{package}", self.code())
            }
            Self::UnsupportedObjectKind {
                package,
                object_kind,
            } => write!(
                formatter,
                "{}: 包 {package} 引用了不支持的缓存对象类型：{object_kind}",
                self.code()
            ),
            Self::UnstableOrder => write!(formatter, "{}: 环境映射顺序不稳定", self.code()),
            Self::Cache { error } => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for MappingError {}

impl From<CacheError> for MappingError {
    /// 将缓存错误保留在映射错误中。
    fn from(error: CacheError) -> Self {
        Self::Cache { error }
    }
}

/// 按 D1 包图的确定性节点顺序导入所有源码对象并生成环境映射。
#[allow(clippy::result_large_err)]
pub fn materialize_package_mappings(
    graph: &PackageGraph,
    cache: &CacheStore,
) -> Result<Vec<PackageObjectMapping>, MappingError> {
    let mut mappings = Vec::with_capacity(graph.nodes.len());
    for (identity, node) in &graph.nodes {
        let object = cache.import_source_directory(&node.root)?;
        mappings.push(PackageObjectMapping {
            package: identity.clone(),
            object: object.reference,
        });
    }
    validate_package_mappings(&mappings)?;
    Ok(mappings)
}

/// 验证映射唯一性、排序和当前批次的对象类型。
#[allow(clippy::result_large_err)]
pub fn validate_package_mappings(mappings: &[PackageObjectMapping]) -> Result<(), MappingError> {
    let mut previous = None;
    for mapping in mappings {
        if let Some(previous_package) = previous {
            if previous_package >= &mapping.package {
                return Err(if previous_package == &mapping.package {
                    MappingError::DuplicatePackage {
                        package: mapping.package.clone(),
                    }
                } else {
                    MappingError::UnstableOrder
                });
            }
        }
        if mapping.object.object_kind != CacheObjectKind::Source {
            return Err(MappingError::UnsupportedObjectKind {
                package: mapping.package.clone(),
                object_kind: mapping.object.object_kind.as_str().to_owned(),
            });
        }
        previous = Some(&mapping.package);
    }
    Ok(())
}

/// 从环境元数据中解析一个包，并在返回前校验其缓存对象摘要。
#[allow(clippy::result_large_err)]
pub fn resolve_package_object(
    metadata: &EnvironmentMetadata,
    package: &PackageIdentity,
    cache: &CacheStore,
) -> Result<CacheObject, MappingError> {
    let mapping = metadata
        .package_mappings
        .iter()
        .find(|mapping| &mapping.package == package)
        .ok_or_else(|| MappingError::MissingPackage {
            package: package.clone(),
        })?;
    validate_package_mappings(&metadata.package_mappings)?;
    if mapping.object.object_kind != CacheObjectKind::Source {
        return Err(MappingError::UnsupportedObjectKind {
            package: package.clone(),
            object_kind: mapping.object.object_kind.as_str().to_owned(),
        });
    }
    Ok(cache.verify_source_object(&mapping.object.digest)?)
}
