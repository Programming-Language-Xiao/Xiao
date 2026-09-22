//! X0-A 子进程协议、长度前缀帧和 Rust 核心入口。
//!
//! 协议层只负责传输、版本协商和结果映射；源码解析、类型检查、生命周期分析、
//! 字节码降低、VM 执行和 LLVM 构建仍分别由已有驱动器负责。帧的长度字段是 8 字节
//! 大端无符号整数，只计算 UTF-8 JSON 负载；解码器在分配前检查 16 MiB 上限。

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use xiao_bytecode::FORMAT_VERSION;
use xiao_codegen_llvm::{
    CODEGEN_VERSION, CodegenOptions, Endian, ObjectFormat, TargetDescription, Toolchain,
    ToolchainVersions,
};
use xiao_diagnostics::window::{
    DIAGNOSTIC_START_CODE, DiagnosticActivation, activation_path, write_activation,
};
use xiao_diagnostics::{
    Diagnostic, DiagnosticParam, FrameKind, ReportClass, ReportRecord, Severity, StackFrame,
};
use xiao_runtime::RuntimeValue;
use xiao_runtime_abi::ABI_ENCODED_VERSION;
use xiao_vm::{VmEvent, VmOptions};

use crate::diagnostics::{DiagnosticOptions, DiagnosticSession, start_error_details};
use crate::frontend::{FrontendContext, FrontendRequest};
use crate::native::{FrontendNativeDriver, NativeBuildRequest, NativeDriverError};
use crate::run::{
    CancellationToken, DRIVER_VERSION, DriverError, DriverExecution, DriverOutcome, DriverPhase,
    DriverRequest, ExitCode, FrontendVmDriver,
};

/// 当前协议版本。协议字段和帧布局变化时必须递增。
pub const PROTOCOL_VERSION: u16 = 1;
/// CLI 与核心比较的统一兼容版本；组件版本只作为诊断信息返回。
pub const CORE_VERSION: u32 = 1;
/// 长度字段的固定字节数。
pub const FRAME_LENGTH_BYTES: usize = 8;
/// 单帧 JSON 负载的最大字节数。
pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// 帧或协议错误：稳定编号不依赖本地化文本。
pub const FRAME_ERROR_CODE: &str = "X11-PROTOCOL-001";
/// 请求字段或 JSON 形状非法。
pub const REQUEST_ERROR_CODE: &str = "X11-PROTOCOL-002";
/// 核心处理请求时发生 panic。
pub const CORE_CRASH_CODE: &str = "X11-PROTOCOL-003";
/// 协议版本或统一核心版本不兼容。
pub const VERSION_MISMATCH_CODE: &str = "X11-PROTOCOL-004";
/// 协议请求主动取消。
pub const CANCELLED_ERROR_CODE: &str = "X11-PROTOCOL-005";
/// X0-A 尚未支持的协议操作或配置。
pub const UNSUPPORTED_OPERATION_CODE: &str = "X11-PROTOCOL-006";
/// 原生构建驱动器错误的协议包装编号。
pub const BUILD_ERROR_CODE: &str = "X11-PROTOCOL-007";

/// 核心各组件的诊断版本；兼容判断只使用 [`CORE_VERSION`]。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoreVersions {
    /// 协议版本。
    pub protocol: u16,
    /// 统一核心兼容版本。
    pub core: u32,
    /// 语言语义版本。
    pub language: String,
    /// Runtime ABI 编码版本。
    pub runtime_abi: u64,
    /// 前端流水线版本。
    pub frontend: u32,
    /// 内部驱动器版本。
    pub driver: u32,
    /// 字节码格式版本。
    pub bytecode_format: u8,
    /// LLVM 代码生成接口版本。
    pub codegen: u32,
}

impl CoreVersions {
    /// 返回当前 Rust 核心的版本清单。
    #[must_use]
    pub fn current() -> Self {
        Self {
            protocol: PROTOCOL_VERSION,
            core: CORE_VERSION,
            language: "0.1.0".to_owned(),
            runtime_abi: ABI_ENCODED_VERSION,
            frontend: crate::FRONTEND_VERSION,
            driver: DRIVER_VERSION,
            bytecode_format: FORMAT_VERSION,
            codegen: CODEGEN_VERSION,
        }
    }
}

/// 目标条件；运行请求会把 `triple` 传给前端上下文，构建请求还会转换成 LLVM 目标。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolTarget {
    /// LLVM target triple 或 `host`。
    pub triple: String,
    /// 目标指针宽度。
    pub pointer_width: u16,
    /// `little` 或 `big`。
    pub endian: String,
    /// `coff`、`elf` 或 `macho`。
    pub object_format: String,
}

impl ProtocolTarget {
    /// 返回当前宿主的规范化目标。
    #[must_use]
    pub fn host() -> Self {
        Self::from_target(TargetDescription::host())
    }

    /// 从 LLVM 目标描述建立协议目标。
    #[must_use]
    pub fn from_target(target: TargetDescription) -> Self {
        Self {
            triple: target.triple,
            pointer_width: target.pointer_width,
            endian: match target.endian {
                Endian::Little => "little".to_owned(),
                Endian::Big => "big".to_owned(),
            },
            object_format: match target.object_format {
                ObjectFormat::Coff => "coff".to_owned(),
                ObjectFormat::Elf => "elf".to_owned(),
                ObjectFormat::MachO => "macho".to_owned(),
            },
        }
    }

    /// 将协议目标校验并转换为 LLVM 目标描述。
    pub fn to_target(&self) -> Result<TargetDescription, ProtocolError> {
        let endian = match self.endian.as_str() {
            "little" => Endian::Little,
            "big" => Endian::Big,
            _ => {
                return Err(ProtocolError::request(
                    "target.endian",
                    "必须是 little 或 big",
                ));
            }
        };
        let object_format = match self.object_format.as_str() {
            "coff" => ObjectFormat::Coff,
            "elf" => ObjectFormat::Elf,
            "macho" => ObjectFormat::MachO,
            _ => {
                return Err(ProtocolError::request(
                    "target.object_format",
                    "必须是 coff、elf 或 macho",
                ));
            }
        };
        TargetDescription::new(
            self.triple.clone(),
            self.pointer_width,
            endian,
            object_format,
        )
        .map_err(|error| ProtocolError::request("target", error.to_string()))
    }
}

/// 优化与调试配置。优化级别在 X0-A 只接受 `0`，字段先进入稳定协议。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct OptimizationConfig {
    /// 优化级别，X0-A 仅支持零。
    pub level: u8,
    /// 是否请求调试元数据；诊断窗口留给 X0-D。
    pub debug: bool,
    /// 可选的诊断等级和输出配置；未提供时使用 Runtime 默认值。
    #[serde(default)]
    pub diagnostics: Option<DiagnosticConfig>,
}

/// `-debug` 诊断参数的窄协议镜像。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticConfig {
    /// 终端等级。
    pub terminal_level: Option<String>,
    /// 文件等级。
    pub file_level: Option<String>,
    /// 日志目录。
    pub log_dir: Option<String>,
    /// 总日志文件。
    pub log_file: Option<String>,
    /// 堆栈详细程度。
    pub stacktrace: Option<String>,
    /// 模块/源码聚焦规则。
    #[serde(default)]
    pub focus: Vec<DiagnosticFocusConfig>,
}

/// 一个诊断聚焦规则。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticFocusConfig {
    /// 模块匹配项。
    pub module: Option<String>,
    /// 源码匹配项。
    pub source: Option<String>,
    /// 输出文件。
    pub output: String,
    /// 文件等级。
    pub level: Option<String>,
    /// 是否镜像到总日志。
    #[serde(default)]
    pub mirror: bool,
}

/// 源码与模块身份；协议传递真实 Xiao 源码，不接受手写 TAC。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceIdentity {
    /// 逻辑模块名。
    pub module: String,
    /// 可选的源文件路径。
    pub path: Option<String>,
    /// UTF-8 Xiao 源码文本。
    pub text: String,
}

/// VM 请求参数。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunOptions {
    /// 最大调用深度。
    pub max_call_depth: usize,
    /// 事件接收器容量。
    pub event_capacity: usize,
    /// 可选的驱动器边界超时（毫秒）。
    pub timeout_ms: Option<u64>,
}

impl Default for RunOptions {
    /// 返回与生产 VM 一致的默认调用深度和事件容量。
    fn default() -> Self {
        Self {
            max_call_depth: VmOptions::default().max_call_depth,
            event_capacity: xiao_vm::DEFAULT_EVENT_CAPACITY,
            timeout_ms: None,
        }
    }
}

/// LLVM 工具链描述；工具路径由调用方注入，核心不会自行搜索 PATH。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolchainSpec {
    /// clang 可执行文件路径。
    pub clang: String,
    /// 可选 llvm-as 路径。
    pub llvm_as: Option<String>,
    /// 可选 llc 路径。
    pub llc: Option<String>,
    /// 可选 Runtime 静态库路径。
    pub runtime_library: Option<String>,
    /// 链接所需的原生库参数。
    pub native_static_libraries: Vec<String>,
    /// 已探测的工具版本文本。
    pub versions: ToolchainVersionsSpec,
}

/// 工具链版本文本，不参与核心协议兼容判断。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolchainVersionsSpec {
    /// clang 版本首行。
    pub clang: String,
    /// llvm-as 版本首行。
    pub llvm_as: Option<String>,
    /// llc 版本首行。
    pub llc: Option<String>,
    /// rustc 版本首行。
    pub rustc: Option<String>,
}

/// Rust/TypeScript 共享的请求消息。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum ProtocolRequest {
    /// 首条消息必须用于版本协商。
    Hello {
        /// 客户端请求编号。
        request_id: String,
        /// 客户端支持的协议版本。
        protocol_version: u16,
        /// 客户端要求的统一核心版本。
        core_version: u32,
    },
    /// 使用真实源码调用 `FrontendVmDriver`。
    Run {
        /// 请求编号，用于取消和响应关联。
        request_id: String,
        /// 协议版本。
        protocol_version: u16,
        /// 统一核心版本。
        core_version: u32,
        /// 语言版本上下文。
        language_version: String,
        /// Runtime 版本提示。
        runtime_version: String,
        /// 目标条件。
        target: ProtocolTarget,
        /// 优化/调试配置。
        optimization: OptimizationConfig,
        /// 源码与模块身份。
        source: SourceIdentity,
        /// VM 参数。
        options: RunOptions,
    },
    /// 使用真实源码调用 `FrontendNativeDriver`。
    Build {
        /// 请求编号。
        request_id: String,
        /// 协议版本。
        protocol_version: u16,
        /// 统一核心版本。
        core_version: u32,
        /// 语言版本上下文。
        language_version: String,
        /// Runtime 版本提示。
        runtime_version: String,
        /// 目标条件。
        target: ProtocolTarget,
        /// 优化/调试配置。
        optimization: OptimizationConfig,
        /// 源码与模块身份。
        source: SourceIdentity,
        /// 产物输出路径。
        output: String,
        /// 可选 LLVM 文本输出路径。
        llvm_ir_output: Option<String>,
        /// 外部工具链描述。
        toolchain: ToolchainSpec,
    },
    /// 请求取消另一个正在执行的请求。
    Cancel {
        /// 取消消息自身的编号。
        request_id: String,
        /// 协议版本。
        protocol_version: u16,
        /// 统一核心版本。
        core_version: u32,
        /// 要取消的运行/构建请求编号。
        target_request_id: String,
    },
    /// 请求核心优雅结束。
    Shutdown {
        /// 请求编号。
        request_id: String,
        /// 协议版本。
        protocol_version: u16,
        /// 统一核心版本。
        core_version: u32,
    },
}

/// 结构化消息参数，保留原始类型以便 CLI 本地化。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ProtocolParam {
    /// 文本参数。
    Text(String),
    /// 整数参数。
    Integer(i128),
    /// 布尔参数。
    Boolean(bool),
}

/// 源码区间。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolSpan {
    /// 起始字节偏移。
    pub start: usize,
    /// 结束字节偏移（不包含）。
    pub end: usize,
}

/// 结构化前端诊断。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolDiagnostic {
    /// 稳定诊断码。
    pub code: String,
    /// 消息目录键。
    pub message_id: String,
    /// 严重级别。
    pub severity: String,
    /// 可选源码区间。
    pub span: Option<ProtocolSpan>,
    /// 未本地化参数。
    pub params: BTreeMap<String, ProtocolParam>,
    /// 当前语言预览文本。
    pub message: String,
}

/// 统一调用栈帧。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolStackFrame {
    /// 模块名。
    pub module: String,
    /// 函数名。
    pub function: String,
    /// 可选源码路径。
    pub source: Option<String>,
    /// 源码区间。
    pub span: Option<ProtocolSpan>,
    /// 后端字节码/原生位置。
    pub backend: ProtocolBackendLocation,
    /// `user` 或 `runtime`。
    pub kind: String,
}

/// 后端位置摘要。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolBackendLocation {
    /// 可选字节码偏移。
    pub bytecode_offset: Option<u64>,
    /// 可选原生地址。
    pub native_address: Option<u64>,
    /// 可选内联深度。
    pub inline_depth: Option<u32>,
}

/// 统一错误报告；`cause` 和 `suppressed` 保留递归结构。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolReport {
    /// `recoverable` 或 `fatal`。
    pub class: String,
    /// 稳定错误码。
    pub code: String,
    /// 进程内错误事件编号。
    pub error_id: u64,
    /// 消息目录键。
    pub message_id: String,
    /// 未本地化参数。
    pub params: BTreeMap<String, ProtocolParam>,
    /// 当前语言预览文本。
    pub message: String,
    /// 直接源码位置。
    pub location: Option<ProtocolSpan>,
    /// 操作上下文。
    pub context: BTreeMap<String, ProtocolParam>,
    /// 调用栈。
    pub stack: Vec<ProtocolStackFrame>,
    /// 原因链。
    pub cause: Option<Box<ProtocolReport>>,
    /// 清理阶段次生错误。
    pub suppressed: Vec<ProtocolReport>,
}

/// 一条机器可读诊断事件。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolEvent {
    /// 稳定事件类型名。
    pub kind: String,
    /// 事件字段，不依赖人类可读文本解析。
    pub data: BTreeMap<String, Value>,
}

/// VM 聚合指标。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolMetrics {
    /// 执行指令数。
    pub instructions: u64,
    /// 最大调用深度。
    pub max_call_depth: usize,
    /// 最大栈深度。
    pub max_stack_depth: usize,
    /// 释放数量。
    pub releases: usize,
    /// spill 次数。
    pub spill_count: u64,
    /// 栈映射点数量。
    pub stack_map_entries: usize,
    /// 跨调用保存次数。
    pub call_save_count: u64,
    /// 被容量丢弃的事件数。
    pub dropped_events: usize,
}

/// 入口返回值的稳定摘要；复杂 Runtime 对象不跨进程暴露内部句柄。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolValue {
    /// Xiao Runtime 类型名。
    pub kind: String,
    /// 标量文本或复杂对象的稳定摘要。
    pub value: String,
}

/// 协议错误本体。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolErrorBody {
    /// 稳定机器码。
    pub code: String,
    /// 消息目录键。
    pub message_id: String,
    /// 未本地化的预览文本。
    pub message: String,
    /// 可选阶段名。
    pub phase: Option<String>,
    /// 可执行的下一步。
    pub next_step: Option<String>,
    /// 机器字段扩展。
    pub details: BTreeMap<String, Value>,
}

/// Rust/TypeScript 共享的响应消息。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum ProtocolResponse {
    /// 版本协商结果。
    Hello {
        /// 对应请求编号。
        request_id: String,
        /// 是否接受客户端版本。
        accepted: bool,
        /// 当前协议版本。
        protocol_version: u16,
        /// 当前统一核心版本。
        core_version: u32,
        /// 组件诊断版本。
        versions: CoreVersions,
        /// 可调用操作。
        capabilities: Vec<String>,
        /// 版本不兼容时的结构化错误。
        error: Option<ProtocolErrorBody>,
    },
    /// 运行或构建完成结果。
    Result {
        /// 对应请求编号。
        request_id: String,
        /// `run` 或 `build`。
        operation: String,
        /// B0-D 冻结的进程码。
        exit_code: u8,
        /// 稳定退出语义名称。
        exit_name: String,
        /// 前端警告/错误诊断。
        diagnostics: Vec<ProtocolDiagnostic>,
        /// VM 或后端报告。
        report: Option<ProtocolReport>,
        /// VM 事件。
        events: Vec<ProtocolEvent>,
        /// VM 指标。
        metrics: Option<ProtocolMetrics>,
        /// 可选入口值摘要。
        value: Option<ProtocolValue>,
        /// 可选原生构建产物摘要。
        artifact: Option<ProtocolArtifact>,
    },
    /// 请求或协议层失败。
    Error {
        /// 可能为空（例如帧损坏发生在请求编号可读之前）。
        request_id: Option<String>,
        /// 协议错误。
        error: ProtocolErrorBody,
        /// 可选的驱动器/VM 结构化报告。
        #[serde(default)]
        report: Option<ProtocolReport>,
        /// 对应的稳定进程码；核心崩溃为 `4`。
        exit_code: u8,
    },
    /// 取消确认；目标请求还会随后返回 `exit_code = 2` 的结果。
    Cancelled {
        /// 取消消息编号。
        request_id: String,
        /// 被取消的请求编号。
        target_request_id: String,
        /// 是否找到目标。
        accepted: bool,
        /// `ArtifactRejected` 的冻结进程码。
        exit_code: u8,
    },
    /// 关闭确认。
    Shutdown {
        /// 对应请求编号。
        request_id: String,
    },
}

/// 原生构建产物摘要。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolArtifact {
    /// 可执行文件路径。
    pub executable: String,
    /// LLVM 文本路径（若请求写入）。
    pub llvm_ir_output: Option<String>,
    /// 工具链指纹。
    pub toolchain_fingerprint: String,
    /// 是否依赖 Runtime ABI。
    pub uses_runtime: bool,
    /// Runtime 组件列表。
    pub runtime_components: Vec<String>,
    /// `-debug` 构建生成的持久激活位；普通构建为空。
    #[serde(default)]
    pub diagnostic_activation: Option<ProtocolDiagnosticActivation>,
}

/// 产物中诊断激活元数据的协议摘要。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolDiagnosticActivation {
    /// 激活元数据文件路径。
    pub path: String,
    /// 是否强制开窗。
    pub enabled: bool,
    /// 源码映射是否随产物声明。
    pub source_map: bool,
    /// Runtime 钩子能力是否声明。
    pub hooks: bool,
}

/// 帧解码错误。
#[derive(Debug)]
pub enum FrameError {
    /// 底层 I/O 错误。
    Io(io::Error),
    /// 长度字段超过协议上限。
    LengthTooLarge {
        /// 收到的长度。
        length: u64,
        /// 允许的最大长度。
        maximum: usize,
    },
    /// 长度字段只读到一部分。
    TruncatedLength {
        /// 已读取字节数。
        read: usize,
    },
    /// JSON 负载只读到一部分。
    TruncatedPayload {
        /// 声明的负载长度。
        expected: usize,
        /// 实际读取字节数。
        read: usize,
    },
    /// 负载不是 UTF-8。
    InvalidUtf8,
    /// JSON 语法或形状非法。
    InvalidJson(String),
}

impl FrameError {
    /// 返回稳定错误码。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        FRAME_ERROR_CODE
    }
}

impl Display for FrameError {
    /// 将帧错误渲染为调试摘要；调用方仍应读取 [`FrameError::code`]。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "帧 I/O 失败：{error}"),
            Self::LengthTooLarge { length, maximum } => {
                write!(formatter, "帧长度 {length} 超过上限 {maximum}")
            }
            Self::TruncatedLength { read } => {
                write!(
                    formatter,
                    "长度字段被截断：已读取 {read}/{FRAME_LENGTH_BYTES} 字节"
                )
            }
            Self::TruncatedPayload { expected, read } => {
                write!(formatter, "JSON 负载被截断：需要 {expected}，实际 {read}")
            }
            Self::InvalidUtf8 => formatter.write_str("JSON 负载不是 UTF-8"),
            Self::InvalidJson(error) => write!(formatter, "JSON 负载无效：{error}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<io::Error> for FrameError {
    /// 将底层流错误归入帧错误。
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// 协议层验证错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolError {
    code: &'static str,
    field: Option<String>,
    message: String,
}

impl ProtocolError {
    /// 创建请求字段错误。
    fn request(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: REQUEST_ERROR_CODE,
            field: Some(field.into()),
            message: message.into(),
        }
    }

    /// 创建版本不兼容错误。
    fn version(message: impl Into<String>) -> Self {
        Self {
            code: VERSION_MISMATCH_CODE,
            field: None,
            message: message.into(),
        }
    }

    /// 返回稳定错误码。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// 返回出错字段。
    #[must_use]
    pub fn field(&self) -> Option<&str> {
        self.field.as_deref()
    }

    /// 返回开发者原因。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for ProtocolError {
    /// 将协议错误渲染为日志摘要。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(field) = &self.field {
            write!(formatter, "{}: {} ({})", self.code, field, self.message)
        } else {
            write!(formatter, "{}: {}", self.code, self.message)
        }
    }
}

impl std::error::Error for ProtocolError {}

/// 将 JSON 值编码为一帧，长度只包含 JSON 字节。
pub fn encode_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, FrameError> {
    let payload =
        serde_json::to_vec(value).map_err(|error| FrameError::InvalidJson(error.to_string()))?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameError::LengthTooLarge {
            length: payload.len() as u64,
            maximum: MAX_FRAME_BYTES,
        });
    }
    let length = u64::try_from(payload.len()).map_err(|_| FrameError::LengthTooLarge {
        length: u64::MAX,
        maximum: MAX_FRAME_BYTES,
    })?;
    let mut frame = Vec::with_capacity(FRAME_LENGTH_BYTES + payload.len());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// 从一帧 JSON 负载解码消息。
pub fn decode_frame<T: DeserializeOwned>(payload: &[u8]) -> Result<T, FrameError> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameError::LengthTooLarge {
            length: payload.len() as u64,
            maximum: MAX_FRAME_BYTES,
        });
    }
    std::str::from_utf8(payload).map_err(|_| FrameError::InvalidUtf8)?;
    serde_json::from_slice(payload).map_err(|error| FrameError::InvalidJson(error.to_string()))
}

/// 从流中读取一帧；干净 EOF 返回 `Ok(None)`。
pub fn read_frame<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>, FrameError> {
    let mut length_bytes = [0_u8; FRAME_LENGTH_BYTES];
    let mut read = 0;
    while read < FRAME_LENGTH_BYTES {
        match reader.read(&mut length_bytes[read..])? {
            0 if read == 0 => return Ok(None),
            0 => return Err(FrameError::TruncatedLength { read }),
            amount => read += amount,
        }
    }
    let length = u64::from_be_bytes(length_bytes);
    if length > MAX_FRAME_BYTES as u64 {
        return Err(FrameError::LengthTooLarge {
            length,
            maximum: MAX_FRAME_BYTES,
        });
    }
    let expected = usize::try_from(length).map_err(|_| FrameError::LengthTooLarge {
        length,
        maximum: MAX_FRAME_BYTES,
    })?;
    let mut payload = vec![0_u8; expected];
    let mut received = 0;
    while received < expected {
        match reader.read(&mut payload[received..])? {
            0 => {
                return Err(FrameError::TruncatedPayload {
                    expected,
                    read: received,
                });
            }
            amount => received += amount,
        }
    }
    Ok(Some(payload))
}

/// 将 JSON 值编码并写入流，然后刷新输出。
pub fn write_frame<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<(), FrameError> {
    let frame = encode_frame(value)?;
    writer.write_all(&frame)?;
    writer.flush()?;
    Ok(())
}

/// 从流读取并解码一条请求。
pub fn read_request<R: Read>(reader: &mut R) -> Result<Option<ProtocolRequest>, FrameError> {
    let Some(payload) = read_frame(reader)? else {
        return Ok(None);
    };
    decode_frame(&payload).map(Some)
}

/// 将诊断参数映射到协议类型。
#[must_use]
pub fn protocol_param(value: &DiagnosticParam) -> ProtocolParam {
    match value {
        DiagnosticParam::Text(value) => ProtocolParam::Text(value.clone()),
        DiagnosticParam::Integer(value) => ProtocolParam::Integer(*value),
        DiagnosticParam::Boolean(value) => ProtocolParam::Boolean(*value),
    }
}

/// 转换一张诊断参数表，并保持稳定的键排序。
fn protocol_params(values: &BTreeMap<String, DiagnosticParam>) -> BTreeMap<String, ProtocolParam> {
    values
        .iter()
        .map(|(key, value)| (key.clone(), protocol_param(value)))
        .collect()
}

/// 将内部源码区间转换为协议区间。
fn protocol_span(span: Option<xiao_source::SourceSpan>) -> Option<ProtocolSpan> {
    span.map(|span| ProtocolSpan {
        start: span.start(),
        end: span.end(),
    })
}

/// 将前端诊断转换为协议诊断。
#[must_use]
pub fn protocol_diagnostic(diagnostic: &Diagnostic) -> ProtocolDiagnostic {
    ProtocolDiagnostic {
        code: diagnostic.code().to_owned(),
        message_id: diagnostic.message_id().to_owned(),
        severity: match diagnostic.severity() {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
        .to_owned(),
        span: protocol_span(diagnostic.span()),
        params: protocol_params(diagnostic.params()),
        message: diagnostic.message().to_owned(),
    }
}

/// 将统一调用栈帧转换为跨进程摘要。
fn protocol_stack_frame(frame: &StackFrame) -> ProtocolStackFrame {
    ProtocolStackFrame {
        module: frame.module.clone(),
        function: frame.function.clone(),
        source: frame.source.clone(),
        span: protocol_span(frame.span),
        backend: ProtocolBackendLocation {
            bytecode_offset: frame.backend.bytecode_offset,
            native_address: frame.backend.native_address,
            inline_depth: frame.backend.inline_depth,
        },
        kind: match frame.kind {
            FrameKind::User => "user",
            FrameKind::Runtime => "runtime",
        }
        .to_owned(),
    }
}

/// 递归转换统一错误报告及其原因链。
fn protocol_report(report: &ReportRecord) -> ProtocolReport {
    ProtocolReport {
        class: match report.class {
            ReportClass::Recoverable => "recoverable",
            ReportClass::Fatal => "fatal",
        }
        .to_owned(),
        code: report.code.clone(),
        error_id: report.error_id,
        message_id: report.message_id.clone(),
        params: protocol_params(&report.params),
        message: report.message.clone(),
        location: protocol_span(report.location),
        context: protocol_params(&report.context),
        stack: report.stack.iter().map(protocol_stack_frame).collect(),
        cause: report.cause.as_deref().map(protocol_report).map(Box::new),
        suppressed: report.suppressed.iter().map(protocol_report).collect(),
    }
}

/// 将 VM 指标和事件丢弃计数转换为协议结构。
fn protocol_metrics(metrics: xiao_vm::VmMetrics, dropped_events: usize) -> ProtocolMetrics {
    ProtocolMetrics {
        instructions: metrics.instructions,
        max_call_depth: metrics.max_call_depth,
        max_stack_depth: metrics.max_stack_depth,
        releases: metrics.releases,
        spill_count: metrics.spill_count,
        stack_map_entries: metrics.stack_map_entries,
        call_save_count: metrics.call_save_count,
        dropped_events,
    }
}

/// 将 VM 事件转换为稳定类型名和机器字段。
fn protocol_event(event: &VmEvent) -> ProtocolEvent {
    let (kind, data) = match event {
        VmEvent::ModuleLoaded { module } => ("module_loaded", json!({ "module": module })),
        VmEvent::FunctionEntered { function, depth } => (
            "function_entered",
            json!({ "function": function, "depth": depth }),
        ),
        VmEvent::FunctionReturned { function, depth } => (
            "function_returned",
            json!({ "function": function, "depth": depth }),
        ),
        VmEvent::ScopeEntered { scope } => ("scope_entered", json!({ "scope": scope })),
        VmEvent::ScopeExited { scope, exit } => {
            ("scope_exited", json!({ "scope": scope, "exit": exit }))
        }
        VmEvent::HandlerEntered { scope, handler } => (
            "handler_entered",
            json!({ "scope": scope, "handler": handler }),
        ),
        VmEvent::HandlerMatched {
            scope,
            handler,
            catch_type,
        } => (
            "handler_matched",
            json!({ "scope": scope, "handler": handler, "catch_type": catch_type }),
        ),
        VmEvent::HandlerUnmatched { scope } => ("handler_unmatched", json!({ "scope": scope })),
        VmEvent::ValueReleased {
            scope,
            exit,
            value,
            kind,
        } => (
            "value_released",
            json!({ "scope": scope, "exit": exit, "value": value, "kind": kind }),
        ),
        VmEvent::ErrorRaised { code, message_id } => (
            "error_raised",
            json!({ "code": code, "message_id": message_id }),
        ),
        VmEvent::FatalRaised { code } => ("fatal_raised", json!({ "code": code })),
        VmEvent::StackFrame {
            function,
            depth,
            return_to,
        } => (
            "stack_frame",
            json!({
                "function": function,
                "depth": depth,
                "return_to": return_to.map(|value| value.get()),
            }),
        ),
        VmEvent::BackendLocationMissing {
            function,
            block,
            instruction,
        } => (
            "backend_location_missing",
            json!({ "function": function, "block": block, "instruction": instruction }),
        ),
        VmEvent::Metrics {
            instructions,
            max_call_depth,
            max_stack_depth,
            releases,
            spill_count,
            stack_map_entries,
            call_save_count,
            dropped_events,
        } => (
            "metrics",
            json!({
                "instructions": instructions,
                "max_call_depth": max_call_depth,
                "max_stack_depth": max_stack_depth,
                "releases": releases,
                "spill_count": spill_count,
                "stack_map_entries": stack_map_entries,
                "call_save_count": call_save_count,
                "dropped_events": dropped_events,
            }),
        ),
    };
    let data = data
        .as_object()
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default();
    ProtocolEvent {
        kind: kind.to_owned(),
        data,
    }
}

/// 将入口值转换为不泄露 Runtime 句柄的稳定摘要。
fn protocol_value(value: &RuntimeValue) -> ProtocolValue {
    let kind = value.type_name();
    let text = match value {
        RuntimeValue::Int(value) => value.to_string(),
        RuntimeValue::Sint(value) => value.to_string(),
        RuntimeValue::Lint(value) | RuntimeValue::Lfloat(value) => value.clone(),
        RuntimeValue::Float(value) => value.to_string(),
        RuntimeValue::Sfloat(value) => value.to_string(),
        RuntimeValue::Bool(value) => value.to_string(),
        RuntimeValue::Str(value) => value
            .to_string()
            .unwrap_or_else(|_| "<invalid-string-handle>".to_owned()),
        RuntimeValue::None => "none".to_owned(),
        RuntimeValue::Table(_)
        | RuntimeValue::TableDropView(_)
        | RuntimeValue::Array(_)
        | RuntimeValue::Tuple(_)
        | RuntimeValue::DictTable(_)
        | RuntimeValue::DictColumn(_)
        | RuntimeValue::Set(_)
        | RuntimeValue::Error(_) => format!("<{kind}>"),
    };
    ProtocolValue { kind, value: text }
}

/// 返回冻结退出码对应的稳定语义名称。
fn exit_name(code: ExitCode) -> &'static str {
    match code {
        ExitCode::Success => "success",
        ExitCode::SourceRejected => "source_rejected",
        ExitCode::ArtifactRejected => "artifact_rejected",
        ExitCode::RuntimeError => "runtime_error",
        ExitCode::Fatal => "fatal",
    }
}

/// 创建带下一步建议和结构化字段的协议错误体。
fn protocol_error_body(
    code: impl Into<String>,
    message_id: impl Into<String>,
    message: impl Into<String>,
    phase: Option<String>,
    next_step: Option<String>,
    details: BTreeMap<String, Value>,
) -> ProtocolErrorBody {
    ProtocolErrorBody {
        code: code.into(),
        message_id: message_id.into(),
        message: message.into(),
        phase,
        next_step,
        details,
    }
}

/// 将内部协议验证错误转换为跨语言错误体。
fn protocol_error_from_error(error: &ProtocolError) -> ProtocolErrorBody {
    let mut details = BTreeMap::new();
    if let Some(field) = error.field() {
        details.insert("field".to_owned(), Value::String(field.to_owned()));
    }
    protocol_error_body(
        error.code(),
        "x11.protocol.request",
        error.message(),
        Some("protocol".to_owned()),
        Some("修正请求字段后重试".to_owned()),
        details,
    )
}

/// 校验协议版本和统一核心版本。
fn validate_versions(protocol_version: u16, core_version: u32) -> Result<(), ProtocolError> {
    if protocol_version != PROTOCOL_VERSION {
        return Err(ProtocolError::version(format!(
            "协议版本不兼容：需要 {}，收到 {}",
            PROTOCOL_VERSION, protocol_version
        )));
    }
    if core_version != CORE_VERSION {
        return Err(ProtocolError::version(format!(
            "核心版本不兼容：需要 {}，收到 {}",
            CORE_VERSION, core_version
        )));
    }
    Ok(())
}

/// 将协议源码字段转换为既有前端请求。
fn frontend_request(
    source: &SourceIdentity,
    language_version: &str,
    target: &ProtocolTarget,
) -> FrontendRequest {
    let mut context = FrontendContext::host();
    context.language_version = language_version.to_owned();
    context.target = target.triple.clone();
    let request = match &source.path {
        Some(path) => FrontendRequest::from_text_at(source.text.clone(), path.clone()),
        None => FrontendRequest::from_text(source.text.clone()),
    };
    request.with_context(context)
}

/// 校验源码和模块身份的最小边界。
fn validate_source(source: &SourceIdentity) -> Result<(), ProtocolError> {
    if source.module.trim().is_empty() {
        return Err(ProtocolError::request("source.module", "模块名不能为空"));
    }
    Ok(())
}

/// 校验目标字段而不复制 LLVM 目标语义。
fn validate_target(target: &ProtocolTarget) -> Result<(), ProtocolError> {
    target.to_target().map(|_| ())
}

/// 将协议 VM 参数交给生产 VM 自身的范围校验。
fn run_options(
    options: &RunOptions,
) -> Result<(VmOptions, usize, Option<Duration>), ProtocolError> {
    let vm_options = VmOptions {
        max_call_depth: options.max_call_depth,
    };
    vm_options
        .validate()
        .map_err(|error| ProtocolError::request("options.max_call_depth", error.to_string()))?;
    if options.event_capacity == 0 || options.event_capacity > xiao_vm::MAX_EVENT_CAPACITY {
        return Err(ProtocolError::request(
            "options.event_capacity",
            format!(
                "必须位于 1..={}（收到 {}）",
                xiao_vm::MAX_EVENT_CAPACITY,
                options.event_capacity
            ),
        ));
    }
    let timeout = options.timeout_ms.map(Duration::from_millis);
    Ok((vm_options, options.event_capacity, timeout))
}

/// 将三段驱动器结果转换为运行响应。
fn run_response(request_id: String, outcome: DriverOutcome) -> ProtocolResponse {
    let exit_code = outcome.exit_code();
    match outcome {
        DriverOutcome::Frontend(error) => ProtocolResponse::Result {
            request_id,
            operation: "run".to_owned(),
            exit_code: exit_code.as_process_code(),
            exit_name: exit_name(exit_code).to_owned(),
            diagnostics: error
                .diagnostics()
                .iter()
                .map(protocol_diagnostic)
                .collect(),
            report: None,
            events: Vec::new(),
            metrics: None,
            value: None,
            artifact: None,
        },
        DriverOutcome::Rejected(error) => rejected_response(request_id, exit_code, &error),
        DriverOutcome::Executed(execution) => executed_response(request_id, exit_code, &execution),
    }
}

/// 把协议诊断配置转换成 Runtime 会话配置。
fn diagnostic_options(config: Option<DiagnosticConfig>) -> DiagnosticOptions {
    let Some(config) = config else {
        return DiagnosticOptions::default();
    };
    DiagnosticOptions {
        terminal_level: config.terminal_level,
        file_level: config.file_level,
        log_dir: config.log_dir.map(PathBuf::from),
        log_file: config.log_file.map(PathBuf::from),
        stacktrace: config.stacktrace,
        focus: config
            .focus
            .into_iter()
            .map(|focus| crate::diagnostics::DiagnosticFocus {
                module: focus.module,
                source: focus.source,
                output: PathBuf::from(focus.output),
                level: focus.level,
                mirror: focus.mirror,
            })
            .collect(),
    }
}

/// 将 Runtime 侧诊断启动失败转换为稳定协议响应。
fn diagnostic_start_response(
    request_id: String,
    error: crate::diagnostics::DiagnosticStartError,
) -> ProtocolResponse {
    let details = start_error_details(&error);
    let code = error.code;
    let message = error.message;
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            code,
            "x11.diagnostics.start_failed",
            message,
            Some("diagnostic_startup".to_owned()),
            Some("安装可用终端并重试，或检查 XIAO_DIAGNOSTICS_PATH".to_owned()),
            details,
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

/// 在用户代码开始前建立诊断会话，执行后投递所有 VM 事件。
fn run_with_diagnostics(
    request_id: String,
    debug: bool,
    module: String,
    source: Option<String>,
    config: Option<DiagnosticConfig>,
    request: &DriverRequest,
) -> ProtocolResponse {
    let mut session = if debug {
        match DiagnosticSession::start(module.clone(), source.clone(), &diagnostic_options(config))
        {
            Ok(session) => Some(session),
            Err(error) => return diagnostic_start_response(request_id, error),
        }
    } else {
        None
    };
    let outcome = FrontendVmDriver::new().run(request);
    if let Some(mut session) = session.take() {
        if let DriverOutcome::Executed(execution) = &outcome {
            for event in execution.events() {
                session.record(event);
            }
        }
        session.finish();
    }
    run_response(request_id, outcome)
}

/// 将执行前拒绝转换为稳定协议错误。
fn rejected_response(
    request_id: String,
    exit_code: ExitCode,
    error: &DriverError,
) -> ProtocolResponse {
    let mut details = BTreeMap::new();
    details.insert("code".to_owned(), Value::String(error.code().to_owned()));
    if let Some(path) = error.path() {
        details.insert("path".to_owned(), Value::String(path.to_owned()));
    }
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            error.code(),
            "x11.driver.rejected",
            error.message(),
            Some(driver_phase_name(error.phase()).to_owned()),
            Some("检查源码、产物或取消状态后重试".to_owned()),
            details,
        ),
        report: error.report().map(protocol_report),
        exit_code: exit_code.as_process_code(),
    }
}

/// 返回驱动器阶段的稳定名称。
fn driver_phase_name(phase: DriverPhase) -> &'static str {
    match phase {
        DriverPhase::Control => "control",
        DriverPhase::Verification => "verification",
        DriverPhase::Request => "request",
    }
}

/// 将已进入 VM 的执行结果转换为结构化响应。
fn executed_response(
    request_id: String,
    exit_code: ExitCode,
    execution: &DriverExecution,
) -> ProtocolResponse {
    let outcome = &execution.outcome;
    let value = outcome.value.as_ref().map(protocol_value);
    ProtocolResponse::Result {
        request_id,
        operation: "run".to_owned(),
        exit_code: exit_code.as_process_code(),
        exit_name: exit_name(exit_code).to_owned(),
        diagnostics: execution
            .diagnostics()
            .iter()
            .map(protocol_diagnostic)
            .collect(),
        report: outcome.report.as_ref().map(protocol_report),
        events: outcome.events.iter().map(protocol_event).collect(),
        metrics: Some(protocol_metrics(outcome.metrics, outcome.dropped_events)),
        value,
        artifact: None,
    }
}

/// 将调用方注入的工具链字段转换为 LLVM 驱动器对象。
fn build_toolchain(spec: &ToolchainSpec) -> Result<Toolchain, ProtocolError> {
    if spec.clang.trim().is_empty() {
        return Err(ProtocolError::request(
            "toolchain.clang",
            "原生构建必须显式提供 clang 路径",
        ));
    }
    let versions = ToolchainVersions {
        clang: spec.versions.clang.clone(),
        llvm_as: spec.versions.llvm_as.clone(),
        llc: spec.versions.llc.clone(),
        rustc: spec.versions.rustc.clone(),
    };
    let mut toolchain = Toolchain::new(PathBuf::from(&spec.clang)).with_versions(versions);
    if let Some(path) = &spec.llvm_as {
        toolchain = toolchain.with_llvm_as(path);
    }
    if let Some(path) = &spec.llc {
        toolchain = toolchain.with_llc(path);
    }
    if let Some(path) = &spec.runtime_library {
        toolchain = toolchain.with_runtime_library(path);
    }
    Ok(toolchain.with_native_static_libraries(spec.native_static_libraries.clone()))
}

#[allow(clippy::too_many_arguments)]
/// 调用原生驱动器并转换构建结果或结构化后端错误。
fn build_response(
    request_id: String,
    language_version: String,
    target: ProtocolTarget,
    optimization: OptimizationConfig,
    source: SourceIdentity,
    output: String,
    llvm_ir_output: Option<String>,
    toolchain: ToolchainSpec,
    cancellation: &CancellationToken,
) -> ProtocolResponse {
    if cancellation.is_cancelled() {
        return cancelled_error_response(request_id);
    }
    if optimization.level != 0 {
        return ProtocolResponse::Error {
            request_id: Some(request_id),
            error: protocol_error_body(
                UNSUPPORTED_OPERATION_CODE,
                "x11.protocol.optimization_unavailable",
                "X0-A 只接受优化级别 0",
                Some("build".to_owned()),
                Some("使用 level=0，优化接线留给后续阶段".to_owned()),
                BTreeMap::from([("level".to_owned(), json!(optimization.level))]),
            ),
            report: None,
            exit_code: ExitCode::ArtifactRejected.as_process_code(),
        };
    }
    if output.trim().is_empty() {
        return protocol_error_response(
            Some(request_id),
            &ProtocolError::request("output", "原生构建必须提供输出路径"),
        );
    }
    // 先移除旧激活位，避免失败的普通/调试重建继续误启用上一次的诊断配置。
    let activation_file = activation_path(&output);
    if let Err(error) = fs::remove_file(&activation_file)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return ProtocolResponse::Error {
            request_id: Some(request_id),
            error: protocol_error_body(
                DIAGNOSTIC_START_CODE,
                "x11.diagnostics.activation_cleanup_failed",
                format!("无法清理旧的调试产物激活位：{error}"),
                Some("build".to_owned()),
                Some("检查产物目录权限后重试".to_owned()),
                BTreeMap::from([(
                    "path".to_owned(),
                    json!(activation_file.display().to_string()),
                )]),
            ),
            report: None,
            exit_code: ExitCode::ArtifactRejected.as_process_code(),
        };
    }
    let target_description = match target.to_target() {
        Ok(target) => target,
        Err(error) => return protocol_error_response(Some(request_id), &error),
    };
    let toolchain = match build_toolchain(&toolchain) {
        Ok(toolchain) => toolchain,
        Err(error) => return protocol_error_response(Some(request_id), &error),
    };
    let frontend = frontend_request(&source, &language_version, &target);
    let mut request = NativeBuildRequest::new(
        frontend,
        target_description.clone(),
        toolchain,
        output.clone(),
    )
    .with_codegen_options(CodegenOptions::for_target(target_description));
    if let Some(path) = &llvm_ir_output {
        request = request.with_llvm_ir_output(path);
    }
    if cancellation.is_cancelled() {
        return cancelled_error_response(request_id);
    }
    match FrontendNativeDriver::new().build(&request) {
        Ok(result) => {
            if cancellation.is_cancelled() {
                return cancelled_error_response(request_id);
            }
            let diagnostic_activation = if optimization.debug {
                let activation = DiagnosticActivation {
                    format_version: 1,
                    enabled: true,
                    source_map: Some(
                        source
                            .path
                            .clone()
                            .unwrap_or_else(|| "<embedded>".to_owned()),
                    ),
                    metadata_version: 1,
                    hooks: true,
                };
                match write_activation(&result.native.executable, &activation) {
                    Ok(path) => Some(ProtocolDiagnosticActivation {
                        path: path.display().to_string(),
                        enabled: activation.enabled,
                        source_map: activation.source_map.is_some(),
                        hooks: activation.hooks,
                    }),
                    Err(error) => {
                        return ProtocolResponse::Error {
                            request_id: Some(request_id),
                            error: protocol_error_body(
                                DIAGNOSTIC_START_CODE,
                                "x11.diagnostics.activation_write_failed",
                                format!("无法写入调试产物激活位：{error}"),
                                Some("build".to_owned()),
                                Some("检查产物目录权限后重试".to_owned()),
                                BTreeMap::new(),
                            ),
                            report: None,
                            exit_code: ExitCode::ArtifactRejected.as_process_code(),
                        };
                    }
                }
            } else {
                None
            };
            ProtocolResponse::Result {
                request_id,
                operation: "build".to_owned(),
                exit_code: ExitCode::Success.as_process_code(),
                exit_name: exit_name(ExitCode::Success).to_owned(),
                diagnostics: result
                    .frontend
                    .diagnostics()
                    .iter()
                    .map(protocol_diagnostic)
                    .collect(),
                report: None,
                events: Vec::new(),
                metrics: None,
                value: None,
                artifact: Some(ProtocolArtifact {
                    executable: result.native.executable.display().to_string(),
                    llvm_ir_output,
                    toolchain_fingerprint: result.native.toolchain_fingerprint.to_string(),
                    uses_runtime: result.native.module.uses_runtime,
                    runtime_components: result.native.module.runtime_components,
                    diagnostic_activation,
                }),
            }
        }
        Err(NativeDriverError::Frontend(error)) => ProtocolResponse::Result {
            request_id,
            operation: "build".to_owned(),
            exit_code: ExitCode::SourceRejected.as_process_code(),
            exit_name: exit_name(ExitCode::SourceRejected).to_owned(),
            diagnostics: error
                .diagnostics()
                .iter()
                .map(protocol_diagnostic)
                .collect(),
            report: None,
            events: Vec::new(),
            metrics: None,
            value: None,
            artifact: None,
        },
        Err(NativeDriverError::Backend(error)) => ProtocolResponse::Error {
            request_id: Some(request_id),
            error: protocol_error_body(
                BUILD_ERROR_CODE,
                "x11.driver.native_build",
                error.to_string(),
                Some("build".to_owned()),
                Some("检查目标描述、输出路径和外部 LLVM 工具链".to_owned()),
                BTreeMap::new(),
            ),
            report: None,
            exit_code: ExitCode::ArtifactRejected.as_process_code(),
        },
    }
}

/// 创建统一取消响应，使用 `ArtifactRejected` 进程码。
fn cancelled_error_response(request_id: String) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            CANCELLED_ERROR_CODE,
            "x11.protocol.cancelled",
            "请求已取消",
            Some("control".to_owned()),
            Some("重新提交请求或继续等待其他请求".to_owned()),
            BTreeMap::new(),
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

/// 将内部协议验证错误包装为响应。
fn protocol_error_response(request_id: Option<String>, error: &ProtocolError) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id,
        error: protocol_error_from_error(error),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

/// 直接处理一条请求；适合契约测试和不需要并发取消的调用方。
pub fn dispatch(request: ProtocolRequest) -> ProtocolResponse {
    match request {
        ProtocolRequest::Hello {
            request_id,
            protocol_version,
            core_version,
        } => {
            let versions = CoreVersions::current();
            match validate_versions(protocol_version, core_version) {
                Ok(()) => ProtocolResponse::Hello {
                    request_id,
                    accepted: true,
                    protocol_version: PROTOCOL_VERSION,
                    core_version: CORE_VERSION,
                    versions,
                    capabilities: vec!["run".to_owned(), "build".to_owned(), "cancel".to_owned()],
                    error: None,
                },
                Err(error) => ProtocolResponse::Hello {
                    request_id,
                    accepted: false,
                    protocol_version: PROTOCOL_VERSION,
                    core_version: CORE_VERSION,
                    versions,
                    capabilities: Vec::new(),
                    error: Some(protocol_error_from_error(&error)),
                },
            }
        }
        ProtocolRequest::Run {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            options,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            if let Err(error) = validate_source(&source).and_then(|_| validate_target(&target)) {
                return protocol_error_response(Some(request_id), &error);
            }
            if optimization.level != 0 {
                return protocol_error_response(
                    Some(request_id),
                    &ProtocolError::request("optimization.level", "X0-A 只接受优化级别 0"),
                );
            }
            let (vm_options, event_capacity, timeout) = match run_options(&options) {
                Ok(value) => value,
                Err(error) => return protocol_error_response(Some(request_id), &error),
            };
            let debug = optimization.debug;
            let diagnostic_config = optimization.diagnostics.clone();
            let module_name = source.module.clone();
            let source_name = source.path.clone();
            let token = CancellationToken::new();
            let mut driver_request =
                DriverRequest::new(frontend_request(&source, &language_version, &target))
                    .with_options(vm_options)
                    .with_module_name(module_name.clone())
                    .with_event_capacity(event_capacity)
                    .with_cancellation(token);
            if let Some(path) = source_name.clone() {
                driver_request = driver_request.with_source_name(path);
            }
            if let Some(timeout) = timeout {
                driver_request = driver_request.with_timeout(timeout);
            }
            run_with_diagnostics(
                request_id,
                debug,
                module_name,
                source_name,
                diagnostic_config,
                &driver_request,
            )
        }
        ProtocolRequest::Build {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            output,
            llvm_ir_output,
            toolchain,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            if let Err(error) = validate_source(&source).and_then(|_| validate_target(&target)) {
                return protocol_error_response(Some(request_id), &error);
            }
            build_response(
                request_id,
                language_version,
                target,
                optimization,
                source,
                output,
                llvm_ir_output,
                toolchain,
                &CancellationToken::new(),
            )
        }
        ProtocolRequest::Cancel {
            request_id,
            protocol_version,
            core_version,
            target_request_id,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            ProtocolResponse::Cancelled {
                request_id,
                target_request_id,
                accepted: false,
                exit_code: ExitCode::ArtifactRejected.as_process_code(),
            }
        }
        ProtocolRequest::Shutdown {
            request_id,
            protocol_version,
            core_version,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            ProtocolResponse::Shutdown { request_id }
        }
    }
}

/// 服务线程共享的串行输出锁。
type SharedWriter<W> = Arc<Mutex<W>>;
/// 请求 ID 到取消令牌的登记表。
type CancellationMap = Arc<Mutex<BTreeMap<String, CancellationToken>>>;

/// 在线程安全的输出锁上写入一条响应。
fn write_response<W: Write>(writer: &SharedWriter<W>, response: &ProtocolResponse) {
    if let Ok(mut writer) = writer.lock() {
        let _ = write_frame(&mut *writer, response);
    }
}

/// 在线程中执行运行/构建请求，并把 panic 转为稳定响应。
fn worker_response(request: ProtocolRequest, token: CancellationToken) -> ProtocolResponse {
    match request {
        ProtocolRequest::Run {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            options,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            if let Err(error) = validate_source(&source).and_then(|_| validate_target(&target)) {
                return protocol_error_response(Some(request_id), &error);
            }
            if optimization.level != 0 {
                return protocol_error_response(
                    Some(request_id),
                    &ProtocolError::request("optimization.level", "X0-A 只接受优化级别 0"),
                );
            }
            let (vm_options, event_capacity, timeout) = match run_options(&options) {
                Ok(value) => value,
                Err(error) => return protocol_error_response(Some(request_id), &error),
            };
            let debug = optimization.debug;
            let diagnostic_config = optimization.diagnostics.clone();
            let module_name = source.module.clone();
            let source_name = source.path.clone();
            let mut driver_request =
                DriverRequest::new(frontend_request(&source, &language_version, &target))
                    .with_options(vm_options)
                    .with_module_name(module_name.clone())
                    .with_event_capacity(event_capacity)
                    .with_cancellation(token);
            if let Some(path) = source_name.clone() {
                driver_request = driver_request.with_source_name(path);
            }
            if let Some(timeout) = timeout {
                driver_request = driver_request.with_timeout(timeout);
            }
            run_with_diagnostics(
                request_id,
                debug,
                module_name,
                source_name,
                diagnostic_config,
                &driver_request,
            )
        }
        ProtocolRequest::Build {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            output,
            llvm_ir_output,
            toolchain,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            if let Err(error) = validate_source(&source).and_then(|_| validate_target(&target)) {
                return protocol_error_response(Some(request_id), &error);
            }
            build_response(
                request_id,
                language_version,
                target,
                optimization,
                source,
                output,
                llvm_ir_output,
                toolchain,
                &token,
            )
        }
        other => dispatch(other),
    }
}

/// 在拥有的输入/输出流上运行可取消的协议服务。
pub fn serve<R, W>(reader: R, writer: W) -> Result<(), FrameError>
where
    R: Read,
    W: Write + Send + 'static,
{
    let mut reader = BufReader::new(reader);
    let writer = Arc::new(Mutex::new(BufWriter::new(writer)));
    let cancellations: CancellationMap = Arc::new(Mutex::new(BTreeMap::new()));
    let mut workers: Vec<JoinHandle<()>> = Vec::new();
    let mut first_frame = true;
    let mut negotiated = false;
    while let Some(request) = read_request(&mut reader)? {
        if first_frame {
            first_frame = false;
            if !matches!(request, ProtocolRequest::Hello { .. }) {
                let request_id = match &request {
                    ProtocolRequest::Run { request_id, .. }
                    | ProtocolRequest::Build { request_id, .. }
                    | ProtocolRequest::Cancel { request_id, .. }
                    | ProtocolRequest::Shutdown { request_id, .. } => Some(request_id.clone()),
                    ProtocolRequest::Hello { .. } => None,
                };
                let error = ProtocolError::version("首帧必须是 hello 版本协商");
                write_response(&writer, &protocol_error_response(request_id, &error));
                break;
            }
        }
        match request {
            ProtocolRequest::Hello { .. } => {
                let response = dispatch(request);
                negotiated = matches!(&response, ProtocolResponse::Hello { accepted: true, .. });
                write_response(&writer, &response);
            }
            ProtocolRequest::Cancel {
                request_id,
                protocol_version,
                core_version,
                target_request_id,
            } => {
                if !negotiated {
                    let error = ProtocolError::version("必须先完成 hello 版本协商");
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                    continue;
                }
                if let Err(error) = validate_versions(protocol_version, core_version) {
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                    continue;
                }
                let accepted = cancellations
                    .lock()
                    .ok()
                    .and_then(|map| map.get(&target_request_id).cloned())
                    .map(|token| {
                        token.cancel();
                        true
                    })
                    .unwrap_or(false);
                write_response(
                    &writer,
                    &ProtocolResponse::Cancelled {
                        request_id,
                        target_request_id,
                        accepted,
                        exit_code: ExitCode::ArtifactRejected.as_process_code(),
                    },
                );
            }
            ProtocolRequest::Shutdown {
                request_id,
                protocol_version,
                core_version,
            } => {
                if !negotiated {
                    let error = ProtocolError::version("必须先完成 hello 版本协商");
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                    continue;
                }
                if let Err(error) = validate_versions(protocol_version, core_version) {
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                } else {
                    write_response(&writer, &ProtocolResponse::Shutdown { request_id });
                    break;
                }
            }
            request @ (ProtocolRequest::Run { .. } | ProtocolRequest::Build { .. }) => {
                if !negotiated {
                    let request_id = match &request {
                        ProtocolRequest::Run { request_id, .. }
                        | ProtocolRequest::Build { request_id, .. } => request_id.clone(),
                        _ => unreachable!(),
                    };
                    let error = ProtocolError::version("必须先完成 hello 版本协商");
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                    continue;
                }
                let request_id = match &request {
                    ProtocolRequest::Run { request_id, .. }
                    | ProtocolRequest::Build { request_id, .. } => request_id.clone(),
                    _ => unreachable!(),
                };
                let token = CancellationToken::new();
                if let Ok(mut map) = cancellations.lock() {
                    map.insert(request_id.clone(), token.clone());
                }
                let writer_clone = Arc::clone(&writer);
                let cancellations_clone = Arc::clone(&cancellations);
                workers.push(thread::spawn(move || {
                    let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        worker_response(request, token)
                    }))
                    .unwrap_or_else(|_| ProtocolResponse::Error {
                        request_id: Some(request_id.clone()),
                        error: protocol_error_body(
                            CORE_CRASH_CODE,
                            "x11.protocol.core_crash",
                            "Rust 核心处理请求时发生内部崩溃",
                            Some("core".to_owned()),
                            Some("收集核心日志并重启核心进程".to_owned()),
                            BTreeMap::from([
                                ("status".to_owned(), json!("panic")),
                                (
                                    "process_exit_code".to_owned(),
                                    json!(ExitCode::Fatal.as_process_code()),
                                ),
                            ]),
                        ),
                        report: None,
                        exit_code: ExitCode::Fatal.as_process_code(),
                    });
                    if let Ok(mut map) = cancellations_clone.lock() {
                        map.remove(&request_id);
                    }
                    write_response(&writer_clone, &response);
                }));
            }
        }
    }
    for worker in workers {
        let _ = worker.join();
    }
    Ok(())
}

/// 使用标准输入/输出启动 Rust 核心协议服务。
pub fn serve_stdio() -> Result<(), FrameError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve(stdin, stdout)
}

/// 将任意 panic 统一包装为核心崩溃响应。
#[must_use]
pub fn core_crash_response(request_id: Option<String>) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id,
        error: protocol_error_body(
            CORE_CRASH_CODE,
            "x11.protocol.core_crash",
            "Rust 核心处理请求时发生内部崩溃",
            Some("core".to_owned()),
            Some("收集核心日志并重启核心进程".to_owned()),
            BTreeMap::from([
                ("status".to_owned(), json!("panic")),
                (
                    "process_exit_code".to_owned(),
                    json!(ExitCode::Fatal.as_process_code()),
                ),
            ]),
        ),
        report: None,
        exit_code: ExitCode::Fatal.as_process_code(),
    }
}

#[cfg(test)]
/// 覆盖帧边界、版本协商、取消和真实前端运行路径。
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::Arc;

    /// 测试用的线程安全输出缓冲区。
    #[derive(Clone, Default)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuffer {
        /// 追加响应帧字节。
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().expect("buffer lock").extend_from_slice(bytes);
            Ok(bytes.len())
        }

        /// 测试缓冲区无需额外刷新动作。
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// 构造一条正确版本的 hello 请求。
    fn hello() -> ProtocolRequest {
        ProtocolRequest::Hello {
            request_id: "hello-1".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
        }
    }

    #[test]
    /// 长度字段固定为 8 字节大端且只计算 JSON 负载。
    fn frame_uses_eight_byte_big_endian_payload_length() {
        let frame = encode_frame(&hello()).expect("frame");
        assert_eq!(
            &frame[..FRAME_LENGTH_BYTES],
            &[0, 0, 0, 0, 0, 0, 0, frame.len() as u8 - 8]
        );
        let decoded: ProtocolRequest = decode_frame(&frame[FRAME_LENGTH_BYTES..]).expect("decode");
        assert_eq!(decoded, hello());
    }

    #[test]
    /// 干净 EOF 可结束服务，部分长度必须拒绝。
    fn read_frame_accepts_clean_eof_and_rejects_truncation() {
        assert!(
            read_frame(&mut Cursor::new(Vec::<u8>::new()))
                .expect("eof")
                .is_none()
        );
        let error = read_frame(&mut Cursor::new(vec![1, 2])).expect_err("truncated");
        assert!(matches!(error, FrameError::TruncatedLength { read: 2 }));
    }

    #[test]
    /// 超长帧在分配前被拒绝。
    fn read_frame_rejects_oversized_payload_before_allocating() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&((MAX_FRAME_BYTES as u64) + 1).to_be_bytes());
        let error = read_frame(&mut Cursor::new(bytes)).expect_err("oversized");
        assert!(matches!(error, FrameError::LengthTooLarge { .. }));
    }

    #[test]
    /// 版本失配返回稳定机器错误码。
    fn version_mismatch_is_machine_readable() {
        let response = dispatch(ProtocolRequest::Hello {
            request_id: "bad".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION + 1,
        });
        let ProtocolResponse::Hello {
            accepted, error, ..
        } = response
        else {
            panic!("expected hello response");
        };
        assert!(!accepted);
        assert_eq!(error.expect("error").code, VERSION_MISMATCH_CODE);
    }

    #[test]
    /// 首帧不是 hello 时会拒绝会话，不让请求绕过版本协商。
    fn service_requires_hello_as_first_frame() {
        let request = ProtocolRequest::Run {
            request_id: "run-before-hello".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            language_version: "0.1.0".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            target: ProtocolTarget::host(),
            optimization: OptimizationConfig::default(),
            source: SourceIdentity {
                module: "main".to_owned(),
                path: None,
                text: "value = 1\n".to_owned(),
            },
            options: RunOptions::default(),
        };
        let mut input = encode_frame(&request).expect("run frame");
        input.extend_from_slice(&encode_frame(&hello()).expect("hello frame"));
        let output = SharedBuffer::default();
        let output_view = Arc::clone(&output.0);
        serve(Cursor::new(input), output).expect("service");

        let mut cursor = Cursor::new(output_view.lock().expect("buffer lock").clone());
        let payload = read_frame(&mut cursor)
            .expect("error frame")
            .expect("one response");
        let ProtocolResponse::Error { error, .. } = decode_frame(&payload).expect("response")
        else {
            panic!("首帧违规应返回 error");
        };
        assert_eq!(error.code, VERSION_MISMATCH_CODE);
        assert!(read_frame(&mut cursor).expect("end of session").is_none());
    }

    #[test]
    /// 帧损坏和语义请求错误使用不同的稳定机器码。
    fn frame_and_request_errors_keep_distinct_codes() {
        let frame_error = decode_frame::<ProtocolRequest>(b"not-json").expect_err("bad json");
        assert_eq!(frame_error.code(), FRAME_ERROR_CODE);

        let response = dispatch(ProtocolRequest::Run {
            request_id: "bad-source".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            language_version: "0.1.0".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            target: ProtocolTarget::host(),
            optimization: OptimizationConfig::default(),
            source: SourceIdentity {
                module: "  ".to_owned(),
                path: None,
                text: "value = 1\n".to_owned(),
            },
            options: RunOptions::default(),
        });
        let ProtocolResponse::Error { error, .. } = response else {
            panic!("无效源码身份应返回 error");
        };
        assert_eq!(error.code, REQUEST_ERROR_CODE);
    }

    #[test]
    /// 取消令牌映射到冻结的产物拒绝进程码。
    fn real_source_run_maps_cancel_to_artifact_rejected() {
        let token = CancellationToken::new();
        token.cancel();
        let request = ProtocolRequest::Run {
            request_id: "run-1".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            language_version: "0.1.0".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            target: ProtocolTarget::host(),
            optimization: OptimizationConfig::default(),
            source: SourceIdentity {
                module: "main".to_owned(),
                path: None,
                text: "value = 1\n".to_owned(),
            },
            options: RunOptions::default(),
        };
        let response = worker_response(request, token);
        let (ProtocolResponse::Result { exit_code, .. }
        | ProtocolResponse::Error { exit_code, .. }) = response
        else {
            panic!("expected run response");
        };
        assert_eq!(exit_code, ExitCode::ArtifactRejected.as_process_code());
    }

    #[test]
    /// 真实 Xiao 源码经前端和 VM 后返回结构化结果。
    fn real_source_run_returns_structured_result_without_text_parsing() {
        let response = dispatch(ProtocolRequest::Run {
            request_id: "run-success".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            language_version: "0.1.0".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            target: ProtocolTarget::host(),
            optimization: OptimizationConfig::default(),
            source: SourceIdentity {
                module: "main".to_owned(),
                path: Some("main.xiao".to_owned()),
                text: "value = 1 + 2\n".to_owned(),
            },
            options: RunOptions::default(),
        });
        let ProtocolResponse::Result {
            exit_code,
            exit_name,
            metrics,
            ..
        } = response
        else {
            panic!("真实源码应产生结构化结果");
        };
        assert_eq!(exit_code, ExitCode::Success.as_process_code());
        assert_eq!(exit_name, "success");
        assert!(metrics.is_some());
    }

    #[test]
    /// 服务入口先协商版本，再处理真实源码并确认关闭。
    fn service_round_trips_hello_run_and_shutdown_frames() {
        let run = ProtocolRequest::Run {
            request_id: "service-run".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
            language_version: "0.1.0".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            target: ProtocolTarget::host(),
            optimization: OptimizationConfig::default(),
            source: SourceIdentity {
                module: "main".to_owned(),
                path: None,
                text: "value = 1\n".to_owned(),
            },
            options: RunOptions::default(),
        };
        let shutdown = ProtocolRequest::Shutdown {
            request_id: "service-shutdown".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            core_version: CORE_VERSION,
        };
        let mut input = encode_frame(&hello()).expect("hello frame");
        input.extend_from_slice(&encode_frame(&run).expect("run frame"));
        input.extend_from_slice(&encode_frame(&shutdown).expect("shutdown frame"));
        let output = SharedBuffer::default();
        let output_view = Arc::clone(&output.0);
        serve(Cursor::new(input), output).expect("service");

        let mut cursor = Cursor::new(output_view.lock().expect("buffer lock").clone());
        let mut responses = Vec::new();
        while let Some(payload) = read_frame(&mut cursor).expect("response frame") {
            responses.push(decode_frame::<ProtocolResponse>(&payload).expect("response"));
        }
        assert!(
            responses
                .iter()
                .any(|response| matches!(response, ProtocolResponse::Hello { accepted: true, .. }))
        );
        assert!(responses.iter().any(|response| matches!(response, ProtocolResponse::Result { request_id, exit_code: 0, .. } if request_id == "service-run")));
        assert!(responses.iter().any(|response| matches!(response, ProtocolResponse::Shutdown { request_id } if request_id == "service-shutdown")));
    }
}
