//! 资源型语言包的清单、校验、合并和进程内降级。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use xiao_runtime_abi::{ABI_ENCODED_VERSION, ABI_MAJOR_VERSION, ABI_MINOR_VERSION};

use crate::{CATALOG_VERSION, Catalog, LocaleContext, MessageRenderer, MessageTemplate};

/// 语言包清单文件名。
pub const LANGUAGE_PACK_MANIFEST_FILE: &str = "xiao-language-pack.json";
/// 首版语言包清单格式版本。
pub const LANGUAGE_PACK_MANIFEST_VERSION: u16 = 1;
/// 语言包清单无效的稳定诊断编号。
pub const LANGUAGE_PACK_MANIFEST_INVALID_CODE: &str = "X11-I18N-PACK-001";
/// 语言包兼容范围不满足的稳定诊断编号。
pub const LANGUAGE_PACK_COMPATIBILITY_CODE: &str = "X11-I18N-PACK-002";
/// 语言包内容摘要不匹配的稳定诊断编号。
pub const LANGUAGE_PACK_DIGEST_MISMATCH_CODE: &str = "X11-I18N-PACK-003";
/// 语言包目录冲突的稳定诊断编号。
pub const LANGUAGE_PACK_CONFLICT_CODE: &str = "X11-I18N-PACK-004";
/// 语言包资源文件无效的稳定诊断编号。
pub const LANGUAGE_PACK_RESOURCE_INVALID_CODE: &str = "X11-I18N-PACK-005";

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_RESOURCE_BYTES: u64 = 16 * 1024 * 1024;

/// 语言包与当前 Xiao/Runtime 的兼容上下文。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguagePackCompatibility {
    /// 当前 Xiao 版本，使用严格三段版本文本。
    pub xiao_version: String,
    /// 当前 Runtime ABI 的可比较编码。
    pub runtime_abi: u64,
}

impl LanguagePackCompatibility {
    /// 返回当前构建使用的 Xiao 和 Runtime ABI 版本。
    #[must_use]
    pub fn current() -> Self {
        Self {
            xiao_version: env!("CARGO_PKG_VERSION").to_owned(),
            runtime_abi: ABI_ENCODED_VERSION,
        }
    }

    fn runtime_version(&self) -> String {
        if self.runtime_abi == ABI_ENCODED_VERSION {
            return format!("{ABI_MAJOR_VERSION}.{ABI_MINOR_VERSION}.0");
        }
        format!("{}.{}.0", self.runtime_abi >> 16, self.runtime_abi & 0xffff)
    }
}

/// 资源型语言包的声明式清单。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LanguagePackManifest {
    /// 清单格式版本。
    pub manifest_version: u16,
    /// 插件的稳定标识，不是安装顺序或显示名称。
    pub plugin_id: String,
    /// 语言包自身的严格 SemVer 版本。
    pub package_version: String,
    /// 提供的语言目录，键为规范语言标签，值为包内相对资源路径。
    pub catalogs: BTreeMap<String, String>,
    /// 消息目录格式版本。
    pub catalog_version: u16,
    /// 兼容的 Xiao 版本范围，使用受限的 SemVer 合取语法。
    pub xiao_version: String,
    /// 兼容的 Runtime ABI 版本范围，使用受限的 SemVer 合取语法。
    pub runtime_abi: String,
    /// 所有目录资源文件的原始字节总长度，不包含本清单。
    pub content_length: u64,
    /// 目录资源规范摘要，不包含本清单。
    pub sha256: String,
    /// 可选显示名称。
    #[serde(default)]
    pub display_name: Option<String>,
    /// 可选版权声明。
    #[serde(default)]
    pub copyright: Option<String>,
    /// 可选贡献者列表。
    #[serde(default)]
    pub contributors: Vec<String>,
}

impl LanguagePackManifest {
    /// 校验清单自身及其对当前运行时的兼容性声明。
    pub fn validate(
        &self,
        compatibility: &LanguagePackCompatibility,
    ) -> Result<(), LanguagePackError> {
        if self.manifest_version != LANGUAGE_PACK_MANIFEST_VERSION
            || self.catalog_version != CATALOG_VERSION
        {
            return Err(LanguagePackError::Manifest {
                path: PathBuf::from(LANGUAGE_PACK_MANIFEST_FILE),
                message: "清单或目录版本不受支持".to_owned(),
            });
        }
        if !valid_identifier(&self.plugin_id) {
            return Err(LanguagePackError::Manifest {
                path: PathBuf::from(LANGUAGE_PACK_MANIFEST_FILE),
                message: "插件标识必须是无控制字符的稳定标识".to_owned(),
            });
        }
        if parse_version(&self.package_version).is_err() {
            return Err(LanguagePackError::Manifest {
                path: PathBuf::from(LANGUAGE_PACK_MANIFEST_FILE),
                message: "包版本必须是严格三段 SemVer".to_owned(),
            });
        }
        if self.catalogs.is_empty() {
            return Err(LanguagePackError::Manifest {
                path: PathBuf::from(LANGUAGE_PACK_MANIFEST_FILE),
                message: "语言包至少要提供一个目录".to_owned(),
            });
        }
        if !valid_digest(&self.sha256) {
            return Err(LanguagePackError::Manifest {
                path: PathBuf::from(LANGUAGE_PACK_MANIFEST_FILE),
                message: "目录摘要必须是 64 位小写 SHA-256".to_owned(),
            });
        }
        if !requirement_valid(&self.xiao_version)
            || !requirement_valid(&self.runtime_abi)
            || !requirement_matches(&self.xiao_version, &compatibility.xiao_version)
                .unwrap_or(false)
            || !requirement_matches(&self.runtime_abi, &compatibility.runtime_version())
                .unwrap_or(false)
        {
            return Err(LanguagePackError::Compatibility {
                plugin_id: self.plugin_id.clone(),
                xiao_requirement: self.xiao_version.clone(),
                runtime_requirement: self.runtime_abi.clone(),
                current_xiao: compatibility.xiao_version.clone(),
                current_runtime: compatibility.runtime_version(),
            });
        }
        for locale in self.catalogs.keys() {
            validate_locale(locale).map_err(|message| LanguagePackError::Manifest {
                path: PathBuf::from(LANGUAGE_PACK_MANIFEST_FILE),
                message,
            })?;
        }
        for value in [self.display_name.as_deref(), self.copyright.as_deref()]
            .into_iter()
            .flatten()
        {
            if value.chars().any(char::is_control) {
                return Err(LanguagePackError::Manifest {
                    path: PathBuf::from(LANGUAGE_PACK_MANIFEST_FILE),
                    message: "清单显示元数据不得包含控制字符".to_owned(),
                });
            }
        }
        if self
            .contributors
            .iter()
            .any(|value| value.is_empty() || value.chars().any(char::is_control))
        {
            return Err(LanguagePackError::Manifest {
                path: PathBuf::from(LANGUAGE_PACK_MANIFEST_FILE),
                message: "贡献者名称不得为空或含控制字符".to_owned(),
            });
        }
        Ok(())
    }
}

/// 语言资源内容对应的不可变缓存键。
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct LanguagePackCacheKey {
    /// 目录格式版本。
    pub catalog_version: u16,
    /// Xiao 版本。
    pub xiao_version: String,
    /// Runtime ABI 可比较编码。
    pub runtime_abi: u64,
    /// 目录资源摘要。
    pub content_digest: String,
}

impl LanguagePackCacheKey {
    /// 创建并校验一枚语言资源缓存键。
    pub fn new(
        catalog_version: u16,
        xiao_version: impl Into<String>,
        runtime_abi: u64,
        content_digest: impl Into<String>,
    ) -> Result<Self, LanguagePackError> {
        let key = Self {
            catalog_version,
            xiao_version: xiao_version.into(),
            runtime_abi,
            content_digest: content_digest.into(),
        };
        key.validate()?;
        Ok(key)
    }

    /// 校验缓存键字段，供反序列化或公开字段构造后的缓存边界再次确认。
    pub fn validate(&self) -> Result<(), LanguagePackError> {
        if self.catalog_version == 0
            || parse_version(&self.xiao_version).is_err()
            || self
                .xiao_version
                .chars()
                .any(|character| character.is_control() || matches!(character, '/' | '\\'))
            || !valid_digest(&self.content_digest)
        {
            return Err(LanguagePackError::Manifest {
                path: PathBuf::from(LANGUAGE_PACK_MANIFEST_FILE),
                message: "语言资源缓存键包含非法字段".to_owned(),
            });
        }
        Ok(())
    }

    /// 返回不会与源码对象缓存混淆的相对对象路径。
    #[must_use]
    pub fn relative_path(&self) -> String {
        format!(
            "catalog-v{}/xiao-v{}/abi-{}/{}",
            self.catalog_version, self.xiao_version, self.runtime_abi, self.content_digest
        )
    }
}

/// 已通过清单、摘要和目录签名校验的语言包。
#[derive(Clone, Debug)]
pub struct LanguagePack {
    manifest: LanguagePackManifest,
    catalogs: BTreeMap<String, Catalog>,
    resources: BTreeMap<String, Vec<u8>>,
    manifest_bytes: Vec<u8>,
    content_digest: String,
    content_length: u64,
}

impl LanguagePack {
    /// 从目录加载语言包；此过程只读取 JSON 资源，不执行任何包内容。
    pub fn load_directory(
        root: impl AsRef<Path>,
        compatibility: &LanguagePackCompatibility,
    ) -> Result<Self, LanguagePackError> {
        let root = root.as_ref();
        let root_metadata =
            fs::symlink_metadata(root).map_err(|error| resource_error(root, error))?;
        if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
            return Err(LanguagePackError::Resource {
                path: root.to_path_buf(),
                message: "语言包根必须是普通目录".to_owned(),
            });
        }
        let manifest_path = root.join(LANGUAGE_PACK_MANIFEST_FILE);
        let manifest_bytes = read_bounded(&manifest_path, MAX_MANIFEST_BYTES)?;
        let manifest: LanguagePackManifest =
            serde_json::from_slice(&manifest_bytes).map_err(|error| {
                LanguagePackError::Manifest {
                    path: manifest_path.clone(),
                    message: format!("清单不是严格 JSON：{error}"),
                }
            })?;
        manifest.validate(compatibility)?;

        let mut catalogs = BTreeMap::new();
        let mut resources = BTreeMap::new();
        let mut allowed = BTreeSet::from([LANGUAGE_PACK_MANIFEST_FILE.to_owned()]);
        let mut seen_paths = BTreeSet::new();
        for (declared_locale, relative_path) in &manifest.catalogs {
            let locale = canonical_locale(declared_locale).map_err(|message| {
                LanguagePackError::Manifest {
                    path: manifest_path.clone(),
                    message,
                }
            })?;
            let relative_path =
                resource_path(relative_path).map_err(|message| LanguagePackError::Manifest {
                    path: manifest_path.clone(),
                    message,
                })?;
            if !relative_path.starts_with("catalogs/") || !relative_path.ends_with(".json") {
                return Err(LanguagePackError::Manifest {
                    path: manifest_path.clone(),
                    message: "目录资源必须位于 catalogs/ 下且使用 .json 扩展名".to_owned(),
                });
            }
            if !seen_paths.insert(relative_path.clone()) {
                return Err(LanguagePackError::Manifest {
                    path: manifest_path.clone(),
                    message: "多个语言标签不能指向同一个目录文件".to_owned(),
                });
            }
            let path = root.join(Path::new(&relative_path));
            let bytes = read_bounded(&path, MAX_RESOURCE_BYTES)?;
            let document: CatalogDocument =
                serde_json::from_slice(&bytes).map_err(|error| LanguagePackError::Resource {
                    path: path.clone(),
                    message: format!("目录不是严格 JSON：{error}"),
                })?;
            if document.version != manifest.catalog_version {
                return Err(LanguagePackError::Resource {
                    path,
                    message: "目录版本与清单不一致".to_owned(),
                });
            }
            let document_locale = canonical_locale(&document.locale).map_err(|message| {
                LanguagePackError::Resource {
                    path: root.join(Path::new(&relative_path)),
                    message,
                }
            })?;
            if document_locale != locale {
                return Err(LanguagePackError::Resource {
                    path: root.join(Path::new(&relative_path)),
                    message: "目录文件中的语言标签与清单不一致".to_owned(),
                });
            }
            let catalog =
                Catalog::new_strict(locale.clone(), document.entries).map_err(|message| {
                    LanguagePackError::Resource {
                        path: root.join(Path::new(&relative_path)),
                        message,
                    }
                })?;
            if catalogs.insert(locale, catalog).is_some() {
                return Err(LanguagePackError::Manifest {
                    path: manifest_path.clone(),
                    message: "清单声明了重复的规范语言标签".to_owned(),
                });
            }
            allowed.insert(relative_path.clone());
            resources.insert(relative_path, bytes);
        }

        let actual_files = collect_files(root)?;
        for relative_path in actual_files {
            if !allowed.contains(&relative_path) {
                return Err(LanguagePackError::Resource {
                    path: root.join(Path::new(&relative_path)),
                    message: "语言包只能包含清单和声明的 JSON 目录资源".to_owned(),
                });
            }
        }
        let (content_length, content_digest) = digest_resources(&resources);
        if content_length != manifest.content_length || content_digest != manifest.sha256 {
            return Err(LanguagePackError::DigestMismatch {
                plugin_id: manifest.plugin_id.clone(),
                expected: manifest.sha256,
                actual: content_digest,
                content_length: manifest.content_length,
                actual_length: content_length,
            });
        }
        Ok(Self {
            manifest,
            catalogs,
            resources,
            manifest_bytes,
            content_digest,
            content_length,
        })
    }

    /// 返回经过校验的清单。
    #[must_use]
    pub fn manifest(&self) -> &LanguagePackManifest {
        &self.manifest
    }

    /// 返回资源目录摘要。
    #[must_use]
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    /// 返回资源目录原始字节长度。
    #[must_use]
    pub const fn content_length(&self) -> u64 {
        self.content_length
    }

    /// 返回只读的规范化目录集合。
    #[must_use]
    pub fn catalogs(&self) -> &BTreeMap<String, Catalog> {
        &self.catalogs
    }

    /// 返回包内目录资源的原始字节；调用方不得修改缓存对象内容。
    #[must_use]
    pub fn resources(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.resources
    }

    /// 返回原始清单字节，供包缓存保留声明内容。
    #[must_use]
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    /// 按当前构建的 Xiao/Runtime 版本生成缓存键。
    pub fn cache_key(&self) -> Result<LanguagePackCacheKey, LanguagePackError> {
        self.cache_key_for(&LanguagePackCompatibility::current())
    }

    /// 按指定 Xiao/Runtime 版本生成缓存键，便于跨版本缓存隔离测试。
    pub fn cache_key_for(
        &self,
        compatibility: &LanguagePackCompatibility,
    ) -> Result<LanguagePackCacheKey, LanguagePackError> {
        LanguagePackCacheKey::new(
            self.manifest.catalog_version,
            compatibility.xiao_version.clone(),
            compatibility.runtime_abi,
            self.content_digest.clone(),
        )
    }
}

/// 可合并的语言包目录注册表；内置目录始终先于插件目录存在。
#[derive(Clone, Debug)]
pub struct LanguagePackRegistry {
    catalogs: BTreeMap<String, Catalog>,
    origins: BTreeMap<(String, String), String>,
    installed: BTreeMap<String, String>,
}

impl Default for LanguagePackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguagePackRegistry {
    /// 创建只含内置中英目录的注册表。
    #[must_use]
    pub fn new() -> Self {
        let builtin = crate::builtin_renderer();
        let catalogs = builtin
            .catalogs_snapshot()
            .into_iter()
            .map(|catalog| (catalog.locale.clone(), catalog))
            .collect::<BTreeMap<_, _>>();
        let mut origins = BTreeMap::new();
        for catalog in catalogs.values() {
            for id in catalog.entries().keys() {
                origins.insert((catalog.locale.clone(), id.clone()), "builtin".to_owned());
            }
        }
        Self {
            catalogs,
            origins,
            installed: BTreeMap::new(),
        }
    }

    /// 以原子方式注册一套已经验证的语言包目录。
    pub fn register(&mut self, pack: &LanguagePack) -> Result<(), LanguagePackError> {
        let mut candidate = self.clone();
        candidate.register_inner(pack)?;
        *self = candidate;
        Ok(())
    }

    /// 使用当前注册表生成不可变渲染器。
    pub fn renderer(&self) -> Result<MessageRenderer, LanguagePackError> {
        MessageRenderer::new(self.catalogs.values().cloned().collect()).map_err(|message| {
            LanguagePackError::Manifest {
                path: PathBuf::from(LANGUAGE_PACK_MANIFEST_FILE),
                message,
            }
        })
    }

    /// 返回已安装插件的标识和资源摘要。
    #[must_use]
    pub fn installed(&self) -> &BTreeMap<String, String> {
        &self.installed
    }

    fn register_inner(&mut self, pack: &LanguagePack) -> Result<(), LanguagePackError> {
        if self.installed.contains_key(&pack.manifest.plugin_id) {
            self.remove_plugin(&pack.manifest.plugin_id)?;
        }
        let origin = format!(
            "{}@{}#{}",
            pack.manifest.plugin_id, pack.manifest.package_version, pack.content_digest
        );
        for (locale, catalog) in &pack.catalogs {
            let mut entries = self
                .catalogs
                .get(locale)
                .map(|existing| existing.entries().clone())
                .unwrap_or_default();
            for (id, entry) in catalog.entries() {
                if entries.contains_key(id) {
                    return Err(LanguagePackError::Conflict {
                        plugin_id: pack.manifest.plugin_id.clone(),
                        digest: pack.content_digest.clone().into_boxed_str(),
                        locale: locale.clone(),
                        message_id: id.clone(),
                        existing_source: self
                            .origins
                            .get(&(locale.clone(), id.clone()))
                            .cloned()
                            .unwrap_or_else(|| "已注册目录".to_owned())
                            .into_boxed_str(),
                        reason: "同一语言标签中的 message_id 重复，禁止按安装顺序覆盖"
                            .to_owned()
                            .into_boxed_str(),
                    });
                }
                if let Some(previous) = self.find_signature(id) {
                    if previous != entry.params {
                        return Err(LanguagePackError::Conflict {
                            plugin_id: pack.manifest.plugin_id.clone(),
                            digest: pack.content_digest.clone().into_boxed_str(),
                            locale: locale.clone(),
                            message_id: id.clone(),
                            existing_source: "已注册目录".to_owned().into_boxed_str(),
                            reason: "message_id 的参数签名不一致".to_owned().into_boxed_str(),
                        });
                    }
                }
                entries.insert(id.clone(), entry.clone());
            }
            let merged = Catalog::from_entries(locale.clone(), entries.into_values()).map_err(
                |message| LanguagePackError::Conflict {
                    plugin_id: pack.manifest.plugin_id.clone(),
                    digest: pack.content_digest.clone().into_boxed_str(),
                    locale: locale.clone(),
                    message_id: "<catalog>".to_owned(),
                    existing_source: "已注册目录".to_owned().into_boxed_str(),
                    reason: message.into_boxed_str(),
                },
            )?;
            for id in catalog.entries().keys() {
                self.origins
                    .insert((locale.clone(), id.clone()), origin.clone());
            }
            self.catalogs.insert(locale.clone(), merged);
        }
        self.installed
            .insert(pack.manifest.plugin_id.clone(), pack.content_digest.clone());
        Ok(())
    }

    fn remove_plugin(&mut self, plugin_id: &str) -> Result<(), LanguagePackError> {
        let source_prefix = format!("{plugin_id}@");
        let digest = self.installed.get(plugin_id).cloned().unwrap_or_default();
        let entries = self
            .origins
            .iter()
            .filter(|(_, source)| source.starts_with(&source_prefix))
            .map(|((locale, id), _)| (locale.clone(), id.clone()))
            .collect::<Vec<_>>();

        for (locale, id) in entries {
            self.origins.remove(&(locale.clone(), id.clone()));
            let Some(existing) = self.catalogs.get(&locale) else {
                continue;
            };
            let mut remaining = existing.entries().clone();
            remaining.remove(&id);
            if remaining.is_empty() {
                self.catalogs.remove(&locale);
                continue;
            }
            let rebuilt = Catalog::from_entries(locale.clone(), remaining.into_values()).map_err(
                |message| LanguagePackError::Conflict {
                    plugin_id: plugin_id.to_owned(),
                    digest: digest.clone().into_boxed_str(),
                    locale: locale.clone(),
                    message_id: id.clone(),
                    existing_source: "已注册目录".to_owned().into_boxed_str(),
                    reason: message.into_boxed_str(),
                },
            )?;
            self.catalogs.insert(locale, rebuilt);
        }
        self.installed.remove(plugin_id);
        Ok(())
    }

    fn find_signature(&self, id: &str) -> Option<BTreeMap<String, crate::ParamKind>> {
        self.catalogs
            .values()
            .find_map(|catalog| catalog.entries().get(id).map(|entry| entry.params.clone()))
    }
}

/// 语言包激活结果，区分成功、沿用旧目录和完全不可用。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LanguagePackActivation {
    /// 新目录已通过校验并成为当前进程目录。
    Activated {
        /// 已激活的插件标识。
        plugin_id: String,
        /// 已验证的资源摘要。
        digest: String,
    },
    /// 新目录失败，但当前进程继续使用上一次已验证目录。
    RetainedPrevious {
        /// 失败目录携带的插件标识；无法解析清单时为空。
        plugin_id: Option<String>,
        /// 失败目录携带的摘要；无法完成摘要校验时为空。
        digest: Option<String>,
        /// 导致降级的原始错误。
        error: LanguagePackError,
    },
    /// 当前进程没有可保留的已验证插件目录。
    Unavailable {
        /// 导致目录不可用的原始错误。
        error: LanguagePackError,
    },
}

/// 进程内语言包状态；新进程从未验证目录开始，不继承旧进程信任。
#[derive(Clone, Debug)]
pub struct LanguagePackRuntime {
    registry: LanguagePackRegistry,
    last_verified: Option<LanguagePackRegistry>,
    diagnostics: Vec<LanguagePackError>,
}

impl Default for LanguagePackRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguagePackRuntime {
    /// 创建只含内置目录、没有已验证外部目录的新进程状态。
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: LanguagePackRegistry::new(),
            last_verified: None,
            diagnostics: Vec::new(),
        }
    }

    /// 激活一套已加载的语言包；失败时只保留本进程此前验证成功的状态。
    pub fn activate(
        &mut self,
        result: Result<LanguagePack, LanguagePackError>,
    ) -> LanguagePackActivation {
        let pack = match result {
            Ok(pack) => pack,
            Err(error) => return self.failed(error),
        };
        let mut candidate = self.registry.clone();
        if let Err(error) = candidate.register(&pack) {
            return self.failed(error);
        }
        self.registry = candidate.clone();
        self.last_verified = Some(candidate);
        LanguagePackActivation::Activated {
            plugin_id: pack.manifest.plugin_id.clone(),
            digest: pack.content_digest.clone(),
        }
    }

    /// 从目录加载并尝试激活语言包。
    pub fn activate_directory(
        &mut self,
        root: impl AsRef<Path>,
        compatibility: &LanguagePackCompatibility,
    ) -> LanguagePackActivation {
        self.activate(LanguagePack::load_directory(root, compatibility))
    }

    /// 返回当前进程使用的渲染器；失败降级仍保留内置目录。
    pub fn renderer(&self) -> Result<MessageRenderer, LanguagePackError> {
        self.registry.renderer()
    }

    /// 取出并清空本进程记录的加载诊断。
    pub fn take_diagnostics(&mut self) -> Vec<LanguagePackError> {
        std::mem::take(&mut self.diagnostics)
    }

    fn failed(&mut self, error: LanguagePackError) -> LanguagePackActivation {
        self.diagnostics.push(error.clone());
        if let Some(previous) = &self.last_verified {
            self.registry = previous.clone();
            return LanguagePackActivation::RetainedPrevious {
                plugin_id: error.plugin_id().map(str::to_owned),
                digest: error.digest().map(str::to_owned),
                error,
            };
        }
        LanguagePackActivation::Unavailable { error }
    }
}

/// 语言包校验、摘要或合并失败。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LanguagePackError {
    /// 清单字段或 JSON 结构无效。
    Manifest {
        /// 触发错误的清单路径。
        path: PathBuf,
        /// 稳定错误上下文。
        message: String,
    },
    /// 目录资源不是允许的声明式 JSON 文件。
    Resource {
        /// 触发错误的资源路径。
        path: PathBuf,
        /// 稳定错误上下文。
        message: String,
    },
    /// Xiao 或 Runtime ABI 版本不满足清单约束。
    Compatibility {
        /// 插件标识。
        plugin_id: String,
        /// Xiao 约束。
        xiao_requirement: String,
        /// Runtime ABI 约束。
        runtime_requirement: String,
        /// 当前 Xiao 版本。
        current_xiao: String,
        /// 当前 Runtime 版本。
        current_runtime: String,
    },
    /// 资源原始长度或 SHA-256 与清单不一致。
    DigestMismatch {
        /// 插件标识。
        plugin_id: String,
        /// 清单摘要。
        expected: String,
        /// 实际摘要。
        actual: String,
        /// 清单长度。
        content_length: u64,
        /// 实际长度。
        actual_length: u64,
    },
    /// 与已经注册的目录发生冲突。
    Conflict {
        /// 当前插件标识。
        plugin_id: String,
        /// 当前插件摘要。
        digest: Box<str>,
        /// 冲突语言标签。
        locale: String,
        /// 冲突消息身份。
        message_id: String,
        /// 已注册来源。
        existing_source: Box<str>,
        /// 冲突原因。
        reason: Box<str>,
    },
}

impl LanguagePackError {
    /// 返回稳定诊断编号。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Manifest { .. } => LANGUAGE_PACK_MANIFEST_INVALID_CODE,
            Self::Resource { .. } => LANGUAGE_PACK_RESOURCE_INVALID_CODE,
            Self::Compatibility { .. } => LANGUAGE_PACK_COMPATIBILITY_CODE,
            Self::DigestMismatch { .. } => LANGUAGE_PACK_DIGEST_MISMATCH_CODE,
            Self::Conflict { .. } => LANGUAGE_PACK_CONFLICT_CODE,
        }
    }

    fn plugin_id(&self) -> Option<&str> {
        match self {
            Self::Compatibility { plugin_id, .. }
            | Self::DigestMismatch { plugin_id, .. }
            | Self::Conflict { plugin_id, .. } => Some(plugin_id),
            Self::Manifest { .. } | Self::Resource { .. } => None,
        }
    }

    fn digest(&self) -> Option<&str> {
        match self {
            Self::Conflict { digest, .. } => Some(digest),
            Self::Manifest { .. } | Self::Resource { .. } | Self::Compatibility { .. } => None,
            Self::DigestMismatch { expected, .. } => Some(expected),
        }
    }
}

impl Display for LanguagePackError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest { path, message } | Self::Resource { path, message } => {
                write!(
                    formatter,
                    "{}: {}：{}",
                    self.code(),
                    path.display(),
                    message
                )
            }
            Self::Compatibility {
                plugin_id,
                xiao_requirement,
                runtime_requirement,
                current_xiao,
                current_runtime,
            } => write!(
                formatter,
                "{}: 插件 {plugin_id} 不兼容当前 Xiao {current_xiao}/Runtime {current_runtime}（要求 Xiao {xiao_requirement}、Runtime {runtime_requirement}）",
                self.code()
            ),
            Self::DigestMismatch {
                plugin_id,
                expected,
                actual,
                content_length,
                actual_length,
            } => write!(
                formatter,
                "{}: 插件 {plugin_id} 摘要或长度不匹配：期望 {expected}/{content_length}，实际 {actual}/{actual_length}",
                self.code()
            ),
            Self::Conflict {
                plugin_id,
                digest,
                locale,
                message_id,
                existing_source,
                reason,
            } => write!(
                formatter,
                "{}: 插件 {plugin_id}（摘要 {digest}）在 {locale} 的 {message_id} 与 {existing_source} 冲突：{reason}",
                self.code()
            ),
        }
    }
}

impl std::error::Error for LanguagePackError {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogDocument {
    version: u16,
    locale: String,
    entries: Vec<MessageTemplate>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimpleVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Ord for SimpleVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl PartialOrd for SimpleVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_version(text: &str) -> Result<SimpleVersion, ()> {
    let mut parts = text.split('.');
    let version = SimpleVersion {
        major: parts.next().ok_or(())?.parse().map_err(|_| ())?,
        minor: parts.next().ok_or(())?.parse().map_err(|_| ())?,
        patch: parts.next().ok_or(())?.parse().map_err(|_| ())?,
    };
    if parts.next().is_some()
        || text.is_empty()
        || text
            .split('.')
            .any(|part| part.is_empty() || part.len() > 1 && part.starts_with('0'))
    {
        return Err(());
    }
    Ok(version)
}

fn requirement_valid(requirement: &str) -> bool {
    if requirement.trim().is_empty() {
        return false;
    }
    requirement.split(',').all(|term| {
        !term.is_empty()
            && !term.chars().any(char::is_whitespace)
            && term_matches(
                term,
                SimpleVersion {
                    major: 0,
                    minor: 0,
                    patch: 0,
                },
            )
            .is_ok()
    })
}

fn requirement_matches(requirement: &str, current: &str) -> Result<bool, ()> {
    let current = parse_version(current)?;
    requirement
        .split(',')
        .map(|term| term_matches(term, current))
        .collect::<Result<Vec<_>, _>>()
        .map(|matches| matches.into_iter().all(|value| value))
}

fn term_matches(term: &str, current: SimpleVersion) -> Result<bool, ()> {
    if term == "*" {
        return Ok(true);
    }
    let (operator, value) = if let Some(value) = term.strip_prefix(">=") {
        (">=", value)
    } else if let Some(value) = term.strip_prefix("<=") {
        ("<=", value)
    } else if let Some(value) = term.strip_prefix('>') {
        (">", value)
    } else if let Some(value) = term.strip_prefix('<') {
        ("<", value)
    } else if let Some(value) = term.strip_prefix('=') {
        ("=", value)
    } else {
        ("=", term)
    };
    if let Some(value) = value.strip_prefix('^') {
        if operator != "=" {
            return Err(());
        }
        let lower = parse_version(value)?;
        let upper = if lower.major > 0 {
            SimpleVersion {
                major: lower.major + 1,
                minor: 0,
                patch: 0,
            }
        } else if lower.minor > 0 {
            SimpleVersion {
                major: 0,
                minor: lower.minor + 1,
                patch: 0,
            }
        } else {
            SimpleVersion {
                major: 0,
                minor: 0,
                patch: lower.patch + 1,
            }
        };
        return Ok(current >= lower && current < upper);
    }
    if let Some(value) = value.strip_prefix('~') {
        if operator != "=" {
            return Err(());
        }
        let lower = parse_version(value)?;
        let upper = SimpleVersion {
            major: lower.major,
            minor: lower.minor + 1,
            patch: 0,
        };
        return Ok(current >= lower && current < upper);
    }
    if value.contains('*') {
        if operator != "=" {
            return Err(());
        }
        let parts = value.split('.').collect::<Vec<_>>();
        if parts.len() > 3
            || parts
                .iter()
                .enumerate()
                .any(|(index, part)| *part == "*" && index + 1 != parts.len())
        {
            return Err(());
        }
        let major = parts.first().ok_or(())?.parse::<u64>().map_err(|_| ())?;
        if parts.len() == 1 {
            return Ok(current.major == major);
        }
        let minor = parts[1].parse::<u64>().map_err(|_| ())?;
        return Ok(current.major == major && (parts.len() == 2 || current.minor == minor));
    }
    let target = parse_version(value)?;
    Ok(match operator {
        "=" => current == target,
        ">=" => current >= target,
        "<=" => current <= target,
        ">" => current > target,
        "<" => current < target,
        _ => false,
    })
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
}

fn validate_locale(locale: &str) -> Result<(), String> {
    if locale.is_empty()
        || locale.len() > 64
        || locale.chars().any(|character| {
            character.is_control() || !character.is_ascii() || matches!(character, '/' | '\\')
        })
    {
        return Err("语言标签必须是有限 ASCII BCP-47 形状".to_owned());
    }
    Ok(())
}

fn canonical_locale(locale: &str) -> Result<String, String> {
    validate_locale(locale)?;
    Ok(LocaleContext::new(locale).tag().to_owned())
}

fn resource_path(path: &str) -> Result<String, String> {
    if path.is_empty() || path.contains('\\') {
        return Err("资源路径必须是非空相对路径".to_owned());
    }
    let mut segments = Vec::new();
    for component in Path::new(path).components() {
        let Component::Normal(segment) = component else {
            return Err("资源路径不能包含根、当前目录或父目录".to_owned());
        };
        let segment = segment
            .to_str()
            .ok_or_else(|| "资源路径必须是 UTF-8".to_owned())?;
        if segment.is_empty() || segment.chars().any(char::is_control) {
            return Err("资源路径片段无效".to_owned());
        }
        segments.push(segment.to_owned());
    }
    Ok(segments.join("/"))
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, LanguagePackError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| resource_error(path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LanguagePackError::Resource {
            path: path.to_path_buf(),
            message: "资源必须是普通文件且不能是符号链接".to_owned(),
        });
    }
    if metadata.len() > maximum {
        return Err(LanguagePackError::Resource {
            path: path.to_path_buf(),
            message: format!("资源超过 {} 字节上限", maximum),
        });
    }
    fs::read(path).map_err(|error| resource_error(path, error))
}

fn collect_files(root: &Path) -> Result<BTreeSet<String>, LanguagePackError> {
    let mut files = BTreeSet::new();
    collect_files_inner(root, Path::new(""), &mut files)?;
    Ok(files)
}

fn collect_files_inner(
    directory: &Path,
    relative_directory: &Path,
    files: &mut BTreeSet<String>,
) -> Result<(), LanguagePackError> {
    let entries = fs::read_dir(directory).map_err(|error| resource_error(directory, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| resource_error(directory, error))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| resource_error(&path, error))?;
        if metadata.file_type().is_symlink() {
            return Err(LanguagePackError::Resource {
                path,
                message: "语言包不能包含符号链接".to_owned(),
            });
        }
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| LanguagePackError::Resource {
                path: path.clone(),
                message: "资源路径必须是 UTF-8".to_owned(),
            })?
            .to_owned();
        let relative = if relative_directory.as_os_str().is_empty() {
            name
        } else {
            format!(
                "{}/{}",
                relative_directory.to_string_lossy().replace('\\', "/"),
                name
            )
        };
        if metadata.is_dir() {
            collect_files_inner(&path, Path::new(&relative), files)?;
        } else if metadata.is_file() {
            files.insert(relative);
        } else {
            return Err(LanguagePackError::Resource {
                path,
                message: "语言包只支持普通文件和目录".to_owned(),
            });
        }
    }
    Ok(())
}

/// 按排序路径和长度前缀计算语言资源摘要。
#[must_use]
pub fn digest_resources(resources: &BTreeMap<String, Vec<u8>>) -> (u64, String) {
    let mut hasher = Sha256::new();
    hasher.update(b"xiao-language-pack-v1\0");
    let mut length = 0_u64;
    for (path, bytes) in resources {
        append_length(&mut hasher, path.as_bytes());
        append_length(&mut hasher, bytes);
        length = length.saturating_add(bytes.len() as u64);
    }
    (length, format!("{:x}", hasher.finalize()))
}

fn append_length(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn valid_digest(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn resource_error(path: &Path, error: std::io::Error) -> LanguagePackError {
    LanguagePackError::Resource {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}
