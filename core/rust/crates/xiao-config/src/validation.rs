//! 配置模式、保留表和项目相对路径校验。
//!
//! 该模块只检查已经由 [`crate::parser`] 建立的字面量树，不进行求值、文件访问、
//! 依赖解析或模块扫描。后续包管理器可以在不改变此边界的前提下消费规范化结果。

use std::collections::BTreeSet;

use xiao_diagnostics::{Diagnostic, DiagnosticParam};
use xiao_source::SourceSpan;

use crate::diagnostics::*;
use crate::model::{ConfigDocument, ConfigTable, ConfigValue, NormalizedConfig};

/// 当前已登记的顶层配置表。
///
/// 扩展表只保存静态值，专属字段语义由后续工程期实现；将它们列入保留表可以
/// 让全局/调试/包管理配置先安全通过同一解析边界。
const RESERVED_TABLES: &[&str] = &[
    "artifacts",
    "build",
    "cli",
    "debug",
    "dependencies",
    "devdependencies",
    "exports",
    "language",
    "optimization",
    "package",
    "project",
    "runtime",
    "resources",
    "sources",
    "toolchain",
    "vm",
];

/// 校验通用配置并返回可供后续阶段消费的规范化副本。
pub fn validate_config(document: &ConfigDocument) -> Result<NormalizedConfig, ConfigDiagnostics> {
    validate(document, false)
}

/// 校验项目配置并强制要求 `[project].name/version`。
pub fn validate_project_config(
    document: &ConfigDocument,
) -> Result<NormalizedConfig, ConfigDiagnostics> {
    validate(document, true)
}

/// 执行表白名单和字段模式检查。
fn validate(
    document: &ConfigDocument,
    require_project: bool,
) -> Result<NormalizedConfig, ConfigDiagnostics> {
    let mut diagnostics = Vec::new();
    let mut ordered_tables = document.tables.values().collect::<Vec<_>>();
    ordered_tables.sort_by_key(|table| table.span.start());
    for table in ordered_tables {
        let name = &table.name;
        if !RESERVED_TABLES.contains(&name.as_str()) {
            diagnostics.push(error(
                UNKNOWN_TABLE_CODE,
                "x05.config.unknown_table",
                table.span,
                format!("未知配置表 {name:?}"),
                [("table", DiagnosticParam::Text(name.clone()))],
            ));
            continue;
        }
        match name.as_str() {
            "project" => validate_project_table(table, &mut diagnostics),
            "exports" => validate_exports_table(table, &mut diagnostics),
            "language" => validate_language_table(table, &mut diagnostics),
            "optimization" => validate_optimization_table(table, &mut diagnostics),
            "dependencies" | "devdependencies" => {
                validate_dependency_table(table, &mut diagnostics)
            }
            "sources" => validate_source_table(table, &mut diagnostics),
            "resources" => validate_resources_table(table, &mut diagnostics),
            // CLI、Debug、VM、工具链和构建相关表在本阶段只保留静态值。
            _ => {}
        }
    }

    if require_project && !document.tables.contains_key("project") {
        diagnostics.push(error(
            MISSING_REQUIRED_CODE,
            "x05.config.missing_project_table",
            document.span,
            "项目配置必须声明 [project] 表".to_owned(),
            std::iter::empty::<(&str, DiagnosticParam)>(),
        ));
    }

    if diagnostics.is_empty() {
        Ok(normalize(document))
    } else {
        Err(diagnostics)
    }
}

/// 检查显式资源映射。键是归档内逻辑路径，值是项目根相对来源文件。
///
/// 资源表故意不接受目录、数组或字典值：每一项都必须对应一次明确的文件读取，
/// 这样收集器无需遍历项目目录，也不会把环境或凭据作为隐式输入带入归档。
fn validate_resources_table(table: &ConfigTable, diagnostics: &mut ConfigDiagnostics) {
    for (logical_path, entry) in &table.entries {
        if !is_safe_resource_path(logical_path) {
            diagnostics.push(error(
                INVALID_RESOURCE_LOGICAL_PATH_CODE,
                "x05.config.invalid_resource_logical_path",
                entry.span,
                format!("资源逻辑路径 {logical_path:?} 必须是项目根相对规范路径"),
                [("path", DiagnosticParam::Text(logical_path.clone()))],
            ));
        }
        let ConfigValue::String(source_path) = &entry.value else {
            diagnostics.push(type_error(entry.span, logical_path, "str", &entry.value));
            continue;
        };
        if !is_safe_resource_path(source_path) {
            diagnostics.push(error(
                INVALID_RESOURCE_SOURCE_PATH_CODE,
                "x05.config.invalid_resource_source_path",
                entry.span,
                format!("资源 {logical_path:?} 的来源路径必须位于项目根内"),
                [
                    ("logical_path", DiagnosticParam::Text(logical_path.clone())),
                    ("source_path", DiagnosticParam::Text(source_path.clone())),
                ],
            ));
        }
    }
}

/// 对多源具名条目分别校验直接源与列表导入的字段白名单。
fn validate_source_table(table: &ConfigTable, diagnostics: &mut ConfigDiagnostics) {
    let mut entries = table.entries.values().collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.span.start());
    for entry in entries {
        let ConfigValue::Dictionary(fields) = &entry.value else {
            diagnostics.push(type_error(entry.span, &entry.key, "dict", &entry.value));
            continue;
        };
        let imported = fields.contains_key("list");
        let allowed: &[&str] = if imported {
            &["list", "digest"]
        } else {
            &[
                "kind", "location", "alias", "display", "protocol", "rev", "tag", "branch",
            ]
        };
        for (field, value) in fields {
            if !allowed.contains(&field.as_str()) {
                diagnostics.push(error(
                    UNKNOWN_FIELD_CODE,
                    "x05.config.unknown_source_field",
                    entry.span,
                    format!("源 {:?} 中未知字段 {field:?}", entry.key),
                    [("field", DiagnosticParam::Text(field.clone()))],
                ));
                continue;
            }
            let valid = if field == "protocol" {
                matches!(value, ConfigValue::Integer(version) if *version > 0)
            } else {
                matches!(value, ConfigValue::String(text) if !text.trim().is_empty() && !text.chars().any(char::is_control))
            };
            if !valid {
                diagnostics.push(type_error(
                    entry.span,
                    field,
                    if field == "protocol" {
                        "positive int"
                    } else {
                        "nonempty str"
                    },
                    value,
                ));
            }
        }
        for required in if imported {
            &["list", "digest"][..]
        } else {
            &["kind", "location"][..]
        } {
            if !fields.contains_key(*required) {
                diagnostics.push(error(
                    MISSING_REQUIRED_CODE,
                    "x05.config.missing_source_field",
                    entry.span,
                    format!("源 {:?} 缺少必填字段 {required:?}", entry.key),
                    [("field", DiagnosticParam::Text((*required).to_owned()))],
                ));
            }
        }
        if !imported
            && ["rev", "tag", "branch"]
                .iter()
                .any(|field| fields.contains_key(*field))
        {
            let count = ["rev", "tag", "branch"]
                .iter()
                .filter(|field| fields.contains_key(**field))
                .count();
            if count != 1
                || !matches!(fields.get("kind"), Some(ConfigValue::String(kind)) if kind == "git-index")
                || ["rev", "tag", "branch"].iter().any(|field| {
                    fields
                        .get(*field)
                        .and_then(ConfigValue::as_str)
                        .is_some_and(|value| !valid_git_reference(value))
                })
            {
                diagnostics.push(error(
                    INVALID_DEPENDENCY_GIT_CODE,
                    "x05.config.invalid_source_git_ref",
                    entry.span,
                    format!("源 {:?} 的 Git 引用仅能在 git-index 中指定一个", entry.key),
                    [("field", DiagnosticParam::Text("rev/tag/branch".to_owned()))],
                ));
            }
        }
    }
}

/// 检查项目身份字段和严格字段白名单。
fn validate_project_table(table: &ConfigTable, diagnostics: &mut ConfigDiagnostics) {
    check_known_fields(table, &["name", "version"], diagnostics);
    for required in ["name", "version"] {
        let Some(entry) = table.entries.get(required) else {
            diagnostics.push(error(
                MISSING_REQUIRED_CODE,
                "x05.config.missing_project_field",
                table.span,
                format!("[project] 缺少必填字段 {required:?}"),
                [("field", DiagnosticParam::Text(required.to_owned()))],
            ));
            continue;
        };
        if !matches!(&entry.value, ConfigValue::String(value) if !value.trim().is_empty()) {
            diagnostics.push(type_error(entry.span, required, "str", &entry.value));
        }
    }
}

/// 检查包外导出键到模块路径的映射。
fn validate_exports_table(table: &ConfigTable, diagnostics: &mut ConfigDiagnostics) {
    for (name, entry) in &table.entries {
        let ConfigValue::String(path) = &entry.value else {
            diagnostics.push(type_error(entry.span, name, "str", &entry.value));
            continue;
        };
        if !is_relative_xiao_path(path) {
            diagnostics.push(error(
                INVALID_EXPORT_PATH_CODE,
                "x05.config.invalid_export_path",
                entry.span,
                format!("导出 {name:?} 的路径必须是项目根相对 .xiao 文件"),
                [("path", DiagnosticParam::Text(path.clone()))],
            ));
        }
    }
}

/// 检查国际化表的首版字段；具体语言目录仍由 11C 负责。
fn validate_language_table(table: &ConfigTable, diagnostics: &mut ConfigDiagnostics) {
    check_known_fields(table, &["locale"], diagnostics);
    if let Some(entry) = table.entries.get("locale") {
        if !matches!(&entry.value, ConfigValue::String(value) if !value.trim().is_empty()) {
            diagnostics.push(type_error(entry.span, "locale", "str", &entry.value));
        }
    }
}

/// 检查 13A 优化配置允许的静态字段。
///
/// 这里仅建立声明式配置边界，不创建优化器对象，也不执行任何 Pass；级别和列表
/// 的规范化由 CLI 归一化层消费同一组字段完成。
fn validate_optimization_table(table: &ConfigTable, diagnostics: &mut ConfigDiagnostics) {
    check_known_fields(
        table,
        &[
            "level",
            "pass_set",
            "debug_info",
            "source_map",
            "diagnostic_events",
            "allow_cpu_specialization",
            "allow_lto",
            "experimental_passes",
        ],
        diagnostics,
    );
    if let Some(entry) = table.entries.get("level") {
        if !matches!(&entry.value, ConfigValue::Integer(level) if (0..=3).contains(level)) {
            diagnostics.push(error(
                CONFIG_INVALID_VALUE_CODE,
                "x05.config.invalid_optimization_level",
                entry.span,
                "优化级别必须是 0、1、2 或 3".to_owned(),
                [("field", DiagnosticParam::Text("level".to_owned()))],
            ));
        }
    }
    for field in [
        "debug_info",
        "source_map",
        "diagnostic_events",
        "allow_cpu_specialization",
        "allow_lto",
    ] {
        if let Some(entry) = table.entries.get(field)
            && !matches!(entry.value, ConfigValue::Boolean(_))
        {
            diagnostics.push(type_error(entry.span, field, "bool", &entry.value));
        }
    }
    for field in ["pass_set", "experimental_passes"] {
        if let Some(entry) = table.entries.get(field)
            && !matches!(&entry.value, ConfigValue::Array(values) if values.iter().all(|value| matches!(value, ConfigValue::String(text) if !text.trim().is_empty() && !text.chars().any(char::is_control))))
        {
            diagnostics.push(type_error(entry.span, field, "array[str]", &entry.value));
        }
    }
}

/// 检查本地路径或显式 Git 引用的声明式依赖条目。
fn validate_dependency_table(table: &ConfigTable, diagnostics: &mut ConfigDiagnostics) {
    for (name, entry) in &table.entries {
        let Some(fields) = entry.value.as_dictionary() else {
            diagnostics.push(type_error(entry.span, name, "dict", &entry.value));
            continue;
        };

        for (field, value) in fields {
            if !matches!(
                field.as_str(),
                "path" | "version" | "source" | "git" | "rev" | "tag" | "branch"
            ) {
                diagnostics.push(error(
                    UNKNOWN_FIELD_CODE,
                    "x05.config.unknown_dependency_field",
                    entry.span,
                    format!("依赖 {name:?} 中未知字段 {field:?}"),
                    [
                        ("table", DiagnosticParam::Text(table.name.clone())),
                        ("package", DiagnosticParam::Text(name.clone())),
                        ("field", DiagnosticParam::Text(field.clone())),
                    ],
                ));
            }
            if (field == "version" || field == "source")
                && !matches!(value, ConfigValue::String(text) if !text.trim().is_empty() && !text.chars().any(char::is_control))
            {
                let code = if field == "version" {
                    INVALID_DEPENDENCY_CONSTRAINT_CODE
                } else {
                    INVALID_DEPENDENCY_SOURCE_CODE
                };
                let message_id = if field == "version" {
                    "x05.config.invalid_dependency_constraint"
                } else {
                    "x05.config.invalid_dependency_source"
                };
                diagnostics.push(error(
                    code,
                    message_id,
                    entry.span,
                    format!("依赖 {name:?} 的 {field:?} 必须是非空静态字符串"),
                    [
                        ("package", DiagnosticParam::Text(name.clone())),
                        ("field", DiagnosticParam::Text(field.clone())),
                    ],
                ));
            }
            if matches!(field.as_str(), "git" | "rev" | "tag" | "branch")
                && !matches!(value, ConfigValue::String(text) if !text.trim().is_empty() && !text.chars().any(char::is_control))
            {
                diagnostics.push(type_error(entry.span, field, "nonempty str", value));
            }
        }

        let has_git = fields.contains_key("git");
        let references = ["rev", "tag", "branch"]
            .iter()
            .filter(|field| fields.contains_key(**field))
            .count();
        if has_git || references != 0 {
            let invalid = fields.contains_key("path")
                || fields.contains_key("source")
                || !has_git
                || references != 1
                || fields
                    .get("git")
                    .and_then(ConfigValue::as_str)
                    .is_none_or(|url| !valid_git_url(url))
                || ["rev", "tag", "branch"].iter().any(|field| {
                    fields
                        .get(*field)
                        .and_then(ConfigValue::as_str)
                        .is_some_and(|reference| !valid_git_reference(reference))
                });
            if invalid {
                diagnostics.push(error(
                    INVALID_DEPENDENCY_GIT_CODE,
                    "x05.config.invalid_dependency_git",
                    entry.span,
                    format!("依赖 {name:?} 须指定安全的 https git 地址和唯一的 rev/tag/branch，且不能同时声明 path/source"),
                    [("package", DiagnosticParam::Text(name.clone()))],
                ));
            }
            continue;
        }
        let Some(path) = fields.get("path") else {
            if fields
                .get("version")
                .and_then(ConfigValue::as_str)
                .is_some()
            {
                continue;
            }
            diagnostics.push(error(
                MISSING_REQUIRED_CODE,
                "x05.config.missing_dependency_path",
                entry.span,
                format!("依赖 {name:?} 必须声明 path"),
                [
                    ("package", DiagnosticParam::Text(name.clone())),
                    ("field", DiagnosticParam::Text("path".to_owned())),
                ],
            ));
            continue;
        };
        let ConfigValue::String(path) = path else {
            diagnostics.push(type_error(entry.span, "path", "str", path));
            continue;
        };
        if !is_relative_dependency_path(path) {
            diagnostics.push(error(
                INVALID_DEPENDENCY_PATH_CODE,
                "x05.config.invalid_dependency_path",
                entry.span,
                format!("依赖 {name:?} 的 path 必须是项目根相对路径"),
                [
                    ("package", DiagnosticParam::Text(name.clone())),
                    ("path", DiagnosticParam::Text(path.clone())),
                ],
            ));
        }
    }
}

fn valid_git_url(url: &str) -> bool {
    let Some(remainder) = url.strip_prefix("https://") else {
        return false;
    };
    let Some((authority, path)) = remainder.split_once('/') else {
        return false;
    };
    !authority.is_empty()
        && authority.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b':' | b'[' | b']')
        })
        && !path.is_empty()
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && !matches!(segment, "." | "..")
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
}

fn valid_git_reference(reference: &str) -> bool {
    !reference.is_empty()
        && !reference.contains("..")
        && reference.split('/').all(|segment| {
            !segment.is_empty()
                && !segment.starts_with('.')
                && !segment.ends_with('.')
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
}

/// 检查严格表中的未知字段。
fn check_known_fields(table: &ConfigTable, known: &[&str], diagnostics: &mut ConfigDiagnostics) {
    let known = known.iter().copied().collect::<BTreeSet<_>>();
    for (name, entry) in &table.entries {
        if !known.contains(name.as_str()) {
            diagnostics.push(error(
                UNKNOWN_FIELD_CODE,
                "x05.config.unknown_field",
                entry.span,
                format!("表 {:?} 中未知字段 {:?}", table.name, name),
                [
                    ("table", DiagnosticParam::Text(table.name.clone())),
                    ("field", DiagnosticParam::Text(name.clone())),
                ],
            ));
        }
    }
}

/// 判断本地依赖路径是否为非空、非绝对的静态路径。
fn is_relative_dependency_path(path: &str) -> bool {
    if path.trim().is_empty()
        || path.chars().any(char::is_control)
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains(':')
    {
        return false;
    }
    true
}

/// 判断资源逻辑路径或来源路径是否为安全的项目根相对文件路径。
///
/// 这里拒绝反斜杠而不是先替换它，避免在不同宿主平台上把 `a\\..\\b` 解释成
/// 与当前平台相关的另一条路径。空段、`.`、`..` 和卷标也全部拒绝，规范化由
/// 声明者直接提供，收集器随后只执行该单个文件的读取。
fn is_safe_resource_path(path: &str) -> bool {
    if path.is_empty()
        || path.chars().any(char::is_control)
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains('\\')
        || path.as_bytes().get(1) == Some(&b':')
    {
        return false;
    }
    path.split('/')
        .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

/// 构造类型错误诊断。
fn type_error(
    span: SourceSpan,
    field: &str,
    expected: &str,
    value: &ConfigValue,
) -> ConfigDiagnostic {
    error(
        CONFIG_TYPE_MISMATCH_CODE,
        "x05.config.type_mismatch",
        span,
        format!(
            "字段 {field:?} 需要 {expected}，实际为 {}",
            value.kind().as_str()
        ),
        [
            ("field", DiagnosticParam::Text(field.to_owned())),
            ("expected", DiagnosticParam::Text(expected.to_owned())),
            (
                "actual",
                DiagnosticParam::Text(value.kind().as_str().to_owned()),
            ),
        ],
    )
}

/// 创建带结构化参数的错误诊断。
fn error<I, K>(
    code: &str,
    message_id: &str,
    span: SourceSpan,
    message: String,
    params: I,
) -> ConfigDiagnostic
where
    I: IntoIterator<Item = (K, DiagnosticParam)>,
    K: Into<String>,
{
    Diagnostic::error_at(code, message_id, span, message)
        .with_params(params.into_iter().map(|(key, value)| (key.into(), value)))
}

/// 判断导出路径是否为安全的项目相对 `.xiao` 路径。
fn is_relative_xiao_path(path: &str) -> bool {
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains(':')
        || !path.ends_with(".xiao")
    {
        return false;
    }
    let normalized = path.replace('\\', "/");
    !normalized
        .split('/')
        .any(|segment| segment.is_empty() || segment == "..")
}

/// 生成规范化配置副本；当前主要统一表名和导出路径分隔符。
fn normalize(document: &ConfigDocument) -> ConfigDocument {
    let mut normalized = document.clone();
    if let Some(exports) = normalized.tables.get_mut("exports") {
        for entry in exports.entries.values_mut() {
            if let ConfigValue::String(path) = &mut entry.value {
                let normalized = path.replace('\\', "/");
                *path = normalized
                    .split('/')
                    .filter(|segment| !segment.is_empty() && *segment != ".")
                    .collect::<Vec<_>>()
                    .join("/");
            }
        }
    }
    for table_name in ["dependencies", "devdependencies"] {
        if let Some(table) = normalized.tables.get_mut(table_name) {
            for entry in table.entries.values_mut() {
                if let ConfigValue::Dictionary(fields) = &mut entry.value {
                    if let Some(ConfigValue::String(path)) = fields.get_mut("path") {
                        let normalized_path = path.replace('\\', "/");
                        let parts = normalized_path
                            .split('/')
                            .filter(|segment| !segment.is_empty() && *segment != ".")
                            .collect::<Vec<_>>();
                        *path = if parts.is_empty() {
                            ".".to_owned()
                        } else {
                            parts.join("/")
                        };
                    }
                }
            }
        }
    }
    normalized
}

#[cfg(test)]
/// 覆盖配置白名单、身份字段、导出路径和类型诊断。
mod tests {
    use crate::{parse_config, parse_config_project};
    use xiao_source::SourceFile;

    #[test]
    /// 未知项目字段必须以稳定编号失败。
    fn rejects_unknown_project_field() {
        let source =
            SourceFile::from_text("[project]\nname = \"demo\"\nversion = \"1\"\ntypo = true\n");
        let errors = parse_config_project(&source).expect_err("未知字段应失败");
        assert!(
            errors
                .iter()
                .any(|error| error.code() == super::UNKNOWN_FIELD_CODE)
        );
    }

    #[test]
    /// 导出路径必须位于项目根下并以 `.xiao` 结尾。
    fn rejects_unsafe_export_path() {
        let source = SourceFile::from_text(
            "[project]\nname = \"demo\"\nversion = \"1\"\n[exports]\napi = \"../api.xiao\"\n",
        );
        let errors = parse_config(&source).expect_err("路径穿越应失败");
        assert!(
            errors
                .iter()
                .any(|error| error.code() == super::INVALID_EXPORT_PATH_CODE)
        );
    }

    #[test]
    /// 缺少项目身份时，项目专用入口应报告必填表诊断。
    fn requires_project_identity_for_project_entry() {
        let source = SourceFile::from_text("[language]\nlocale = \"zh-CN\"\n");
        let errors = parse_config_project(&source).expect_err("项目配置需要身份");
        assert!(
            errors
                .iter()
                .any(|error| error.code() == super::MISSING_REQUIRED_CODE)
        );
    }

    #[test]
    /// 优化表沿用 13A 字段并接受 O0--O3 的静态声明。
    fn accepts_optimization_configuration_fields() {
        let source = SourceFile::from_text(
            "[optimization]\nlevel = 2\npass_set = [\"fold\"]\ndebug_info = false\nsource_map = true\ndiagnostic_events = true\nallow_cpu_specialization = false\nallow_lto = false\nexperimental_passes = []\n",
        );
        assert!(parse_config(&source).is_ok());
    }

    #[test]
    /// 优化级别超出冻结范围时使用既有配置值错误边界。
    fn rejects_invalid_optimization_level() {
        let source = SourceFile::from_text("[Optimization]\nlevel = 4\n");
        let errors = parse_config(&source).expect_err("非法优化级别应失败");
        assert!(errors.iter().any(|error| {
            error.code() == super::CONFIG_INVALID_VALUE_CODE
                && error.message_id() == "x05.config.invalid_optimization_level"
        }));
    }
}
