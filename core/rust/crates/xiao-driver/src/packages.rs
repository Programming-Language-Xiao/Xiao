//! 只读环境包映射到静态命名空间与运行时源码身份的转换。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use xiao_modules::{ModuleKind, ModuleName, analyze_project};
use xiao_package::{
    CacheLayout, ENVIRONMENT_METADATA_FILE, environment_package_view, read_environment_metadata,
};
use xiao_syntax::KeywordKind;
use xiao_types::ExternalNamespaces;

/// 一份包内文件模块的来源和静态公开符号。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageModule {
    /// 包根。
    pub package: String,
    /// 来源环境的绝对路径。
    pub environment: PathBuf,
    /// 文件模块的完整路径。
    pub source: PathBuf,
    /// 所在包源码对象根目录。
    pub project_root: PathBuf,
    /// 静态发现的导出名称。
    pub exports: BTreeSet<String>,
}

/// 与只读查询共用映射来源、但不持有任何已初始化状态的包表。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageRegistry {
    /// 编译期可见的命名空间及成员。
    pub namespaces: ExternalNamespaces,
    /// 运行期按需读取的具体文件模块，以完整包路径为键。
    pub modules: BTreeMap<String, PackageModule>,
    /// 本次视图实际读取的环境路径。
    pub environment_path: PathBuf,
    /// E0 已有的环境汇总指纹；环境不存在时为空。
    pub environment_fingerprint: Option<String>,
    /// 视图中的完整包身份和源码对象摘要，避免只比较环境路径。
    pub package_fingerprints: Vec<PackageFingerprint>,
}

/// 环境包视图中的稳定包内容摘要。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageFingerprint {
    /// 包名。
    pub name: String,
    /// 包版本。
    pub version: String,
    /// 包来源身份。
    pub source_id: String,
    /// 包源码对象摘要。
    pub object_digest: String,
}

/// 用于判断同一核心会话是否仍可复用模块状态的环境指纹。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageRegistryFingerprint {
    /// 当前生效环境路径。
    pub environment_path: PathBuf,
    /// E0 汇总指纹。
    pub environment_fingerprint: Option<String>,
    /// 包视图内容摘要。
    pub packages: Vec<PackageFingerprint>,
}

impl PackageRegistry {
    /// 从已激活的环境或全局环境获取只读视图，绝不运行包模块。
    pub fn from_environment(active: Option<&Path>) -> Result<Self, String> {
        let layout = CacheLayout::from_environment().map_err(|error| error.to_string())?;
        let environment = match active {
            Some(path) if path.is_absolute() => path.to_path_buf(),
            Some(path) => return Err(format!("来源环境 {} 不是绝对路径", path.display())),
            None => layout
                .global_environment_path("global")
                .map_err(|error| error.to_string())?,
        };
        Self::from_layout(&layout, &environment, active.is_none())
    }

    /// 在隔离缓存中建立视图，供运行测试复用。
    pub fn from_layout(
        layout: &CacheLayout,
        environment: &Path,
        allow_missing: bool,
    ) -> Result<Self, String> {
        if allow_missing && !environment.exists() {
            return Ok(Self {
                environment_path: environment.to_path_buf(),
                ..Self::default()
            });
        }
        let mappings = environment_package_view(environment).map_err(|error| error.to_string())?;
        let environment_fingerprint =
            read_environment_metadata(environment.join(ENVIRONMENT_METADATA_FILE))
                .map_err(|error| error.to_string())?
                .environment_fingerprint;
        let package_fingerprints: Vec<PackageFingerprint> = mappings
            .iter()
            .map(|mapping| PackageFingerprint {
                name: mapping.package.name.clone(),
                version: mapping.package.version.clone(),
                source_id: mapping.package.source.source_id.clone(),
                object_digest: mapping.object.digest.clone(),
            })
            .collect();
        let mut package_fingerprints = package_fingerprints;
        package_fingerprints.sort_by(|left, right| {
            (
                &left.name,
                &left.version,
                &left.source_id,
                &left.object_digest,
            )
                .cmp(&(
                    &right.name,
                    &right.version,
                    &right.source_id,
                    &right.object_digest,
                ))
        });
        let mut result = Self {
            environment_path: environment.to_path_buf(),
            environment_fingerprint: Some(environment_fingerprint),
            package_fingerprints,
            ..Self::default()
        };
        let mut seen = BTreeSet::new();
        for mapping in mappings {
            let root = &mapping.package.name;
            if !valid_package_root(root) {
                continue;
            }
            if !seen.insert(root.clone()) {
                return Err(format!(
                    "来源环境 {} 中包根 {root} 冲突，须显式导入",
                    environment.display()
                ));
            }
            result.namespaces.members.entry(root.clone()).or_default();
            let object_path = layout
                .source_object_path(&mapping.object.digest)
                .map_err(|error| error.to_string())?;
            let project = analyze_project(&object_path);
            for name in project.namespaces.keys().chain(project.modules.keys()) {
                let full_path = format!("{root}.{name}");
                result.namespaces.members.entry(full_path).or_default();
                let parent = name.parent().unwrap_or_else(ModuleName::root);
                let parent_path = if parent.is_root() {
                    root.clone()
                } else {
                    format!("{root}.{parent}")
                };
                if let Some(segment) = name.segments().last() {
                    result
                        .namespaces
                        .members
                        .entry(parent_path)
                        .or_default()
                        .insert(segment.clone());
                }
            }
            for (name, record) in &project.modules {
                if record.kind != ModuleKind::File {
                    continue;
                }
                let full_path = format!("{root}.{name}");
                let exports = record
                    .symbols
                    .values()
                    .map(|symbol| symbol.name.clone())
                    .collect();
                result.namespaces.members.insert(full_path.clone(), exports);
                result.modules.insert(
                    full_path,
                    PackageModule {
                        package: root.clone(),
                        environment: environment.to_path_buf(),
                        source: record.path.clone(),
                        project_root: object_path.clone(),
                        exports: record
                            .symbols
                            .values()
                            .map(|symbol| symbol.name.clone())
                            .collect(),
                    },
                );
            }
        }
        Ok(result)
    }

    /// 返回环境路径、E0 指纹与包视图内容组成的会话判断键。
    #[must_use]
    pub fn session_fingerprint(&self) -> PackageRegistryFingerprint {
        PackageRegistryFingerprint {
            environment_path: self.environment_path.clone(),
            environment_fingerprint: self.environment_fingerprint.clone(),
            packages: self.package_fingerprints.clone(),
        }
    }
}

pub(crate) fn valid_package_root(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
        && KeywordKind::from_word(segment).is_none()
}
