//! 按规范源身份保存不可变快照，并原子切换当前指向。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::adapters::IndexSnapshot;
use crate::cache::CacheLayout;
use crate::diagnostics::{
    SOURCE_CACHE_CORRUPT_CODE, SOURCE_CACHE_IO_CODE, SOURCE_INVALID_CODE,
    SOURCE_SNAPSHOT_OWNER_CODE,
};
use crate::entry_lock::EntryLock;
use crate::jcs::jcs_digest;
use crate::lockfile::atomic_write_file;
use crate::source::SourceError;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SnapshotFile {
    index: IndexSnapshotFile,
    observed_at_ms: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct IndexSnapshotFile {
    manifest: crate::federation::SnapshotManifest,
    digest: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CurrentPointer {
    source_id: String,
    snapshot_id: String,
    digest: String,
    observed_at_ms: u64,
}

/// 已验证且可用于展示新鲜度的源清单。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredSnapshot {
    /// 不可变的源索引清单和 JCS 摘要。
    pub index: IndexSnapshot,
    /// 最后一次成功从源读取快照的毫秒时间戳；不参与过期判定。
    pub observed_at_ms: u64,
}

/// `current.json` 与历史快照始终保存在 SHA-256 源身份目录下。
#[derive(Clone, Debug)]
pub struct SnapshotStore {
    layout: CacheLayout,
}

impl SnapshotStore {
    /// 复用 E1 的缓存根布局，不读取远程源。
    #[must_use]
    pub fn new(layout: CacheLayout) -> Self {
        Self { layout }
    }

    /// 清单及分片已经校验后写入历史版本，最后才切换当前指向。
    pub fn save(&self, index: &IndexSnapshot) -> Result<StoredSnapshot, SourceError> {
        verify_index(index, &index.manifest.source_id)?;
        let directory = self.source_directory(&index.manifest.source_id)?;
        let _guard = EntryLock::acquire(&directory.join("current.lock"))?;
        let path = directory.join(format!("{}.json", index.manifest.snapshot_id));
        if path.exists() {
            let previous = self.read_snapshot(&path, &index.manifest.source_id)?;
            if previous.index != *index {
                return Err(SourceError::new(
                    SOURCE_SNAPSHOT_OWNER_CODE,
                    "同名不可变快照被不同内容占用",
                ));
            }
        } else {
            let record = SnapshotFile {
                index: IndexSnapshotFile {
                    manifest: index.manifest.clone(),
                    digest: index.digest.clone(),
                },
                observed_at_ms: now_ms(),
            };
            atomic_write_file(&path, &serde_json::to_vec(&record).map_err(cache_corrupt)?)
                .map_err(cache_io)?;
        }
        let stored = self.read_snapshot(&path, &index.manifest.source_id)?;
        let pointer = CurrentPointer {
            source_id: index.manifest.source_id.clone(),
            snapshot_id: index.manifest.snapshot_id.clone(),
            digest: index.digest.clone(),
            observed_at_ms: stored.observed_at_ms,
        };
        atomic_write_file(
            &directory.join("current.json"),
            &serde_json::to_vec(&pointer).map_err(cache_corrupt)?,
        )
        .map_err(cache_io)?;
        Ok(stored)
    }

    /// 只接受身份、指向、正文摘要均相符的当前快照；缺失与损坏不同。
    pub fn current(&self, source_id: &str) -> Result<Option<StoredSnapshot>, SourceError> {
        let directory = self.source_directory(source_id)?;
        let path = directory.join("current.json");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(cache_io(error)),
        };
        let pointer: CurrentPointer = serde_json::from_slice(&bytes).map_err(cache_corrupt)?;
        if pointer.source_id != source_id || !valid_snapshot_name(&pointer.snapshot_id) {
            return Err(SourceError::new(
                SOURCE_SNAPSHOT_OWNER_CODE,
                "当前快照指向了其他源或非法路径",
            ));
        }
        let stored = self.read_snapshot(
            &directory.join(format!("{}.json", pointer.snapshot_id)),
            source_id,
        )?;
        if stored.index.digest != pointer.digest || stored.observed_at_ms != pointer.observed_at_ms
        {
            return Err(SourceError::new(
                SOURCE_CACHE_CORRUPT_CODE,
                "当前快照指向的身份或时间戳不匹配",
            ));
        }
        Ok(Some(stored))
    }

    fn source_directory(&self, source_id: &str) -> Result<PathBuf, SourceError> {
        if source_id.is_empty() {
            return Err(SourceError::new(SOURCE_INVALID_CODE, "源身份不能为空"));
        }
        let digest = format!("{:x}", Sha256::digest(source_id.as_bytes()));
        Ok(self.layout.snapshots_root().join(digest))
    }

    fn read_snapshot(&self, path: &Path, source_id: &str) -> Result<StoredSnapshot, SourceError> {
        let bytes = fs::read(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                cache_corrupt("当前指向的快照文件缺失")
            } else {
                cache_io(error)
            }
        })?;
        let record: SnapshotFile = serde_json::from_slice(&bytes).map_err(cache_corrupt)?;
        let index = IndexSnapshot {
            manifest: record.index.manifest,
            digest: record.index.digest,
        };
        verify_index(&index, source_id)?;
        if path.file_name().and_then(|name| name.to_str())
            != Some(format!("{}.json", index.manifest.snapshot_id).as_str())
        {
            return Err(SourceError::new(
                SOURCE_SNAPSHOT_OWNER_CODE,
                "快照文件名与快照身份不匹配",
            ));
        }
        Ok(StoredSnapshot {
            index,
            observed_at_ms: record.observed_at_ms,
        })
    }
}

fn verify_index(index: &IndexSnapshot, source_id: &str) -> Result<(), SourceError> {
    if index.manifest.source_id != source_id {
        return Err(SourceError::new(
            SOURCE_SNAPSHOT_OWNER_CODE,
            "快照清单与源身份不匹配",
        ));
    }
    index.manifest.validate(source_id)?;
    if !valid_snapshot_name(&index.manifest.snapshot_id) {
        return Err(SourceError::new(
            SOURCE_INVALID_CODE,
            "快照标识不能安全地用作文件名",
        ));
    }
    let text = serde_json::to_string(&index.manifest).map_err(cache_corrupt)?;
    if jcs_digest(&text)? != index.digest {
        return Err(SourceError::new(
            SOURCE_CACHE_CORRUPT_CODE,
            "清单内容与 JCS 摘要不一致",
        ));
    }
    Ok(())
}

fn valid_snapshot_name(name: &str) -> bool {
    if name.is_empty()
        || matches!(name, "." | "..")
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return false;
    }
    !matches!(
        name.split('.')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase()
            .as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn cache_io(error: impl std::fmt::Display) -> SourceError {
    SourceError::new(SOURCE_CACHE_IO_CODE, error.to_string())
}

fn cache_corrupt(error: impl std::fmt::Display) -> SourceError {
    SourceError::new(SOURCE_CACHE_CORRUPT_CODE, error.to_string())
}
