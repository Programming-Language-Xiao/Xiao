/** 将协议结构化结果渲染为人类终端或机器 JSON。 */

import type { Colorizer } from "../ui/color.ts";
import { createColorizer } from "../ui/color.ts";
import { renderTable } from "../ui/width.ts";
import type { ProtocolResponse } from "../protocol/messages.ts";
import { CoreClientError } from "../protocol/client.ts";
import { CliConfigError } from "../config/editor.ts";
import { CoreDiscoveryError } from "../platform/core.ts";
import { ToolchainDiscoveryError } from "../platform/toolchain.ts";
import { EnvironmentCommandError } from "../environments/index.ts";
import { cliMessage } from "../i18n.ts";

/** 渲染模式。 */
export interface DiagnosticRenderOptions {
  /** 机器可读 JSON 输出。 */
  json?: boolean;
  /** 颜色模式。 */
  color?: "auto" | "always" | "never";
  /** 输出是否连接 TTY。 */
  isTTY?: boolean;
  /** NO_COLOR 覆盖。 */
  noColor?: boolean;
  /** COLORTERM 值。 */
  colorTerm?: string;
  /** TERM 值。 */
  term?: string;
  /** 系统提示语言；只影响人类文本。 */
  locale?: "zh-CN" | "en-US";
}

/** 渲染后的 CLI 输出。 */
export interface RenderedDiagnostic {
  /** 写入 stdout 的文本。 */
  stdout: string;
  /** 写入 stderr 的文本。 */
  stderr: string;
  /** 应返回的进程码。 */
  exitCode: number;
}

/** CLI 自身稳定退出码；后端 `run` 结果仍直接使用协议的 B0-D 值。 */
export const CLI_EXIT_CODES = {
  /** 参数或命令错误。 */
  usage: 64,
  /** 配置读写错误。 */
  config: 78,
  /** 核心启动、协议和内部边界错误。 */
  infrastructure: 70,
} as const;

/** 将协议响应渲染为终端输出。 */
export function renderProtocolResponse(response: ProtocolResponse, options: DiagnosticRenderOptions = {}): RenderedDiagnostic {
  if (options.json) return { stdout: `${JSON.stringify(response)}\n`, stderr: "", exitCode: responseExitCode(response) };
  const colorizer = makeColorizer(options);
  if (response.type === "hello" || response.type === "shutdown" || response.type === "cancelled") {
    return { stdout: "", stderr: "", exitCode: responseExitCode(response) };
  }
  if (response.type === "test_result") return renderTestResult(response, colorizer, options);
  const diagnostics = response.type === "result" ? response.diagnostics : [];
  const lines: string[] = [];
  for (const diagnostic of diagnostics) {
    if (!isRecord(diagnostic)) continue;
    const code = typeof diagnostic.code === "string" ? diagnostic.code : "X11-DIAGNOSTIC-001";
    const message = protocolMessage(diagnostic, options.locale, code);
    lines.push(colorizer.color(diagnosticSeverity(diagnostic), `${code}: ${message}`));
  }
  if (response.type === "error") {
    lines.push(renderProtocolError(response.error, colorizer, options.locale));
  } else if (response.type === "result" && response.report !== null && isRecord(response.report)) {
    const code = typeof response.report.code === "string" ? response.report.code : response.exit_name;
    const message = protocolMessage(response.report, options.locale, code);
    lines.push(colorizer.color("error", `${code}: ${message}`));
  }
  if (response.type === "result" && response.metrics !== null && isRecord(response.metrics) && response.exit_code === 0) {
    const locale = options.locale ?? "zh-CN";
    const rows = [
      { header: cliMessage("xiao.cli.status.label", locale), values: [cliMessage("xiao.cli.status.success", locale)] },
      { header: cliMessage("xiao.cli.status.exit_code", locale), values: [String(response.exit_code)] },
    ];
    // 表格只展示稳定字段，指标本身不参与退出判断。
    lines.push(renderTable(rows));
  }
  if (response.type === "result" && response.operation === "build" && response.artifact !== null && isRecord(response.artifact) && response.exit_code === 0) {
    const locale = options.locale ?? "zh-CN";
    const executable = typeof response.artifact.executable === "string" ? response.artifact.executable : "<unknown>";
    const fingerprint = typeof response.artifact.toolchain_fingerprint === "string" ? response.artifact.toolchain_fingerprint : "<unknown>";
    lines.push(cliMessage("xiao.cli.build.artifact", locale, { path: executable }));
    lines.push(cliMessage("xiao.cli.build.fingerprint", locale, { value: fingerprint }));
    if (isRecord(response.artifact.artifact_runtime)) {
      const runtime = response.artifact.artifact_runtime;
      const format = typeof runtime.object_format === "string" ? runtime.object_format : "unknown";
      const observed = Array.isArray(runtime.observed_components)
        ? runtime.observed_components.filter((item): item is string => typeof item === "string").join(", ")
        : "";
      const verification = runtime.verification === "unverified-coff-exports"
        ? cliMessage("xiao.cli.build.runtime_unverified", locale)
        : "";
      lines.push(cliMessage("xiao.cli.build.runtime", locale, { format, components: observed || "none" }) + verification);
    }
    if (isRecord(response.artifact.diagnostic_activation) && typeof response.artifact.diagnostic_activation.path === "string") {
      lines.push(cliMessage("xiao.cli.build.debug", locale, { path: response.artifact.diagnostic_activation.path }));
    }
    if (typeof response.artifact.diagnostics_component === "string") {
      lines.push(cliMessage("xiao.cli.build.diagnostics", locale, { path: response.artifact.diagnostics_component }));
    }
    if (isRecord(response.artifact.runtime_config) && typeof response.artifact.runtime_config.path === "string") {
      lines.push(cliMessage("xiao.cli.build.config", locale, { path: response.artifact.runtime_config.path }));
    }
  }
  if (response.type === "result" && (response.operation === "verify" || response.operation === "cache") && isRecord(response.value) && typeof response.value.value === "string" && response.exit_code === 0) {
    const locale = options.locale ?? "zh-CN";
    try {
      const summary = JSON.parse(response.value.value) as Record<string, unknown>;
      if (response.operation === "verify") {
        lines.push(cliMessage("xiao.cli.verify.success", locale, { kind: String(summary.kind ?? "artifact") }));
      } else {
        lines.push(cliMessage("xiao.cli.cache.status", locale, { action: String(summary.action ?? "cache"), status: String(summary.status ?? "unknown") }));
      }
    } catch {
      lines.push(response.value.value);
    }
  }
  const stderr = lines.length === 0 ? "" : `${lines.join("\n")}\n`;
  const stdout = response.type === "result" ? intrinsicOutput(response.events) : "";
  return { stdout, stderr, exitCode: responseExitCode(response) };
}

/** 将 VM 的结构化 intrinsic 输出还原为人类 CLI 的标准输出；JSON 模式保留事件原样。 */
function intrinsicOutput(events: unknown[]): string {
  return events
    .filter(isRecord)
    .filter((event) => event.kind === "intrinsic_output")
    .map((event) => {
      const data = isRecord(event.data) ? event.data : null;
      return data !== null && typeof data.text === "string" ? data.text : "";
    })
    .join("");
}

/** 渲染项目测试的逐用例路径、统计和结构化诊断。 */
function renderTestResult(
  response: Extract<ProtocolResponse, { type: "test_result" }>,
  colorizer: Colorizer,
  options: DiagnosticRenderOptions,
): RenderedDiagnostic {
  const locale = options.locale ?? "zh-CN";
  const lines = [
    cliMessage("xiao.cli.test.summary", locale, {
      passed: response.passed,
      total: response.total,
      failed: response.failed,
    }),
  ];
  for (const test of response.tests) {
    const passed = test.exit_code === 0;
    const status = passed ? cliMessage("xiao.cli.test.passed", locale) : cliMessage("xiao.cli.test.failed", locale);
    lines.push(colorizer.color(passed ? "success" : "error", `${status}  ${test.path}  [${test.exit_code}]`));
    for (const diagnostic of test.diagnostics) {
      if (!isRecord(diagnostic)) continue;
      const code = typeof diagnostic.code === "string" ? diagnostic.code : "X11-DIAGNOSTIC-001";
      const message = protocolMessage(diagnostic, options.locale, code);
      lines.push(`  ${colorizer.color(diagnosticSeverity(diagnostic), `${code}: ${message}`)}`);
    }
    if (test.error !== null) lines.push(`  ${renderProtocolError(test.error, colorizer, options.locale)}`);
    if (test.report !== null && isRecord(test.report)) {
      const code = typeof test.report.code === "string" ? test.report.code : test.exit_name;
      const message = protocolMessage(test.report, options.locale, code);
      lines.push(`  ${colorizer.color("error", `${code}: ${message}`)}`);
    }
  }
  return { stdout: "", stderr: `${lines.join("\n")}\n`, exitCode: response.exit_code };
}

/** 将 CLI 自身异常渲染为稳定诊断。 */
export function renderCliError(error: unknown, options: DiagnosticRenderOptions = {}): RenderedDiagnostic {
  const normalized = normalizeCliError(error);
  if (options.json) {
    return {
      stdout: `${JSON.stringify({ type: "error", code: normalized.code, message: normalized.message, details: normalized.details, exit_code: normalized.exitCode })}\n`,
      stderr: "",
      exitCode: normalized.exitCode,
    };
  }
  const colorizer = makeColorizer(options);
  const text = colorizer.color("error", `${normalized.code}: ${normalized.message}`);
  return { stdout: "", stderr: `${text}\n`, exitCode: normalized.exitCode };
}

/** 返回协议响应中的 B0-D 退出码，不读取本地化文本。 */
export function responseExitCode(response: ProtocolResponse): number {
  if (response.type === "result" || response.type === "test_result" || response.type === "error" || response.type === "cancelled") return response.exit_code;
  return 0;
}

/** 读取兼容旧核心的 Runtime 对象峰值字节；缺失或越过 JS 安全整数时返回未知。 */
export function peakLiveBytes(response: ProtocolResponse | null): number | null {
  const metrics = response?.type === "result" ? response.metrics : null;
  if (!isRecord(metrics)) return null;
  const bytes = metrics.peak_live_bytes;
  return typeof bytes === "number" && Number.isSafeInteger(bytes) && bytes >= 0 ? bytes : null;
}

/** 统一 CLI 异常的机器字段。 */
interface NormalizedCliError { code: string; message: string; details: Record<string, unknown>; exitCode: number }

/** 把协议、配置和参数异常归一化为同一诊断形状。 */
function normalizeCliError(error: unknown): NormalizedCliError {
  if (error instanceof CoreClientError) return { code: error.code, message: error.message.replace(`${error.code}: `, ""), details: error.details, exitCode: error.exitCode };
  if (error instanceof CoreDiscoveryError) {
    return {
      code: error.code,
      message: error.message.replace(`${error.code}: `, ""),
      details: {
        candidates: error.candidates,
        candidate_sources: error.candidateDetails,
      },
      exitCode: CLI_EXIT_CODES.infrastructure,
    };
  }
  if (error instanceof ToolchainDiscoveryError) {
    return {
      code: error.code,
      message: error.message.replace(`${error.code}: `, ""),
      details: { ...error.details, candidates: error.candidates },
      exitCode: CLI_EXIT_CODES.infrastructure,
    };
  }
  if (error instanceof EnvironmentCommandError) {
    return {
      code: error.code,
      message: error.message.replace(`${error.code}: `, ""),
      details: error.details,
      exitCode: error.exitCode,
    };
  }
  if (error instanceof CliConfigError) return { code: error.code, message: error.message.replace(`${error.code}: `, ""), details: { ...error.details, path: error.path }, exitCode: CLI_EXIT_CODES.config };
  if (isCommandError(error)) return { code: error.code, message: error.message.replace(`${error.code}: `, ""), details: error.details, exitCode: error.exitCode };
  if (isArgumentError(error)) return { code: error.code, message: error.message.replace(`${error.code}: `, ""), details: {}, exitCode: CLI_EXIT_CODES.usage };
  if (error instanceof Error) return { code: "X11-CLI-001", message: error.message, details: {}, exitCode: CLI_EXIT_CODES.infrastructure };
  return { code: "X11-CLI-001", message: String(error), details: {}, exitCode: CLI_EXIT_CODES.infrastructure };
}

/** 渲染协议错误体；状态判断已经在调用方读取结构化字段完成。 */
function renderProtocolError(error: unknown, colorizer: Colorizer, locale?: "zh-CN" | "en-US"): string {
  if (!isRecord(error)) return colorizer.color("error", "X11-PROTOCOL-002: 协议错误响应无效");
  const code = typeof error.code === "string" ? error.code : "X11-PROTOCOL-002";
  const message = protocolMessage(error, locale, code);
  const nextStep = locale !== "en-US" && typeof error.next_step === "string" ? `（${error.next_step}）` : "";
  return colorizer.color("error", `${code}: ${message}${nextStep}`);
}

/** Rust 已渲染的文本优先；旧核心在已选语言时退回稳定消息身份和参数。 */
function protocolMessage(value: Record<string, unknown>, locale: "zh-CN" | "en-US" | undefined, code: string): string {
  if (typeof value.text === "string" && value.text.length > 0) return value.text;
  if (locale !== undefined && typeof value.message_id === "string") {
    const params = isRecord(value.params) ? Object.entries(value.params).sort(([left], [right]) => left.localeCompare(right))
      .map(([name, param]) => `${name}=${JSON.stringify(isRecord(param) && "value" in param ? param.value : param)}`) : [];
    return `${value.message_id}${params.length === 0 ? "" : ` (${params.join(", ")})`}`;
  }
  return typeof value.message === "string" ? value.message : code;
}

/** 把协议诊断级别映射为少量语义色角色。 */
function diagnosticSeverity(value: Record<string, unknown>): "success" | "error" | "info" {
  return value.severity === "error" ? "error" : value.severity === "warning" ? "info" : "info";
}

/** 根据终端能力创建颜色器。 */
function makeColorizer(options: DiagnosticRenderOptions): Colorizer {
  return createColorizer({
    mode: options.color ?? "auto",
    isTTY: options.isTTY,
    noColor: options.noColor,
    colorTerm: options.colorTerm,
    term: options.term,
  });
}

/** 判断未知 JSON 值是否为对象。 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** 判断命令执行器异常的结构化形状。 */
function isCommandError(value: unknown): value is { code: string; message: string; details: Record<string, unknown>; exitCode: number } {
  return isRecord(value)
    && typeof value.code === "string"
    && typeof value.message === "string"
    && typeof value.exitCode === "number"
    && isRecord(value.details);
}

/** 判断参数解析器抛出的稳定错误形状，避免呈现层依赖命令模块。 */
/** 判断参数解析器异常的结构化形状。 */
function isArgumentError(value: unknown): value is { code: string; message: string; usage: true } {
  return isRecord(value) && typeof value.code === "string" && typeof value.message === "string" && value.usage === true;
}
