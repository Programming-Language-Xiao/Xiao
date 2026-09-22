/** TypeScript CLI 对 Rust `xiao-core` 进程的主机接线。 */

import { access, constants, stat } from "node:fs/promises";
import { statSync } from "node:fs";
import { dirname, join, normalize, resolve } from "node:path";
import { execFileSync } from "node:child_process";

/** CLI 侧使用的目标条件；字段与 X0-A 协议保持一致。 */
export interface HostTarget {
  /** LLVM target triple。 */
  triple: string;
  /** 指针宽度。 */
  pointer_width: number;
  /** 字节序。 */
  endian: "little" | "big";
  /** 目标文件格式。 */
  object_format: "coff" | "elf" | "macho";
}

/** 核心发现选项。 */
export interface CoreDiscoveryOptions {
  /** 工作目录，用于开发树相对候选。 */
  cwd?: string;
  /** 环境变量；默认使用当前环境。 */
  env?: NodeJS.ProcessEnv;
  /** 显式覆盖核心路径。 */
  overridePath?: string;
}

/** 核心发现失败的稳定错误。 */
export class CoreDiscoveryError extends Error {
  /** 稳定诊断编号。 */
  readonly code = "X11-CLI-CORE-001";
  /** 已检查的候选路径。 */
  readonly candidates: readonly string[];

  /** 创建核心发现错误。 */
  constructor(message: string, candidates: readonly string[] = []) {
    super(`X11-CLI-CORE-001: ${message}`);
    this.name = "CoreDiscoveryError";
    this.candidates = candidates;
  }
}

/** 返回当前宿主的协议目标；只描述主机，不发现外部工具链。 */
export function hostTarget(platform = process.platform, architecture = process.arch): HostTarget {
  const pointer_width = architecture === "ia32" || architecture === "arm" ? 32 : 64;
  const arch = architecture === "arm64" ? "aarch64" : architecture === "arm" ? "armv7" : architecture === "ia32" ? "i686" : architecture === "x64" ? "x86_64" : null;
  if (arch === null) throw new CoreDiscoveryError(`不支持的 CPU 架构：${architecture}`);
  if (platform === "win32") {
    return { triple: `${arch}-pc-windows-msvc`, pointer_width, endian: "little", object_format: "coff" };
  }
  if (platform === "darwin") {
    return { triple: `${arch}-apple-darwin`, pointer_width, endian: "little", object_format: "macho" };
  }
  if (platform === "linux") {
    return { triple: `${arch}-unknown-linux-gnu`, pointer_width, endian: "little", object_format: "elf" };
  }
  throw new CoreDiscoveryError(`不支持的宿主平台：${platform}`);
}

/** 判断路径是否是可执行的常规文件。 */
export async function isExecutableFile(path: string): Promise<boolean> {
  try {
    const metadata = await stat(path);
    if (!metadata.isFile()) return false;
    if (process.platform === "win32") return true;
    await access(path, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/** 查找开发树或 PATH 中的 Rust 核心可执行文件。
 *
 * X0-B 只提供可测试的开发发现顺序；安装包和完整三平台搜索归 X0-C。
 */
export async function discoverCore(options: CoreDiscoveryOptions = {}): Promise<string> {
  const env = options.env ?? process.env;
  const cwd = resolve(options.cwd ?? process.cwd());
  const override = options.overridePath ?? env.XIAO_CORE_PATH;
  const candidates: string[] = [];
  if (override?.trim()) {
    const path = resolve(cwd, override);
    candidates.push(path);
    if (await isExecutableFile(path)) return path;
    throw new CoreDiscoveryError(`XIAO_CORE_PATH 指向的核心不可执行：${path}`, candidates);
  }

  const roots = unique([cwd, findRepositoryRoot(cwd), dirname(cwd)]);
  for (const root of roots) {
    for (const relative of developmentCandidates()) {
      const candidate = join(root, relative);
      candidates.push(candidate);
      if (await isExecutableFile(candidate)) return normalize(candidate);
    }
  }

  const pathCandidate = findOnPath(env.PATH, env);
  if (pathCandidate !== null) {
    candidates.push(pathCandidate);
    if (await isExecutableFile(pathCandidate)) return pathCandidate;
  }
  throw new CoreDiscoveryError("找不到 xiao-core；请构建核心或设置 XIAO_CORE_PATH", candidates);
}

/** 返回当前实现用于开发回环的候选相对路径。 */
export function developmentCandidates(platform = process.platform): readonly string[] {
  const suffix = platform === "win32" ? ".exe" : "";
  return [
    `core/rust/target/debug/xiao-core${suffix}`,
    `core/rust/target/release/xiao-core${suffix}`,
    `target/debug/xiao-core${suffix}`,
    `target/release/xiao-core${suffix}`,
    `xiao-core${suffix}`,
  ];
}

/** 从起点向上寻找同时包含 workspace 和 Rust workspace 的仓库根。 */
function findRepositoryRoot(start: string): string | null {
  let current = resolve(start);
  while (true) {
    if (existsSync(join(current, "Cargo.toml")) && existsSync(join(current, "core", "rust", "Cargo.toml"))) return current;
    const parent = dirname(current);
    if (parent === current) return null;
    current = parent;
  }
}

/** 使用同步 stat 做发现阶段的轻量文件存在判断。 */
function existsSync(path: string): boolean {
  try { return requireStat(path); } catch { return false; }
}

/** 返回同步 stat 的常规文件结果。 */
function requireStat(path: string): boolean {
  // `statSync` 只用于发现阶段的轻量根目录判断，异步可执行性检查仍走上面的 API。
  return statSync(path).isFile();
}

/** 使用宿主 PATH 命令或手工目录扫描寻找核心。 */
function findOnPath(pathValue: string | undefined, env: NodeJS.ProcessEnv): string | null {
  const command = process.platform === "win32" ? "where.exe" : "which";
  try {
    const output = execFileSync(command, ["xiao-core"], { env, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
    const first = output.split(/\r?\n/u).find((line) => line.trim().length > 0)?.trim();
    return first ?? null;
  } catch {
    // 某些嵌入式宿主没有 where/which；下面的手工扫描保持行为可测试。
    if (!pathValue) return null;
    const suffixes = process.platform === "win32" ? [".exe", ""] : [""];
    for (const directory of pathValue.split(process.platform === "win32" ? ";" : ":")) {
      for (const suffix of suffixes) {
        const candidate = join(directory, `xiao-core${suffix}`);
        try { if (requireStat(candidate)) return candidate; } catch { /* 继续扫描 */ }
      }
    }
    return null;
  }
}

/** 规范化并去重候选根目录。 */
function unique(values: readonly (string | null)[]): string[] {
  return [...new Set(values.filter((value): value is string => value !== null).map((value) => resolve(value)))];
}
