//! LLVM 文本验证、编译、链接和运行的内部构建驱动器。

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use xiao_ir::IrProgram;

use crate::CODEGEN_VERSION;
use crate::error::{CodegenError, Result};
use crate::ir::{CodegenOptions, LlvmModule, lower_program};
use crate::target::TargetDescription;
use crate::toolchain::{Toolchain, ToolchainFingerprint};

/// 一次原生构建请求。
#[derive(Clone, Debug)]
pub struct BuildRequest {
    /// 已通过前端验证的类型化 IR。
    pub program: IrProgram,
    /// 代码生成选项。
    pub options: CodegenOptions,
    /// 外部 LLVM 工具链。
    pub toolchain: Toolchain,
    /// 原生可执行文件输出路径。
    pub output: PathBuf,
    /// 是否把生成的 LLVM 文本同时落盘。
    pub llvm_ir_output: Option<PathBuf>,
}

impl BuildRequest {
    /// 创建一个只指定 IR、目标、工具链和输出路径的请求。
    #[must_use]
    pub fn new(
        program: IrProgram,
        target: TargetDescription,
        toolchain: Toolchain,
        output: impl Into<PathBuf>,
    ) -> Self {
        Self {
            program,
            options: CodegenOptions::for_target(target),
            toolchain,
            output: output.into(),
            llvm_ir_output: None,
        }
    }

    /// 设置代码生成选项。
    #[must_use]
    pub fn with_options(mut self, options: CodegenOptions) -> Self {
        self.options = options;
        self
    }

    /// 设置 LLVM 文本输出路径。
    #[must_use]
    pub fn with_llvm_ir_output(mut self, path: impl Into<PathBuf>) -> Self {
        self.llvm_ir_output = Some(path.into());
        self
    }
}

/// 已完成的原生构建产物。
#[derive(Clone, Debug)]
pub struct NativeArtifact {
    /// 生成的 LLVM 模块。
    pub module: LlvmModule,
    /// 可执行文件路径。
    pub executable: PathBuf,
    /// LLVM 工具链与目标组成的指纹。
    pub toolchain_fingerprint: ToolchainFingerprint,
}

/// 原生构建驱动器；它不解析命令行，也不发现工具链。
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeBuild;

impl NativeBuild {
    /// 创建一个无状态构建驱动器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// 只生成 LLVM 文本并验证输入，不调用外部工具链。
    pub fn lower(&self, program: &IrProgram, options: &CodegenOptions) -> Result<LlvmModule> {
        lower_program(program, options)
    }

    /// 执行 LLVM 文本验证、编译和链接。
    pub fn build(&self, request: &BuildRequest) -> Result<NativeArtifact> {
        let module = lower_program(&request.program, &request.options)?;
        if let Some(path) = &request.llvm_ir_output {
            if let Some(parent) = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                fs::create_dir_all(parent).map_err(|error| CodegenError::Io {
                    path: parent.to_path_buf(),
                    message: error.to_string(),
                })?;
            }
            fs::write(path, &module.text).map_err(|error| CodegenError::Io {
                path: path.clone(),
                message: error.to_string(),
            })?;
        }
        let executable =
            request
                .toolchain
                .compile(&module.text, &request.options.target, &request.output)?;
        let fingerprint = request
            .toolchain
            .fingerprint(&request.options.target, CODEGEN_VERSION);
        Ok(NativeArtifact {
            module,
            executable,
            toolchain_fingerprint: fingerprint,
        })
    }

    /// 只验证 LLVM 文本，不生成目标文件。
    pub fn validate_llvm(
        &self,
        text: &str,
        target: &TargetDescription,
        toolchain: &Toolchain,
    ) -> Result<()> {
        toolchain.validate_llvm_ir(text, target)
    }
}

/// 一次原生可执行文件的运行结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeRunResult {
    /// 宿主进程退出码。
    pub status: Option<i32>,
    /// 标准输出。
    pub stdout: String,
    /// 标准错误。
    pub stderr: String,
}

/// 原生程序运行器；只负责观察进程，不改变语言语义。
#[derive(Clone, Debug)]
pub struct NativeRun {
    /// 要运行的可执行文件路径。
    pub executable: PathBuf,
}

impl NativeRun {
    /// 从构建产物创建运行器。
    #[must_use]
    pub fn new(artifact: &NativeArtifact) -> Self {
        Self {
            executable: artifact.executable.clone(),
        }
    }

    /// 使用无参数启动程序并捕获输出。
    pub fn run(&self) -> Result<NativeRunResult> {
        self.run_with_args(std::iter::empty::<&std::ffi::OsStr>())
    }

    /// 使用指定参数启动程序并捕获输出。
    pub fn run_with_args<I, S>(&self, args: I) -> Result<NativeRunResult>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let output = Command::new(&self.executable)
            .args(args)
            .output()
            .map_err(|error| CodegenError::ToolchainUnavailable {
                tool: self.executable.display().to_string(),
                message: error.to_string(),
            })?;
        Ok(NativeRunResult {
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
