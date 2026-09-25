//! 协议版本、响应消息和跨进程值类型。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use xiao_bytecode::FORMAT_VERSION;
use xiao_codegen_llvm::CODEGEN_VERSION;
use xiao_package::{EnvironmentMetadata, PackageOperationResult};
use xiao_runtime_abi::ABI_ENCODED_VERSION;

use crate::run::DRIVER_VERSION;

/// 当前协议版本。破坏性字段或帧布局变化时递增；新增兼容操作由 hello 能力协商。
pub const PROTOCOL_VERSION: u16 = 1;
/// CLI 与核心比较的统一兼容版本；组件版本只作为诊断信息返回。
pub const CORE_VERSION: u32 = 1;

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

/// 一个项目测试用例的结构化执行结果。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolTestCaseResult {
    /// 项目相对路径或协议提供的稳定模块名。
    pub path: String,
    /// 测试用例的逻辑模块名。
    pub module: String,
    /// B0-D 冻结的单用例退出码。
    pub exit_code: u8,
    /// 单用例退出语义名称。
    pub exit_name: String,
    /// 前端警告/错误诊断。
    pub diagnostics: Vec<ProtocolDiagnostic>,
    /// VM 结构化报告。
    pub report: Option<ProtocolReport>,
    /// VM 事件。
    pub events: Vec<ProtocolEvent>,
    /// VM 指标。
    pub metrics: Option<ProtocolMetrics>,
    /// 可选入口值摘要；本批不以它作为判据。
    pub value: Option<ProtocolValue>,
    /// 单用例在协议或驱动边界被拒绝时的错误体。
    pub error: Option<ProtocolErrorBody>,
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
    /// 环境指纹元数据生成成功。
    EnvironmentResult {
        /// 对应请求编号。
        request_id: String,
        /// 规范化配置、工具链和目标指纹。
        metadata: EnvironmentMetadata,
    },
    /// 同步或安装完成。
    PackageResult {
        /// 对应请求编号。
        request_id: String,
        /// 环境与锁文件操作摘要。
        result: PackageOperationResult,
    },
    /// 项目测试完成结果；`tests` 顺序与请求中的源码顺序一致。
    TestResult {
        /// 对应请求编号。
        request_id: String,
        /// 固定为 `test`。
        operation: String,
        /// 首个非零单用例退出码；所有用例成功时为 `0`。
        exit_code: u8,
        /// 整体退出语义名称。
        exit_name: String,
        /// 发现并执行的用例总数。
        total: usize,
        /// 退出码为 `0` 的用例数。
        passed: usize,
        /// 退出码非 `0` 的用例数。
        failed: usize,
        /// 各用例的结构化结果。
        tests: Vec<ProtocolTestCaseResult>,
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
    /// 随调试产物复制的独立诊断组件路径。
    #[serde(default)]
    pub diagnostics_component: Option<String>,
    /// 构建时固化的运行时配置旁置文件。
    #[serde(default)]
    pub runtime_config: Option<ProtocolRuntimeConfig>,
}

/// 固化运行时配置的旁置文件摘要。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolRuntimeConfig {
    /// 配置摘要文件路径。
    pub path: String,
    /// 固化格式版本。
    pub format_version: u16,
    /// 是否允许普通命令行覆盖。
    pub cli_overrides: bool,
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
