//! 离线索引快照、联邦合并及源列表指纹输入。

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::adapters::{IndexSnapshot, PackageSourceAdapter};
use crate::diagnostics::{SOURCE_INVALID_CODE, SOURCE_UNSUPPORTED_VERSION_CODE};
use crate::jcs::canonicalize_json;
use crate::source::{
    ConfiguredSource, SOURCE_PROTOCOL_VERSION, SourceDeclaration, SourceError, SourceListImport,
    valid_digest,
};

/// 完整或仅索引的源快照状态；不可达不能表现为空集合。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotStatus {
    /// 当前新鲜快照。
    Fresh,
    /// 已校验且可沿用的缓存快照。
    Cached,
    /// 无可验证快照。
    Unavailable,
}

/// 单个源的清单及 SHA-256 已验证的索引分片列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotManifest {
    /// 源协议版本。
    pub protocol_version: u64,
    /// 规范源身份。
    pub source_id: String,
    /// 不可变快照标识。
    pub snapshot_id: String,
    /// 按包名稀疏查询的 JCS 摘要映射。
    pub shards: std::collections::BTreeMap<String, String>,
    /// 可选镜像端点，仅供后续传输使用，不改变源身份。
    #[serde(default)]
    pub mirrors: Vec<String>,
    /// 可选快照过期信息，仅作刷新参考。
    #[serde(default)]
    pub expires_at: Option<String>,
    /// 留给后续签名协议的可选元数据；本批不把它作为信任依据。
    #[serde(default)]
    pub signature: Option<String>,
}

impl SnapshotManifest {
    /// 未知协议版本直接拒绝。
    pub fn validate(&self, source_id: &str) -> Result<(), SourceError> {
        if self.protocol_version != SOURCE_PROTOCOL_VERSION {
            return Err(SourceError::new(
                SOURCE_UNSUPPORTED_VERSION_CODE,
                format!("不支持索引协议版本 {}", self.protocol_version),
            ));
        }
        if self.source_id != source_id
            || self.snapshot_id.is_empty()
            || self
                .shards
                .iter()
                .any(|(name, digest)| !valid_package_name(name) || !valid_digest(digest))
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "索引清单身份或分片名称不合法",
            ));
        }
        Ok(())
    }
}

/// 包正文和可选编译产物均按需读取。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReference {
    /// 相对源根目录的内容地址。
    pub location: String,
    /// 字节长度。
    pub length: u64,
    /// 小写 SHA-256 十六进制摘要。
    pub digest: String,
}

/// 仅保存解析依赖图需要的直接约束。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexDependency {
    /// 规范包名。
    pub name: String,
    /// 版本条件。
    pub version: String,
    /// 可选源别名或规范身份。
    pub source: Option<String>,
}

/// 源端发布的单个完整版本元数据，不含包正文。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexPackage {
    /// 规范包名。
    pub name: String,
    /// 版本文本。
    pub version: String,
    /// 目标/ABI 变体；通用源码为 `any`。
    pub variant: String,
    /// 已撤回的版本仍保留在索引中。
    pub withdrawn: bool,
    /// 直接依赖约束。
    pub dependencies: Vec<IndexDependency>,
    /// 特性条件（空表示无条件）。
    #[serde(default)]
    pub features: Vec<String>,
    /// 目标条件。
    pub target: Option<String>,
    /// ABI 条件。
    pub abi: Option<String>,
    /// Xiao 兼容范围。
    pub xiao_range: Option<String>,
    /// Runtime 兼容范围。
    pub runtime_range: Option<String>,
    /// 延迟获取的源码产物。
    pub source_artifact: ArtifactReference,
    /// 首版可为空的预编译产物。
    pub binary_artifacts: Vec<ArtifactReference>,
}

/// 单包分片携带所属快照以避免混合版本。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageShard {
    /// 协议版本。
    pub protocol_version: u64,
    /// 来源身份。
    pub source_id: String,
    /// 来源快照标识。
    pub snapshot_id: String,
    /// 此包的全部可用版本与变体。
    pub packages: Vec<IndexPackage>,
}

/// 已读取的源索引状态；失败仍必须保留源序号。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSnapshot {
    /// 包源最终序号。
    pub config_order: usize,
    /// 规范源身份。
    pub source_id: String,
    /// 可读到完整清单时保存的快照标识。
    pub snapshot_id: Option<String>,
    /// JCS 清单摘要。
    pub snapshot_digest: Option<String>,
    /// 本次使用的快照状态。
    pub status: SnapshotStatus,
    /// 已按包名稀疏读取的候选；完整源清单仍保留全部分片摘要。
    pub candidates: Vec<IndexPackage>,
    /// 已成功完成稀疏查询的包名；空候选不代表已查过该包。
    pub queried_packages: BTreeSet<String>,
}

impl SourceSnapshot {
    /// 根据已验证索引清单创建尚未查询任何包的可用快照。
    pub fn from_index(
        config_order: usize,
        status: SnapshotStatus,
        index: &IndexSnapshot,
    ) -> Result<Self, SourceError> {
        if status == SnapshotStatus::Unavailable {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "不可用源不能持有可用索引",
            ));
        }
        index.manifest.validate(&index.manifest.source_id)?;
        if !valid_digest(&index.digest) {
            return Err(SourceError::new(SOURCE_INVALID_CODE, "快照摘要不合法"));
        }
        Ok(Self {
            config_order,
            source_id: index.manifest.source_id.clone(),
            snapshot_id: Some(index.manifest.snapshot_id.clone()),
            snapshot_digest: Some(index.digest.clone()),
            status,
            candidates: Vec::new(),
            queried_packages: BTreeSet::new(),
        })
    }

    /// 仅当适配器成功返回指定快照的完整包分片后，才标记已查询。
    pub fn query_package(
        &mut self,
        adapter: &impl PackageSourceAdapter,
        source: &crate::source::SourceDescriptor,
        index: &IndexSnapshot,
        name: &str,
    ) -> Result<(), SourceError> {
        if self.status == SnapshotStatus::Unavailable
            || self.source_id != source.source.source_id
            || self.source_id != index.manifest.source_id
            || self.snapshot_id.as_deref() != Some(index.manifest.snapshot_id.as_str())
            || self.snapshot_digest.as_deref() != Some(index.digest.as_str())
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "查询源与快照身份不匹配",
            ));
        }
        let packages = adapter.read_package(source, index, name)?;
        self.candidates.retain(|package| package.name != name);
        self.candidates.extend(packages);
        self.queried_packages.insert(name.to_owned());
        Ok(())
    }
}

/// 索引记录的确定性合并键，保留来源与变体。
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct FederationKey {
    /// 源优先级。
    pub config_order: usize,
    /// 规范源身份。
    pub source_id: String,
    /// 包名。
    pub name: String,
    /// 版本。
    pub version: String,
    /// 目标/ABI 变体。
    pub variant: String,
}

/// 可供后续求解器使用的联邦记录。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FederatedRecord {
    /// 含来源维度的合并键。
    pub key: FederationKey,
    /// 产出此记录的不可变快照。
    pub snapshot_id: String,
    /// 清单规范字节摘要。
    pub snapshot_digest: String,
    /// 新鲜或沿用缓存的状态。
    pub status: SnapshotStatus,
    /// 完整版本元数据。
    pub package: IndexPackage,
}

/// 无需网络地合并已经验证的快照，拒绝单源重复身份，保留跨源同名同版本。
pub fn federate(snapshots: &[SourceSnapshot]) -> Result<Vec<FederatedRecord>, SourceError> {
    let mut output = Vec::new();
    let mut seen = BTreeSet::new();
    for snapshot in snapshots {
        if snapshot.status == SnapshotStatus::Unavailable {
            if !snapshot.candidates.is_empty() || !snapshot.queried_packages.is_empty() {
                return Err(SourceError::new(
                    SOURCE_INVALID_CODE,
                    "不可用源不得带候选或查询结果",
                ));
            }
            continue;
        }
        let (Some(snapshot_id), Some(snapshot_digest)) =
            (&snapshot.snapshot_id, &snapshot.snapshot_digest)
        else {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "可用快照缺少标识或摘要",
            ));
        };
        if snapshot_id.is_empty() || !valid_digest(snapshot_digest) {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "快照标识或摘要不合法",
            ));
        }
        for package in &snapshot.candidates {
            if !valid_package_name(&package.name)
                || !snapshot.queried_packages.contains(&package.name)
                || package.version.is_empty()
                || package.variant.is_empty()
            {
                return Err(SourceError::new(SOURCE_INVALID_CODE, "索引版本身份不合法"));
            }
            let key = FederationKey {
                config_order: snapshot.config_order,
                source_id: snapshot.source_id.clone(),
                name: package.name.clone(),
                version: package.version.clone(),
                variant: package.variant.clone(),
            };
            if !seen.insert(key.clone()) {
                return Err(SourceError::new(
                    SOURCE_INVALID_CODE,
                    "单源索引中有重复版本变体",
                ));
            }
            output.push(FederatedRecord {
                key,
                snapshot_id: snapshot_id.clone(),
                snapshot_digest: snapshot_digest.clone(),
                status: snapshot.status,
                package: package.clone(),
            });
        }
    }
    output.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(output)
}

/// 供既有配置指纹输入组合使用的有序源清单部分。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceListFingerprint {
    /// 完整展开列表的 SHA-256 JCS 摘要。
    pub list_digest: String,
    /// 保留重复、优先级和导入出处的规范源序列。
    pub ordered_sources: Vec<SourceSequenceEntry>,
}

/// 别名与显示名不影响缓存身份的源序列元素。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceSequenceEntry {
    /// 规范源身份。
    pub source_id: String,
    /// 直接声明时为空，导入时为列表地址。
    pub imported_from: Option<String>,
    /// 导入列表的固定摘要。
    pub import_digest: Option<String>,
}

/// 将列表摘要及最终优先级序列一并保留，供后续配置指纹组合。
pub fn source_list_fingerprint(sources: &[ConfiguredSource]) -> SourceListFingerprint {
    let mut ordered = sources.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|entry| entry.config_order);
    let ordered_sources = ordered
        .into_iter()
        .map(|entry| SourceSequenceEntry {
            source_id: entry.descriptor.source.source_id.clone(),
            imported_from: entry
                .imported_from
                .as_ref()
                .map(|item| item.location.clone()),
            import_digest: entry.imported_from.as_ref().map(|item| item.digest.clone()),
        })
        .collect::<Vec<_>>();
    fingerprint_source_sequence(ordered_sources)
}

/// 配置尚未读取导入清单时，以钉住的摘要标记间接来源；直接源仍按最终优先级编码。
pub(crate) fn source_declaration_fingerprint(
    declarations: &[SourceDeclaration],
) -> SourceListFingerprint {
    let mut ordered_sources = Vec::new();
    for declaration in declarations {
        if let SourceDeclaration::Direct(source) = declaration {
            ordered_sources.push(SourceSequenceEntry {
                source_id: source.source.source_id.clone(),
                imported_from: None,
                import_digest: None,
            });
        }
    }
    for declaration in declarations {
        if let SourceDeclaration::Import(SourceListImport { location, digest }) = declaration {
            ordered_sources.push(SourceSequenceEntry {
                source_id: String::new(),
                imported_from: Some(location.clone()),
                import_digest: Some(digest.clone()),
            });
        }
    }
    fingerprint_source_sequence(ordered_sources)
}

/// 在保留顺序与导入出处的序列上计算规范化摘要。
fn fingerprint_source_sequence(ordered_sources: Vec<SourceSequenceEntry>) -> SourceListFingerprint {
    let encoded = serde_json::to_string(&ordered_sources).expect("受控的源序列可序列化");
    let canonical = canonicalize_json(&encoded).expect("受控的源序列符合 I-JSON");
    SourceListFingerprint {
        list_digest: format!("{:x}", Sha256::digest(canonical)),
        ordered_sources,
    }
}

/// 仅接受可安全用于稀疏分片文件名的规范包名。
pub(crate) fn valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && !matches!(name, "." | "..")
        && name.bytes().all(|part| {
            part.is_ascii_lowercase() || part.is_ascii_digit() || matches!(part, b'-' | b'_' | b'.')
        })
}
