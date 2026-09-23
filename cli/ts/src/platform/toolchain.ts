/** X0-E 主机 LLVM/Rust 工具链发现；只负责路径、版本和候选诊断。 */

import { access, constants, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { statSync } from "node:fs";
import { execFile } from "node:child_process";
import { basename, dirname, join, normalize, resolve } from "node:path";
import { tmpdir } from "node:os";

import { hostTarget, type HostTarget } from "./core.ts";
import type { ToolchainSpec, ToolchainVersions } from "../protocol/messages.ts";

/** 工具链候选来源；顺序是构建契约的一部分。 */
export type ToolchainDiscoverySource = "override" | "adjacent" | "path" | "development";

/** 工具链中可被发现的工具名。 */
export type ToolchainName = "clang" | "llvm-as" | "llc" | "rustc" | "runtime" | "diagnostics";

/** 一组工具目录及其发现来源。 */
interface ToolchainRoot {
  path: string;
  source: Extract<ToolchainDiscoverySource, "override" | "adjacent">;
}

/** 一次候选尝试的完整机器摘要。 */
export interface ToolchainDiscoveryCandidate {
  /** 工具名。 */
  tool: ToolchainName;
  /** 规范化候选路径。 */
  path: string;
  /** 候选来源。 */
  source: ToolchainDiscoverySource;
  /** 尝试结果。 */
  status: "selected" | "missing" | "unusable";
  /** 版本首行或失败原因。 */
  reason?: string;
  /** 成功探测到的版本文本。 */
  version?: string;
  /** 工具报告的目标三元组（若版本输出包含 Target 行）。 */
  target?: string;
}

/** 工具链发现选项。 */
export interface ToolchainDiscoveryOptions {
  /** 当前工作目录。 */
  cwd?: string;
  /** 环境变量覆盖。 */
  env?: NodeJS.ProcessEnv;
  /** CLI 可执行文件路径，用于同目录候选。 */
  executablePath?: string;
  /** 目标平台；默认当前宿主。 */
  platform?: NodeJS.Platform;
  /** 目标架构；仅用于开发候选和错误信息。 */
  architecture?: string;
  /** 目标三元组；缺省从平台和架构推导。 */
  target?: HostTarget;
  /** 调试构建是否必须找到诊断组件。 */
  requireDiagnostics?: boolean;
  /** 是否执行一次真实的编译和链接探测；默认开启。 */
  probeLink?: boolean;
}

/** 工具链发现成功结果。 */
export interface ToolchainDiscoveryResult {
  /** 可传给 Rust 协议的工具链描述。 */
  toolchain: ToolchainSpec;
  /** 选中的候选来源摘要。 */
  selected: Readonly<Record<ToolchainName, ToolchainDiscoveryCandidate | null>>;
  /** 按尝试顺序保留全部候选。 */
  candidates: readonly ToolchainDiscoveryCandidate[];
  /** 配置文件登记的最低 LLVM 主版本。 */
  minimumMajor: number;
}

/** 工具链发现失败。 */
export class ToolchainDiscoveryError extends Error {
  /** 稳定机器错误码。 */
  readonly code: string;
  /** 候选尝试清单。 */
  readonly candidates: readonly ToolchainDiscoveryCandidate[];
  /** 结构化附加字段。 */
  readonly details: Record<string, unknown>;

  /** 创建工具链诊断。 */
  constructor(code: string, message: string, candidates: readonly ToolchainDiscoveryCandidate[] = [], details: Record<string, unknown> = {}) {
    super(`${code}: ${message}`);
    this.name = "ToolchainDiscoveryError";
    this.code = code;
    this.candidates = candidates;
    this.details = details;
  }
}

/** 从版本输出中提取 LLVM/Clang 主版本。 */
export function parseToolchainVersion(text: string): number | null {
  const match = /(?:clang|llvm)(?:\s+version)?\s+(\d+)(?:[.]\d+)?/iu.exec(text)
    ?? /(?:^|\s)(\d+)(?:[.]\d+){1,3}(?:\s|$)/u.exec(text);
  if (!match) return null;
  const major = Number.parseInt(match[1], 10);
  return Number.isSafeInteger(major) ? major : null;
}

/** 读取 `llvm-toolchain.toml` 登记的最低 LLVM 主版本。 */
export async function readMinimumMajor(cwd = process.cwd()): Promise<number> {
  const root = findRepositoryRoot(resolve(cwd));
  if (root === null) return 18;
  try {
    const text = await readFile(join(root, "core", "rust", "llvm-toolchain.toml"), "utf8");
    const match = /^\s*minimum_major\s*=\s*(\d+)\s*$/mu.exec(text);
    const value = match ? Number.parseInt(match[1], 10) : NaN;
    return Number.isSafeInteger(value) && value > 0 ? value : 18;
  } catch {
    return 18;
  }
}

/** 按固定候选顺序发现主机工具链。 */
export async function discoverToolchainWithMetadata(options: ToolchainDiscoveryOptions = {}): Promise<ToolchainDiscoveryResult> {
  const env = options.env ?? process.env;
  const cwd = resolve(options.cwd ?? process.cwd());
  const platform = options.platform ?? process.platform;
  const target = options.target ?? hostTarget(platform, (options.architecture ?? process.arch) as NodeJS.Architecture);
  const probeLink = options.probeLink ?? true;
  const candidates: ToolchainDiscoveryCandidate[] = [];
  const minimumMajor = await readMinimumMajor(cwd);
  const selected: Record<ToolchainName, ToolchainDiscoveryCandidate | null> = {
    clang: null,
    "llvm-as": null,
    llc: null,
    rustc: null,
    runtime: null,
    diagnostics: null,
  };

  const roots = toolchainRoots(cwd, env, options.executablePath);
  const pathDirectories = pathEntries(env.PATH, platform);
  const repositoryRoot = findRepositoryRoot(cwd);
  const development = repositoryRoot === null ? [] : developmentCandidates(repositoryRoot, platform);
  const names = executableNames(platform);

  for (const tool of ["clang", "llvm-as", "llc", "rustc"] as const) {
    const explicit = explicitToolPath(tool, env, cwd);
    const found = await discoverExecutable(tool, explicit, roots, pathDirectories, development, names, platform, env, cwd, candidates);
    if (explicit !== null && (found === null || found.status !== "selected")) {
      throw explicitOverrideError(tool, explicit, candidates);
    }
    if (found?.status === "selected") selected[tool] = found;
  }

  const runtime = await discoverRuntime(env, cwd, roots, pathDirectories, development, platform, candidates);
  if (runtime?.status === "selected") selected.runtime = runtime;
  else if (env.XIAO_RUNTIME_LIBRARY?.trim()) {
    throw explicitOverrideError("runtime", resolve(cwd, env.XIAO_RUNTIME_LIBRARY), candidates);
  }
  if (options.requireDiagnostics) {
    const diagnostics = await discoverDiagnostics(env, cwd, roots, pathDirectories, development, platform, candidates);
    if (diagnostics?.status !== "selected") {
      if (env.XIAO_DIAGNOSTICS_PATH?.trim()) {
        throw explicitOverrideError("diagnostics", resolve(cwd, env.XIAO_DIAGNOSTICS_PATH), candidates);
      }
      throw new ToolchainDiscoveryError(
        "X11-CLI-TOOLCHAIN-003",
        "调试构建找不到 xiao-diagnostics；请随分发包携带它或设置 XIAO_DIAGNOSTICS_PATH",
        candidates,
        { required: ["xiao-diagnostics"] },
      );
    }
    selected.diagnostics = diagnostics;
  }

  const clang = selected.clang;
  if (clang === null) {
    throw new ToolchainDiscoveryError(
      "X11-CLI-TOOLCHAIN-001",
      "找不到必需的 clang；请安装 LLVM 18 或更高版本，或设置 XIAO_CLANG",
      candidates,
      { required: ["clang"], minimum_major: minimumMajor },
    );
  }
  const clangMajor = parseToolchainVersion(clang.version ?? "");
  if (clangMajor === null) {
    throw new ToolchainDiscoveryError(
      "X11-CLI-TOOLCHAIN-002",
      `无法读取 clang 版本：${clang.path}`,
      candidates,
      { path: clang.path, minimum_major: minimumMajor },
    );
  }
  if (clangMajor < minimumMajor) {
    throw new ToolchainDiscoveryError(
      "X11-CLI-TOOLCHAIN-VERSION-001",
      `clang 主版本 ${clangMajor} 低于最低要求 ${minimumMajor}：${clang.path}`,
      candidates,
      { path: clang.path, actual_major: clangMajor, minimum_major: minimumMajor },
    );
  }
  const targetReason = targetCompatibilityReason(clang, target);
  if (targetReason !== null && !probeLink) {
    clang.status = "unusable";
    clang.reason = targetReason;
    throw new ToolchainDiscoveryError(
      "X11-CLI-TOOLCHAIN-TARGET-001",
      `clang 不能为目标 ${target.triple} 提供兼容目标：${targetReason}`,
      candidates,
      { path: clang.path, target: target.triple, reported_target: clang.target ?? null },
    );
  }
  if (probeLink) {
    const probe = await probeClang(clang.path, target, platform, env, cwd);
    if (!probe.ok) {
      clang.status = "unusable";
      clang.reason = probe.reason;
      throw new ToolchainDiscoveryError(
        targetReason === null ? "X11-CLI-TOOLCHAIN-LINK-001" : "X11-CLI-TOOLCHAIN-TARGET-001",
        targetReason === null
          ? `clang 无法完成目标 ${target.triple} 的编译/链接探测：${probe.reason}`
          : `clang 报告 ${clang.target}，且无法为目标 ${target.triple} 完成编译/链接探测：${probe.reason}`,
        candidates,
        { path: clang.path, target: target.triple, reported_target: clang.target ?? null, phase: probe.phase },
      );
    }
  }

  const versions: ToolchainVersions = {
    clang: clang.version ?? "",
    llvm_as: selected["llvm-as"]?.version ?? null,
    llc: selected.llc?.version ?? null,
    rustc: selected.rustc?.version ?? null,
  };
  return {
    toolchain: {
      clang: clang.path,
      llvm_as: selected["llvm-as"]?.path ?? null,
      llc: selected.llc?.path ?? null,
      runtime_library: runtime?.path ?? null,
      native_static_libraries: [],
      rustc: selected.rustc?.path ?? null,
      diagnostics_path: selected.diagnostics?.path ?? null,
      versions,
    },
    selected,
    candidates,
    minimumMajor,
  };
}

/** 只返回协议工具链描述的便捷入口。 */
export async function discoverToolchain(options: ToolchainDiscoveryOptions = {}): Promise<ToolchainSpec> {
  return (await discoverToolchainWithMetadata(options)).toolchain;
}

/** 为每个工具生成一次候选并执行版本探测。 */
async function discoverExecutable(
  tool: Exclude<ToolchainName, "runtime" | "diagnostics">,
  explicit: string | null,
  roots: readonly ToolchainRoot[],
  pathDirectories: readonly string[],
  development: readonly string[],
  names: Record<Exclude<ToolchainName, "runtime" | "diagnostics">, string>,
  platform: NodeJS.Platform,
  env: NodeJS.ProcessEnv,
  cwd: string,
  candidates: ToolchainDiscoveryCandidate[],
): Promise<ToolchainDiscoveryCandidate | null> {
  const attempted = new Set<string>();
  const tryPath = async (path: string, source: ToolchainDiscoverySource, hard = false): Promise<ToolchainDiscoveryCandidate | null> => {
    const normalized = normalize(resolve(path));
    if (attempted.has(normalized)) return null;
    attempted.add(normalized);
    const candidate: ToolchainDiscoveryCandidate = { tool, path: normalized, source, status: "missing" };
    candidates.push(candidate);
    if (!(await isExecutableFile(normalized, platform))) {
      candidate.reason = "文件不存在或不可执行";
      if (hard) return candidate;
      return null;
    }
    try {
      const version = await versionLine(normalized, platform, env, cwd);
      candidate.status = "selected";
      candidate.version = version.line;
      candidate.target = version.target;
      return candidate;
    } catch (error) {
      candidate.status = "unusable";
      candidate.reason = String(error);
      if (hard) return candidate;
      return null;
    }
  };
  if (explicit !== null) return tryPath(explicit, "override", true);
  for (const root of roots) {
    const found = await tryPath(join(root.path, names[tool]), root.source);
    if (found?.status === "selected") return found;
  }
  for (const directory of pathDirectories) {
    const found = await tryPath(join(directory, names[tool]), "path");
    if (found?.status === "selected") return found;
  }
  for (const path of development) {
    if (!path.endsWith(names[tool])) continue;
    const found = await tryPath(path, "development");
    if (found?.status === "selected") return found;
  }
  return null;
}

/** 发现可选 Runtime staticlib。 */
async function discoverRuntime(
  env: NodeJS.ProcessEnv,
  cwd: string,
  roots: readonly ToolchainRoot[],
  pathDirectories: readonly string[],
  development: readonly string[],
  platform: NodeJS.Platform,
  candidates: ToolchainDiscoveryCandidate[],
): Promise<ToolchainDiscoveryCandidate | null> {
  const explicit = env.XIAO_RUNTIME_LIBRARY?.trim() ? resolve(cwd, env.XIAO_RUNTIME_LIBRARY) : null;
  const names = platform === "win32" ? ["xiao_runtime.lib", "libxiao_runtime.lib"] : ["libxiao_runtime.a", "xiao_runtime.a"];
  const attempted = new Set<string>();
  const tryPath = async (path: string, source: ToolchainDiscoverySource, hard = false): Promise<ToolchainDiscoveryCandidate | null> => {
    const normalized = normalize(resolve(path));
    if (attempted.has(normalized)) return null;
    attempted.add(normalized);
    const candidate: ToolchainDiscoveryCandidate = { tool: "runtime", path: normalized, source, status: "missing" };
    candidates.push(candidate);
    if (await isExecutableFile(normalized, platform) || await isRegularFile(normalized)) {
      candidate.status = "selected";
      return candidate;
    }
    candidate.reason = "文件不存在";
    return hard ? candidate : null;
  };
  if (explicit !== null) return tryPath(explicit, "override", true);
  for (const root of roots) for (const name of names) {
    const found = await tryPath(join(root.path, name), root.source);
    if (found?.status === "selected") return found;
  }
  for (const directory of pathDirectories) for (const name of names) {
    const found = await tryPath(join(directory, name), "path");
    if (found?.status === "selected") return found;
  }
  for (const path of development) {
    if (!names.some((name) => path.endsWith(name))) continue;
    const found = await tryPath(path, "development");
    if (found?.status === "selected") return found;
  }
  return null;
}

/** 发现调试构建必须随产物可用的诊断进程。 */
async function discoverDiagnostics(
  env: NodeJS.ProcessEnv,
  cwd: string,
  roots: readonly ToolchainRoot[],
  pathDirectories: readonly string[],
  development: readonly string[],
  platform: NodeJS.Platform,
  candidates: ToolchainDiscoveryCandidate[],
): Promise<ToolchainDiscoveryCandidate | null> {
  const name = platform === "win32" ? "xiao-diagnostics.exe" : "xiao-diagnostics";
  const explicit = env.XIAO_DIAGNOSTICS_PATH?.trim() ? resolve(cwd, env.XIAO_DIAGNOSTICS_PATH) : null;
  const attempted = new Set<string>();
  const tryPath = async (path: string, source: ToolchainDiscoverySource, hard = false): Promise<ToolchainDiscoveryCandidate | null> => {
    const normalized = normalize(resolve(path));
    if (attempted.has(normalized)) return null;
    attempted.add(normalized);
    const candidate: ToolchainDiscoveryCandidate = { tool: "diagnostics", path: normalized, source, status: "missing" };
    candidates.push(candidate);
    if (await isExecutableFile(normalized, platform)) {
      candidate.status = "selected";
      return candidate;
    }
    candidate.reason = "文件不存在或不可执行";
    return hard ? candidate : null;
  };
  if (explicit !== null) return tryPath(explicit, "override", true);
  for (const root of roots) {
    const found = await tryPath(join(root.path, name), root.source);
    if (found?.status === "selected") return found;
  }
  for (const directory of pathDirectories) {
    const found = await tryPath(join(directory, name), "path");
    if (found?.status === "selected") return found;
  }
  for (const path of development) {
    if (!path.endsWith(name)) continue;
    const found = await tryPath(path, "development");
    if (found?.status === "selected") return found;
  }
  return null;
}

/** 读取一个工具的版本首行，并保留目标三元组。 */
async function versionLine(path: string, platform: NodeJS.Platform, environment: NodeJS.ProcessEnv, cwd: string): Promise<{ line: string; target?: string }> {
  try {
    const result = await runExecutable(path, ["--version"], platform, environment, cwd);
    const output = result.stdout || result.stderr;
    return {
      line: output.split(/\r?\n/u).find((line) => line.trim())?.trim() ?? path,
      target: parseReportedTarget(output),
    };
  } catch (error) {
    // 子进程错误通常仍携带 stdout/stderr，优先保留工具原文。
    const record = error as { stdout?: string | Buffer; stderr?: string | Buffer; status?: number | null };
    if (record.status !== undefined && record.status !== 0) throw new Error(String(record.stderr || record.stdout || error));
    throw error;
  }
}

/** 执行外部工具；Windows 统一经 cmd，并把完整路径放进同一条 `/c` 命令行。 */
async function runExecutable(
  path: string,
  arguments_: readonly string[],
  platform: NodeJS.Platform,
  environment: NodeJS.ProcessEnv,
  cwd: string,
): Promise<{ stdout: string; stderr: string }> {
  return new Promise<{ stdout: string; stderr: string }>((resolvePromise, reject) => {
      const command = platform === "win32" ? (process.env.ComSpec ?? "cmd.exe") : path;
      const args = platform === "win32"
        ? [
            "/d",
            "/c",
            [
              "call",
              quoteWindowsCommandArgument(path),
              ...arguments_.map((argument) => /[\s"]/u.test(argument) ? quoteWindowsCommandArgument(argument) : argument),
            ].join(" "),
          ]
        : [...arguments_];
      const env = platform === "win32"
        ? { ...environment, PATH: `${dirname(path)};${environment.PATH ?? ""}` }
        : environment;
      execFile(command, args, {
        cwd,
        encoding: "utf8",
        windowsHide: true,
        windowsVerbatimArguments: platform === "win32",
        timeout: 10_000,
        env,
      }, (error, stdout, stderr) => {
        if (error) {
          Object.assign(error, { stdout, stderr });
          reject(error);
          return;
        }
        resolvePromise({ stdout, stderr });
      });
  });
}

/** 为 Windows `cmd /c` 命令行引用一个无 shell 语义的参数。 */
function quoteWindowsCommandArgument(value: string): string {
  return `"${value.replaceAll('"', '""')}"`;
}

/** 读取 clang `--version` 输出中的 Target 行。 */
function parseReportedTarget(output: string): string | undefined {
  return /(?:^|\n)\s*Target:\s*([^\s\r\n]+)/iu.exec(output)?.[1];
}

/** 显式覆盖路径失效时立即失败，不把用户意图静默改成 PATH 候选。 */
function explicitOverrideError(
  tool: ToolchainName,
  path: string,
  candidates: readonly ToolchainDiscoveryCandidate[],
): ToolchainDiscoveryError {
  return new ToolchainDiscoveryError(
    "X11-CLI-TOOLCHAIN-OVERRIDE-001",
    `显式 ${tool} 路径不可用：${path}`,
    candidates,
    { tool, path, source: "override" },
  );
}

/** 根据 clang 版本中的 Target 行拒绝明显不匹配的 ABI。 */
function targetCompatibilityReason(candidate: ToolchainDiscoveryCandidate, target: HostTarget): string | null {
  const reported = candidate.target?.toLowerCase();
  if (!reported) return null;
  if (target.object_format === "coff" && !reported.includes("windows-msvc")) {
    return `工具报告 ${candidate.target}，需要 windows-msvc（GNU clang 不能直接链接冻结的 MSVC 目标）`;
  }
  if (target.object_format === "elf" && !reported.includes("linux")) {
    return `工具报告 ${candidate.target}，需要 Linux ELF 目标`;
  }
  if (target.object_format === "macho" && !reported.includes("darwin")) {
    return `工具报告 ${candidate.target}，需要 macOS Mach-O 目标`;
  }
  return null;
}

/** 空 C 程序探测返回的阶段与失败原因。 */
interface ClangProbeResult {
  ok: boolean;
  phase: "compile" | "link";
  reason: string;
}

/** 在临时目录中编译并链接一个空 C 入口，提前发现 SDK/linker 不可用。 */
async function probeClang(
  clang: string,
  target: HostTarget,
  platform: NodeJS.Platform,
  environment: NodeJS.ProcessEnv,
  cwd: string,
): Promise<ClangProbeResult> {
  const directory = await mkdtemp(join(tmpdir(), "xiao-toolchain-"));
  const source = join(directory, "probe.c");
  const object = join(directory, platform === "win32" ? "probe.obj" : "probe.o");
  const executable = join(directory, platform === "win32" ? "probe.exe" : "probe");
  try {
    await writeFile(source, "int main(void) { return 0; }\n", "utf8");
    const compile = await runProbeCommand(clang, ["-target", target.triple, "-x", "c", "-c", source, "-o", object], platform, environment, cwd);
    if (!compile.ok) return { ok: false, phase: "compile", reason: compile.reason };
    const link = await runProbeCommand(clang, ["-target", target.triple, source, "-o", executable], platform, environment, cwd);
    if (!link.ok) return { ok: false, phase: "link", reason: link.reason };
    return { ok: true, phase: "link", reason: "" };
  } finally {
    await rm(directory, { recursive: true, force: true }).catch(() => undefined);
  }
}

/** 执行一次 clang 探测命令，并把进程失败收敛成候选原因。 */
async function runProbeCommand(
  path: string,
  arguments_: readonly string[],
  platform: NodeJS.Platform,
  environment: NodeJS.ProcessEnv,
  cwd: string,
): Promise<{ ok: boolean; reason: string }> {
  try {
    const result = await runExecutable(path, arguments_, platform, environment, cwd);
    return { ok: true, reason: result.stderr.trim() };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return { ok: false, reason: message.slice(0, 500) };
  }
}

/** 解析显式单工具覆盖。 */
function explicitToolPath(tool: Exclude<ToolchainName, "runtime">, env: NodeJS.ProcessEnv, cwd: string): string | null {
  const key = tool === "clang" ? "XIAO_CLANG" : tool === "llvm-as" ? "XIAO_LLVM_AS" : tool === "llc" ? "XIAO_LLC" : "XIAO_RUSTC";
  const value = env[key]?.trim();
  return value ? resolve(cwd, value) : null;
}

/** 生成工具根目录候选；显式根目录优先。 */
function toolchainRoots(cwd: string, env: NodeJS.ProcessEnv, executablePath: string | undefined): ToolchainRoot[] {
  const roots: ToolchainRoot[] = [];
  const add = (value: string | undefined, source: ToolchainRoot["source"]): void => {
    if (!value?.trim()) return;
    const absolute = resolve(cwd, value);
    const bin = basename(absolute).toLowerCase() === "bin" ? absolute : join(absolute, "bin");
    if (!roots.some((root) => root.path === bin)) roots.push({ path: bin, source });
  };
  add(env.XIAO_LLVM_BIN, "override");
  add(env.XIAO_TOOLCHAIN_ROOT, "override");
  if (executablePath) {
    const adjacent = dirname(resolve(executablePath));
    add(join(adjacent, "llvm"), "adjacent");
    if (!roots.some((root) => root.path === adjacent)) roots.push({ path: adjacent, source: "adjacent" });
  }
  return roots;
}

/** 返回 PATH 目录，保留宿主分隔符。 */
function pathEntries(value: string | undefined, platform: NodeJS.Platform): string[] {
  return (value ?? "").split(platform === "win32" ? ";" : ":").map((item) => item.trim()).filter(Boolean);
}

/** 不同宿主的工具文件名。 */
function executableNames(platform: NodeJS.Platform): Record<Exclude<ToolchainName, "runtime" | "diagnostics">, string> {
  const suffix = platform === "win32" ? ".exe" : "";
  return { clang: `clang${suffix}`, "llvm-as": `llvm-as${suffix}`, llc: `llc${suffix}`, rustc: `rustc${suffix}` };
}

/** 开发树候选；生产目录没有仓库根时不会启用。 */
function developmentCandidates(root: string, platform: NodeJS.Platform): string[] {
  const suffix = platform === "win32" ? ".exe" : "";
  const llvm = [
    join(root, "core", "rust", "target", "debug"),
    join(root, "core", "rust", "target", "release"),
  ];
  const runtime = platform === "win32" ? "xiao_runtime.lib" : "libxiao_runtime.a";
  return [
    ...llvm.flatMap((dir) => [
      join(dir, `clang${suffix}`),
      join(dir, `llvm-as${suffix}`),
      join(dir, `llc${suffix}`),
      join(dir, `rustc${suffix}`),
      join(dir, runtime),
      join(dir, `xiao-diagnostics${suffix}`),
    ]),
    join(root, "target", "debug", runtime),
    join(root, "target", "release", runtime),
  ];
}

/** 判断路径是否为普通文件。 */
async function isRegularFile(path: string): Promise<boolean> {
  try { return (await stat(path)).isFile(); } catch { return false; }
}

/** 判断路径是否可执行；Windows 文件存在即视为可执行。 */
async function isExecutableFile(path: string, platform: NodeJS.Platform): Promise<boolean> {
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

/** 向上寻找同时包含 JS 与 Rust 工作区的仓库根。 */
function findRepositoryRoot(start: string): string | null {
  let current = resolve(start);
  while (true) {
    if (exists(join(current, "package.json")) && exists(join(current, "core", "rust", "Cargo.toml"))) return current;
    const parent = dirname(current);
    if (parent === current) return null;
    current = parent;
  }
}

/** 仅用于发现阶段的同步文件存在判断。 */
function exists(path: string): boolean {
  try { return statSync(path).isFile(); } catch { return false; }
}
