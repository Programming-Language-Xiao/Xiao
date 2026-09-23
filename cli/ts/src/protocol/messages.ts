/** X0-A 进程协议的 TypeScript 侧消息类型；只镜像公共字段，不镜像 Rust 内部布局。 */

/** 当前协议版本。 */
export const PROTOCOL_VERSION = 1;

/** 当前统一核心兼容版本。 */
export const CORE_VERSION = 1;

/** 版本协商消息。 */
export interface HelloRequest {
  type: "hello";
  request_id: string;
  protocol_version: number;
  core_version: number;
}

/** 运行请求的源码身份。 */
export interface SourceIdentity {
  module: string;
  path: string | null;
  text: string;
}

/** 运行/构建的目标条件。 */
export interface ProtocolTarget {
  triple: string;
  pointer_width: number;
  endian: "little" | "big";
  object_format: "coff" | "elf" | "macho";
}

/** 优化和调试配置。 */
export interface OptimizationConfig {
  level: number;
  debug: boolean;
  diagnostics?: DiagnosticConfig | null;
}

/** `-debug` 诊断等级与输出配置。 */
export interface DiagnosticConfig {
  terminal_level?: string | null;
  file_level?: string | null;
  log_dir?: string | null;
  log_file?: string | null;
  stacktrace?: string | null;
  focus?: DiagnosticFocus[];
}

/** 一个模块/源码聚焦输出规则。 */
export interface DiagnosticFocus {
  module?: string | null;
  source?: string | null;
  output: string;
  level?: string | null;
  mirror?: boolean;
}

/** VM 运行参数。 */
export interface RunOptions {
  max_call_depth: number;
  event_capacity: number;
  timeout_ms: number | null;
}

/** 运行请求。 */
export interface RunRequest {
  type: "run";
  request_id: string;
  protocol_version: number;
  core_version: number;
  language_version: string;
  runtime_version: string;
  target: ProtocolTarget;
  optimization: OptimizationConfig;
  source: SourceIdentity;
  options: RunOptions;
}

/** 工具链版本文本。 */
export interface ToolchainVersions {
  clang: string;
  llvm_as: string | null;
  llc: string | null;
  rustc?: string | null;
}

/** 原生构建工具链描述。 */
export interface ToolchainSpec {
  clang: string;
  llvm_as: string | null;
  llc: string | null;
  runtime_library: string | null;
  native_static_libraries: string[];
  /** Rust 编译器路径；动态 Runtime 构建时由核心查询 native-static-libs。 */
  rustc?: string | null;
  /** 调试原生产物启动 shim 使用的诊断进程路径。 */
  diagnostics_path?: string | null;
  versions: ToolchainVersions;
}

/** 原生构建请求。 */
export interface BuildRequest {
  type: "build";
  request_id: string;
  protocol_version: number;
  core_version: number;
  language_version: string;
  runtime_version: string;
  target: ProtocolTarget;
  optimization: OptimizationConfig;
  source: SourceIdentity;
  output: string;
  llvm_ir_output: string | null;
  toolchain: ToolchainSpec;
  /** 可选的原始 config.xiao；由 Rust 配置解析器验证并固化。 */
  config_text?: string | null;
}

/** 取消请求。 */
export interface CancelRequest {
  type: "cancel";
  request_id: string;
  protocol_version: number;
  core_version: number;
  target_request_id: string;
}

/** 关闭请求。 */
export interface ShutdownRequest {
  type: "shutdown";
  request_id: string;
  protocol_version: number;
  core_version: number;
}

/** 所有请求消息的联合类型。 */
export type ProtocolRequest = HelloRequest | RunRequest | BuildRequest | CancelRequest | ShutdownRequest;

/** 机器可读协议错误。 */
export interface ProtocolErrorBody {
  code: string;
  message_id: string;
  message: string;
  phase: string | null;
  next_step: string | null;
  details: Record<string, unknown>;
}

/** 版本协商响应。 */
export interface HelloResponse {
  type: "hello";
  request_id: string;
  accepted: boolean;
  protocol_version: number;
  core_version: number;
  versions: Record<string, unknown>;
  capabilities: string[];
  error: ProtocolErrorBody | null;
}

/** 运行/构建成功或语义失败结果。 */
export interface ResultResponse {
  type: "result";
  request_id: string;
  operation: "run" | "build";
  exit_code: number;
  exit_name: string;
  diagnostics: unknown[];
  report: unknown | null;
  events: unknown[];
  metrics: unknown | null;
  value: unknown | null;
  artifact: unknown | null;
}

/** 原生构建产物的稳定摘要。 */
export interface ProtocolArtifact {
  executable: string;
  llvm_ir_output: string | null;
  toolchain_fingerprint: string;
  uses_runtime: boolean;
  runtime_components: string[];
  diagnostic_activation?: ProtocolDiagnosticActivation | null;
  /** 随调试产物复制的独立诊断组件。 */
  diagnostics_component?: string | null;
  /** 构建时固化的运行时配置旁置文件。 */
  runtime_config?: ProtocolRuntimeConfig | null;
}

/** 固化运行时配置的旁置文件摘要。 */
export interface ProtocolRuntimeConfig {
  /** 配置文件路径。 */
  path: string;
  /** 固化格式版本。 */
  format_version: number;
  /** 是否允许普通命令行覆盖。 */
  cli_overrides: boolean;
}

/** 调试产物持久激活位摘要。 */
export interface ProtocolDiagnosticActivation {
  path: string;
  enabled: boolean;
  source_map: boolean;
  hooks: boolean;
}

/** 协议级失败响应。 */
export interface ErrorResponse {
  type: "error";
  request_id: string | null;
  error: ProtocolErrorBody;
  report: unknown | null;
  exit_code: number;
}

/** 取消确认响应。 */
export interface CancelledResponse {
  type: "cancelled";
  request_id: string;
  target_request_id: string;
  accepted: boolean;
  exit_code: number;
}

/** 关闭确认响应。 */
export interface ShutdownResponse {
  type: "shutdown";
  request_id: string;
}

/** 所有响应消息的联合类型。 */
export type ProtocolResponse = HelloResponse | ResultResponse | ErrorResponse | CancelledResponse | ShutdownResponse;

/** 判断一个值是否具有字符串字段。 */
export function hasStringField(value: unknown, field: string): value is Record<string, unknown> & Record<string, string> {
  return typeof value === "object" && value !== null && typeof (value as Record<string, unknown>)[field] === "string";
}

/** 验证协议消息的最小公共形状。详细语义仍由 Rust 核心负责。 */
export function validateMessage(value: unknown): ProtocolRequest | ProtocolResponse {
  if (typeof value !== "object" || value === null || typeof (value as { type?: unknown }).type !== "string") {
    throw new Error("X11-PROTOCOL-002: 消息缺少 type 字段");
  }
  const message = value as Record<string, unknown>;
  if (typeof message.request_id !== "string" && message.type !== "error") {
    throw new Error("X11-PROTOCOL-002: 消息缺少 request_id 字段");
  }
  return value as ProtocolRequest | ProtocolResponse;
}
