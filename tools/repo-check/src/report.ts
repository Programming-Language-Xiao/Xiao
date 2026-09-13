/** 仓库检查器的文本、JSON 和 SARIF 报告渲染。 */

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

import type { CheckResult, Diagnostic } from "./types.ts";

/**
 * 检查报告输出格式。
 */
export type ReportFormat = "text" | "json" | "sarif";

/**
 * 将检查结果渲染为人类可读文本。
 *
 * @param result 检查结果。
 * @returns 可直接打印到终端的文本。
 */
export function renderText(result: CheckResult): string {
  if (result.diagnostics.length === 0) return "通过：未发现 A0 规则问题。";
  return result.diagnostics.map((item) => {
    const location = item.line ? `${item.path}:${item.line}` : item.path || "仓库";
    const hint = item.hint ? `\n  修复：${item.hint}` : "";
    return `[${item.severity}] ${item.code} ${location} ${item.subject}\n  ${item.message}${hint}`;
  }).join("\n");
}

/**
 * 将检查结果编码为稳定 JSON 对象。
 *
 * @param result 检查结果。
 * @param checker 检查器名称。
 * @returns 可序列化报告。
 */
export function renderJson(result: CheckResult, checker = "repo-check"): Record<string, unknown> {
  return {
    schemaVersion: 1,
    checker,
    passed: result.passed,
    diagnostics: result.diagnostics,
  };
}

/**
 * 将检查结果转换为 SARIF 2.1.0 报告。
 *
 * @param result 检查结果。
 * @param checker 检查器名称。
 * @returns SARIF 对象。
 */
export function renderSarif(result: CheckResult, checker = "repo-check"): Record<string, unknown> {
  return {
    version: "2.1.0",
    $schema: "https://json.schemastore.org/sarif-2.1.0.json",
    runs: [{
      tool: { driver: { name: checker, informationUri: "https://xiao.dev" } },
      results: result.diagnostics.map((item) => ({
        ruleId: item.code,
        level: item.severity === "error" ? "error" : item.severity === "warning" ? "warning" : "note",
        message: { text: item.message },
        locations: item.path ? [{ physicalLocation: { artifactLocation: { uri: item.path }, ...(item.line ? { region: { startLine: item.line } } : {}) } }] : [],
      })),
    }],
  };
}

/**
 * 按指定格式输出报告到终端或文件。
 *
 * @param result 检查结果。
 * @param format 输出格式。
 * @param outputPath 可选的仓库相对或绝对输出路径。
 * @param root 仓库根目录。
 * @param checker 报告来源名称。
 * @returns 已输出的字符串（文本格式）或 JSON 字符串。
 */
export function emitReport(
  result: CheckResult,
  format: ReportFormat,
  outputPath: string | undefined,
  root: string,
  checker = "repo-check",
): string {
  const value = format === "text" ? renderText(result) : JSON.stringify(format === "json" ? renderJson(result, checker) : renderSarif(result, checker), null, 2);
  if (outputPath) {
    const absolute = outputPath.match(/^(?:[A-Za-z]:[\\/]|\/)/) ? outputPath : `${root}/${outputPath}`;
    mkdirSync(dirname(absolute), { recursive: true });
    writeFileSync(absolute, `${value}\n`, "utf8");
  } else {
    process.stdout.write(`${value}\n`);
  }
  return value;
}

/**
 * 合并多次检查结果并保留诊断顺序。
 *
 * @param results 待合并结果。
 * @returns 合并后的结果。
 */
export function mergeResults(results: CheckResult[]): CheckResult {
  const diagnostics: Diagnostic[] = results.flatMap((item) => item.diagnostics);
  return { passed: !diagnostics.some((item) => item.severity === "error"), diagnostics };
}
