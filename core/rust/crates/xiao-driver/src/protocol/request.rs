//! 请求配置、请求消息和协议层内部错误。

use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use xiao_codegen_llvm::{Endian, ObjectFormat, TargetDescription};
use xiao_vm::VmOptions;

/// 请求字段或 JSON 形状非法。
pub const REQUEST_ERROR_CODE: &str = "X11-PROTOCOL-002";
/// 协议版本或统一核心版本不兼容。
pub const VERSION_MISMATCH_CODE: &str = "X11-PROTOCOL-004";
/// 原生构建驱动器错误的协议包装编号。
pub const BUILD_ERROR_CODE: &str = "X11-PROTOCOL-007";
/// 核心处理请求时发生 panic。
pub const CORE_CRASH_CODE: &str = "X11-PROTOCOL-003";
/// 协议请求主动取消。
pub const CANCELLED_ERROR_CODE: &str = "X11-PROTOCOL-005";
/// X0-A 尚未支持的协议操作或配置。
pub const UNSUPPORTED_OPERATION_CODE: &str = "X11-PROTOCOL-006";

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
    /// 是否在 VM 热循环中启用取消检查点。
    #[serde(default = "default_checkpoints_enabled")]
    pub checkpoints_enabled: bool,
    /// 两次取消检查点之间执行的指令数。
    #[serde(default = "default_checkpoint_interval")]
    pub checkpoint_interval: usize,
}

/// 返回请求反序列化时检查点开关的默认值。
fn default_checkpoints_enabled() -> bool {
    true
}

/// 返回请求反序列化时采用的默认检查点间隔。
fn default_checkpoint_interval() -> usize {
    xiao_vm::DEFAULT_CHECKPOINT_INTERVAL
}

impl Default for RunOptions {
    /// 返回与生产 VM 一致的默认调用深度和事件容量。
    fn default() -> Self {
        Self {
            max_call_depth: VmOptions::default().max_call_depth,
            event_capacity: xiao_vm::DEFAULT_EVENT_CAPACITY,
            timeout_ms: None,
            checkpoints_enabled: true,
            checkpoint_interval: xiao_vm::DEFAULT_CHECKPOINT_INTERVAL,
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
    /// Rust 编译器路径；动态 Runtime 构建时用于查询 native-static-libs。
    #[serde(default)]
    pub rustc: Option<String>,
    /// 调试原生产物启动 shim 使用的诊断进程路径。
    #[serde(default)]
    pub diagnostics_path: Option<String>,
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
    #[serde(default)]
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
    /// 使用真实源码顺序执行项目测试用例。
    Test {
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
        /// 已按项目相对路径排序的测试源码。
        cases: Vec<SourceIdentity>,
        /// 每个测试用例复用的 VM 参数。
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
        /// 可选的原始 config.xiao；由 Rust 配置解析器验证并固化。
        #[serde(default)]
        config_text: Option<String>,
    },
    /// 基于已规范化配置和工具链描述生成环境指纹元数据。
    Environment {
        /// 请求编号。
        request_id: String,
        /// 协议版本。
        protocol_version: u16,
        /// 统一核心版本。
        core_version: u32,
        /// 项目根目录；只作为环境布局来源，不进入指纹。
        project_root: String,
        /// 可选逻辑环境名称；省略时使用 `venv`。
        logical_name: Option<String>,
        /// 可选项目配置源码；仅解析为静态配置树。
        config_text: Option<String>,
        /// 目标平台描述。
        target: ProtocolTarget,
        /// 工具链路径与版本描述。
        toolchain: ToolchainSpec,
    },
    /// 使用 D1 本地包图同步或安装；目标环境由 Rust 唯一选择。
    Package {
        /// 请求编号。
        request_id: String,
        /// 协议版本。
        protocol_version: u16,
        /// 核心版本。
        core_version: u32,
        /// sync 或 install。
        operation: String,
        /// 项目根绝对路径。
        project_root: String,
        /// 已激活环境的绝对路径。
        active_environment: Option<String>,
        /// 静态配置原文。
        config_text: String,
        /// 是否保留多余映射。
        keep_extra: bool,
        /// 锁文件必须为最新。
        locked: bool,
        /// 只读现有锁文件。
        frozen: bool,
        /// 平台目标。
        target: ProtocolTarget,
        /// 已发现的工具链。
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

/// 协议层验证错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolError {
    code: &'static str,
    field: Option<String>,
    message: String,
}

impl ProtocolError {
    /// 创建请求字段错误。
    pub(super) fn request(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: REQUEST_ERROR_CODE,
            field: Some(field.into()),
            message: message.into(),
        }
    }

    /// 创建版本不兼容错误。
    pub(super) fn version(message: impl Into<String>) -> Self {
        Self {
            code: VERSION_MISMATCH_CODE,
            field: None,
            message: message.into(),
        }
    }

    /// 创建构建阶段错误。
    pub(super) fn build(message: impl Into<String>) -> Self {
        Self {
            code: BUILD_ERROR_CODE,
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
