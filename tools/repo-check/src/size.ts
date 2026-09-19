/** 单文件物理行数门禁与结构大纲摘要。 */

import { existsSync, readFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";

import type { CheckResult, Diagnostic, RepositoryManifest } from "./types.ts";
import { scanRustFiles } from "../../doc-coverage/src/rust-adapter.ts";

import { resolveRepoPath } from "./paths.ts";

/** 单个源文件允许的最大物理行数。 */
export const MAX_SOURCE_LINES = 2500;

/** Rust 与 TypeScript 大纲适配器共享的节点形状。 */
export interface OutlineNodeLike {
  /** 节点类别。 */
  kind: string;
  /** 节点名称。 */
  name: string;
  /** 声明自身起始行。 */
  line: number;
  /** 整个项结束行。 */
  end_line: number;
  /** 节点覆盖行数。 */
  lines: number;
  /** 去掉名称的定义句。 */
  signature: string;
  /** 声明起始行源码。 */
  source_line: string;
  /** 递归子节点。 */
  children: OutlineNodeLike[];
}

/** 可注入的大纲提供器；测试可借此隔离真实解析器和 Rust 工具链。 */
export type OutlineProvider = (file: string) => OutlineNodeLike[] | Promise<OutlineNodeLike[]>;

/** 文件大小检查的可选配置。 */
export interface SizeCheckOptions {
  /** 超标文件的大纲提供器。 */
  outlineProvider?: OutlineProvider;
}

/**
 * 按统一口径统计物理行数。
 *
 * CRLF 算一个换行，孤立 CR、U+2028 和 U+2029 也算换行；末尾换行不增加
 * 一个虚假的空行。该计数器独立于任何 AST 解析器。
 */
export function countPhysicalLines(text: string): number {
  const lines = text.split(/\r\n|[\r\n\u2028\u2029]/u);
  if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines.length;
}

/**
 * 检查一组仓库相对源文件的单文件行数。
 *
 * 只有文件确实超过阈值时才会调用大纲提供器；行数本身永远不依赖解析器。
 */
export async function checkFileSizes(
  root: string,
  _manifest: RepositoryManifest,
  files: string[],
  options: SizeCheckOptions = {},
): Promise<CheckResult> {
  const diagnostics: Diagnostic[] = [];
  const oversized: Array<{ file: string; absolute: string; lines: number }> = [];
  for (const file of [...files].sort()) {
    let absolute: string;
    try {
      absolute = resolveRepoPath(root, file);
    } catch (error) {
      diagnostics.push({
        code: "A0-LAYOUT-002",
        severity: "error",
        path: file,
        subject: file,
        message: `无法解析待检查的源文件路径：${String(error)}`,
        hint: "确认源文件路径是仓库内的相对路径。",
        message_id: "a0.size.invalid_path",
      });
      continue;
    }
    let lines: number;
    try {
      lines = countPhysicalLines(readFileSync(absolute, "utf8"));
    } catch (error) {
      diagnostics.push({
        code: "A0-PARSER-002",
        severity: "error",
        path: file,
        subject: file,
        message: `无法读取待检查的源文件：${String(error)}`,
        hint: "确认文件存在且当前用户拥有读取权限。",
        message_id: "a0.parser.source_unreadable",
      });
      continue;
    }
    if (lines > MAX_SOURCE_LINES) oversized.push({ file, absolute, lines });
  }

  for (const item of oversized) {
    const exemption = readExemption(item.absolute);
    const exempted = exemption.exists && exemption.missing.length === 0;
    if (exemption.exists && exemption.missing.length > 0) {
      diagnostics.push({
        code: "A0-SIZE-001",
        severity: "error",
        path: item.file,
        subject: `${item.lines} 行 / 上限 ${MAX_SOURCE_LINES}`,
        message: `单文件豁免说明内容不完整：缺少${exemption.missing.join("、")}。`,
        hint: "补齐四段非空内容；不完整的说明不会降低门禁级别。",
        message_id: "a0.size.exemption_incomplete",
      });
    }

    let nodes: OutlineNodeLike[] | undefined;
    try {
      nodes = await (options.outlineProvider ?? createOutlineProvider(root))(item.absolute);
    } catch (error) {
      diagnostics.push(outlineUnavailableDiagnostic(item.file, String(error)));
    }
    const details = nodes ? renderOutlineDetails(nodes) : undefined;
    const anchor = nodes ? largestTopLevelSummary(nodes) : undefined;
    diagnostics.push({
      code: "A0-SIZE-001",
      severity: exempted ? "warning" : "error",
      path: item.file,
      subject: `${item.lines} 行 / 上限 ${MAX_SOURCE_LINES}`,
      message: exempted
        ? `单文件共 ${item.lines} 行，超过上限 ${MAX_SOURCE_LINES} 行；存在已审核的硬耦合豁免。${anchor ?? ""}`
        : `单文件共 ${item.lines} 行，超过上限 ${MAX_SOURCE_LINES} 行。${anchor ?? ""}`,
      hint: exempted ? "保留豁免说明，并按移除计划拆分文件。" : "拆分文件或记录经过审核且四段完整的硬耦合豁免说明。",
      message_id: "a0.size.file_too_long",
      ...(details && details.length > 0 ? { details } : {}),
    });
  }
  return { passed: !diagnostics.some((item) => item.severity === "error"), diagnostics };
}

/** 创建按语言路由的真实大纲提供器。 */
export function createOutlineProvider(root: string): OutlineProvider {
  return async (file: string): Promise<OutlineNodeLike[]> => {
    const lower = file.toLowerCase();
    if (lower.endsWith(".rs")) {
      const result = scanRustFiles({ root, files: [file], outline: true });
      if (result.diagnostics.length > 0) throw new Error(result.diagnostics.map((item) => item.message).join("；"));
      const outline = result.outlines[0];
      if (!outline) throw new Error("Rust AST 适配器未返回所请求文件的大纲。");
      return outline.nodes;
    }
    if (lower.endsWith(".ts") || lower.endsWith(".tsx")) {
      const adapter = await import("../../doc-coverage/src/typescript-adapter.ts");
      return adapter.outlineTypeScriptFile(file);
    }
    throw new Error(`不支持为该扩展名生成结构大纲：${file}`);
  };
}

/** 校验旁置豁免说明的四个必需章节。 */
export function exemptionMissingSections(content: string): string[] {
  const required = ["边界", "理由", "替代方案", "移除计划"];
  const sections = new Map<string, string>();
  const matches = [...content.matchAll(/^\s*#{1,6}\s*(边界|理由|替代方案|移除计划)\s*[:：]?\s*$/gmu)];
  for (let index = 0; index < matches.length; index += 1) {
    const match = matches[index];
    const next = matches[index + 1];
    const bodyStart = (match.index ?? 0) + match[0].length;
    sections.set(match[1], content.slice(bodyStart, next?.index ?? content.length).trim());
  }
  return required.filter((name) => !(sections.get(name)?.trim()));
}

/** 读取超长源文件旁置的硬耦合豁免说明。 */
function readExemption(absolute: string): { exists: boolean; missing: string[] } {
  const pathValue = join(dirname(absolute), `${basename(absolute)}的硬耦合需要的说明.md`);
  if (!existsSync(pathValue)) return { exists: false, missing: [] };
  try {
    return { exists: true, missing: exemptionMissingSections(readFileSync(pathValue, "utf8")) };
  } catch {
    return { exists: true, missing: ["边界", "理由", "替代方案", "移除计划"] };
  }
}

/** 创建大纲不可用诊断；它不会改变尺寸规则的 error 级别。 */
export function outlineUnavailableDiagnostic(file: string, reason: string): Diagnostic {
  const request = JSON.stringify({ protocol_version: 2, files: [file], outline: true });
  return {
    code: "A0-PARSER-001",
    severity: "error",
    path: file,
    subject: "outline",
    message: `超长文件结构大纲不可用：${reason}`,
    hint: `手工复现请求 ${request}；可设置 XIAO_RUST_DOC_ADAPTER=<预编译二进制>，或先运行 cargo build -p xiao-doc-coverage-rust。`,
    message_id: "a0.parser.outline_unavailable",
  };
}

/** 将树形大纲转换为稳定的文本行，并按节点数量做结构化截断。 */
export function renderOutlineDetails(nodes: OutlineNodeLike[], maxNodes = 400): string[] {
  const total = countNodes(nodes);
  const descendants = nodes.flatMap((node, index) => collectDescendants(node, [index]));
  // 顶层节点是拆分锚点，始终保留。选中深层节点时把祖先一并计入预算，
  // 才能既保持树形上下文，也不会让深层节点悄悄突破报告上限。
  const selected = new Set(nodes.map((_, index) => String(index)));
  const limit = Math.max(selected.size, Math.floor(maxNodes));
  for (const item of descendants.sort((left, right) => right.node.lines - left.node.lines || comparePath(left.path, right.path))) {
    const required = outlinePathPrefixes(item.path).filter((key) => !selected.has(key));
    if (selected.size + required.length > limit) continue;
    required.forEach((key) => selected.add(key));
  }
  const details: string[] = [];
  const emit = (node: OutlineNodeLike, path: number[], depth: number): void => {
    const key = path.join(".");
    if (!selected.has(key)) return;
    const signature = node.signature || node.source_line;
    details.push(`${"  ".repeat(depth)}${node.kind} ${node.name} [第 ${node.line}-${node.end_line} 行，共 ${node.lines} 行] ${signature}`.trimEnd());
    node.children.forEach((child, index) => emit(child, [...path, index], depth + 1));
  };
  nodes.forEach((node, index) => emit(node, [index], 0));
  const omitted = total - selected.size;
  if (omitted > 0) details.push(`……省略 ${omitted} 个更小的节点`);
  return details;
}

/** 从顶层节点中选择最大的拆分锚点，保持主消息为单行摘要。 */
function largestTopLevelSummary(nodes: OutlineNodeLike[]): string | undefined {
  const largest = [...nodes].sort((left, right) => right.lines - left.lines || left.line - right.line)[0];
  return largest ? ` 最大顶层节点 ${largest.kind} ${largest.name} 起于第 ${largest.line} 行，占 ${largest.lines} 行。` : undefined;
}

/** 统计一棵大纲树的节点数。 */
function countNodes(nodes: OutlineNodeLike[]): number {
  return nodes.reduce((total, node) => total + 1 + countNodes(node.children), 0);
}

/** 收集非顶层节点和稳定路径。 */
function collectDescendants(node: OutlineNodeLike, parentPath: number[]): Array<{ node: OutlineNodeLike; path: number[] }> {
  return node.children.flatMap((child, index) => [{ node: child, path: [...parentPath, index] }, ...collectDescendants(child, [...parentPath, index])]);
}

/** 返回一个路径本身及其非顶层祖先的稳定键。 */
function outlinePathPrefixes(path: number[]): string[] {
  const prefixes: string[] = [];
  for (let length = 2; length <= path.length; length += 1) prefixes.push(path.slice(0, length).join("."));
  return prefixes;
}

/** 以稳定字典序比较节点路径。 */
function comparePath(left: number[], right: number[]): number {
  for (let index = 0; index < Math.min(left.length, right.length); index += 1) {
    if (left[index] !== right[index]) return left[index] - right[index];
  }
  return left.length - right.length;
}
