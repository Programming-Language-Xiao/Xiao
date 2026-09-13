/** 文档覆盖率工具的 Bun 命令行入口。 */

import { findRepositoryRoot } from "../../repo-check/src/manifest.ts";
import { checkCoverage } from "./checker.ts";
import { emitCoverageReport, type CoverageReportFormat } from "./report.ts";

/**
 * 解析覆盖率命令行参数。
 *
 * @param argv 不含脚本路径的参数。
 * @returns 规范化覆盖率选项。
 */
export function parseCoverageArguments(argv: string[]): {
  root?: string;
  format: CoverageReportFormat;
  output?: string;
  totalThreshold?: number;
  publicThreshold?: number;
  rustAdapter?: string;
} {
  let root: string | undefined;
  let format: CoverageReportFormat = "text";
  let output: string | undefined;
  let totalThreshold: number | undefined;
  let publicThreshold: number | undefined;
  let rustAdapter: string | undefined;
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--root") root = argv[++index];
    else if (argument === "--format") {
      const candidate = argv[++index];
      if (candidate !== "text" && candidate !== "json" && candidate !== "sarif") throw new Error(`不支持的报告格式：${candidate}`);
      format = candidate;
    } else if (argument === "--out") output = argv[++index];
    else if (argument === "--total-threshold") totalThreshold = parseThreshold(argv[++index], "总体");
    else if (argument === "--public-threshold") publicThreshold = parseThreshold(argv[++index], "公共");
    else if (argument === "--rust-adapter") rustAdapter = argv[++index];
    else if (argument === "--help" || argument === "-h") continue;
    else throw new Error(`未知参数：${argument}`);
  }
  return { root, format, output, totalThreshold, publicThreshold, rustAdapter };
}

/**
 * 执行覆盖率命令。
 *
 * @param options 规范化参数。
 * @returns 覆盖率结果和仓库根目录。
 */
export async function runCoverageCommand(options: ReturnType<typeof parseCoverageArguments>) {
  const start = options.root ?? process.cwd();
  const result = await checkCoverage({
    root: start,
    totalThreshold: options.totalThreshold,
    publicThreshold: options.publicThreshold,
    rustAdapter: options.rustAdapter,
  });
  return { result, root: findRepositoryRoot(start) ?? start };
}

/** 解析并验证百分比阈值。 */
function parseThreshold(value: string | undefined, label: string): number {
  const result = Number(value);
  if (!Number.isFinite(result) || result < 0 || result > 100) throw new Error(`${label}阈值必须是 0 到 100 之间的数字：${value}`);
  return result;
}

if (import.meta.main) {
  try {
    const options = parseCoverageArguments(process.argv.slice(2));
    if (process.argv.includes("--help") || process.argv.includes("-h")) {
      process.stdout.write("用法：xiao-doc-coverage [--root 路径] [--format text|json|sarif] [--out 文件] [--total-threshold 数字] [--public-threshold 数字] [--rust-adapter 文件]\n");
      process.exit(0);
    }
    const { result, root } = await runCoverageCommand(options);
    emitCoverageReport(result, options.format, options.output, root);
    process.exitCode = result.passed ? 0 : 1;
  } catch (error) {
    process.stderr.write(`A0-INTERNAL-001：${String(error)}\n`);
    process.exitCode = 2;
  }
}
