/** DevDocs、UseDocs、模块登记和 Markdown 链接检查。 */

import { existsSync, readdirSync } from "node:fs";
import { basename, dirname, extname, join } from "node:path";

import {
  isDirectory,
  isFile,
  readText,
  repoRelative,
  resolveRepoPath,
} from "./paths.ts";
import type { Diagnostic, ModuleRecord, ModuleRegistry } from "./types.ts";

/**
 * 已解析的 Markdown 页面。
 */
export interface MarkdownPage {
  /** 页面仓库相对路径。 */
  path: string;
  /** 页面中的 YAML front matter 字段。 */
  frontMatter: Record<string, string | string[]>;
  /** 页面标题锚点。 */
  headings: Set<string>;
  /** 页面正文引用的相对链接目标。 */
  links: string[];
  /**
   * front matter `related` 声明的相对引用。
   *
   * 这些条目参与断链与锚点校验，但不计入入链统计：它们在渲染后不产生可点击导航，
   * 把它算作入链会掩盖真正的孤立页面。
   */
  related: string[];
}

/**
 * 扫描一个文档目录中的 Markdown 页面。
 *
 * @param root 仓库根目录。
 * @param directory 文档目录相对路径。
 * @returns 页面列表和扫描诊断。
 */
export function scanMarkdownDirectory(root: string, directory: string): { pages: MarkdownPage[]; diagnostics: Diagnostic[] } {
  const pages: MarkdownPage[] = [];
  const diagnostics: Diagnostic[] = [];
  let absolute: string;
  try {
    absolute = resolveRepoPath(root, directory);
  } catch (error) {
    return { pages, diagnostics: [docsDiagnostic("A0-DOCS-001", directory, String(error), "a0.docs.invalid_root")] };
  }
  if (!isDirectory(absolute)) {
    return { pages, diagnostics: [docsDiagnostic("A0-DOCS-001", directory, "文档目录不存在。", "a0.docs.missing_root")] };
  }
  const visit = (current: string): void => {
    let entries;
    try {
      entries = readdirSync(current, { withFileTypes: true });
    } catch (error) {
      diagnostics.push(docsDiagnostic("A0-DOCS-001", repoRelative(root, current), `无法读取文档目录：${String(error)}`, "a0.docs.read_failed"));
      return;
    }
    for (const entry of entries) {
      if (entry.name === ".git" || entry.name === "node_modules") continue;
      const child = join(current, entry.name);
      if (entry.isDirectory()) {
        visit(child);
      } else if (entry.isFile() && extname(entry.name).toLowerCase() === ".md") {
        const pathValue = repoRelative(root, child);
        try {
          pages.push(parseMarkdownPage(pathValue, readText(child)));
        } catch (error) {
          diagnostics.push(docsDiagnostic("A0-DOCS-001", pathValue, `Markdown 无法读取：${String(error)}`, "a0.docs.page_read_failed"));
        }
      }
    }
  };
  visit(absolute);
  return { pages, diagnostics };
}

/**
 * 校验 DevDocs 和 UseDocs 页面中的相对链接。
 *
 * @param root 仓库根目录。
 * @param pages 已扫描的页面。
 * @returns 断链、坏锚点和孤立页面诊断。
 */
export function checkMarkdownLinks(root: string, pages: MarkdownPage[]): Diagnostic[] {
  const diagnostics: Diagnostic[] = [];
  const byPath = new Map(pages.map((page) => [page.path, page]));
  const inbound = new Map<string, number>();
  for (const page of pages) inbound.set(page.path, 0);
  for (const page of pages) {
    for (const rawLink of page.links) {
      checkReference(root, page, rawLink, byPath, inbound, true, diagnostics);
    }
    for (const rawLink of page.related) {
      checkReference(root, page, rawLink, byPath, inbound, false, diagnostics);
    }
  }
  for (const page of pages) {
    if (basename(page.path).toLowerCase() === "readme.md") continue;
    if ((inbound.get(page.path) ?? 0) === 0) {
      diagnostics.push(docsDiagnostic("A0-DOCS-001", page.path, "页面没有入链，无法从主题索引到达。", "a0.docs.orphan_page"));
    }
  }
  return diagnostics;
}

/**
 * 校验 UseDocs 的 front matter、链接图和模块状态。
 *
 * @param root 仓库根目录。
 * @param registry 模块登记表。
 * @returns UseDocs 相关诊断。
 */
export function checkUseDocs(root: string, registry: ModuleRegistry): Diagnostic[] {
  const scanned = scanMarkdownDirectory(root, "docs/UseDocs");
  const diagnostics = [...scanned.diagnostics, ...checkMarkdownLinks(root, scanned.pages)];
  const pageByPath = new Map(scanned.pages.map((page) => [page.path, page]));
  for (const page of scanned.pages) {
    if (basename(page.path).toLowerCase() === "readme.md") continue;
    const hasFrontMatter = Object.keys(page.frontMatter).length > 0;
    if (!hasFrontMatter) {
      diagnostics.push(docsDiagnostic("A0-DOCS-001", page.path, "具体 UseDocs 页面必须包含 YAML front matter。", "a0.usedocs.front_matter_missing"));
      continue;
    }
    for (const field of ["id", "title", "status", "audience", "module", "stage"]) {
      if (!page.frontMatter[field]) diagnostics.push(docsDiagnostic("A0-DOCS-001", page.path, `UseDocs 缺少元数据字段：${field}`, "a0.usedocs.front_matter_field"));
    }
    const status = page.frontMatter.status;
    if (typeof status === "string" && !["planned", "draft", "verified", "deprecated"].includes(status)) {
      diagnostics.push(docsDiagnostic("A0-DOCS-001", page.path, `UseDocs 状态非法：${status}`, "a0.usedocs.status_invalid"));
    }
  }
  for (const module of registry.modules) {
    checkModuleRecord(root, module, pageByPath, diagnostics);
  }
  checkSpecFixtureExecution(root, registry, diagnostics);
  return diagnostics;
}

/**
 * 校验 DevDocs 目录的 Markdown 链接。
 *
 * @param root 仓库根目录。
 * @returns DevDocs 断链和孤立页诊断。
 */
export function checkDevDocs(root: string): Diagnostic[] {
  const scanned = scanMarkdownDirectory(root, "docs/DevDocs");
  return [...scanned.diagnostics, ...checkMarkdownLinks(root, scanned.pages)];
}

/** 解析 Markdown 页面中的元数据、标题和链接。 */
function parseMarkdownPage(pathValue: string, content: string): MarkdownPage {
  const frontMatter = parseFrontMatter(content);
  const body = stripFencedCode(content);
  const headings = new Set<string>();
  for (const match of body.matchAll(/^\s*#{1,6}\s+(.+?)\s*#*\s*$/gmu)) {
    headings.add(normalizeAnchor(match[1]));
  }
  const links: string[] = [];
  for (const match of body.matchAll(/!?\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)/gu)) links.push(match[1]);
  const related = Array.isArray(frontMatter.related) ? frontMatter.related : [];
  return { path: pathValue, frontMatter, headings, links, related };
}

/** 解析 A0 所需的有限 YAML front matter 子集。 */
function parseFrontMatter(content: string): Record<string, string | string[]> {
  const lines = content.split(/\r?\n/u);
  if (lines[0]?.trim() !== "---") return {};
  const end = lines.findIndex((line, index) => index > 0 && line.trim() === "---");
  if (end < 0) return {};
  const result: Record<string, string | string[]> = {};
  let currentArray: string[] | undefined;
  let currentKey = "";
  for (const line of lines.slice(1, end)) {
    const item = line.match(/^\s*-\s*(.+?)\s*$/u);
    if (item && currentKey) {
      currentArray ??= [];
      currentArray.push(unquote(item[1]));
      result[currentKey] = currentArray;
      continue;
    }
    const pair = line.match(/^\s*([A-Za-z0-9_.-]+)\s*:\s*(.*?)\s*$/u);
    if (!pair) continue;
    currentKey = pair[1];
    currentArray = undefined;
    const value = pair[2];
    if (value === "") {
      currentArray = [];
      result[currentKey] = currentArray;
    } else if (value.startsWith("[") && value.endsWith("]")) {
      result[currentKey] = value.slice(1, -1).split(",").map((item) => unquote(item.trim())).filter(Boolean);
    } else {
      result[currentKey] = unquote(value);
    }
  }
  return result;
}

/** 从链接扫描输入中移除围栏代码块。 */
function stripFencedCode(content: string): string {
  return content.replace(/^\s*```[\s\S]*?^\s*```\s*$/gmu, "");
}

/**
 * 校验页面声明的一条相对引用。
 *
 * 正文链接和 front matter `related` 共用同一套解析与校验规则，只有入链统计不同：
 * `countInbound` 为假时只报告断链和坏锚点，不把目标登记为「有入链」。
 *
 * @param root 仓库根目录。
 * @param page 声明该引用的页面。
 * @param rawLink 原始引用文本，可带锚点。
 * @param byPath 已扫描页面索引，用于解析锚点。
 * @param inbound 入链计数表。
 * @param countInbound 是否把解析成功的目标计入入链。
 * @param diagnostics 诊断输出列表。
 */
function checkReference(
  root: string,
  page: MarkdownPage,
  rawLink: string,
  byPath: Map<string, MarkdownPage>,
  inbound: Map<string, number>,
  countInbound: boolean,
  diagnostics: Diagnostic[],
): void {
  if (isExternalLink(rawLink) || rawLink.startsWith("mailto:")) return;
  const [rawTarget, rawAnchor] = splitAnchor(rawLink);
  if (!rawTarget) {
    const anchor = normalizeAnchor(rawAnchor);
    if (!page.headings.has(anchor)) {
      diagnostics.push(docsDiagnostic("A0-DOCS-001", page.path, `页面锚点不存在：#${rawAnchor}`, "a0.docs.anchor_missing"));
    }
    return;
  }
  let targetPath: string;
  try {
    targetPath = resolveMarkdownTarget(root, page.path, decodeURIComponent(rawTarget));
  } catch (error) {
    diagnostics.push(docsDiagnostic("A0-DOCS-001", page.path, `链接路径非法：${rawLink}（${String(error)}）`, "a0.docs.link_invalid"));
    return;
  }
  const targetFile = isDirectory(targetPath) ? join(targetPath, "README.md") : targetPath;
  if (!isFile(targetFile)) {
    diagnostics.push(docsDiagnostic("A0-DOCS-001", page.path, `链接目标不存在：${rawLink}`, "a0.docs.link_missing"));
    return;
  }
  const targetPagePath = repoRelative(root, targetFile);
  if (countInbound) inbound.set(targetPagePath, (inbound.get(targetPagePath) ?? 0) + 1);
  if (rawAnchor) {
    const targetPage = byPath.get(targetPagePath);
    if (targetPage && !targetPage.headings.has(normalizeAnchor(rawAnchor))) {
      diagnostics.push(docsDiagnostic("A0-DOCS-001", page.path, `链接锚点不存在：${rawLink}`, "a0.docs.anchor_missing"));
    }
  }
}

/** 安全解析 Markdown 相对链接目标。 */
function resolveMarkdownTarget(root: string, sourcePath: string, target: string): string {
  const sourceAbsolute = resolveRepoPath(root, sourcePath);
  const sourceDirectory = dirname(sourceAbsolute);
  const candidate = resolveRepoPath(root, join(repoRelative(root, sourceDirectory), target));
  if (isDirectory(candidate)) return join(candidate, "README.md");
  return candidate;
}

/** 分离链接路径和锚点部分。 */
function splitAnchor(value: string): [string, string] {
  const index = value.indexOf("#");
  return index < 0 ? [value, ""] : [value.slice(0, index), value.slice(index + 1)];
}

/** 按 GitHub 风格生成标题锚点。 */
function normalizeAnchor(value: string): string {
  return value.trim().toLowerCase().replace(/[^\p{Letter}\p{Number}\s_-]/gu, "").replace(/[\s_]+/gu, "-");
}

/** 判断链接是否属于外部协议。 */
function isExternalLink(value: string): boolean {
  return /^(?:[a-z][a-z0-9+.-]*:|\/\/)/iu.test(value);
}

/** 去除 front matter 标量两侧引号。 */
function unquote(value: string): string {
  return value.replace(/^['"]|['"]$/g, "");
}

/** 校验单个模块登记项的路径和交付状态。 */
function checkModuleRecord(root: string, module: ModuleRecord, pages: Map<string, MarkdownPage>, diagnostics: Diagnostic[]): void {
  for (const pathValue of [...module.code, ...module.tests, ...module.usedocs]) {
    let absolute: string;
    try {
      absolute = resolveRepoPath(root, pathValue);
    } catch (error) {
      diagnostics.push(docsDiagnostic("A0-DOCS-001", pathValue, `模块登记路径非法：${String(error)}`, "a0.registry.path_invalid"));
      continue;
    }
    if (!existsSync(absolute)) diagnostics.push(docsDiagnostic("A0-DOCS-001", pathValue, `模块登记路径不存在：${pathValue}`, "a0.registry.path_missing"));
  }
  if (module.status !== "verified") return;
  const verifiedPage = module.usedocs.some((pathValue) => {
    const normalized = pathValue.replaceAll("\\", "/");
    const direct = pages.get(normalized);
    if (direct) return direct.frontMatter.status === "verified";
    for (const [pagePath, page] of pages) {
      if (pagePath.startsWith(`${normalized}/`) && page.frontMatter.status === "verified") return true;
    }
    return false;
  });
  if (!verifiedPage) diagnostics.push(docsDiagnostic("A0-DOCS-002", module.id, "已完成模块没有 verified UseDocs 页面。", "a0.registry.verified_missing"));
}

/**
 * 校验每个 `tests/spec` 子目录都同时具备登记项和真实加载者。
 *
 * 仅检查路径存在会让「有夹具、无执行入口」的债项长期存活；这里复用模块登记的
 * 测试路径扫描源文件，并要求源文件正文出现对应的规范目录标记。它不要求某种
 * Rust/TypeScript 测试框架，也不把 README 或 JSON 本身误认为执行入口。
 */
function checkSpecFixtureExecution(root: string, registry: ModuleRegistry, diagnostics: Diagnostic[]): void {
  let specRoot: string;
  try {
    specRoot = resolveRepoPath(root, "tests/spec");
  } catch (error) {
    diagnostics.push(docsDiagnostic("A0-DOCS-003", "tests/spec", `规格目录路径非法：${String(error)}`, "a0.spec.invalid_root"));
    return;
  }
  if (!isDirectory(specRoot)) return;

  const sources = new Map<string, string>();
  const visit = (pathValue: string): void => {
    for (const entry of readdirSync(pathValue, { withFileTypes: true })) {
      const child = join(pathValue, entry.name);
      if (entry.isDirectory()) {
        visit(child);
        continue;
      }
      if (!entry.isFile() || ![".rs", ".ts", ".tsx", ".js", ".jsx"].includes(extname(entry.name).toLowerCase())) continue;
      try {
        sources.set(repoRelative(root, child), readText(child));
      } catch {
        // 路径存在性已由模块登记检查负责；不可读源码不构成有效执行入口。
      }
    }
  };

  for (const module of registry.modules) {
    for (const testPath of module.tests) {
      let absolute: string;
      try {
        absolute = resolveRepoPath(root, testPath);
      } catch {
        continue;
      }
      if (isDirectory(absolute)) visit(absolute);
      else if (isFile(absolute) && [".rs", ".ts", ".tsx", ".js", ".jsx"].includes(extname(absolute).toLowerCase())) {
        try {
          sources.set(repoRelative(root, absolute), readText(absolute));
        } catch {
          // 不可读源码不构成有效执行入口。
        }
      }
    }
  }

  for (const entry of readdirSync(specRoot, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const specPath = `tests/spec/${entry.name}`;
    const registered = registry.modules.some((module) => module.tests.some((testPath) => {
      const normalized = testPath.replaceAll("\\", "/").replace(/\/$/u, "");
      return normalized === specPath || normalized.startsWith(`${specPath}/`);
    }));
    if (!registered) {
      diagnostics.push(docsDiagnostic("A0-DOCS-003", specPath, "规格目录没有在 module-registry.json 的 tests 数组中登记。", "a0.spec.registry_missing"));
      continue;
    }
    const marker = `${specPath}/`;
    const loaders = [...sources.entries()].filter(([, content]) => content.includes(marker));
    if (loaders.length === 0) {
      diagnostics.push(docsDiagnostic("A0-DOCS-003", specPath, "规格目录已登记，但没有测试源码加载其中的夹具。", "a0.spec.loader_missing"));
    }
  }
}

/** 创建文档检查器统一诊断。 */
function docsDiagnostic(code: string, pathValue: string, message: string, messageId: string): Diagnostic {
  return {
    code,
    severity: "error",
    path: pathValue.replaceAll("\\", "/"),
    subject: pathValue,
    message,
    hint: "更新文档索引、页面元数据或模块登记后重新运行检查。",
    message_id: messageId,
  };
}
