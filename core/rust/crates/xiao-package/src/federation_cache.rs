//! 联邦源索引的原子持久化与跨源共享的不可变包元数据。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cache::CacheLayout;
use crate::diagnostics::{SOURCE_CACHE_CORRUPT_CODE, SOURCE_CACHE_IO_CODE, SOURCE_INVALID_CODE};
use crate::entry_lock::EntryLock;
use crate::federation::{
    FederatedRecord, IndexPackage, SourceListFingerprint, SourceSnapshot, federate,
};
use crate::jcs::canonicalize_json;
use crate::lockfile::atomic_write_file;
use crate::source::{SourceError, valid_digest};

static NEXT_METADATA: AtomicU64 = AtomicU64::new(0);

/// 内容地址相同的包元数据只存一份，与来源身份无关。
#[derive(Clone, Debug)]
pub struct MetadataCache {
    layout: CacheLayout,
}

impl MetadataCache {
    /// 使用 E1 的缓存根建立独立的元数据命名空间。
    #[must_use]
    pub fn new(layout: CacheLayout) -> Self {
        Self { layout }
    }

    /// 在同目录完整写好暂存对象后发布，不暴露半成品目录。
    pub fn store(&self, package: &IndexPackage) -> Result<String, SourceError> {
        let text = serde_json::to_string(package).map_err(cache_corrupt)?;
        let bytes = canonicalize_json(&text)?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let object = self.object_path(&digest)?;
        let _guard = EntryLock::acquire(&object.with_extension("lock"))?;
        if let Some(existing) = self.read(&digest)? {
            if existing != *package {
                return Err(cache_corrupt("同摘要元数据不一致"));
            }
            return Ok(digest);
        }
        let parent = object
            .parent()
            .ok_or_else(|| cache_io("元数据对象缺少父目录"))?;
        fs::create_dir_all(parent).map_err(cache_io)?;
        let staging = parent.join(format!(
            ".{digest}.tmp-{}-{}",
            std::process::id(),
            NEXT_METADATA.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&staging).map_err(cache_io)?;
        let result = atomic_write_file(&staging.join("metadata.json"), &bytes)
            .and_then(|()| fs::rename(&staging, &object));
        if let Err(error) = result {
            let _ = fs::remove_file(staging.join("metadata.json"));
            let _ = fs::remove_dir(&staging);
            return Err(cache_io(error));
        }
        Ok(digest)
    }

    /// 在使用前逐字节复算 JCS 摘要，区分不存在与损坏。
    pub fn read(&self, digest: &str) -> Result<Option<IndexPackage>, SourceError> {
        let object = self.object_path(digest)?;
        let path = object.join("metadata.json");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound && !object.exists() => {
                return Ok(None);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(cache_corrupt("元数据对象目录缺少正文"));
            }
            Err(error) => return Err(cache_io(error)),
        };
        let text = std::str::from_utf8(&bytes).map_err(cache_corrupt)?;
        if format!(
            "{:x}",
            Sha256::digest(canonicalize_json(text).map_err(cache_corrupt)?)
        ) != digest
        {
            return Err(cache_corrupt("包元数据 JCS 摘要不匹配"));
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(cache_corrupt)
    }

    fn object_path(&self, digest: &str) -> Result<PathBuf, SourceError> {
        if !valid_digest(digest) {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "元数据摘要格式不合法",
            ));
        }
        Ok(self
            .layout
            .metadata_objects_root()
            .join(&digest[..2])
            .join(digest))
    }
}

/// 一次完整合并的快照、候选和源顺序，不能存半份更新。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FederationIndex {
    /// 当前配置指纹，阻止其他项目的源顺序复用。
    pub config_fingerprint: String,
    /// 已展开源序列与导入出处的规范摘要。
    pub sources: SourceListFingerprint,
    /// 每个源的完整查询覆盖与三态。
    pub snapshots: Vec<SourceSnapshot>,
    /// 含源身份与不可变快照引用的候选。
    pub records: Vec<FederatedRecord>,
}

/// 联邦索引覆盖存储；旧索引更新失败时保持可读。
#[derive(Clone, Debug)]
pub struct FederationCache {
    layout: CacheLayout,
}

impl FederationCache {
    /// 从已经选定的用户缓存根构造。
    #[must_use]
    pub fn new(layout: CacheLayout) -> Self {
        Self { layout }
    }

    /// 验证完整联邦记录及元数据对象后再原子提交。
    pub fn store(&self, index: &FederationIndex) -> Result<(), SourceError> {
        self.commit(index, atomic_write_file)
    }

    /// 只读取与当前配置及展开源序列都相符的完整缓存。
    pub fn read(
        &self,
        config_fingerprint: &str,
        sources: &SourceListFingerprint,
    ) -> Result<Option<FederationIndex>, SourceError> {
        let path = self.layout.federation_root().join("index.json");
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(cache_io(error)),
        };
        let index: FederationIndex = serde_json::from_slice(&bytes).map_err(cache_corrupt)?;
        if index.config_fingerprint != config_fingerprint || index.sources != *sources {
            return Ok(None);
        }
        self.validate(&index)?;
        Ok(Some(index))
    }

    fn commit(
        &self,
        index: &FederationIndex,
        writer: impl FnOnce(&Path, &[u8]) -> io::Result<()>,
    ) -> Result<(), SourceError> {
        self.validate(index)?;
        let path = self.layout.federation_root().join("index.json");
        let _guard = EntryLock::acquire(&path.with_extension("lock"))?;
        writer(&path, &serde_json::to_vec(index).map_err(cache_corrupt)?).map_err(cache_io)
    }

    fn validate(&self, index: &FederationIndex) -> Result<(), SourceError> {
        if index.config_fingerprint.is_empty()
            || index.snapshots.len() != index.sources.ordered_sources.len()
            || index.snapshots.iter().enumerate().any(|(order, snapshot)| {
                snapshot.config_order != order
                    || snapshot.source_id != index.sources.ordered_sources[order].source_id
            })
            || federate(&index.snapshots)? != index.records
        {
            return Err(cache_corrupt("联邦索引顺序、快照或候选记录不一致"));
        }
        let metadata = MetadataCache::new(self.layout.clone());
        for record in &index.records {
            let text = serde_json::to_string(&record.package).map_err(cache_corrupt)?;
            let digest = format!("{:x}", Sha256::digest(canonicalize_json(&text)?));
            if metadata.read(&digest)?.as_ref() != Some(&record.package) {
                return Err(cache_corrupt("联邦候选缺少已验证的包元数据"));
            }
        }
        Ok(())
    }
}

fn cache_io(error: impl std::fmt::Display) -> SourceError {
    SourceError::new(SOURCE_CACHE_IO_CODE, error.to_string())
}

fn cache_corrupt(error: impl std::fmt::Display) -> SourceError {
    SourceError::new(SOURCE_CACHE_CORRUPT_CODE, error.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{FederationCache, FederationIndex};
    use crate::cache::CacheLayout;
    use crate::federation::source_list_fingerprint;

    #[test]
    /// 更新失败时旧索引仍是完整可读的版本。
    fn failed_commit_retains_previous_index() {
        let home = std::env::temp_dir().join(format!(
            "xiao-e3b-fed-failure-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&home).unwrap();
        let layout = CacheLayout::from_xiao_home(Some(&home), &home).unwrap();
        let store = FederationCache::new(layout);
        let original = FederationIndex {
            config_fingerprint: "first".to_owned(),
            sources: source_list_fingerprint(&[]),
            snapshots: Vec::new(),
            records: Vec::new(),
        };
        store.store(&original).unwrap();
        let mut changed = original.clone();
        changed.config_fingerprint = "second".to_owned();
        assert_eq!(
            store
                .commit(&changed, |_, _| Err(std::io::Error::other("中断")))
                .unwrap_err()
                .code,
            crate::diagnostics::SOURCE_CACHE_IO_CODE
        );
        assert_eq!(
            store.read("first", &original.sources).unwrap(),
            Some(original)
        );
        fs::remove_file(home.join("cache/federation/index.json")).unwrap();
        fs::remove_dir(home.join("cache/federation")).unwrap();
        fs::remove_dir(home.join("cache")).unwrap();
        fs::remove_dir(home).unwrap();
    }
}
