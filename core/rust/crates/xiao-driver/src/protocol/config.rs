//! 构建前配置解析、固化和旁置文件写入。

use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use xiao_config::{ConfigDocument, ConfigValue, parse_config_text};

use super::message::ProtocolRuntimeConfig;
use super::request::ProtocolError;

/// 构建时已验证的配置摘要；写文件延迟到原生链接成功之后。
pub(super) struct FrozenRuntimeConfig {
    pub(super) value: Value,
}

/// 解析并校验 `config.xiao`，只保留静态配置树，不执行用户代码。
pub(super) fn freeze_runtime_config(
    _output: &str,
    text: Option<&str>,
) -> Result<Option<FrozenRuntimeConfig>, ProtocolError> {
    let Some(text) = text else {
        return Ok(None);
    };
    let document = parse_config_text(text).map_err(|diagnostics| {
        let first = diagnostics
            .first()
            .map(|diagnostic| format!("{}: {}", diagnostic.code(), diagnostic.message()))
            .unwrap_or_else(|| "配置解析失败".to_owned());
        ProtocolError::build(format!("config.xiao 校验失败：{first}"))
    })?;
    Ok(Some(FrozenRuntimeConfig {
        value: config_document_value(&document),
    }))
}

/// 把不可执行配置模型转换为确定性 JSON 值。
fn config_document_value(document: &ConfigDocument) -> Value {
    let tables = document
        .tables
        .iter()
        .map(|(name, table)| {
            let entries = table
                .entries
                .iter()
                .map(|(key, entry)| (key.clone(), config_value(&entry.value)))
                .collect::<serde_json::Map<_, _>>();
            (name.clone(), Value::Object(entries))
        })
        .collect::<serde_json::Map<_, _>>();
    Value::Object(serde_json::Map::from_iter([
        ("format_version".to_owned(), json!(1)),
        ("cli_overrides".to_owned(), json!(true)),
        ("tables".to_owned(), Value::Object(tables)),
    ]))
}

/// 递归转换配置字面量。
fn config_value(value: &ConfigValue) -> Value {
    match value {
        ConfigValue::String(value) => Value::String(value.clone()),
        ConfigValue::Integer(value) => json!(value),
        ConfigValue::Float(value) => json!(value),
        ConfigValue::Boolean(value) => json!(value),
        ConfigValue::Array(values) => Value::Array(values.iter().map(config_value).collect()),
        ConfigValue::Dictionary(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), config_value(value)))
                .collect(),
        ),
    }
}

/// 将配置摘要原子地写到可执行文件旁边。
pub(super) fn write_runtime_config(
    executable: &std::path::Path,
    config: &FrozenRuntimeConfig,
) -> Result<ProtocolRuntimeConfig, ProtocolError> {
    let path = runtime_config_path(executable);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| ProtocolError::build(format!("无法创建配置目录：{error}")))?;
    }
    let bytes = serde_json::to_vec_pretty(&config.value)
        .map_err(|error| ProtocolError::build(format!("无法编码运行时配置：{error}")))?;
    let temporary = PathBuf::from(format!("{}.tmp-{}", path.display(), std::process::id()));
    fs::write(&temporary, [bytes.as_slice(), b"\n"].concat())
        .map_err(|error| ProtocolError::build(format!("无法写入运行时配置：{error}")))?;
    if cfg!(windows)
        && let Err(error) = fs::remove_file(&path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        let _ = fs::remove_file(&temporary);
        return Err(ProtocolError::build(format!("无法替换运行时配置：{error}")));
    }
    if let Err(error) = fs::rename(&temporary, &path) {
        let _ = fs::remove_file(&temporary);
        return Err(ProtocolError::build(format!("无法提交运行时配置：{error}")));
    }
    Ok(ProtocolRuntimeConfig {
        path: path.display().to_string(),
        format_version: 1,
        cli_overrides: true,
    })
}

/// 返回原生可执行文件旁的运行时配置路径。
pub(super) fn runtime_config_path(executable: &std::path::Path) -> PathBuf {
    PathBuf::from(format!("{}.xiao-runtime.json", executable.display()))
}
