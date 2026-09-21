//! 外部 LLVM 工具链适配。
//!
//! 工具路径由调用方注入；本模块不搜索 PATH、不读取 CLI 配置，也不把宿主路径写入构建
//! 指纹。版本文本和规范化目标字段会进入指纹，确保工具升级不会静默复用旧产物。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{CodegenError, Result};
use crate::target::TargetDescription;

/// 外部工具链版本清单。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ToolchainVersions {
    /// `clang --version` 的规范化首行。
    pub clang: String,
    /// `llvm-as --version` 的规范化首行。
    pub llvm_as: Option<String>,
    /// `llc --version` 的规范化首行。
    pub llc: Option<String>,
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
            "codegen={codegen_version};{};clang={};llvm-as={};llc={}",
            target.fingerprint_fields(),
            self.versions.clang,
            self.versions.llvm_as.as_deref().unwrap_or("<none>"),
            self.versions.llc.as_deref().unwrap_or("<none>")
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
        let result = run_command(
            &self.clang,
            &[
                "-target",
                &target.triple,
                "-Wno-override-module",
                &path_text(&temp.path),
                "-o",
                &path_text(&output),
            ],
            "clang",
        )?;
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
