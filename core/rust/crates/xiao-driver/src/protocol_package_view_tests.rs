//! 包视图协议的隔离环境与静态读取探针。
use super::*;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use xiao_package::ENVIRONMENT_METADATA_FILE;

/// 为包视图测试创建单独的环境目录，永不使用用户的全局缓存。
struct Workspace {
    root: PathBuf,
    layout: CacheLayout,
}

impl Workspace {
    /// 为每个测试构造一个不共享的绝对路径。
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("xiao-repl-packages-{}-{stamp}", std::process::id()));
        fs::create_dir(&root).expect("create workspace");
        let layout = CacheLayout::from_xiao_home(Some(&root.join("home")), &root).expect("layout");
        Self { root, layout }
    }

    /// 写入已有环境映射，不通过协议入口创建或修改包。
    fn environment(&self, name: &str, packages: &[(&str, &str, &str)]) -> String {
        let path = if name == "global" {
            self.layout
                .global_environment_path(name)
                .expect("global path")
        } else {
            self.root.join(name)
        };
        fs::create_dir_all(&path).expect("environment");
        let mappings = packages.iter().map(|(root, version, digest)| json!({
            "package": { "name": root, "version": version,
                "source": { "source_id": format!("path:/{root}/{version}"), "alias": null, "display_name": root } },
            "object": { "digest": digest, "object_kind": "source" }
        })).collect::<Vec<_>>();
        fs::write(
            path.join(ENVIRONMENT_METADATA_FILE),
            json!({
                "metadata_version": 2, "logical_name": name, "directory_name": name,
                "config_fingerprint": "config", "toolchain_fingerprint": "toolchain",
                "target_fingerprint": "target", "environment_fingerprint": "environment",
                "lockfile_summary": null, "package_mappings": mappings
            })
            .to_string(),
        )
        .expect("metadata");
        path.to_string_lossy().into_owned()
    }

    /// 直接放置可解析源码，探针在真正运行时会抛错。
    fn source(&self, digest: &str, module: &str, text: &str) {
        let directory = self.layout.source_object_path(digest).expect("object path");
        fs::create_dir_all(&directory).expect("object directory");
        fs::write(directory.join(format!("{module}.xiao")), text).expect("source");
    }

    /// 构造一个只读查询响应。
    fn query(&self, active: Option<&str>, module: Option<&str>) -> ProtocolResponse {
        repl_packages_with_layout(
            "query".to_owned(),
            &self.layout,
            active.map(str::to_owned),
            module.map(str::to_owned),
        )
    }
}

impl Drop for Workspace {
    /// 只清理本测试刚创建且仍处于系统临时目录下的工作区。
    fn drop(&mut self) {
        if let (Ok(root), Ok(temp)) = (
            self.root.canonicalize(),
            std::env::temp_dir().canonicalize(),
        ) {
            if root.starts_with(temp)
                && root
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("xiao-repl-packages-"))
            {
                let _ = fs::remove_dir_all(root);
            }
        }
    }
}

#[test]
/// 激活环境与全局环境只读取各自映射，缺失的全局环境为空视图。
fn active_and_global_views_do_not_leak() {
    let workspace = Workspace::new();
    let ProtocolResponse::ReplPackagesResult { packages, .. } = workspace.query(None, None) else {
        panic!("missing global environment should be empty");
    };
    assert!(packages.is_empty());
    workspace.environment(
        "global",
        &[
            ("global_lib", "1.0.0", &"a".repeat(64)),
            ("not-identifier", "1.0.0", &"c".repeat(64)),
        ],
    );
    let active = workspace.environment("active", &[("local_lib", "1.0.0", &"b".repeat(64))]);
    let ProtocolResponse::ReplPackagesResult { packages, .. } = workspace.query(None, None) else {
        panic!("global view");
    };
    assert_eq!(
        packages
            .iter()
            .map(|entry| entry.root.as_str())
            .collect::<Vec<_>>(),
        ["global_lib"]
    );
    let ProtocolResponse::ReplPackagesResult { packages, .. } =
        workspace.query(Some(&active), None)
    else {
        panic!("active view");
    };
    assert_eq!(
        packages
            .iter()
            .map(|entry| entry.root.as_str())
            .collect::<Vec<_>>(),
        ["local_lib"]
    );
}

#[test]
/// 静态接口读取不会执行模块顶层的必然抛错探针。
fn interface_query_does_not_execute_module() {
    let workspace = Workspace::new();
    let digest = "c".repeat(64);
    let active = workspace.environment("active", &[("lib", "1.0.0", &digest)]);
    let text = "def greet(str name) -> str\n    return name\nraise ArithmeticError(code = \"MUST_NOT_RUN\")\n";
    workspace.source(&digest, "api", text);
    let ProtocolResponse::ReplPackagesResult {
        packages,
        interface,
        ..
    } = workspace.query(Some(&active), None)
    else {
        panic!("root enumeration");
    };
    assert_eq!(packages[0].root, "lib");
    assert!(interface.is_none());
    let response = workspace.query(Some(&active), Some("lib.api"));
    let ProtocolResponse::ReplPackagesResult {
        interface: Some(interface),
        ..
    } = response
    else {
        panic!("static query must not execute the raise probe: {response:?}");
    };
    assert_eq!(
        interface
            .exports
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["greet"]
    );
    assert!(
        interface.exports[0]
            .signature
            .as_deref()
            .is_some_and(|text| text.contains("str")),
        "{:?}",
        interface.exports
    );
    let registry = crate::packages::PackageRegistry::from_layout(
        &workspace.layout,
        std::path::Path::new(&active),
        false,
    )
    .expect("readonly package registry");
    assert!(registry.namespaces.members["lib"].contains("api"));
    assert!(registry.namespaces.members["lib.api"].contains("greet"));
    let mut context = crate::frontend::FrontendContext::host();
    context.package_registry = Some(registry);
    let compiled = crate::frontend::FrontendCompiler::new().compile(
        &crate::frontend::FrontendRequest::from_text("module = lib.api\n").with_context(context),
    );
    assert!(
        compiled.is_ok(),
        "static lookup must not run raise probe: {compiled:?}"
    );
    let executed = crate::run::run(&crate::run::DriverRequest::new(
        crate::frontend::FrontendRequest::from_text(text),
    ));
    assert_eq!(executed.exit_code(), ExitCode::RuntimeError);
}

#[test]
/// 同根包不依赖映射顺序选胜者，返回明确的两个候选与显式导入建议。
fn duplicate_roots_are_rejected() {
    let workspace = Workspace::new();
    let active = workspace.environment(
        "active",
        &[
            ("lib", "1.0.0", &"a".repeat(64)),
            ("lib", "2.0.0", &"b".repeat(64)),
        ],
    );
    let ProtocolResponse::Error { error, .. } = workspace.query(Some(&active), None) else {
        panic!("duplicate root must fail");
    };
    assert_eq!(error.code, REPL_ROOT_CONFLICT_CODE);
    assert_eq!(
        error.details["candidates"]
            .as_array()
            .expect("candidates")
            .len(),
        2
    );
    assert!(error.message.contains("import"));
}
