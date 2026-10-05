//! 18B 产物验证和缓存维护协议边界。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::mapping::protocol_error_body;
use super::message::{ProtocolResponse, ProtocolValue};
use super::run::protocol_error_response;
use super::validate::validate_versions;
use crate::run::ExitCode;
use xiao_artifacts::ArtifactStore;
use xiao_package::CacheLayout;

/// 只验证 `.xiaoc` 或 `.xar`，不创建 VM、不执行用户代码。
pub(super) fn verify_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    path: String,
    detail: bool,
) -> ProtocolResponse {
    if let Err(error) = validate_versions(protocol_version, core_version) {
        return protocol_error_response(Some(request_id), &error);
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return verification_error(
                request_id,
                &path,
                "X11-VERIFY-001",
                format!("无法读取产物：{error}"),
            );
        }
    };
    let lower = Path::new(&path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let summary = match lower.as_str() {
        "xiaoc" => match xiao_bytecode::decode_xiaoc(&bytes) {
            Ok(file) => json!({
                "kind": "xiaoc",
                "valid": true,
                "bytes": bytes.len(),
                "format_major": file.header.format_major,
                "format_minor": file.header.format_minor,
                "runtime_abi_min": file.header.runtime_abi_min,
                "runtime_abi_max": file.header.runtime_abi_max,
                "detail": detail,
            }),
            Err(error) => {
                return verification_error(
                    request_id,
                    &path,
                    "X11-VERIFY-002",
                    format!("`.xiaoc` 校验失败：{error}"),
                );
            }
        },
        "xar" => match xiao_xar::decode_xar(&bytes) {
            Ok(archive) => json!({
                "kind": "xar",
                "valid": true,
                "bytes": bytes.len(),
                "members": archive.members().len(),
                "index_schema_major": archive.index().schema_major,
                "index_schema_minor": archive.index().schema_minor,
                "entry": archive.index().entry,
                "detail": detail,
            }),
            Err(error) => {
                return verification_error(
                    request_id,
                    &path,
                    "X11-VERIFY-003",
                    format!("`.xar` 校验失败：{error}"),
                );
            }
        },
        _ => {
            return verification_error(
                request_id,
                &path,
                "X11-VERIFY-004",
                "verify 只接受 `.xiaoc` 或 `.xar` 输入".to_owned(),
            );
        }
    };
    ProtocolResponse::Result {
        request_id,
        operation: "verify".to_owned(),
        exit_code: ExitCode::Success.as_process_code(),
        exit_name: "success".to_owned(),
        diagnostics: Vec::new(),
        report: None,
        events: Vec::new(),
        metrics: None,
        value: Some(ProtocolValue {
            kind: "verification".to_owned(),
            value: serde_json::to_string(&summary).expect("验证摘要应可序列化"),
        }),
        artifact: None,
        audit: None,
    }
}

/// 使用 16B 的两阶段 API 维护缓存；`clean` 默认只生成计划。
pub(super) fn cache_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    action: String,
    apply: bool,
) -> ProtocolResponse {
    if let Err(error) = validate_versions(protocol_version, core_version) {
        return protocol_error_response(Some(request_id), &error);
    }
    let layout = match CacheLayout::from_environment() {
        Ok(layout) => layout,
        Err(error) => {
            return cache_error(
                request_id,
                "X11-CACHE-001",
                format!("无法定位缓存：{error}"),
            );
        }
    };
    let cache_root = layout.cache_root();
    if action == "list" {
        return cache_result(
            request_id,
            json!({
                "action": "list",
                "status": "observed",
                "cache_root": cache_root,
            }),
        );
    }
    let store = match ArtifactStore::open(&cache_root) {
        Ok(store) => store,
        Err(error) => return cache_error(request_id, "X11-CACHE-002", error.to_string()),
    };
    let references = match store.collect_references(None, &[], &[]) {
        Ok(references) => references,
        Err(error) => return cache_error(request_id, "X11-CACHE-003", error.to_string()),
    };
    match action.as_str() {
        "verify" => {
            let plan = match store.plan_cleanup(&references) {
                Ok(plan) => plan,
                Err(error) => return cache_error(request_id, "X11-CACHE-004", error.to_string()),
            };
            cache_result(
                request_id,
                json!({
                    "action": "verify",
                    "status": "verified",
                    "cache_root": cache_root,
                    "protected_references": references.iter().count(),
                    "candidate_count": plan.candidates().len(),
                }),
            )
        }
        "rebuild" => cache_result(
            request_id,
            json!({
                "action": "rebuild",
                "status": "planned",
                "cache_root": cache_root,
                "protected_references": references.iter().count(),
            }),
        ),
        "clean" => {
            let plan = match store.plan_cleanup(&references) {
                Ok(plan) => plan,
                Err(error) => return cache_error(request_id, "X11-CACHE-004", error.to_string()),
            };
            if !apply {
                return cache_result(
                    request_id,
                    json!({
                        "action": "clean",
                        "status": "planned",
                        "cache_root": cache_root,
                        "candidate_count": plan.candidates().len(),
                    }),
                );
            }
            match store.apply_cleanup(&plan, &references) {
                Ok(report) => cache_result(
                    request_id,
                    json!({
                        "action": "clean",
                        "status": "applied",
                        "cache_root": cache_root,
                        "removed_count": report.removed.len(),
                        "quarantined_count": report.quarantined.len(),
                    }),
                ),
                Err(error) => cache_error(request_id, "X11-CACHE-005", error.to_string()),
            }
        }
        _ => cache_error(
            request_id,
            "X11-CACHE-006",
            format!("不支持的 cache 操作：{action}"),
        ),
    }
}

fn verification_error(
    request_id: String,
    path: &str,
    code: &'static str,
    message: String,
) -> ProtocolResponse {
    let mut details = BTreeMap::new();
    details.insert("path".to_owned(), Value::String(path.to_owned()));
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            code,
            "x11.verify.failed",
            message,
            Some("verification".to_owned()),
            Some("检查输入路径和产物完整性后重试".to_owned()),
            details,
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

fn cache_result(request_id: String, value: Value) -> ProtocolResponse {
    ProtocolResponse::Result {
        request_id,
        operation: "cache".to_owned(),
        exit_code: ExitCode::Success.as_process_code(),
        exit_name: "success".to_owned(),
        diagnostics: Vec::new(),
        report: None,
        events: Vec::new(),
        metrics: None,
        value: Some(ProtocolValue {
            kind: "cache".to_owned(),
            value: serde_json::to_string(&value).expect("缓存摘要应可序列化"),
        }),
        artifact: None,
        audit: None,
    }
}

fn cache_error(request_id: String, code: &'static str, message: String) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            code,
            "x11.cache.failed",
            message,
            Some("cache".to_owned()),
            Some("检查 XIAO_HOME 和缓存权限后重试".to_owned()),
            BTreeMap::new(),
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}
