//! 11A-D1 的本地路径依赖声明模型。
//!
//! 本模块只从已经通过配置校验的 [`ConfigDocument`] 提取静态依赖声明；它不访问文件系统、
//! 不解析版本约束，也不解析包源协议。后续包管理阶段可以消费这些声明而不重新扫描文本。

use xiao_source::SourceSpan;

use crate::model::{ConfigDocument, ConfigValue};

/// 不访问网络的单仓库 Git 依赖声明。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitDependencyDeclaration {
    /// 配置包名。
    pub name: String,
    /// 声明类别。
    pub kind: DependencyKind,
    /// HTTPS 仓库地址。
    pub git: String,
    /// 引用类型：rev、tag 或 branch。
    pub reference_kind: String,
    /// 公开的引用文本，不作为完整性证明。
    pub reference: String,
    /// 可选版本条件（此阶段不求解）。
    pub version: Option<String>,
    /// 声明源码范围。
    pub span: SourceSpan,
}

/// 从已校验的静态树提取显式 Git 依赖，而不解析其引用。
#[must_use]
pub fn git_dependency_declarations(document: &ConfigDocument) -> Vec<GitDependencyDeclaration> {
    let mut declarations = Vec::new();
    for (table_name, kind) in [
        ("dependencies", DependencyKind::Runtime),
        ("devdependencies", DependencyKind::Development),
    ] {
        let Some(table) = document.table(table_name) else {
            continue;
        };
        for (name, entry) in table.iter() {
            let Some(fields) = entry.value.as_dictionary() else {
                continue;
            };
            let Some(git) = fields.get("git").and_then(ConfigValue::as_str) else {
                continue;
            };
            let Some((reference_kind, reference)) =
                ["rev", "tag", "branch"].into_iter().find_map(|field| {
                    fields
                        .get(field)
                        .and_then(ConfigValue::as_str)
                        .map(|value| (field, value))
                })
            else {
                continue;
            };
            declarations.push(GitDependencyDeclaration {
                name: name.clone(),
                kind,
                git: git.to_owned(),
                reference_kind: reference_kind.to_owned(),
                reference: reference.to_owned(),
                version: fields
                    .get("version")
                    .and_then(ConfigValue::as_str)
                    .map(str::to_owned),
                span: entry.span,
            });
        }
    }
    declarations
}

/// 依赖声明所属的配置表。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DependencyKind {
    /// `[dependencies]` 中的运行时依赖。
    Runtime,
    /// `[devdependencies]` 中的开发依赖。
    Development,
}

impl DependencyKind {
    /// 返回对应的规范化配置表名。
    #[must_use]
    pub const fn table_name(self) -> &'static str {
        match self {
            Self::Runtime => "dependencies",
            Self::Development => "devdependencies",
        }
    }
}

/// 一条已经通过静态配置校验的本地路径依赖声明。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyDeclaration {
    /// 依赖表中的包名；D1 要求它与被引用包的 `[project].name` 一致。
    pub name: String,
    /// 依赖所属的配置表。
    pub kind: DependencyKind,
    /// 相对于声明包根目录的本地路径。
    pub path: String,
    /// 可选的版本约束占位文本；D1 只保存，不执行版本求解。
    pub version: Option<String>,
    /// 可选的来源引用；D1 只保存 alias/source_id 形状，不实现源协议。
    pub source: Option<String>,
    /// 覆盖整个依赖条目的源码区间。
    pub span: SourceSpan,
}

/// 从规范化配置树中按稳定顺序提取全部依赖声明。
///
/// 调用者应传入 [`crate::parse_config`] 或 [`crate::parse_config_project`] 返回的文档。
/// 对于未通过校验的手工构造树，形状不完整的条目会被跳过，而不会触发文件访问或执行。
#[must_use]
pub fn dependency_declarations(document: &ConfigDocument) -> Vec<DependencyDeclaration> {
    let mut declarations = Vec::new();
    for (table_name, kind) in [
        ("dependencies", DependencyKind::Runtime),
        ("devdependencies", DependencyKind::Development),
    ] {
        let Some(table) = document.table(table_name) else {
            continue;
        };
        for (name, entry) in table.iter() {
            let Some(fields) = entry.value.as_dictionary() else {
                continue;
            };
            let Some(path) = fields.get("path").and_then(ConfigValue::as_str) else {
                continue;
            };
            declarations.push(DependencyDeclaration {
                name: name.clone(),
                kind,
                path: path.to_owned(),
                version: fields
                    .get("version")
                    .and_then(ConfigValue::as_str)
                    .map(str::to_owned),
                source: fields
                    .get("source")
                    .and_then(ConfigValue::as_str)
                    .map(str::to_owned),
                span: entry.span,
            });
        }
    }
    declarations
}
