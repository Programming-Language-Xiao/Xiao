/** 仓库路径安全、遍历和 README 最小结构工具。 */

import { existsSync, lstatSync, readdirSync, readFileSync, realpathSync } from "node:fs";
import { isAbsolute, join, relative, resolve, sep } from "node:path";

import type { Diagnostic } from "./types.ts";

/**
 * 将宿主平台路径转换为仓库清单使用的正斜杠形式。
 *
 * @param value 待规范化的路径。
 * @returns 使用 `/` 分隔且不带首尾斜杠的路径。
 */
export function slashPath(value: string): string {
  return value.replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/$/, "");
}

/**
 * 判断一个绝对路径是否位于指定根目录中。
 *
 * @param root 根目录绝对路径。
 * @param candidate 待判断的绝对路径。
 * @returns 路径等于根目录或位于其子树时返回 `true`。
 */
export function isInside(root: string, candidate: string): boolean {
  const rootReal = resolve(root);
  const candidateResolved = resolve(candidate);
  const rest = relative(rootReal, candidateResolved);
  return rest === "" || (!rest.startsWith(`..${sep}`) && rest !== "..");
}

/**
 * 将清单中的仓库相对路径安全地解析为绝对路径。
 *
 * @param root 仓库根目录。
 * @param value 仓库相对路径。
 * @returns 解析后的绝对路径；路径非法时抛出错误。
 */
export function resolveRepoPath(root: string, value: string): string {
  if (!value || isAbsolute(value)) {
    throw new Error(`路径必须是非空相对路径: ${value}`);
  }
  const candidate = resolve(root, value);
  if (!isInside(root, candidate)) {
    throw new Error(`路径越出仓库根目录: ${value}`);
  }
  return candidate;
}

/**
 * 将绝对路径转换为稳定的仓库相对路径。
 *
 * @param root 仓库根目录。
 * @param value 绝对路径。
 * @returns 正斜杠分隔的相对路径。
 */
export function repoRelative(root: string, value: string): string {
  return slashPath(relative(resolve(root), resolve(value)));
}

/**
 * 判断路径是否存在且为目录。
 *
 * @param pathValue 待检查路径。
 * @returns 路径存在并为目录时返回 `true`。
 */
export function isDirectory(pathValue: string): boolean {
  try {
    return lstatSync(pathValue).isDirectory();
  } catch {
    return false;
  }
}

/**
 * 判断路径是否存在且为普通文件。
 *
 * @param pathValue 待检查路径。
 * @returns 路径存在并为普通文件时返回 `true`。
 */
export function isFile(pathValue: string): boolean {
  try {
    return lstatSync(pathValue).isFile();
  } catch {
    return false;
  }
}

/**
 * 读取 UTF-8 文本文件。
 *
 * @param pathValue 文件绝对路径。
 * @returns 文件文本；读取失败时抛出原始错误。
 */
export function readText(pathValue: string): string {
  return readFileSync(pathValue, "utf8");
}

/**
 * 读取 JSON 文件并保留解析错误供上层归类。
 *
 * @param pathValue JSON 文件绝对路径。
 * @returns 解析后的未知值。
 */
export function readJson(pathValue: string): unknown {
  return JSON.parse(readText(pathValue));
}

/**
 * 递归发现包含项目源文件的目录，并报告无法安全遍历的路径。
 *
 * @param root 仓库根目录。
 * @param roots 待扫描的仓库相对根目录。
 * @param extensions 允许的源文件扩展名。
 * @param excludedNames 要跳过的目录名称集合。
 * @returns 源文件目录、源文件相对路径集合和遍历诊断。
 */
export function discoverSourceDirectories(
  root: string,
  roots: string[],
  extensions: string[],
  excludedNames: Set<string>,
): { directories: Set<string>; files: string[]; diagnostics: Diagnostic[] } {
  const directories = new Set<string>();
  const files: string[] = [];
  const diagnostics: Diagnostic[] = [];
  const extensionSet = new Set(extensions.map((item) => item.toLowerCase()));
  const visited = new Set<string>();

  const visit = (directory: string): void => {
    let realDirectory: string;
    try {
      realDirectory = realpathSync(directory);
    } catch (error) {
      diagnostics.push({
        code: "A0-LAYOUT-004",
        severity: "error",
        path: repoRelative(root, directory),
        subject: directory,
        message: `无法读取目录：${String(error)}`,
        hint: "确认目录存在且当前用户拥有读取权限。",
        message_id: "a0.layout.unreadable_directory",
      });
      return;
    }
    if (!isInside(root, realDirectory)) {
      diagnostics.push({
        code: "A0-LAYOUT-002",
        severity: "error",
        path: repoRelative(root, directory),
        subject: directory,
        message: "符号链接目标越出仓库根目录。",
        hint: "移除越界链接，或把实际代码放回仓库内。",
        message_id: "a0.layout.symlink_escape",
      });
      return;
    }
    if (visited.has(realDirectory)) return;
    visited.add(realDirectory);

    let entries;
    try {
      entries = readdirSync(directory, { withFileTypes: true });
    } catch (error) {
      diagnostics.push({
        code: "A0-LAYOUT-004",
        severity: "error",
        path: repoRelative(root, directory),
        subject: directory,
        message: `无法枚举目录：${String(error)}`,
        hint: "确认目录权限和文件系统状态。",
        message_id: "a0.layout.enumeration_failed",
      });
      return;
    }

    let hasSource = false;
    for (const entry of entries) {
      const child = join(directory, entry.name);
      if (entry.isSymbolicLink()) {
        let target: string;
        try {
          target = realpathSync(child);
        } catch (error) {
          diagnostics.push({
            code: "A0-LAYOUT-003",
            severity: "error",
            path: repoRelative(root, child),
            subject: child,
            message: `符号链接无法解析：${String(error)}`,
            hint: "删除悬空链接或修复其目标。",
            message_id: "a0.layout.broken_symlink",
          });
          continue;
        }
        if (!isInside(root, target)) {
          diagnostics.push({
            code: "A0-LAYOUT-002",
            severity: "error",
            path: repoRelative(root, child),
            subject: child,
            message: "符号链接目标越出仓库根目录。",
            hint: "代码目录不得通过链接逃逸仓库。",
            message_id: "a0.layout.symlink_escape",
          });
        }
        continue;
      }
      if (entry.isDirectory()) {
        if (excludedNames.has(entry.name)) continue;
        visit(child);
        continue;
      }
      if (!entry.isFile()) continue;
      const extension = entry.name.slice(entry.name.lastIndexOf(".")).toLowerCase();
      if (extensionSet.has(extension)) {
        hasSource = true;
        files.push(repoRelative(root, child));
      }
    }
    if (hasSource) directories.add(repoRelative(root, directory));
  };

  for (const rootPath of roots) {
    try {
      visit(resolveRepoPath(root, rootPath));
    } catch (error) {
      diagnostics.push({
        code: "A0-MANIFEST-002",
        severity: "error",
        path: rootPath,
        subject: rootPath,
        message: String(error),
        hint: "把 codeRoots 改为仓库内存在的相对目录。",
        message_id: "a0.manifest.invalid_path",
      });
    }
  }
  files.sort();
  return { directories, files, diagnostics };
}

/**
 * 校验一个目录 README 是否具备最小交接信息。
 *
 * @param content README 文本。
 * @returns 缺失信息的可读标签；空数组表示通过。
 */
export function readmeMissingSections(content: string): string[] {
  const missing: string[] = [];
  if (!content.trim() || !/^\s*#\s+\S+/m.test(content)) missing.push("标题");
  if (!/(工程期|阶段|里程碑|对应工程)/u.test(content)) missing.push("工程期");
  if (!/(职责|内容|放置|存放|提供|实现|模块|规则|禁止|边界)/u.test(content)) missing.push("职责或边界");
  return missing;
}

/**
 * 检查路径是否为符号链接。
 *
 * @param pathValue 待检查路径。
 * @returns 路径存在且是符号链接时返回 `true`。
 */
export function isSymlink(pathValue: string): boolean {
  try {
    return lstatSync(pathValue).isSymbolicLink();
  } catch {
    return false;
  }
}
