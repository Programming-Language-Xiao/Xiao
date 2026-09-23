/** 项目测试文件发现与确定性排序；不执行源码。 */

import { readdir, stat } from "node:fs/promises";
import { relative, resolve, sep } from "node:path";

import { CLI_EXIT_CODES } from "../diagnostics/render.ts";

/** 一个待交给项目测试协议执行的源码文件。 */
export interface ProjectTestFile {
  /** 项目根下的正斜杠相对路径。 */
  relativePath: string;
  /** 文件的绝对路径。 */
  absolutePath: string;
}

/** 项目测试发现失败的结构化命令错误。 */
class TestDiscoveryError extends Error {
  /** 稳定诊断编号。 */
  readonly code: string;
  /** CLI usage 退出码。 */
  readonly exitCode = CLI_EXIT_CODES.usage;
  /** 供机器模式消费的路径和发现根。 */
  readonly details: Record<string, unknown>;

  /** 创建测试发现错误。 */
  constructor(code: string, message: string, details: Record<string, unknown>) {
    super(`${code}: ${message}`);
    this.name = "TestDiscoveryError";
    this.code = code;
    this.details = details;
  }
}

/**
 * 发现项目测试文件并按项目相对路径稳定排序。
 *
 * @param projectPath 项目目录；可为绝对路径或相对当前工作目录的路径。
 * @param cwd 相对项目路径的解析基准目录。
 * @returns 按正斜杠相对路径排序的测试文件。
 */
export async function discoverProjectTests(projectPath = ".", cwd = process.cwd()): Promise<ProjectTestFile[]> {
  const projectRoot = resolve(cwd, projectPath);
  await requireDirectory(projectRoot, "X11-CLI-TEST-002", "项目路径不是可读取的目录", { project: projectRoot });
  const testsRoot = resolve(projectRoot, "tests");
  await requireDirectory(testsRoot, "X11-CLI-TEST-003", "项目没有可读取的 tests 目录", { project: projectRoot, tests_root: testsRoot });

  const files: ProjectTestFile[] = [];
  await collectTestFiles(testsRoot, projectRoot, files);
  files.sort((left, right) => compareStablePath(left.relativePath, right.relativePath));
  if (files.length === 0) {
    throw new TestDiscoveryError("X11-CLI-TEST-003", "项目 tests 目录中没有 .xiao 测试文件", {
      project: projectRoot,
      tests_root: testsRoot,
      pattern: "tests/**/*.xiao",
    });
  }
  return files;
}

/** 将测试相对路径转换为稳定的协议模块名。 */
export function testModuleName(relativePath: string): string {
  return relativePath.replace(/\.xiao$/iu, "");
}

/** 递归收集普通 `.xiao` 文件；不跟随符号链接，避免扫描越出项目根。 */
async function collectTestFiles(directory: string, projectRoot: string, output: ProjectTestFile[]): Promise<void> {
  const entries = await readdir(directory, { withFileTypes: true });
  for (const entry of entries) {
    const absolutePath = resolve(directory, entry.name);
    if (entry.isSymbolicLink()) continue;
    if (entry.isDirectory()) {
      await collectTestFiles(absolutePath, projectRoot, output);
      continue;
    }
    if (!entry.isFile() || !entry.name.toLowerCase().endsWith(".xiao")) continue;
    output.push({
      relativePath: relative(projectRoot, absolutePath).split(sep).join("/"),
      absolutePath,
    });
  }
}

/** 要求路径存在且为目录，并把文件系统失败转换为稳定命令错误。 */
async function requireDirectory(
  path: string,
  code: string,
  message: string,
  details: Record<string, unknown>,
): Promise<void> {
  try {
    if (!(await stat(path)).isDirectory()) throw new Error("不是目录");
  } catch (error) {
    throw new TestDiscoveryError(code, `${message}：${String(error)}`, details);
  }
}

/** 使用宿主无关的 Unicode 码点顺序比较相对路径。 */
function compareStablePath(left: string, right: string): number {
  if (left === right) return 0;
  return left < right ? -1 : 1;
}
