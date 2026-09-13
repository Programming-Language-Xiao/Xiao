/** 文档覆盖率 text、JSON 和 SARIF 输出。 */

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join } from "node:path";

import { toCoverageReport } from "./checker.ts";
import type { CoverageResult } from "./types.ts";

/**
 * 覆盖率报告输出格式。
 */
export type CoverageReportFormat = "text" | "json" | "sarif";

/**
 * 渲染覆盖率结果为终端文本。
 *
 * @param result 覆盖率结果。
 * @returns 可读文本。
 */
export function renderCoverageText(result: CoverageResult): string {
  const summary = result.summary;
  const lines = [
    `总体：${summary.documented}/${summary.total}（${summary.percentage.toFixed(2)}%）`,
    `公共 API：${summary.publicDocumented}/${summary.publicTotal}（${summary.publicPercentage.toFixed(2)}%）`,
    `状态：${result.passed ? "通过" : "失败"}`,
  ];
  for (const member of summary.members) {
    lines.push(`成员 ${member.member}：${member.documented}/${member.total}（${member.percentage.toFixed(2)}%），公共 ${member.publicDocumented}/${member.publicTotal}（${member.publicPercentage.toFixed(2)}%）`);
  }
  for (const diagnostic of result.diagnostics) {
    const location = diagnostic.line ? `${diagnostic.path}:${diagnostic.line}` : diagnostic.path || "仓库";
    lines.push(`[${diagnostic.severity}] ${diagnostic.code} ${location} ${diagnostic.subject}：${diagnostic.message}`);
  }
  return lines.join("\n");
}

/**
 * 将覆盖率结果转换为 SARIF 2.1.0。
 *
 * @param result 覆盖率结果。
 * @returns SARIF 对象。
 */
export function renderCoverageSarif(result: CoverageResult): Record<string, unknown> {
  return {
    version: "2.1.0",
    $schema: "https://json.schemastore.org/sarif-2.1.0.json",
    runs: [{
      tool: { driver: { name: "xiao-doc-coverage", version: result.scannerVersion } },
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
 * 输出覆盖率报告。
 *
 * @param result 覆盖率结果。
 * @param format 输出格式。
 * @param output 可选输出路径。
 * @param root 仓库根目录。
 * @returns 序列化后的报告文本。
 */
export function emitCoverageReport(result: CoverageResult, format: CoverageReportFormat, output: string | undefined, root: string): string {
  const value = format === "text" ? renderCoverageText(result) : JSON.stringify(format === "json" ? toCoverageReport(result) : renderCoverageSarif(result), null, 2);
  if (output) {
    const target = isAbsolute(output) ? output : join(root, output);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, `${value}\n`, "utf8");
  } else {
    process.stdout.write(`${value}\n`);
  }
  return value;
}
