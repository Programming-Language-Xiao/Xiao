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
use xiao_artifacts::{
    ArtifactStore, Digest256, GlobalRecord, IndexStore, ObjectKind, ReferenceSet,
};
use xiao_package::CacheLayout;

/// 未实现签名时必须随报告显示的完整性边界。
const UNSIGNED_INTEGRITY_WARNING: &str = "SHA-256 只保证完整性，不代表发布者可信";

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
            Ok(file) => {
                let details = if detail {
                    match xiao_bytecode::inspect_xiaoc(&bytes) {
                        Ok(inspection) => json!({
                            "module_id": inspection.module_id,
                            "optimization_fingerprint": inspection.optimization_fingerprint,
                            "dependency_lock_digest": inspection.dependency_lock_digest,
                            "sections": inspection
                                .sections
                                .into_iter()
                                .map(|(name, bytes)| json!({ "name": name, "bytes": bytes }))
                                .collect::<Vec<_>>(),
                        }),
                        Err(error) => {
                            return verification_error(
                                request_id,
                                &path,
                                "X11-VERIFY-002",
                                format!("`.xiaoc` 详细校验失败：{error}"),
                            );
                        }
                    }
                } else {
                    Value::Null
                };
                json!({
                    "kind": "xiaoc",
                    "valid": true,
                    "bytes": bytes.len(),
                    "format_major": file.header.format_major,
                    "format_minor": file.header.format_minor,
                    "runtime_abi_min": file.header.runtime_abi_min,
                    "runtime_abi_max": file.header.runtime_abi_max,
                    "detail": detail,
                    "details": details,
                    "release_report": xiaoc_release_report(&file, &bytes),
                })
            }
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
            Ok(archive) => {
                let details = if detail {
                    let members = archive
                        .members()
                        .iter()
                        .map(|member| {
                            let digest = archive
                                .read_member(&member.path)
                                .map(|bytes| Digest256::of_bytes(&bytes).as_hex())
                                .unwrap_or_default();
                            json!({
                                "path": member.path,
                                "compression": format!("{:?}", member.compression),
                                "compressed_size": member.compressed_size,
                                "uncompressed_size": member.uncompressed_size,
                                "crc32": member.crc32,
                                "local_header_offset": member.local_header_offset,
                                "sha256": digest,
                            })
                        })
                        .collect::<Vec<_>>();
                    json!({
                        "members": members,
                        "entries": archive
                            .index()
                            .entries
                            .iter()
                            .map(|entry| json!({
                                "logical_path": entry.logical_path,
                                "object_kind": entry.object_kind.as_str(),
                                "digest": entry.digest.as_hex(),
                                "module": entry.module,
                                "target": entry.target,
                                "length": entry.length,
                            }))
                            .collect::<Vec<_>>(),
                    })
                } else {
                    Value::Null
                };
                json!({
                    "kind": "xar",
                    "valid": true,
                    "bytes": bytes.len(),
                    "members": archive.members().len(),
                    "index_schema_major": archive.index().schema_major,
                    "index_schema_minor": archive.index().schema_minor,
                    "entry": archive.index().entry,
                    "detail": detail,
                    "details": details,
                    "release_report": xar_release_report(&archive, &bytes),
                })
            }
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
        cache: None,
    }
}

/// 为单模块 `.xiaoc` 生成发布报告。
fn xiaoc_release_report(file: &xiao_bytecode::XiaocFile, bytes: &[u8]) -> Value {
    json!({
        "schema_version": 1,
        "artifact_sha256": Digest256::of_bytes(bytes).as_hex(),
        "source_digest": file.metadata.source_digest,
        "dependency_lock_digest": file.metadata.dependency_lock_digest,
        "optimization_fingerprint": file.metadata.optimization_fingerprint,
        "toolchain": format!("xiao-codegen-llvm/{}", xiao_codegen_llvm::CODEGEN_VERSION),
        "xiaoc_objects": [{
            "module_id": file.metadata.module_id,
            "sha256": Digest256::of_bytes(bytes).as_hex(),
            "bytes": bytes.len(),
        }],
        "archive_members": [],
        "target_platform": xiaoc_platform_label(&file.metadata.platform),
        "runtime_abi": {
            "min": file.header.runtime_abi_min,
            "max": file.header.runtime_abi_max,
        },
                    "reproducibility": {
                        "status": "not-measured",
                        "allowed_differences": [],
                    },
                    "revocation": {
                        "status": "unavailable",
                        "message": "当前没有撤销机制；撤销策略等待信任模型冻结",
                    },
                    "signature": {
            "status": "unsigned",
            "warning": UNSIGNED_INTEGRITY_WARNING,
        },
    })
}

/// 为已验证 `.xar` 生成发布报告。
fn xar_release_report(archive: &xiao_xar::XarArchive, bytes: &[u8]) -> Value {
    let index = archive.index();
    let xiaoc_metadata = index
        .entries
        .iter()
        .filter(|entry| entry.object_kind == ObjectKind::Xiaoc)
        .filter_map(|entry| {
            archive
                .read_object(ObjectKind::Xiaoc, entry.digest)
                .ok()
                .and_then(|bytes| xiao_bytecode::decode_xiaoc(&bytes).ok())
                .map(|file| {
                    (
                        file.metadata.source_digest,
                        file.metadata.optimization_fingerprint,
                    )
                })
        })
        .collect::<Vec<_>>();
    let xiaoc_objects = index
        .entries
        .iter()
        .filter(|entry| entry.object_kind == ObjectKind::Xiaoc)
        .map(|entry| {
            json!({
                "logical_path": entry.logical_path,
                "module_id": entry.module,
                "sha256": entry.digest.as_hex(),
                "bytes": entry.length,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": 1,
        "artifact_sha256": Digest256::of_bytes(bytes).as_hex(),
        "source_digest": xiaoc_metadata
            .first()
            .map_or(Value::Null, |(digest, _)| Value::String(digest.clone())),
        "dependency_lock_digest": index.dependency_lock_digest,
        "optimization_fingerprint": xiaoc_metadata.first().map_or_else(
            || Value::Null,
            |(_, fingerprint)| Value::String(fingerprint.clone()),
        ),
        "toolchain": xiao_xar::XAR_TOOLCHAIN_FINGERPRINT,
        "xiaoc_objects": xiaoc_objects,
        "archive_members": archive.members().iter().map(|member| json!({
            "path": member.path,
            "compressed_size": member.compressed_size,
            "uncompressed_size": member.uncompressed_size,
            "sha256": archive.read_member(&member.path)
                .map(|bytes| Digest256::of_bytes(&bytes).as_hex())
                .unwrap_or_default(),
        })).collect::<Vec<_>>(),
        "target_platform": index.platform,
        "runtime_abi": {
            "min": index.runtime_abi_min,
            "max": index.runtime_abi_max,
        },
        "reproducibility": {
            "status": "not-measured",
            "allowed_differences": [],
        },
        "revocation": {
            "status": "unavailable",
            "message": "当前没有撤销机制；撤销策略等待信任模型冻结",
        },
        "signature": {
            "status": "unsigned",
            "warning": UNSIGNED_INTEGRITY_WARNING,
        },
    })
}

/// 将 `.xiaoc` 平台约束转换为不含宿主路径的报告标签。
fn xiaoc_platform_label(platform: &xiao_bytecode::XiaocPlatform) -> String {
    match platform {
        xiao_bytecode::XiaocPlatform::Independent => "portable".to_owned(),
        xiao_bytecode::XiaocPlatform::Constrained { target, features } => {
            if features.is_empty() {
                target.clone()
            } else {
                format!("{target};features={}", features.join(","))
            }
        }
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
    if !matches!(action.as_str(), "list" | "verify" | "rebuild" | "clean") {
        return cache_error(
            request_id,
            "X11-CACHE-006",
            format!("不支持的 cache 操作：{action}"),
        );
    }
    if apply && action != "clean" {
        return cache_error(
            request_id,
            "X11-CACHE-007",
            "只有 cache clean 支持 --apply".to_owned(),
        );
    }
    if action == "list" {
        let store = ArtifactStore::open_read_only(&cache_root);
        let mut inventories = Vec::new();
        for kind in object_kinds() {
            match store.inventory(kind) {
                Ok(inventory) => inventories.push(json!({
                    "kind": inventory.kind.as_str(),
                    "verified_count": inventory.verified_count,
                    "invalid_count": inventory.invalid_count,
                    "verified_bytes": inventory.verified_bytes,
                })),
                Err(error) => {
                    return cache_error(request_id, "X11-CACHE-002", error.to_string());
                }
            }
        }
        let object_count = inventories
            .iter()
            .filter_map(|entry| entry.get("verified_count").and_then(Value::as_u64))
            .sum::<u64>();
        let invalid_count = inventories
            .iter()
            .filter_map(|entry| entry.get("invalid_count").and_then(Value::as_u64))
            .sum::<u64>();
        let verified_bytes = inventories
            .iter()
            .filter_map(|entry| entry.get("verified_bytes").and_then(Value::as_u64))
            .sum::<u64>();
        return cache_result(
            request_id,
            json!({
                "action": "list",
                "status": "observed",
                "cache_root": cache_root,
                "object_count": object_count,
                "invalid_count": invalid_count,
                "verified_bytes": verified_bytes,
                "objects": inventories,
            }),
        );
    }
    let store = if action == "verify" || (action == "clean" && !apply) {
        ArtifactStore::open_read_only(&cache_root)
    } else {
        match ArtifactStore::open(&cache_root) {
            Ok(store) => store,
            Err(error) => return cache_error(request_id, "X11-CACHE-002", error.to_string()),
        }
    };
    // 当前 cache 顶层命令没有接收项目锁或归档索引路径；没有可枚举的引用来源时
    // 必须保守保护所有已验证对象，不能把“未扫描到”当成“没有引用”。
    let references = ReferenceSet::protect_all();
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
                    "reference_mode": if references.is_conservative() {
                        "conservative-all"
                    } else {
                        "scanned"
                    },
                    "protected_references": references.iter().count(),
                    "candidate_count": plan.candidates().len(),
                    "invalid_count": plan.invalid_count(),
                }),
            )
        }
        "rebuild" => {
            let indexes = match IndexStore::open(&cache_root) {
                Ok(indexes) => indexes,
                Err(error) => return cache_error(request_id, "X11-CACHE-004", error.to_string()),
            };
            let path = match indexes.rebuild_global_atomic(&store, |reference| {
                Some(GlobalRecord {
                    request_key: format!(
                        "object:{}:{}",
                        reference.kind.as_str(),
                        reference.digest.as_hex()
                    ),
                    object_kind: reference.kind,
                    digest: reference.digest,
                    target: "unknown".to_owned(),
                    optimization_level: 0,
                    codegen_version: xiao_codegen_llvm::CODEGEN_VERSION,
                    length: reference.length,
                })
            }) {
                Ok(path) => path,
                Err(error) => return cache_error(request_id, "X11-CACHE-004", error.to_string()),
            };
            let record_count = match indexes.read_global() {
                Ok(index) => index.records.len(),
                Err(error) => return cache_error(request_id, "X11-CACHE-004", error.to_string()),
            };
            cache_result(
                request_id,
                json!({
                    "action": "rebuild",
                    "status": "rebuilt",
                    "cache_root": cache_root,
                    "index": path,
                    "record_count": record_count,
                }),
            )
        }
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
                        "reference_mode": if references.is_conservative() {
                            "conservative-all"
                        } else {
                            "scanned"
                        },
                        "candidate_count": plan.candidates().len(),
                        "invalid_count": plan.invalid_count(),
                    }),
                );
            }
            // 16B 要求执行阶段重新收集引用，避免计划生成后新发布的对象被误删。
            let current_references = ReferenceSet::protect_all();
            match store.apply_cleanup(&plan, &current_references) {
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
        _ => unreachable!("cache 操作已经在入口校验"),
    }
}

fn object_kinds() -> [ObjectKind; 7] {
    [
        ObjectKind::Source,
        ObjectKind::Xiaoc,
        ObjectKind::Native,
        ObjectKind::Xar,
        ObjectKind::Language,
        ObjectKind::Resource,
        ObjectKind::Debug,
    ]
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
        cache: None,
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
