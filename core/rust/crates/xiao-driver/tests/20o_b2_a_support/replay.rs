//! A5真实.xiaoc/.xar编码重放；使用完整语言语料和正式归档驱动器。

use super::{fixture, repository};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use xiao_driver::{
    ArchiveBuildRequest, CancellationToken, FrontendCompiler, ProtocolRequest, ProtocolResponse,
    RunOptions, build_archive, dispatch,
};
use xiao_vm::VmEvent;

/// ABI/容器/指令流沿用现有版本，删除源文件后归档仍能逐字恢复精确值。
pub(super) fn replay() {
    let directory = std::env::temp_dir().join(format!(
        "xiao-b2-a5-replay-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    for name in super::CASES {
        let (request, golden) = fixture(name);
        let artifact = FrontendCompiler::new().compile(&request).unwrap();
        let program = xiao_bytecode::lower_program(&artifact.ir);
        let bytes = xiao_bytecode::encode_xiaoc(&program, xiao_bytecode::XiaocMetadata::new(*name))
            .unwrap();
        let decoded = xiao_bytecode::decode_xiaoc(&bytes).unwrap();
        assert_eq!(
            u64::from(decoded.header.runtime_abi_min),
            xiao_runtime_abi::encoded_abi_version(1, 18)
        );
        assert_eq!(
            decoded.header.runtime_abi_min,
            decoded.header.runtime_abi_max
        );
        // carrier语料导入官方set模块，真实归档入口负责加载，不用无加载器入口冒充它。
        if *name != "a5_carriers" {
            let result =
                xiao_vm::run_xiaoc_production(&bytes, Default::default(), 4096, None).unwrap();
            assert!(result.result.is_success(), "{name}: {result:?}");
            let stdout: String = result
                .events
                .iter()
                .filter_map(|event| match event {
                    VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(stdout, golden, "{name} .xiaoc");
            println!("B2-A5-REPLAY case={name} format=xiaoc result=success");
        }
        let source = directory.join(format!("{name}.xiao"));
        fs::write(&source, request.source.text()).unwrap();
        let mut build = ArchiveBuildRequest::new(&source);
        build.cache_root = Some(directory.join(format!("{name}-cache")));
        let archive = build_archive(&build, &CancellationToken::new()).unwrap();
        assert_eq!(archive.index.stdlib_interface_version, 1);
        let container = xiao_xar::decode_xar(&fs::read(&archive.output).unwrap()).unwrap();
        let portable = xiao_bytecode::decode_xiaoc(&container.read_entry().unwrap()).unwrap();
        assert_eq!(
            portable.header.runtime_abi_min,
            decoded.header.runtime_abi_min
        );
        fs::remove_file(&source).unwrap();
        let result = dispatch(ProtocolRequest::RunArchive {
            request_id: format!("b2-a5-{name}"),
            protocol_version: xiao_driver::PROTOCOL_VERSION,
            core_version: xiao_driver::CORE_VERSION,
            locale: None,
            path: archive.output.display().to_string(),
            options: RunOptions {
                timeout_ms: Some(10_000),
                event_capacity: 4096,
                ..Default::default()
            },
            diagnostics: None,
            debug: false,
        });
        let ProtocolResponse::Result {
            exit_code, events, ..
        } = result
        else {
            panic!("{name}: {result:?}")
        };
        assert_eq!(exit_code, 0, "{name}: {events:?}");
        let stdout: String = events
            .iter()
            .filter(|event| event.kind == "intrinsic_output")
            .filter_map(|event| event.data.get("text").and_then(serde_json::Value::as_str))
            .collect();
        assert_eq!(stdout, golden, "{name} .xar");
        println!(
            "B2-A5-REPLAY case={name} format=xar artifact={} source_removed=true result=success",
            archive.output.display()
        );
    }
    assert!(repository().join("docs/module-registry.json").is_file());
}
