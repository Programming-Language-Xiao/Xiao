//! 测量输入与产物内容身份；拒绝不属于仓库的stdlib及中途源码修改。

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 固定源码与依赖锁的逐文件内容摘要。
#[derive(Clone, Debug, Serialize)]
pub struct Identity {
    /// 实际测量仓库的绝对路径。
    pub repository: String,
    /// 源码、夹具与依赖锁的逐文件原始SHA-256。
    pub files: BTreeMap<String, String>,
    /// 实际宿主目标三元组。
    pub target: String,
    /// 编译器完整版本和宿主信息。
    pub rustc: String,
    /// 实际Rust与LLVM工具文件摘要及版本。
    pub tools: BTreeMap<String, Tool>,
    /// 显式注入的release Runtime静态库身份。
    pub runtime: Option<File>,
    /// 本次新建测量进程使用的控制器文件身份。
    pub worker: File,
}

/// 实际磁盘文件身份，不使用文件名代替内容。
#[derive(Clone, Debug, Serialize)]
pub struct File {
    /// 执行时实际读取文件的绝对路径。
    pub path: String,
    /// 实际文件字节数。
    pub bytes: u64,
    /// 原始文件内容的SHA-256。
    pub sha256: String,
}

/// 实际编译器文件及其完整版本输出；工具名不能代替二进制身份。
#[derive(Clone, Debug, Serialize)]
pub struct Tool {
    /// 版本命令实际执行的工具二进制身份。
    pub file: File,
    /// 工具完整版本输出，不从路径猜测版本。
    pub version: String,
}

/// 在实际二进制上查询版本，同时绑定它的内容。
fn tool(path: &Path, version_argument: &str) -> Tool {
    let path = path.canonicalize().expect("测量工具路径必须存在");
    let output = std::process::Command::new(&path)
        .arg(version_argument)
        .output()
        .unwrap();
    assert!(output.status.success());
    Tool {
        file: file(&path),
        version: String::from_utf8(output.stdout).unwrap(),
    }
}

/// SHA-256与文件实际大小。
pub fn file(path: &Path) -> File {
    let raw = std::fs::read(path).unwrap();
    File {
        path: path.display().to_string(),
        bytes: raw.len() as u64,
        sha256: xiao_artifacts::Digest256::of_bytes(&raw).to_string(),
    }
}

/// 按稳定路径顺序收集输入；排除构建目录、依赖目录和Git内部数据。
fn walk(repository: &Path, path: &Path, files: &mut BTreeMap<String, String>) {
    if path.is_dir() {
        let mut children: Vec<_> = std::fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        children.sort();
        for child in children {
            if child.file_name().is_some_and(|name| {
                matches!(name.to_str(), Some("target" | "node_modules" | ".git"))
            }) {
                continue;
            }
            walk(repository, &child, files);
        }
    } else if path.is_file() {
        files.insert(
            path.strip_prefix(repository)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            file(path).sha256,
        );
    }
}

/// 所有数学/类型/后端、测量源码、fixture与锁均绑定，不能事后署当前版本摘要。
pub fn sources(repository: &Path) -> BTreeMap<String, String> {
    let expected = repository.join("stdlib").canonicalize().unwrap();
    assert_eq!(
        xiao_standard_library::source_root().canonicalize().unwrap(),
        expected,
        "A6必须加载本仓库stdlib"
    );
    let mut files = BTreeMap::new();
    for name in [
        "core/rust/crates",
        "core/rust/Cargo.toml",
        "core/rust/Cargo.lock",
        "core/rust/rust-toolchain.toml",
        "stdlib",
        "tests/fixtures/20o-b2-a6",
        ".cargo/config.toml",
    ] {
        let path = repository.join(name);
        if path.exists() {
            walk(repository, &path, &mut files);
        }
    }
    files
}

/// release可执行文件、Runtime与真实Rust版本一起记录；不改全局环境。
pub fn capture(repository: &Path) -> Identity {
    let rustc_path = std::env::var_os("RUSTC").map_or_else(
        || {
            let output = std::process::Command::new("rustup")
                .args(["which", "rustc"])
                .output()
                .unwrap();
            assert!(output.status.success());
            PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
        },
        PathBuf::from,
    );
    let compiler = tool(&rustc_path, "-Vv");
    let rustc = compiler.version.clone();
    assert!(rustc.starts_with("rustc 1.96.0"));
    let mut tools = BTreeMap::from([("rustc".to_owned(), compiler)]);
    for name in ["XIAO_CLANG", "XIAO_LLVM_AS", "XIAO_LLC"] {
        if let Some(path) = std::env::var_os(name) {
            tools.insert(name.to_owned(), tool(&PathBuf::from(path), "--version"));
        }
    }
    let runtime = std::env::var_os("XIAO_RUNTIME_LIBRARY")
        .map(PathBuf::from)
        .map(|path| file(&path));
    Identity {
        repository: repository.display().to_string(),
        files: sources(repository),
        target: xiao_codegen_llvm::TargetDescription::host().triple,
        rustc,
        tools,
        runtime,
        worker: file(&std::env::current_exe().unwrap()),
    }
}
