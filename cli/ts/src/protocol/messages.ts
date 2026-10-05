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
  checkpoints_enabled: boolean;
  checkpoint_interval: number;
}

/** 运行请求。 */
export interface RunRequest {
  type: "run";
  request_id: string;
  protocol_version: number;
  core_version: number;
  language_version: string;
  runtime_version: string;
  /** 本次运行的可选语言标签；旧客户端可省略。 */
  locale?: string | null;
  target: ProtocolTarget;
  optimization: OptimizationConfig;
  source: SourceIdentity;
  options: RunOptions;
}

/** 归档运行请求；入口与依赖只由 Rust 核心从唯一索引读取。 */
export interface RunArchiveRequest {
  type: "run_archive";
  request_id: string;
  protocol_version: number;
  core_version: number;
  locale?: string | null;
  path: string;
  options: RunOptions;
  debug: boolean;
}

/** 只验证 `.xiaoc`/`.xar`，不构建、不执行用户代码。 */
export interface VerifyRequest {
  type: "verify";
  request_id: string;
  protocol_version: number;
  core_version: number;
  path: string;
  detail: boolean;
}

/** 16B 缓存查询或两阶段维护请求。 */
export interface CacheRequest {
  type: "cache";
  request_id: string;
  protocol_version: number;
  core_version: number;
  action: "list" | "verify" | "rebuild" | "clean";
  apply: boolean;
}

/** 项目测试请求；cases 顺序就是核心执行和结果返回顺序。 */
export interface TestRequest {
  type: "test";
  request_id: string;
  protocol_version: number;
  core_version: number;
  language_version: string;
  runtime_version: string;
  /** 本次测试的可选语言标签；旧客户端可省略。 */
  locale?: string | null;
  target: ProtocolTarget;
  optimization: OptimizationConfig;
  cases: SourceIdentity[];
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
  /** 本次构建的可选语言标签；旧客户端可省略。 */
  locale?: string | null;
  target: ProtocolTarget;
  optimization: OptimizationConfig;
  source: SourceIdentity;
  output: string;
  llvm_ir_output: string | null;
  toolchain: ToolchainSpec;
  /** 可选的原始 config.xiao；由 Rust 配置解析器验证并固化。 */
  config_text?: string | null;
}

/** 环境指纹请求；项目根只用于布局，不进入任何指纹字段。 */
export interface EnvironmentRequest {
  type: "environment";
  request_id: string;
  protocol_version: number;
  core_version: number;
  project_root: string;
  logical_name: string | null;
  config_text: string | null;
  /** 环境诊断使用的可选语言标签；旧客户端可省略。 */
  locale?: string | null;
  target: ProtocolTarget;
  toolchain: ToolchainSpec;
}

/** 只读查询 REPL 所选环境的包根与可选模块接口，不触发运行时加载。 */
export interface ReplPackagesRequest {
  type: "repl_packages";
  request_id: string;
  protocol_version: number;
  core_version: number;
  /** REPL 包视图诊断使用的可选语言标签；旧客户端可省略。 */
  locale?: string | null;
  active_environment?: string | null;
  module_path?: string | null;
}

/** Rust 侧唯一的包操作入口。 */
export interface PackageRequest {
  type: "package";
  request_id: string;
  protocol_version: number;
  core_version: number;
  /** 包操作诊断使用的可选语言标签；旧客户端可省略。 */
  locale?: string | null;
  operation: "sync" | "install" | "lock" | "update" | "add" | "remove";
  project_root: string;
  active_environment: string | null;
  config_text: string;
  keep_extra: boolean;
  locked: boolean;
  frozen: boolean;
  package_name?: string | null;
  package_path?: string | null;
  package_version?: string | null;
  development?: boolean;
  target: ProtocolTarget;
  toolchain: ToolchainSpec;
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
export type ProtocolRequest = HelloRequest | RunRequest | RunArchiveRequest | VerifyRequest | CacheRequest | TestRequest | BuildRequest | EnvironmentRequest | ReplPackagesRequest | PackageRequest | CancelRequest | ShutdownRequest;

/** 机器可读协议错误。 */
export interface ProtocolErrorBody {
  code: string;
  message_id: string;
  message: string;
  /** Rust 核心提供的可选本地化人类文本。 */
  text?: string | null;
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
  operation: "run" | "run_archive" | "build" | "verify" | "cache";
  exit_code: number;
  exit_name: string;
  diagnostics: unknown[];
  report: unknown | null;
  events: unknown[];
  metrics: unknown | null;
  value: unknown | null;
  artifact: unknown | null;
  /** 归档运行前的机器可读校验记录；源码和构建结果为 null。 */
  audit?: ArchiveAuditRecord | null;
}

/** 归档启动前校验的稳定摘要；不包含路径、凭据或环境变量值。 */
export interface ArchiveAuditRecord {
  archive_digest: string;
  index_schema_major: number;
  index_schema_minor: number;
  verified_member_count: number;
  archive_platform: string;
  host_platform: string;
  runtime_abi_compatible: boolean;
  platform_compatible: boolean;
  debug_activation: boolean;
  language_requested: string;
  language_effective: string;
  language_fallback: boolean;
}

/** 环境指纹元数据。 */
export interface EnvironmentMetadata {
  metadata_version: number;
  logical_name: string;
  directory_name: string;
  config_fingerprint: string;
  toolchain_fingerprint: string;
  target_fingerprint: string;
  environment_fingerprint: string;
  lockfile_summary: string | null;
}

/** 环境指纹生成结果。 */
export interface EnvironmentResultResponse {
  type: "environment_result";
  request_id: string;
  metadata: EnvironmentMetadata;
}

/** 与包源、版本绑定的 REPL 根名；不等同于包管理操作。 */
export interface ReplPackage {
  root: string;
  identity: {
    name: string;
    version: string;
    source: { source_id: string; alias: string | null; display_name: string };
  };
}

/** 一个模块的静态导出，不含运行时对象。 */
export interface ReplInterface {
  module_path: string;
  exports: { name: string; kind: "value" | "function" | "table" | "module" | "namespace"; signature: string | null }[];
}

/** 当前环境的包根及可选接口读取结果。 */
export interface ReplPackagesResultResponse {
  type: "repl_packages_result";
  request_id: string;
  environment_path: string;
  packages: ReplPackage[];
  interface: ReplInterface | null;
}

/** 包操作结果；环境选择不在 CLI 重算。 */
export interface PackageResultResponse {
  type: "package_result";
  request_id: string;
  result: { environment_path: string; created: boolean; changed: boolean; activate: boolean; lock_status: string | null };
}

/** 一个项目测试用例的结构化执行结果。 */
export interface ProtocolTestCaseResult {
  path: string;
  module: string;
  exit_code: number;
  exit_name: string;
  diagnostics: unknown[];
  report: unknown | null;
  events: unknown[];
  metrics: unknown | null;
  value: unknown | null;
  error: ProtocolErrorBody | null;
}

/** 项目测试聚合结果；tests 顺序与请求 cases 顺序一致。 */
export interface TestResultResponse {
  type: "test_result";
  request_id: string;
  operation: "test";
  exit_code: number;
  exit_name: string;
  total: number;
  passed: number;
  failed: number;
  tests: ProtocolTestCaseResult[];
}

/** 原生构建产物的稳定摘要。 */
export interface ProtocolArtifact {
  executable: string;
  llvm_ir_output: string | null;
  toolchain_fingerprint: string;
  uses_runtime: boolean;
  runtime_components: string[];
  optimization_level?: number;
  artifact_runtime?: ProtocolArtifactRuntime | null;
  diagnostic_activation?: ProtocolDiagnosticActivation | null;
  /** 随调试产物复制的独立诊断组件。 */
  diagnostics_component?: string | null;
  /** 构建时固化的运行时配置旁置文件。 */
  runtime_config?: ProtocolRuntimeConfig | null;
}

/** 链接后产物的 Runtime 组成事实。 */
export interface ProtocolArtifactRuntime {
  object_format: string;
  declared_components: string[];
  observed_components: string[];
  runtime_symbols: string[];
  /** 未能映射到已知组件的 Runtime 符号；旧服务端可能省略。 */
  unclassified_runtime_symbols?: string[];
  dependencies: string[];
  diagnostic_symbols: string[];
  /** 产物观察可信度；COFF 导出表只能作为未验证的自报事实。 */
  verification?: string;
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
export type ProtocolResponse = HelloResponse | ResultResponse | EnvironmentResultResponse | ReplPackagesResultResponse | PackageResultResponse | TestResultResponse | ErrorResponse | CancelledResponse | ShutdownResponse;

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
