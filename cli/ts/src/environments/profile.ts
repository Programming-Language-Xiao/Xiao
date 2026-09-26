/** 显式、可逆的 Shell profile 钩子安装；不创建用户配置目录。 */

import { randomBytes } from "node:crypto";
import { constants } from "node:fs";
import { chmod, copyFile, lstat, readFile, rename, rm, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, isAbsolute, join } from "node:path";

import type { ShellName } from "../commands/parser.ts";

const START = "# >>> xiao init >>>";
const END = "# <<< xiao init <<<";

/** profile 操作结果，输出路径以便审计和手工恢复。 */
export interface ShellProfileResult {
  profile: string;
  backup: string | null;
  changed: boolean;
  action: "install" | "uninstall";
}

/** Shell 钩子安装的稳定诊断。 */
export class ShellProfileError extends Error {
  readonly details: Record<string, unknown> = {};

  /** 保留 Shell 诊断编号与 CLI 退出码，不把 profile 内容写入错误文本。 */
  constructor(readonly code: string, message: string, readonly exitCode = 64) {
    super(`${code}: ${message}`);
    this.name = "ShellProfileError";
  }
}

/** 仅在显式请求时修改已有目录内的 profile，改写前保存独立备份。 */
export async function editShellProfile(
  shell: ShellName,
  action: "install" | "uninstall",
  providedPath?: string,
  env: NodeJS.ProcessEnv = process.env,
): Promise<ShellProfileResult> {
  if (shell === "cmd") throw new ShellProfileError("X11-CLI-SHELL-002", "cmd 不支持安装钩子");
  const profile = profilePath(shell, providedPath, env);
  const result: ShellProfileResult = { profile, backup: null, changed: false, action };
  let parent;
  try {
    parent = await lstat(dirname(profile));
  } catch (error) {
    if (action === "uninstall" && isMissing(error)) return result;
    throw new ShellProfileError("X11-CLI-SHELL-003", `profile 上级目录不可用：${dirname(profile)}`);
  }
  if (!parent.isDirectory() || parent.isSymbolicLink()) {
    throw new ShellProfileError("X11-CLI-SHELL-003", `profile 上级目录不是普通目录：${dirname(profile)}`);
  }
  let original: Buffer | null = null;
  let mode = 0o600;
  try {
    const entry = await lstat(profile);
    if (!entry.isFile() || entry.isSymbolicLink()) throw new ShellProfileError("X11-CLI-SHELL-004", "profile 必须是普通文件，不能是链接");
    mode = entry.mode & 0o777;
    original = await readFile(profile);
  } catch (error) {
    if (!(isMissing(error) && action === "install")) {
      if (isMissing(error) && action === "uninstall") return result;
      if (error instanceof ShellProfileError) throw error;
      throw new ShellProfileError("X11-CLI-SHELL-006", `无法读取 profile：${profile}`, 70);
    }
  }
  let contents: string;
  try {
    contents = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(original ?? new Uint8Array());
  } catch {
    throw new ShellProfileError("X11-CLI-SHELL-004", "profile 不是有效的 UTF-8 文本");
  }
  const range = markerRange(contents);
  const newline = contents.includes("\r\n") && !contents.replaceAll("\r\n", "").includes("\n") ? "\r\n" : "\n";
  const prefix = range === null ? contents : contents.slice(0, range.start);
  const suffix = range === null ? "" : contents.slice(range.end);
  const updated = action === "uninstall"
    ? range === null ? contents : prefix + (range.start > 0 && suffix.length > 0 ? newline : "") + suffix
    : range === null
      ? contents + (contents.length > 0 ? newline : "") + markerBlock(shell, newline)
      : prefix + (range.start > 0 ? newline : "") + markerBlock(shell, newline) + suffix;
  if (updated === contents) return result;

  const nonce = randomBytes(8).toString("hex");
  const temporary = `${profile}.xiao-${nonce}.tmp`;
  if (original !== null) {
    result.backup = `${profile}.bak-${nonce}`;
    try {
      await copyFile(profile, result.backup, constants.COPYFILE_EXCL);
    } catch {
      throw new ShellProfileError("X11-CLI-SHELL-006", `无法备份 profile：${profile}`, 70);
    }
  }
  try {
    await writeFile(temporary, updated, { flag: "wx", mode });
    if (process.platform !== "win32") await chmod(temporary, mode);
    if (original !== null && !(await readFile(profile)).equals(original)) {
      throw new ShellProfileError("X11-CLI-SHELL-006", "profile 在备份后发生变化，已拒绝覆盖", 70);
    }
    await rename(temporary, profile);
  } catch (error) {
    if (error instanceof ShellProfileError) throw error;
    throw new ShellProfileError("X11-CLI-SHELL-006", `无法修改 profile；备份：${result.backup ?? "无"}`, 70);
  } finally {
    await rm(temporary, { force: true }).catch(() => {});
  }
  return { ...result, changed: true };
}

/** 在显式路径和少量固定默认路径之间选择，并拒绝歧义配置。 */
function profilePath(shell: Exclude<ShellName, "cmd">, provided: string | undefined, env: NodeJS.ProcessEnv): string {
  if (provided !== undefined) {
    if (!isAbsolute(provided) || /[\x00-\x1f\x7f]/u.test(provided)) {
      throw new ShellProfileError("X11-CLI-SHELL-003", "--profile 必须是无控制字符的绝对路径");
    }
    return provided;
  }
  if (shell === "powershell" || process.platform === "win32") {
    throw new ShellProfileError("X11-CLI-SHELL-003", "当前平台请显式提供 --profile 的绝对路径");
  }
  const home = env.HOME ?? homedir();
  if (!isAbsolute(home)) throw new ShellProfileError("X11-CLI-SHELL-003", "HOME 必须是绝对路径");
  if (shell === "fish") return join(home, ".config", "fish", "config.fish");
  return join(home, shell === "zsh" ? ".zshrc" : ".bashrc");
}

/** 只生成可整体移除的一段初始化命令。 */
function markerBlock(shell: Exclude<ShellName, "cmd">, newline: string): string {
  const command = shell === "powershell" ? "xiao shell-init powershell | Out-String | Invoke-Expression"
    : shell === "fish" ? "command xiao shell-init fish | source -"
      : `eval "$(command xiao shell-init ${shell})"`;
  return [START, command, END, ""].join(newline);
}

/** 定位唯一、完整的标记块，连同安装时追加的分隔换行返回。 */
function markerRange(contents: string): { start: number; end: number } | null {
  const begin = contents.indexOf(START);
  const finish = contents.indexOf(END);
  if (begin < 0 && finish < 0) return null;
  const afterStart = contents.slice(begin + START.length);
  const afterEnd = contents.slice(finish + END.length);
  if (begin < 0 || finish <= begin || contents.indexOf(START, begin + START.length) >= 0
    || contents.indexOf(END, finish + END.length) >= 0
    || (begin > 0 && contents[begin - 1] !== "\n")
    || contents[finish - 1] !== "\n"
    || !(afterStart.startsWith("\r\n") || afterStart.startsWith("\n"))
    || !(afterEnd === "" || afterEnd.startsWith("\r\n") || afterEnd.startsWith("\n"))) {
    throw new ShellProfileError("X11-CLI-SHELL-005", "profile 中的 xiao 标记块不完整或重复");
  }
  const lineEnd = finish + END.length;
  const after = contents.startsWith("\r\n", lineEnd) ? lineEnd + 2
    : contents.startsWith("\n", lineEnd) ? lineEnd + 1 : lineEnd;
  const before = contents.slice(0, begin).endsWith("\r\n") ? begin - 2
    : begin > 0 ? begin - 1 : begin;
  return { start: before, end: after };
}

/** 只把真正的路径缺失作为可创建或已卸载状态。 */
function isMissing(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT";
}
