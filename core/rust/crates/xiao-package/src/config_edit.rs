//! 基于已解析配置区间的保留格式依赖编辑。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use xiao_config::{ConfigDocument, DependencyKind, parse_config_project};
use xiao_source::SourceFile;

use crate::diagnostics::SYNC_INVALID_INPUT_CODE;
use crate::federation::valid_package_name;
use crate::lockfile::atomic_write_file;
use crate::sync::PackageSyncError;
use crate::version::VersionRequirement;

/// 一次只编辑指定依赖表中的一个条目；其余文本逐字保留。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyEdit {
    /// 增加尚不存在的依赖，不覆盖已有条目。
    Add {
        /// 规范包名。
        name: String,
        /// 依赖分类。
        kind: DependencyKind,
        /// 新条目的静态字符串字段。
        fields: BTreeMap<String, String>,
    },
    /// 删除存在的依赖，不改动另一个依赖表。
    Remove {
        /// 规范包名。
        name: String,
        /// 依赖分类。
        kind: DependencyKind,
    },
}

fn invalid(message: &str) -> PackageSyncError {
    PackageSyncError {
        code: SYNC_INVALID_INPUT_CODE.to_owned(),
        message: message.to_owned(),
    }
}

/// 消费配置解析器提供的源码区间；不对原文做二次词法分析。
pub fn edit_dependency(
    document: &ConfigDocument,
    original: &str,
    edit: &DependencyEdit,
) -> Result<String, PackageSyncError> {
    if document.span.start() != 0 || document.span.end() != original.len() {
        return Err(invalid("配置原文与解析文档不一致"));
    }
    let (name, kind) = match edit {
        DependencyEdit::Add { name, kind, .. } | DependencyEdit::Remove { name, kind } => {
            (name, kind)
        }
    };
    if !valid_package_name(name) {
        return Err(invalid("依赖名称不是合法包名"));
    }
    let table_name = kind.table_name();
    let table = document.table(table_name);
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let result = match edit {
        DependencyEdit::Add { fields, .. } => {
            if table.is_some_and(|table| table.get(name).is_some()) {
                return Err(invalid("依赖已存在；请先删除旧条目"));
            }
            if let Some(version) = fields.get("version") {
                VersionRequirement::parse(version).map_err(|_| invalid("版本约束不合法"))?;
            }
            if fields.is_empty() {
                return Err(invalid("依赖缺少声明字段"));
            }
            let values = fields
                .iter()
                .map(|(key, value)| {
                    format!(
                        "{key} = {}",
                        serde_json::to_string(value).expect("字符串可序列化")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let entry = format!(
                "{} = {{ {values} }}{newline}",
                serde_json::to_string(name).expect("包名可序列化")
            );
            if let Some(table) = table {
                let end = table
                    .entries
                    .values()
                    .map(|entry| entry.span.end())
                    .max()
                    .unwrap_or(table.span.end());
                let rest = original
                    .get(end..)
                    .ok_or_else(|| invalid("配置源码区间不合法"))?;
                let position = rest
                    .find('\n')
                    .map_or(original.len(), |offset| end + offset + 1);
                let prefix = if position == original.len() && !original.ends_with('\n') {
                    newline
                } else {
                    ""
                };
                format!(
                    "{}{prefix}{entry}{}",
                    &original[..position],
                    &original[position..]
                )
            } else {
                let separator =
                    if original.is_empty() || original.ends_with(&format!("{newline}{newline}")) {
                        ""
                    } else if original.ends_with('\n') {
                        newline
                    } else {
                        "\n\n"
                    };
                format!("{original}{separator}[{table_name}]{newline}{entry}")
            }
        }
        DependencyEdit::Remove { .. } => {
            let entry = table
                .and_then(|table| table.get(name))
                .ok_or_else(|| invalid("依赖不存在"))?;
            let start = entry.span.start();
            let end = entry.span.end();
            let line_start = original[..start].rfind('\n').map_or(0, |offset| offset + 1);
            let line_end = original[end..]
                .find('\n')
                .map_or(original.len(), |offset| end + offset);
            if !original[line_start..start].trim().is_empty() {
                return Err(invalid("依赖源码区间不是独立行"));
            }
            let suffix = original[end..line_end].trim_start();
            if suffix.starts_with('#') {
                format!(
                    "{}{}{}",
                    &original[..line_start],
                    &original[line_start..start],
                    &original[end..]
                )
            } else if suffix.is_empty() {
                let after = if line_end < original.len() {
                    line_end + 1
                } else {
                    line_end
                };
                format!("{}{}", &original[..line_start], &original[after..])
            } else {
                return Err(invalid("依赖行尾包含不可保留的内容"));
            }
        }
    };
    parse_config_project(&SourceFile::from_text(&result))
        .map_err(|_| invalid("编辑后配置未通过静态校验"))?;
    Ok(result)
}

/// 检查文件未被其他进程改写后，以同目录暂存文件原子提交编辑。
pub fn write_config_edit(
    path: &Path,
    original: &str,
    edited: &str,
) -> Result<(), PackageSyncError> {
    let current = fs::read_to_string(path).map_err(|_| invalid("读取 config.xiao 失败"))?;
    if current != original {
        return Err(invalid("config.xiao 已被修改，请重新执行包操作"));
    }
    atomic_write_file(path, edited.as_bytes()).map_err(|_| invalid("原子写入 config.xiao 失败"))
}
