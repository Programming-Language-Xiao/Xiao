//! B2 单入口完整示例、真实编码与归档回归；由维护双精确入口共用。

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use xiao_driver::{
    ArchiveBuildRequest, CancellationToken, DriverRequest, FrontendCompiler, FrontendRequest,
    FrontendVmDriver, ProtocolRequest, ProtocolResponse, RunOptions, build_archive, dispatch,
};
use xiao_vm::VmEvent;

use crate::b2_a as native_support;

/// 仓库定位独立于进程工作目录；完整示例与planned教程逐字一致。
fn fixture() -> (FrontendRequest, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let path = root.join("tests/fixtures/docs/numeric/b2_explicit_divide.xiao");
    let source = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    let golden = fs::read_to_string(path.with_extension("stdout"))
        .unwrap()
        .replace("\r\n", "\n");
    let page = fs::read_to_string(root.join("docs/UseDocs/language/basics/types/conversions.md"))
        .unwrap()
        .replace("\r\n", "\n");
    assert!(page.contains(&format!("```xiao\n{source}```")));
    assert!(page.contains(&format!("```text\n{golden}```")));
    assert!(page.contains("B2显式精度除法（planned）"));
    (FrontendRequest::from_text_at(source, path), golden)
}

/// 生产VM、.xiaoc及删除源码后的真实.xar逐字执行同一份冻结示例。
pub fn vm_suite() {
    let (request, golden) = fixture();
    let artifact = FrontendCompiler::new().compile(&request).unwrap();
    let outcome = FrontendVmDriver::new().run_artifact(
        &artifact,
        &DriverRequest::new(request.clone())
            .with_timeout(Duration::from_secs(15))
            .with_event_capacity(1024),
    );
    assert!(outcome.is_success(), "{outcome:?}");
    let stdout: String = outcome
        .as_executed()
        .unwrap()
        .events()
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(stdout, golden);
    let program = xiao_bytecode::lower_program(&artifact.ir);
    let bytes = xiao_bytecode::encode_xiaoc(
        &program,
        xiao_bytecode::XiaocMetadata::new("b2-explicit-divide"),
    )
    .unwrap();
    let decoded = xiao_bytecode::decode_xiaoc(&bytes).unwrap();
    assert_eq!(
        u64::from(decoded.header.runtime_abi_min),
        xiao_runtime_abi::ABI_ENCODED_VERSION
    );
    let result = xiao_vm::run_xiaoc_production(&bytes, Default::default(), 1024, None).unwrap();
    assert!(result.result.is_success(), "{result:?}");
    let stdout: String = result
        .events
        .iter()
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(stdout, golden);
    let directory = std::env::temp_dir().join(format!(
        "xiao-b2-divide-archive-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    let source = directory.join("main.xiao");
    fs::write(&source, request.source.text()).unwrap();
    let mut build = ArchiveBuildRequest::new(&source);
    build.cache_root = Some(directory.join("cache"));
    let archive = build_archive(&build, &CancellationToken::new()).unwrap();
    fs::remove_file(&source).unwrap();
    let replay = dispatch(ProtocolRequest::RunArchive {
        request_id: "b2-divide".into(),
        protocol_version: xiao_driver::PROTOCOL_VERSION,
        core_version: xiao_driver::CORE_VERSION,
        locale: None,
        path: archive.output.display().to_string(),
        options: RunOptions {
            timeout_ms: Some(15000),
            event_capacity: 1024,
            ..Default::default()
        },
        diagnostics: None,
        debug: false,
    });
    let ProtocolResponse::Result {
        exit_code, events, ..
    } = replay
    else {
        panic!("{replay:?}")
    };
    assert_eq!(exit_code, 0, "{events:?}");
    let stdout: String = events
        .iter()
        .filter(|event| event.kind == "intrinsic_output")
        .filter_map(|event| event.data.get("text").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(stdout, golden);
    println!(
        "B2-B2-DIVIDE backend=vm formats=source,xiaoc,xar intrinsic=33 source_removed=true result=success"
    );
}

/// 使用现有受控原生驱动，在独立PID/时间戳路径执行O0/O3，不改既有样例集合。
pub fn native_suite() {
    let _ = fixture();
    native_support::native_fixtures(&["b2_explicit_divide"]);
    println!("B2-B2-DIVIDE backend=native optimization=0,3 intrinsic=33 result=success");
}
