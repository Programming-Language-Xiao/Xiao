//! 18B 真实归档构建、精确能力闭包、缓存破坏和项目副作用回归。

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use xiao_artifacts::{ArtifactStore, ObjectKind};
use xiao_driver::{ArchiveBuildRequest, CancellationToken, build_archive};
use xiao_intrinsics::{CAPABILITY_FILESYSTEM, CAPABILITY_PROCESS};
use xiao_optimizer::OptimizationLevel;
use xiao_xar::{XarRunOptions, decode_xar, run_archive};

/// 并行用例共享的临时目录序列，避免名称碰撞。
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 项目源码与全局缓存物理分离的归档测试工作区。
struct Fixture {
    base: PathBuf,
    root: PathBuf,
    cache: PathBuf,
}

impl Fixture {
    /// 创建本用例独占的临时工作区。
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "xiao-18b-archive-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let root = base.join("project");
        fs::create_dir_all(&root).unwrap();
        Self {
            cache: base.join("cache"),
            base,
            root,
        }
    }
    /// 写入明确指定的项目夹具，不收集其他目录内容。
    fn write(&self, path: &str, source: &str) -> PathBuf {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        path
    }
    /// 生成指向隔离缓存的真实归档构建请求。
    fn request(&self, input: &str) -> ArchiveBuildRequest {
        let mut request = ArchiveBuildRequest::new(self.root.join(input));
        request.cache_root = Some(self.cache.clone());
        request
    }
}

impl Drop for Fixture {
    /// 仅清理本用例创建的目录或核心进程。
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[test]
/// O0–O3 的归档输出一致，字节码只存在于全局缓存。
fn script_builds_runs_and_keeps_xiaoc_in_global_cache() {
    let fixture = Fixture::new();
    fixture.write("hello.xiao", "print(\"archive\")\n");
    for level in [
        OptimizationLevel::O0,
        OptimizationLevel::O1,
        OptimizationLevel::O2,
        OptimizationLevel::O3,
    ] {
        let mut request = fixture.request("hello.xiao");
        request.optimization_level = level;
        let result = build_archive(&request, &CancellationToken::new()).unwrap();
        assert_eq!(
            result.output,
            fixture.root.canonicalize().unwrap().join("hello.xar")
        );
        assert!(result.index.stdlib_modules.is_empty());
        let bytes = fs::read(&result.output).unwrap();
        let archive = decode_xar(&bytes).unwrap();
        assert_eq!(archive.index().entries.len(), 1);
        let outcome = run_archive(&bytes, XarRunOptions::default()).unwrap();
        assert!(outcome.result.is_success(), "{outcome:?}");
        assert!(outcome.events.iter().any(|event| matches!(event, xiao_vm::VmEvent::IntrinsicOutput { text } if text == "archive\n")));
        assert!(!fixture.root.join("hello.xiaoc").exists());
        assert_eq!(
            ArtifactStore::open(&fixture.cache)
                .unwrap()
                .scan(ObjectKind::Xiaoc)
                .unwrap()
                .len(),
            level.as_u8() as usize + 1
        );
    }
}

#[test]
/// 官方模块导入和别名只增加实际调用 intrinsic 的能力。
fn imported_stdlib_functions_have_precise_capabilities() {
    let fixture = Fixture::new();
    for (source, expected) in [
        (
            "from stdlib.os import open_file\nf = open_file(\"unused\")\n",
            CAPABILITY_FILESYSTEM,
        ),
        (
            "from stdlib.os import spawn\np = spawn(\"unused\")\n",
            CAPABILITY_PROCESS,
        ),
        (
            "from stdlib.os import open_file, spawn\nf = open_file(\"unused\")\np = spawn(\"unused\")\n",
            CAPABILITY_FILESYSTEM | CAPABILITY_PROCESS,
        ),
        (
            "from stdlib.os import open_file, spawn\ndef unused()\n    p = spawn(\"unused\")\nf = open_file(\"unused\")\n",
            CAPABILITY_FILESYSTEM,
        ),
        (
            "import stdlib.os as os\nf = os.open_file(\"unused\")\n",
            CAPABILITY_FILESYSTEM,
        ),
        (
            "from stdlib.os import open_file as read\ncopy = (read)\nf = copy(\"unused\")\n",
            CAPABILITY_FILESYSTEM,
        ),
    ] {
        fixture.write("main.xiao", source);
        let result =
            build_archive(&fixture.request("main.xiao"), &CancellationToken::new()).unwrap();
        assert_eq!(result.index.stdlib_modules, ["stdlib.os"]);
        assert_eq!(result.index.required_capabilities, expected, "{source}");
        assert!(
            result
                .index
                .entries
                .iter()
                .all(|entry| !entry.module.starts_with("stdlib"))
        );
        let archive = decode_xar(&fs::read(result.output).unwrap()).unwrap();
        let entry = xiao_bytecode::decode_xiaoc(&archive.read_entry().unwrap()).unwrap();
        assert!(
            entry
                .program
                .functions
                .iter()
                .all(|function| function.name != "spawn" && function.name != "open_file"),
            "官方函数不能复制进入口对象"
        );
    }
}

#[test]
/// 真实 `build -xar` 不复制官方 `set` 模块，方法调用由宿主模块加载器完成。
fn archive_build_runs_set_methods_through_host_module() {
    let fixture = Fixture::new();
    fixture.write(
        "main.xiao",
        "values = {1}\nvalues.add(2)\nprint(values.union({3}).size())\n",
    );
    let result = build_archive(&fixture.request("main.xiao"), &CancellationToken::new()).unwrap();
    assert_eq!(result.index.stdlib_modules, ["stdlib.set"]);
    let bytes = fs::read(&result.output).unwrap();
    let archive = decode_xar(&bytes).unwrap();
    let entry = xiao_bytecode::decode_xiaoc(&archive.read_entry().unwrap()).unwrap();
    assert!(
        entry
            .program
            .functions
            .iter()
            .all(|function| !function.name.starts_with("__xiao_stdlib_set_")),
        "外部标准库函数不能复制进入口对象"
    );
    let response = xiao_driver::dispatch(xiao_driver::ProtocolRequest::RunArchive {
        request_id: "set-method-archive".to_owned(),
        protocol_version: xiao_driver::PROTOCOL_VERSION,
        core_version: xiao_driver::CORE_VERSION,
        locale: Some("zh-CN".to_owned()),
        path: result.output.display().to_string(),
        options: Default::default(),
        diagnostics: None,
        debug: false,
    });
    let xiao_driver::ProtocolResponse::Result {
        exit_code, events, ..
    } = response
    else {
        panic!("归档协议应返回方法执行结果");
    };
    assert_eq!(exit_code, 0);
    assert!(events.iter().any(|event| {
        event.kind == "intrinsic_output"
            && event.data.get("text").and_then(serde_json::Value::as_str) == Some("3\n")
    }));
}

#[test]
/// 项目归档仅携带可达模块与声明资源，删除源码后仍能运行。
fn config_uses_unique_main_and_packages_only_reachable_local_modules_and_resources() {
    let fixture = Fixture::new();
    let config = fixture.write("config.xiao", "[project]\nname = \"app\"\nversion = \"0.1.0\"\n[resources]\n\"data/message\" = \"message.txt\"\n");
    fixture.write("app.xiao", "[main]\nfrom helper import greet\ngreet()\n");
    fixture.write("helper.xiao", "def greet()\n    print(\"module\")\n");
    fixture.write("unused.xiao", "def unused()\n    print(\"unused\")\n");
    fixture.write("message.txt", "resource");
    fixture.write("secret.txt", "not declared");
    let before = fs::metadata(&config).unwrap().modified().unwrap();
    let result = build_archive(&fixture.request("config.xiao"), &CancellationToken::new()).unwrap();
    assert_eq!(result.output.file_name().unwrap(), "config.xar");
    assert!(
        result
            .index
            .entries
            .iter()
            .any(|entry| entry.module == "project:helper")
    );
    assert!(
        !result
            .index
            .entries
            .iter()
            .any(|entry| entry.module == "project:unused")
    );
    assert!(
        result
            .index
            .entries
            .iter()
            .any(|entry| entry.logical_path == "data/message")
    );
    assert!(
        !result
            .index
            .entries
            .iter()
            .any(|entry| entry.logical_path.contains("secret"))
    );
    let bytes = fs::read(result.output).unwrap();
    fs::remove_file(fixture.root.join("helper.xiao")).unwrap();
    let outcome = run_archive(&bytes, XarRunOptions::default()).unwrap();
    assert!(outcome.result.is_success(), "{outcome:?}");
    assert!(outcome.events.iter().any(
        |event| matches!(event, xiao_vm::VmEvent::IntrinsicOutput { text } if text == "module\n")
    ));
    assert_eq!(fs::metadata(config).unwrap().modified().unwrap(), before);
}

#[test]
/// 索引命中的对象损坏时拒绝构建，保留此前完整输出。
fn indexed_corrupt_hit_is_rejected_without_replacing_output() {
    let fixture = Fixture::new();
    fixture.write("main.xiao", "print(\"cached\")\n");
    let request = fixture.request("main.xiao");
    let first = build_archive(&request, &CancellationToken::new()).unwrap();
    let bytes = fs::read(&first.output).unwrap();
    let hit = build_archive(&request, &CancellationToken::new()).unwrap();
    assert!(hit.modules.iter().all(|module| module.hit));
    let store = ArtifactStore::open(&fixture.cache).unwrap();
    let path = store.object_path(ObjectKind::Xiaoc, hit.modules[0].digest);
    #[cfg(windows)]
    #[allow(clippy::permissions_set_readonly_false)]
    // 仅 Windows 的只读属性；Unix 分支显式设定 0600。
    {
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&path, permissions).unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    fs::write(path, b"corrupted hit").unwrap();
    assert!(build_archive(&request, &CancellationToken::new()).is_err());
    assert_eq!(fs::read(first.output).unwrap(), bytes);
}

#[test]
/// 入口歧义、第三方依赖和缺失资源均在归档写出前拒绝。
fn missing_or_multiple_main_third_party_and_bad_resources_leave_no_archive() {
    let fixture = Fixture::new();
    fixture.write(
        "config.xiao",
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n",
    );
    assert!(
        build_archive(&fixture.request("config.xiao"), &CancellationToken::new())
            .unwrap_err()
            .message
            .contains("缺少")
    );
    fixture.write("one.xiao", "[main]\nprint(1)\n");
    fixture.write("two.xiao", "[main]\nprint(2)\n");
    assert!(
        build_archive(&fixture.request("config.xiao"), &CancellationToken::new())
            .unwrap_err()
            .message
            .contains("多个")
    );
    fs::remove_file(fixture.root.join("two.xiao")).unwrap();
    fixture.write(
        "config.xiao",
        "[dependencies]\nexternal = { version = \"1.0.0\", path = \"deps/external\" }\n",
    );
    let error =
        build_archive(&fixture.request("config.xiao"), &CancellationToken::new()).unwrap_err();
    assert_eq!(error.code, "X11-PROTOCOL-006");
    fixture.write(
        "config.xiao",
        "[resources]\n\"missing\" = \"missing.txt\"\n",
    );
    let error =
        build_archive(&fixture.request("config.xiao"), &CancellationToken::new()).unwrap_err();
    assert_eq!(error.code, "X17-XAR-001");
    assert!(!fixture.root.join("config.xar").exists());
}

#[test]
/// 调试符号独立存储，源码正文不会因调试开关而自动进入归档。
fn debug_has_separate_symbols_and_explicit_output() {
    let fixture = Fixture::new();
    fixture.write("main.xiao", "print(\"debug\")\n");
    let mut request = fixture.request("main.xiao");
    request.output = Some(fixture.base.join("output/custom.xar"));
    request.debug = true;
    let result = build_archive(&request, &CancellationToken::new()).unwrap();
    assert!(result.index.debug_activation);
    assert!(
        result
            .index
            .entries
            .iter()
            .any(|entry| entry.object_kind == ObjectKind::Debug)
    );
    assert!(
        !result
            .index
            .entries
            .iter()
            .any(|entry| entry.object_kind == ObjectKind::Source)
    );
    let archive = decode_xar(&fs::read(result.output).unwrap()).unwrap();
    let entry = xiao_bytecode::decode_xiaoc(&archive.read_entry().unwrap()).unwrap();
    assert!(entry.sections.iter().all(|section| !matches!(
        section.kind,
        Some(
            xiao_bytecode::XiaocSectionKind::Source | xiao_bytecode::XiaocSectionKind::DebugSymbols
        )
    )));
    assert!(!fixture.root.join("main.xar").exists());
}

#[test]
/// 跟踪函数实参和本地重导出，未调用函数不扩大要求闭包。
fn closure_tracks_calls_keywords_reexports_and_empty_requirements() {
    let fixture = Fixture::new();
    for source in [
        "from stdlib.os import open_file, spawn\ndef invoke(action, ignored) -> file\n    return action(\"unused\")\nf = invoke(ignored = spawn, action = open_file)\n",
        "def read() -> file\n    from stdlib.os import open_file\n    return open_file(\"unused\")\nf = read()\n",
        "from stdlib.os import open_file\ndef read() -> file\n    return open_file(\"unused\")\nf = read()\n",
        "import stdlib.os as os\ndef read() -> file\n    return os.open_file(\"unused\")\nf = read()\n",
        "from helper import read\nf = read(\"unused\")\n",
    ] {
        fixture.write("leaf.xiao", "def read(str path) -> file\n    from stdlib.os import open_file\n    return open_file(path)\n");
        fixture.write("helper.xiao", "from leaf import read\n");
        fixture.write("main.xiao", source);
        let result =
            build_archive(&fixture.request("main.xiao"), &CancellationToken::new()).unwrap();
        assert_eq!(
            result.index.required_capabilities, CAPABILITY_FILESYSTEM,
            "{source}"
        );
        assert_eq!(result.index.stdlib_modules, ["stdlib.os"]);
    }
    fixture.write("main.xiao", "def unused() -> process\n    from stdlib.os import spawn\n    return spawn(\"unused\")\nvalue = 1\n");
    let result = build_archive(&fixture.request("main.xiao"), &CancellationToken::new()).unwrap();
    assert!(result.index.stdlib_modules.is_empty());
    assert_eq!(result.index.required_capabilities, 0);
}

/// 使用实际协议进程隔离 XIAO_HOME，不修改父进程环境或用户缓存。
struct CoreProcess {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: std::io::BufReader<std::process::ChildStdout>,
}

impl CoreProcess {
    /// 在隔离缓存环境中启动真实核心并完成版本握手。
    fn new(fixture: &Fixture) -> Self {
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_xiao-core"))
            .env("XIAO_HOME", fixture.base.join("home"))
            .env_remove("XIAO_ACTIVE_ENV")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut core = Self {
            child,
            input,
            output,
        };
        let hello = core.call(xiao_driver::ProtocolRequest::Hello {
            request_id: "hello".to_owned(),
            protocol_version: xiao_driver::PROTOCOL_VERSION,
            core_version: xiao_driver::CORE_VERSION,
        });
        assert!(matches!(
            hello,
            xiao_driver::ProtocolResponse::Hello { accepted: true, .. }
        ));
        core
    }

    /// 通过实际长度前缀协议同步发送请求并读取关联响应。
    fn call(&mut self, request: xiao_driver::ProtocolRequest) -> xiao_driver::ProtocolResponse {
        xiao_driver::write_frame(&mut self.input, &request).unwrap();
        let bytes = xiao_driver::read_frame(&mut self.output).unwrap().unwrap();
        xiao_driver::decode_frame(&bytes).unwrap()
    }
}

impl Drop for CoreProcess {
    /// 仅清理本用例创建的目录或核心进程。
    fn drop(&mut self) {
        let _ = self.call(xiao_driver::ProtocolRequest::Shutdown {
            request_id: "shutdown".to_owned(),
            protocol_version: xiao_driver::PROTOCOL_VERSION,
            core_version: xiao_driver::CORE_VERSION,
        });
        let _ = self.child.wait();
    }
}

#[test]
/// 实际进程验证归档构建、宿主标准库、源码缓存和 REPL 隔离。
fn served_protocol_builds_runs_stdlib_and_keeps_repl_out_of_global_cache() {
    use xiao_driver::{CORE_VERSION, PROTOCOL_VERSION, ProtocolRequest, ProtocolResponse};
    let fixture = Fixture::new();
    fixture.write(
        "config.xiao",
        "[project]\nname = \"protocol\"\nversion = \"0.1.0\"\n",
    );
    fixture.write(
        "app.xiao",
        "[main]\nfrom helper import measure\nprint(measure(\"hello\"))\n",
    );
    fixture.write(
        "helper.xiao",
        "from stdlib.text import length\ndef measure(str value) -> int\n    return length(value)\n",
    );
    let mut core = CoreProcess::new(&fixture);
    let baseline = core.call(ProtocolRequest::Run {
        request_id: "baseline".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        language_version: "0.1.0".to_owned(),
        runtime_version: "0.1.0".to_owned(),
        locale: None,
        target: xiao_driver::ProtocolTarget::host(),
        optimization: Default::default(),
        source: xiao_driver::SourceIdentity {
            module: "app".to_owned(),
            path: Some(fixture.root.join("app.xiao").display().to_string()),
            text: fs::read_to_string(fixture.root.join("app.xiao")).unwrap(),
        },
        options: Default::default(),
    });
    let ProtocolResponse::Result {
        exit_code: 0,
        events: baseline_events,
        ..
    } = baseline
    else {
        panic!("{baseline:?}")
    };
    let response = core.call(ProtocolRequest::BuildArchive {
        request_id: "build".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        locale: Some("zh-CN".to_owned()),
        path: fixture.root.join("config.xiao").display().to_string(),
        output: None,
        optimization: xiao_driver::OptimizationConfig {
            level: 2,
            debug: false,
            diagnostics: None,
        },
    });
    let ProtocolResponse::Result {
        exit_code: 0,
        operation,
        value: Some(value),
        ..
    } = response
    else {
        panic!("{response:?}")
    };
    assert_eq!(operation, "build_archive");
    let summary: serde_json::Value = serde_json::from_str(&value.value).unwrap();
    assert_eq!(
        summary["stdlib_modules"],
        serde_json::json!(["stdlib.text"])
    );
    let archive = summary["output"].as_str().unwrap().to_owned();
    fs::remove_file(fixture.root.join("helper.xiao")).unwrap();
    let response = core.call(ProtocolRequest::RunArchive {
        request_id: "archive".to_owned(),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
        locale: Some("zh-CN".to_owned()),
        path: archive,
        options: Default::default(),
        diagnostics: None,
        debug: false,
    });
    let ProtocolResponse::Result {
        exit_code: 0,
        events,
        ..
    } = response
    else {
        panic!("{response:?}")
    };
    assert!(events.iter().any(|event| event.kind == "intrinsic_output"
        && event.data.get("text") == Some(&serde_json::json!("5\n"))));
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == "intrinsic_output")
            .collect::<Vec<_>>(),
        baseline_events
            .iter()
            .filter(|event| event.kind == "intrinsic_output")
            .collect::<Vec<_>>()
    );
    let cache = fixture.base.join("home/cache");
    fixture.write(
        "helper.xiao",
        "from stdlib.text import length\ndef measure(str value) -> int\n    return length(value)\n",
    );
    let store = ArtifactStore::open(&cache).unwrap();
    let before = store.scan(ObjectKind::Xiaoc).unwrap();
    let source = fixture.write("single.xiao", "print(\"source\")\n");
    for path in [Some(source.display().to_string()), None] {
        let response = core.call(ProtocolRequest::Run {
            request_id: "source".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            language_version: "0.1.0".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            locale: None,
            target: xiao_driver::ProtocolTarget::host(),
            optimization: Default::default(),
            source: xiao_driver::SourceIdentity {
                module: "single".to_owned(),
                path: path.clone(),
                text: "print(\"source\")\n".to_owned(),
            },
            options: Default::default(),
        });
        let ProtocolResponse::Result {
            exit_code: 0,
            cache: Some(observation),
            ..
        } = response
        else {
            panic!("{response:?}")
        };
        assert_eq!(
            observation.status,
            if path.is_some() { "written" } else { "session" }
        );
        assert_eq!(
            store.scan(ObjectKind::Xiaoc).unwrap().len(),
            before.len() + 1
        );
        assert!(!fixture.root.join("single.xiaoc").exists());
    }
}

#[test]
/// 锁文件使用完整校验器且只读，畸形结构不能被误当作无依赖。
fn lockfile_uses_authoritative_validation_and_is_never_rewritten() {
    let fixture = Fixture::new();
    let config_text = "[project]\nname = \"locked\"\nversion = \"0.1.0\"\n";
    fixture.write("config.xiao", config_text);
    fixture.write("app.xiao", "[main]\nprint(\"locked\")\n");
    let resolution = xiao_package::resolve_project(&fixture.root);
    let config = xiao_config::parse_config_text(config_text).unwrap();
    let home = fixture.base.join("lock-home");
    let layout = xiao_package::CacheLayout::from_xiao_home(Some(&home), &fixture.base).unwrap();
    let cache = xiao_package::CacheStore::open(layout).unwrap();
    let lock = xiao_package::build_lockfile(&resolution.graph, &config, &cache).unwrap();
    let lock_path = fixture.write("xiao.lock.json", &lock.to_json());
    let original = fs::read(&lock_path).unwrap();
    let modified = fs::metadata(&lock_path).unwrap().modified().unwrap();
    let result = build_archive(&fixture.request("config.xiao"), &CancellationToken::new()).unwrap();
    assert_eq!(
        result.index.dependency_lock_digest,
        xiao_artifacts::Digest256::of_bytes(&original).as_hex()
    );
    assert_eq!(fs::read(&lock_path).unwrap(), original);
    assert_eq!(
        fs::metadata(&lock_path).unwrap().modified().unwrap(),
        modified
    );
    let mut overwrite = fixture.request("config.xiao");
    overwrite.output = Some(lock_path.clone());
    assert!(build_archive(&overwrite, &CancellationToken::new()).is_err());
    assert_eq!(fs::read(&lock_path).unwrap(), original);
    assert_eq!(
        fs::metadata(&lock_path).unwrap().modified().unwrap(),
        modified
    );
    fs::write(&lock_path, "{\"packages\":{}}").unwrap();
    let error =
        build_archive(&fixture.request("config.xiao"), &CancellationToken::new()).unwrap_err();
    assert_eq!(error.code, xiao_package::LOCKFILE_INVALID_CODE);
    assert!(result.output.is_file());
}

#[test]
/// 分支中的未知调用目标不能被另一条分支的已知目标掩盖。
fn review_mixed_known_and_unresolved_targets_cannot_omit_capabilities() {
    let fixture = Fixture::new();
    fixture.write("helper.xiao", "def read() -> none\n    from stdlib.os import open_file\n    value = open_file(\"unused\")\ndef launch() -> none\n    from stdlib.os import spawn\n    value = spawn(\"unused\")\n");
    fixture.write("main.xiao", "from helper import read, launch\nfrom stdlib.text import length\naction = read\nflag = length(\"x\") > 0\nif flag\n    callbacks = [launch]\n    action = callbacks[0]\naction()\n");
    match build_archive(&fixture.request("main.xiao"), &CancellationToken::new()) {
        Ok(result) => assert_ne!(
            result.index.required_capabilities & CAPABILITY_PROCESS,
            0,
            "不允许成功写出漏报 process 的归档"
        ),
        Err(error) => {
            assert!(
                error.message.contains("无法静态确定"),
                "只允许明确拒绝未解析的调用闭包：{error}"
            );
            assert!(!fixture.root.join("main.xar").exists());
        }
    }
}

#[test]
/// 配置入口构建的输出不得覆盖真正的入口、依赖源码或配置资源。
fn review_output_cannot_replace_any_archive_input() {
    for output in [
        "app.xiao",
        "helper.xiao",
        "unused.xiao",
        "config.xiao",
        "message.txt",
    ] {
        let fixture = Fixture::new();
        fixture.write("config.xiao", "[project]\nname = \"review\"\nversion = \"0.1.0\"\n[resources]\n\"message\" = \"message.txt\"\n");
        fixture.write("app.xiao", "[main]\nfrom helper import work\nwork()\n");
        fixture.write("helper.xiao", "def work() -> none\n    print(\"review\")\n");
        fixture.write(
            "unused.xiao",
            "def unused() -> none\n    print(\"unused\")\n",
        );
        fixture.write("message.txt", "resource");
        let output = fixture.root.join(output);
        let before = fs::read(&output).unwrap();
        let modified = fs::metadata(&output).unwrap().modified().unwrap();
        let mut request = fixture.request("config.xiao");
        request.output = Some(output.clone());
        assert!(
            build_archive(&request, &CancellationToken::new()).is_err(),
            "输出不得覆盖构建输入：{}",
            output.display()
        );
        assert_eq!(fs::read(&output).unwrap(), before);
        assert_eq!(fs::metadata(&output).unwrap().modified().unwrap(), modified);
    }
}

#[test]
/// 输出不能替换缓存索引或写入缓存内部，失败不改变缓存内容与修改时间。
fn review_archive_output_cannot_corrupt_its_cache() {
    let fixture = Fixture::new();
    fixture.write("main.xiao", "print(\"cache\")\n");
    let mut request = fixture.request("main.xiao");
    build_archive(&request, &CancellationToken::new()).unwrap();
    let index = fixture.cache.join("global.index.pb");
    let before = fs::read(&index).unwrap();
    let modified = fs::metadata(&index).unwrap().modified().unwrap();
    for output in [
        index.clone(),
        fixture.cache.join("../cache/global.index.pb"),
        fixture.cache.join("new.xar"),
    ] {
        request.output = Some(output);
        let error = build_archive(&request, &CancellationToken::new()).unwrap_err();
        assert!(error.message.contains("缓存"), "{error}");
        assert_eq!(fs::read(&index).unwrap(), before);
        assert_eq!(fs::metadata(&index).unwrap().modified().unwrap(), modified);
        assert!(!fixture.cache.join("new.xar").exists());
    }
}
