//! 17C/17D 归档运行的成功路径和校验前拒绝回归。

use std::fs;
use std::sync::{Mutex, OnceLock};

use xiao_artifacts::{
    ArchiveEntry, ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR, ObjectKind,
};
use xiao_bytecode::{TAC_RUNTIME_ABI_VERSION, XiaocMetadata, encode_xiaoc, lower_program};
use xiao_driver::{
    CORE_VERSION, FrontendCompiler, FrontendRequest, PROTOCOL_VERSION, ProtocolRequest,
    ProtocolResponse, RunOptions, dispatch,
};
use xiao_runtime_abi::ABI_ENCODED_VERSION;
use xiao_xar::{
    XarObject, XarRunOptions, audit_archive, encode_xar, run_archive,
    run_archive_with_event_observer,
};

static DIAGNOSTIC_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn archive_for(source: &str) -> Vec<u8> {
    archive_for_with_debug(source, false)
}

fn xiaoc_for(source: &str) -> Vec<u8> {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("源码应通过前端");
    encode_xiaoc(&lower_program(&artifact.ir), XiaocMetadata::new("main")).expect("应编码 .xiaoc")
}

fn archive_for_with_debug(source: &str, debug: bool) -> Vec<u8> {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .expect("源码应通过前端");
    let program = lower_program(&artifact.ir);
    assert_eq!(program.abi.runtime_abi_version, TAC_RUNTIME_ABI_VERSION);
    let metadata = if debug {
        XiaocMetadata::new("main").with_debug("test-diagnostics")
    } else {
        XiaocMetadata::new("main")
    };
    let xiaoc = encode_xiaoc(&program, metadata).expect("应编码 .xiaoc");
    let object = XarObject::from_bytes(ObjectKind::Xiaoc, xiaoc);
    let index = ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "main.xiaoc".to_owned(),
        entries: vec![ArchiveEntry {
            logical_path: "main.xiaoc".to_owned(),
            object_kind: ObjectKind::Xiaoc,
            digest: object.digest,
            module: "main".to_owned(),
            target: "portable".to_owned(),
            length: object.bytes.len() as u64,
        }],
        dependency_lock_digest: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_owned(),
        runtime_abi_min: ABI_ENCODED_VERSION,
        runtime_abi_max: ABI_ENCODED_VERSION,
        platform: "portable".to_owned(),
        debug_activation: debug,
        language_locale: "zh-CN".to_owned(),
    };
    encode_xar(&index, &[object]).expect("应编码归档")
}

fn debug_archive_for(source: &str) -> Vec<u8> {
    let archive =
        xiao_xar::decode_xar(&archive_for_with_debug(source, true)).expect("应解码测试归档");
    let mut index = archive.index().clone();
    index.debug_activation = true;
    let entry = &index.entries[0];
    let object = XarObject::from_bytes(
        entry.object_kind,
        archive
            .read_object(entry.object_kind, entry.digest)
            .expect("入口对象应存在"),
    );
    encode_xar(&index, &[object]).expect("应编码调试归档")
}

#[test]
fn archive_success_preserves_intrinsic_output_and_success_result() {
    let archive = archive_for("print(\"hello world!\")\n");
    let audit = audit_archive(
        &archive,
        &XarRunOptions {
            runtime_abi: ABI_ENCODED_VERSION,
            debug: true,
            ..Default::default()
        },
    )
    .expect("审计记录应可生成");
    let audit_json = audit.to_json_bytes().expect("审计记录应可序列化");
    assert!(
        String::from_utf8(audit_json)
            .unwrap()
            .contains("archive_digest")
    );
    assert!(audit.debug_activation);
    let outcome = run_archive(
        &archive,
        XarRunOptions {
            runtime_abi: ABI_ENCODED_VERSION,
            ..Default::default()
        },
    )
    .expect("归档应成功运行");
    assert!(outcome.result.is_success());
    assert!(outcome.events.iter().any(|event| {
        matches!(event, xiao_vm::VmEvent::IntrinsicOutput { text } if text == "hello world!\n")
    }));
}

#[test]
fn direct_xiaoc_run_reuses_the_production_contract_and_verifies_bytes() {
    let path = std::env::temp_dir().join(format!(
        "xiao-direct-xiaoc-{}-{}.xiaoc",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let bytes = xiaoc_for("print(\"direct\")\n");
    fs::write(&path, &bytes).expect("写入测试 .xiaoc");
    let response = dispatch(ProtocolRequest::RunXiaoc {
        request_id: "run-xiaoc".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        locale: None,
        path: path.display().to_string(),
        options: RunOptions::default(),
        diagnostics: None,
        debug: false,
    });
    let _ = fs::remove_file(&path);
    let ProtocolResponse::Result {
        operation,
        exit_code,
        events,
        cache: Some(cache),
        ..
    } = response
    else {
        panic!("直接 .xiaoc 运行应返回统一结果");
    };
    assert_eq!(operation, "run_xiaoc");
    assert_eq!(exit_code, 0);
    assert!(events.iter().any(|event| event.kind == "intrinsic_output"));
    assert_eq!(cache.status, "hit");
    assert!(cache.verified);
    assert!(!cache.recompiled);
}

#[test]
fn invalid_archive_produces_no_execution_events() {
    let mut archive = archive_for("print(\"must not run\")\n");
    let marker = b"objects/xiaoc/";
    let object_position = archive
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("归档对象路径");
    let digest_position = object_position + marker.len() + 10;
    archive[digest_position] ^= 1;
    let mut events = Vec::new();
    let result = run_archive_with_event_observer(
        &archive,
        XarRunOptions {
            runtime_abi: ABI_ENCODED_VERSION,
            ..Default::default()
        },
        |event| events.push(format!("{event:?}")),
    );
    assert!(result.is_err());
    assert!(events.is_empty());
}

#[test]
fn archive_run_honors_production_event_capacity() {
    let archive = archive_for("print(\"bounded\")\n");
    let outcome = run_archive(
        &archive,
        XarRunOptions {
            runtime_abi: ABI_ENCODED_VERSION,
            event_capacity: 1,
            ..Default::default()
        },
    )
    .expect("归档应成功运行");
    assert!(outcome.events.len() <= 1);
    assert!(outcome.dropped_events > 0);
}

#[test]
/// 归档入口直接消费内存字节，不创建运行期临时文件。
fn archive_runner_does_not_create_temporary_files() {
    let marker =
        std::env::temp_dir().join(format!("xiao-d17d-memory-only-{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let archive = archive_for("print(\"memory only\")\n");
    let outcome = run_archive(&archive, XarRunOptions::default()).expect("归档应成功运行");
    assert!(outcome.result.is_success());
    assert!(!marker.exists(), "归档运行不应落盘临时内容");
}

#[test]
fn archive_protocol_result_carries_machine_readable_audit() {
    let archive = archive_for("print(\"audited\")\n");
    let path = std::env::temp_dir().join(format!("xiao-d17d-audit-{}.xar", std::process::id()));
    std::fs::write(&path, archive).expect("应写入临时归档");
    let response = dispatch(ProtocolRequest::RunArchive {
        request_id: "archive-audit".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        locale: Some("zh-CN".to_owned()),
        path: path.display().to_string(),
        options: RunOptions::default(),
        diagnostics: None,
        debug: false,
    });
    let _ = std::fs::remove_file(&path);
    let ProtocolResponse::Result {
        audit: Some(audit),
        operation,
        exit_code,
        ..
    } = response
    else {
        panic!("归档成功响应必须携带审计记录");
    };
    assert_eq!(operation, "run_archive");
    assert_eq!(exit_code, 0);
    assert_eq!(audit.index_schema_major, INDEX_SCHEMA_MAJOR);
    assert!(audit.runtime_abi_compatible);
    assert!(audit.platform_compatible);
    let json = audit.to_json_bytes().expect("审计记录应可序列化");
    assert!(!String::from_utf8(json).unwrap().contains("XIAO_HOME"));
}

#[test]
fn verify_protocol_validates_xar_without_running_user_code() {
    let path = std::env::temp_dir().join(format!("xiao-d17d-verify-{}.xar", std::process::id()));
    std::fs::write(&path, archive_for("print(\"must not run\")\n")).expect("应写入验证归档");
    let response = dispatch(ProtocolRequest::Verify {
        request_id: "verify-xar".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        path: path.display().to_string(),
        detail: true,
    });
    let _ = std::fs::remove_file(&path);
    let ProtocolResponse::Result {
        operation,
        value: Some(value),
        events,
        exit_code,
        ..
    } = response
    else {
        panic!("合法 xar 应返回验证结果");
    };
    assert_eq!(operation, "verify");
    assert_eq!(exit_code, 0);
    assert!(events.is_empty());
    assert!(value.value.contains("\"kind\":\"xar\""));
}

#[test]
fn archive_validation_error_keeps_audit_in_error_details() {
    let source_archive =
        xiao_xar::decode_xar(&archive_for("print(\"platform\")\n")).expect("应解码测试归档");
    let mut index = source_archive.index().clone();
    index.platform = "unsupported-test-platform".to_owned();
    let entry = &index.entries[0];
    let object = XarObject::from_bytes(
        entry.object_kind,
        source_archive
            .read_object(entry.object_kind, entry.digest)
            .expect("入口对象应存在"),
    );
    let path =
        std::env::temp_dir().join(format!("xiao-d17d-audit-error-{}.xar", std::process::id()));
    std::fs::write(
        &path,
        encode_xar(&index, &[object]).expect("应编码平台错误归档"),
    )
    .expect("应写入临时归档");
    let response = dispatch(ProtocolRequest::RunArchive {
        request_id: "archive-audit-error".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        locale: Some("zh-CN".to_owned()),
        path: path.display().to_string(),
        options: RunOptions::default(),
        diagnostics: None,
        debug: false,
    });
    let _ = std::fs::remove_file(&path);
    let ProtocolResponse::Error { error, .. } = response else {
        panic!("平台不兼容必须返回错误响应");
    };
    assert_eq!(error.code, xiao_xar::ARCHIVE_DEPENDENCY_UNSATISFIED_CODE);
    assert!(error.details.contains_key("audit"));
}

#[test]
fn archive_debug_start_failure_rejects_before_returning_execution_events() {
    let _guard = DIAGNOSTIC_ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("诊断环境锁不应中毒");
    let archive = debug_archive_for("print(\"must not execute\")\n");
    let archive_path =
        std::env::temp_dir().join(format!("xiao-d17d-debug-{}.xar", std::process::id()));
    std::fs::write(&archive_path, archive).expect("应写入临时调试归档");
    let missing_renderer = std::env::temp_dir().join(format!(
        "xiao-d17d-missing-renderer-{}{}",
        std::process::id(),
        if cfg!(windows) { ".exe" } else { "" }
    ));
    let previous_renderer = std::env::var_os("XIAO_DIAGNOSTICS_PATH");
    unsafe {
        std::env::set_var("XIAO_DIAGNOSTICS_PATH", &missing_renderer);
    }
    let response = dispatch(ProtocolRequest::RunArchive {
        request_id: "archive-debug-failure".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        locale: Some("zh-CN".to_owned()),
        path: archive_path.display().to_string(),
        options: RunOptions::default(),
        diagnostics: None,
        debug: false,
    });
    unsafe {
        if let Some(previous_renderer) = previous_renderer {
            std::env::set_var("XIAO_DIAGNOSTICS_PATH", previous_renderer);
        } else {
            std::env::remove_var("XIAO_DIAGNOSTICS_PATH");
        }
    }
    let _ = std::fs::remove_file(&archive_path);
    let ProtocolResponse::Error {
        error, exit_code, ..
    } = response
    else {
        panic!("诊断窗口启动失败必须整体拒绝归档");
    };
    assert_eq!(error.code, "X11-DIAGNOSTIC-START-001");
    assert_eq!(exit_code, 2);
}
