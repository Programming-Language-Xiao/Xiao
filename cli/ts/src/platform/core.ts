/** TypeScript CLI 对 Rust `xiao-core` 进程的主机接线。 */

import { access, constants, stat } from "node:fs/promises";
import { statSync } from "node:fs";
import { basename, dirname, join, normalize, resolve } from "node:path";
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

/** 核心发现候选的来源；来源是机器字段，不依赖本地化文案。 */
export type CoreDiscoverySource = "override" | "adjacent" | "path" | "development";

/** 一条核心发现候选及其布局来源。 */
export interface CoreDiscoveryCandidate {
  /** 规范化后的候选路径。 */
  path: string;
  /** 候选来源。 */
  source: CoreDiscoverySource;
}

/** 核心发现成功后的结构化结果。 */
export interface CoreDiscoveryResult {
  /** 实际选中的核心路径。 */
  path: string;
  /** 选中路径的来源。 */
  source: CoreDiscoverySource;
  /** 按尝试顺序保留的全部候选。 */
  candidates: readonly CoreDiscoveryCandidate[];
}

/** 核心发现选项。 */
export interface CoreDiscoveryOptions {
  /** 工作目录，用于开发树相对候选。 */
  cwd?: string;
  /** 环境变量；默认使用当前环境。 */
  env?: NodeJS.ProcessEnv;
  /** 显式覆盖核心路径。 */
  overridePath?: string;
  /** CLI 可执行文件路径；用于生产环境的同目录发现，测试可注入。 */
  executablePath?: string;
  /** 发现目标的平台；默认当前宿主平台。 */
  platform?: NodeJS.Platform;
  /** 发现目标的 CPU 架构；默认当前宿主架构。 */
  architecture?: string;
}

/** 核心发现失败的稳定错误。 */
export class CoreDiscoveryError extends Error {
  /** 稳定诊断编号。 */
  readonly code = "X11-CLI-CORE-001";
  /** 已检查的候选路径。 */
  readonly candidates: readonly string[];

  /** 带来源的候选清单，供机器诊断标记开发布局。 */
  readonly candidateDetails: readonly CoreDiscoveryCandidate[];

  /** 创建核心发现错误。 */
  constructor(message: string, candidates: readonly string[] = [], candidateDetails: readonly CoreDiscoveryCandidate[] = []) {
    super(`X11-CLI-CORE-001: ${message}`);
    this.name = "CoreDiscoveryError";
    this.candidates = candidates;
    this.candidateDetails = candidateDetails;
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
export async function isExecutableFile(path: string, platform = process.platform): Promise<boolean> {
  try {
    const metadata = await stat(path);
    if (!metadata.isFile()) return false;
    if (platform === "win32") return true;
    await access(path, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/** 按生产顺序查找相邻、PATH 或开发树中的 Rust 核心可执行文件。 */
export async function discoverCore(options: CoreDiscoveryOptions = {}): Promise<string> {
  return (await discoverCoreWithMetadata(options)).path;
}

/** 按生产顺序发现核心，并保留候选来源供诊断和打包器使用。 */
export async function discoverCoreWithMetadata(options: CoreDiscoveryOptions = {}): Promise<CoreDiscoveryResult> {
  const env = options.env ?? process.env;
  const cwd = resolve(options.cwd ?? process.cwd());
  const platform = options.platform ?? process.platform;
  const candidates: CoreDiscoveryCandidate[] = [];
  const addCandidate = (path: string, source: CoreDiscoverySource): CoreDiscoveryCandidate => {
    const candidate = { path: normalize(path), source };
    candidates.push(candidate);
    return candidate;
  };
  const fail = (message: string): never => {
    throw new CoreDiscoveryError(message, candidates.map((candidate) => candidate.path), candidates);
  };

  // 显式路径具有最高优先级；配置了错误路径时不静默回退到另一个核心。
  const override = options.overridePath ?? env.XIAO_CORE_PATH;
  if (override?.trim()) {
    const candidate = addCandidate(resolve(cwd, override), "override");
    if (await isExecutableFile(candidate.path, platform)) return { path: candidate.path, source: candidate.source, candidates };
    return fail(`XIAO_CORE_PATH 指向的核心不可执行：${candidate.path}`);
  }

  // 编译后的 xiao 与 xiao-core 同目录；源码模式下 Bun 自身不是安装目录。
  const executablePath = options.executablePath ?? runtimeExecutablePath();
  if (executablePath) {
    const candidate = addCandidate(join(dirname(resolve(executablePath)), coreExecutableName(platform)), "adjacent");
    if (await isExecutableFile(candidate.path, platform)) return { path: candidate.path, source: candidate.source, candidates };
  }

  const pathCandidate = findOnPath(env.PATH, env, platform);
  if (pathCandidate !== null) {
    const candidate = addCandidate(pathCandidate, "path");
    if (await isExecutableFile(candidate.path, platform)) return { path: candidate.path, source: candidate.source, candidates };
  }

  // 只有确认处在仓库树中才启用开发布局，避免生产目录误扫相对路径。
  const repositoryRoot = findRepositoryRoot(cwd);
  if (repositoryRoot !== null) {
    for (const relative of developmentCandidates(platform)) {
      const candidate = addCandidate(join(repositoryRoot, relative), "development");
      if (await isExecutableFile(candidate.path, platform)) return { path: candidate.path, source: candidate.source, candidates };
    }
  }
  const suffix = candidates.some((candidate) => candidate.source === "development") ? "；候选中包含开发布局路径" : "";
  return fail(`找不到 xiao-core；请构建核心或设置 XIAO_CORE_PATH${suffix}`);
}

/** 返回与分发目录约定一致的核心文件名。 */
export function coreExecutableName(platform = process.platform): string {
  return platform === "win32" ? "xiao-core.exe" : "xiao-core";
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

/** 从起点向上寻找同时包含 Bun manifest 和 Rust workspace 的仓库根。 */
function findRepositoryRoot(start: string): string | null {
  let current = resolve(start);
  while (true) {
    if (existsSync(join(current, "package.json")) && existsSync(join(current, "core", "rust", "Cargo.toml"))) return current;
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
function findOnPath(pathValue: string | undefined, env: NodeJS.ProcessEnv, platform: NodeJS.Platform): string | null {
  const command = platform === "win32" ? "where.exe" : "which";
  try {
    const output = execFileSync(command, ["xiao-core"], { env, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
    const first = output.split(/\r?\n/u).find((line) => line.trim().length > 0)?.trim();
    return first ?? null;
  } catch {
    // 某些嵌入式宿主没有 where/which；下面的手工扫描保持行为可测试。
    if (!pathValue) return null;
    const suffixes = platform === "win32" ? [".exe", ""] : [""];
    for (const directory of pathValue.split(platform === "win32" ? ";" : ":")) {
      for (const suffix of suffixes) {
        const candidate = join(directory, `xiao-core${suffix}`);
        try { if (requireStat(candidate)) return candidate; } catch { /* 继续扫描 */ }
      }
    }
    return null;
  }
}

/** 返回编译后 CLI 的可执行文件路径；Bun/Node 源码运行时返回空。 */
function runtimeExecutablePath(): string | null {
  const candidate = process.execPath;
  const name = basename(candidate).toLowerCase();
  if (["bun", "bun.exe", "node", "node.exe"].includes(name)) return null;
  return candidate || null;
}
