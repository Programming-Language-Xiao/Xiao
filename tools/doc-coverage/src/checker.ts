/** 文档覆盖率计算、阈值判定和报告数据模型。 */

import { readdirSync, realpathSync } from "node:fs";
import { join, relative, resolve } from "node:path";

import { findRepositoryRoot, loadRepositoryManifest } from "../../repo-check/src/manifest.ts";
import { isInside } from "../../repo-check/src/paths.ts";
import { scanRustFiles } from "./rust-adapter.ts";
import { scanTypeScriptFile } from "./typescript-adapter.ts";
import type {
  CoverageDiagnostic,
  CoverageOptions,
  CoverageResult,
  CoverageSummary,
  DeclarationRecord,
  MemberSummary,
} from "./types.ts";

/**
 * 当前覆盖率报告协议版本。
 */
export const COVERAGE_SCHEMA_VERSION = 1;

/**
 * 当前扫描器版本。
 */
export const SCANNER_VERSION = "0.1.0";

/**
 * 扫描仓库并检查公共 API 与总体文档覆盖率门槛。
 *
 * @param startOrOptions 仓库路径或完整检查选项。
 * @returns 含声明明细、成员摘要和稳定诊断的结果。
 */
export async function checkCoverage(startOrOptions: string | CoverageOptions = process.cwd()): Promise<CoverageResult> {
  const options: CoverageOptions = typeof startOrOptions === "string" ? { root: startOrOptions } : startOrOptions;
  const start = options.root ?? process.cwd();
  const root = findRepositoryRoot(start) ?? resolve(start);
  const totalThreshold = options.totalThreshold ?? 90;
  const publicThreshold = options.publicThreshold ?? 100;
  const diagnostics: CoverageDiagnostic[] = [];
  const loaded = loadRepositoryManifest(root);
  diagnostics.push(...loaded.diagnostics.map(toCoverageDiagnostic));
  if (!loaded.manifest) return buildResult([], diagnostics, totalThreshold, publicThreshold);

  const files = discoverSourceFiles(root, loaded.manifest.codeRoots, loaded.manifest.sourceExtensions, new Set(loaded.manifest.excludedDirectories));
  const rustFiles = files.filter((file) => file.toLowerCase().endsWith(".rs"));
  const typeScriptFiles = files.filter((file) => /\.(?:tsx?|mts|cts)$/iu.test(file));
  const declarations: DeclarationRecord[] = [];
  const rust = scanRustFiles({ root, files: rustFiles, adapterPath: options.rustAdapter });
  declarations.push(...rust.declarations);
  diagnostics.push(...rust.diagnostics);
  for (const file of typeScriptFiles) {
    try {
      declarations.push(...scanTypeScriptFile(root, file));
    } catch (error) {
      diagnostics.push({
        code: "A0-PARSER-001",
        severity: "error",
        path: relativePath(root, file),
        subject: file,
        message: `TypeScript AST 解析失败：${String(error)}`,
        hint: "修复源文件或 TypeScript 编译器版本后重试。",
        message_id: "a0.parser.typescript_source",
      });
    }
  }
  return buildResult(declarations, diagnostics, totalThreshold, publicThreshold, loaded.manifest.rust.members.concat(loaded.manifest.typescript.members));
}

/**
 * 计算一组声明的覆盖率百分比。
 *
 * @param declarations 声明记录。
 * @returns 总体和公共 API 的统计数字。
 */
export function calculateSummary(declarations: DeclarationRecord[], members: string[] = []): CoverageSummary {
  const documented = declarations.filter((item) => item.hasDoc).length;
  const publicDeclarations = declarations.filter((item) => item.isPublic);
  const publicDocumented = publicDeclarations.filter((item) => item.hasDoc).length;
  const memberSummaries: MemberSummary[] = members.map((member) => {
    const selected = declarations.filter((item) => item.file === member || item.file.startsWith(`${member}/`));
    const selectedPublic = selected.filter((item) => item.isPublic);
    return {
      member,
      total: selected.length,
      documented: selected.filter((item) => item.hasDoc).length,
      percentage: percentage(selected.filter((item) => item.hasDoc).length, selected.length),
      publicTotal: selectedPublic.length,
      publicDocumented: selectedPublic.filter((item) => item.hasDoc).length,
      publicPercentage: percentage(selectedPublic.filter((item) => item.hasDoc).length, selectedPublic.length),
    };
  });
  return {
    total: declarations.length,
    documented,
    percentage: percentage(documented, declarations.length),
    publicTotal: publicDeclarations.length,
    publicDocumented,
    publicPercentage: percentage(publicDocumented, publicDeclarations.length),
    members: memberSummaries,
  };
}

/**
 * 以稳定 JSON 结构导出覆盖率结果。
 *
 * @param result 覆盖率结果。
 * @returns 可序列化报告对象。
 */
export function toCoverageReport(result: CoverageResult): Record<string, unknown> {
  return {
    schemaVersion: COVERAGE_SCHEMA_VERSION,
    scannerVersion: result.scannerVersion,
    passed: result.passed,
    summary: result.summary,
    declarations: result.declarations,
    diagnostics: result.diagnostics,
  };
}

/** 根据声明和阈值组装最终覆盖率结果。 */
function buildResult(
  declarations: DeclarationRecord[],
  diagnostics: CoverageDiagnostic[],
  totalThreshold: number,
  publicThreshold: number,
  members: string[] = [],
): CoverageResult {
  const summary = calculateSummary(declarations, members);
  if (summary.publicPercentage < publicThreshold) {
    diagnostics.push({
      code: "A0-COVERAGE-001",
      severity: "error",
      path: "",
      subject: "public-api",
      message: `公共 API 文档覆盖率 ${summary.publicPercentage.toFixed(2)}% 低于 ${publicThreshold}%。`,
      hint: "为每个 pub/export 声明添加直接关联的 Rustdoc/JSDoc。",
      message_id: "a0.coverage.public_threshold",
    });
  }
  if (summary.percentage < totalThreshold) {
    diagnostics.push({
      code: "A0-COVERAGE-001",
      severity: "error",
      path: "",
      subject: "repository",
      message: `全仓库文档覆盖率 ${summary.percentage.toFixed(2)}% 低于 ${totalThreshold}%。`,
      hint: "为缺口声明补充代码文档，或在清单中明确登记机器生成文件。",
      message_id: "a0.coverage.total_threshold",
    });
  }
  for (const declaration of declarations.filter((item) => !item.hasDoc)) {
    diagnostics.push({
      code: "A0-COVERAGE-002",
      severity: declaration.isPublic ? "error" : "warning",
      path: declaration.file,
      line: declaration.line,
      subject: `${declaration.kind} ${declaration.name}`,
      message: declaration.isPublic ? "公共声明缺少代码文档。" : "声明缺少代码文档。",
      hint: "在声明前添加实质性的 Rustdoc/JSDoc；UseDocs 不能替代它。",
      message_id: "a0.coverage.declaration_missing",
    });
  }
  return {
    passed: !diagnostics.some((item) => item.severity === "error"),
    diagnostics,
    summary,
    declarations,
    scannerVersion: SCANNER_VERSION,
  };
}

/** 按政策清单递归发现待扫描的源码文件。 */
function discoverSourceFiles(root: string, roots: string[], extensions: string[], excludedNames: Set<string>): string[] {
  const found: string[] = [];
  const extensionSet = new Set(extensions.map((item) => item.toLowerCase()));
  const visited = new Set<string>();
  const visit = (directory: string): void => {
    let real: string;
    try {
      real = realpathSync(directory);
    } catch {
      return;
    }
    if (!isInside(root, real)) return;
    if (visited.has(real)) return;
    visited.add(real);
    let entries;
    try {
      entries = readdirSync(directory, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      if (excludedNames.has(entry.name)) continue;
      const child = join(directory, entry.name);
      if (entry.isSymbolicLink()) continue;
      if (entry.isDirectory()) {
        visit(child);
      } else if (entry.isFile() && extensionSet.has(entry.name.slice(entry.name.lastIndexOf(".")).toLowerCase())) {
        found.push(child);
      }
    }
  };
  for (const item of roots) visit(resolve(root, item));
  return found.sort();
}

/** 计算百分比并处理空集合。 */
function percentage(numerator: number, denominator: number): number {
  return denominator === 0 ? 100 : (numerator / denominator) * 100;
}

/** 将绝对文件路径转换为仓库相对路径。 */
function relativePath(root: string, file: string): string {
  return relative(resolve(root), resolve(file)).replaceAll("\\", "/");
}

/** 将仓库检查器诊断映射为覆盖率诊断。 */
function toCoverageDiagnostic(value: { code: string; severity: string; path: string; subject: string; message: string; hint?: string; message_id?: string; line?: number }): CoverageDiagnostic {
  return {
    code: value.code,
    severity: value.severity === "warning" ? "warning" : value.severity === "info" ? "info" : "error",
    path: value.path,
    subject: value.subject,
    message: value.message,
    hint: value.hint,
    message_id: value.message_id,
    ...(value.line ? { line: value.line } : {}),
  };
}
