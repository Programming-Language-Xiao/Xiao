//! 16A 内容寻址对象、完整性校验和确定性二进制索引。
//!
//! 对象内容以完整字节计算 SHA-256；文件名只提供期望摘要，读取时始终重算。
//! 对象写入采用同文件系统临时文件加原子改名，碰撞和损坏对象进入隔离区。

#![allow(clippy::result_large_err)]

use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// 首版索引主版本。
pub const INDEX_SCHEMA_MAJOR: u32 = 1;
/// 首版索引次版本。
pub const INDEX_SCHEMA_MINOR: u32 = 0;
/// 首版索引没有额外必需能力位。
pub const INDEX_REQUIRED_FEATURES: u64 = 0;
/// 首版归档索引记录类型。
pub const ARCHIVE_INDEX_RECORD_TYPE: u32 = 1;
/// 首版全局缓存索引记录类型。
pub const GLOBAL_INDEX_RECORD_TYPE: u32 = 2;

/// 进程内临时文件和隔离文件的单调序列。
static NEXT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 内容寻址对象的独立命名空间。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ObjectKind {
    /// 源码或依赖内容。
    Source,
    /// 规范化 `.xiaoc` 字节码。
    Xiaoc,
    /// 原生构建片段或最终二进制。
    Native,
    /// `.xar` 归档。
    Xar,
    /// 语言包消息目录。
    Language,
}

impl ObjectKind {
    /// 返回稳定的目录名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Xiaoc => "xiaoc",
            Self::Native => "native",
            Self::Xar => "xar",
            Self::Language => "language",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Xiaoc => "xiaoc",
            Self::Native => "bin",
            Self::Xar => "xar",
            Self::Language => "language",
        }
    }
}

/// 规范化的 32 字节 SHA-256 摘要。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Digest256([u8; 32]);

impl Digest256 {
    /// 对完整字节计算 SHA-256。
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// 从流式读取器计算 SHA-256，并返回读取字节数。
    pub fn of_reader(reader: &mut impl Read) -> Result<(Self, u64), ArtifactError> {
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        let mut length = 0_u64;
        loop {
            let read = reader.read(&mut buffer).map_err(ArtifactError::Io)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            length = length
                .checked_add(read as u64)
                .ok_or(ArtifactError::SizeOverflow)?;
        }
        Ok((Self(hasher.finalize().into()), length))
    }

    /// 返回 64 字符小写十六进制文本。
    #[must_use]
    pub fn as_hex(self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// 从严格的 64 字符小写十六进制文本解析摘要。
    pub fn parse(value: &str) -> Result<Self, ArtifactError> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(ArtifactError::InvalidDigest(value.to_owned()));
        }
        let mut bytes = [0_u8; 32];
        for (index, slot) in bytes.iter_mut().enumerate() {
            *slot = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
                .map_err(|_| ArtifactError::InvalidDigest(value.to_owned()))?;
        }
        Ok(Self(bytes))
    }
}

impl Display for Digest256 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.as_hex())
    }
}

/// 内容寻址对象的稳定引用。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectReference {
    /// 对象类型命名空间。
    pub kind: ObjectKind,
    /// 完整 SHA-256 摘要。
    pub digest: Digest256,
    /// 完整字节长度。
    pub length: u64,
    /// 对象实际路径。
    pub path: PathBuf,
}

/// 清理时保护的一条对象引用；`None` 表示该摘要在所有对象命名空间中都保留。
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CacheReference {
    /// 对象类型；未知来源时使用 `None` 采取保守保护。
    pub object_kind: Option<ObjectKind>,
    /// 完整 SHA-256 摘要。
    pub digest: Digest256,
}

impl CacheReference {
    /// 创建指定命名空间的精确引用。
    #[must_use]
    pub const fn exact(object_kind: ObjectKind, digest: Digest256) -> Self {
        Self {
            object_kind: Some(object_kind),
            digest,
        }
    }

    /// 创建保护所有命名空间中同摘要对象的引用。
    #[must_use]
    pub const fn any_namespace(digest: Digest256) -> Self {
        Self {
            object_kind: None,
            digest,
        }
    }
}

/// 清理前现算的引用集合，不写入缓存。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReferenceSet {
    references: BTreeSet<CacheReference>,
}

impl ReferenceSet {
    /// 创建空引用集合。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 加入一条引用。
    pub fn insert(&mut self, reference: CacheReference) {
        self.references.insert(reference);
    }

    /// 返回规范排序的引用。
    pub fn iter(&self) -> impl Iterator<Item = &CacheReference> {
        self.references.iter()
    }

    fn protects(&self, kind: ObjectKind, digest: Digest256) -> bool {
        self.references
            .contains(&CacheReference::exact(kind, digest))
            || self
                .references
                .contains(&CacheReference::any_namespace(digest))
    }
}

/// 两阶段清理的只读计划。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupPlan {
    candidates: Vec<ObjectReference>,
    quarantine_candidates: Vec<PathBuf>,
}

impl CleanupPlan {
    /// 返回本计划准备删除的已验证对象。
    #[must_use]
    pub fn candidates(&self) -> &[ObjectReference] {
        &self.candidates
    }
}

/// 清理执行结果。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CleanupReport {
    /// 成功删除的对象。
    pub removed: Vec<ObjectReference>,
    /// 执行期间发现并隔离的损坏对象路径。
    pub quarantined: Vec<PathBuf>,
}

/// 对象操作失败原因。
#[derive(Debug)]
pub enum ArtifactError {
    /// 文件系统错误。
    Io(io::Error),
    /// 摘要文本不符合 64 位小写格式。
    InvalidDigest(String),
    /// 文件过大导致长度溢出。
    SizeOverflow,
    /// 对象命名、魔数或扩展名不匹配。
    InvalidObject {
        /// 对象路径。
        path: PathBuf,
        /// 无效原因。
        reason: String,
    },
    /// 目标摘要已经存在但内容不一致。
    HashCollision {
        /// 发生碰撞的对象或隔离路径。
        path: PathBuf,
        /// 目标摘要。
        digest: Digest256,
    },
    /// 已存在对象损坏，且已移动到隔离区。
    CorruptObject {
        /// 原对象路径。
        path: PathBuf,
        /// 期望摘要。
        expected: Digest256,
        /// 实际摘要。
        actual: Option<Digest256>,
        /// 隔离路径。
        quarantine: Option<PathBuf>,
    },
    /// 索引编码或解码失败。
    Index(String),
    /// 缓存维护或引用来源读取失败。
    Maintenance(String),
    /// 跨进程锁失败。
    Lock(String),
    /// 索引主版本或记录类型不支持。
    UnsupportedIndexVersion {
        /// 收到的主版本。
        major: u32,
        /// 索引记录类型。
        record_type: u32,
    },
}

impl Display for ArtifactError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O 错误：{error}"),
            Self::InvalidDigest(value) => write!(formatter, "摘要无效：{value}"),
            Self::SizeOverflow => formatter.write_str("对象长度溢出"),
            Self::InvalidObject { path, reason } => {
                write!(formatter, "对象无效（{}）：{reason}", path.display())
            }
            Self::HashCollision { path, digest } => {
                write!(formatter, "摘要碰撞（{}）：{}", path.display(), digest)
            }
            Self::CorruptObject {
                path,
                expected,
                actual,
                quarantine,
            } => write!(
                formatter,
                "对象损坏（{}）：期望 {expected}，实际 {actual:?}，隔离 {quarantine:?}",
                path.display()
            ),
            Self::Index(message) => write!(formatter, "索引错误：{message}"),
            Self::Maintenance(message) => write!(formatter, "缓存维护错误：{message}"),
            Self::Lock(message) => write!(formatter, "锁错误：{message}"),
            Self::UnsupportedIndexVersion { major, record_type } => write!(
                formatter,
                "不支持的索引主版本或记录类型：major={major}、type={record_type}"
            ),
        }
    }
}

impl std::error::Error for ArtifactError {}
impl From<io::Error> for ArtifactError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// 内容寻址对象根目录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    /// 创建对象存储并确保临时、隔离目录存在。
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, ArtifactError> {
        let root = root.into();
        fs::create_dir_all(root.join("objects"))?;
        fs::create_dir_all(root.join("quarantine"))?;
        fs::create_dir_all(root.join("tmp"))?;
        Ok(Self { root })
    }

    /// 返回对象根目录。
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 返回指定类型和摘要的不可变对象路径。
    pub fn object_path(&self, kind: ObjectKind, digest: Digest256) -> PathBuf {
        self.root
            .join("objects")
            .join(kind.as_str())
            .join("sha256")
            .join(&digest.as_hex()[..2])
            .join(format!("{}.{}", digest, kind.extension()))
    }

    /// 写入任意对象；目标已存在时比较完整字节，不静默覆盖。
    pub fn put(&self, kind: ObjectKind, bytes: &[u8]) -> Result<ObjectReference, ArtifactError> {
        self.put_reader(kind, &mut io::Cursor::new(bytes))
    }

    /// 流式写入对象并返回完整引用。
    pub fn put_reader(
        &self,
        kind: ObjectKind,
        reader: &mut impl Read,
    ) -> Result<ObjectReference, ArtifactError> {
        let _maintenance = xiao_lock::EntryLock::acquire(&self.root.join("maintenance.lock"))
            .map_err(lock_error)?;
        let sequence = NEXT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = self
            .root
            .join("tmp")
            .join(format!("incoming-{}-{sequence}", std::process::id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        let mut length = 0_u64;
        loop {
            let read = match reader.read(&mut buffer) {
                Ok(read) => read,
                Err(error) => {
                    let _ = fs::remove_file(&temporary);
                    return Err(ArtifactError::Io(error));
                }
            };
            if read == 0 {
                break;
            }
            if let Err(error) = file.write_all(&buffer[..read]) {
                let _ = fs::remove_file(&temporary);
                return Err(ArtifactError::Io(error));
            }
            hasher.update(&buffer[..read]);
            length = length.checked_add(read as u64).ok_or_else(|| {
                let _ = fs::remove_file(&temporary);
                ArtifactError::SizeOverflow
            })?;
        }
        if let Err(error) = file.sync_all() {
            let _ = fs::remove_file(&temporary);
            return Err(ArtifactError::Io(error));
        }
        drop(file);
        if let Err(error) = make_immutable(&temporary) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        let digest = Digest256(hasher.finalize().into());
        let path = self.object_path(kind, digest);
        if path.exists() {
            let equal = match files_equal(&path, &temporary) {
                Ok(equal) => equal,
                Err(error) => {
                    let _ = fs::remove_file(&temporary);
                    return Err(error);
                }
            };
            if equal {
                let _ = fs::remove_file(&temporary);
                make_immutable(&path)?;
                return Ok(ObjectReference {
                    kind,
                    digest,
                    length,
                    path,
                });
            }
            let quarantine = self.quarantine_collision(&path, &temporary)?;
            return Err(ArtifactError::HashCollision {
                path: quarantine,
                digest,
            });
        }
        let parent = path.parent().ok_or_else(|| ArtifactError::InvalidObject {
            path: path.clone(),
            reason: "对象没有父目录".to_owned(),
        })?;
        if let Err(error) = fs::create_dir_all(parent) {
            let _ = fs::remove_file(&temporary);
            return Err(ArtifactError::Io(error));
        }
        if let Err(error) = fs::rename(&temporary, &path) {
            if error.kind() == io::ErrorKind::AlreadyExists {
                let equal = match files_equal(&path, &temporary) {
                    Ok(equal) => equal,
                    Err(error) => {
                        let _ = fs::remove_file(&temporary);
                        return Err(error);
                    }
                };
                if equal {
                    let _ = fs::remove_file(&temporary);
                    make_immutable(&path)?;
                    return Ok(ObjectReference {
                        kind,
                        digest,
                        length,
                        path,
                    });
                }
                let quarantine = self.quarantine_collision(&path, &temporary)?;
                return Err(ArtifactError::HashCollision {
                    path: quarantine,
                    digest,
                });
            }
            let _ = fs::remove_file(&temporary);
            return Err(ArtifactError::Io(error));
        }
        Ok(ObjectReference {
            kind,
            digest,
            length,
            path,
        })
    }

    /// 写入完整 `.xiaoc` 对象并执行权威格式校验。
    pub fn put_xiaoc(&self, bytes: &[u8]) -> Result<ObjectReference, ArtifactError> {
        xiao_bytecode::validate_xiaoc(bytes).map_err(|error| ArtifactError::InvalidObject {
            path: PathBuf::from("<memory>"),
            reason: format!("`.xiaoc` 格式校验失败：{error}"),
        })?;
        self.put(ObjectKind::Xiaoc, bytes)
    }

    /// 读取对象前重算摘要、校验长度和对象后缀；损坏对象会隔离。
    pub fn read(&self, kind: ObjectKind, digest: Digest256) -> Result<Vec<u8>, ArtifactError> {
        self.read_checked(kind, digest, None)
    }

    /// 读取对象并额外校验索引提供的完整字节长度。
    pub fn read_checked(
        &self,
        kind: ObjectKind,
        digest: Digest256,
        expected_length: Option<u64>,
    ) -> Result<Vec<u8>, ArtifactError> {
        let path = self.object_path(kind, digest);
        let bytes = fs::read(&path).map_err(ArtifactError::Io)?;
        let actual = Digest256::of_bytes(&bytes);
        if actual != digest {
            let quarantine = self.quarantine_path(&path).ok();
            return Err(ArtifactError::CorruptObject {
                path,
                expected: digest,
                actual: Some(actual),
                quarantine,
            });
        }
        if expected_length.is_some_and(|length| length != bytes.len() as u64) {
            let quarantine = self.quarantine_path(&path).ok();
            return Err(ArtifactError::CorruptObject {
                path,
                expected: digest,
                actual: Some(actual),
                quarantine,
            });
        }
        if kind == ObjectKind::Xiaoc && xiao_bytecode::validate_xiaoc(&bytes).is_err() {
            let quarantine = self.quarantine_path(&path).ok();
            return Err(ArtifactError::CorruptObject {
                path,
                expected: digest,
                actual: Some(actual),
                quarantine,
            });
        }
        Ok(bytes)
    }

    /// 按摘要扫描一个全局对象命名空间，仅读取格式正确的对象。
    pub fn scan(&self, kind: ObjectKind) -> Result<Vec<ObjectReference>, ArtifactError> {
        let root = self.root.join("objects").join(kind.as_str()).join("sha256");
        let mut objects = Vec::new();
        if !root.exists() {
            return Ok(objects);
        }
        for shard in fs::read_dir(&root)? {
            let shard = shard?.path();
            if !shard.is_dir() {
                continue;
            }
            for entry in fs::read_dir(shard)? {
                let path = entry?.path();
                if !path.is_file() {
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some(kind.extension()) {
                    self.quarantine_path(&path)?;
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                    self.quarantine_path(&path)?;
                    continue;
                };
                let digest = match Digest256::parse(stem) {
                    Ok(value) => value,
                    Err(_) => {
                        self.quarantine_path(&path)?;
                        continue;
                    }
                };
                if path
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|value| value.to_str())
                    != Some(&digest.as_hex()[..2])
                {
                    self.quarantine_path(&path)?;
                    continue;
                }
                let bytes = match self.read_checked(kind, digest, None) {
                    Ok(bytes) => bytes,
                    Err(ArtifactError::CorruptObject { .. }) => continue,
                    Err(error) => return Err(error),
                };
                objects.push(ObjectReference {
                    kind,
                    digest,
                    length: bytes.len() as u64,
                    path,
                });
            }
        }
        objects.sort_by_key(|object| object.digest);
        Ok(objects)
    }

    /// 从项目 `xiao.lock.json`、归档索引和显式引用现算保护集合。
    ///
    /// 任一显式来源缺失、损坏或无法解析都会失败，调用方不得把失败来源当作无引用。
    pub fn collect_references(
        &self,
        project_lock: Option<&Path>,
        archive_indexes: &[PathBuf],
        explicit: &[CacheReference],
    ) -> Result<ReferenceSet, ArtifactError> {
        let mut references = ReferenceSet::new();
        for reference in explicit {
            references.insert(reference.clone());
        }
        if let Some(path) = project_lock {
            collect_project_lock_references(path, &mut references)?;
        }
        for path in archive_indexes {
            let bytes = fs::read(path).map_err(|error| {
                ArtifactError::Maintenance(format!("无法读取归档索引 {}：{error}", path.display()))
            })?;
            let index = ArchiveIndex::decode(&bytes)?;
            for entry in index.entries {
                references.insert(CacheReference::exact(entry.object_kind, entry.digest));
            }
        }
        Ok(references)
    }

    /// 只读扫描所有命名空间，生成不修改文件系统的清理计划。
    pub fn plan_cleanup(&self, protected: &ReferenceSet) -> Result<CleanupPlan, ArtifactError> {
        let mut candidates = Vec::new();
        let mut quarantine_candidates = Vec::new();
        for kind in [
            ObjectKind::Source,
            ObjectKind::Xiaoc,
            ObjectKind::Native,
            ObjectKind::Xar,
            ObjectKind::Language,
        ] {
            let (objects, invalid) = self.scan_read_only(kind)?;
            quarantine_candidates.extend(invalid);
            for object in objects {
                if !protected.protects(kind, object.digest) {
                    candidates.push(object);
                }
            }
        }
        candidates.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then(left.digest.cmp(&right.digest))
        });
        Ok(CleanupPlan {
            candidates,
            quarantine_candidates,
        })
    }

    /// 在调用方确认计划并重扫引用后，重新验证并删除仍未引用的对象。
    ///
    /// 调用方必须在执行期间暂停会发布对象引用的构建与归档操作；来源读取失败时不能调用本方法。
    /// 损坏对象会进入隔离区而不是被删除。
    pub fn apply_cleanup(
        &self,
        plan: &CleanupPlan,
        current_references: &ReferenceSet,
    ) -> Result<CleanupReport, ArtifactError> {
        let _lock = xiao_lock::EntryLock::acquire(&self.root.join("maintenance.lock"))
            .map_err(lock_error)?;
        let mut report = CleanupReport::default();
        for path in &plan.quarantine_candidates {
            if path.exists() {
                report.quarantined.push(self.quarantine_path(path)?);
            }
        }
        for candidate in &plan.candidates {
            if current_references.protects(candidate.kind, candidate.digest) {
                continue;
            }
            let expected_path = self.object_path(candidate.kind, candidate.digest);
            if candidate.path != expected_path {
                return Err(ArtifactError::Maintenance(
                    "清理计划包含不属于对象存储的路径".to_owned(),
                ));
            }
            match self.read_checked(candidate.kind, candidate.digest, Some(candidate.length)) {
                Ok(_) => match fs::remove_file(&candidate.path) {
                    Ok(()) => report.removed.push(candidate.clone()),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(ArtifactError::Io(error)),
                },
                Err(ArtifactError::CorruptObject { quarantine, .. }) => {
                    if let Some(path) = quarantine {
                        report.quarantined.push(path);
                    }
                }
                Err(ArtifactError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(report)
    }

    fn scan_read_only(
        &self,
        kind: ObjectKind,
    ) -> Result<(Vec<ObjectReference>, Vec<PathBuf>), ArtifactError> {
        let root = self.root.join("objects").join(kind.as_str()).join("sha256");
        let mut objects = Vec::new();
        let mut invalid = Vec::new();
        if !root.exists() {
            return Ok((objects, invalid));
        }
        for shard in fs::read_dir(&root)? {
            let shard = shard?.path();
            if !shard.is_dir() {
                continue;
            }
            for entry in fs::read_dir(&shard)? {
                let path = entry?.path();
                if !path.is_file() {
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some(kind.extension()) {
                    invalid.push(path);
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                    invalid.push(path);
                    continue;
                };
                let digest = match Digest256::parse(stem) {
                    Ok(digest) => digest,
                    Err(_) => {
                        invalid.push(path);
                        continue;
                    }
                };
                if path
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|value| value.to_str())
                    != Some(&digest.as_hex()[..2])
                {
                    invalid.push(path);
                    continue;
                }
                let bytes = fs::read(&path)?;
                if Digest256::of_bytes(&bytes) != digest {
                    invalid.push(path);
                    continue;
                }
                if kind == ObjectKind::Xiaoc && xiao_bytecode::validate_xiaoc(&bytes).is_err() {
                    invalid.push(path);
                    continue;
                }
                objects.push(ObjectReference {
                    kind,
                    digest,
                    length: bytes.len() as u64,
                    path,
                });
            }
        }
        objects.sort_by_key(|object| object.digest);
        invalid.sort();
        Ok((objects, invalid))
    }

    fn quarantine_path(&self, path: &Path) -> Result<PathBuf, ArtifactError> {
        let sequence = NEXT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("object");
        let destination = self
            .root
            .join("quarantine")
            .join(format!("{name}.corrupt-{}-{sequence}", std::process::id()));
        if path.exists() {
            fs::rename(path, &destination)?;
        }
        Ok(destination)
    }

    fn quarantine_collision(
        &self,
        existing: &Path,
        incoming: &Path,
    ) -> Result<PathBuf, ArtifactError> {
        let existing_quarantine = self.quarantine_path(existing)?;
        let _incoming_quarantine = self.quarantine_path(incoming)?;
        Ok(existing_quarantine)
    }

    /// 扫描所有已验证对象并重建全局索引。
    pub fn rebuild_global<F>(&self, mut describe: F) -> Result<GlobalIndex, ArtifactError>
    where
        F: FnMut(&ObjectReference) -> Option<GlobalRecord>,
    {
        let mut records = Vec::new();
        for kind in [
            ObjectKind::Source,
            ObjectKind::Xiaoc,
            ObjectKind::Native,
            ObjectKind::Xar,
            ObjectKind::Language,
        ] {
            for object in self.scan(kind)? {
                if let Some(record) = describe(&object) {
                    if record.object_kind != object.kind
                        || record.digest != object.digest
                        || record.length != object.length
                    {
                        return Err(ArtifactError::Index(
                            "扫描重建回调返回的对象引用不匹配".to_owned(),
                        ));
                    }
                    records.push(record);
                }
            }
        }
        records.sort_by(|left, right| {
            left.request_key
                .cmp(&right.request_key)
                .then(left.digest.cmp(&right.digest))
        });
        Ok(GlobalIndex {
            schema_major: INDEX_SCHEMA_MAJOR,
            schema_minor: INDEX_SCHEMA_MINOR,
            records,
        })
    }
}

fn collect_project_lock_references(
    path: &Path,
    references: &mut ReferenceSet,
) -> Result<(), ArtifactError> {
    let text = fs::read_to_string(path).map_err(|error| {
        ArtifactError::Maintenance(format!("无法读取项目锁文件 {}：{error}", path.display()))
    })?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|error| {
        ArtifactError::Maintenance(format!("项目锁文件 {} 无效：{error}", path.display()))
    })?;
    let lock_version = value
        .get("lock_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ArtifactError::Maintenance("项目锁文件缺少 lock_version".to_owned()))?;
    if lock_version != 1 && lock_version != 2 {
        return Err(ArtifactError::Maintenance(
            "项目锁文件 lock_version 不受支持".to_owned(),
        ));
    }
    if value
        .get("config_fingerprint")
        .and_then(serde_json::Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(ArtifactError::Maintenance(
            "项目锁文件缺少 config_fingerprint".to_owned(),
        ));
    }
    let root = value
        .get("root")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| ArtifactError::Maintenance("项目锁文件缺少 root".to_owned()))?;
    let root_key = lock_package_key(root)?;
    let packages = value
        .get("packages")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| ArtifactError::Maintenance("项目锁文件缺少 packages".to_owned()))?;
    if !packages.contains_key(&root_key) {
        return Err(ArtifactError::Maintenance(
            "项目锁文件 packages 缺少 root 条目".to_owned(),
        ));
    }
    for (stored_key, package) in packages {
        let package = package
            .as_object()
            .ok_or_else(|| ArtifactError::Maintenance("项目锁包条目不是对象".to_owned()))?;
        let package_key = lock_package_key(package)?;
        if stored_key != &package_key {
            return Err(ArtifactError::Maintenance(
                "项目锁包条目键与身份不一致".to_owned(),
            ));
        }
        if package
            .get("dependencies")
            .and_then(serde_json::Value::as_object)
            .is_none()
        {
            return Err(ArtifactError::Maintenance(
                "项目锁包条目缺少 dependencies".to_owned(),
            ));
        }
        let content_digest = package_string(package, "content_digest")?;
        let digest = Digest256::parse(content_digest)?;
        references.insert(CacheReference::exact(ObjectKind::Source, digest));
        if let Some(artifact) = package.get("source_artifact") {
            if lock_version < 2 {
                return Err(ArtifactError::Maintenance(
                    "旧版项目锁不能包含 source_artifact".to_owned(),
                ));
            }
            let artifact = artifact
                .as_object()
                .ok_or_else(|| ArtifactError::Maintenance("source_artifact 不是对象".to_owned()))?;
            let digest = package_string(artifact, "digest")?;
            if package_string(artifact, "location")?.is_empty()
                || artifact
                    .get("length")
                    .and_then(serde_json::Value::as_u64)
                    .is_none_or(|length| length == 0)
            {
                return Err(ArtifactError::Maintenance(
                    "source_artifact 的位置或长度无效".to_owned(),
                ));
            }
            references.insert(CacheReference::any_namespace(Digest256::parse(digest)?));
        }
    }
    Ok(())
}

fn package_string<'a>(
    package: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<&'a str, ArtifactError> {
    package
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ArtifactError::Maintenance(format!("项目锁包条目缺少 {field}")))
}

fn lock_package_key(
    package: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, ArtifactError> {
    let name = package_string(package, "name")?;
    let version = package_string(package, "version")?;
    let source = package
        .get("source")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| ArtifactError::Maintenance("项目锁包条目缺少 source".to_owned()))?;
    let source_id = package_string(source, "source_id")?;
    Ok(format!("{name}@{version}[{source_id}]"))
}

/// 索引快照的原子读写边界。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexStore {
    root: PathBuf,
}

impl IndexStore {
    /// 创建索引根目录。
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, ArtifactError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// 原子写入全局缓存索引快照。
    pub fn write_global(&self, index: &GlobalIndex) -> Result<PathBuf, ArtifactError> {
        let _lock = xiao_lock::EntryLock::acquire(&self.root.join("global.index.lock"))
            .map_err(lock_error)?;
        let path = self.root.join("global.index.pb");
        write_index_atomic(&path, &index.encode()?)
    }

    /// 读取并解码全局缓存索引。
    pub fn read_global(&self) -> Result<GlobalIndex, ArtifactError> {
        GlobalIndex::decode(&fs::read(self.root.join("global.index.pb"))?)
    }

    /// 扫描已验证对象、加锁并原子提交重建后的全局索引。
    pub fn rebuild_global_atomic<F>(
        &self,
        artifacts: &ArtifactStore,
        describe: F,
    ) -> Result<PathBuf, ArtifactError>
    where
        F: FnMut(&ObjectReference) -> Option<GlobalRecord>,
    {
        let index = artifacts.rebuild_global(describe)?;
        self.write_global(&index)
    }

    /// 原子写入归档索引；归档调用方负责把该字节作为唯一索引成员提交。
    pub fn write_archive(
        &self,
        path: impl AsRef<Path>,
        index: &ArchiveIndex,
    ) -> Result<PathBuf, ArtifactError> {
        let lock = path.as_ref().with_extension("index.lock");
        if let Some(parent) = lock.parent() {
            fs::create_dir_all(parent)?;
        }
        let _lock = xiao_lock::EntryLock::acquire(&lock).map_err(lock_error)?;
        write_index_atomic(path.as_ref(), &index.encode()?)
    }

    /// 读取并解码归档索引；缺失或损坏时直接失败，不扫描归档成员猜测重建。
    pub fn read_archive(&self, path: impl AsRef<Path>) -> Result<ArchiveIndex, ArtifactError> {
        let lock = path.as_ref().with_extension("index.lock");
        if let Some(parent) = lock.parent() {
            fs::create_dir_all(parent)?;
        }
        ArchiveIndex::decode(&fs::read(path)?)
    }
}

fn write_index_atomic(path: &Path, bytes: &[u8]) -> Result<PathBuf, ArtifactError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let sequence = NEXT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_extension(format!("tmp-{}-{sequence}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    if let Err(error) = file.write_all(bytes) {
        let _ = fs::remove_file(&temporary);
        return Err(ArtifactError::Io(error));
    }
    if let Err(error) = file.sync_all() {
        let _ = fs::remove_file(&temporary);
        return Err(ArtifactError::Io(error));
    }
    drop(file);
    if let Err(error) = replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(ArtifactError::Io(error));
    }
    Ok(path.to_path_buf())
}

fn make_immutable(path: &Path) -> Result<(), ArtifactError> {
    let mut permissions = fs::metadata(path)?.permissions();
    #[cfg(unix)]
    permissions.set_mode(0o444);
    #[cfg(not(unix))]
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary, destination)
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let source: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: both strings are NUL-terminated UTF-16 buffers owned for this call.
    let replaced = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn lock_error(error: xiao_lock::LockError) -> ArtifactError {
    ArtifactError::Lock(format!("{}：{error}", error.path().display()))
}

fn files_equal(first: &Path, second: &Path) -> Result<bool, ArtifactError> {
    let first_file = File::open(first)?;
    let second_file = File::open(second)?;
    if first_file.metadata()?.len() != second_file.metadata()?.len() {
        return Ok(false);
    }
    let mut first_reader = io::BufReader::new(first_file);
    let mut second_reader = io::BufReader::new(second_file);
    let mut first_buffer = [0_u8; 64 * 1024];
    let mut second_buffer = [0_u8; 64 * 1024];
    loop {
        let first_read = first_reader.read(&mut first_buffer)?;
        let second_read = second_reader.read(&mut second_buffer)?;
        if first_read != second_read {
            return Ok(false);
        }
        if first_read == 0 {
            return Ok(true);
        }
        if first_buffer[..first_read] != second_buffer[..second_read] {
            return Ok(false);
        }
    }
}

/// 归档索引中的一条不可变对象记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveEntry {
    /// 归档内的逻辑路径。
    pub logical_path: String,
    /// 对象类型。
    pub object_kind: ObjectKind,
    /// 对象摘要。
    pub digest: Digest256,
    /// 模块逻辑名称。
    pub module: String,
    /// 目标平台或架构。
    pub target: String,
    /// 规范对象字节长度。
    pub length: u64,
}

/// `.xar` 内唯一的归档索引。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveIndex {
    /// Schema 主版本。
    pub schema_major: u32,
    /// Schema 次版本。
    pub schema_minor: u32,
    /// 入口逻辑路径。
    pub entry: String,
    /// 按规范键排序的对象条目。
    pub entries: Vec<ArchiveEntry>,
}

/// 全局缓存索引中的构建引用记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalRecord {
    /// 构建请求键。
    pub request_key: String,
    /// 对象类型。
    pub object_kind: ObjectKind,
    /// 对象摘要。
    pub digest: Digest256,
    /// 目标平台。
    pub target: String,
    /// 优化级别。
    pub optimization_level: u32,
    /// 代码生成版本。
    pub codegen_version: u32,
    /// 对象字节长度。
    pub length: u64,
}

/// 本机全局缓存索引。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalIndex {
    /// Schema 主版本。
    pub schema_major: u32,
    /// Schema 次版本。
    pub schema_minor: u32,
    /// 按 request_key 排序的记录。
    pub records: Vec<GlobalRecord>,
}

impl ArchiveIndex {
    /// 以确定性 protobuf wire 格式编码。
    pub fn encode(&self) -> Result<Vec<u8>, ArtifactError> {
        if self.schema_major != INDEX_SCHEMA_MAJOR {
            return Err(ArtifactError::UnsupportedIndexVersion {
                major: self.schema_major,
                record_type: ARCHIVE_INDEX_RECORD_TYPE,
            });
        }
        validate_archive_index(self)?;
        let mut entries = self.entries.clone();
        entries.sort_by(|left, right| {
            left.logical_path
                .cmp(&right.logical_path)
                .then(left.digest.cmp(&right.digest))
        });
        let mut output = Vec::new();
        put_varint_field(&mut output, 1, self.schema_major as u64);
        put_varint_field(&mut output, 2, self.schema_minor as u64);
        put_bytes_field(&mut output, 3, self.entry.as_bytes());
        for entry in entries {
            put_bytes_field(&mut output, 4, &encode_archive_entry(&entry)?);
        }
        put_varint_field(&mut output, 5, u64::from(ARCHIVE_INDEX_RECORD_TYPE));
        put_varint_field(&mut output, 6, INDEX_REQUIRED_FEATURES);
        Ok(output)
    }

    /// 解码并检查主版本。
    pub fn decode(bytes: &[u8]) -> Result<Self, ArtifactError> {
        let mut reader = ProtoReader::new(bytes);
        let mut index = Self {
            schema_major: 0,
            schema_minor: 0,
            entry: String::new(),
            entries: Vec::new(),
        };
        let mut record_type = None;
        let mut required_features = INDEX_REQUIRED_FEATURES;
        while let Some((field, wire)) = reader.next_key()? {
            match field {
                1 => index.schema_major = reader.varint(wire)? as u32,
                2 => index.schema_minor = reader.varint(wire)? as u32,
                3 => index.entry = reader.string(wire)?,
                4 => index
                    .entries
                    .push(decode_archive_entry(reader.bytes(wire)?)?),
                5 => record_type = Some(reader.varint(wire)? as u32),
                6 => required_features = reader.varint(wire)?,
                _ => reader.skip(wire)?,
            }
        }
        if index.schema_major != INDEX_SCHEMA_MAJOR {
            return Err(ArtifactError::UnsupportedIndexVersion {
                major: index.schema_major,
                record_type: ARCHIVE_INDEX_RECORD_TYPE,
            });
        }
        if record_type != Some(ARCHIVE_INDEX_RECORD_TYPE) {
            return Err(ArtifactError::Index("归档索引记录类型不匹配".to_owned()));
        }
        if required_features != INDEX_REQUIRED_FEATURES {
            return Err(ArtifactError::Index("归档索引包含未知必需能力".to_owned()));
        }
        validate_archive_index(&index)?;
        index.entries.sort_by(|left, right| {
            left.logical_path
                .cmp(&right.logical_path)
                .then(left.digest.cmp(&right.digest))
        });
        Ok(index)
    }
}

impl GlobalIndex {
    /// 以确定性 protobuf wire 格式编码全局索引。
    pub fn encode(&self) -> Result<Vec<u8>, ArtifactError> {
        if self.schema_major != INDEX_SCHEMA_MAJOR {
            return Err(ArtifactError::UnsupportedIndexVersion {
                major: self.schema_major,
                record_type: GLOBAL_INDEX_RECORD_TYPE,
            });
        }
        let mut records = self.records.clone();
        records.sort_by(|left, right| {
            left.request_key
                .cmp(&right.request_key)
                .then(left.digest.cmp(&right.digest))
        });
        let mut output = Vec::new();
        put_varint_field(&mut output, 1, self.schema_major as u64);
        put_varint_field(&mut output, 2, self.schema_minor as u64);
        for record in records {
            validate_global_record(&record)?;
            put_bytes_field(&mut output, 3, &encode_global_record(&record)?);
        }
        put_varint_field(&mut output, 4, u64::from(GLOBAL_INDEX_RECORD_TYPE));
        put_varint_field(&mut output, 5, INDEX_REQUIRED_FEATURES);
        Ok(output)
    }

    /// 解码并检查主版本。
    pub fn decode(bytes: &[u8]) -> Result<Self, ArtifactError> {
        let mut reader = ProtoReader::new(bytes);
        let mut index = Self {
            schema_major: 0,
            schema_minor: 0,
            records: Vec::new(),
        };
        let mut record_type = None;
        let mut required_features = INDEX_REQUIRED_FEATURES;
        while let Some((field, wire)) = reader.next_key()? {
            match field {
                1 => index.schema_major = reader.varint(wire)? as u32,
                2 => index.schema_minor = reader.varint(wire)? as u32,
                3 => index
                    .records
                    .push(decode_global_record(reader.bytes(wire)?)?),
                4 => record_type = Some(reader.varint(wire)? as u32),
                5 => required_features = reader.varint(wire)?,
                _ => reader.skip(wire)?,
            }
        }
        if index.schema_major != INDEX_SCHEMA_MAJOR {
            return Err(ArtifactError::UnsupportedIndexVersion {
                major: index.schema_major,
                record_type: GLOBAL_INDEX_RECORD_TYPE,
            });
        }
        if record_type != Some(GLOBAL_INDEX_RECORD_TYPE) {
            return Err(ArtifactError::Index("全局索引记录类型不匹配".to_owned()));
        }
        if required_features != INDEX_REQUIRED_FEATURES {
            return Err(ArtifactError::Index("全局索引包含未知必需能力".to_owned()));
        }
        index.records.sort_by(|left, right| {
            left.request_key
                .cmp(&right.request_key)
                .then(left.digest.cmp(&right.digest))
        });
        Ok(index)
    }

    /// 按构建请求键查找记录。
    pub fn find(&self, request_key: &str) -> Option<&GlobalRecord> {
        self.records
            .iter()
            .find(|record| record.request_key == request_key)
    }

    /// 按对象类型和摘要反查所有构建请求。
    pub fn find_by_digest(
        &self,
        object_kind: ObjectKind,
        digest: Digest256,
    ) -> impl Iterator<Item = &GlobalRecord> {
        self.records
            .iter()
            .filter(move |record| record.object_kind == object_kind && record.digest == digest)
    }

    /// 按目标、优化级别和代码生成版本筛选构建记录。
    pub fn find_matching(
        &self,
        target: &str,
        optimization_level: u32,
        codegen_version: u32,
    ) -> impl Iterator<Item = &GlobalRecord> {
        self.records.iter().filter(move |record| {
            record.target == target
                && record.optimization_level == optimization_level
                && record.codegen_version == codegen_version
        })
    }

    /// 按构建请求键追加或替换记录，并保持规范排序。
    pub fn upsert(&mut self, record: GlobalRecord) {
        self.records
            .retain(|existing| existing.request_key != record.request_key);
        self.records.push(record);
        self.records.sort_by(|left, right| {
            left.request_key
                .cmp(&right.request_key)
                .then(left.digest.cmp(&right.digest))
        });
    }
}

fn encode_archive_entry(entry: &ArchiveEntry) -> Result<Vec<u8>, ArtifactError> {
    let mut output = Vec::new();
    put_bytes_field(&mut output, 1, entry.logical_path.as_bytes());
    put_varint_field(&mut output, 2, object_kind_number(entry.object_kind));
    put_bytes_field(&mut output, 3, entry.digest.as_hex().as_bytes());
    put_bytes_field(&mut output, 4, entry.module.as_bytes());
    put_bytes_field(&mut output, 5, entry.target.as_bytes());
    put_varint_field(&mut output, 6, entry.length);
    Ok(output)
}

fn validate_archive_index(index: &ArchiveIndex) -> Result<(), ArtifactError> {
    if index.entry.is_empty() {
        return Err(ArtifactError::Index("归档索引缺少入口".to_owned()));
    }
    for entry in &index.entries {
        validate_archive_entry(entry)?;
    }
    Ok(())
}

fn validate_archive_entry(entry: &ArchiveEntry) -> Result<(), ArtifactError> {
    if entry.logical_path.is_empty() || entry.module.is_empty() || entry.target.is_empty() {
        return Err(ArtifactError::Index("归档索引条目缺少必需字段".to_owned()));
    }
    Ok(())
}

fn decode_archive_entry(bytes: &[u8]) -> Result<ArchiveEntry, ArtifactError> {
    let mut reader = ProtoReader::new(bytes);
    let mut entry = ArchiveEntry {
        logical_path: String::new(),
        object_kind: ObjectKind::Xiaoc,
        digest: Digest256::parse(&"0".repeat(64))?,
        module: String::new(),
        target: String::new(),
        length: 0,
    };
    let mut has_kind = false;
    let mut has_digest = false;
    let mut has_length = false;
    while let Some((field, wire)) = reader.next_key()? {
        match field {
            1 => entry.logical_path = reader.string(wire)?,
            2 => {
                entry.object_kind = object_kind_from_number(reader.varint(wire)?)?;
                has_kind = true;
            }
            3 => {
                entry.digest = Digest256::parse(&reader.string(wire)?)?;
                has_digest = true;
            }
            4 => entry.module = reader.string(wire)?,
            5 => entry.target = reader.string(wire)?,
            6 => {
                entry.length = reader.varint(wire)?;
                has_length = true;
            }
            _ => reader.skip(wire)?,
        }
    }
    if !has_kind || !has_digest || !has_length {
        return Err(ArtifactError::Index("归档索引条目缺少必需字段".to_owned()));
    }
    Ok(entry)
}

fn encode_global_record(record: &GlobalRecord) -> Result<Vec<u8>, ArtifactError> {
    let mut output = Vec::new();
    put_bytes_field(&mut output, 1, record.request_key.as_bytes());
    put_varint_field(&mut output, 2, object_kind_number(record.object_kind));
    put_bytes_field(&mut output, 3, record.digest.as_hex().as_bytes());
    put_bytes_field(&mut output, 4, record.target.as_bytes());
    put_varint_field(&mut output, 5, record.optimization_level as u64);
    put_varint_field(&mut output, 6, record.codegen_version as u64);
    put_varint_field(&mut output, 7, record.length);
    Ok(output)
}

fn validate_global_record(record: &GlobalRecord) -> Result<(), ArtifactError> {
    if record.request_key.is_empty() || record.target.is_empty() || record.codegen_version == 0 {
        return Err(ArtifactError::Index("全局索引记录缺少必需字段".to_owned()));
    }
    Ok(())
}

fn decode_global_record(bytes: &[u8]) -> Result<GlobalRecord, ArtifactError> {
    let mut reader = ProtoReader::new(bytes);
    let mut record = GlobalRecord {
        request_key: String::new(),
        object_kind: ObjectKind::Xiaoc,
        digest: Digest256::parse(&"0".repeat(64))?,
        target: String::new(),
        optimization_level: 0,
        codegen_version: 0,
        length: 0,
    };
    let mut has_kind = false;
    let mut has_digest = false;
    let mut has_length = false;
    while let Some((field, wire)) = reader.next_key()? {
        match field {
            1 => record.request_key = reader.string(wire)?,
            2 => {
                record.object_kind = object_kind_from_number(reader.varint(wire)?)?;
                has_kind = true;
            }
            3 => {
                record.digest = Digest256::parse(&reader.string(wire)?)?;
                has_digest = true;
            }
            4 => record.target = reader.string(wire)?,
            5 => record.optimization_level = reader.varint(wire)? as u32,
            6 => record.codegen_version = reader.varint(wire)? as u32,
            7 => {
                record.length = reader.varint(wire)?;
                has_length = true;
            }
            _ => reader.skip(wire)?,
        }
    }
    if record.request_key.is_empty()
        || record.target.is_empty()
        || record.codegen_version == 0
        || !has_kind
        || !has_digest
        || !has_length
    {
        return Err(ArtifactError::Index("全局索引记录缺少必需字段".to_owned()));
    }
    Ok(record)
}

fn object_kind_number(kind: ObjectKind) -> u64 {
    kind as u64 + 1
}
fn object_kind_from_number(value: u64) -> Result<ObjectKind, ArtifactError> {
    match value {
        1 => Ok(ObjectKind::Source),
        2 => Ok(ObjectKind::Xiaoc),
        3 => Ok(ObjectKind::Native),
        4 => Ok(ObjectKind::Xar),
        5 => Ok(ObjectKind::Language),
        _ => Err(ArtifactError::Index(format!("未知对象类型：{value}"))),
    }
}
fn put_varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}
fn put_varint_field(output: &mut Vec<u8>, field: u32, value: u64) {
    put_varint(output, (field as u64) << 3);
    put_varint(output, value);
}
fn put_bytes_field(output: &mut Vec<u8>, field: u32, value: &[u8]) {
    put_varint(output, ((field as u64) << 3) | 2);
    put_varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

struct ProtoReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> ProtoReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn next_key(&mut self) -> Result<Option<(u32, u8)>, ArtifactError> {
        if self.offset == self.bytes.len() {
            return Ok(None);
        }
        let key = self.read_varint()?;
        if key >> 3 > 0x1fff_ffff {
            return Err(ArtifactError::Index("Protobuf 字段号超出范围".to_owned()));
        }
        let field = (key >> 3) as u32;
        let wire = (key & 7) as u8;
        if field == 0 {
            return Err(ArtifactError::Index("Protobuf 字段号不能为零".to_owned()));
        }
        Ok(Some((field, wire)))
    }
    fn read_varint(&mut self) -> Result<u64, ArtifactError> {
        let mut value = 0_u64;
        for index in 0..10 {
            let byte = *self
                .bytes
                .get(self.offset)
                .ok_or_else(|| ArtifactError::Index("Protobuf varint 截断".to_owned()))?;
            self.offset += 1;
            if index == 9 && byte > 1 {
                return Err(ArtifactError::Index("Protobuf varint 溢出".to_owned()));
            }
            value |= u64::from(byte & 0x7f) << (index * 7);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ArtifactError::Index("Protobuf varint 过长".to_owned()))
    }
    fn varint(&mut self, wire: u8) -> Result<u64, ArtifactError> {
        if wire == 0 {
            self.read_varint()
        } else {
            Err(ArtifactError::Index("Protobuf wire 类型错误".to_owned()))
        }
    }
    fn bytes(&mut self, wire: u8) -> Result<&'a [u8], ArtifactError> {
        if wire != 2 {
            return Err(ArtifactError::Index(
                "Protobuf 长度字段 wire 类型错误".to_owned(),
            ));
        }
        let length =
            usize::try_from(self.read_varint()?).map_err(|_| ArtifactError::SizeOverflow)?;
        let end = self
            .offset
            .checked_add(length)
            .ok_or(ArtifactError::SizeOverflow)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| ArtifactError::Index("Protobuf 长度字段截断".to_owned()))?;
        self.offset = end;
        Ok(bytes)
    }
    fn string(&mut self, wire: u8) -> Result<String, ArtifactError> {
        String::from_utf8(self.bytes(wire)?.to_vec())
            .map_err(|_| ArtifactError::Index("Protobuf 字符串不是 UTF-8".to_owned()))
    }
    fn skip(&mut self, wire: u8) -> Result<(), ArtifactError> {
        match wire {
            0 => {
                self.read_varint()?;
                Ok(())
            }
            1 => {
                self.offset = self
                    .offset
                    .checked_add(8)
                    .ok_or(ArtifactError::SizeOverflow)?;
                if self.offset <= self.bytes.len() {
                    Ok(())
                } else {
                    Err(ArtifactError::Index("Protobuf 64 位字段截断".to_owned()))
                }
            }
            2 => self.bytes(2).map(|_| ()),
            5 => {
                self.offset = self
                    .offset
                    .checked_add(4)
                    .ok_or(ArtifactError::SizeOverflow)?;
                if self.offset <= self.bytes.len() {
                    Ok(())
                } else {
                    Err(ArtifactError::Index("Protobuf 32 位字段截断".to_owned()))
                }
            }
            _ => Err(ArtifactError::Index(
                "不支持的 Protobuf wire 类型".to_owned(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use xiao_bytecode::{
        TAC_BYTECODE_ABI_VERSION, TAC_RUNTIME_ABI_VERSION, TAC_VERSION, TacAbi, TacProgram,
        XiaocMetadata, encode_xiaoc,
    };

    fn temp_root(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "xiao-artifacts-{label}-{}-{stamp}",
            std::process::id()
        ))
    }

    fn valid_xiaoc() -> Vec<u8> {
        let program = TacProgram {
            version: TAC_VERSION,
            abi: TacAbi {
                bytecode_abi_version: TAC_BYTECODE_ABI_VERSION,
                runtime_abi_version: TAC_RUNTIME_ABI_VERSION,
                ir_version: 1,
                language_version: "0.1.0".to_owned(),
                target: "portable".to_owned(),
            },
            constants: Default::default(),
            signatures: Default::default(),
            functions: Vec::new(),
            categories: Default::default(),
            plans: Vec::new(),
            selection_plans: Vec::new(),
            broadcast_assignment_plans: Vec::new(),
            random_seed_plans: Vec::new(),
            table_definitions: Vec::new(),
            unsupported: Vec::new(),
        };
        encode_xiaoc(&program, XiaocMetadata::default()).unwrap()
    }

    fn make_writable(path: &Path) {
        let mut permissions = fs::metadata(path).unwrap().permissions();
        #[cfg(unix)]
        permissions.set_mode(0o644);
        #[cfg(windows)]
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn digest_and_sharded_xiaoc_object_are_deterministic() {
        let root = temp_root("digest");
        let store = ArtifactStore::open(&root).unwrap();
        let bytes = valid_xiaoc();
        let object = store.put_xiaoc(&bytes).unwrap();
        assert_eq!(object.digest.as_hex().len(), 64);
        assert_eq!(
            object.path.file_name().unwrap().to_string_lossy(),
            format!("{}.xiaoc", object.digest)
        );
        assert_eq!(
            object.path.parent().unwrap().file_name().unwrap(),
            &object.digest.as_hex()[..2]
        );
        assert_eq!(store.read(ObjectKind::Xiaoc, object.digest).unwrap(), bytes);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn corrupt_object_is_quarantined() {
        let root = temp_root("corrupt");
        let store = ArtifactStore::open(&root).unwrap();
        let object = store.put(ObjectKind::Native, b"native-one").unwrap();
        make_writable(&object.path);
        fs::write(&object.path, b"tampered").unwrap();
        let error = store.read(ObjectKind::Native, object.digest).unwrap_err();
        assert!(matches!(error, ArtifactError::CorruptObject { .. }));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn objects_are_read_only_and_hash_collisions_quarantine_both_inputs() {
        let root = temp_root("collision");
        let store = ArtifactStore::open(&root).unwrap();
        let incoming = b"incoming-content";
        let digest = Digest256::of_bytes(incoming);
        let existing = store.object_path(ObjectKind::Native, digest);
        fs::create_dir_all(existing.parent().unwrap()).unwrap();
        fs::write(&existing, b"different-content").unwrap();
        let error = store.put(ObjectKind::Native, incoming).unwrap_err();
        assert!(matches!(error, ArtifactError::HashCollision { .. }));
        assert!(!existing.exists());
        let quarantined = fs::read_dir(root.join("quarantine"))
            .unwrap()
            .filter_map(Result::ok)
            .count();
        assert_eq!(quarantined, 2);

        let object = store.put(ObjectKind::Native, b"immutable").unwrap();
        assert!(fs::metadata(object.path).unwrap().permissions().readonly());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scan_quarantines_objects_with_invalid_names_or_shards() {
        let root = temp_root("scan-boundary");
        let store = ArtifactStore::open(&root).unwrap();
        let shard = root.join("objects/native/sha256/00");
        fs::create_dir_all(&shard).unwrap();
        fs::write(shard.join("not-a-digest.bin"), b"bad-name").unwrap();
        fs::write(
            shard.join(format!("{}.bin", Digest256::of_bytes(b"wrong-shard"))),
            b"wrong-shard",
        )
        .unwrap();
        assert!(store.scan(ObjectKind::Native).unwrap().is_empty());
        assert_eq!(fs::read_dir(root.join("quarantine")).unwrap().count(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn indexes_round_trip_deterministically_and_reject_unknown_major() {
        let digest = Digest256::of_bytes(b"xiaoc");
        let index = GlobalIndex {
            schema_major: INDEX_SCHEMA_MAJOR,
            schema_minor: INDEX_SCHEMA_MINOR,
            records: vec![GlobalRecord {
                request_key: "request".to_owned(),
                object_kind: ObjectKind::Xiaoc,
                digest,
                target: "x86_64-unknown-linux-gnu".to_owned(),
                optimization_level: 2,
                codegen_version: 2,
                length: 5,
            }],
        };
        let first = index.encode().unwrap();
        assert_eq!(first, index.encode().unwrap());
        assert_eq!(GlobalIndex::decode(&first).unwrap(), index);
        let bad = GlobalIndex {
            schema_major: 2,
            ..index.clone()
        };
        assert!(matches!(
            bad.encode(),
            Err(ArtifactError::UnsupportedIndexVersion { .. })
        ));

        let mut wrong_type = first.clone();
        wrong_type.extend_from_slice(&[0x20, 0x63]);
        assert!(matches!(
            GlobalIndex::decode(&wrong_type),
            Err(ArtifactError::Index(_))
        ));

        let archive = ArchiveIndex {
            schema_major: INDEX_SCHEMA_MAJOR,
            schema_minor: INDEX_SCHEMA_MINOR,
            entry: "main.xiaoc".to_owned(),
            entries: vec![ArchiveEntry {
                logical_path: "objects/xiaoc/main.xiaoc".to_owned(),
                object_kind: ObjectKind::Xiaoc,
                digest,
                module: "main".to_owned(),
                target: "portable".to_owned(),
                length: 5,
            }],
        };
        assert_eq!(
            ArchiveIndex::decode(&archive.encode().unwrap()).unwrap(),
            archive
        );

        let replacement = GlobalRecord {
            request_key: "request".to_owned(),
            object_kind: ObjectKind::Native,
            digest,
            target: "x86_64-unknown-linux-gnu".to_owned(),
            optimization_level: 3,
            codegen_version: 2,
            length: 5,
        };
        let mut updated = index;
        updated.upsert(replacement.clone());
        assert_eq!(updated.find("request"), Some(&replacement));
    }

    #[test]
    fn archive_index_entry_validation_is_symmetric() {
        let missing_entry = [0x08, 0x01, 0x10, 0x00, 0x28, 0x01, 0x30, 0x00];
        let empty_entry = [0x08, 0x01, 0x10, 0x00, 0x1a, 0x00, 0x28, 0x01, 0x30, 0x00];
        let non_empty_entry = [
            0x08, 0x01, 0x10, 0x00, 0x1a, 0x03, b'm', b'a', b'i', 0x28, 0x01, 0x30, 0x00,
        ];
        assert!(ArchiveIndex::decode(&missing_entry).is_err());
        assert!(ArchiveIndex::decode(&empty_entry).is_err());
        assert_eq!(ArchiveIndex::decode(&non_empty_entry).unwrap().entry, "mai");
    }

    #[test]
    fn xiaoc_validation_comes_from_authoritative_codec() {
        let root = temp_root("authoritative-xiaoc-validation");
        let store = ArtifactStore::open(&root).unwrap();
        let error = store.put_xiaoc(&[]).unwrap_err();
        match error {
            ArtifactError::InvalidObject { reason, .. } => {
                assert!(reason.contains("XIAOC-001"));
                assert!(!reason.contains("不是完整规范 `.xiaoc` 文件"));
            }
            other => panic!("`.xiaoc` 校验错误类型错误：{other:?}"),
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn index_store_commits_global_snapshot_atomically() {
        let root = temp_root("index");
        let store = IndexStore::open(&root).unwrap();
        let index = GlobalIndex {
            schema_major: INDEX_SCHEMA_MAJOR,
            schema_minor: INDEX_SCHEMA_MINOR,
            records: Vec::new(),
        };
        let path = store.write_global(&index).unwrap();
        assert!(path.is_file());
        assert_eq!(store.read_global().unwrap(), index);
        assert!(!root.join("global.index.tmp").exists());
        assert!(!root.join("global.index.lock").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn indexes_remain_readable_while_their_write_locks_are_held() {
        let root = temp_root("readonly-index");
        let store = IndexStore::open(&root).unwrap();
        let global = GlobalIndex {
            schema_major: INDEX_SCHEMA_MAJOR,
            schema_minor: INDEX_SCHEMA_MINOR,
            records: Vec::new(),
        };
        store.write_global(&global).unwrap();
        let archive_path = root.join("sample.index.pb");
        let archive = ArchiveIndex {
            schema_major: INDEX_SCHEMA_MAJOR,
            schema_minor: INDEX_SCHEMA_MINOR,
            entry: "main.xiaoc".to_owned(),
            entries: Vec::new(),
        };
        store.write_archive(&archive_path, &archive).unwrap();

        let _global_lock = xiao_lock::EntryLock::acquire(&root.join("global.index.lock")).unwrap();
        let _archive_lock =
            xiao_lock::EntryLock::acquire(&archive_path.with_extension("index.lock")).unwrap();
        assert_eq!(store.read_global().unwrap(), global);
        assert_eq!(store.read_archive(&archive_path).unwrap(), archive);
        drop((_global_lock, _archive_lock));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn global_index_can_rebuild_from_verified_objects_and_query_both_directions() {
        let root = temp_root("rebuild");
        let artifacts = ArtifactStore::open(root.join("objects")).unwrap();
        let object = artifacts.put(ObjectKind::Native, b"native").unwrap();
        let indexes = IndexStore::open(root.join("indexes")).unwrap();
        indexes
            .rebuild_global_atomic(&artifacts, |reference| {
                Some(GlobalRecord {
                    request_key: "request".to_owned(),
                    object_kind: reference.kind,
                    digest: reference.digest,
                    target: "x86_64-unknown-linux-gnu".to_owned(),
                    optimization_level: 2,
                    codegen_version: 2,
                    length: reference.length,
                })
            })
            .unwrap();
        let index = indexes.read_global().unwrap();
        assert_eq!(index.find("request").unwrap().digest, object.digest);
        assert_eq!(
            index
                .find_by_digest(ObjectKind::Native, object.digest)
                .count(),
            1
        );
        assert_eq!(
            index
                .find_matching("x86_64-unknown-linux-gnu", 2, 2)
                .count(),
            1
        );
        assert!(!root.join("indexes/global.index.lock").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cleanup_plan_protects_project_lock_references_and_apply_is_explicit() {
        let root = temp_root("maintenance");
        let store = ArtifactStore::open(&root).unwrap();
        let protected = store.put(ObjectKind::Source, b"protected-source").unwrap();
        let removable = store.put(ObjectKind::Native, b"removable-native").unwrap();
        let lock_path = root.join("xiao.lock.json");
        fs::write(
            &lock_path,
            format!(
                r#"{{"lock_version":2,"config_fingerprint":"config","root":{{"name":"main","version":"0.1.0","source":{{"source_id":"path:/main","alias":null,"display_name":"main"}}}},"packages":{{"main@0.1.0[path:/main]":{{"name":"main","version":"0.1.0","source":{{"source_id":"path:/main","alias":null,"display_name":"main"}},"content_digest":"{}","dependencies":{{}},"precompiled_variants":[],"target_conditions":[]}}}}}}"#,
                protected.digest
            ),
        )
        .unwrap();
        let references = store
            .collect_references(Some(&lock_path), &[], &[])
            .unwrap();
        let plan = store.plan_cleanup(&references).unwrap();
        assert_eq!(plan.candidates(), std::slice::from_ref(&removable));
        assert!(!root.join("maintenance.lock").exists());
        assert!(store.read(ObjectKind::Source, protected.digest).is_ok());

        let report = store.apply_cleanup(&plan, &references).unwrap();
        assert_eq!(report.removed, vec![removable]);
        assert!(matches!(
            store.read(ObjectKind::Native, Digest256::of_bytes(b"removable-native")),
            Err(ArtifactError::Io(error)) if error.kind() == io::ErrorKind::NotFound
        ));
        assert!(!root.join("maintenance.lock").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cleanup_reference_source_failure_is_conservative() {
        let root = temp_root("maintenance-failure");
        let store = ArtifactStore::open(&root).unwrap();
        let object = store.put(ObjectKind::Native, b"keep-on-error").unwrap();
        let lock_path = root.join("broken.lock.json");
        fs::write(&lock_path, b"not-json").unwrap();
        assert!(
            store
                .collect_references(Some(&lock_path), &[], &[])
                .is_err()
        );
        fs::write(
            &lock_path,
            br#"{"lock_version":2,"config_fingerprint":"x","root":{},"packages":{}}"#,
        )
        .unwrap();
        assert!(
            store
                .collect_references(Some(&lock_path), &[], &[])
                .is_err()
        );
        assert!(store.read(ObjectKind::Native, object.digest).is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cleanup_revalidates_and_quarantines_object_changed_after_plan() {
        let root = temp_root("maintenance-corrupt");
        let store = ArtifactStore::open(&root).unwrap();
        let object = store.put(ObjectKind::Native, b"before-plan").unwrap();
        let plan = store.plan_cleanup(&ReferenceSet::new()).unwrap();
        make_writable(&object.path);
        fs::write(&object.path, b"after-plan-tamper").unwrap();
        let report = store.apply_cleanup(&plan, &ReferenceSet::new()).unwrap();
        assert_eq!(report.removed, Vec::<ObjectReference>::new());
        assert_eq!(report.quarantined.len(), 1);
        assert!(!object.path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cleanup_plan_defers_invalid_object_isolation_until_apply() {
        let root = temp_root("maintenance-invalid");
        let store = ArtifactStore::open(&root).unwrap();
        let shard = root.join("objects/native/sha256/00");
        fs::create_dir_all(&shard).unwrap();
        let invalid = shard.join("bad-name.bin");
        fs::write(&invalid, b"invalid-name").unwrap();
        let plan = store.plan_cleanup(&ReferenceSet::new()).unwrap();
        assert!(plan.candidates().is_empty());
        assert!(invalid.exists());
        let report = store.apply_cleanup(&plan, &ReferenceSet::new()).unwrap();
        assert_eq!(report.removed.len(), 0);
        assert_eq!(report.quarantined.len(), 1);
        assert!(!invalid.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cleanup_rechecks_references_added_after_plan_creation() {
        let root = temp_root("maintenance-new-reference");
        let store = ArtifactStore::open(&root).unwrap();
        let object = store.put(ObjectKind::Native, b"newly-referenced").unwrap();
        let plan = store.plan_cleanup(&ReferenceSet::new()).unwrap();
        assert_eq!(plan.candidates(), std::slice::from_ref(&object));

        let mut current_references = ReferenceSet::new();
        current_references.insert(CacheReference::exact(object.kind, object.digest));
        let report = store.apply_cleanup(&plan, &current_references).unwrap();
        assert!(report.removed.is_empty());
        assert!(store.read(object.kind, object.digest).is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cleanup_archive_reference_protects_its_object() {
        let root = temp_root("maintenance-archive-reference");
        let store = ArtifactStore::open(root.join("cache")).unwrap();
        let object = store.put(ObjectKind::Xiaoc, &valid_xiaoc()).unwrap();
        let index_store = IndexStore::open(root.join("indexes")).unwrap();
        let archive_path = root.join("archive-index.pb");
        let archive = ArchiveIndex {
            schema_major: INDEX_SCHEMA_MAJOR,
            schema_minor: INDEX_SCHEMA_MINOR,
            entry: "main.xiaoc".to_owned(),
            entries: vec![ArchiveEntry {
                logical_path: "objects/xiaoc/main.xiaoc".to_owned(),
                object_kind: ObjectKind::Xiaoc,
                digest: object.digest,
                module: "main".to_owned(),
                target: "portable".to_owned(),
                length: object.length,
            }],
        };
        index_store.write_archive(&archive_path, &archive).unwrap();
        let references = store
            .collect_references(None, std::slice::from_ref(&archive_path), &[])
            .unwrap();
        let plan = store.plan_cleanup(&references).unwrap();
        assert!(plan.candidates().is_empty());
        let report = store.apply_cleanup(&plan, &references).unwrap();
        assert!(report.removed.is_empty());
        assert!(store.read(ObjectKind::Xiaoc, object.digest).is_ok());
        let _ = fs::remove_dir_all(root);
    }
}
