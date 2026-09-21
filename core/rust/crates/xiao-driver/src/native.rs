//! 10A 前端到 LLVM 后端的内部构建驱动器。
//!
//! 本模块只编排既有 `FrontendCompiler` 和 `xiao-codegen-llvm`。它不解析 CLI 参数、不发现
//! 工具链，也不复制类型检查；用户可见的 `xiao build` 留给 11/X0。

use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use xiao_codegen_llvm::{
    BuildRequest, CodegenError, CodegenOptions, NativeArtifact, NativeBuild, TargetDescription,
    Toolchain,
};

use crate::frontend::{FrontendArtifact, FrontendCompiler, FrontendError, FrontendRequest};

/// 一次前端到原生程序的内部构建请求。
#[derive(Clone, Debug)]
pub struct NativeBuildRequest {
    /// 真实 Xiao 源码前端请求。
    pub frontend: FrontendRequest,
    /// 规范化目标描述。
    pub target: TargetDescription,
    /// 调用方注入的 LLVM 工具链。
    pub toolchain: Toolchain,
    /// 原生可执行文件输出路径。
    pub output: PathBuf,
    /// 可选的 LLVM 文本输出路径。
    pub llvm_ir_output: Option<PathBuf>,
    /// N0-A 代码生成选项。
    pub codegen_options: CodegenOptions,
}

impl NativeBuildRequest {
    /// 创建一份前端到原生的默认请求。
    #[must_use]
    pub fn new(
        frontend: FrontendRequest,
        target: TargetDescription,
        toolchain: Toolchain,
        output: impl Into<PathBuf>,
    ) -> Self {
        Self {
            frontend,
            codegen_options: CodegenOptions::for_target(target.clone()),
            target,
            toolchain,
            output: output.into(),
            llvm_ir_output: None,
        }
    }

    /// 设置代码生成选项。
    #[must_use]
    pub fn with_codegen_options(mut self, options: CodegenOptions) -> Self {
        self.codegen_options = options;
        self
    }

    /// 设置 LLVM 文本输出路径。
    #[must_use]
    pub fn with_llvm_ir_output(mut self, path: impl Into<PathBuf>) -> Self {
        self.llvm_ir_output = Some(path.into());
        self
    }
}

/// 前端成功且原生链接完成后的结果。
#[derive(Debug)]
pub struct NativeBuildResult {
    /// 前端产物；与交给后端的 IR 是同一份值对象。
    pub frontend: FrontendArtifact,
    /// 原生构建产物。
    pub native: NativeArtifact,
}

/// 前端到原生构建失败的结构化错误。
#[derive(Debug)]
pub enum NativeDriverError {
    /// 前端拒绝源码。
    Frontend(FrontendError),
    /// LLVM 后端或工具链拒绝构建。
    Backend(CodegenError),
}

impl Display for NativeDriverError {
    /// 将驱动器错误渲染为用户可读的稳定文本。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frontend(error) => Display::fmt(error, formatter),
            Self::Backend(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for NativeDriverError {}

/// 前端到 LLVM 的无状态内部驱动器。
#[derive(Clone, Copy, Debug, Default)]
pub struct FrontendNativeDriver;

impl FrontendNativeDriver {
    /// 创建驱动器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// 运行一次真实前端并构建原生程序。
    pub fn build(
        &self,
        request: &NativeBuildRequest,
    ) -> Result<NativeBuildResult, NativeDriverError> {
        let frontend = FrontendCompiler::new()
            .compile(&request.frontend)
            .map_err(NativeDriverError::Frontend)?;
        self.build_artifact(&frontend, request)
    }

    /// 使用一份已经由统一前端产出的产物构建原生程序。
    ///
    /// 该入口专门供差分测试和后续内部编排使用：调用方可以先把同一份
    /// [`FrontendArtifact`] 交给 VM，再把它交给 LLVM，避免第二次解析、类型检查或
    /// 生命周期分析。
    pub fn build_artifact(
        &self,
        frontend: &FrontendArtifact,
        request: &NativeBuildRequest,
    ) -> Result<NativeBuildResult, NativeDriverError> {
        let mut backend_request = BuildRequest::new(
            frontend.ir.clone(),
            request.target.clone(),
            request.toolchain.clone(),
            request.output.clone(),
        )
        .with_options(request.codegen_options.clone());
        if let Some(path) = &request.llvm_ir_output {
            backend_request = backend_request.with_llvm_ir_output(path.clone());
        }
        let native = NativeBuild::new()
            .build(&backend_request)
            .map_err(NativeDriverError::Backend)?;
        Ok(NativeBuildResult {
            frontend: frontend.clone(),
            native,
        })
    }
}
