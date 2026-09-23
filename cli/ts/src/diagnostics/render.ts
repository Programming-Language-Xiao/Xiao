/** 将协议结构化结果渲染为人类终端或机器 JSON。 */

import type { Colorizer } from "../ui/color.ts";
import { createColorizer } from "../ui/color.ts";
import { renderTable } from "../ui/width.ts";
import type { ProtocolResponse } from "../protocol/messages.ts";
import { CoreClientError } from "../protocol/client.ts";
import { CliConfigError } from "../config/editor.ts";
import { CoreDiscoveryError } from "../platform/core.ts";
import { ToolchainDiscoveryError } from "../platform/toolchain.ts";

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
    const message = typeof diagnostic.message === "string" ? diagnostic.message : code;
    lines.push(colorizer.color(diagnosticSeverity(diagnostic), `${code}: ${message}`));
  }
  if (response.type === "error") {
    lines.push(renderProtocolError(response.error, colorizer));
  } else if (response.type === "result" && response.report !== null && isRecord(response.report)) {
    const code = typeof response.report.code === "string" ? response.report.code : response.exit_name;
    const message = typeof response.report.message === "string" ? response.report.message : code;
    lines.push(colorizer.color("error", `${code}: ${message}`));
  }
  if (response.type === "result" && response.metrics !== null && isRecord(response.metrics) && response.exit_code === 0) {
    const rows = [
      { header: "状态", values: [localized(options.locale, "成功", "success")] },
      { header: "退出码", values: [String(response.exit_code)] },
    ];
    // 表格只展示稳定字段，指标本身不参与退出判断。
    lines.push(renderTable(rows));
  }
  if (response.type === "result" && response.operation === "build" && response.artifact !== null && isRecord(response.artifact) && response.exit_code === 0) {
    const executable = typeof response.artifact.executable === "string" ? response.artifact.executable : "<unknown>";
    const fingerprint = typeof response.artifact.toolchain_fingerprint === "string" ? response.artifact.toolchain_fingerprint : "<unknown>";
    lines.push(`产物  ${executable}`);
    lines.push(`指纹  ${fingerprint}`);
    if (isRecord(response.artifact.diagnostic_activation) && typeof response.artifact.diagnostic_activation.path === "string") {
      lines.push(`调试  ${response.artifact.diagnostic_activation.path}`);
    }
    if (typeof response.artifact.diagnostics_component === "string") {
      lines.push(`诊断  ${response.artifact.diagnostics_component}`);
    }
    if (isRecord(response.artifact.runtime_config) && typeof response.artifact.runtime_config.path === "string") {
      lines.push(`配置  ${response.artifact.runtime_config.path}`);
    }
  }
  const stderr = lines.length === 0 ? "" : `${lines.join("\n")}\n`;
  return { stdout: "", stderr, exitCode: responseExitCode(response) };
}

/** 渲染项目测试的逐用例路径、统计和结构化诊断。 */
function renderTestResult(
  response: Extract<ProtocolResponse, { type: "test_result" }>,
  colorizer: Colorizer,
  options: DiagnosticRenderOptions,
): RenderedDiagnostic {
  const lines = [
    `${localized(options.locale, "测试结果", "test results")}  ${response.passed}/${response.total} ${localized(options.locale, "通过", "passed")}，${localized(options.locale, "失败", "failed")} ${response.failed}`,
  ];
  for (const test of response.tests) {
    const passed = test.exit_code === 0;
    const status = passed ? localized(options.locale, "通过", "passed") : localized(options.locale, "失败", "failed");
    lines.push(colorizer.color(passed ? "success" : "error", `${status}  ${test.path}  [${test.exit_code}]`));
    for (const diagnostic of test.diagnostics) {
      if (!isRecord(diagnostic)) continue;
      const code = typeof diagnostic.code === "string" ? diagnostic.code : "X11-DIAGNOSTIC-001";
      const message = typeof diagnostic.message === "string" ? diagnostic.message : code;
      lines.push(`  ${colorizer.color(diagnosticSeverity(diagnostic), `${code}: ${message}`)}`);
    }
    if (test.error !== null) lines.push(`  ${renderProtocolError(test.error, colorizer)}`);
    if (test.report !== null && isRecord(test.report)) {
      const code = typeof test.report.code === "string" ? test.report.code : test.exit_name;
      const message = typeof test.report.message === "string" ? test.report.message : code;
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
  if (error instanceof CliConfigError) return { code: error.code, message: error.message.replace(`${error.code}: `, ""), details: { ...error.details, path: error.path }, exitCode: CLI_EXIT_CODES.config };
  if (isCommandError(error)) return { code: error.code, message: error.message.replace(`${error.code}: `, ""), details: error.details, exitCode: error.exitCode };
  if (isArgumentError(error)) return { code: error.code, message: error.message.replace(`${error.code}: `, ""), details: {}, exitCode: CLI_EXIT_CODES.usage };
  if (error instanceof Error) return { code: "X11-CLI-001", message: error.message, details: {}, exitCode: CLI_EXIT_CODES.infrastructure };
  return { code: "X11-CLI-001", message: String(error), details: {}, exitCode: CLI_EXIT_CODES.infrastructure };
}

/** 渲染协议错误体；状态判断已经在调用方读取结构化字段完成。 */
function renderProtocolError(error: unknown, colorizer: Colorizer): string {
  if (!isRecord(error)) return colorizer.color("error", "X11-PROTOCOL-002: 协议错误响应无效");
  const code = typeof error.code === "string" ? error.code : "X11-PROTOCOL-002";
  const message = typeof error.message === "string" ? error.message : code;
  const nextStep = typeof error.next_step === "string" ? `（${error.next_step}）` : "";
  return colorizer.color("error", `${code}: ${message}${nextStep}`);
}

/** 把协议诊断级别映射为少量语义色角色。 */
function diagnosticSeverity(value: Record<string, unknown>): "success" | "error" | "info" {
  return value.severity === "error" ? "error" : value.severity === "warning" ? "info" : "info";
}

/** 只替换人类状态文本，不参与错误或退出码计算。 */
function localized(locale: "zh-CN" | "en-US" | undefined, chinese: string, english: string): string {
  return locale === "en-US" ? english : chinese;
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
