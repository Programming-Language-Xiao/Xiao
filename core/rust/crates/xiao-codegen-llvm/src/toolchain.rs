//! 外部 LLVM 工具链适配。
//!
//! 工具路径由调用方注入；本模块不搜索 PATH、不读取 CLI 配置，也不把宿主路径写入构建
//! 指纹。版本文本、Runtime 原生库清单和规范化目标字段会进入指纹，确保工具升级或链接
//! 依赖变化不会静默复用旧产物。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{CodegenError, Result};
use crate::target::TargetDescription;
use xiao_runtime_abi::ABI_ENCODED_VERSION;

/// 外部工具链版本清单。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ToolchainVersions {
    /// `clang --version` 的规范化首行。
    pub clang: String,
    /// `llvm-as --version` 的规范化首行。
    pub llvm_as: Option<String>,
    /// `llc --version` 的规范化首行。
    pub llc: Option<String>,
    /// `rustc --version` 的规范化首行；查询 Runtime 原生库时一并登记。
    pub rustc: Option<String>,
}

/// 一个稳定构建指纹。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ToolchainFingerprint(String);

impl ToolchainFingerprint {
    /// 返回指纹的固定宽度十六进制文本。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ToolchainFingerprint {
    /// 将构建指纹写入日志或产物元数据。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// 调用方注入的 LLVM 工具路径和版本信息。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Toolchain {
    /// `clang` 可执行文件路径。
    pub clang: PathBuf,
    /// 可选的 `llvm-as` 路径；提供后优先用它做语法验证。
    pub llvm_as: Option<PathBuf>,
    /// 可选的 `llc` 路径；N0-A 的直接链接路径不强制使用它。
    pub llc: Option<PathBuf>,
    /// 可选的 Xiao Runtime 静态库路径；动态模块必须由调用方显式注入。
    pub runtime_library: Option<PathBuf>,
    /// 链接 Xiao Runtime staticlib 时由 Rust 工具链报告的原生库参数。
    ///
    /// 这里保存已经适合传给 clang 的参数（例如 `-lkernel32`），不在后端按平台
    /// 硬编码。动态模块若配置了 Runtime 静态库，也必须配置这份清单。
    pub native_static_libraries: Vec<String>,
    /// 已登记的工具版本文本。
    pub versions: ToolchainVersions,
}

impl Toolchain {
    /// 用一个 clang 路径创建工具链描述。
    #[must_use]
    pub fn new(clang: impl Into<PathBuf>) -> Self {
        Self {
            clang: clang.into(),
            llvm_as: None,
            llc: None,
            runtime_library: None,
            native_static_libraries: Vec::new(),
            versions: ToolchainVersions::default(),
        }
    }

    /// 设置 `llvm-as` 路径。
    #[must_use]
    pub fn with_llvm_as(mut self, path: impl Into<PathBuf>) -> Self {
        self.llvm_as = Some(path.into());
        self
    }

    /// 设置 `llc` 路径。
    #[must_use]
    pub fn with_llc(mut self, path: impl Into<PathBuf>) -> Self {
        self.llc = Some(path.into());
        self
    }

    /// 设置 Xiao Runtime 静态库路径；后端不自行搜索或发现它。
    #[must_use]
    pub fn with_runtime_library(mut self, path: impl Into<PathBuf>) -> Self {
        self.runtime_library = Some(path.into());
        self
    }

    /// 设置链接 Runtime staticlib 所需的原生库参数。
    #[must_use]
    pub fn with_native_static_libraries<I, S>(mut self, libraries: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.native_static_libraries = libraries.into_iter().map(Into::into).collect();
        self
    }

    /// 从 `rustc --print native-static-libs` 的输出查询原生库参数。
    ///
    /// 调用方提供 `rustc` 路径和目标描述；后端不会搜索 Rust 或 C 工具链。查询结果
    /// 会保留 Rust 报告的顺序，并在解析时把 Windows `.lib` 名称转成 clang 的 `-l`
    /// 形式，过滤 Rust 输出中的 `/defaultlib:*` 元参数。
    pub fn probe_native_static_libraries(
        self,
        rustc: impl AsRef<Path>,
        target: &TargetDescription,
    ) -> Result<Self> {
        let rustc = rustc.as_ref();
        let libraries = query_native_static_libraries(rustc, target)?;
        let rustc_version = version_line(rustc, "rustc")?;
        Ok(self
            .with_native_static_libraries(libraries)
            .with_rustc_version(rustc_version))
    }

    /// 登记 Rust 编译器版本文本；通常由 [`Self::probe_native_static_libraries`] 填充。
    #[must_use]
    pub fn with_rustc_version(mut self, version: impl Into<String>) -> Self {
        self.versions.rustc = Some(version.into());
        self
    }

    /// 设置版本清单；版本必须由调用方固定或通过 [`Self::probe_versions`] 获取。
    #[must_use]
    pub fn with_versions(mut self, versions: ToolchainVersions) -> Self {
        self.versions = versions;
        self
    }

    /// 运行外部工具并读取版本首行，返回新的带版本描述。
    pub fn probe_versions(mut self) -> Result<Self> {
        self.versions.clang = version_line(&self.clang, "clang")?;
        self.versions.llvm_as = self
            .llvm_as
            .as_deref()
            .map(|path| version_line(path, "llvm-as"))
            .transpose()?;
        self.versions.llc = self
            .llc
            .as_deref()
            .map(|path| version_line(path, "llc"))
            .transpose()?;
        Ok(self)
    }

    /// 计算目标和版本字段组成的稳定指纹。
    ///
    /// 指纹输入不含绝对路径，避免同一工具链安装到不同目录后产生无意义失配；路径仍由
    /// 调用方保留在构建日志中供诊断追溯。
    #[must_use]
    pub fn fingerprint(
        &self,
        target: &TargetDescription,
        codegen_version: u32,
    ) -> ToolchainFingerprint {
        let canonical = format!(
            "codegen={codegen_version};abi={ABI_ENCODED_VERSION};runtime={};native={};{};clang={};llvm-as={};llc={};rustc={}",
            self.runtime_library
                .as_deref()
                .map(runtime_fingerprint)
                .unwrap_or_else(|| "<none>".to_owned()),
            self.native_static_libraries.join(","),
            target.fingerprint_fields(),
            self.versions.clang,
            self.versions.llvm_as.as_deref().unwrap_or("<none>"),
            self.versions.llc.as_deref().unwrap_or("<none>"),
            self.versions.rustc.as_deref().unwrap_or("<none>")
        );
        ToolchainFingerprint(format!(
            "xiao-fnv1a64-{:016x}",
            fnv1a64(canonical.as_bytes())
        ))
    }

    /// 用 `llvm-as` 或 clang 验证一份 LLVM IR 文本。
    pub fn validate_llvm_ir(&self, text: &str, target: &TargetDescription) -> Result<()> {
        let temp = TempFile::new("xiao-verify", "ll")?;
        fs::write(&temp.path, text).map_err(|error| CodegenError::Io {
            path: temp.path.clone(),
            message: error.to_string(),
        })?;
        let result = if let Some(llvm_as) = &self.llvm_as {
            let bitcode = temp.path.with_extension("bc");
            let output = run_command(
                llvm_as,
                &["-o", &path_text(&bitcode), &path_text(&temp.path)],
                "llvm-as",
            );
            let _ = fs::remove_file(&bitcode);
            output?
        } else {
            let object = temp.path.with_extension("o");
            let output = run_command(
                &self.clang,
                &[
                    "-target",
                    &target.triple,
                    "-x",
                    "ir",
                    "-c",
                    "-o",
                    &path_text(&object),
                    &path_text(&temp.path),
                ],
                "clang",
            );
            let _ = fs::remove_file(&object);
            output?
        };
        if result.status.success() {
            Ok(())
        } else {
            Err(CodegenError::ToolchainFailed {
                tool: result.tool,
                status: result.status.code(),
                stderr: result.stderr,
            })
        }
    }

    /// 验证 LLVM IR 后调用 clang 生成并链接原生可执行文件。
    pub fn compile(
        &self,
        text: &str,
        target: &TargetDescription,
        output: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        self.compile_inner(text, target, output, None, false)
    }

    /// 验证 LLVM IR 后仅链接 IR 自身；即便工具链配置过 Runtime，也不会继承它。
    pub fn compile_without_runtime(
        &self,
        text: &str,
        target: &TargetDescription,
        output: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        self.compile_inner(text, target, output, None, false)
    }

    /// 验证 LLVM IR 后调用 clang，并按需链接调用方注入的 Runtime 静态库。
    pub fn compile_with_runtime(
        &self,
        text: &str,
        target: &TargetDescription,
        output: impl AsRef<Path>,
        runtime_library: Option<&Path>,
    ) -> Result<PathBuf> {
        self.compile_inner(text, target, output, runtime_library, true)
    }

    /// 按是否继承配置中的 Runtime 库执行一次验证、编译和链接。
    fn compile_inner(
        &self,
        text: &str,
        target: &TargetDescription,
        output: impl AsRef<Path>,
        runtime_library: Option<&Path>,
        inherit_configured_runtime: bool,
    ) -> Result<PathBuf> {
        let output = output.as_ref().to_path_buf();
        if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|error| CodegenError::Io {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }
        self.validate_llvm_ir(text, target)?;
        let temp = TempFile::new("xiao-build", "ll")?;
        fs::write(&temp.path, text).map_err(|error| CodegenError::Io {
            path: temp.path.clone(),
            message: error.to_string(),
        })?;
        let mut args = vec![
            "-target".to_owned(),
            target.triple.clone(),
            "-Wno-override-module".to_owned(),
            path_text(&temp.path),
            "-o".to_owned(),
            path_text(&output),
        ];
        let runtime_library = if inherit_configured_runtime {
            runtime_library.or(self.runtime_library.as_deref())
        } else {
            runtime_library
        };
        if let Some(runtime_library) = runtime_library {
            if self.native_static_libraries.is_empty() {
                return Err(CodegenError::ToolchainUnavailable {
                    tool: "rustc".to_owned(),
                    message: "链接 Xiao Runtime staticlib 前必须查询 native-static-libs".to_owned(),
                });
            }
            args.push(path_text(runtime_library));
            args.extend(self.native_static_libraries.iter().cloned());
        }
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let result = run_command(&self.clang, &arg_refs, "clang")?;
        if !result.status.success() {
            let _ = fs::remove_file(&output);
            return Err(CodegenError::ToolchainFailed {
                tool: result.tool,
                status: result.status.code(),
                stderr: result.stderr,
            });
        }
        Ok(output)
    }
}

/// 运行结果的内部轻量包装，避免把 `std::process::Output` 暴露为 ABI。
struct CommandResult {
    tool: String,
    status: std::process::ExitStatus,
    stderr: String,
}

/// 运行调用方注入的外部工具并收集诊断输出。
fn run_command(path: &Path, args: &[&str], name: &str) -> Result<CommandResult> {
    if path.as_os_str().is_empty() {
        return Err(CodegenError::ToolchainUnavailable {
            tool: name.to_owned(),
            message: "路径为空；工具链必须由调用方注入".to_owned(),
        });
    }
    let output = Command::new(path).args(args).output().map_err(|error| {
        CodegenError::ToolchainUnavailable {
            tool: name.to_owned(),
            message: error.to_string(),
        }
    })?;
    Ok(CommandResult {
        tool: name.to_owned(),
        status: output.status,
        stderr: text_from_bytes(&output.stderr),
    })
}

/// 解析 `rustc --print native-static-libs` 的稳定提示行。
///
/// Rust 在不同平台输出的库名格式不同：Windows 通常是 `kernel32.lib`，Unix 通常是
/// `-ldl`。解析器只把明显的 `.lib` 文件名转换成 clang 可接受的 `-l` 参数，保留已经
/// 是链接参数的 token，并过滤 `/defaultlib:*` 这种由 MSVC 默认库机制处理的元参数。
pub fn parse_native_static_libraries(output: &str) -> Result<Vec<String>> {
    let marker = "native-static-libs:";
    let Some(rest) = output
        .lines()
        .find_map(|line| line.find(marker).map(|index| &line[index + marker.len()..]))
    else {
        return Err(CodegenError::ToolchainFailed {
            tool: "rustc".to_owned(),
            status: None,
            stderr: "输出中没有 native-static-libs 清单".to_owned(),
        });
    };
    let libraries = rest
        .split_whitespace()
        .filter_map(normalize_native_library)
        .collect::<Vec<_>>();
    if libraries.is_empty() {
        return Err(CodegenError::ToolchainFailed {
            tool: "rustc".to_owned(),
            status: None,
            stderr: "native-static-libs 清单为空".to_owned(),
        });
    }
    Ok(libraries)
}

/// 查询指定 Rust 编译器和目标的原生 staticlib 依赖。
pub fn query_native_static_libraries(
    rustc: &Path,
    target: &TargetDescription,
) -> Result<Vec<String>> {
    if rustc.as_os_str().is_empty() {
        return Err(CodegenError::ToolchainUnavailable {
            tool: "rustc".to_owned(),
            message: "路径为空；Rust 编译器必须由调用方注入".to_owned(),
        });
    }
    let source = TempFile::new("xiao-native-libs", "rs")?;
    let artifact = source
        .path
        .with_extension(if cfg!(windows) { "lib" } else { "a" });
    fs::write(&source.path, "pub fn xiao_native_static_lib_probe() {}\n").map_err(|error| {
        CodegenError::Io {
            path: source.path.clone(),
            message: error.to_string(),
        }
    })?;
    let output = Command::new(rustc)
        .args([
            "--crate-type=staticlib",
            "--print=native-static-libs",
            &format!("--target={}", target.triple),
            "-o",
            &path_text(&artifact),
            &path_text(&source.path),
        ])
        .output()
        .map_err(|error| CodegenError::ToolchainUnavailable {
            tool: "rustc".to_owned(),
            message: error.to_string(),
        })?;
    let combined = format!(
        "{}\n{}",
        text_from_bytes(&output.stdout),
        text_from_bytes(&output.stderr)
    );
    let _ = fs::remove_file(&artifact);
    if !output.status.success() {
        return Err(CodegenError::ToolchainFailed {
            tool: "rustc".to_owned(),
            status: output.status.code(),
            stderr: combined,
        });
    }
    parse_native_static_libraries(&combined)
}

/// 把 Rust 输出中的单个库 token 规范化为 clang 参数。
fn normalize_native_library(token: &str) -> Option<String> {
    let lower = token.to_ascii_lowercase();
    if token.is_empty() || lower.starts_with("/defaultlib:") || lower.starts_with("-defaultlib:") {
        return None;
    }
    if token.starts_with("-l") || token.starts_with("-framework") {
        return Some(token.to_owned());
    }
    if lower.ends_with(".lib") {
        let name = token.rsplit(['/', '\\']).next().unwrap_or(token);
        let name = &name[..name.len() - 4];
        return (!name.is_empty()).then(|| format!("-l{name}"));
    }
    Some(token.to_owned())
}

/// 读取外部工具版本输出的第一行。
fn version_line(path: &Path, name: &str) -> Result<String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| CodegenError::ToolchainUnavailable {
            tool: name.to_owned(),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(CodegenError::ToolchainFailed {
            tool: name.to_owned(),
            status: output.status.code(),
            stderr: text_from_bytes(&output.stderr),
        });
    }
    let text = text_from_bytes(&output.stdout);
    Ok(text.lines().next().unwrap_or("unknown").trim().to_owned())
}

/// 将工具输出按 UTF-8 宽松解码并去除首尾空白。
fn text_from_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_owned()
}

/// 转换路径为传给外部进程的稳定文本。
fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// 自动清理的临时 LLVM 文本文件路径。
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    /// 在系统临时目录生成唯一文件名，不创建文件本身。
    fn new(prefix: &str, extension: &str) -> Result<Self> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let file = format!("{prefix}-{}-{now}.{extension}", std::process::id());
        Ok(Self {
            path: std::env::temp_dir().join(file),
        })
    }
}

impl Drop for TempFile {
    /// 删除验证或构建结束后留下的临时文件。
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// 计算工具链指纹使用的 64 位 FNV-1a 哈希。
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// 计算 Runtime 静态库内容指纹；路径变化不会导致无意义失配。
fn runtime_fingerprint(path: &Path) -> String {
    match fs::read(path) {
        Ok(bytes) => format!("{:016x}", fnv1a64(&bytes)),
        Err(_) => "<unreadable>".to_owned(),
    }
}

#[cfg(test)]
/// 覆盖 Rust 原生库清单的跨平台解析、过滤和构建指纹隔离。
mod tests {
    use super::{Toolchain, parse_native_static_libraries};
    use crate::target::TargetDescription;

    #[test]
    /// Rust 的 Windows 清单应转换为 clang `-l` 参数并过滤默认库元参数。
    fn parses_windows_native_static_libraries() {
        let output = "note: native-static-libs: kernel32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib /defaultlib:msvcrt\n";
        assert_eq!(
            parse_native_static_libraries(output).expect("应解析清单"),
            [
                "-lkernel32",
                "-lntdll",
                "-luserenv",
                "-lws2_32",
                "-ldbghelp"
            ]
        );
    }

    #[test]
    /// Unix 风格的 `-l` 参数必须保留顺序和重复项。
    fn preserves_unix_native_static_libraries() {
        let output = "native-static-libs: -lgcc_s -lutil -lgcc_s\n";
        assert_eq!(
            parse_native_static_libraries(output).expect("应解析清单"),
            ["-lgcc_s", "-lutil", "-lgcc_s"]
        );
    }

    #[test]
    /// 缺少 Rust 清单时应返回结构化工具链错误。
    fn rejects_missing_native_static_library_marker() {
        assert!(parse_native_static_libraries("rustc finished successfully").is_err());
    }

    #[test]
    /// 大小写变化的 MSVC 元参数仍应被过滤，库文件名应只保留基名。
    fn normalizes_case_insensitive_windows_tokens() {
        let output = "native-static-libs: C:\\sdk\\Kernel32.LIB /DEFAULTLIB:MSVCRT\n";
        assert_eq!(
            parse_native_static_libraries(output).expect("应解析大小写变化的清单"),
            ["-lKernel32"]
        );
    }

    #[test]
    /// 原生库清单变化必须改变工具链指纹，避免复用错误的链接产物。
    fn native_library_list_is_fingerprinted() {
        let target = TargetDescription::windows_x86_64();
        let first = Toolchain::new("clang").with_native_static_libraries(["-lkernel32"]);
        let second = Toolchain::new("clang").with_native_static_libraries(["-lntdll"]);
        assert_ne!(
            first.fingerprint(&target, 1),
            second.fingerprint(&target, 1)
        );
    }
}
