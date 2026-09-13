/** A0 仓库完整性检查器的 Bun 命令行入口。 */

import { resolve } from "node:path";

import { checkDevDocs, checkUseDocs } from "./docs.ts";
import { findRepositoryRoot, loadRepository } from "./manifest.ts";
import { checkLayout, checkLoadedLayout } from "./layout.ts";
import { emitReport, mergeResults, type ReportFormat } from "./report.ts";
import type { CheckResult } from "./types.ts";

/**
 * A0 检查器支持的子命令。
 */
export type RepoCheckCommand = "layout" | "docs" | "usedocs" | "all";

/**
 * 解析检查器命令行参数。
 *
 * @param argv 不含 Bun 可执行文件和脚本路径的参数。
 * @returns 规范化参数。
 */
export function parseArguments(argv: string[]): {
  command: RepoCheckCommand;
  root?: string;
  format: ReportFormat;
  output?: string;
} {
  let command: RepoCheckCommand = "all";
  let root: string | undefined;
  let format: ReportFormat = "text";
  let output: string | undefined;
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "layout" || argument === "docs" || argument === "usedocs" || argument === "all") {
      command = argument;
    } else if (argument === "--root") {
      root = argv[++index];
    } else if (argument === "--format") {
      const candidate = argv[++index];
      if (candidate !== "text" && candidate !== "json" && candidate !== "sarif") throw new Error(`不支持的报告格式：${candidate}`);
      format = candidate;
    } else if (argument === "--out") {
      output = argv[++index];
    } else if (argument === "--help" || argument === "-h") {
      command = "all";
    } else {
      throw new Error(`未知参数：${argument}`);
    }
  }
  return { command, root, format, output };
}

/**
 * 执行指定的仓库检查命令。
 *
 * @param options 规范化命令参数。
 * @returns 检查结果及实际仓库根目录。
 */
export async function runCommand(options: ReturnType<typeof parseArguments>): Promise<{ result: CheckResult; root: string }> {
  const start = options.root ?? process.cwd();
  if (options.command === "layout") {
    const result = checkLayout(start);
    return { result, root: findRepositoryRoot(start) ?? resolve(start) };
  }
  const loaded = loadRepository(start);
  if (!loaded.repository) return { result: { passed: false, diagnostics: loaded.diagnostics }, root: findRepositoryRoot(start) ?? resolve(start) };
  const results: CheckResult[] = [{ passed: loaded.diagnostics.length === 0, diagnostics: loaded.diagnostics }];
  if (options.command === "all") results.push({ passed: true, diagnostics: checkLoadedLayout(loaded.repository) });
  if (options.command === "docs" || options.command === "all") results.push({ passed: true, diagnostics: checkDevDocs(loaded.repository.root) });
  if (options.command === "usedocs" || options.command === "all") {
    results.push({ passed: true, diagnostics: loaded.repository.registry ? checkUseDocs(loaded.repository.root, loaded.repository.registry) : [] });
  }
  if (options.command === "all") {
    const coverage = await import("../../doc-coverage/src/checker.ts");
    results.push(await coverage.checkCoverage(loaded.repository.root));
  }
  return { result: mergeResults(results), root: loaded.repository.root };
}

if (import.meta.main) {
  try {
    const options = parseArguments(process.argv.slice(2));
    if (process.argv.includes("--help") || process.argv.includes("-h")) {
      process.stdout.write("用法：xiao-repo-check [layout|docs|usedocs|all] [--root 路径] [--format text|json|sarif] [--out 文件]\n");
      process.exit(0);
    }
    const { result, root } = await runCommand(options);
    emitReport(result, options.format, options.output, root, "xiao-repo-check");
    process.exitCode = result.passed ? 0 : 1;
  } catch (error) {
    process.stderr.write(`A0-INTERNAL-001：${String(error)}\n`);
    process.exitCode = 2;
  }
}
