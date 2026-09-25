//! GitHub 稀疏索引：smart HTTP refs 广告 + 不可变 commit 的 raw 文件。

use std::collections::BTreeMap;
use std::sync::Mutex;

use super::git_refs::{GitReference, ResolvedGitRef, parse_advertised_refs};
use super::http_static::{HttpStaticAdapter, join_url};
use super::{
    IndexSnapshot, PackageSourceAdapter, check_package_request, parse_package, parse_snapshot,
};
use crate::diagnostics::{SOURCE_CACHE_IO_CODE, SOURCE_DIGEST_MISMATCH_CODE, SOURCE_INVALID_CODE};
use crate::federation::{ArtifactReference, IndexPackage};
use crate::lockfile::LockedSourceSnapshot;
use crate::source::{SourceDescriptor, SourceError};

/// 用标准 Git HTTP refs 查询引用，只读取 raw 服务上的目标包分片。
pub struct GitHubAdapter {
    reference: GitReference,
    raw_base: Option<String>,
    transport: HttpStaticAdapter,
    observed: Mutex<BTreeMap<String, String>>,
}

impl Default for GitHubAdapter {
    fn default() -> Self {
        Self::new(GitReference::Head)
    }
}

impl GitHubAdapter {
    fn reference_for<'a>(&'a self, source: &'a SourceDescriptor) -> &'a GitReference {
        source.git_reference.as_ref().unwrap_or(&self.reference)
    }
    /// 构造供仓库稀疏索引使用的显式引用（默认使用广告的 HEAD）。
    #[must_use]
    pub fn new(reference: GitReference) -> Self {
        Self {
            reference,
            raw_base: None,
            transport: HttpStaticAdapter::new(),
            observed: Mutex::new(BTreeMap::new()),
        }
    }

    /// 自托管或本地模拟 Git 服务可注入符合相同路径规范的 raw 基址。
    #[must_use]
    pub fn with_raw_base(reference: GitReference, raw_base: impl Into<String>) -> Self {
        Self {
            raw_base: Some(raw_base.into()),
            ..Self::new(reference)
        }
    }

    /// 读取服务器 refs 并验证可变标签没有偏离锁定提交。
    pub fn resolve_ref(
        &self,
        source: &SourceDescriptor,
        pinned: Option<&LockedSourceSnapshot>,
    ) -> Result<ResolvedGitRef, SourceError> {
        self.raw_for(source)?;
        let resolved = if let GitReference::Commit(commit) = self.reference_for(source) {
            if !matches!(
                GitReference::from_config("rev", commit)?,
                GitReference::Commit(_)
            ) {
                return Err(SourceError::new(
                    SOURCE_INVALID_CODE,
                    "必须指定完整 Git 提交哈希",
                ));
            }
            ResolvedGitRef {
                commit: commit.clone(),
                name: commit.clone(),
            }
        } else {
            let bytes = self.transport.get_refs(&source.location)?;
            parse_advertised_refs(&bytes, self.reference_for(source))?
        };
        if matches!(self.reference_for(source), GitReference::Tag(_))
            && pinned.is_some_and(|pinned| pinned.snapshot_id != resolved.commit)
        {
            return Err(SourceError::new(
                SOURCE_DIGEST_MISMATCH_CODE,
                format!(
                    "标签 {} 已从锁定提交移至 {}",
                    resolved.name, resolved.commit
                ),
            ));
        }
        Ok(resolved)
    }

    /// 在已知提交下读取并校验正文，不读取或执行仓库内任何安装脚本。
    pub fn read_artifact_at(
        &self,
        source: &SourceDescriptor,
        commit: &str,
        artifact: &ArtifactReference,
    ) -> Result<Vec<u8>, SourceError> {
        GitReference::from_config("rev", commit).and_then(|parsed| match parsed {
            GitReference::Commit(_) => Ok(()),
            _ => Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "正文必须钉住完整 Git 提交",
            )),
        })?;
        let base = self.raw_for(source)?;
        let base = join_url(&base, commit)?;
        self.transport.artifact(&base, artifact)
    }

    fn raw_for(&self, source: &SourceDescriptor) -> Result<String, SourceError> {
        if source.kind != "git-index"
            || !source.location.starts_with("https://") && !source.location.starts_with("http://")
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "GitHub 索引只支持 HTTP(S) Git 源",
            ));
        }
        join_url(&source.location, "info/refs")?;
        if let Some(base) = &self.raw_base {
            join_url(base, "snapshot.json")?;
            return Ok(base.clone());
        }
        let uri: ureq::http::Uri = source
            .location
            .parse()
            .map_err(|_| SourceError::new(SOURCE_INVALID_CODE, "GitHub 仓库地址不合法"))?;
        if uri.scheme_str() != Some("https")
            || uri.host() != Some("github.com")
            || uri.port_u16().is_some()
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "自动 raw 地址仅支持 github.com HTTPS 仓库",
            ));
        }
        let path = uri.path().trim_matches('/').trim_end_matches(".git");
        let segments = path.split('/').collect::<Vec<_>>();
        if segments.len() != 2
            || segments
                .iter()
                .any(|part| !super::http_static::valid_relative(part))
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "GitHub 仓库需要 owner/repo 路径",
            ));
        }
        Ok(format!(
            "https://raw.githubusercontent.com/{}/{}",
            segments[0], segments[1]
        ))
    }
}

impl PackageSourceAdapter for GitHubAdapter {
    fn supports_partial_reads(&self) -> bool {
        true
    }

    fn requires_online_pin_check(&self, source: &SourceDescriptor) -> bool {
        source.kind == "git-index" && matches!(self.reference_for(source), GitReference::Tag(_))
    }

    fn read_snapshot(&self, source: &SourceDescriptor) -> Result<IndexSnapshot, SourceError> {
        self.read_snapshot_pinned(source, None)
    }

    fn read_snapshot_pinned(
        &self,
        source: &SourceDescriptor,
        pinned: Option<&LockedSourceSnapshot>,
    ) -> Result<IndexSnapshot, SourceError> {
        let resolved = self.resolve_ref(source, pinned)?;
        let base = join_url(&self.raw_for(source)?, &resolved.commit)?;
        let snapshot = parse_snapshot(source, &self.transport.get_text(&base, "snapshot.json")?)?;
        if snapshot.manifest.snapshot_id != resolved.commit {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "Git 快照标识必须等于解析出的不可变提交",
            ));
        }
        if pinned.is_some_and(|pinned| {
            pinned.snapshot_id == resolved.commit && pinned.snapshot_digest != snapshot.digest
        }) {
            return Err(SourceError::new(
                SOURCE_DIGEST_MISMATCH_CODE,
                "同一 Git 提交的快照清单摘要改变",
            ));
        }
        self.observed
            .lock()
            .map_err(|_| SourceError::new(SOURCE_CACHE_IO_CODE, "Git 读取状态损坏"))?
            .insert(source.source.source_id.clone(), resolved.commit);
        Ok(snapshot)
    }

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
        let commit = &snapshot.manifest.snapshot_id;
        if !matches!(
            GitReference::from_config("rev", commit)?,
            GitReference::Commit(_)
        ) {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "Git 快照不是完整提交哈希",
            ));
        }
        let base = join_url(&self.raw_for(source)?, commit)?;
        parse_package(
            source,
            snapshot,
            name,
            &self
                .transport
                .get_text(&base, &format!("index/{name}.json"))?,
        )
    }

    fn read_artifact(
        &self,
        source: &SourceDescriptor,
        artifact: &ArtifactReference,
    ) -> Result<Vec<u8>, SourceError> {
        let commit = self
            .observed
            .lock()
            .map_err(|_| SourceError::new(SOURCE_CACHE_IO_CODE, "Git 读取状态损坏"))?
            .get(&source.source.source_id)
            .cloned()
            .ok_or_else(|| {
                SourceError::new(SOURCE_INVALID_CODE, "正文读取前必须先绑定不可变 Git 快照")
            })?;
        self.read_artifact_at(source, &commit, artifact)
    }
}
