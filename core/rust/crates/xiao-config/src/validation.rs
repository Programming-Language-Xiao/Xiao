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
            // CLI、Debug、VM、依赖和构建相关表在本阶段只保留静态值。
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
}
