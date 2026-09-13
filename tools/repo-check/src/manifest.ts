/** A0 仓库政策清单和模块登记表加载器。 */

import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";

import {
  isDirectory,
  isFile,
  readJson,
  resolveRepoPath,
  slashPath,
} from "./paths.ts";
import type {
  Diagnostic,
  LoadedRepository,
  ModuleRecord,
  ModuleRegistry,
  RepositoryManifest,
  WorkspacePolicy,
} from "./types.ts";

/**
 * 从给定目录向上查找包含 `.git` 的仓库根目录。
 *
 * @param start 起始目录或文件路径。
 * @returns 仓库根目录绝对路径；未找到时返回 `undefined`。
 */
export function findRepositoryRoot(start: string): string | undefined {
  let current = resolve(start);
  if (!isDirectory(current)) current = dirname(current);
  while (true) {
    if (existsSync(resolve(current, ".git"))) return current;
    const parent = dirname(current);
    if (parent === current) return undefined;
    current = parent;
  }
}

/**
 * 载入并校验仓库政策清单。
 *
 * @param root 仓库根目录。
 * @param manifestPath 清单相对路径，默认使用 A0 固定位置。
 * @returns 加载结果和诊断；格式错误时结果为空。
 */
export function loadRepositoryManifest(
  root: string,
  manifestPath = "tools/repo-check/repository.manifest.json",
): { manifest?: RepositoryManifest; diagnostics: Diagnostic[] } {
  const diagnostics: Diagnostic[] = [];
  let pathValue: string;
  try {
    pathValue = resolveRepoPath(root, manifestPath);
  } catch (error) {
    diagnostics.push(manifestError(manifestPath, String(error), "a0.manifest.invalid_path"));
    return { diagnostics };
  }
  if (!isFile(pathValue)) {
    diagnostics.push(manifestError(manifestPath, "政策清单不存在。", "a0.manifest.missing"));
    return { diagnostics };
  }
  let value: unknown;
  try {
    value = readJson(pathValue);
  } catch (error) {
    diagnostics.push(manifestError(manifestPath, `JSON 无法解析：${String(error)}`, "a0.manifest.invalid_json"));
    return { diagnostics };
  }
  const manifest = parseRepositoryManifest(value);
  if (!manifest) {
    diagnostics.push(manifestError(manifestPath, "缺少必需字段或字段类型不正确。", "a0.manifest.invalid_shape"));
    return { diagnostics };
  }
  return { manifest, diagnostics };
}

/**
 * 载入模块登记表；登记表缺失或形状错误会返回稳定诊断。
 *
 * @param root 仓库根目录。
 * @param pathValue 登记表相对路径。
 * @returns 登记表和诊断。
 */
export function loadModuleRegistry(
  root: string,
  pathValue: string,
): { registry?: ModuleRegistry; diagnostics: Diagnostic[] } {
  const diagnostics: Diagnostic[] = [];
  let absolute: string;
  try {
    absolute = resolveRepoPath(root, pathValue);
  } catch (error) {
    diagnostics.push(manifestError(pathValue, String(error), "a0.docs.invalid_registry_path"));
    return { diagnostics };
  }
  if (!isFile(absolute)) {
    diagnostics.push(manifestError(pathValue, "模块登记表不存在。", "a0.docs.missing_registry"));
    return { diagnostics };
  }
  let value: unknown;
  try {
    value = readJson(absolute);
  } catch (error) {
    diagnostics.push(manifestError(pathValue, `JSON 无法解析：${String(error)}`, "a0.docs.invalid_registry_json"));
    return { diagnostics };
  }
  const registry = parseModuleRegistry(value);
  if (!registry) {
    diagnostics.push(manifestError(pathValue, "模块登记表字段不完整或类型错误。", "a0.docs.invalid_registry_shape"));
    return { diagnostics };
  }
  return { registry, diagnostics };
}

/**
 * 读取仓库根目录及其政策和模块登记。
 *
 * @param start 起始目录。
 * @returns 可供各检查器复用的仓库对象和诊断。
 */
export function loadRepository(start: string): { repository?: LoadedRepository; diagnostics: Diagnostic[] } {
  const diagnostics: Diagnostic[] = [];
  const root = findRepositoryRoot(start);
  if (!root) {
    diagnostics.push(manifestError("", "找不到包含 .git 的仓库根目录。", "a0.manifest.root_missing"));
    return { diagnostics };
  }
  const loadedManifest = loadRepositoryManifest(root);
  diagnostics.push(...loadedManifest.diagnostics);
  if (!loadedManifest.manifest) return { diagnostics };
  const loadedRegistry = loadModuleRegistry(root, loadedManifest.manifest.moduleRegistry);
  diagnostics.push(...loadedRegistry.diagnostics);
  return {
    repository: {
      root,
      manifest: loadedManifest.manifest,
      registry: loadedRegistry.registry,
    },
    diagnostics,
  };
}

/** 将 JSON 未知值验证为仓库政策清单。 */
function parseRepositoryManifest(value: unknown): RepositoryManifest | undefined {
  if (!isRecord(value) || value.schemaVersion !== 1) return undefined;
  const rust = parseWorkspacePolicy(value.rust);
  const typescript = parseWorkspacePolicy(value.typescript);
  if (!rust || !typescript) return undefined;
  if (!isUniqueStringArray(value.codeRoots) || value.codeRoots.length === 0
    || !isUniqueStringArray(value.sourceExtensions) || value.sourceExtensions.length === 0
    || !isUniqueStringArray(value.excludedDirectories)) return undefined;
  if (value.readmeFile !== "README.md" || typeof value.moduleRegistry !== "string") return undefined;
  return {
    schemaVersion: 1,
    rust,
    typescript,
    codeRoots: value.codeRoots,
    sourceExtensions: value.sourceExtensions,
    excludedDirectories: value.excludedDirectories,
    readmeFile: "README.md",
    moduleRegistry: value.moduleRegistry,
  };
}

/** 验证单个 workspace 政策对象。 */
function parseWorkspacePolicy(value: unknown): WorkspacePolicy | undefined {
  if (!isRecord(value) || typeof value.manifest !== "string" || value.manifest.length === 0 || !isUniqueStringArray(value.members) || value.members.length === 0) return undefined;
  return { manifest: value.manifest, members: value.members };
}

/** 将 JSON 未知值验证为模块登记表。 */
function parseModuleRegistry(value: unknown): ModuleRegistry | undefined {
  if (!isRecord(value) || value.schemaVersion !== 1 || !Array.isArray(value.modules)) return undefined;
  const modules: ModuleRecord[] = [];
  for (const item of value.modules) {
    if (!isRecord(item) || typeof item.id !== "string" || typeof item.stage !== "string" || !isStatus(item.status) || !isStringArray(item.code) || !isStringArray(item.tests) || !isStringArray(item.usedocs)) return undefined;
    if (item.replacement !== undefined && typeof item.replacement !== "string") return undefined;
    modules.push({
      id: item.id,
      stage: item.stage,
      status: item.status,
      code: item.code,
      tests: item.tests,
      usedocs: item.usedocs,
      ...(item.replacement ? { replacement: item.replacement } : {}),
    });
  }
  return { schemaVersion: 1, modules };
}

/** 判断值是否为非数组对象。 */
function isRecord(value: unknown): value is Record<string, any> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** 判断值是否为非空字符串数组。 */
function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string" && item.length > 0);
}

/** 判断值是否为不含重复项的非空字符串数组。 */
function isUniqueStringArray(value: unknown): value is string[] {
  return isStringArray(value) && new Set(value).size === value.length;
}

/** 判断模块状态是否属于已冻结枚举。 */
function isStatus(value: unknown): value is ModuleRecord["status"] {
  return value === "planned" || value === "draft" || value === "verified" || value === "deprecated";
}

/** 创建清单加载统一诊断。 */
function manifestError(pathValue: string, message: string, messageId: string): Diagnostic {
  return {
    code: "A0-MANIFEST-001",
    severity: "error",
    path: slashPath(pathValue),
    subject: pathValue || "repository",
    message,
    hint: "修复政策清单或仓库根路径后重新运行检查。",
    message_id: messageId,
  };
}
