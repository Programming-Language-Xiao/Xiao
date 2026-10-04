//! Xiao `.xar` 的确定性 ZIP/ZIP64 容器与安全编解码。
//!
//! 本 crate 只处理格式层：它不会解析入口、执行模块或启动 Runtime。归档在成功打开前会
//! 完成中央目录、索引、路径、长度、CRC、摘要和 `.xiaoc` 载荷校验，调用方拿到的对象字节
//! 已经通过“先验证、再暴露”边界。

#![allow(clippy::result_large_err)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::io::{self, Cursor, Read, Write};

use crc32fast::Hasher as Crc32;
use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use xiao_artifacts::{
    ArchiveIndex, ArtifactError, Digest256, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR, ObjectKind,
};
use xiao_bytecode::validate_xiaoc;

/// `.xar` 中唯一的索引成员路径。
pub const XAR_INDEX_PATH: &str = "META-INF/xiao/index.pb";
/// 初始 `.xar` 格式主版本。
pub const XAR_FORMAT_MAJOR: u16 = 1;
/// 初始 `.xar` 格式次版本。
pub const XAR_FORMAT_MINOR: u16 = 0;
/// 归档格式与压缩后端组成的稳定工具链指纹文本。
pub const XAR_TOOLCHAIN_FINGERPRINT: &str = "xar-zip64-v1;flate2=1.1.10;backend=rust";
/// 受限归档允许的最大成员数量。
pub const MAX_MEMBER_COUNT: u64 = 100_000;
/// 索引成员允许的最大解压长度。
pub const MAX_INDEX_SIZE: u64 = 16 * 1024 * 1024;
/// 普通对象允许的最大解压长度。
pub const MAX_MEMBER_SIZE: u64 = 512 * 1024 * 1024;
/// 压缩比超过该值的成员按 ZIP bomb 拒绝。
pub const MAX_COMPRESSION_RATIO: u64 = 1_000;
const ZIP_LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const ZIP_CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const ZIP_EOCD_SIGNATURE: u32 = 0x0605_4b50;
const ZIP64_EOCD_SIGNATURE: u32 = 0x0606_4b50;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const ZIP64_EXTRA_ID: u16 = 0x0001;
const UTF8_FLAG: u16 = 1 << 11;
const STORE_METHOD: u16 = 0;
const DEFLATE_METHOD: u16 = 8;
const DOS_DATE_1980_01_01: u16 = 0x0021;
const ZIP32_MAX: u64 = u32::MAX as u64;

/// ZIP 成员采用的压缩方式。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionMethod {
    /// 原样存储。
    Store,
    /// RFC 1951 DEFLATE。
    Deflate,
}

impl CompressionMethod {
    fn code(self) -> u16 {
        match self {
            Self::Store => STORE_METHOD,
            Self::Deflate => DEFLATE_METHOD,
        }
    }
}

/// 归档写入选项。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XarBuildOptions {
    /// 强制使用 ZIP64 头部，用于跨实现的 ZIP64 回归测试。
    pub force_zip64: bool,
    /// DEFLATE 等级；17A 冻结为 6。
    pub compression_level: u32,
}

impl Default for XarBuildOptions {
    fn default() -> Self {
        Self {
            force_zip64: false,
            compression_level: 6,
        }
    }
}

/// 待写入归档的内容寻址对象。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XarObject {
    /// 对象命名空间。
    pub kind: ObjectKind,
    /// 期望摘要。
    pub digest: Digest256,
    /// 规范对象字节。
    pub bytes: Vec<u8>,
}

impl XarObject {
    /// 从对象字节构造并计算摘要。
    #[must_use]
    pub fn from_bytes(kind: ObjectKind, bytes: impl Into<Vec<u8>>) -> Self {
        let bytes = bytes.into();
        Self {
            kind,
            digest: Digest256::of_bytes(&bytes),
            bytes,
        }
    }
}

/// 已通过中央目录结构检查的成员摘要。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XarMemberInfo {
    /// 物理成员路径。
    pub path: String,
    /// 成员压缩方式。
    pub compression: CompressionMethod,
    /// 压缩后长度。
    pub compressed_size: u64,
    /// 解压后长度。
    pub uncompressed_size: u64,
    /// CRC-32 校验值。
    pub crc32: u32,
    /// 本地文件头在归档中的偏移。
    pub local_header_offset: u64,
}

/// 已验证的 `.xar` 归档。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XarArchive {
    bytes: Vec<u8>,
    index: ArchiveIndex,
    members: Vec<XarMemberInfo>,
}

impl XarArchive {
    /// 返回唯一归档索引。
    #[must_use]
    pub fn index(&self) -> &ArchiveIndex {
        &self.index
    }

    /// 返回按物理路径排序的成员摘要。
    #[must_use]
    pub fn members(&self) -> &[XarMemberInfo] {
        &self.members
    }

    /// 读取一个已通过结构检查的成员，并在暴露前校验 CRC。
    pub fn read_member(&self, path: &str) -> Result<Vec<u8>, XarError> {
        validate_physical_path(path)?;
        let member = self
            .members
            .iter()
            .find(|member| member.path == path)
            .ok_or_else(|| XarError::MissingMember(path.to_owned()))?;
        decode_member(&self.bytes, member)
    }

    /// 按对象命名空间和摘要读取内容，并校验摘要和长度。
    pub fn read_object(&self, kind: ObjectKind, digest: Digest256) -> Result<Vec<u8>, XarError> {
        let path = object_member_path(kind, digest);
        let bytes = self.read_member(&path)?;
        if Digest256::of_bytes(&bytes) != digest {
            return Err(XarError::DigestMismatch {
                path,
                expected: digest,
                actual: Digest256::of_bytes(&bytes),
            });
        }
        Ok(bytes)
    }

    /// 从已验证归档中读取入口对应的对象字节。
    pub fn read_entry(&self) -> Result<Vec<u8>, XarError> {
        let entry = self
            .index
            .entries
            .iter()
            .find(|entry| entry.logical_path == self.index.entry)
            .ok_or_else(|| XarError::InvalidIndex("入口没有对应索引条目".to_owned()))?;
        self.read_object(entry.object_kind, entry.digest)
    }
}

/// 构造并编码一个 `.xar`。
pub struct XarBuilder {
    index: ArchiveIndex,
    objects: Vec<XarObject>,
    options: XarBuildOptions,
}

impl XarBuilder {
    /// 创建默认选项的归档构造器。
    #[must_use]
    pub fn new(index: ArchiveIndex) -> Self {
        Self {
            index,
            objects: Vec::new(),
            options: XarBuildOptions::default(),
        }
    }

    /// 替换写入选项。
    #[must_use]
    pub const fn with_options(mut self, options: XarBuildOptions) -> Self {
        self.options = options;
        self
    }

    /// 添加一个内容寻址对象。
    pub fn add_object(&mut self, object: XarObject) -> Result<(), XarError> {
        if self
            .objects
            .iter()
            .any(|existing| existing.kind == object.kind && existing.digest == object.digest)
        {
            return Err(XarError::DuplicateObject(object.kind, object.digest));
        }
        self.objects.push(object);
        Ok(())
    }

    /// 编码归档并执行所有格式和载荷校验。
    pub fn build(self) -> Result<Vec<u8>, XarError> {
        encode_with_options(&self.index, &self.objects, self.options)
    }
}

/// 对一个索引和对象集合执行默认确定性编码。
pub fn encode_xar(index: &ArchiveIndex, objects: &[XarObject]) -> Result<Vec<u8>, XarError> {
    encode_with_options(index, objects, XarBuildOptions::default())
}

/// 对一个索引和对象集合执行可选 ZIP64 编码。
pub fn encode_xar_with_options(
    index: &ArchiveIndex,
    objects: &[XarObject],
    options: XarBuildOptions,
) -> Result<Vec<u8>, XarError> {
    encode_with_options(index, objects, options)
}

/// 打开并完整验证一个 `.xar`。
pub fn decode_xar(bytes: &[u8]) -> Result<XarArchive, XarError> {
    let parsed = parse_zip(bytes)?;
    let index_member = parsed
        .members
        .iter()
        .find(|member| member.path == XAR_INDEX_PATH)
        .ok_or_else(|| XarError::MissingMember(XAR_INDEX_PATH.to_owned()))?;
    if parsed
        .members
        .iter()
        .filter(|member| member.path == XAR_INDEX_PATH)
        .count()
        != 1
    {
        return Err(XarError::DuplicateIndex);
    }
    if index_member.compression != CompressionMethod::Store {
        return Err(XarError::InvalidMember("索引必须使用 STORE".to_owned()));
    }
    if index_member.uncompressed_size > MAX_INDEX_SIZE {
        return Err(XarError::SizeLimit("索引过大".to_owned()));
    }
    let index_bytes = decode_member(bytes, index_member)?;
    let index = ArchiveIndex::decode(&index_bytes).map_err(XarError::Artifact)?;
    validate_archive_index(&index)?;
    let archive = XarArchive {
        bytes: bytes.to_vec(),
        index,
        members: parsed.members,
    };
    validate_members_against_index(&archive)?;
    Ok(archive)
}

/// 只验证归档，不保留可读对象。
pub fn validate_xar(bytes: &[u8]) -> Result<(), XarError> {
    decode_xar(bytes).map(|_| ())
}

/// 列出已验证归档的物理成员。
pub fn list_xar(bytes: &[u8]) -> Result<Vec<XarMemberInfo>, XarError> {
    Ok(decode_xar(bytes)?.members)
}

/// 返回内容寻址对象在归档中的规范物理路径。
#[must_use]
pub fn object_member_path(kind: ObjectKind, digest: Digest256) -> String {
    let hex = digest.as_hex();
    let suffix = if kind == ObjectKind::Xiaoc {
        ".xiaoc"
    } else {
        ""
    };
    format!(
        "objects/{}/sha256/{}/{}{}",
        kind.as_str(),
        &hex[..2],
        hex,
        suffix
    )
}

/// 返回归档格式使用的稳定工具链指纹。
#[must_use]
pub const fn toolchain_fingerprint() -> &'static str {
    XAR_TOOLCHAIN_FINGERPRINT
}

/// 归档格式错误。
#[derive(Debug)]
pub enum XarError {
    /// 输入长度或 ZIP 结构损坏。
    Malformed(String),
    /// 归档使用了受限格式之外的功能。
    Unsupported(String),
    /// 索引结构错误。
    InvalidIndex(String),
    /// 物理成员错误。
    InvalidMember(String),
    /// 入口成员缺失。
    MissingMember(String),
    /// 索引成员重复。
    DuplicateIndex,
    /// 对象重复。
    DuplicateObject(ObjectKind, Digest256),
    /// 长度或压缩比超过安全边界。
    SizeLimit(String),
    /// 对象摘要不匹配。
    DigestMismatch {
        /// 成员路径。
        path: String,
        /// 期望摘要。
        expected: Digest256,
        /// 实际摘要。
        actual: Digest256,
    },
    /// 底层对象索引错误。
    Artifact(ArtifactError),
    /// 底层输入输出错误。
    Io(io::Error),
    /// DEFLATE 解码错误。
    Compression(String),
    /// 内容完整性错误。
    Integrity(String),
}

impl Display for XarError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(message) => write!(formatter, "归档结构错误：{message}"),
            Self::Unsupported(message) => write!(formatter, "归档功能不支持：{message}"),
            Self::InvalidIndex(message) => write!(formatter, "归档索引错误：{message}"),
            Self::InvalidMember(message) => write!(formatter, "归档成员错误：{message}"),
            Self::MissingMember(path) => write!(formatter, "归档成员缺失：{path}"),
            Self::DuplicateIndex => formatter.write_str("归档索引成员重复"),
            Self::DuplicateObject(kind, digest) => {
                write!(formatter, "归档对象重复：{} {digest}", kind.as_str())
            }
            Self::SizeLimit(message) => write!(formatter, "归档安全边界：{message}"),
            Self::DigestMismatch {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "归档摘要不匹配：{path} 期望 {expected} 实际 {actual}"
            ),
            Self::Artifact(error) => Display::fmt(error, formatter),
            Self::Io(error) => write!(formatter, "归档 I/O 错误：{error}"),
            Self::Compression(message) => write!(formatter, "归档压缩错误：{message}"),
            Self::Integrity(message) => write!(formatter, "归档完整性错误：{message}"),
        }
    }
}

impl std::error::Error for XarError {}

impl From<io::Error> for XarError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ArtifactError> for XarError {
    fn from(error: ArtifactError) -> Self {
        Self::Artifact(error)
    }
}

#[derive(Clone)]
struct EncodedMember {
    path: String,
    method: CompressionMethod,
    crc32: u32,
    uncompressed_size: u64,
    compressed: Vec<u8>,
}

#[derive(Clone)]
struct ParsedZip {
    members: Vec<XarMemberInfo>,
}

fn encode_with_options(
    index: &ArchiveIndex,
    objects: &[XarObject],
    options: XarBuildOptions,
) -> Result<Vec<u8>, XarError> {
    if options.compression_level > 9 {
        return Err(XarError::Unsupported(
            "DEFLATE 等级必须位于 0..=9".to_owned(),
        ));
    }
    validate_archive_index(index)?;
    let index_bytes = index.encode().map_err(XarError::Artifact)?;
    if index_bytes.len() as u64 > MAX_INDEX_SIZE {
        return Err(XarError::SizeLimit("索引过大".to_owned()));
    }
    let mut object_map = BTreeMap::new();
    for object in objects {
        if Digest256::of_bytes(&object.bytes) != object.digest {
            return Err(XarError::DigestMismatch {
                path: object_member_path(object.kind, object.digest),
                expected: object.digest,
                actual: Digest256::of_bytes(&object.bytes),
            });
        }
        if object.bytes.len() as u64 > MAX_MEMBER_SIZE {
            return Err(XarError::SizeLimit("对象过大".to_owned()));
        }
        if object.kind == ObjectKind::Xiaoc {
            validate_xiaoc(&object.bytes)
                .map_err(|error| XarError::InvalidMember(format!("`.xiaoc` 校验失败：{error}")))?;
        }
        let key = (object.kind, object.digest);
        if object_map.insert(key, object).is_some() {
            return Err(XarError::DuplicateObject(key.0, key.1));
        }
    }
    let mut referenced = BTreeSet::new();
    for entry in &index.entries {
        validate_logical_path(&entry.logical_path)?;
        if entry.logical_path == index.entry && entry.length == 0 {
            return Err(XarError::InvalidIndex("入口对象长度不能为零".to_owned()));
        }
        let key = (entry.object_kind, entry.digest);
        let object = object_map.get(&key).ok_or_else(|| {
            XarError::InvalidIndex(format!("索引对象缺失：{}", entry.logical_path))
        })?;
        if object.bytes.len() as u64 != entry.length {
            return Err(XarError::InvalidIndex(format!(
                "对象长度不匹配：{}",
                entry.logical_path
            )));
        }
        referenced.insert(key);
    }
    if !index
        .entries
        .iter()
        .any(|entry| entry.logical_path == index.entry)
    {
        return Err(XarError::InvalidIndex("入口没有对应索引条目".to_owned()));
    }
    if referenced.len() != object_map.len() {
        return Err(XarError::InvalidIndex("归档包含未被索引的对象".to_owned()));
    }

    let mut members = Vec::with_capacity(objects.len() + 1);
    members.push(encode_member(
        XAR_INDEX_PATH.to_owned(),
        &index_bytes,
        CompressionMethod::Store,
        options.compression_level,
    )?);
    for object in object_map.values() {
        let path = object_member_path(object.kind, object.digest);
        let method = choose_method(&object.bytes, options.compression_level)?;
        members.push(encode_member(
            path,
            &object.bytes,
            method,
            options.compression_level,
        )?);
    }
    members.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    write_zip(&members, options.force_zip64)
}

fn validate_archive_index(index: &ArchiveIndex) -> Result<(), XarError> {
    if index.schema_major != INDEX_SCHEMA_MAJOR || index.schema_minor > INDEX_SCHEMA_MINOR {
        return Err(XarError::InvalidIndex("归档索引版本不受支持".to_owned()));
    }
    validate_logical_path(&index.entry)?;
    if index.dependency_lock_digest.is_empty() {
        return Err(XarError::InvalidIndex("缺少依赖锁摘要".to_owned()));
    }
    if index.runtime_abi_min == 0 || index.runtime_abi_max == 0 {
        return Err(XarError::InvalidIndex("缺少 Runtime ABI 范围".to_owned()));
    }
    if index.runtime_abi_max != 0 && index.runtime_abi_min > index.runtime_abi_max {
        return Err(XarError::InvalidIndex(
            "Runtime ABI 最小版本大于最大版本".to_owned(),
        ));
    }
    if !index.dependency_lock_digest.is_empty() {
        Digest256::parse(&index.dependency_lock_digest).map_err(XarError::Artifact)?;
    }
    if index.language_locale.is_empty() {
        return Err(XarError::InvalidIndex("缺少系统文案语言默认值".to_owned()));
    }
    let mut logical_paths = BTreeSet::new();
    let mut objects = BTreeSet::new();
    for entry in &index.entries {
        validate_logical_path(&entry.logical_path)?;
        if !logical_paths.insert(entry.logical_path.as_bytes().to_vec()) {
            return Err(XarError::InvalidIndex("索引逻辑路径重复".to_owned()));
        }
        if !objects.insert((entry.object_kind, entry.digest)) {
            return Err(XarError::InvalidIndex("索引对象摘要重复".to_owned()));
        }
        if entry.length > MAX_MEMBER_SIZE {
            return Err(XarError::SizeLimit("索引对象长度超限".to_owned()));
        }
    }
    Ok(())
}

fn validate_members_against_index(archive: &XarArchive) -> Result<(), XarError> {
    let mut referenced = BTreeSet::new();
    for entry in &archive.index.entries {
        let path = object_member_path(entry.object_kind, entry.digest);
        let member = archive
            .members
            .iter()
            .find(|member| member.path == path)
            .ok_or_else(|| XarError::MissingMember(path.clone()))?;
        if member.uncompressed_size != entry.length {
            return Err(XarError::InvalidIndex(format!("成员长度不匹配：{path}")));
        }
        let bytes = decode_member(&archive.bytes, member)?;
        if Digest256::of_bytes(&bytes) != entry.digest {
            return Err(XarError::DigestMismatch {
                path,
                expected: entry.digest,
                actual: Digest256::of_bytes(&bytes),
            });
        }
        if entry.object_kind == ObjectKind::Xiaoc {
            validate_xiaoc(&bytes)
                .map_err(|error| XarError::InvalidMember(format!("`.xiaoc` 校验失败：{error}")))?;
        }
        referenced.insert(member.path.clone());
    }
    for member in &archive.members {
        if member.path == XAR_INDEX_PATH {
            continue;
        }
        if !referenced.contains(&member.path) {
            return Err(XarError::InvalidIndex(format!(
                "未索引成员：{}",
                member.path
            )));
        }
    }
    Ok(())
}

fn validate_logical_path(path: &str) -> Result<(), XarError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .next()
            .is_some_and(|segment| segment.contains(':'))
    {
        return Err(XarError::InvalidIndex(format!("逻辑路径不安全：{path}")));
    }
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(XarError::InvalidIndex(format!(
            "逻辑路径包含非法段：{path}"
        )));
    }
    Ok(())
}

fn validate_physical_path(path: &str) -> Result<(), XarError> {
    if !path.is_ascii()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .next()
            .is_some_and(|segment| segment.contains(':'))
    {
        return Err(XarError::InvalidMember(format!("物理路径不安全：{path}")));
    }
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(XarError::InvalidMember(format!(
            "物理路径包含非法段：{path}"
        )));
    }
    if path != XAR_INDEX_PATH && !path.starts_with("objects/") {
        return Err(XarError::InvalidMember(format!("未知物理路径：{path}")));
    }
    Ok(())
}

fn choose_method(bytes: &[u8], level: u32) -> Result<CompressionMethod, XarError> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(level));
    encoder.write_all(bytes)?;
    let compressed = encoder
        .finish()
        .map_err(|error| XarError::Compression(error.to_string()))?;
    Ok(if compressed.len() < bytes.len() {
        CompressionMethod::Deflate
    } else {
        CompressionMethod::Store
    })
}

fn encode_member(
    path: String,
    bytes: &[u8],
    method: CompressionMethod,
    level: u32,
) -> Result<EncodedMember, XarError> {
    validate_physical_path(&path)?;
    let compressed = if method == CompressionMethod::Store {
        bytes.to_vec()
    } else {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(level));
        encoder.write_all(bytes)?;
        encoder
            .finish()
            .map_err(|error| XarError::Compression(error.to_string()))?
    };
    let mut crc = Crc32::new();
    crc.update(bytes);
    Ok(EncodedMember {
        path,
        method,
        crc32: crc.finalize(),
        uncompressed_size: bytes.len() as u64,
        compressed,
    })
}

fn write_zip(members: &[EncodedMember], force_zip64: bool) -> Result<Vec<u8>, XarError> {
    if members.len() as u64 > MAX_MEMBER_COUNT {
        return Err(XarError::SizeLimit("成员数量超限".to_owned()));
    }
    let mut output = Vec::new();
    let mut central = Vec::new();
    let mut zip64 = force_zip64;
    let mut offsets = Vec::with_capacity(members.len());
    for member in members {
        let offset = output.len() as u64;
        let name = member.path.as_bytes();
        let local_zip64 = force_zip64
            || member.uncompressed_size > ZIP32_MAX
            || member.compressed.len() as u64 > ZIP32_MAX;
        zip64 |= local_zip64;
        let extra = if local_zip64 {
            zip64_extra(
                member.uncompressed_size,
                member.compressed.len() as u64,
                None,
            )
        } else {
            Vec::new()
        };
        push_u32(&mut output, ZIP_LOCAL_SIGNATURE);
        push_u16(&mut output, if local_zip64 { 45 } else { 20 });
        push_u16(&mut output, UTF8_FLAG);
        push_u16(&mut output, member.method.code());
        push_u16(&mut output, 0);
        push_u16(&mut output, DOS_DATE_1980_01_01);
        push_u32(&mut output, member.crc32);
        push_u32(
            &mut output,
            if local_zip64 {
                u32::MAX
            } else {
                member.compressed.len() as u32
            },
        );
        push_u32(
            &mut output,
            if local_zip64 {
                u32::MAX
            } else {
                member.uncompressed_size as u32
            },
        );
        push_u16(&mut output, name.len() as u16);
        push_u16(&mut output, extra.len() as u16);
        output.extend_from_slice(name);
        output.extend_from_slice(&extra);
        output.extend_from_slice(&member.compressed);
        offsets.push((offset, local_zip64));
    }
    let central_offset = output.len() as u64;
    for (member, (offset, entry_zip64)) in members.iter().zip(offsets.iter().copied()) {
        let name = member.path.as_bytes();
        let central_zip64 = force_zip64
            || entry_zip64
            || member.uncompressed_size > ZIP32_MAX
            || member.compressed.len() as u64 > ZIP32_MAX
            || offset > ZIP32_MAX;
        zip64 |= central_zip64;
        let extra = if central_zip64 {
            zip64_extra(
                member.uncompressed_size,
                member.compressed.len() as u64,
                Some(offset),
            )
        } else {
            Vec::new()
        };
        push_u32(&mut central, ZIP_CENTRAL_SIGNATURE);
        push_u16(&mut central, if central_zip64 { 45 } else { 20 });
        push_u16(&mut central, if central_zip64 { 45 } else { 20 });
        push_u16(&mut central, UTF8_FLAG);
        push_u16(&mut central, member.method.code());
        push_u16(&mut central, 0);
        push_u16(&mut central, DOS_DATE_1980_01_01);
        push_u32(&mut central, member.crc32);
        push_u32(
            &mut central,
            if central_zip64 {
                u32::MAX
            } else {
                member.compressed.len() as u32
            },
        );
        push_u32(
            &mut central,
            if central_zip64 {
                u32::MAX
            } else {
                member.uncompressed_size as u32
            },
        );
        push_u16(&mut central, name.len() as u16);
        push_u16(&mut central, extra.len() as u16);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u32(&mut central, 0);
        push_u32(
            &mut central,
            if central_zip64 {
                u32::MAX
            } else {
                offset as u32
            },
        );
        central.extend_from_slice(name);
        central.extend_from_slice(&extra);
    }
    let central_size = central.len() as u64;
    zip64 |=
        central_size > ZIP32_MAX || central_offset > ZIP32_MAX || members.len() > u16::MAX as usize;
    output.extend_from_slice(&central);
    if zip64 {
        let zip64_offset = output.len() as u64;
        push_u32(&mut output, ZIP64_EOCD_SIGNATURE);
        push_u64(&mut output, 44);
        push_u16(&mut output, 45);
        push_u16(&mut output, 45);
        push_u32(&mut output, 0);
        push_u32(&mut output, 0);
        push_u64(&mut output, members.len() as u64);
        push_u64(&mut output, members.len() as u64);
        push_u64(&mut output, central_size);
        push_u64(&mut output, central_offset);
        push_u32(&mut output, ZIP64_LOCATOR_SIGNATURE);
        push_u32(&mut output, 0);
        push_u64(&mut output, zip64_offset);
        push_u32(&mut output, 1);
    }
    push_u32(&mut output, ZIP_EOCD_SIGNATURE);
    push_u16(&mut output, 0);
    push_u16(&mut output, 0);
    push_u16(
        &mut output,
        if zip64 {
            u16::MAX
        } else {
            members.len() as u16
        },
    );
    push_u16(
        &mut output,
        if zip64 {
            u16::MAX
        } else {
            members.len() as u16
        },
    );
    push_u32(
        &mut output,
        if zip64 { u32::MAX } else { central_size as u32 },
    );
    push_u32(
        &mut output,
        if zip64 {
            u32::MAX
        } else {
            central_offset as u32
        },
    );
    push_u16(&mut output, 0);
    Ok(output)
}

fn parse_zip(bytes: &[u8]) -> Result<ParsedZip, XarError> {
    let eocd = find_eocd(bytes)?;
    let disk = read_u16(bytes, eocd + 4)?;
    let central_disk = read_u16(bytes, eocd + 6)?;
    if disk != 0 || central_disk != 0 {
        return Err(XarError::Unsupported("多卷 ZIP 不受支持".to_owned()));
    }
    let entries16 = read_u16(bytes, eocd + 10)?;
    let central_size32 = read_u32(bytes, eocd + 12)?;
    let central_offset32 = read_u32(bytes, eocd + 16)?;
    let needs_zip64 =
        entries16 == u16::MAX || central_size32 == u32::MAX || central_offset32 == u32::MAX;
    let (count, central_size, central_offset) = if needs_zip64 {
        parse_zip64_directory(bytes, eocd)?
    } else {
        (
            entries16 as u64,
            central_size32 as u64,
            central_offset32 as u64,
        )
    };
    if count > MAX_MEMBER_COUNT {
        return Err(XarError::SizeLimit("成员数量超限".to_owned()));
    }
    let start = usize_from_u64(central_offset, "中央目录偏移")?;
    let size = usize_from_u64(central_size, "中央目录长度")?;
    let end = start
        .checked_add(size)
        .ok_or_else(|| XarError::Malformed("中央目录边界溢出".to_owned()))?;
    if end > bytes.len() || end > eocd {
        return Err(XarError::Malformed("中央目录越界".to_owned()));
    }
    let mut cursor = start;
    let mut members = Vec::with_capacity(count as usize);
    let mut paths = BTreeSet::new();
    for _ in 0..count {
        if read_u32(bytes, cursor)? != ZIP_CENTRAL_SIGNATURE {
            return Err(XarError::Malformed("中央目录成员魔数错误".to_owned()));
        }
        let version_needed = read_u16(bytes, cursor + 6)?;
        let flags = read_u16(bytes, cursor + 8)?;
        if flags != UTF8_FLAG {
            return Err(XarError::Unsupported(
                "成员必须使用 UTF-8 且不能带数据描述符".to_owned(),
            ));
        }
        let method = method_from_code(read_u16(bytes, cursor + 10)?)?;
        if read_u16(bytes, cursor + 14)? != DOS_DATE_1980_01_01
            || read_u16(bytes, cursor + 12)? != 0
        {
            return Err(XarError::InvalidMember("时间戳不是固定纪元".to_owned()));
        }
        let crc32 = read_u32(bytes, cursor + 16)?;
        let compressed32 = read_u32(bytes, cursor + 20)?;
        let uncompressed32 = read_u32(bytes, cursor + 24)?;
        let name_len = read_u16(bytes, cursor + 28)? as usize;
        let extra_len = read_u16(bytes, cursor + 30)? as usize;
        let comment_len = read_u16(bytes, cursor + 32)? as usize;
        if comment_len != 0
            || read_u16(bytes, cursor + 34)? != 0
            || read_u32(bytes, cursor + 38)? != 0
        {
            return Err(XarError::Unsupported(
                "成员注释、分卷或权限字段不受支持".to_owned(),
            ));
        }
        let end_header = cursor
            .checked_add(46)
            .and_then(|value| value.checked_add(name_len))
            .and_then(|value| value.checked_add(extra_len))
            .ok_or_else(|| XarError::Malformed("中央目录成员长度溢出".to_owned()))?;
        if end_header > end {
            return Err(XarError::Malformed("中央目录成员越界".to_owned()));
        }
        let name = std::str::from_utf8(&bytes[cursor + 46..cursor + 46 + name_len])
            .map_err(|_| XarError::InvalidMember("成员路径不是 UTF-8".to_owned()))?
            .to_owned();
        validate_physical_path(&name)?;
        if !paths.insert(name.clone()) {
            return Err(XarError::InvalidMember("物理成员重复".to_owned()));
        }
        let extra = &bytes[cursor + 46 + name_len..end_header];
        let local_offset32 = read_u32(bytes, cursor + 42)?;
        let (compressed_size64, uncompressed_size64, offset64) = parse_zip64_extra(
            extra,
            compressed32 == u32::MAX,
            uncompressed32 == u32::MAX,
            local_offset32 == u32::MAX,
        )?;
        let info = XarMemberInfo {
            path: name,
            compression: method,
            compressed_size: if compressed32 == u32::MAX {
                compressed_size64
            } else {
                compressed32 as u64
            },
            uncompressed_size: if uncompressed32 == u32::MAX {
                uncompressed_size64
            } else {
                uncompressed32 as u64
            },
            crc32,
            local_header_offset: if local_offset32 == u32::MAX {
                offset64
            } else {
                local_offset32 as u64
            },
        };
        validate_member_limits(&info)?;
        if version_needed > 45 {
            return Err(XarError::Unsupported("成员版本过高".to_owned()));
        }
        members.push(info);
        cursor = end_header;
    }
    if cursor != end {
        return Err(XarError::Malformed("中央目录有尾随字节".to_owned()));
    }
    for member in &members {
        validate_local_header(bytes, member)?;
    }
    let mut ranges = members
        .iter()
        .map(|member| member_range(bytes, member))
        .collect::<Result<Vec<_>, XarError>>()?;
    ranges.sort_by_key(|(start, _)| *start);
    for window in ranges.windows(2) {
        if window[0].1 > window[1].0 {
            return Err(XarError::Malformed("成员数据区重叠".to_owned()));
        }
    }
    let central_start = usize_from_u64(central_offset, "中央目录偏移")?;
    if ranges.iter().any(|(_, end)| *end > central_start) {
        return Err(XarError::Malformed("成员数据覆盖中央目录".to_owned()));
    }
    Ok(ParsedZip { members })
}

fn member_range(bytes: &[u8], member: &XarMemberInfo) -> Result<(usize, usize), XarError> {
    let offset = usize_from_u64(member.local_header_offset, "本地文件头偏移")?;
    let name_len = read_u16(bytes, offset + 26)? as usize;
    let extra_len = read_u16(bytes, offset + 28)? as usize;
    let start = offset
        .checked_add(30)
        .and_then(|value| value.checked_add(name_len))
        .and_then(|value| value.checked_add(extra_len))
        .ok_or_else(|| XarError::Malformed("成员数据偏移溢出".to_owned()))?;
    let end = start
        .checked_add(usize_from_u64(member.compressed_size, "成员压缩长度")?)
        .ok_or_else(|| XarError::Malformed("成员数据边界溢出".to_owned()))?;
    if end > bytes.len() {
        return Err(XarError::Malformed("成员数据越界".to_owned()));
    }
    Ok((offset, end))
}

fn validate_local_header(bytes: &[u8], member: &XarMemberInfo) -> Result<(), XarError> {
    let offset = usize_from_u64(member.local_header_offset, "本地文件头偏移")?;
    if read_u32(bytes, offset)? != ZIP_LOCAL_SIGNATURE {
        return Err(XarError::Malformed("本地文件头魔数错误".to_owned()));
    }
    let flags = read_u16(bytes, offset + 6)?;
    if flags != UTF8_FLAG || method_from_code(read_u16(bytes, offset + 8)?)? != member.compression {
        return Err(XarError::InvalidMember(
            "本地文件头与中央目录不一致".to_owned(),
        ));
    }
    if read_u16(bytes, offset + 10)? != 0 || read_u16(bytes, offset + 12)? != DOS_DATE_1980_01_01 {
        return Err(XarError::InvalidMember("本地文件头时间戳不规范".to_owned()));
    }
    if read_u32(bytes, offset + 14)? != member.crc32 {
        return Err(XarError::InvalidMember("本地文件头 CRC 不一致".to_owned()));
    }
    let compressed32 = read_u32(bytes, offset + 18)?;
    let uncompressed32 = read_u32(bytes, offset + 22)?;
    let name_len = read_u16(bytes, offset + 26)? as usize;
    let extra_len = read_u16(bytes, offset + 28)? as usize;
    let end_header = offset
        .checked_add(30)
        .and_then(|value| value.checked_add(name_len))
        .and_then(|value| value.checked_add(extra_len))
        .ok_or_else(|| XarError::Malformed("本地文件头长度溢出".to_owned()))?;
    if end_header > bytes.len() {
        return Err(XarError::Malformed("本地文件头越界".to_owned()));
    }
    let name = std::str::from_utf8(&bytes[offset + 30..offset + 30 + name_len])
        .map_err(|_| XarError::InvalidMember("本地路径不是 UTF-8".to_owned()))?;
    if name != member.path {
        return Err(XarError::InvalidMember(
            "本地路径与中央目录不一致".to_owned(),
        ));
    }
    let extra = &bytes[offset + 30 + name_len..end_header];
    let (compressed_size64, uncompressed_size64, _) = parse_zip64_extra(
        extra,
        compressed32 == u32::MAX,
        uncompressed32 == u32::MAX,
        false,
    )?;
    if (if compressed32 == u32::MAX {
        compressed_size64
    } else {
        compressed32 as u64
    }) != member.compressed_size
        || (if uncompressed32 == u32::MAX {
            uncompressed_size64
        } else {
            uncompressed32 as u64
        }) != member.uncompressed_size
    {
        return Err(XarError::InvalidMember(
            "本地长度与中央目录不一致".to_owned(),
        ));
    }
    let data_end = end_header
        .checked_add(usize_from_u64(member.compressed_size, "成员压缩长度")?)
        .ok_or_else(|| XarError::Malformed("成员数据边界溢出".to_owned()))?;
    if data_end > bytes.len() {
        return Err(XarError::Malformed("成员数据越界".to_owned()));
    }
    Ok(())
}

fn decode_member(bytes: &[u8], member: &XarMemberInfo) -> Result<Vec<u8>, XarError> {
    let offset = usize_from_u64(member.local_header_offset, "本地文件头偏移")?;
    let name_len = read_u16(bytes, offset + 26)? as usize;
    let extra_len = read_u16(bytes, offset + 28)? as usize;
    let data_start = offset
        .checked_add(30)
        .and_then(|value| value.checked_add(name_len))
        .and_then(|value| value.checked_add(extra_len))
        .ok_or_else(|| XarError::Malformed("成员数据偏移溢出".to_owned()))?;
    let compressed_len = usize_from_u64(member.compressed_size, "成员压缩长度")?;
    let data_end = data_start
        .checked_add(compressed_len)
        .ok_or_else(|| XarError::Malformed("成员数据长度溢出".to_owned()))?;
    if data_end > bytes.len() {
        return Err(XarError::Malformed("成员数据越界".to_owned()));
    }
    let compressed = &bytes[data_start..data_end];
    let decoded = match member.compression {
        CompressionMethod::Store => compressed.to_vec(),
        CompressionMethod::Deflate => {
            let decoder = DeflateDecoder::new(Cursor::new(compressed));
            let mut output = Vec::with_capacity(member.uncompressed_size.min(64 * 1024) as usize);
            decoder
                .take(member.uncompressed_size.saturating_add(1))
                .read_to_end(&mut output)
                .map_err(|error| XarError::Compression(error.to_string()))?;
            output
        }
    };
    if decoded.len() as u64 != member.uncompressed_size {
        return Err(XarError::Malformed("解压长度与中央目录不一致".to_owned()));
    }
    let mut crc = Crc32::new();
    crc.update(&decoded);
    if crc.finalize() != member.crc32 {
        return Err(XarError::Integrity("CRC-32 不匹配".to_owned()));
    }
    Ok(decoded)
}

fn validate_member_limits(member: &XarMemberInfo) -> Result<(), XarError> {
    if member.uncompressed_size > MAX_MEMBER_SIZE {
        return Err(XarError::SizeLimit("成员解压长度超限".to_owned()));
    }
    if member.uncompressed_size > 0
        && (member.compressed_size == 0
            || member.uncompressed_size
                > member
                    .compressed_size
                    .checked_mul(MAX_COMPRESSION_RATIO)
                    .ok_or_else(|| XarError::SizeLimit("成员压缩比计算溢出".to_owned()))?)
    {
        return Err(XarError::SizeLimit("成员压缩比疑似 ZIP bomb".to_owned()));
    }
    Ok(())
}

fn find_eocd(bytes: &[u8]) -> Result<usize, XarError> {
    if bytes.len() < 22 {
        return Err(XarError::Malformed("缺少 EOCD".to_owned()));
    }
    let start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    for offset in (start..=bytes.len() - 22).rev() {
        if read_u32(bytes, offset)? == ZIP_EOCD_SIGNATURE {
            let comment_len = read_u16(bytes, offset + 20)? as usize;
            if offset + 22 + comment_len == bytes.len() {
                if comment_len != 0 {
                    return Err(XarError::Unsupported("归档注释必须为空".to_owned()));
                }
                return Ok(offset);
            }
        }
    }
    Err(XarError::Malformed("找不到完整 EOCD".to_owned()))
}

fn parse_zip64_directory(bytes: &[u8], eocd: usize) -> Result<(u64, u64, u64), XarError> {
    if eocd < 20 || read_u32(bytes, eocd - 20)? != ZIP64_LOCATOR_SIGNATURE {
        return Err(XarError::Malformed("缺少 ZIP64 locator".to_owned()));
    }
    if read_u32(bytes, eocd - 16)? != 0 || read_u32(bytes, eocd - 4)? != 1 {
        return Err(XarError::Unsupported("ZIP64 多卷归档不受支持".to_owned()));
    }
    let record = usize_from_u64(read_u64(bytes, eocd - 12)?, "ZIP64 EOCD 偏移")?;
    if record > bytes.len() || record.checked_add(56).is_none_or(|end| end > bytes.len()) {
        return Err(XarError::Malformed("ZIP64 EOCD 越界".to_owned()));
    }
    let record_size = read_u64(bytes, record + 4)?;
    let record_size_usize = usize_from_u64(record_size, "ZIP64 EOCD 长度")?;
    let record_end = record
        .checked_add(12)
        .and_then(|value| value.checked_add(record_size_usize))
        .ok_or_else(|| XarError::Malformed("ZIP64 EOCD 边界溢出".to_owned()))?;
    if record_end > eocd || read_u32(bytes, record)? != ZIP64_EOCD_SIGNATURE || record_size < 44 {
        return Err(XarError::Malformed("ZIP64 EOCD 不完整".to_owned()));
    }
    if read_u32(bytes, record + 16)? != 0 || read_u32(bytes, record + 20)? != 0 {
        return Err(XarError::Unsupported("ZIP64 多卷归档不受支持".to_owned()));
    }
    Ok((
        read_u64(bytes, record + 32)?,
        read_u64(bytes, record + 40)?,
        read_u64(bytes, record + 48)?,
    ))
}

fn parse_zip64_extra(
    extra: &[u8],
    need_compressed: bool,
    need_uncompressed: bool,
    need_offset: bool,
) -> Result<(u64, u64, u64), XarError> {
    if !need_compressed && !need_uncompressed && !need_offset {
        if !extra.is_empty() {
            return Err(XarError::Unsupported(
                "存在未冻结的 ZIP extra field".to_owned(),
            ));
        }
        return Ok((0, 0, 0));
    }
    if extra.len() < 4 || read_u16(extra, 0)? != ZIP64_EXTRA_ID {
        return Err(XarError::Malformed("缺少 ZIP64 extra field".to_owned()));
    }
    let length = read_u16(extra, 2)? as usize;
    if length + 4 != extra.len() {
        return Err(XarError::Malformed("ZIP64 extra field 长度错误".to_owned()));
    }
    let mut cursor = 4;
    let uncompressed = if need_uncompressed {
        let value = read_u64(extra, cursor)?;
        cursor += 8;
        value
    } else {
        0
    };
    let compressed = if need_compressed {
        let value = read_u64(extra, cursor)?;
        cursor += 8;
        value
    } else {
        0
    };
    let offset = if need_offset {
        let value = read_u64(extra, cursor)?;
        cursor += 8;
        value
    } else {
        0
    };
    if cursor != extra.len() {
        return Err(XarError::Malformed(
            "ZIP64 extra field 含尾随数据".to_owned(),
        ));
    }
    Ok((compressed, uncompressed, offset))
}

fn zip64_extra(uncompressed: u64, compressed: u64, offset: Option<u64>) -> Vec<u8> {
    let length = if offset.is_some() { 24 } else { 16 };
    let mut output = Vec::with_capacity(length + 4);
    push_u16(&mut output, ZIP64_EXTRA_ID);
    push_u16(&mut output, length as u16);
    push_u64(&mut output, uncompressed);
    push_u64(&mut output, compressed);
    if let Some(offset) = offset {
        push_u64(&mut output, offset);
    }
    output
}

fn method_from_code(code: u16) -> Result<CompressionMethod, XarError> {
    match code {
        STORE_METHOD => Ok(CompressionMethod::Store),
        DEFLATE_METHOD => Ok(CompressionMethod::Deflate),
        _ => Err(XarError::Unsupported(format!("压缩方法 {code}"))),
    }
}

fn usize_from_u64(value: u64, field: &str) -> Result<usize, XarError> {
    usize::try_from(value).map_err(|_| XarError::Malformed(format!("{field} 超出宿主范围")))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, XarError> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| XarError::Malformed("读取 u16 边界溢出".to_owned()))?;
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| XarError::Malformed("读取 u16 越界".to_owned()))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, XarError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| XarError::Malformed("读取 u32 边界溢出".to_owned()))?;
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| XarError::Malformed("读取 u32 越界".to_owned()))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, XarError> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| XarError::Malformed("读取 u64 边界溢出".to_owned()))?;
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| XarError::Malformed("读取 u64 越界".to_owned()))?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use xiao_artifacts::ArchiveEntry;

    fn index_for(bytes: &[u8]) -> (ArchiveIndex, XarObject) {
        let object = XarObject::from_bytes(ObjectKind::Source, bytes);
        let index = ArchiveIndex {
            schema_major: INDEX_SCHEMA_MAJOR,
            schema_minor: INDEX_SCHEMA_MINOR,
            entry: "main.xiao".to_owned(),
            entries: vec![ArchiveEntry {
                logical_path: "main.xiao".to_owned(),
                object_kind: ObjectKind::Source,
                digest: object.digest,
                module: "main".to_owned(),
                target: "portable".to_owned(),
                length: object.bytes.len() as u64,
            }],
            dependency_lock_digest:
                "0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
            runtime_abi_min: 1,
            runtime_abi_max: 1,
            platform: "portable".to_owned(),
            debug_activation: false,
            language_locale: "zh-CN".to_owned(),
        };
        (index, object)
    }

    #[test]
    fn deterministic_round_trip_and_index_store() {
        let (index, object) = index_for(b"hello hello hello hello");
        let first = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
        let second = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
        assert_eq!(first, second);
        let archive = decode_xar(&first).unwrap();
        assert_eq!(archive.index(), &index);
        assert_eq!(archive.read_entry().unwrap(), object.bytes);
        assert_eq!(archive.members()[0].path, XAR_INDEX_PATH);
        assert_eq!(archive.members()[0].compression, CompressionMethod::Store);
        assert!(toolchain_fingerprint().contains("flate2=1.1.10"));
    }

    #[test]
    fn zip64_round_trip_and_equal_compression_uses_store() {
        let (index, object) = index_for(b"a");
        let bytes = encode_xar_with_options(
            &index,
            std::slice::from_ref(&object),
            XarBuildOptions {
                force_zip64: true,
                ..Default::default()
            },
        )
        .unwrap();
        let archive = decode_xar(&bytes).unwrap();
        assert_eq!(archive.read_entry().unwrap(), b"a");
        assert!(
            archive
                .members()
                .iter()
                .all(|member| member.compressed_size > 0)
        );
        assert!(
            archive
                .members()
                .iter()
                .find(|member| member.path.starts_with("objects/"))
                .is_some_and(|member| member.compression == CompressionMethod::Store)
        );
    }

    #[test]
    fn rejects_path_traversal_and_truncation() {
        let (mut index, object) = index_for(b"payload");
        index.entry = "../main.xiao".to_owned();
        assert!(encode_xar(&index, std::slice::from_ref(&object)).is_err());
        let (index, object) = index_for(b"payload");
        let bytes = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
        for length in 0..bytes.len() {
            assert!(decode_xar(&bytes[..length]).is_err());
        }
    }

    #[test]
    fn rejects_duplicate_objects_absolute_paths_and_zip_bombs() {
        let (index, object) = index_for(b"payload");
        let duplicate = encode_xar(&index, &[object.clone(), object.clone()]);
        assert!(matches!(duplicate, Err(XarError::DuplicateObject(..))));

        let mut unsafe_index = index.clone();
        unsafe_index.entry = "C:/main.xiao".to_owned();
        assert!(encode_xar(&unsafe_index, std::slice::from_ref(&object)).is_err());

        let bomb_object = XarObject::from_bytes(ObjectKind::Source, vec![0_u8; 5_000_000]);
        let mut bomb_index = index_for(bomb_object.bytes.as_slice()).0;
        bomb_index.entries[0].digest = bomb_object.digest;
        bomb_index.entries[0].length = bomb_object.bytes.len() as u64;
        let bomb = encode_xar(&bomb_index, std::slice::from_ref(&bomb_object)).unwrap();
        assert!(decode_xar(&bomb).is_err());
    }

    #[test]
    fn rejects_encryption_and_multi_volume_flags() {
        let (index, object) = index_for(b"flag probe");
        let bytes = encode_xar(&index, std::slice::from_ref(&object)).unwrap();
        let central = bytes
            .windows(4)
            .position(|window| window == ZIP_CENTRAL_SIGNATURE.to_le_bytes())
            .unwrap();
        let mut encrypted = bytes.clone();
        encrypted[central + 8..central + 10].copy_from_slice(&1_u16.to_le_bytes());
        assert!(decode_xar(&encrypted).is_err());

        let mut multi_volume = bytes;
        let eocd = multi_volume.len() - 22;
        multi_volume[eocd + 4..eocd + 6].copy_from_slice(&1_u16.to_le_bytes());
        assert!(decode_xar(&multi_volume).is_err());
    }
}
