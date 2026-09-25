//! 静态索引元数据与包正文分离的适配器边界。

use std::fs;
use std::path::{Component, Path};

use sha2::{Digest, Sha256};

use crate::diagnostics::{
    SOURCE_DIGEST_MISMATCH_CODE, SOURCE_INVALID_CODE, SOURCE_UNAVAILABLE_CODE,
    SOURCE_UNSUPPORTED_VERSION_CODE,
};
use crate::federation::{
    ArtifactReference, IndexPackage, PackageShard, SnapshotManifest, valid_package_name,
};
use crate::jcs::canonicalize_json;
use crate::lockfile::LockedSourceSnapshot;
use crate::source::{SOURCE_PROTOCOL_VERSION, SourceDescriptor, SourceError};

/// Git 智能 HTTP 引用帧解析。
mod git_refs;
/// GitHub 标准 refs 和 raw 稀疏索引读取。
mod github;
/// 静态 HTTP 索引及正文续传。
mod http_static;
/// 按源种类派发已校验内容。
mod multi;

/// 重导出 Git 引用类型与标准 refs 解析入口。
pub use git_refs::{GitReference, ResolvedGitRef, parse_advertised_refs};
/// 重导出不可变 Git 稀疏索引适配器。
pub use github::GitHubAdapter;
/// 重导出同步 HTTP 静态包源适配器。
pub use http_static::HttpStaticAdapter;
/// 重导出本地、HTTP 与 Git 的统一派发适配器。
pub use multi::MultiSourceAdapter;

/// 已验证清单的快照身份及稀疏索引目录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexSnapshot {
    /// 规范化的源清单。
    pub manifest: SnapshotManifest,
    /// 清单 JCS 字节的 SHA-256。
    pub digest: String,
}

/// 传输仅返回规范元数据；源码/预编译产物另按引用获取。
pub trait PackageSourceAdapter {
    /// 是否支持断点后的部分读取；本地目录源固定不支持。
    fn supports_partial_reads(&self) -> bool {
        false
    }
    /// 仅读取源清单，不下载任何正文。
    fn read_snapshot(&self, source: &SourceDescriptor) -> Result<IndexSnapshot, SourceError>;
    /// 有锁定快照时可验证可变引用和内容；离线快速路径不调用此方法。
    fn read_snapshot_pinned(
        &self,
        source: &SourceDescriptor,
        _pinned: Option<&LockedSourceSnapshot>,
    ) -> Result<IndexSnapshot, SourceError> {
        self.read_snapshot(source)
    }
    /// 标签需要在在线解析时检查是否被改写。
    fn requires_online_pin_check(&self, _source: &SourceDescriptor) -> bool {
        false
    }
    /// 只获取指定包名的分片及全部版本元数据。
    fn read_package(
        &self,
        source: &SourceDescriptor,
        snapshot: &IndexSnapshot,
        name: &str,
    ) -> Result<Vec<IndexPackage>, SourceError>;
    /// 按需读取并完整校验一个正文对象。
    fn read_artifact(
        &self,
        source: &SourceDescriptor,
        artifact: &ArtifactReference,
    ) -> Result<Vec<u8>, SourceError>;
}

/// 无网络调用的本地目录适配器。
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalDirectoryAdapter;

impl PackageSourceAdapter for LocalDirectoryAdapter {
    /// 从目录根读取清单，先按 JCS 求摘要再校验协议和来源。
    fn read_snapshot(&self, source: &SourceDescriptor) -> Result<IndexSnapshot, SourceError> {
        let text = read_text(source, "snapshot.json")?;
        parse_snapshot(source, &text)
    }

    /// 只读取请求包的分片，不获取任何源码或预编译正文。
    fn read_package(
        &self,
        source: &SourceDescriptor,
        snapshot: &IndexSnapshot,
        name: &str,
    ) -> Result<Vec<IndexPackage>, SourceError> {
        check_package_request(source, snapshot, name)?;
        if !snapshot.manifest.shards.contains_key(name) {
            return Ok(Vec::new());
        }
        let text = read_text(source, &format!("index/{name}.json"))?;
        parse_package(source, snapshot, name, &text)
    }

    /// 延迟读取指定正文并核对长度和 SHA-256。
    fn read_artifact(
        &self,
        source: &SourceDescriptor,
        artifact: &ArtifactReference,
    ) -> Result<Vec<u8>, SourceError> {
        let path = safe_path(source, &artifact.location)?;
        let bytes = fs::read(&path).map_err(|error| {
            SourceError::new(
                SOURCE_UNAVAILABLE_CODE,
                format!("无法读取 {}: {error}", path.display()),
            )
        })?;
        verify_artifact(artifact, &bytes)?;
        Ok(bytes)
    }
}

fn parse_snapshot(source: &SourceDescriptor, text: &str) -> Result<IndexSnapshot, SourceError> {
    let digest = jcs_sha256(text)?;
    let manifest: SnapshotManifest = serde_json::from_str(text)
        .map_err(|error| SourceError::new(SOURCE_INVALID_CODE, error.to_string()))?;
    manifest.validate(&source.source.source_id)?;
    Ok(IndexSnapshot { manifest, digest })
}

fn check_package_request(
    source: &SourceDescriptor,
    snapshot: &IndexSnapshot,
    name: &str,
) -> Result<(), SourceError> {
    if !valid_package_name(name) || snapshot.manifest.source_id != source.source.source_id {
        return Err(SourceError::new(
            SOURCE_INVALID_CODE,
            "索引包名或源身份不匹配",
        ));
    }
    Ok(())
}

fn parse_package(
    source: &SourceDescriptor,
    snapshot: &IndexSnapshot,
    name: &str,
    text: &str,
) -> Result<Vec<IndexPackage>, SourceError> {
    check_package_request(source, snapshot, name)?;
    let Some(expected) = snapshot.manifest.shards.get(name) else {
        return Ok(Vec::new());
    };
    if jcs_sha256(text)? != *expected {
        return Err(SourceError::new(
            SOURCE_DIGEST_MISMATCH_CODE,
            format!("分片 {name} 的摘要不匹配"),
        ));
    }
    let shard: PackageShard = serde_json::from_str(text)
        .map_err(|error| SourceError::new(SOURCE_INVALID_CODE, error.to_string()))?;
    if shard.protocol_version != SOURCE_PROTOCOL_VERSION {
        return Err(SourceError::new(
            SOURCE_UNSUPPORTED_VERSION_CODE,
            format!("不支持分片协议版本 {}", shard.protocol_version),
        ));
    }
    if shard.source_id != snapshot.manifest.source_id
        || shard.snapshot_id != snapshot.manifest.snapshot_id
        || shard.packages.iter().any(|package| {
            package.name != name
                || package.version.is_empty()
                || package.variant.is_empty()
                || package.dependencies.iter().any(|dependency| {
                    !valid_package_name(&dependency.name) || dependency.version.is_empty()
                })
                || !valid_artifact(&package.source_artifact)
                || package
                    .binary_artifacts
                    .iter()
                    .any(|artifact| !valid_artifact(artifact))
        })
    {
        return Err(SourceError::new(
            SOURCE_INVALID_CODE,
            "分片的源、快照或包名不匹配",
        ));
    }
    Ok(shard.packages)
}

fn verify_artifact(artifact: &ArtifactReference, bytes: &[u8]) -> Result<(), SourceError> {
    if !valid_artifact(artifact)
        || bytes.len() as u64 != artifact.length
        || format!("{:x}", Sha256::digest(bytes)) != artifact.digest
    {
        return Err(SourceError::new(
            SOURCE_DIGEST_MISMATCH_CODE,
            format!("正文 {} 校验失败", artifact.location),
        ));
    }
    Ok(())
}

/// 计算静态 JSON 的规范字节摘要。
fn jcs_sha256(text: &str) -> Result<String, SourceError> {
    Ok(format!("{:x}", Sha256::digest(canonicalize_json(text)?)))
}

/// 通过受限相对路径读取 UTF-8 文件。
fn read_text(source: &SourceDescriptor, relative: &str) -> Result<String, SourceError> {
    let path = safe_path(source, relative)?;
    fs::read_to_string(&path).map_err(|error| {
        SourceError::new(
            SOURCE_UNAVAILABLE_CODE,
            format!("无法读取 {}: {error}", path.display()),
        )
    })
}

/// 用规范路径边界阻止绝对路径、上级路径和符号链接逃逸。
fn safe_path(source: &SourceDescriptor, relative: &str) -> Result<std::path::PathBuf, SourceError> {
    if source.kind != "path"
        || relative.is_empty()
        || !Path::new(relative)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(SourceError::new(
            SOURCE_INVALID_CODE,
            "本地源路径必须是源目录内的普通相对路径",
        ));
    }
    let root = fs::canonicalize(&source.location).map_err(|error| {
        SourceError::new(
            SOURCE_UNAVAILABLE_CODE,
            format!("无法读取源根目录: {error}"),
        )
    })?;
    let candidate = fs::canonicalize(root.join(relative)).map_err(|error| {
        SourceError::new(SOURCE_UNAVAILABLE_CODE, format!("无法读取源文件: {error}"))
    })?;
    if !candidate.starts_with(root) || !candidate.is_file() {
        return Err(SourceError::new(
            SOURCE_INVALID_CODE,
            "源文件不得离开源根目录",
        ));
    }
    Ok(candidate)
}

/// 校验索引正文引用的摘要语法和源目录内相对路径。
fn valid_artifact(artifact: &ArtifactReference) -> bool {
    artifact.digest.len() == 64
        && artifact
            .digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && http_static::valid_relative(&artifact.location)
}
