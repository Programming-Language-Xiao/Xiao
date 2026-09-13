/** workspace 目录、源目录和 README 完整性规则。 */

import { existsSync } from "node:fs";
import { join } from "node:path";

import { loadRepository } from "./manifest.ts";
import {
  discoverSourceDirectories,
  isDirectory,
  isFile,
  readText,
  readmeMissingSections,
  repoRelative,
  resolveRepoPath,
} from "./paths.ts";
import { inspectWorkspaces } from "./workspace.ts";
import type { CheckResult, Diagnostic, LoadedRepository } from "./types.ts";

/**
 * 执行 A0 的 workspace、目录和 README 完整性检查。
 *
 * @param start 仓库根目录或其下任意目录。
 * @returns 统一检查结果。
 */
export function checkLayout(start: string): CheckResult {
  const loaded = loadRepository(start);
  const diagnostics = [...loaded.diagnostics];
  if (!loaded.repository) return finish(diagnostics);
  diagnostics.push(...checkLoadedLayout(loaded.repository));
  return finish(diagnostics);
}

/**
 * 在已加载的仓库对象上执行目录检查，供 `all` 命令复用。
 *
 * @param repository 已加载的仓库。
 * @returns workspace、路径和 README 诊断。
 */
export function checkLoadedLayout(repository: LoadedRepository): Diagnostic[] {
  const { root, manifest } = repository;
  const diagnostics: Diagnostic[] = [];
  diagnostics.push(...inspectWorkspaces(root, manifest));

  for (const codeRoot of manifest.codeRoots) {
    let absolute: string;
    try {
      absolute = resolveRepoPath(root, codeRoot);
    } catch (error) {
      diagnostics.push(layoutDiagnostic("A0-MANIFEST-002", codeRoot, String(error), "a0.layout.root_invalid"));
      continue;
    }
    if (!isDirectory(absolute)) {
      diagnostics.push(layoutDiagnostic("A0-MANIFEST-002", codeRoot, "代码根目录不存在。", "a0.layout.root_missing"));
    }
  }

  const discovered = discoverSourceDirectories(root, manifest.codeRoots, manifest.sourceExtensions, new Set(manifest.excludedDirectories));
  diagnostics.push(...discovered.diagnostics);
  const allPathKeys = new Map<string, string>();
  for (const directory of discovered.directories) {
    const key = directory.toLowerCase();
    const previous = allPathKeys.get(key);
    if (previous && previous !== directory) {
      diagnostics.push(layoutDiagnostic("A0-LAYOUT-002", directory, `路径大小写冲突：${previous} 与 ${directory}`, "a0.layout.case_collision"));
    } else {
      allPathKeys.set(key, directory);
    }
    checkDirectoryReadme(root, directory, manifest.readmeFile, diagnostics);
  }

  const members = [...manifest.rust.members, ...manifest.typescript.members];
  for (const member of members) {
    let memberPath: string;
    try {
      memberPath = resolveRepoPath(root, member);
    } catch (error) {
      diagnostics.push(layoutDiagnostic("A0-MANIFEST-002", member, String(error), "a0.layout.member_invalid"));
      continue;
    }
    if (!isDirectory(memberPath)) continue;
    checkDirectoryReadme(root, member, manifest.readmeFile, diagnostics);
  }
  checkSourceDirectoryRegistration(discovered.directories, repository.registry, diagnostics);
  return diagnostics;
}

/** 确认发现的源码目录能从模块登记表的 code/tests 路径追溯。 */
function checkSourceDirectoryRegistration(
  directories: Set<string>,
  registry: LoadedRepository["registry"],
  diagnostics: Diagnostic[],
): void {
  if (!registry) return;
  const registered = registry.modules.flatMap((module) => [...module.code, ...module.tests]).map(normalizeRelativePath);
  for (const directory of directories) {
    const normalized = normalizeRelativePath(directory);
    if (registered.some((item) => normalized === item || normalized.startsWith(`${item}/`))) continue;
    diagnostics.push(layoutDiagnostic(
      "A0-LAYOUT-002",
      directory,
      "发现的源码目录未在 module-registry.json 的 code/tests 路径中登记。",
      "a0.layout.source_unregistered",
    ));
  }
}

/** 统一清单相对路径的分隔符和首尾斜杠。 */
function normalizeRelativePath(value: string): string {
  return value.replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/$/, "");
}

/** 检查单个源目录的 README 和最小交接信息。 */
function checkDirectoryReadme(root: string, directory: string, readmeName: string, diagnostics: Diagnostic[]): void {
  let directoryPath: string;
  try {
    directoryPath = resolveRepoPath(root, directory);
  } catch (error) {
    diagnostics.push(layoutDiagnostic("A0-LAYOUT-001", directory, String(error), "a0.layout.directory_invalid"));
    return;
  }
  const readmePath = join(directoryPath, readmeName);
  if (!isFile(readmePath)) {
    diagnostics.push(layoutDiagnostic("A0-LAYOUT-001", directory, `代码目录缺少 ${readmeName}。`, "a0.layout.readme_missing"));
    return;
  }
  let missing: string[];
  try {
    missing = readmeMissingSections(readText(readmePath));
  } catch (error) {
    diagnostics.push(layoutDiagnostic("A0-LAYOUT-001", repoRelative(root, readmePath), `README 无法读取：${String(error)}`, "a0.layout.readme_unreadable"));
    return;
  }
  if (missing.length > 0) {
    diagnostics.push(layoutDiagnostic("A0-LAYOUT-001", repoRelative(root, readmePath), `README 缺少交接信息：${missing.join("、")}`, "a0.layout.readme_sections"));
  }
}

/** 创建目录检查器统一诊断。 */
function layoutDiagnostic(code: string, pathValue: string, message: string, messageId: string): Diagnostic {
  return {
    code,
    severity: "error",
    path: pathValue.replaceAll("\\", "/"),
    subject: pathValue,
    message,
    hint: "补齐目录 README、修复路径或同步 repository.manifest.json。",
    message_id: messageId,
  };
}

/** 根据诊断列表计算检查是否通过。 */
function finish(diagnostics: Diagnostic[]): CheckResult {
  return { passed: !diagnostics.some((item) => item.severity === "error"), diagnostics };
}
