//! I4b 会话生命周期、模块缓存和协议串行执行规格。

use std::fs;
use std::io::{BufReader, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use xiao_driver::protocol::{
    CORE_VERSION, PROTOCOL_VERSION, ProtocolRequest, ProtocolResponse, ProtocolTarget, RunOptions,
    SourceIdentity, decode_frame, encode_frame, read_frame, serve, write_frame,
};
use xiao_driver::{
    CancellationToken, DriverOutcome, DriverRequest, FrontendContext, FrontendRequest,
    FrontendVmDriver,
};
use xiao_package::{CacheLayout, ENVIRONMENT_METADATA_FILE};
use xiao_vm::VmEvent;

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("xiao-i4b-{}-{stamp}", std::process::id()));
        fs::create_dir(&path).expect("workspace");
        Self(path)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(path, text).expect("source");
    }

    fn request(&self, source: &str) -> DriverRequest {
        let path = self.0.join("main.xiao");
        self.write("main.xiao", source);
        let mut context = FrontendContext::host();
        context.project_root = Some(self.0.clone());
        DriverRequest::new(FrontendRequest::from_text_at(source, path).with_context(context))
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if let (Ok(path), Ok(temp)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize()) {
            if path.starts_with(temp)
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("xiao-i4b-"))
            {
                let _ = fs::remove_dir_all(path);
            }
        }
    }
}

fn loaded(outcome: &DriverOutcome, module: &str) -> usize {
    let DriverOutcome::Executed(execution) = outcome else {
        panic!("expected execution: {outcome:?}");
    };
    execution
        .outcome
        .events
        .iter()
        .filter(
            |event| matches!(event, VmEvent::ModuleLoaded { module: loaded } if loaded == module),
        )
        .count()
}

#[test]
fn driver_reuses_module_initialization_but_not_request_bindings() {
    let workspace = Workspace::new();
    workspace.write("helper.xiao", "answer = 42\n");
    let mut driver = FrontendVmDriver::new();

    let first = driver.run(&workspace.request("import helper\nvalue = helper.answer\n"));
    assert!(first.is_success(), "{first:?}");
    assert_eq!(loaded(&first, "project:helper"), 1);

    let second = driver.run(&workspace.request("import helper\nvalue = helper.answer\n"));
    assert!(second.is_success(), "{second:?}");
    assert_eq!(loaded(&second, "project:helper"), 0);

    let isolated = driver.run(&workspace.request("value = value + 1\n"));
    assert!(
        matches!(isolated, DriverOutcome::Frontend(_)),
        "{isolated:?}"
    );
}

#[test]
fn main_singletons_restart_each_run_while_module_singletons_remain_loaded() {
    let workspace = Workspace::new();
    workspace.write("helper.xiao", "[State]\n    count = 7\n");
    let mut driver = FrontendVmDriver::new();
    let source = "[Local]\n    count = 3\nimport helper\nif Local.count != 3\n    raise ArithmeticError(code = \"LOCAL_TABLE\")\nif helper.State.count != 7\n    raise ArithmeticError(code = \"MODULE_TABLE\")\n";

    let first = driver.run(&workspace.request(source));
    assert!(first.is_success(), "{first:?}");
    assert_eq!(loaded(&first, "project:helper"), 1);

    let second = driver.run(&workspace.request(source));
    assert!(second.is_success(), "{second:?}");
    assert_eq!(loaded(&second, "project:helper"), 0);
}

#[test]
fn failed_module_run_discards_session_and_allows_retry() {
    let workspace = Workspace::new();
    workspace.write("helper.xiao", "raise ArithmeticError(code = \"BROKEN\")\n");
    let mut driver = FrontendVmDriver::new();

    let failed = driver.run(&workspace.request("import helper\n"));
    assert!(matches!(failed, DriverOutcome::Executed(_)), "{failed:?}");
    assert!(!failed.is_success());

    workspace.write("helper.xiao", "answer = 42\n");
    let retried = driver.run(&workspace.request("import helper\nvalue = helper.answer\n"));
    assert!(retried.is_success(), "{retried:?}");
    assert_eq!(loaded(&retried, "project:helper"), 1);
}

#[test]
fn early_cancellation_discards_existing_session_before_retry() {
    let workspace = Workspace::new();
    workspace.write("helper.xiao", "answer = 42\n");
    let mut driver = FrontendVmDriver::new();

    let first = driver.run(&workspace.request("import helper\nvalue = helper.answer\n"));
    assert!(first.is_success(), "{first:?}");
    assert_eq!(loaded(&first, "project:helper"), 1);

    let token = CancellationToken::new();
    token.cancel();
    let cancelled = driver.run(
        &workspace
            .request("import helper\nvalue = helper.answer\n")
            .with_cancellation(token),
    );
    assert!(!cancelled.is_success(), "{cancelled:?}");

    let retried = driver.run(&workspace.request("import helper\nvalue = helper.answer\n"));
    assert!(retried.is_success(), "{retried:?}");
    assert_eq!(loaded(&retried, "project:helper"), 1);
}

#[derive(Clone)]
struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl Write for CaptureWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("capture lock")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn run_request(request_id: &str, path: &Path, source: &str) -> ProtocolRequest {
    ProtocolRequest::Run {
        request_id: request_id.to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        target: ProtocolTarget::host(),
        optimization: Default::default(),
        source: SourceIdentity {
            module: "main".to_owned(),
            path: Some(path.display().to_string()),
            text: source.to_owned(),
        },
        options: RunOptions::default(),
    }
}

#[test]
fn protocol_session_serializes_runs_and_reuses_loaded_modules() {
    let workspace = Workspace::new();
    workspace.write("helper.xiao", "answer = 42\n");
    let main_path = workspace.0.join("main.xiao");
    let source = "import helper\nvalue = helper.answer\n";
    workspace.write("main.xiao", source);

    let requests = [
        ProtocolRequest::Hello {
            request_id: "hello".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
        },
        run_request("first", &main_path, source),
        run_request("second", &main_path, source),
        ProtocolRequest::Shutdown {
            request_id: "shutdown".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
        },
    ];
    let mut input = Vec::new();
    for request in requests {
        input.extend(encode_frame(&request).expect("request frame"));
    }

    let output = Arc::new(Mutex::new(Vec::new()));
    serve(Cursor::new(input), CaptureWriter(Arc::clone(&output))).expect("serve");

    let bytes = output.lock().expect("capture lock").clone();
    let mut reader = Cursor::new(bytes);
    let mut responses = Vec::new();
    while let Some(payload) = read_frame(&mut reader).expect("response frame") {
        responses.push(decode_frame::<ProtocolResponse>(&payload).expect("response"));
    }

    let run_responses: Vec<_> = responses
        .iter()
        .filter_map(|response| match response {
            ProtocolResponse::Result {
                request_id, events, ..
            } => Some((request_id, events)),
            _ => None,
        })
        .collect();
    assert_eq!(run_responses.len(), 2, "{responses:?}");
    assert_eq!(run_responses[0].0, "first");
    assert_eq!(run_responses[1].0, "second");
    assert_eq!(
        run_responses[0]
            .1
            .iter()
            .filter(
                |event| event.kind == "module_loaded" && event.data["module"] == "project:helper",
            )
            .count(),
        1
    );
    assert_eq!(
        run_responses[1]
            .1
            .iter()
            .filter(
                |event| event.kind == "module_loaded" && event.data["module"] == "project:helper",
            )
            .count(),
        0
    );
}

#[test]
fn protocol_session_does_not_reuse_project_modules_across_project_roots() {
    let first_workspace = Workspace::new();
    first_workspace.write("helper.xiao", "answer = 41\n");
    let second_workspace = Workspace::new();
    second_workspace.write("helper.xiao", "answer = 42\n");
    let first_path = first_workspace.0.join("main.xiao");
    let second_path = second_workspace.0.join("main.xiao");
    let source = "import helper\nvalue = helper.answer\n";
    first_workspace.write("main.xiao", source);
    second_workspace.write("main.xiao", source);

    let requests = [
        ProtocolRequest::Hello {
            request_id: "hello".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
        },
        run_request("first", &first_path, source),
        run_request("second", &second_path, source),
        ProtocolRequest::Shutdown {
            request_id: "shutdown".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
        },
    ];
    let mut input = Vec::new();
    for request in requests {
        input.extend(encode_frame(&request).expect("request frame"));
    }

    let output = Arc::new(Mutex::new(Vec::new()));
    serve(Cursor::new(input), CaptureWriter(Arc::clone(&output))).expect("serve");

    let bytes = output.lock().expect("capture lock").clone();
    let mut reader = Cursor::new(bytes);
    let mut responses = Vec::new();
    while let Some(payload) = read_frame(&mut reader).expect("response frame") {
        responses.push(decode_frame::<ProtocolResponse>(&payload).expect("response"));
    }

    let run_responses: Vec<_> = responses
        .iter()
        .filter_map(|response| match response {
            ProtocolResponse::Result {
                request_id, events, ..
            } => Some((request_id, events)),
            _ => None,
        })
        .collect();
    assert_eq!(run_responses.len(), 2, "{responses:?}");
    for (_, events) in run_responses {
        assert_eq!(
            events
                .iter()
                .filter(|event| {
                    event.kind == "module_loaded" && event.data["module"] == "project:helper"
                })
                .count(),
            1
        );
    }
}

fn exchange(
    input: &mut impl Write,
    output: &mut impl Read,
    request: ProtocolRequest,
) -> ProtocolResponse {
    write_frame(input, &request).expect("write request");
    let frame = read_frame(output)
        .expect("read response")
        .expect("response frame");
    decode_frame(&frame).expect("decode response")
}

#[test]
fn package_source_change_invalidates_the_live_core_session() {
    let workspace = Workspace::new();
    let home = workspace.0.join("home");
    let environment = workspace.0.join("active");
    let layout = CacheLayout::from_xiao_home(Some(&home), &workspace.0).expect("cache layout");
    fs::create_dir_all(&environment).expect("environment");
    let main_path = workspace.0.join("main.xiao");

    let install = |digest: &str, value: i32| {
        let object = layout.source_object_path(digest).expect("source object");
        fs::create_dir_all(&object).expect("object directory");
        fs::write(object.join("api.xiao"), format!("value = {value}\n")).expect("api source");
        let metadata = json!({
            "metadata_version": 2, "logical_name": "active", "directory_name": "active",
            "config_fingerprint": "config", "toolchain_fingerprint": "toolchain",
            "target_fingerprint": "target", "environment_fingerprint": "unchanged",
            "lockfile_summary": null,
            "package_mappings": [{
                "package": { "name": "lib", "version": "1.0.0",
                    "source": { "source_id": "path:/lib", "alias": null,
                        "display_name": "lib" } },
                "object": { "digest": digest, "object_kind": "source" }
            }]
        });
        fs::write(
            environment.join(ENVIRONMENT_METADATA_FILE),
            metadata.to_string(),
        )
        .expect("metadata");
    };

    install(&"a".repeat(64), 7);
    let mut core = Command::new(env!("CARGO_BIN_EXE_xiao-core"))
        .env("XIAO_HOME", home)
        .env("XIAO_ACTIVE_ENV", &environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn core");
    let mut input = core.stdin.take().expect("core stdin");
    let mut output = BufReader::new(core.stdout.take().expect("core stdout"));

    let hello = exchange(
        &mut input,
        &mut output,
        ProtocolRequest::Hello {
            request_id: "hello".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
        },
    );
    assert!(matches!(
        hello,
        ProtocolResponse::Hello { accepted: true, .. }
    ));

    for (request_id, digest, value) in [("first", "a".repeat(64), 7), ("second", "b".repeat(64), 9)]
    {
        if request_id == "second" {
            install(&digest, value);
        }
        let source = format!(
            "first = lib.api.value\nif first != {value}\n    raise ArithmeticError(code = \"BAD_VALUE\")\n"
        );
        workspace.write("main.xiao", &source);
        let response = exchange(
            &mut input,
            &mut output,
            run_request(request_id, &main_path, &source),
        );
        let ProtocolResponse::Result {
            exit_code, events, ..
        } = &response
        else {
            panic!("expected run result: {response:?}");
        };
        assert_eq!(*exit_code, 0, "{response:?}");
        assert_eq!(
            events
                .iter()
                .filter(|event| {
                    event.kind == "module_loaded" && event.data["module"] == "package:lib.api"
                })
                .count(),
            1,
            "{response:?}"
        );
    }

    let shutdown = exchange(
        &mut input,
        &mut output,
        ProtocolRequest::Shutdown {
            request_id: "shutdown".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
        },
    );
    assert!(matches!(shutdown, ProtocolResponse::Shutdown { .. }));
    drop(input);
    assert!(core.wait().expect("core exit").success());
}
