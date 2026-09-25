//! 11A-E1 的本地源码对象缓存。
//!
//! 本模块把目录快照编码成确定性的字节流，使用 SHA-256 生成内容身份，并将完整快照
//! 写入 `XIAO_HOME/cache/objects/source/sha256/`。缓存对象不包含来源、包名或版本信息，
//! 这些逻辑身份由 [`crate::model::PackageIdentity`] 和环境映射保存。

use std::env;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::diagnostics::{
    CACHE_HOME_UNAVAILABLE_CODE, CACHE_INVALID_INPUT_CODE, CACHE_INVALID_SOURCE_CODE,
    CACHE_OBJECT_CORRUPT_CODE,
};

/// `XIAO_HOME` 环境变量名。
pub const XIAO_HOME_ENV: &str = "XIAO_HOME";
/// 源码对象的对象类型名。
pub const SOURCE_OBJECT_KIND: &str = "source";
/// 源码对象摘要算法名。
pub const SOURCE_OBJECT_ALGORITHM: &str = "sha256";
/// 已物化环境目录的元数据标记文件名，用于排除生成内容。
const GENERATED_ENVIRONMENT_METADATA_FILE: &str = ".xiao-environment.json";
/// 项目锁文件由 E2A 管理，不属于包源码对象内容。
const GENERATED_LOCKFILE: &str = "xiao.lock.json";

/// 生成进程内唯一暂存对象名称的计数器。
static NEXT_TEMP_OBJECT: AtomicU64 = AtomicU64::new(0);

/// 环境映射中保存的缓存对象类型。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheObjectKind {
    /// 本地路径包的规范化源码目录对象。
    Source,
}

impl CacheObjectKind {
    /// 返回稳定的 JSON 对象类型名。
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Source => SOURCE_OBJECT_KIND,
        }
    }
}

/// 环境映射指向的不可变缓存对象引用。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CacheObjectReference {
    /// 完整的小写 SHA-256 十六进制摘要。
    pub digest: String,
    /// 对象类型；源码映射固定为 `source`。
    pub object_kind: CacheObjectKind,
}

/// 已通过摘要校验的缓存对象。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheObject {
    /// 对象的稳定引用。
    pub reference: CacheObjectReference,
    /// 全局缓存中的实际目录路径。
    pub path: PathBuf,
}

/// Xiao 用户域、源码对象和全局环境的确定性路径布局。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheLayout {
    xiao_home: PathBuf,
}

impl CacheLayout {
    /// 从进程环境读取 `XIAO_HOME`，未设置或为空时回退用户主目录。
    pub fn from_environment() -> Result<Self, CacheError> {
        let user_home = user_home_directory()?;
        Self::from_xiao_home(
            env::var_os(XIAO_HOME_ENV).as_deref().map(Path::new),
            user_home,
        )
    }

    /// 使用注入的 `XIAO_HOME` 和用户主目录构造布局，供宿主和测试隔离环境。
    pub fn from_xiao_home(
        xiao_home: Option<&Path>,
        fallback_home: impl AsRef<Path>,
    ) -> Result<Self, CacheError> {
        let fallback = fallback_home.as_ref();
        let selected = xiao_home
            .filter(|path| !path.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| fallback.join(".xiao"));
        if !selected.is_absolute() {
            return Err(CacheError::InvalidInput {
                path: selected,
                reason: "XIAO_HOME 必须是绝对路径",
            });
        }
        Ok(Self {
            xiao_home: selected,
        })
    }

    /// 返回用户域根目录。
    #[must_use]
    pub fn xiao_home(&self) -> &Path {
        &self.xiao_home
    }

    /// 返回全局缓存根目录。
    #[must_use]
    pub fn cache_root(&self) -> PathBuf {
        self.xiao_home.join("cache")
    }

    /// 返回源码对象根目录。
    #[must_use]
    pub fn source_objects_root(&self) -> PathBuf {
        self.cache_root()
            .join("objects")
            .join(SOURCE_OBJECT_KIND)
            .join(SOURCE_OBJECT_ALGORITHM)
    }

    /// 返回同文件系统内的对象暂存目录。
    #[must_use]
    pub fn temporary_root(&self) -> PathBuf {
        self.cache_root().join("objects").join("tmp")
    }

    /// 返回全局环境根目录。
    #[must_use]
    pub fn environments_root(&self) -> PathBuf {
        self.xiao_home.join("envs")
    }

    /// 根据完整摘要构造源码对象目录。
    pub fn source_object_path(&self, digest: &str) -> Result<PathBuf, CacheError> {
        validate_digest(digest)?;
        Ok(self.source_objects_root().join(&digest[..2]).join(digest))
    }

    /// 根据逻辑名称构造全局环境目录。
    pub fn global_environment_path(&self, logical_name: &str) -> Result<PathBuf, CacheError> {
        if logical_name.is_empty()
            || logical_name == "."
            || logical_name == ".."
            || logical_name
                .chars()
                .any(|character| character.is_control() || character == '/' || character == '\\')
        {
            return Err(CacheError::InvalidInput {
                path: self.environments_root().join(logical_name),
                reason: "全局环境名称非法",
            });
        }
        Ok(self.environments_root().join(logical_name))
    }
}

/// 已打开且具备写入能力的全局内容缓存。
#[derive(Clone, Debug)]
pub struct CacheStore {
    layout: CacheLayout,
}

impl CacheStore {
    /// 创建缓存目录并验证 `XIAO_HOME` 可写；失败时不会静默切换目录。
    pub fn open(layout: CacheLayout) -> Result<Self, CacheError> {
        let roots = [
            layout.source_objects_root(),
            layout.temporary_root(),
            layout.environments_root(),
        ];
        for root in roots {
            fs::create_dir_all(&root).map_err(|error| CacheError::HomeUnavailable {
                path: root.clone(),
                message: format!("创建缓存目录失败：{error}"),
            })?;
        }
        let probe_id = NEXT_TEMP_OBJECT.fetch_add(1, Ordering::Relaxed);
        let probe = layout
            .temporary_root()
            .join(format!(".write-probe-{}-{probe_id}", std::process::id()));
        fs::write(&probe, []).map_err(|error| CacheError::HomeUnavailable {
            path: probe.clone(),
            message: format!("写入能力检查失败：{error}"),
        })?;
        let _ = fs::remove_file(probe);
        Ok(Self { layout })
    }

    /// 返回缓存布局。
    #[must_use]
    pub fn layout(&self) -> &CacheLayout {
        &self.layout
    }

    /// 将本地路径包导入为经过摘要校验的不可变源码对象。
    pub fn import_source_directory(
        &self,
        source_root: impl AsRef<Path>,
    ) -> Result<CacheObject, CacheError> {
        let snapshot = collect_snapshot(source_root.as_ref())?;
        let digest = digest_snapshot(&snapshot);
        let object_path = self.layout.source_object_path(&digest)?;

        if fs::symlink_metadata(&object_path).is_ok() {
            match self.verify_source_object(&digest) {
                Ok(object) => return Ok(object),
                Err(CacheError::ObjectCorrupt { .. }) => {
                    quarantine_object(&object_path)?;
                }
                Err(error) => return Err(error),
            }
        }

        let temporary_path = self.write_snapshot(&snapshot, &digest)?;
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent).map_err(|error| CacheError::Write {
                path: parent.to_path_buf(),
                operation: "创建对象分片目录",
                message: error.to_string(),
            })?;
        }
        if let Err(error) = fs::rename(&temporary_path, &object_path) {
            remove_tree(&temporary_path);
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                return self.verify_source_object(&digest);
            }
            return Err(CacheError::Write {
                path: object_path,
                operation: "原子提交缓存对象",
                message: error.to_string(),
            });
        }
        if let Err(error) = set_tree_read_only(&object_path) {
            let _ = quarantine_object(&object_path);
            return Err(error);
        }
        self.verify_source_object(&digest)
    }

    /// 验证源码对象摘要并返回其实际路径；损坏对象会被隔离。
    pub fn verify_source_object(&self, digest: &str) -> Result<CacheObject, CacheError> {
        let object_path = self.layout.source_object_path(digest)?;
        let snapshot = match collect_snapshot(&object_path) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                if fs::symlink_metadata(&object_path).is_ok() {
                    let quarantine = quarantine_object(&object_path).ok();
                    return Err(CacheError::ObjectCorrupt {
                        path: object_path,
                        expected: digest.to_owned(),
                        actual: Some(error.to_string()),
                        quarantine,
                    });
                }
                return Err(CacheError::Write {
                    path: object_path,
                    operation: "读取缓存对象",
                    message: error.to_string(),
                });
            }
        };
        let actual = digest_snapshot(&snapshot);
        if actual != digest {
            let quarantine = quarantine_object(&object_path).ok();
            return Err(CacheError::ObjectCorrupt {
                path: object_path,
                expected: digest.to_owned(),
                actual: Some(actual),
                quarantine,
            });
        }
        Ok(CacheObject {
            reference: CacheObjectReference {
                digest: digest.to_owned(),
                object_kind: CacheObjectKind::Source,
            },
            path: object_path,
        })
    }

    /// 验证对象后读取其中一个相对文件，不在项目环境中创建副本。
    pub fn read_source_file(
        &self,
        digest: &str,
        relative_path: impl AsRef<Path>,
    ) -> Result<Vec<u8>, CacheError> {
        let object = self.verify_source_object(digest)?;
        let normalized = normalize_relative_path(relative_path.as_ref())?;
        let path = object.path.join(Path::new(&normalized));
        fs::read(&path).map_err(|error| CacheError::Write {
            path,
            operation: "读取缓存对象文件",
            message: error.to_string(),
        })
    }

    /// 将内存中的一致性快照写入同文件系统暂存目录。
    fn write_snapshot(
        &self,
        snapshot: &[SnapshotEntry],
        digest: &str,
    ) -> Result<PathBuf, CacheError> {
        let id = NEXT_TEMP_OBJECT.fetch_add(1, Ordering::Relaxed);
        let temporary_path = self
            .layout
            .temporary_root()
            .join(format!(".source-{digest}-{}-{id}", std::process::id()));
        fs::create_dir(&temporary_path).map_err(|error| CacheError::Write {
            path: temporary_path.clone(),
            operation: "创建缓存对象暂存目录",
            message: error.to_string(),
        })?;
        for entry in snapshot {
            let path = temporary_path.join(Path::new(&entry.relative_path));
            match &entry.kind {
                SnapshotKind::Directory => {
                    fs::create_dir_all(&path).map_err(|error| CacheError::Write {
                        path: path.clone(),
                        operation: "创建缓存对象目录",
                        message: error.to_string(),
                    })?;
                }
                SnapshotKind::File(content) => {
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent).map_err(|error| CacheError::Write {
                            path: parent.to_path_buf(),
                            operation: "创建缓存对象文件目录",
                            message: error.to_string(),
                        })?;
                    }
                    fs::write(&path, content).map_err(|error| CacheError::Write {
                        path: path.clone(),
                        operation: "写入缓存对象文件",
                        message: error.to_string(),
                    })?;
                }
            }
        }
        Ok(temporary_path)
    }
}

/// 为本地源码目录计算规范化内容的 SHA-256 摘要。
pub fn source_directory_digest(source_root: impl AsRef<Path>) -> Result<String, CacheError> {
    Ok(digest_snapshot(&collect_snapshot(source_root.as_ref())?))
}

/// 缓存路径、源码快照或对象完整性错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CacheError {
    /// `XIAO_HOME` 为相对路径或其他输入路径非法。
    InvalidInput {
        /// 触发错误的路径。
        path: PathBuf,
        /// 稳定的错误原因。
        reason: &'static str,
    },
    /// 缓存根目录无法创建或写入。
    HomeUnavailable {
        /// 无法访问的路径。
        path: PathBuf,
        /// 主机错误文本。
        message: String,
    },
    /// 源目录含有不支持的文件类型或路径。
    InvalidSource {
        /// 触发错误的路径。
        path: PathBuf,
        /// 稳定的错误原因。
        reason: &'static str,
    },
    /// 普通缓存读写失败。
    Write {
        /// 失败操作涉及的路径。
        path: PathBuf,
        /// 失败操作名称。
        operation: &'static str,
        /// 主机错误文本。
        message: String,
    },
    /// 既有对象的完整性校验失败并已尝试隔离。
    ObjectCorrupt {
        /// 损坏对象原路径。
        path: PathBuf,
        /// 映射要求的摘要。
        expected: String,
        /// 实际摘要或读取失败原因。
        actual: Option<String>,
        /// 隔离后的路径；无法隔离时为空。
        quarantine: Option<PathBuf>,
    },
}

impl CacheError {
    /// 返回稳定诊断编号。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput { .. } => CACHE_INVALID_INPUT_CODE,
            Self::HomeUnavailable { .. } => CACHE_HOME_UNAVAILABLE_CODE,
            Self::InvalidSource { .. } => CACHE_INVALID_SOURCE_CODE,
            Self::Write { .. } => CACHE_HOME_UNAVAILABLE_CODE,
            Self::ObjectCorrupt { .. } => CACHE_OBJECT_CORRUPT_CODE,
        }
    }
}

impl Display for CacheError {
    /// 输出带稳定编号的缓存诊断。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput { path, reason } => {
                write!(formatter, "{}: {reason}：{}", self.code(), path.display())
            }
            Self::HomeUnavailable { path, message } => {
                write!(
                    formatter,
                    "{}: 缓存路径 {} 不可用：{message}",
                    self.code(),
                    path.display()
                )
            }
            Self::InvalidSource { path, reason } => {
                write!(
                    formatter,
                    "{}: 源路径 {} 不支持：{reason}",
                    self.code(),
                    path.display()
                )
            }
            Self::Write {
                path,
                operation,
                message,
            } => write!(
                formatter,
                "{}: {operation} {} 失败：{message}",
                self.code(),
                path.display()
            ),
            Self::ObjectCorrupt {
                path,
                expected,
                actual,
                quarantine,
            } => write!(
                formatter,
                "{}: 缓存对象 {} 摘要校验失败（期望 {expected}，实际 {}，隔离到 {}）",
                self.code(),
                path.display(),
                actual.as_deref().unwrap_or("不可读取"),
                quarantine
                    .as_deref()
                    .map_or_else(|| "<失败>".to_owned(), |value| value.display().to_string())
            ),
        }
    }
}

impl std::error::Error for CacheError {}

/// 规范化源码树中的一个目录或文件条目。
#[derive(Clone, Debug, Eq, PartialEq)]
struct SnapshotEntry {
    relative_path: String,
    kind: SnapshotKind,
}

/// 规范化源码树条目的内容类型。
#[derive(Clone, Debug, Eq, PartialEq)]
enum SnapshotKind {
    Directory,
    File(Vec<u8>),
}

/// 读取源码目录并生成与根路径无关的排序快照。
fn collect_snapshot(root: &Path) -> Result<Vec<SnapshotEntry>, CacheError> {
    let metadata = fs::symlink_metadata(root).map_err(|error| CacheError::Write {
        path: root.to_path_buf(),
        operation: "读取源码目录",
        message: error.to_string(),
    })?;
    if metadata.file_type().is_symlink() {
        return Err(CacheError::InvalidSource {
            path: root.to_path_buf(),
            reason: "不跟随符号链接",
        });
    }
    if !metadata.is_dir() {
        return Err(CacheError::InvalidSource {
            path: root.to_path_buf(),
            reason: "源码根必须是目录",
        });
    }
    let mut entries = Vec::new();
    collect_snapshot_entries(root, Path::new(""), &mut entries)?;
    entries.sort_by(|left, right| {
        left.relative_path
            .as_bytes()
            .cmp(right.relative_path.as_bytes())
    });
    Ok(entries)
}

/// 递归收集一个目录下的规范化源码条目。
fn collect_snapshot_entries(
    directory: &Path,
    relative_directory: &Path,
    entries: &mut Vec<SnapshotEntry>,
) -> Result<(), CacheError> {
    let mut children = fs::read_dir(directory)
        .map_err(|error| CacheError::Write {
            path: directory.to_path_buf(),
            operation: "读取源码目录项",
            message: error.to_string(),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CacheError::Write {
            path: directory.to_path_buf(),
            operation: "读取源码目录项",
            message: error.to_string(),
        })?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let file_name = child.file_name();
        let segment = file_name
            .to_str()
            .ok_or_else(|| CacheError::InvalidSource {
                path: child.path(),
                reason: "路径必须是有效 UTF-8",
            })?;
        if segment.contains('\\') || segment.contains('/') {
            return Err(CacheError::InvalidSource {
                path: child.path(),
                reason: "路径分隔符不合法",
            });
        }
        let relative = if relative_directory.as_os_str().is_empty() {
            segment.to_owned()
        } else {
            format!(
                "{}/{}",
                relative_directory.to_string_lossy().replace('\\', "/"),
                segment
            )
        };
        let path = child.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| CacheError::Write {
            path: path.clone(),
            operation: "读取源码目录项",
            message: error.to_string(),
        })?;
        if metadata.file_type().is_symlink() {
            return Err(CacheError::InvalidSource {
                path,
                reason: "不跟随符号链接",
            });
        }
        if metadata.is_file() && is_generated_lockfile(segment) {
            continue;
        }
        if metadata.is_dir() {
            if path.join(GENERATED_ENVIRONMENT_METADATA_FILE).is_file() {
                continue;
            }
            entries.push(SnapshotEntry {
                relative_path: relative.clone(),
                kind: SnapshotKind::Directory,
            });
            collect_snapshot_entries(&path, Path::new(&relative), entries)?;
        } else if metadata.is_file() {
            let content = fs::read(&path).map_err(|error| CacheError::Write {
                path: path.clone(),
                operation: "读取源码文件",
                message: error.to_string(),
            })?;
            entries.push(SnapshotEntry {
                relative_path: relative,
                kind: SnapshotKind::File(content),
            });
        } else {
            return Err(CacheError::InvalidSource {
                path,
                reason: "只支持普通文件和目录",
            });
        }
    }
    Ok(())
}

/// 判断目录项是否为应从源码摘要排除的项目生成文件。
fn is_generated_lockfile(name: &str) -> bool {
    name == GENERATED_LOCKFILE || name.starts_with(".xiao.lock.json.tmp-")
}

/// 对规范化快照编码并计算 SHA-256 摘要。
fn digest_snapshot(snapshot: &[SnapshotEntry]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"xiao-source-object-v1\0");
    for entry in snapshot {
        match &entry.kind {
            SnapshotKind::Directory => hasher.update([b'D']),
            SnapshotKind::File(_) => hasher.update([b'F']),
        }
        append_length_prefixed(&mut hasher, entry.relative_path.as_bytes());
        if let SnapshotKind::File(content) = &entry.kind {
            append_length_prefixed(&mut hasher, content);
        }
    }
    encode_digest(&hasher.finalize())
}

/// 向摘要输入追加一个大端长度前缀字节字段。
fn append_length_prefixed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(
        u64::try_from(bytes.len())
            .expect("源码快照字段长度必须能表示为 64 位无符号整数")
            .to_be_bytes(),
    );
    hasher.update(bytes);
}

/// 将摘要字节编码为小写十六进制文本。
fn encode_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(b"0123456789abcdef"[(byte >> 4) as usize]));
        output.push(char::from(b"0123456789abcdef"[(byte & 0x0f) as usize]));
    }
    output
}

/// 校验对象摘要的完整长度和小写十六进制格式。
fn validate_digest(digest: &str) -> Result<(), CacheError> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(CacheError::InvalidInput {
            path: PathBuf::from(digest),
            reason: "对象摘要必须是 64 位小写十六进制文本",
        });
    }
    Ok(())
}

/// 将待读取对象文件路径规范化为安全的正斜杠相对路径。
fn normalize_relative_path(path: &Path) -> Result<String, CacheError> {
    let mut segments = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => {
                let segment = segment.to_str().ok_or_else(|| CacheError::InvalidInput {
                    path: path.to_path_buf(),
                    reason: "相对路径必须是有效 UTF-8",
                })?;
                if segment.contains('\\') || segment.contains('/') {
                    return Err(CacheError::InvalidInput {
                        path: path.to_path_buf(),
                        reason: "相对路径分隔符不合法",
                    });
                }
                segments.push(segment.to_owned());
            }
            Component::CurDir => {}
            _ => {
                return Err(CacheError::InvalidInput {
                    path: path.to_path_buf(),
                    reason: "对象文件路径必须是相对路径且不能穿越目录",
                });
            }
        }
    }
    if segments.is_empty() {
        return Err(CacheError::InvalidInput {
            path: path.to_path_buf(),
            reason: "对象文件路径不能为空",
        });
    }
    Ok(segments.join("/"))
}

/// 递归设置缓存对象只读属性和 Unix 只读权限。
fn set_tree_read_only(root: &Path) -> Result<(), CacheError> {
    let metadata = fs::symlink_metadata(root).map_err(|error| CacheError::Write {
        path: root.to_path_buf(),
        operation: "读取缓存对象权限",
        message: error.to_string(),
    })?;
    if metadata.file_type().is_symlink() {
        return Err(CacheError::InvalidSource {
            path: root.to_path_buf(),
            reason: "缓存对象不能包含符号链接",
        });
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(root).map_err(|error| CacheError::Write {
            path: root.to_path_buf(),
            operation: "读取缓存对象权限",
            message: error.to_string(),
        })? {
            let entry = entry.map_err(|error| CacheError::Write {
                path: root.to_path_buf(),
                operation: "读取缓存对象权限",
                message: error.to_string(),
            })?;
            set_tree_read_only(&entry.path())?;
        }
    }
    let mut permissions = metadata.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(root, permissions).map_err(|error| CacheError::Write {
        path: root.to_path_buf(),
        operation: "设置缓存对象只读属性",
        message: error.to_string(),
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if metadata.is_dir() { 0o555 } else { 0o444 };
        let mut permissions = fs::metadata(root)
            .map_err(|error| CacheError::Write {
                path: root.to_path_buf(),
                operation: "读取缓存对象权限",
                message: error.to_string(),
            })?
            .permissions();
        permissions.set_mode(mode);
        fs::set_permissions(root, permissions).map_err(|error| CacheError::Write {
            path: root.to_path_buf(),
            operation: "设置缓存对象 Unix 权限",
            message: error.to_string(),
        })?;
    }
    Ok(())
}

/// 为清理或隔离临时解除缓存对象的只读属性。
fn make_tree_writable(root: &Path) {
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return;
    };
    if metadata.file_type().is_symlink() {
        return;
    }
    if metadata.is_dir() {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                make_tree_writable(&entry.path());
            }
        }
    }
    let mut permissions = metadata.permissions();
    #[cfg(not(unix))]
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    let _ = fs::set_permissions(root, permissions);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(root)
            .map(|value| value.permissions())
            .unwrap_or_else(|_| metadata.permissions());
        permissions.set_mode(if metadata.is_dir() { 0o755 } else { 0o644 });
        let _ = fs::set_permissions(root, permissions);
    }
}

/// 清理一个可能包含只读文件的暂存目录。
fn remove_tree(root: &Path) {
    make_tree_writable(root);
    let _ = fs::remove_dir_all(root);
}

/// 将损坏对象移动到同分片目录下的唯一隔离名称。
fn quarantine_object(path: &Path) -> Result<PathBuf, CacheError> {
    make_tree_writable(path);
    let parent = path.parent().ok_or_else(|| CacheError::Write {
        path: path.to_path_buf(),
        operation: "隔离损坏缓存对象",
        message: "对象没有父目录".to_owned(),
    })?;
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| CacheError::Write {
            path: path.to_path_buf(),
            operation: "隔离损坏缓存对象",
            message: "对象名称不是有效 UTF-8".to_owned(),
        })?;
    for suffix in 0..10_000_u32 {
        let candidate = if suffix == 0 {
            parent.join(format!("{name}.corrupt"))
        } else {
            parent.join(format!("{name}.corrupt.{suffix}"))
        };
        if fs::symlink_metadata(&candidate).is_ok() {
            continue;
        }
        fs::rename(path, &candidate).map_err(|error| CacheError::Write {
            path: path.to_path_buf(),
            operation: "隔离损坏缓存对象",
            message: error.to_string(),
        })?;
        return Ok(candidate);
    }
    Err(CacheError::Write {
        path: path.to_path_buf(),
        operation: "隔离损坏缓存对象",
        message: "损坏对象隔离名称耗尽".to_owned(),
    })
}

/// 按宿主平台环境变量确定用户主目录。
fn user_home_directory() -> Result<PathBuf, CacheError> {
    #[cfg(windows)]
    let home = env::var_os("USERPROFILE").or_else(|| {
        let drive = env::var_os("HOMEDRIVE")?;
        let path = env::var_os("HOMEPATH")?;
        Some(PathBuf::from(drive).join(path).into_os_string())
    });
    #[cfg(not(windows))]
    let home = env::var_os("HOME");
    home.map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| CacheError::HomeUnavailable {
            path: PathBuf::from("<user-home>"),
            message: "无法确定用户主目录".to_owned(),
        })
}
