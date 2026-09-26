//! 远程正文只能先验证字节摘要，再受限展开为 E1 目录对象。

use std::collections::BTreeSet;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};

use crate::adapters::PackageSourceAdapter;
use crate::cache::{CacheObject, CacheStore, source_directory_digest};
use crate::diagnostics::{SOURCE_CACHE_IO_CODE, TRUST_ARCHIVE_CODE, TRUST_ARTIFACT_CODE};
use crate::entry_lock::EntryLock;
use crate::federation::ArtifactReference;
use crate::source::{SourceDescriptor, SourceError};

static NEXT_EXTRACT: AtomicU64 = AtomicU64::new(0);
const MAX_ENTRIES: usize = 16_384;
const MAX_UNPACKED_BYTES: u64 = 128 * 1024 * 1024;

fn invalid() -> SourceError {
    SourceError::new(TRUST_ARCHIVE_CODE, "远程源码正文不是安全的无压缩 TAR")
}

fn io_failure() -> SourceError {
    SourceError::new(SOURCE_CACHE_IO_CODE, "无法创建远程源码缓存暂存目录")
}

struct TemporaryDirectory(PathBuf);

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl TemporaryDirectory {
    fn new(cache: &CacheStore) -> Result<Self, SourceError> {
        let root = cache.layout().cache_root().join("tmp/source-import");
        fs::create_dir_all(&root).map_err(|_| io_failure())?;
        for _ in 0..100 {
            let name = format!(
                "{}-{}",
                std::process::id(),
                NEXT_EXTRACT.fetch_add(1, Ordering::Relaxed)
            );
            let path = root.join(name);
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(io_failure()),
            }
        }
        Err(io_failure())
    }
}

fn safe_path(bytes: &[u8]) -> Result<String, SourceError> {
    let path = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let path = path.strip_suffix('/').unwrap_or(path);
    if path.is_empty() || path.contains(['\\', ':', '\0']) || path.starts_with('/') {
        return Err(invalid());
    }
    for segment in path.split('/') {
        let trimmed = segment.trim_end_matches([' ', '.']);
        let upper = trimmed
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if segment.is_empty()
            || matches!(segment, "." | "..")
            || segment != trimmed
            || segment.chars().any(char::is_control)
            || matches!(
                upper.as_str(),
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
        {
            return Err(invalid());
        }
    }
    Ok(path.to_owned())
}

pub(crate) fn import_artifact(
    adapter: &impl PackageSourceAdapter,
    source: &SourceDescriptor,
    artifact: &ArtifactReference,
    expected_content: Option<&str>,
    cache: &CacheStore,
) -> Result<CacheObject, SourceError> {
    let lock_path = cache
        .layout()
        .cache_root()
        .join("locks/remote-artifacts")
        .join(format!("{}.lock", artifact.digest));
    let _guard = EntryLock::acquire(&lock_path)
        .map_err(|error| SourceError::new(error.code, "无法获取远程正文条目锁"))?;
    if let Some(digest) = expected_content {
        match cache.verify_source_object(digest) {
            Ok(object) => return Ok(object),
            Err(crate::cache::CacheError::ObjectCorrupt { .. }) => {}
            Err(crate::cache::CacheError::Write { .. }) => {}
            Err(_) => return Err(io_failure()),
        }
    }
    let bytes = adapter.read_artifact(source, artifact).map_err(|error| {
        if error.code == crate::diagnostics::SOURCE_DIGEST_MISMATCH_CODE {
            SourceError::new(TRUST_ARTIFACT_CODE, "下载正文与锁定产物摘要不符")
        } else {
            error
        }
    })?;
    if bytes.len() as u64 != artifact.length
        || format!("{:x}", Sha256::digest(&bytes)) != artifact.digest
    {
        return Err(SourceError::new(
            TRUST_ARTIFACT_CODE,
            "下载正文与锁定产物摘要不符",
        ));
    }
    let temporary = TemporaryDirectory::new(cache)?;
    unpack(&bytes, &temporary.0)?;
    let digest = source_directory_digest(&temporary.0).map_err(|_| invalid())?;
    if expected_content.is_some_and(|expected| expected != digest) {
        return Err(SourceError::new(
            TRUST_ARTIFACT_CODE,
            "远程源码目录与锁定内容摘要不符",
        ));
    }
    cache
        .import_source_directory(&temporary.0)
        .map_err(|_| io_failure())
}

fn unpack(bytes: &[u8], root: &Path) -> Result<(), SourceError> {
    let mut archive = tar::Archive::new(Cursor::new(bytes));
    let mut seen = BTreeSet::new();
    let mut expanded = 0_u64;
    for entry in archive.entries().map_err(|_| invalid())? {
        let mut entry = entry.map_err(|_| invalid())?;
        if seen.len() >= MAX_ENTRIES {
            return Err(invalid());
        }
        let path = safe_path(&entry.path_bytes())?;
        if !seen.insert(path.to_ascii_lowercase()) {
            return Err(invalid());
        }
        let destination = root.join(&path);
        let kind = entry.header().entry_type();
        if kind == tar::EntryType::Directory {
            fs::create_dir_all(&destination).map_err(|_| invalid())?;
        } else if kind == tar::EntryType::Regular {
            expanded = expanded
                .checked_add(entry.size())
                .filter(|size| *size <= MAX_UNPACKED_BYTES)
                .ok_or_else(invalid)?;
            fs::create_dir_all(destination.parent().ok_or_else(invalid)?).map_err(|_| invalid())?;
            let mut output = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&destination)
                .map_err(|_| invalid())?;
            std::io::copy(&mut entry, &mut output).map_err(|_| invalid())?;
            output.flush().map_err(|_| invalid())?;
        } else {
            return Err(invalid());
        }
    }
    if seen.is_empty() {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_names_are_rejected_before_any_write() {
        for path in [
            "../escape",
            "/absolute",
            "a/../b",
            "a\\b",
            "CON",
            "a.",
            "a//b",
            "C:/drive",
        ] {
            assert_eq!(
                safe_path(path.as_bytes()).unwrap_err().code,
                TRUST_ARCHIVE_CODE
            );
        }
    }

    #[test]
    fn links_and_duplicate_paths_are_rejected() {
        let root = std::env::temp_dir().join(format!("xiao-tar-probe-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_ustar();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_link(&mut header, "link", "../escape")
            .unwrap();
        assert_eq!(
            unpack(&builder.into_inner().unwrap(), &root)
                .unwrap_err()
                .code,
            TRUST_ARCHIVE_CODE
        );
        assert!(!root.join("link").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
