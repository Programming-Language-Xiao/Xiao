//! 20E：真实前端产物的标准库/能力要求、执行前拒绝及协议上下文贯通。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use xiao_artifacts::{
    ArchiveEntry, ArchiveIndex, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR, ObjectKind,
};
use xiao_bytecode::{XiaocMetadata, encode_xiaoc, lower_program};
use xiao_driver::{
    CORE_VERSION, FrontendCompiler, FrontendRequest, PROTOCOL_VERSION, ProtocolRequest,
    ProtocolResponse, RunOptions, dispatch,
};
use xiao_intrinsics::{CAPABILITY_STDOUT, IntrinsicId, KNOWN_CAPABILITY_MASK, active_by_id};
use xiao_standard_library::current_standard_library_identity;
use xiao_xar::{
    XarBuilder, XarObject, XarRunOptions, decode_xar, run_archive, run_archive_with_event_observer,
};

/// 来自统一前端的最小可执行归档输入。
struct Fixture {
    index: ArchiveIndex,
    object: XarObject,
    intrinsics: Vec<IntrinsicId>,
}

impl Fixture {
    /// 从真实源码编译字节码，并从其 intrinsic 契约生成夹具能力要求。
    fn new(source: &str) -> Self {
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .expect("前端通过");
        let program = lower_program(&artifact.ir);
        let intrinsics = program
            .signatures
            .iter()
            .filter_map(|signature| signature.intrinsic_id())
            .map(|id| IntrinsicId::new(id).expect("有效 ID"))
            .collect::<Vec<_>>();
        let required_capabilities = intrinsics.iter().fold(0, |mask, id| {
            mask | active_by_id(*id).unwrap().capability.bit().unwrap_or(0)
        });
        let object = XarObject::from_bytes(
            ObjectKind::Xiaoc,
            encode_xiaoc(&program, XiaocMetadata::new("main")).expect("字节码编码"),
        );
        let identity = current_standard_library_identity();
        let index = ArchiveIndex {
            schema_major: INDEX_SCHEMA_MAJOR,
            schema_minor: INDEX_SCHEMA_MINOR,
            entry: "main.xiaoc".to_owned(),
            entries: vec![ArchiveEntry {
                logical_path: "main.xiaoc".to_owned(),
                object_kind: object.kind,
                digest: object.digest,
                module: "main".to_owned(),
                target: "portable".to_owned(),
                length: object.bytes.len() as u64,
            }],
            dependency_lock_digest: "0".repeat(64),
            runtime_abi_min: xiao_runtime_abi::ABI_ENCODED_VERSION,
            runtime_abi_max: xiao_runtime_abi::ABI_ENCODED_VERSION,
            platform: "portable".to_owned(),
            debug_activation: false,
            language_locale: "zh-CN".to_owned(),
            stdlib_interface_version: identity.stdlib_interface_version,
            language_version: identity.language_version,
            stdlib_modules: Vec::new(),
            required_capabilities,
        };
        Self {
            index,
            object,
            intrinsics,
        }
    }

    /// 使用公开库构造器打包，不经 CLI 或临时构建配置。
    fn build(&self, index: ArchiveIndex) -> Vec<u8> {
        let mut builder = XarBuilder::new(index);
        builder.add_object(self.object.clone()).unwrap();
        builder.build().expect("归档编码")
    }

    /// 模拟由另一语言版本生成的完整载荷，保持字节码内部元数据一致。
    fn with_language(mut self, language: &str) -> Self {
        let mut file = xiao_bytecode::decode_xiaoc(&self.object.bytes).unwrap();
        file.metadata.language_version = language.to_owned();
        file.program.abi.language_version = language.to_owned();
        self.object = XarObject::from_bytes(
            ObjectKind::Xiaoc,
            encode_xiaoc(&file.program, file.metadata).unwrap(),
        );
        self.index.language_version = language.to_owned();
        self.index.entries[0].digest = self.object.digest;
        self.index.entries[0].length = self.object.bytes.len() as u64;
        self
    }
}

/// 负例必须在任何模块加载、函数进入或输出事件之前返回稳定错误。
fn reject_before_execution(bytes: &[u8], options: XarRunOptions, expected: &str) -> String {
    let mut events = Vec::new();
    let error =
        run_archive_with_event_observer(bytes, options, |event| events.push(format!("{event:?}")))
            .expect_err("启动前拒绝");
    assert_eq!(error.code(), expected, "{error:?}");
    assert!(events.is_empty(), "不得产生执行事件：{events:?}");
    error.message().to_owned()
}

#[test]
/// 构造器按契约导出能力，版本/模块要求可往返，归档只有应用对象。
fn builder_derives_requirements_and_keeps_official_objects_external() {
    let fixture = Fixture::new("print(\"20E\")\n");
    let identity = current_standard_library_identity();
    let build = || {
        let mut index = fixture.index.clone();
        index.stdlib_interface_version = 0;
        index.language_version.clear();
        let mut builder = XarBuilder::new(index)
            .with_requirements(
                &identity,
                [
                    "stdlib.text".to_owned(),
                    "stdlib.os".to_owned(),
                    "stdlib.text".to_owned(),
                ],
                fixture.intrinsics.iter().copied(),
            )
            .unwrap();
        builder.add_object(fixture.object.clone()).unwrap();
        builder.build().unwrap()
    };
    let bytes = build();
    assert_eq!(bytes, build());
    let archive = decode_xar(&bytes).unwrap();
    assert_eq!(archive.index().stdlib_modules, ["stdlib.os", "stdlib.text"]);
    assert_eq!(
        archive.index().stdlib_interface_version,
        identity.stdlib_interface_version
    );
    assert_eq!(archive.index().language_version, identity.language_version);
    assert_eq!(archive.index().required_capabilities, CAPABILITY_STDOUT);
    assert_eq!(archive.index().entries.len(), 1);
    assert_eq!(archive.members().len(), 2);
    assert!(
        archive
            .index()
            .entries
            .iter()
            .all(|entry| !entry.module.starts_with("stdlib"))
    );
    let outcome = run_archive(
        &bytes,
        XarRunOptions {
            capabilities: CAPABILITY_STDOUT,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(outcome.result.is_success(), "{outcome:?}");
    assert!(outcome.events.iter().any(
        |event| matches!(event, xiao_vm::VmEvent::IntrinsicOutput { text } if text == "20E\n")
    ));
}

#[test]
/// 接口版本、语言版本和缺模块分别拒绝，错误包含期望/实际或模块身份。
fn incompatible_standard_library_is_rejected_before_execution() {
    let fixture = Fixture::new("print(\"must not run\")\n");
    let mut interface = fixture.index.clone();
    interface.stdlib_interface_version += 1;
    let message = reject_before_execution(
        &fixture.build(interface),
        XarRunOptions::default(),
        "X17-XAR-007",
    );
    assert!(
        message.contains("期望 2") && message.contains("实际 1"),
        "{message}"
    );
    let language = Fixture::new("print(\"must not run\")\n").with_language("0.2.0");
    let message = reject_before_execution(
        &language.build(language.index.clone()),
        XarRunOptions::default(),
        "X17-XAR-007",
    );
    assert!(
        message.contains("0.2.0") && message.contains("0.1.0"),
        "{message}"
    );
    let mut missing = fixture.index.clone();
    missing.stdlib_modules = vec!["stdlib.not_installed_20e".to_owned()];
    let message = reject_before_execution(
        &fixture.build(missing),
        XarRunOptions::default(),
        "X17-XAR-008",
    );
    assert!(message.contains("stdlib.not_installed_20e"));
}

#[test]
/// 七个冻结能力位均在启动前核对，并报告缺失名称。
fn every_missing_capability_is_rejected_before_execution() {
    let fixture = Fixture::new("print(\"must not run\")\n");
    for (bit, name) in [
        "filesystem",
        "clock",
        "process",
        "network",
        "native_extension",
        "stdout",
        "stdin",
    ]
    .into_iter()
    .enumerate()
    {
        let mut index = fixture.index.clone();
        index.required_capabilities |= 1 << bit;
        let options = XarRunOptions {
            capabilities: KNOWN_CAPABILITY_MASK & !(1 << bit),
            ..Default::default()
        };
        let message = reject_before_execution(&fixture.build(index), options, "X06-RUNTIME-026");
        assert!(message.contains(name), "{message}");
    }
}

#[test]
/// 即使测试归档故意少报要求，VM 也不能把受限请求恢复成默认全开。
fn runtime_keeps_the_same_request_capabilities() {
    let fixture = Fixture::new("print(\"must not print\")\n");
    let mut index = fixture.index.clone();
    index.required_capabilities = 0;
    let outcome = run_archive(
        &fixture.build(index),
        XarRunOptions {
            capabilities: 0,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(outcome.report.as_ref().unwrap().code, "X06-RUNTIME-026");
    assert!(
        !outcome
            .events
            .iter()
            .any(|event| matches!(event, xiao_vm::VmEvent::IntrinsicOutput { .. }))
    );
}

/// 协议测试临时归档，仅删除自己创建的单个文件。
struct ArchiveFile(PathBuf);

impl Drop for ArchiveFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
/// 真实输出程序不能通过更改索引标签伪装其载荷语言版本。
fn review_builder_rejects_relabelled_payload_language() {
    let fixture = Fixture::new("print(\"foreign language\")\n").with_language("0.2.0");
    let mut index = fixture.index.clone();
    index.language_version = current_standard_library_identity().language_version;
    let mut builder = XarBuilder::new(index);
    builder.add_object(fixture.object).unwrap();
    let error = xiao_xar::XarRunError::from(builder.build().unwrap_err());
    assert_eq!(error.code(), "X17-XAR-007");
    assert!(error.message().contains("0.1.0") && error.message().contains("0.2.0"));
}

#[test]
/// 协议语言选择和初次读取也只能检查索引，不能抢先解码损坏的对象。
fn review_protocol_checks_context_before_corrupt_payload() {
    let fixture = Fixture::new("print(\"must not run\")\n");
    let path = ArchiveFile(std::env::temp_dir().join(format!(
        "xiao-20e-review-payload-{}.xar",
        std::process::id()
    )));
    for (interface, capabilities, expected) in [
        (
            fixture.index.stdlib_interface_version + 1,
            KNOWN_CAPABILITY_MASK,
            "X17-XAR-007",
        ),
        (fixture.index.stdlib_interface_version, 0, "X06-RUNTIME-026"),
    ] {
        let mut index = fixture.index.clone();
        index.stdlib_interface_version = interface;
        let mut bytes = fixture.build(index);
        let archive = decode_xar(&bytes).unwrap();
        let object = archive
            .members()
            .iter()
            .find(|member| member.path != xiao_xar::XAR_INDEX_PATH)
            .unwrap();
        let header = object.local_header_offset as usize;
        let name_length =
            u16::from_le_bytes(bytes[header + 26..header + 28].try_into().unwrap()) as usize;
        let extra_length =
            u16::from_le_bytes(bytes[header + 28..header + 30].try_into().unwrap()) as usize;
        bytes[header + 30 + name_length + extra_length] ^= 1;
        assert!(decode_xar(&bytes).is_err());
        std::fs::write(&path.0, bytes).unwrap();
        let response = dispatch(ProtocolRequest::RunArchive {
            request_id: "20e-corrupt-payload".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            locale: Some("en-US".to_owned()),
            path: path.0.display().to_string(),
            options: RunOptions {
                capabilities,
                ..Default::default()
            },
            diagnostics: None,
            debug: false,
        });
        let ProtocolResponse::Error { error, .. } = response else {
            panic!("启动前拒绝");
        };
        assert_eq!(error.code, expected, "{error:?}");
        assert_eq!(
            error.details["audit"]["verified_member_count"], 1,
            "只能声称索引已校验"
        );
    }
}

#[test]
/// ABI/平台不兼容必须先于诊断启动；子进程隔离诊断环境，避免启动真实窗口。
fn review_protocol_context_precedes_diagnostic_start() {
    const CHILD_FLAG: &str = "XIAO_20E_REVIEW_DIAGNOSTIC_CHILD";
    if std::env::var_os(CHILD_FLAG).is_none() {
        let missing = std::env::temp_dir().join(format!(
            "xiao-20e-review-missing-renderer-{}",
            std::process::id()
        ));
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "review_protocol_context_precedes_diagnostic_start",
                "--nocapture",
            ])
            .env(CHILD_FLAG, "1")
            .env("XIAO_DIAGNOSTICS_PATH", missing)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        return;
    }
    let fixture = Fixture::new("print(\"must not run\")\n");
    let path = ArchiveFile(std::env::temp_dir().join(format!(
        "xiao-20e-review-diagnostics-{}.xar",
        std::process::id()
    )));
    for case in ["abi", "platform", "entry_target"] {
        let mut index = fixture.index.clone();
        match case {
            "abi" => {
                index.runtime_abi_min += 1;
                index.runtime_abi_max += 1;
            }
            "platform" => index.platform = "review-unsupported-platform".to_owned(),
            _ => index.entries[0].target = "review-unsupported-target".to_owned(),
        }
        std::fs::write(&path.0, fixture.build(index)).unwrap();
        let response = dispatch(ProtocolRequest::RunArchive {
            request_id: "20e-no-diagnostic-start".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            locale: Some("en-US".to_owned()),
            path: path.0.display().to_string(),
            options: RunOptions::default(),
            diagnostics: None,
            debug: true,
        });
        let ProtocolResponse::Error { error, .. } = response else {
            panic!("启动前拒绝");
        };
        assert_eq!(
            error.code,
            if case == "abi" {
                "X17-XAR-007"
            } else {
                "X17-XAR-008"
            },
            "{case}: {error:?}"
        );
        let audit = &error.details["audit"];
        assert_eq!(audit["verified_member_count"], 1);
        if case != "abi" {
            assert_eq!(audit["platform_compatible"], false);
        }
    }
}

#[test]
/// 运行协议把本次掩码传入归档校验；默认省略掩码的旧请求仍可执行。
fn archive_protocol_passes_capabilities_to_preflight() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let fixture = Fixture::new("print(\"protocol\")\n");
    let path = ArchiveFile(std::env::temp_dir().join(format!(
        "xiao-20e-{}-{}.xar",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::write(&path.0, fixture.build(fixture.index.clone())).unwrap();
    for capabilities in [0, CAPABILITY_STDOUT] {
        let response = dispatch(ProtocolRequest::RunArchive {
            request_id: "20e".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            locale: Some("en-US".to_owned()),
            path: path.0.display().to_string(),
            options: RunOptions {
                capabilities,
                ..Default::default()
            },
            diagnostics: None,
            debug: false,
        });
        match response {
            ProtocolResponse::Error { error, .. } if capabilities == 0 => {
                assert_eq!(error.code, "X06-RUNTIME-026");
                assert!(error.message.contains("stdout"));
                assert!(error.text.as_deref().unwrap().contains("stdout"));
            }
            ProtocolResponse::Result {
                exit_code, events, ..
            } if capabilities != 0 => {
                assert_eq!(exit_code, 0);
                assert!(events.iter().any(|event| event.kind == "intrinsic_output"));
            }
            other => panic!("{other:?}"),
        }
    }
    let options: RunOptions = serde_json::from_value(
        serde_json::json!({"max_call_depth":1024,"event_capacity":256,"timeout_ms":null}),
    )
    .unwrap();
    assert_eq!(options.capabilities, KNOWN_CAPABILITY_MASK);
}

#[test]
/// 未设置身份时无法编码，官方标准库不能被作为应用模块复制入包。
fn builder_requires_metadata_and_rejects_official_module_objects() {
    let fixture = Fixture::new("print(\"20E\")\n");
    let mut missing = fixture.index.clone();
    missing.stdlib_interface_version = 0;
    missing.language_version.clear();
    let mut builder = XarBuilder::new(missing);
    builder.add_object(fixture.object.clone()).unwrap();
    assert_eq!(
        xiao_xar::XarRunError::from(builder.build().unwrap_err()).code(),
        "X17-XAR-007"
    );
    let mut copied = fixture.index.clone();
    copied.entries[0].module = "stdlib.text".to_owned();
    let mut builder = XarBuilder::new(copied);
    builder.add_object(fixture.object.clone()).unwrap();
    assert!(
        builder
            .build()
            .unwrap_err()
            .to_string()
            .contains("不能携带官方标准库")
    );
}

#[test]
/// 五个要求登记的消息键及能力失败键在中英文目录均能精确渲染。
fn archive_error_messages_render_in_both_catalogs() {
    let renderer = xiao_i18n::builtin_renderer();
    let params = BTreeMap::from([(
        "message".to_owned(),
        xiao_i18n::MessageParam::Text("fixture".to_owned()),
    )]);
    for locale in ["zh-CN", "en-US"] {
        for key in [
            "archive_missing_object",
            "archive_version_incompatible",
            "archive_dependency_unsatisfied",
            "archive_validation_failed",
            "archive_invalid_requirements",
            "archive_capability_unavailable",
        ] {
            let message = renderer.render(
                &xiao_i18n::LocaleContext::new(locale),
                &format!("x17.xar.{key}"),
                &params,
            );
            assert_eq!(message.fallback, xiao_i18n::Fallback::Exact);
            assert!(!message.format_failed);
            assert!(message.text.contains("fixture"));
        }
    }
}
