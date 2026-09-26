//! 导入源列表只接受 URL 中不含凭据的完整地址，钉住 JCS 摘要后可复用本地字节。

use std::collections::BTreeMap;
use std::fs;

use crate::adapters::HttpStaticAdapter;
use crate::cache::CacheStore;
use crate::diagnostics::{SOURCE_CACHE_IO_CODE, SOURCE_DIGEST_MISMATCH_CODE, SOURCE_INVALID_CODE};
use crate::entry_lock::EntryLock;
use crate::lockfile::atomic_write_file;
use crate::source::{SourceDeclaration, SourceError, SourceList, valid_digest};

pub(crate) fn load(
    declarations: &[SourceDeclaration],
    cache: &CacheStore,
) -> Result<BTreeMap<String, SourceList>, SourceError> {
    let mut lists: BTreeMap<String, SourceList> = BTreeMap::new();
    for declaration in declarations {
        let SourceDeclaration::Import(import) = declaration else {
            continue;
        };
        if !valid_digest(&import.digest) {
            return Err(SourceError::new(SOURCE_INVALID_CODE, "导入源列表摘要无效"));
        }
        if let Some(previous) = lists.get(&import.location) {
            if previous.digest != import.digest {
                return Err(SourceError::new(
                    SOURCE_DIGEST_MISMATCH_CODE,
                    "源列表重复导入时摘要不一致",
                ));
            }
            continue;
        }
        let path = cache
            .layout()
            .cache_root()
            .join("objects/source-lists/sha256")
            .join(&import.digest[..2])
            .join(format!("{}.json", import.digest));
        let lock_path = cache
            .layout()
            .cache_root()
            .join("locks/source-lists")
            .join(format!("{}.lock", import.digest));
        let _guard = EntryLock::acquire(&lock_path)
            .map_err(|error| SourceError::new(error.code, "源列表缓存锁不可用"))?;
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let text = HttpStaticAdapter::new().source_list(&import.location)?;
                let list = SourceList::parse(&text)?;
                if list.digest != import.digest {
                    return Err(SourceError::new(
                        SOURCE_DIGEST_MISMATCH_CODE,
                        "源列表与钉住摘要不符",
                    ));
                }
                atomic_write_file(&path, text.as_bytes())
                    .map_err(|_| SourceError::new(SOURCE_CACHE_IO_CODE, "无法原子保存源列表"))?;
                text
            }
            Err(_) => return Err(SourceError::new(SOURCE_CACHE_IO_CODE, "无法读取源列表缓存")),
        };
        let parsed = SourceList::parse(&text)?;
        if parsed.digest != import.digest {
            return Err(SourceError::new(
                SOURCE_DIGEST_MISMATCH_CODE,
                "已缓存源列表摘要损坏",
            ));
        }
        lists.insert(import.location.clone(), parsed);
    }
    Ok(lists)
}
