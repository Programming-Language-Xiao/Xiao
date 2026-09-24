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
use crate::text::stable_hash;
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
            "xiao-fnv1a64-{}",
            stable_hash(canonical.as_bytes())
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
        self.compile_inner(text, target, output, None, false, None)
    }

    /// 验证 LLVM IR 后仅链接 IR 自身；即便工具链配置过 Runtime，也不会继承它。
    pub fn compile_without_runtime(
        &self,
        text: &str,
        target: &TargetDescription,
        output: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        self.compile_inner(text, target, output, None, false, None)
    }

    /// 验证 LLVM IR 后调用 clang，并按需链接调用方注入的 Runtime 静态库。
    pub fn compile_with_runtime(
        &self,
        text: &str,
        target: &TargetDescription,
        output: impl AsRef<Path>,
        runtime_library: Option<&Path>,
    ) -> Result<PathBuf> {
        self.compile_inner(text, target, output, runtime_library, true, None)
    }

    /// 验证 LLVM IR、链接 Runtime，并把调试启动 shim 链接进原生产物。
    pub fn compile_with_startup_shim(
        &self,
        text: &str,
        target: &TargetDescription,
        output: impl AsRef<Path>,
        runtime_library: Option<&Path>,
        inherit_configured_runtime: bool,
        diagnostics_path: &Path,
    ) -> Result<PathBuf> {
        self.compile_inner(
            text,
            target,
            output,
            runtime_library,
            inherit_configured_runtime,
            Some(diagnostics_path),
        )
    }

    /// 按是否继承配置中的 Runtime 库执行一次验证、编译和链接。
    fn compile_inner(
        &self,
        text: &str,
        target: &TargetDescription,
        output: impl AsRef<Path>,
        runtime_library: Option<&Path>,
        inherit_configured_runtime: bool,
        diagnostics_path: Option<&Path>,
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
        let mut startup_object = None;
        if let Some(diagnostics_path) = diagnostics_path {
            let source = TempFile::new("xiao-startup", "c")?;
            fs::write(&source.path, startup_source(diagnostics_path, target)).map_err(|error| {
                CodegenError::Io {
                    path: source.path.clone(),
                    message: error.to_string(),
                }
            })?;
            let object = source.path.with_extension(
                if matches!(target.object_format, crate::target::ObjectFormat::Coff) {
                    "obj"
                } else {
                    "o"
                },
            );
            let compile_args = [
                "-target".to_owned(),
                target.triple.clone(),
                "-c".to_owned(),
                path_text(&source.path),
                "-o".to_owned(),
                path_text(&object),
            ];
            let compile_refs = compile_args.iter().map(String::as_str).collect::<Vec<_>>();
            let result = match run_command(&self.clang, &compile_refs, "clang-startup-shim") {
                Ok(result) => result,
                Err(error) => {
                    let _ = fs::remove_file(&object);
                    return Err(error);
                }
            };
            if !result.status.success() {
                let _ = fs::remove_file(&object);
                return Err(CodegenError::ToolchainFailed {
                    tool: result.tool,
                    status: result.status.code(),
                    stderr: result.stderr,
                });
            }
            args.insert(3, path_text(&object));
            startup_object = Some(object);
        }
        let runtime_library = if inherit_configured_runtime {
            runtime_library.or(self.runtime_library.as_deref())
        } else {
            runtime_library
        };
        if let Some(runtime_library) = runtime_library {
            if self.native_static_libraries.is_empty() {
                if let Some(object) = startup_object.take() {
                    let _ = fs::remove_file(object);
                }
                return Err(CodegenError::ToolchainUnavailable {
                    tool: "rustc".to_owned(),
                    message: "链接 Xiao Runtime staticlib 前必须查询 native-static-libs".to_owned(),
                });
            }
            args.push(path_text(runtime_library));
            args.extend(self.native_static_libraries.iter().cloned());
        }
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let result = match run_command(&self.clang, &arg_refs, "clang") {
            Ok(result) => result,
            Err(error) => {
                if let Some(object) = startup_object.take() {
                    let _ = fs::remove_file(object);
                }
                return Err(error);
            }
        };
        if let Some(object) = startup_object.take() {
            let _ = fs::remove_file(object);
        }
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

/// 生成跨平台的最小诊断启动 shim；它只创建诊断进程，不执行 Xiao 代码。
fn startup_source(diagnostics_path: &Path, target: &TargetDescription) -> String {
    let diagnostics_name = diagnostics_file_name(diagnostics_path, target);
    if matches!(target.object_format, crate::target::ObjectFormat::Coff) {
        /// Windows 原生调试入口使用的独立控制台与就绪握手模板。
        const WINDOWS_SOURCE: &str = r#"#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <process.h>
#include <stdio.h>
#include <string.h>
#include <wchar.h>

static const wchar_t xiao_renderer_name[] = __XIAO_DIAGNOSTICS_NAME__;

static int xiao_startup_error(const char *reason, DWORD code) {
  fprintf(stderr, "X11-DIAGNOSTIC-START-001: %s (%lu)\n", reason, (unsigned long)code);
  return 70;
}

static DWORD xiao_adjacent_renderer(wchar_t *buffer, DWORD capacity) {
  DWORD length = GetModuleFileNameW(NULL, buffer, capacity);
  if (length == 0 || length >= capacity - 1) {
    DWORD error = GetLastError();
    return error ? error : ERROR_BUFFER_OVERFLOW;
  }
  wchar_t *backslash = wcsrchr(buffer, L'\\');
  wchar_t *slash = wcsrchr(buffer, L'/');
  wchar_t *separator = backslash;
  if (slash && (!separator || slash > separator)) {
    separator = slash;
  }
  DWORD prefix = separator ? (DWORD)(separator - buffer + 1) : 0;
  size_t name_length = wcslen(xiao_renderer_name);
  if ((size_t)prefix + name_length + 1 > capacity) {
    return ERROR_BUFFER_OVERFLOW;
  }
  memcpy(buffer + prefix, xiao_renderer_name, (name_length + 1) * sizeof(wchar_t));
  return ERROR_SUCCESS;
}

int xiao_native_debug_start(void) {
  wchar_t configured[32768];
  wchar_t adjacent[32768];
  DWORD configured_length = GetEnvironmentVariableW(
      L"XIAO_DIAGNOSTICS_PATH", configured,
      (DWORD)(sizeof(configured) / sizeof(configured[0])));
  if (configured_length >= (DWORD)(sizeof(configured) / sizeof(configured[0]))) {
    return xiao_startup_error("XIAO_DIAGNOSTICS_PATH is too long", ERROR_BUFFER_OVERFLOW);
  }
  const wchar_t *renderer = configured;
  if (configured_length == 0) {
    DWORD adjacent_error = xiao_adjacent_renderer(
        adjacent, (DWORD)(sizeof(adjacent) / sizeof(adjacent[0])));
    if (adjacent_error != ERROR_SUCCESS) {
      return xiao_startup_error("cannot resolve the adjacent diagnostic component", adjacent_error);
    }
    renderer = adjacent;
  }

  wchar_t temporary_directory[MAX_PATH];
  wchar_t ready_file[MAX_PATH];
  DWORD temporary_length = GetTempPathW(MAX_PATH, temporary_directory);
  if (temporary_length == 0 || temporary_length >= MAX_PATH) {
    return xiao_startup_error("cannot resolve the temporary directory", GetLastError());
  }
  if (GetTempFileNameW(temporary_directory, L"xdr", 0, ready_file) == 0) {
    return xiao_startup_error("cannot reserve the readiness marker", GetLastError());
  }
  if (!DeleteFileW(ready_file)) {
    return xiao_startup_error("cannot prepare the readiness marker", GetLastError());
  }

  wchar_t command_line[32768];
  int command_length = swprintf(
      command_line, sizeof(command_line) / sizeof(command_line[0]),
      L"\"%ls\" --standalone --parent-pid %lu --ready-file \"%ls\"",
      renderer, (unsigned long)_getpid(), ready_file);
  if (command_length < 0 ||
      command_length >= (int)(sizeof(command_line) / sizeof(command_line[0]))) {
    return xiao_startup_error("diagnostic command line is too long", ERROR_BUFFER_OVERFLOW);
  }

  STARTUPINFOW startup;
  PROCESS_INFORMATION process;
  ZeroMemory(&startup, sizeof(startup));
  ZeroMemory(&process, sizeof(process));
  startup.cb = sizeof(startup);
  if (!CreateProcessW(renderer, command_line, NULL, NULL, FALSE,
                      CREATE_NEW_CONSOLE | CREATE_UNICODE_ENVIRONMENT,
                      NULL, NULL, &startup, &process)) {
    DeleteFileW(ready_file);
    return xiao_startup_error("cannot create the diagnostic console", GetLastError());
  }
  CloseHandle(process.hThread);

  for (DWORD attempt = 0; attempt < 500; ++attempt) {
    DWORD attributes = GetFileAttributesW(ready_file);
    if (attributes != INVALID_FILE_ATTRIBUTES &&
        (attributes & FILE_ATTRIBUTE_DIRECTORY) == 0) {
      DeleteFileW(ready_file);
      CloseHandle(process.hProcess);
      return 0;
    }

    DWORD state = WaitForSingleObject(process.hProcess, 10);
    if (state == WAIT_OBJECT_0) {
      DWORD exit_code = 0;
      GetExitCodeProcess(process.hProcess, &exit_code);
      CloseHandle(process.hProcess);
      DeleteFileW(ready_file);
      return xiao_startup_error("diagnostic process exited before readiness", exit_code);
    }
    if (state == WAIT_FAILED) {
      DWORD error = GetLastError();
      TerminateProcess(process.hProcess, 70);
      CloseHandle(process.hProcess);
      DeleteFileW(ready_file);
      return xiao_startup_error("cannot wait for diagnostic readiness", error);
    }
  }

  TerminateProcess(process.hProcess, 70);
  WaitForSingleObject(process.hProcess, 1000);
  CloseHandle(process.hProcess);
  DeleteFileW(ready_file);
  return xiao_startup_error("diagnostic readiness timed out", ERROR_TIMEOUT);
}
"#;
        return WINDOWS_SOURCE.replace(
            "__XIAO_DIAGNOSTICS_NAME__",
            &c_utf16_array_initializer(&diagnostics_name),
        );
    }

    /// POSIX 原生调试入口使用的进程启动与就绪握手模板。
    const POSIX_SOURCE: &str = r#"#include <errno.h>
#include <stdint.h>
#include <signal.h>
#include <spawn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>
#if defined(__APPLE__)
#include <mach-o/dyld.h>
#endif

extern char **environ;
static const char xiao_renderer_name[] = __XIAO_DIAGNOSTICS_NAME__;

static int xiao_startup_error(const char *reason, int code) {
  fprintf(stderr, "X11-DIAGNOSTIC-START-001: %s (%d)\n", reason, code);
  return 70;
}

static int xiao_adjacent_renderer(char *buffer, size_t capacity) {
#if defined(__APPLE__)
  uint32_t requested = (uint32_t)capacity;
  if (_NSGetExecutablePath(buffer, &requested) != 0) {
    return ENAMETOOLONG;
  }
#else
  ssize_t length = readlink("/proc/self/exe", buffer, capacity - 1);
  if (length < 0) {
    return errno;
  }
  if ((size_t)length >= capacity - 1) {
    return ENAMETOOLONG;
  }
  buffer[length] = '\0';
#endif
  char *separator = strrchr(buffer, '/');
  size_t prefix = separator ? (size_t)(separator - buffer + 1) : 0;
  size_t name_length = strlen(xiao_renderer_name);
  if (prefix + name_length + 1 > capacity) {
    return ENAMETOOLONG;
  }
  memcpy(buffer + prefix, xiao_renderer_name, name_length + 1);
  return 0;
}

int xiao_native_debug_start(void) {
  const char *configured = getenv("XIAO_DIAGNOSTICS_PATH");
  char adjacent[32768];
  const char *renderer = configured;
  if (!renderer || !renderer[0]) {
    int adjacent_error = xiao_adjacent_renderer(adjacent, sizeof(adjacent));
    if (adjacent_error != 0) {
      return xiao_startup_error("cannot resolve the adjacent diagnostic component", adjacent_error);
    }
    renderer = adjacent;
  }
  char parent[32];
  snprintf(parent, sizeof(parent), "%ld", (long)getpid());

  char ready_file[] = "/tmp/xiao-diagnostics-ready-XXXXXX";
  int ready_descriptor = mkstemp(ready_file);
  if (ready_descriptor < 0) {
    return xiao_startup_error("cannot reserve the readiness marker", errno);
  }
  close(ready_descriptor);
  if (unlink(ready_file) != 0) {
    return xiao_startup_error("cannot prepare the readiness marker", errno);
  }

  char *const argv[] = {
      (char *)renderer,
      (char *)"--standalone",
      (char *)"--parent-pid",
      parent,
      (char *)"--ready-file",
      ready_file,
      NULL,
  };
  pid_t child = 0;
  int status = posix_spawn(&child, renderer, NULL, NULL, argv, environ);
  if (status != 0) {
    unlink(ready_file);
    return xiao_startup_error("cannot create the diagnostic process", status);
  }

  for (int attempt = 0; attempt < 500; ++attempt) {
    if (access(ready_file, F_OK) == 0) {
      unlink(ready_file);
      return 0;
    }
    int child_status = 0;
    pid_t state = waitpid(child, &child_status, WNOHANG);
    if (state == child) {
      unlink(ready_file);
      return xiao_startup_error("diagnostic process exited before readiness", child_status);
    }
    if (state < 0) {
      int error = errno;
      kill(child, SIGTERM);
      unlink(ready_file);
      return xiao_startup_error("cannot wait for diagnostic readiness", error);
    }
    usleep(10000);
  }

  kill(child, SIGTERM);
  waitpid(child, NULL, 0);
  unlink(ready_file);
  return xiao_startup_error("diagnostic readiness timed out", ETIMEDOUT);
}
"#;
    POSIX_SOURCE.replace(
        "__XIAO_DIAGNOSTICS_NAME__",
        &c_string_literal(&diagnostics_name),
    )
}

/// 从构建时组件路径提取可随原生产物搬迁的相邻文件名。
fn diagnostics_file_name(path: &Path, target: &TargetDescription) -> String {
    path_text(path)
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            if matches!(target.object_format, crate::target::ObjectFormat::Coff) {
                "xiao-diagnostics.exe".to_owned()
            } else {
                "xiao-diagnostics".to_owned()
            }
        })
}

/// 将 Windows 路径编码为不受源码编码和转义边界影响的 UTF-16 C 数组。
fn c_utf16_array_initializer(value: &str) -> String {
    let mut output = String::from("{");
    for unit in value.encode_utf16() {
        output.push_str(&format!("0x{unit:04x}, "));
    }
    output.push_str("0}");
    output
}

/// 将宿主路径编码为 C 字符串常量。
fn c_string_literal(value: &str) -> String {
    let mut output = String::from("\"");
    for byte in value.bytes() {
        match byte {
            b'\\' => output.push_str("\\\\"),
            b'"' => output.push_str("\\\""),
            b'\n' => output.push_str("\\n"),
            b'\r' => output.push_str("\\r"),
            0x20..=0x7e => output.push(byte as char),
            // 固定三位八进制避免后续 ASCII 十六进制字符被 C 编译器吞进同一转义。
            other => output.push_str(&format!("\\{other:03o}")),
        }
    }
    output.push('"');
    output
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

/// 计算 Runtime 静态库内容指纹；路径变化不会导致无意义失配。
fn runtime_fingerprint(path: &Path) -> String {
    match fs::read(path) {
        Ok(bytes) => stable_hash(&bytes),
        Err(_) => "<unreadable>".to_owned(),
    }
}

#[cfg(test)]
/// 覆盖 Rust 原生库清单的跨平台解析、过滤和构建指纹隔离。
mod tests {
    use std::path::Path;

    use super::{Toolchain, diagnostics_file_name, parse_native_static_libraries, startup_source};
    use crate::target::TargetDescription;

    #[test]
    /// Windows 调试产物必须创建独立控制台，并等待诊断进程显式确认就绪。
    fn windows_startup_shim_opens_console_and_waits_for_readiness() {
        let source = startup_source(
            Path::new(r"C:\Xiao 工具\xiao-diagnostics.exe"),
            &TargetDescription::windows_x86_64(),
        );
        assert!(source.contains("CreateProcessW"));
        assert!(source.contains("CREATE_NEW_CONSOLE"));
        assert!(source.contains("GetModuleFileNameW"));
        assert!(source.contains("xiao_adjacent_renderer"));
        assert!(source.contains("--ready-file"));
        assert!(source.contains("WaitForSingleObject"));
        assert!(source.contains("diagnostic process exited before readiness"));
        assert!(!source.contains("_spawnv"));
    }

    #[test]
    /// POSIX 启动路径也必须以就绪标记阻止用户入口抢跑。
    fn posix_startup_shim_waits_for_readiness() {
        let source = startup_source(
            Path::new("/opt/xiao/bin/xiao-diagnostics"),
            &TargetDescription::linux_x86_64(),
        );
        assert!(source.contains("posix_spawn"));
        assert!(source.contains("/proc/self/exe"));
        assert!(source.contains("_NSGetExecutablePath"));
        assert!(source.contains("--ready-file"));
        assert!(source.contains("waitpid"));
        assert!(source.contains("diagnostic readiness timed out"));
    }

    #[test]
    /// 启动桥只携带组件文件名，确保最终产物可整体搬迁到另一目录。
    fn startup_shim_embeds_only_diagnostics_basename() {
        let windows_path = Path::new(r"C:\build\xiao-diagnostics.exe");
        let posix_path = Path::new("/build/xiao-diagnostics");
        assert_eq!(
            diagnostics_file_name(windows_path, &TargetDescription::windows_x86_64()),
            "xiao-diagnostics.exe"
        );
        assert_eq!(
            diagnostics_file_name(posix_path, &TargetDescription::linux_x86_64()),
            "xiao-diagnostics"
        );

        let source = startup_source(windows_path, &TargetDescription::windows_x86_64());
        assert!(source.contains("0x0078"));
        assert!(!source.contains("C:\\\\build"));
    }

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
