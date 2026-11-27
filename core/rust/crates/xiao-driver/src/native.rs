//! 10A 前端到 LLVM 后端的内部构建驱动器。
//!
//! 本模块只编排既有 `FrontendCompiler` 和 `xiao-codegen-llvm`。它不解析 CLI 参数、不发现
//! 工具链，也不复制类型检查；用户可见的 `xiao build` 留给 11/X0。

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;
use std::time::Duration;

use xiao_codegen_llvm::{
    BuildRequest, CodegenError, CodegenOptions, NativeArtifact, NativeBuild, NativeRun,
    NativeRunResult, TargetDescription, Toolchain,
};
use xiao_intrinsics::CAPABILITY_ENVIRONMENT;
use xiao_ir::{IrProgram, IrStatement, IrStatementKind};

use crate::frontend::{FrontendArtifact, FrontendCompiler, FrontendError, FrontendRequest};
use crate::run::{CancellationToken, ControlWindow, ExitCode, RunControl};

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
    /// 前端原始产物；后端仅在副本中移除已静态接线的标准库导入。
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

    /// 构建并以同一运行能力启动应用。省略环境时由子进程在用户代码前捕获一次。
    pub fn run_application<I, S>(
        &self,
        request: &NativeBuildRequest,
        options: &crate::RunOptions,
        args: I,
    ) -> Result<NativeRunResult, NativeDriverError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.run_application_with_control(request, options, args, &RunControl::new())
    }

    /// 以可选取消令牌或截止启动原生应用。两处均提供超时时取较早的期限。
    pub fn run_application_with_control<I, S>(
        &self,
        request: &NativeBuildRequest,
        options: &crate::RunOptions,
        args: I,
        control: &RunControl,
    ) -> Result<NativeRunResult, NativeDriverError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let mut control = control.clone();
        if let Some(timeout) = options.timeout_ms.map(Duration::from_millis) {
            let timeout = control.timeout().map_or(timeout, |at| at.min(timeout));
            control = control.with_timeout(timeout);
        }
        let window = match ControlWindow::start(&control) {
            Ok(window) => window,
            Err(error) => return Ok(rejected_run(error.code(), error.message())),
        };
        if let Some(error) = window.check() {
            return Ok(rejected_run(error.code(), error.message()));
        }
        if let Err(error) = options.environment_snapshot() {
            return Ok(rejected_run(error.code(), error.message()));
        }
        let mut request = request.clone();
        request.codegen_options = request
            .codegen_options
            .with_capabilities(options.capabilities)
            .expect("运行快照已校验能力掩码");
        let env_entry = options.capabilities & CAPABILITY_ENVIRONMENT != 0;
        let payload = if env_entry {
            options
                .environment
                .as_ref()
                .map(|value| serde_json::to_vec(value).expect("环境仅包含字符串名值对"))
        } else {
            None
        };
        if let Some(error) = window.check() {
            return Ok(rejected_run(error.code(), error.message()));
        }
        let artifact = self.build(&request);
        if let Some(error) = window.check() {
            return Ok(rejected_run(error.code(), error.message()));
        }
        let cancelled = || {
            control
                .cancellation()
                .is_some_and(CancellationToken::is_cancelled)
        };
        let cancellation_check = control
            .cancellation()
            .map(|_| &cancelled as &dyn Fn() -> bool);
        NativeRun::new(&artifact?.native)
            .run_with_control(
                args,
                payload.as_deref(),
                window.deadline,
                cancellation_check,
                env_entry,
            )
            .map_err(NativeDriverError::Backend)
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
        // 标准库首批模块仅导出纯 Xiao 函数：LLVM 将前端已静态链接进入口 IR 的函数
        // 直接调用，不发射 VM 所需的模块初始化和动态命名空间物化。
        let mut backend_ir = frontend.ir.clone();
        strip_static_standard_imports(&mut backend_ir);
        let mut backend_request = BuildRequest::new(
            backend_ir,
            request.target.clone(),
            request.toolchain.clone(),
            request.output.clone(),
        )
        .with_options(
            request
                .codegen_options
                .clone()
                .with_locale(request.frontend.context.locale.clone()),
        );
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

fn rejected_run(code: &str, message: &str) -> NativeRunResult {
    NativeRunResult {
        status: Some(i32::from(ExitCode::ArtifactRejected.as_process_code())),
        stdout: String::new(),
        stderr: format!("{code}: {message}\n"),
    }
}

/// 递归移除已静态链接的标准库函数导入，保留项目模块与非函数导入。
fn strip_static_standard_imports(program: &mut IrProgram) {
    /// 判断导入项是否对应已登记的纯 Xiao 标准库函数。
    fn is_static_function_import(
        functions: &BTreeSet<(String, String)>,
        item: &xiao_ir::IrImportItem,
    ) -> bool {
        let Some(selected) = item.selected.as_ref() else {
            return false;
        };
        if !item.module.starts_with("stdlib.")
            || matches!(
                item.module.as_str(),
                "stdlib.args" | "stdlib.cli" | "stdlib.env" | "stdlib.fs"
            )
            || selected.text != item.binding.text
            || selected.backticked != item.binding.backticked
        {
            return false;
        }
        functions.contains(&(item.module.clone(), selected.text.clone()))
    }

    /// 递归移除可静态链接的标准库函数导入语句。
    fn strip(functions: &BTreeSet<(String, String)>, statements: &mut Vec<IrStatement>) {
        for statement in statements.iter_mut() {
            match &mut statement.kind {
                IrStatementKind::Import { items, .. } => {
                    items.retain(|item| !is_static_function_import(functions, item));
                }
                IrStatementKind::If {
                    body,
                    elif_branches,
                    else_body,
                    ..
                } => {
                    strip(functions, body);
                    for branch in elif_branches {
                        strip(functions, &mut branch.body);
                    }
                    if let Some(body) = else_body {
                        strip(functions, body);
                    }
                }
                IrStatementKind::For { body, .. }
                | IrStatementKind::While { body, .. }
                | IrStatementKind::Function { body, .. } => strip(functions, body),
                IrStatementKind::Try {
                    body,
                    catches,
                    finally_body,
                } => {
                    strip(functions, body);
                    for catch in catches {
                        strip(functions, &mut catch.body);
                    }
                    if let Some(body) = finally_body {
                        strip(functions, body);
                    }
                }
                _ => {}
            }
        }
        statements.retain(|statement| {
            !matches!(&statement.kind, IrStatementKind::Import { items, .. } if items.is_empty())
        });
    }

    let functions = program
        .modules
        .iter()
        .filter(|module| module.name.starts_with("stdlib.") && module.kind == "file")
        .flat_map(|module| {
            module
                .symbols
                .iter()
                .filter(|symbol| symbol.kind == "function")
                .map(|symbol| (module.name.clone(), symbol.name.clone()))
        })
        .collect::<BTreeSet<_>>();
    strip(&functions, &mut program.body);
}
