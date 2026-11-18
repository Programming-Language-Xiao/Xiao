//! 按规范源种类分派同步传输，复用同一索引校验及缓存入口。

use super::{
    GitHubAdapter, GitReference, HttpStaticAdapter, IndexSnapshot, LocalDirectoryAdapter,
    PackageSourceAdapter,
};
use crate::diagnostics::SOURCE_INVALID_CODE;
use crate::federation::{ArtifactReference, IndexPackage};
use crate::lockfile::LockedSourceSnapshot;
use crate::source::{SourceDescriptor, SourceError};

/// 单一解析器中同时支持本地目录、静态 HTTP 与 GitHub 稀疏索引。
#[derive(Default)]
pub struct MultiSourceAdapter {
    static_http: HttpStaticAdapter,
    github: GitHubAdapter,
}

impl MultiSourceAdapter {
    /// 构造默认同步适配器组。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 选择自托管 Git raw 基址；标准 GitHub 源用默认构造器即可。
    #[must_use]
    pub fn with_git_raw_base(raw_base: impl Into<String>) -> Self {
        Self {
            github: GitHubAdapter::with_raw_base(GitReference::Head, raw_base),
            ..Self::default()
        }
    }
}

impl PackageSourceAdapter for MultiSourceAdapter {
    fn supports_partial_reads(&self) -> bool {
        true
    }

    fn requires_online_pin_check(&self, source: &SourceDescriptor) -> bool {
        source.kind == "git-index" && self.github.requires_online_pin_check(source)
    }

    fn read_snapshot(&self, source: &SourceDescriptor) -> Result<IndexSnapshot, SourceError> {
        match source.kind.as_str() {
            "path" => LocalDirectoryAdapter.read_snapshot(source),
            "static" | "registry" => self.static_http.read_snapshot(source),
            "git-index" => self.github.read_snapshot(source),
            _ => Err(unknown()),
        }
    }

    fn read_snapshot_pinned(
        &self,
        source: &SourceDescriptor,
        pinned: Option<&LockedSourceSnapshot>,
    ) -> Result<IndexSnapshot, SourceError> {
        match source.kind.as_str() {
            "git-index" => self.github.read_snapshot_pinned(source, pinned),
            _ => self.read_snapshot(source),
        }
    }

    fn read_package(
        &self,
        source: &SourceDescriptor,
        snapshot: &IndexSnapshot,
        name: &str,
    ) -> Result<Vec<IndexPackage>, SourceError> {
        match source.kind.as_str() {
            "path" => LocalDirectoryAdapter.read_package(source, snapshot, name),
            "static" | "registry" => self.static_http.read_package(source, snapshot, name),
            "git-index" => self.github.read_package(source, snapshot, name),
            _ => Err(unknown()),
        }
    }

    fn read_artifact(
        &self,
        source: &SourceDescriptor,
        artifact: &ArtifactReference,
    ) -> Result<Vec<u8>, SourceError> {
        match source.kind.as_str() {
            "path" => LocalDirectoryAdapter.read_artifact(source, artifact),
            "static" | "registry" => self.static_http.read_artifact(source, artifact),
            "git-index" => self.github.read_artifact(source, artifact),
            _ => Err(unknown()),
        }
    }
}

fn unknown() -> SourceError {
    SourceError::new(SOURCE_INVALID_CODE, "不支持的包源类型")
}
