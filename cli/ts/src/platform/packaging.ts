/** X0-C 的 CLI 独立打包器；只负责 Bun 产物和相邻 Rust 核心的分发。 */

import { chmod, copyFile, mkdir, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";

import {
  coreExecutableName,
  discoverCoreWithMetadata,
  isExecutableFile,
  hostTarget,
  type CoreDiscoverySource,
  type HostTarget,
} from "./core.ts";

/** Bun 可生成的发布平台。 */
export type PackagePlatform = "win32" | "linux" | "darwin";

/** Bun 可生成的发布架构。 */
export type PackageArchitecture = "x64" | "arm64";

/** 一个 CLI 与 Rust 核心必须共同使用的发布目标。 */
export interface PackageTarget {
  /** Bun 的目标参数，例如 `bun-windows-x64`。 */
  bunTarget: string;
  /** Node/Bun 平台名。 */
  platform: PackagePlatform;
  /** CPU 架构名。 */
  architecture: PackageArchitecture;
  /** 与协议目标字段一致的 Rust 目标描述。 */
  rustTarget: HostTarget;
  /** CLI 产物文件名。 */
  executableName: string;
  /** 相邻 Rust 核心文件名。 */
  coreName: string;
  /** 相邻独立诊断进程文件名。 */
  diagnosticsName: string;
}

/** 打包器的程序化选项。路径相对 `cli/ts` 目录解析。 */
export interface PackageBuildOptions {
  /** Bun 目标参数；省略时使用当前宿主。 */
  target?: string;
  /** 输出目录；省略时使用 `dist/<bun-target>`。 */
  outDir?: string;
  /** Rust 核心显式路径；省略时按生产发现顺序寻找。 */
  corePath?: string;
  /** 独立诊断进程显式路径；省略时按核心相邻/开发树顺序寻找。 */
  diagnosticsPath?: string;
  /** 构建工作目录；仅供测试和嵌入式调用覆盖。 */
  cwd?: string;
  /** 构建与发现阶段使用的环境变量。 */
  env?: NodeJS.ProcessEnv;
}

/** 命令行参数解析结果。 */
export interface PackagingCliOptions extends PackageBuildOptions {
  /** 是否只请求帮助文本。 */
  help: boolean;
}

/** 一次成功打包的结构化结果。 */
export interface PackageBuildResult {
  /** 目标描述。 */
  target: PackageTarget;
  /** 最终分发目录。 */
  outputDirectory: string;
  /** 独立 CLI 路径。 */
  executablePath: string;
  /** 相邻核心路径。 */
  corePath: string;
  /** 相邻诊断进程路径。 */
  diagnosticsPath: string;
  /** 分发清单路径。 */
  manifestPath: string;
  /** 核心发现来源。 */
  coreSource: CoreDiscoverySource;
}

/** 打包阶段的稳定错误形状。 */
export class PackagingError extends Error {
  /** 机器可读错误码。 */
  readonly code: string;
  /** 结构化附加字段。 */
  readonly details: Record<string, unknown>;

  /** 创建打包错误。 */
  constructor(code: string, message: string, details: Record<string, unknown> = {}) {
    super(`${code}: ${message}`);
    this.name = "PackagingError";
    this.code = code;
    this.details = details;
  }
}

const PACKAGE_ROOT = resolve(import.meta.dir, "..", "..");
const PACKAGE_VERSION = "0.1.0";

/** 返回当前宿主对应的 Bun 发布目标。 */
export function hostPackageTarget(platform = process.platform, architecture = process.arch): PackageTarget {
  const normalizedPlatform = platform === "win32" ? "win32" : platform === "linux" ? "linux" : platform === "darwin" ? "darwin" : null;
  const normalizedArchitecture = architecture === "x64" ? "x64" : architecture === "arm64" ? "arm64" : null;
  if (normalizedPlatform === null || normalizedArchitecture === null) {
    throw new PackagingError("X11-PACKAGE-TARGET-001", `不支持的打包目标：${platform}/${architecture}`, { platform, architecture });
  }
  return makePackageTarget(normalizedPlatform, normalizedArchitecture);
}

/** 将 `bun-<platform>-<arch>` 参数解析为统一目标。 */
export function parsePackageTarget(value: string): PackageTarget {
  const match = /^bun-(windows|linux|darwin)-(x64|arm64)$/u.exec(value);
  if (!match) {
    throw new PackagingError(
      "X11-PACKAGE-TARGET-001",
      `目标必须是 bun-windows-x64、bun-windows-arm64、bun-linux-x64、bun-linux-arm64、bun-darwin-x64 或 bun-darwin-arm64：${value}`,
      { value },
    );
  }
  const platform = match[1] === "windows" ? "win32" : match[1] as "linux" | "darwin";
  return makePackageTarget(platform, match[2] as PackageArchitecture);
}

/** 解析 `bun run build` 接收的参数。 */
export function parsePackagingArguments(argv: readonly string[]): PackagingCliOptions {
  let target: string | undefined;
  let outDir: string | undefined;
  let corePath: string | undefined;
  let diagnosticsPath: string | undefined;
  let help = false;
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--help" || argument === "-h") {
      help = true;
      continue;
    }
    if (argument === "--target" || argument === "--outdir" || argument === "--core" || argument === "--diagnostics") {
      const value = argv[index + 1];
      if (!value || value.startsWith("-")) throw new PackagingError("X11-PACKAGE-ARG-001", `${argument} 需要一个值`, { argument });
      index += 1;
      if (argument === "--target") target = value;
      else if (argument === "--outdir") outDir = value;
      else if (argument === "--core") corePath = value;
      else diagnosticsPath = value;
      continue;
    }
    if (argument.startsWith("--target=")) target = argument.slice("--target=".length);
    else if (argument.startsWith("--outdir=")) outDir = argument.slice("--outdir=".length);
    else if (argument.startsWith("--core=")) corePath = argument.slice("--core=".length);
    else if (argument.startsWith("--diagnostics=")) diagnosticsPath = argument.slice("--diagnostics=".length);
    else throw new PackagingError("X11-PACKAGE-ARG-001", `未知打包参数：${argument}`, { argument });
  }
  return { target, outDir, corePath, diagnosticsPath, help };
}

/** 返回打包器帮助文本。 */
export function packagingHelpText(): string {
  return [
    "用法：bun run src/platform/packaging.ts [选项]",
    "",
    "  --target <bun-windows-x64|bun-windows-arm64|bun-linux-x64|bun-linux-arm64|bun-darwin-x64|bun-darwin-arm64>",
    "  --outdir <目录>    覆盖默认 dist/<目标> 输出目录",
    "  --core <路径>      显式指定要随包分发的 xiao-core",
    "  --diagnostics <路径> 显式指定独立诊断进程",
    "  --help             显示帮助",
    "",
    "打包需要 Bun；生成的 xiao 可执行文件运行时不需要 Bun 或 Node.js。",
  ].join("\n") + "\n";
}

/** 构建独立 CLI、复制相邻核心并写入分发清单。 */
export async function buildPackage(options: PackageBuildOptions = {}): Promise<PackageBuildResult> {
  const packageRoot = resolve(options.cwd ?? PACKAGE_ROOT);
  const target = options.target ? parsePackageTarget(options.target) : hostPackageTarget();
  const outputDirectory = resolve(packageRoot, options.outDir ?? join("dist", target.bunTarget));
  const executablePath = join(outputDirectory, target.executableName);
  const coreDestination = join(outputDirectory, target.coreName);
  const diagnosticsDestination = join(outputDirectory, target.diagnosticsName);
  const manifestPath = join(outputDirectory, "xiao-package.json");
  const entrypoint = join(packageRoot, "src", "main.ts");
  const environment = { ...process.env, ...(options.env ?? {}) };

  if (!process.versions.bun) {
    throw new PackagingError("X11-PACKAGE-BUILD-001", "独立打包必须由 Bun 执行", { runtime: process.execPath });
  }
  const discovery = await discoverCoreWithMetadata({
    cwd: packageRoot,
    env: environment,
    overridePath: options.corePath,
    platform: target.platform,
    architecture: target.architecture,
  });
  const diagnosticsSource = await discoverDiagnostics({
    cwd: packageRoot,
    corePath: discovery.path,
    explicitPath: options.diagnosticsPath,
    platform: target.platform,
  });
  await mkdir(outputDirectory, { recursive: true });
  const buildResult = spawnSync(process.execPath, [
    "build",
    "--compile",
    `--target=${target.bunTarget}`,
    `--outfile=${executablePath}`,
    entrypoint,
  ], {
    cwd: packageRoot,
    env: environment,
    stdio: "inherit",
  });
  if (buildResult.error || buildResult.status !== 0) {
    throw new PackagingError("X11-PACKAGE-BUILD-002", "Bun 独立可执行构建失败", {
      target: target.bunTarget,
      status: buildResult.status,
      signal: buildResult.signal,
      cause: buildResult.error ? String(buildResult.error) : null,
    });
  }
  await copyFile(discovery.path, coreDestination);
  await copyFile(diagnosticsSource, diagnosticsDestination);
  if (target.platform !== "win32") await chmod(coreDestination, 0o755);
  if (target.platform !== "win32") await chmod(diagnosticsDestination, 0o755);
  const manifest = {
    format_version: 1,
    cli: { file: target.executableName, version: PACKAGE_VERSION, runtime: "bun-compile", requires_node: false },
    core: { file: target.coreName, source: discovery.source },
    diagnostics: { file: target.diagnosticsName, source: diagnosticsSource },
    target: { bun: target.bunTarget, rust: target.rustTarget },
  };
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  return {
    target,
    outputDirectory,
    executablePath,
    corePath: coreDestination,
    diagnosticsPath: diagnosticsDestination,
    manifestPath,
    coreSource: discovery.source,
  };
}

/** 将平台名和架构组装成完整发布目标。 */
function makePackageTarget(platform: PackagePlatform, architecture: PackageArchitecture): PackageTarget {
  const bunPlatform = platform === "win32" ? "windows" : platform;
  return {
    bunTarget: `bun-${bunPlatform}-${architecture}`,
    platform,
    architecture,
    rustTarget: hostTarget(platform, architecture),
    executableName: platform === "win32" ? "xiao.exe" : "xiao",
    coreName: coreExecutableName(platform),
    diagnosticsName: platform === "win32" ? "xiao-diagnostics.exe" : "xiao-diagnostics",
  };
}

/** 按与核心相同的可审计顺序寻找独立诊断进程。 */
async function discoverDiagnostics(options: {
  cwd: string;
  corePath: string;
  explicitPath?: string;
  platform: PackagePlatform;
}): Promise<string> {
  const name = options.platform === "win32" ? "xiao-diagnostics.exe" : "xiao-diagnostics";
  const candidates = options.explicitPath
    ? [resolve(options.cwd, options.explicitPath)]
    : [
        join(dirname(resolve(options.corePath)), name),
        join(options.cwd, "core", "rust", "target", "debug", name),
        join(options.cwd, "core", "rust", "target", "release", name),
      ];
  for (const candidate of candidates) {
    if (await isExecutableFile(candidate, options.platform)) return candidate;
  }
  throw new PackagingError("X11-PACKAGE-DIAGNOSTICS-001", "找不到独立诊断进程；调试构建必须随包携带 xiao-diagnostics", {
    candidates,
    explicit: options.explicitPath ?? null,
  });
}

/** 程序入口：解析参数、构建产物并打印分发位置。 */
async function main(): Promise<void> {
  const options = parsePackagingArguments(process.argv.slice(2));
  if (options.help) {
    process.stdout.write(packagingHelpText());
    return;
  }
  const result = await buildPackage(options);
  process.stdout.write([
    `已生成 ${result.target.bunTarget}`,
    `  CLI: ${result.executablePath}`,
    `  核心: ${result.corePath}（${result.coreSource}）`,
    `  诊断: ${result.diagnosticsPath}`,
    `  清单: ${result.manifestPath}`,
    "",
  ].join("\n"));
}

if (import.meta.main) {
  try {
    await main();
  } catch (error) {
    const normalized = error instanceof PackagingError
      ? error
      : new PackagingError("X11-PACKAGE-BUILD-003", String(error));
    process.stderr.write(`${normalized.message}\n`);
    process.exitCode = 1;
  }
}
