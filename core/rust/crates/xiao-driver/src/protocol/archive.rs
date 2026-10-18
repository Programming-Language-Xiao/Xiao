//! 18B 归档协议适配；编译、缓存和 ZIP 均交给 Rust 库层。

use std::collections::BTreeMap;

use serde_json::json;

use super::mapping::{exit_name, protocol_error_body};
use super::message::{ProtocolResponse, ProtocolValue};
use super::request::OptimizationConfig;
use super::run::protocol_error_response;
use super::validate::validate_versions;
use crate::{ArchiveBuildRequest, CancellationToken, ExitCode, build_archive};

/// 校验协议边界并返回已经写出的归档及逐模块校验/缓存证据。
#[allow(clippy::too_many_arguments)]
pub(super) fn build_archive_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    locale: Option<String>,
    path: String,
    output: Option<String>,
    optimization: OptimizationConfig,
    cancellation: &CancellationToken,
) -> ProtocolResponse {
    if let Err(error) = validate_versions(protocol_version, core_version) {
        return protocol_error_response(Some(request_id), &error);
    }
    let level = match xiao_optimizer::OptimizationLevel::try_from(optimization.level) {
        Ok(level) => level,
        Err(error) => {
            return protocol_error_response(
                Some(request_id),
                &super::request::ProtocolError::request("optimization.level", error.to_string()),
            );
        }
    };
    let mut request = ArchiveBuildRequest::new(path);
    request.output = output.map(Into::into);
    request.optimization_level = level;
    request.debug = optimization.debug;
    request.locale = locale.unwrap_or_else(|| "zh-CN".to_owned());
    match build_archive(&request, cancellation) {
        Ok(result) => ProtocolResponse::Result {
            request_id,
            operation: "build_archive".to_owned(),
            exit_code: 0, exit_name: exit_name(ExitCode::Success).to_owned(),
            diagnostics: Vec::new(), report: None, events: Vec::new(), metrics: None, artifact: None, audit: None, cache: None,
            value: Some(ProtocolValue { kind: "archive_build".to_owned(), value: json!({
                "output": result.output, "digest": result.digest.as_hex(), "verified": true,
                "optimization_level": optimization.level,
                "stdlib_modules": result.index.stdlib_modules,
                "required_capabilities": result.index.required_capabilities,
                "modules": result.modules.iter().map(|module| json!({
                    "module": module.module, "status": if module.hit { "hit" } else { "written" },
                    "digest": module.digest.as_hex(), "object_kind": "xiaoc", "verified": true,
                    "recompiled": !module.hit, "recompile_reason": if module.hit { None } else { Some("cache_miss") },
                    "reason": if module.hit { "cache_hit_verified" } else { "cache_miss" },
                })).collect::<Vec<_>>(),
            }).to_string() }),
        },
        Err(error) => ProtocolResponse::Error {
            request_id: Some(request_id),
            error: protocol_error_body(&error.code, "x11.driver.archive_build", error.message, Some("archive_build".to_owned()), None, BTreeMap::new()),
            report: None,
            exit_code: ExitCode::ArtifactRejected.as_process_code(),
        },
    }
}
