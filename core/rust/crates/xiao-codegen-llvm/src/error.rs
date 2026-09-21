//! 后端错误值；错误不依赖宿主本地化文本。

use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use xiao_ir::IrSpan;

/// 后端操作的统一结果类型。
pub type Result<T> = std::result::Result<T, CodegenError>;

/// LLVM 后端拒绝或工具链失败的结构化原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodegenError {
    /// 输入 IR 版本不是后端支持的版本。
    IrVersion {
        /// 后端要求的版本。
        expected: u32,
        /// 输入实际版本。
        actual: u32,
    },
    /// 输入 IR 的结构验证失败。
    InvalidIr {
        /// 验证器提供的原因。
        message: String,
    },
    /// N0-A 尚未覆盖的语言构造。
    Unsupported {
        /// 被拒绝的语言构造。
        feature: String,
        /// 相关源码区间。
        span: Option<IrSpan>,
    },
    /// 目标描述不完整或不一致。
    InvalidTarget {
        /// 目标描述错误。
        message: String,
    },
    /// 工具链字段缺失或无法执行。
    ToolchainUnavailable {
        /// 不可用的工具名称。
        tool: String,
        /// 宿主错误原因。
        message: String,
    },
    /// 外部 LLVM 工具返回失败。
    ToolchainFailed {
        /// 工具名称。
        tool: String,
        /// 进程退出码；无法取得时为空。
        status: Option<i32>,
        /// 标准错误输出。
        stderr: String,
    },
    /// 文件输入/输出失败。
    Io {
        /// 相关路径。
        path: PathBuf,
        /// 文件系统错误。
        message: String,
    },
    /// 生成的 LLVM 文本没有通过验证。
    InvalidLlvm {
        /// 验证错误原因。
        message: String,
    },
}

impl Display for CodegenError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::IrVersion { expected, actual } => {
                write!(formatter, "IR 版本不兼容：需要 {expected}，收到 {actual}")
            }
            Self::InvalidIr { message } => write!(formatter, "IR 无效：{message}"),
            Self::Unsupported { feature, span } => {
                write!(formatter, "N0-A 不支持 {feature}")?;
                if let Some(span) = span {
                    write!(formatter, "（{}..{}）", span.start, span.end)?;
                }
                Ok(())
            }
            Self::InvalidTarget { message } => write!(formatter, "目标描述无效：{message}"),
            Self::ToolchainUnavailable { tool, message } => {
                write!(formatter, "工具链不可用（{tool}）：{message}")
            }
            Self::ToolchainFailed {
                tool,
                status,
                stderr,
            } => write!(formatter, "工具 {tool} 失败（状态 {status:?}）：{stderr}"),
            Self::Io { path, message } => write!(formatter, "{}：{message}", path.display()),
            Self::InvalidLlvm { message } => write!(formatter, "LLVM IR 无效：{message}"),
        }
    }
}

impl std::error::Error for CodegenError {}

impl From<std::io::Error> for CodegenError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            path: PathBuf::from("<stream>"),
            message: error.to_string(),
        }
    }
}
