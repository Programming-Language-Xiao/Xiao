//! 包源身份、声明读取及有序源清单展开。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};
use std::path::{Component, Path};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use xiao_config::{ConfigDocument, ConfigValue};

use crate::adapters::GitReference;
use crate::diagnostics::{
    SOURCE_ALIAS_CONFLICT_CODE, SOURCE_DIGEST_MISMATCH_CODE, SOURCE_INVALID_CODE,
    SOURCE_UNKNOWN_REFERENCE_CODE, SOURCE_UNSUPPORTED_VERSION_CODE,
};
use crate::jcs::canonicalize_json;
use crate::model::PackageSource;

/// 首版静态索引协议。
pub const SOURCE_PROTOCOL_VERSION: u64 = 1;

/// 源配置或索引的稳定编号错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceError {
    /// 可由调用方映射为本地化诊断的编号。
    pub code: &'static str,
    /// 可读的错误上下文。
    pub message: String,
}

impl SourceError {
    /// 组合稳定源错误编号和上下文文本。
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for SourceError {
    /// 以编号和说明渲染错误。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SourceError {}

/// 按源种类与规范化地址生成稳定身份；路径源也调用本入口。
pub fn source_id(kind: &str, location: &str) -> Result<String, SourceError> {
    if !matches!(kind, "path" | "registry" | "static" | "git-index")
        || (kind != "path" && location.trim().is_empty())
    {
        return Err(SourceError::new(SOURCE_INVALID_CODE, "源类型或地址不合法"));
    }
    let normalized = if kind == "path" {
        let path = location.replace('\\', "/");
        let trimmed = path.trim_end_matches('/');
        if path.is_empty() {
            String::new()
        } else if trimmed.is_empty() {
            "/".to_owned()
        } else {
            trimmed.to_owned()
        }
    } else {
        let (scheme, remainder) = location
            .split_once("://")
            .ok_or_else(|| SourceError::new(SOURCE_INVALID_CODE, "远程源地址需要显式协议"))?;
        let scheme = scheme.to_ascii_lowercase();
        if !matches!(scheme.as_str(), "https" | "http" | "ssh")
            || (kind != "git-index" && scheme == "ssh")
        {
            return Err(SourceError::new(SOURCE_INVALID_CODE, "源地址协议不受支持"));
        }
        let (authority, path) = remainder.split_once('/').unwrap_or((remainder, ""));
        if authority.is_empty()
            || authority.contains(['@', '?', '#', '\\'])
            || path.contains(['?', '#', '\\'])
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "源地址不得包含认证信息、查询或片段",
            ));
        }
        let authority = authority.to_ascii_lowercase();
        let (host, port) = if authority.starts_with('[') {
            let closing = authority
                .find(']')
                .ok_or_else(|| SourceError::new(SOURCE_INVALID_CODE, "IPv6 主机缺少结束括号"))?;
            let host = &authority[..=closing];
            let suffix = &authority[closing + 1..];
            (
                host,
                suffix
                    .strip_prefix(':')
                    .or_else(|| suffix.is_empty().then_some("")),
            )
        } else {
            let (host, port) = authority
                .split_once(':')
                .map_or((authority.as_str(), Some("")), |(host, port)| {
                    (host, Some(port))
                });
            (host, port)
        };
        let port = port.ok_or_else(|| SourceError::new(SOURCE_INVALID_CODE, "主机端口不合法"))?;
        if host.is_empty()
            || host == "[]"
            || host.contains(':') && !host.starts_with('[')
            || !port.is_empty() && port.parse::<u16>().map_or(true, |number| number == 0)
        {
            return Err(SourceError::new(SOURCE_INVALID_CODE, "主机端口不合法"));
        }
        if host.starts_with('[')
            && host[1..host.len() - 1]
                .parse::<std::net::Ipv6Addr>()
                .is_err()
        {
            return Err(SourceError::new(SOURCE_INVALID_CODE, "IPv6 主机不合法"));
        }
        let default_port = if scheme == "https" {
            443
        } else if scheme == "http" {
            80
        } else {
            22
        };
        let authority = match port.parse::<u16>() {
            Ok(number) if number != default_port => format!("{host}:{number}"),
            _ => host.to_owned(),
        };
        if authority.chars().any(char::is_control)
            || authority.chars().any(char::is_whitespace)
            || path.split('/').any(|part| part == "..")
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "源地址的主机或路径不合法",
            ));
        }
        let trimmed = path.trim_end_matches('/');
        let trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);
        format!(
            "{scheme}://{authority}{}",
            if trimmed.is_empty() {
                String::new()
            } else {
                format!("/{trimmed}")
            }
        )
    };
    Ok(format!("{kind}:{normalized}"))
}

/// 单个可直接查询的源描述。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDescriptor {
    /// 原有包身份模型，别名与显示名不参与身份比较。
    pub source: PackageSource,
    /// `path`、`registry`、`static` 或 `git-index`。
    pub kind: String,
    /// 配置中的地址（路径或端点）。
    pub location: String,
    /// 当前支持的协议版本。
    pub protocol_version: u64,
    /// Git 稀疏索引的可变发现引用；空表示使用广告的 HEAD。
    pub git_reference: Option<GitReference>,
}

impl SourceDescriptor {
    /// 创建且校验源身份和协议版本。
    pub fn new(
        kind: &str,
        location: &str,
        alias: Option<&str>,
        display: Option<&str>,
        protocol_version: u64,
    ) -> Result<Self, SourceError> {
        if protocol_version != SOURCE_PROTOCOL_VERSION {
            return Err(SourceError::new(
                SOURCE_UNSUPPORTED_VERSION_CODE,
                format!("不支持源协议版本 {protocol_version}"),
            ));
        }
        if location.trim().is_empty()
            || alias
                .is_some_and(|value| value.trim().is_empty() || value.chars().any(char::is_control))
            || display
                .is_some_and(|value| value.trim().is_empty() || value.chars().any(char::is_control))
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "源地址、别名或显示名不合法",
            ));
        }
        if kind == "path"
            && (!Path::new(location).is_absolute()
                || Path::new(location)
                    .components()
                    .any(|component| matches!(component, Component::ParentDir | Component::CurDir)))
        {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "配置中的目录源需要不含点段的绝对路径",
            ));
        }
        Ok(Self {
            source: PackageSource {
                source_id: source_id(kind, location)?,
                alias: alias.map(str::to_owned),
                display_name: display.unwrap_or(location).to_owned(),
            },
            kind: kind.to_owned(),
            location: location.to_owned(),
            protocol_version,
            git_reference: None,
        })
    }

    /// 使同一仓库的不同 Git 引用拥有各自独立的缓存与源身份。
    pub fn with_git_reference(mut self, reference: GitReference) -> Result<Self, SourceError> {
        if self.kind != "git-index" || self.git_reference.is_some() {
            return Err(SourceError::new(
                SOURCE_INVALID_CODE,
                "只有未指定引用的 Git 索引源可声明 Git 引用",
            ));
        }
        let (kind, name) = match &reference {
            GitReference::Head => ("head", "HEAD"),
            GitReference::Branch(name) => ("branch", name.as_str()),
            GitReference::Tag(name) => ("tag", name.as_str()),
            GitReference::Commit(name) => ("rev", name.as_str()),
        };
        if !matches!(reference, GitReference::Head)
            && GitReference::from_config(kind, name)? != reference
        {
            return Err(SourceError::new(SOURCE_INVALID_CODE, "Git 引用不合法"));
        }
        self.source.source_id = format!("{}@{kind}:{name}", self.source.source_id);
        self.git_reference = Some(reference);
        Ok(self)
    }
}

/// 需要由上游显式提供并钉住摘要的源列表引用。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceListImport {
    /// 不参与源优先级的清单引用地址。
    pub location: String,
    /// 清单的 JCS SHA-256 摘要。
    pub digest: String,
}

/// 单个有序的配置声明。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceDeclaration {
    /// 直接声明的源。
    Direct(SourceDescriptor),
    /// 导入列表声明，不会联网读取。
    Import(SourceListImport),
}

/// 从已解析的静态树读取源声明，按源码位置恢复书写顺序。
pub fn source_declarations(
    document: &ConfigDocument,
) -> Result<Vec<SourceDeclaration>, SourceError> {
    let Some(table) = document.table("sources") else {
        return Ok(Vec::new());
    };
    let mut entries = table.entries.values().collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.span.start());
    entries
        .into_iter()
        .map(|entry| {
            let ConfigValue::Dictionary(fields) = &entry.value else {
                return Err(SourceError::new(
                    SOURCE_INVALID_CODE,
                    format!("源 {} 需要字典", entry.key),
                ));
            };
            if fields.contains_key("list") {
                return Ok(SourceDeclaration::Import(SourceListImport {
                    location: required_string(fields, "list")?.to_owned(),
                    digest: {
                        let digest = required_string(fields, "digest")?;
                        if !valid_digest(digest) {
                            return Err(SourceError::new(
                                SOURCE_INVALID_CODE,
                                "源列表摘要必须是 64 位小写 SHA-256",
                            ));
                        }
                        digest.to_owned()
                    },
                }));
            }
            let kind = required_string(fields, "kind")?;
            let location = required_string(fields, "location")?;
            let alias = fields
                .get("alias")
                .and_then(ConfigValue::as_str)
                .unwrap_or(&entry.key);
            let display = fields.get("display").and_then(ConfigValue::as_str);
            let version = match fields.get("protocol") {
                None => SOURCE_PROTOCOL_VERSION,
                Some(ConfigValue::Integer(version)) => u64::try_from(*version).map_err(|_| {
                    SourceError::new(SOURCE_UNSUPPORTED_VERSION_CODE, "协议版本无效")
                })?,
                Some(_) => {
                    return Err(SourceError::new(
                        SOURCE_INVALID_CODE,
                        "协议版本必须为正整数",
                    ));
                }
            };
            let descriptor = SourceDescriptor::new(kind, location, Some(alias), display, version)?;
            Ok(SourceDeclaration::Direct(with_reference(
                descriptor,
                |field| fields.get(field).and_then(ConfigValue::as_str),
            )?))
        })
        .collect()
}

fn with_reference<'a>(
    descriptor: SourceDescriptor,
    get: impl Fn(&str) -> Option<&'a str>,
) -> Result<SourceDescriptor, SourceError> {
    let refs = ["rev", "tag", "branch"]
        .into_iter()
        .filter_map(|kind| get(kind).map(|value| (kind, value)))
        .collect::<Vec<_>>();
    match refs.as_slice() {
        [] => Ok(descriptor),
        [(kind, value)] => descriptor.with_git_reference(GitReference::from_config(kind, value)?),
        _ => Err(SourceError::new(SOURCE_INVALID_CODE, "Git 源引用不可重复")),
    }
}

/// 从静态字典中读取必填的非空字符串。
fn required_string<'a>(
    fields: &'a BTreeMap<String, ConfigValue>,
    field: &str,
) -> Result<&'a str, SourceError> {
    fields
        .get(field)
        .and_then(ConfigValue::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| SourceError::new(SOURCE_INVALID_CODE, format!("源缺少合法 {field}")))
}

/// 已验证列表中的源，按列表内部顺序排列。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceList {
    /// 已钉住的 JCS 清单摘要。
    pub digest: String,
    /// 清单中的源。
    pub sources: Vec<SourceDescriptor>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// 与包索引清单互不嵌套的有序源列表文档。
struct SourceListDocument {
    protocol_version: u64,
    sources: Vec<SourceListEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// 外部列表中每个源的未校验输入形状。
struct SourceListEntry {
    kind: String,
    location: String,
    alias: Option<String>,
    display: Option<String>,
    protocol_version: Option<u64>,
    rev: Option<String>,
    tag: Option<String>,
    branch: Option<String>,
}

impl SourceList {
    /// 从独立于包索引的静态 JSON 清单生成已校验的展开来源和摘要。
    pub fn parse(text: &str) -> Result<Self, SourceError> {
        let canonical = canonicalize_json(text)?;
        let document: SourceListDocument = serde_json::from_str(text)
            .map_err(|error| SourceError::new(SOURCE_INVALID_CODE, error.to_string()))?;
        if document.protocol_version != SOURCE_PROTOCOL_VERSION {
            return Err(SourceError::new(
                SOURCE_UNSUPPORTED_VERSION_CODE,
                format!("不支持源列表协议版本 {}", document.protocol_version),
            ));
        }
        let sources = document
            .sources
            .into_iter()
            .map(|entry| {
                let descriptor = SourceDescriptor::new(
                    &entry.kind,
                    &entry.location,
                    entry.alias.as_deref(),
                    entry.display.as_deref(),
                    entry.protocol_version.unwrap_or(SOURCE_PROTOCOL_VERSION),
                )?;
                with_reference(descriptor, |field| match field {
                    "rev" => entry.rev.as_deref(),
                    "tag" => entry.tag.as_deref(),
                    "branch" => entry.branch.as_deref(),
                    _ => None,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            digest: format!("{:x}", Sha256::digest(canonical)),
            sources,
        })
    }
}

/// 校验小写完整 SHA-256 摘要的文本形状。
pub(crate) fn valid_digest(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// 展开后保留来源与优先级的配置源。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfiguredSource {
    /// 规范源描述。
    pub descriptor: SourceDescriptor,
    /// 最终优先级（从零开始）。
    pub config_order: usize,
    /// 直接配置为 `None`，导入时为清单引用及其摘要。
    pub imported_from: Option<SourceListImport>,
}

/// 所有直接源在前；导入列表按声明顺序、列表内部顺序在后。
pub fn expand_source_lists(
    declarations: &[SourceDeclaration],
    lists: &BTreeMap<String, SourceList>,
) -> Result<Vec<ConfiguredSource>, SourceError> {
    let mut result = Vec::new();
    let mut aliases = BTreeSet::new();
    let mut append = |descriptor: &SourceDescriptor,
                      imported_from: Option<SourceListImport>|
     -> Result<(), SourceError> {
        if let Some(alias) = &descriptor.source.alias {
            if !aliases.insert(alias.clone()) {
                return Err(SourceError::new(
                    SOURCE_ALIAS_CONFLICT_CODE,
                    format!("源别名 {alias:?} 重复"),
                ));
            }
        }
        result.push(ConfiguredSource {
            descriptor: descriptor.clone(),
            config_order: result.len(),
            imported_from,
        });
        Ok(())
    };
    for entry in declarations {
        if let SourceDeclaration::Direct(source) = entry {
            append(source, None)?;
        }
    }
    for entry in declarations {
        if let SourceDeclaration::Import(import) = entry {
            let list = lists.get(&import.location).ok_or_else(|| {
                SourceError::new(
                    SOURCE_UNKNOWN_REFERENCE_CODE,
                    format!("未提供源列表 {}", import.location),
                )
            })?;
            if list.digest != import.digest {
                return Err(SourceError::new(
                    SOURCE_DIGEST_MISMATCH_CODE,
                    format!("源列表 {} 的摘要不匹配", import.location),
                ));
            }
            for source in &list.sources {
                append(source, Some(import.clone()))?;
            }
        }
    }
    Ok(result)
}
