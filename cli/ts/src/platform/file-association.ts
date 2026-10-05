/** `.xar` 文件关联：三平台计划、安装/检查/移除和双击行为契约。 */

import { access, chmod, constants, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { execFile as execFileCallback } from "node:child_process";
import { basename, dirname, join, resolve } from "node:path";
import { homedir } from "node:os";
import { promisify } from "node:util";

import { cliMessage } from "../i18n.ts";

const execFile = promisify(execFileCallback);
let associationSequence = 0;

/** 支持文件关联的桌面平台。 */
export type AssociationPlatform = "win32" | "linux" | "darwin";
/** 文件关联的三种独立动作。 */
export type AssociationAction = "install" | "check" | "uninstall";

/** 一个可安装/可移除的关联设计。 */
export interface FileAssociationPlan {
  /** 目标平台。 */
  platform: AssociationPlatform;
  /** 关联扩展名。 */
  extension: ".xar";
  /** MIME 或平台内容类型。 */
  contentType: string;
  /** 设计中的注册位置。 */
  registrationTarget: string;
  /** 将归档交给 xiao 的命令参数。 */
  command: readonly string[];
  /** 设计中的卸载动作。 */
  uninstall: string;
}

/** 双击与命令行共享的启动契约。 */
export interface ArchiveLaunchContract {
  /** 双击启动时不使用桌面当前目录。 */
  cwdPolicy: "launcher-directory-independent";
  /** 传给 xiao 的参数，归档路径位于 `-xar` 后。 */
  argv: readonly string[];
  /** 标准输入、输出和错误流全部继承。 */
  stdio: "inherit";
  /** 退出码原样转发。 */
  exitCode: "forwarded";
}

/** 原生平台命令的可注入执行器，便于门控测试和无平台环境验证。 */
export type AssociationCommandRunner = (command: string, args: readonly string[]) => Promise<AssociationCommandResult>;

/** 原生平台命令结果。 */
export interface AssociationCommandResult {
  /** 命令退出码；启动失败时为空。 */
  status: number | null;
  /** 标准输出。 */
  stdout: string;
  /** 标准错误。 */
  stderr: string;
}

/** 关联操作的程序化选项。 */
export interface FileAssociationOptions {
  /** 覆盖目标平台；默认当前宿主。 */
  platform?: AssociationPlatform;
  /** 已安装的 xiao 可执行文件路径；省略时按环境发现。 */
  executablePath?: string;
  /** 环境变量覆盖。 */
  env?: NodeJS.ProcessEnv;
  /** 用户主目录覆盖，供测试隔离。 */
  homeDir?: string;
  /** 原生命令执行器覆盖。 */
  runCommand?: AssociationCommandRunner;
}

/** 关联检查/安装/移除的机器结果。 */
export interface FileAssociationResult {
  /** 固定消息类型。 */
  type: "association";
  /** 执行动作。 */
  action: AssociationAction;
  /** 目标平台。 */
  platform: AssociationPlatform;
  /** 是否检测到关联。 */
  installed: boolean;
  /** 关联是否指向本次发现的 xiao。 */
  matches: boolean;
  /** 实际发现的 xiao 路径；未发现时为空。 */
  executable: string | null;
  /** 是否改变了系统或用户文件。 */
  changed: boolean;
  /** macOS 真实注册是否受环境门控。 */
  gated: boolean;
  /** 平台注册位置或状态文件。 */
  registration: string;
  /** 机器可读附加事实。 */
  details: Record<string, unknown>;
}

/** 关联错误；CLI 直接使用稳定 code 和 details。 */
export class FileAssociationError extends Error {
  /** 稳定错误编号。 */
  readonly code: string;
  /** 结构化附加字段。 */
  readonly details: Record<string, unknown>;
  /** CLI 建议进程码。 */
  readonly exitCode: number;

  /** 创建关联错误。 */
  constructor(code: string, message: string, details: Record<string, unknown> = {}, exitCode = 78) {
    super(`${code}: ${message}`);
    this.name = "FileAssociationError";
    this.code = code;
    this.details = details;
    this.exitCode = exitCode;
  }
}

/** 返回三平台关联设计；执行体另走 install/check/uninstall。 */
export function fileAssociationPlan(platform: AssociationPlatform): FileAssociationPlan {
  if (platform === "win32") {
    return {
      platform,
      extension: ".xar",
      contentType: "Xiao.Archive",
      registrationTarget: "HKCU\\Software\\Classes\\.xar and Xiao.Archive\\shell\\open\\command",
      command: ["xiao.exe", "-xar", "%1"],
      uninstall: "remove the Xiao.Archive class and the .xar user association",
    };
  }
  if (platform === "linux") {
    return {
      platform,
      extension: ".xar",
      contentType: "application/x-xiao-xar",
      registrationTarget: "~/.local/share/applications/xiao-xar.desktop and shared-mime-info",
      command: ["xiao", "-xar", "%f"],
      uninstall: "remove xiao-xar.desktop and the user MIME override",
    };
  }
  return {
    platform,
    extension: ".xar",
    contentType: "com.programming-language-xiao.xar",
    registrationTarget: "~/Library/LaunchServices association for the exported UTI",
    command: ["xiao", "-xar", "%1"],
    uninstall: "remove the user LaunchServices UTI association",
  };
}

/** 生成双击/命令行共用的行为契约；不把桌面 cwd 当作项目根。 */
export function archiveLaunchContract(archivePath: string, args: readonly string[] = []): ArchiveLaunchContract {
  if (archivePath.trim() === "") throw new Error("归档路径不能为空");
  return {
    cwdPolicy: "launcher-directory-independent",
    argv: ["-xar", archivePath, ...args],
    stdio: "inherit",
    exitCode: "forwarded",
  };
}

/** 未安装关联或没有图形环境时显示的双语安装提示。 */
export function noAssociationPrompt(locale: "zh-CN" | "en-US" = "zh-CN"): string {
  return cliMessage("xiao.cli.archive.no_association", locale);
}

/** 安装当前用户范围的 `.xar` 文件关联。 */
export async function installFileAssociation(options: FileAssociationOptions = {}): Promise<FileAssociationResult> {
  const platform = resolvePlatform(options.platform);
  const executable = await resolveXiaoExecutable(platform, options);
  if (executable === null) throw associationUnavailable(platform);
  return applyAssociation(platform, "install", executable, options);
}

/** 检查当前用户范围的 `.xar` 文件关联；不会创建目录、文件或锁。 */
export async function checkFileAssociation(options: FileAssociationOptions = {}): Promise<FileAssociationResult> {
  const platform = resolvePlatform(options.platform);
  const executable = await resolveXiaoExecutable(platform, options, true);
  return inspectAssociation(platform, executable, options);
}

/** 移除当前用户范围的 `.xar` 文件关联。 */
export async function uninstallFileAssociation(options: FileAssociationOptions = {}): Promise<FileAssociationResult> {
  const platform = resolvePlatform(options.platform);
  const executable = await resolveXiaoExecutable(platform, options, true);
  return applyAssociation(platform, "uninstall", executable, options);
}

/** 按动作调用关联接口。 */
export async function manageFileAssociation(action: AssociationAction, options: FileAssociationOptions = {}): Promise<FileAssociationResult> {
  if (action === "install") return installFileAssociation(options);
  if (action === "check") return checkFileAssociation(options);
  return uninstallFileAssociation(options);
}

async function applyAssociation(platform: AssociationPlatform, action: "install" | "uninstall", executable: string | null, options: FileAssociationOptions): Promise<FileAssociationResult> {
  if (platform === "darwin" && process.platform !== "darwin" && options.env?.XIAO_ALLOW_MACOS_ASSOCIATION !== "1") {
    throw new FileAssociationError("X11-ASSOCIATION-005", "macOS 真实 LaunchServices 操作需要 macOS 环境门控", { platform, action, gated: true });
  }
  if (platform === "win32") return windowsAssociation(action, executable, options);
  if (platform === "linux") return linuxAssociation(action, executable, options);
  return darwinAssociation(action, executable, options);
}

async function inspectAssociation(platform: AssociationPlatform, executable: string | null, options: FileAssociationOptions): Promise<FileAssociationResult> {
  if (platform === "win32") return windowsAssociation("check", executable, options);
  if (platform === "linux") return linuxAssociation("check", executable, options);
  return darwinAssociation("check", executable, options);
}

async function windowsAssociation(action: AssociationAction, executable: string | null, options: FileAssociationOptions): Promise<FileAssociationResult> {
  const runner = options.runCommand ?? defaultCommandRunner;
  const classKey = "HKCU\\Software\\Classes\\Xiao.Archive";
  const extensionKey = "HKCU\\Software\\Classes\\.xar";
  const commandKey = `${classKey}\\shell\\open\\command`;
  const command = executable === null ? null : `${quoteWindows(executable)} -xar "%1"`;
  if (action === "check") {
    const extension = await runner("reg.exe", ["QUERY", extensionKey, "/ve"]);
    const registered = extension.status === 0 && /Xiao\.Archive/iu.test(extension.stdout);
    const commandResult = await runner("reg.exe", ["QUERY", commandKey, "/ve"]);
    const target = commandResult.status === 0 ? commandResult.stdout : "";
    return associationResult(action, "win32", executable, registered, command !== null && target.includes(command), false, false, commandKey, {
      extension_key: extensionKey, command_key: commandKey, observed_command: target.trim() || null,
    });
  }
  if (action === "install") {
    if (executable === null || command === null) throw associationUnavailable("win32");
    const existingExtension = await runner("reg.exe", ["QUERY", extensionKey, "/ve"]);
    const existingCommand = await runner("reg.exe", ["QUERY", commandKey, "/ve"]);
    if (existingExtension.status === 0 && /Xiao\.Archive/iu.test(existingExtension.stdout) && existingCommand.status === 0 && existingCommand.stdout.includes(command)) {
      return associationResult(action, "win32", executable, true, true, false, false, commandKey, { command, idempotent: true });
    }
    await requireCommand(runner, "reg.exe", ["ADD", extensionKey, "/ve", "/d", "Xiao.Archive", "/f"], "X11-ASSOCIATION-002");
    await requireCommand(runner, "reg.exe", ["ADD", classKey, "/ve", "/d", "Xiao Archive", "/f"], "X11-ASSOCIATION-002");
    await requireCommand(runner, "reg.exe", ["ADD", commandKey, "/ve", "/d", command, "/f"], "X11-ASSOCIATION-002");
    return associationResult(action, "win32", executable, true, true, true, false, commandKey, { command });
  }
  const before = await runner("reg.exe", ["QUERY", extensionKey, "/ve"]);
  const removed = await runner("reg.exe", ["DELETE", classKey, "/f"]);
  const removedExtension = await runner("reg.exe", ["DELETE", extensionKey, "/f"]);
  if (removed.status !== 0 && removedExtension.status !== 0 && before.status !== 0) return associationResult(action, "win32", executable, false, false, false, false, commandKey, { already_absent: true });
  return associationResult(action, "win32", executable, false, false, true, false, commandKey, { removed: true });
}

async function linuxAssociation(action: AssociationAction, executable: string | null, options: FileAssociationOptions): Promise<FileAssociationResult> {
  const home = options.homeDir ?? options.env?.HOME ?? homedir();
  const desktopPath = join(home, ".local", "share", "applications", "xiao-xar.desktop");
  const mimePath = join(home, ".local", "share", "mime", "packages", "xiao-xar.xml");
  const command = executable === null ? null : `${quoteDesktop(executable)} -xar %f`;
  const registration = `${desktopPath};${mimePath}`;
  if (action === "check") {
    const [desktop, mime] = await Promise.all([readOptional(desktopPath), readOptional(mimePath)]);
    const installed = desktop !== null && mime !== null;
    return associationResult(action, "linux", executable, installed, installed && command !== null && desktop.includes(command), false, false, registration, {
      desktop_path: desktopPath, mime_path: mimePath, desktop_present: desktop !== null, mime_present: mime !== null,
    });
  }
  if (action === "install") {
    if (executable === null || command === null) throw associationUnavailable("linux");
    const desktop = `[Desktop Entry]\nType=Application\nName=Xiao Archive\nExec=${command}\nTerminal=true\nMimeType=application/x-xiao-xar;application/zip;\nNoDisplay=false\n`;
    const mime = `<?xml version="1.0" encoding="UTF-8"?>\n<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info"><mime-type type="application/x-xiao-xar"><comment>Xiao archive</comment><glob pattern="*.xar"/></mime-type></mime-info>\n`;
    const [existingDesktop, existingMime] = await Promise.all([readOptional(desktopPath), readOptional(mimePath)]);
    if (existingDesktop === desktop && existingMime === mime) {
      return associationResult(action, "linux", executable, true, true, false, false, registration, { desktop_path: desktopPath, mime_path: mimePath, idempotent: true });
    }
    await atomicWrite(desktopPath, desktop, 0o644);
    await atomicWrite(mimePath, mime, 0o644);
    await bestEffortCommand(options.runCommand ?? defaultCommandRunner, "update-desktop-database", [dirname(desktopPath)]);
    await bestEffortCommand(options.runCommand ?? defaultCommandRunner, "update-mime-database", [join(home, ".local", "share", "mime")]);
    return associationResult(action, "linux", executable, true, true, true, false, registration, { desktop_path: desktopPath, mime_path: mimePath });
  }
  const before = (await Promise.all([readOptional(desktopPath), readOptional(mimePath)])).some((value) => value !== null);
  await rm(desktopPath, { force: true }); await rm(mimePath, { force: true });
  await bestEffortCommand(options.runCommand ?? defaultCommandRunner, "update-desktop-database", [dirname(desktopPath)]);
  await bestEffortCommand(options.runCommand ?? defaultCommandRunner, "update-mime-database", [join(home, ".local", "share", "mime")]);
  return associationResult(action, "linux", executable, false, false, before, false, registration, { removed: before });
}

async function darwinAssociation(action: AssociationAction, executable: string | null, options: FileAssociationOptions): Promise<FileAssociationResult> {
  const home = options.homeDir ?? options.env?.HOME ?? homedir();
  const bundle = join(home, "Library", "Application Support", "Xiao", "xiao-xar.app");
  const infoPath = join(bundle, "Contents", "Info.plist");
  const launcherPath = join(bundle, "Contents", "MacOS", "xiao-xar");
  const command = executable === null ? null : `${quoteShell(executable)} -xar`;
  const registration = `${bundle};${infoPath}`;
  if (action === "check") {
    const [info, launcher] = await Promise.all([readOptional(infoPath), readOptional(launcherPath)]);
    const installed = info !== null && launcher !== null;
    return associationResult(action, "darwin", executable, installed, installed && command !== null && launcher.includes(command), false, true, registration, {
      bundle, gated: true, info_present: info !== null, launcher_present: launcher !== null,
    });
  }
  if (action === "install") {
    if (executable === null || command === null) throw associationUnavailable("darwin");
    const info = `<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.programming-language-xiao.xar</string><key>CFBundleName</key><string>Xiao Archive</string><key>CFBundleExecutable</key><string>xiao-xar</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleDocumentTypes</key><array><dict><key>CFBundleTypeExtensions</key><array><string>xar</string></array><key>CFBundleTypeRole</key><string>Viewer</string><key>LSItemContentTypes</key><array><string>com.programming-language-xiao.xar</string></array></dict></array></dict></plist>\n`;
    const launcher = `#!/bin/sh\nexec ${command} "$@"\n`;
    const [existingInfo, existingLauncher] = await Promise.all([readOptional(infoPath), readOptional(launcherPath)]);
    if (existingInfo === info && existingLauncher === launcher) {
      return associationResult(action, "darwin", executable, true, true, false, true, registration, { bundle, gated: true, idempotent: true });
    }
    await atomicWrite(infoPath, info, 0o644); await atomicWrite(launcherPath, launcher, 0o755);
    await bestEffortCommand(options.runCommand ?? defaultCommandRunner, "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister", ["-f", bundle]);
    return associationResult(action, "darwin", executable, true, true, true, true, registration, { bundle, gated: true });
  }
  const existed = (await readOptional(infoPath)) !== null || (await readOptional(launcherPath)) !== null;
  await bestEffortCommand(options.runCommand ?? defaultCommandRunner, "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister", ["-u", bundle]);
  await rm(bundle, { recursive: true, force: true });
  return associationResult(action, "darwin", executable, false, false, existed, true, registration, { removed: existed, gated: true });
}

function associationResult(action: AssociationAction, platform: AssociationPlatform, executable: string | null, installed: boolean, matches: boolean, changed: boolean, gated: boolean, registration: string, details: Record<string, unknown>): FileAssociationResult {
  return { type: "association", action, platform, installed, matches, executable, changed, gated, registration, details };
}

function resolvePlatform(platform?: AssociationPlatform): AssociationPlatform {
  const value = platform ?? process.platform;
  if (value === "win32" || value === "linux" || value === "darwin") return value;
  throw new FileAssociationError("X11-ASSOCIATION-003", `不支持的平台：${value}`, { platform: value });
}

async function resolveXiaoExecutable(platform: AssociationPlatform, options: FileAssociationOptions, allowMissing = false): Promise<string | null> {
  const env = options.env ?? process.env;
  const candidates = [options.executablePath, env.XIAO_CLI_PATH].filter((value): value is string => Boolean(value?.trim())).map((value) => resolve(options.homeDir ?? process.cwd(), value));
  for (const candidate of candidates) if (await isExecutable(candidate, platform)) return candidate;
  const result = await (options.runCommand ?? defaultCommandRunner)(platform === "win32" ? "where.exe" : "which", ["xiao"]);
  const discovered = result.stdout.split(/\r?\n/u).map((line) => line.trim()).find((line) => line.length > 0);
  if (discovered && await isExecutable(discovered, platform)) return discovered;
  return allowMissing ? null : null;
}

async function isExecutable(path: string, platform: AssociationPlatform): Promise<boolean> {
  const name = basename(path).toLocaleLowerCase("en-US");
  if (["bun", "bun.exe", "node", "node.exe"].includes(name)) return false;
  try { await access(path, platform === "win32" ? constants.F_OK : constants.X_OK); return true; } catch { return false; }
}

async function readOptional(path: string): Promise<string | null> { try { return await readFile(path, "utf8"); } catch { return null; } }

async function atomicWrite(path: string, contents: string, mode: number): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.tmp-${process.pid}-${++associationSequence}`;
  try { await writeFile(temporary, contents, { encoding: "utf8", mode }); await chmod(temporary, mode); await rename(temporary, path); }
  catch (error) { await rm(temporary, { force: true }); throw new FileAssociationError("X11-ASSOCIATION-004", `无法写入关联文件：${path}：${String(error)}`, { path }); }
}

async function requireCommand(runner: AssociationCommandRunner, command: string, args: readonly string[], code: string): Promise<void> {
  const result = await runner(command, args);
  if (result.status !== 0) throw new FileAssociationError(code, `平台关联命令失败：${command}`, { command, args, status: result.status, stderr: result.stderr });
}

async function bestEffortCommand(runner: AssociationCommandRunner, command: string, args: readonly string[]): Promise<void> { try { await runner(command, args); } catch { /* 刷新命令不存在时保留已写入的用户关联 */ } }

async function defaultCommandRunner(command: string, args: readonly string[]): Promise<AssociationCommandResult> {
  try { const result = await execFile(command, [...args], { windowsHide: true, maxBuffer: 1024 * 1024 }); return { status: 0, stdout: String(result.stdout ?? ""), stderr: String(result.stderr ?? "") }; }
  catch (error) { const value = error as { code?: number; stdout?: string; stderr?: string }; return { status: typeof value.code === "number" ? value.code : 1, stdout: value.stdout ?? "", stderr: value.stderr ?? String(error) }; }
}

function associationUnavailable(platform: AssociationPlatform): FileAssociationError {
  return new FileAssociationError("X11-ASSOCIATION-001", noAssociationPrompt("zh-CN"), { action: "install", platform });
}
function quoteWindows(path: string): string { return `"${path.replaceAll('"', '\\"')}"`; }
function quoteDesktop(path: string): string { return path.replaceAll("\\", "\\\\").replaceAll(" ", "\\ "); }
function quoteShell(path: string): string { return `'${path.replaceAll("'", "'\\''")}'`; }
