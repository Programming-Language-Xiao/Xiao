/** 多行编辑文件的路径、字节读取和稳定诊断边界。 */

import { readFile } from "node:fs/promises";
import { extname, resolve } from "node:path";

import { MAX_LOGICAL_LINES } from "./editor.ts";

/** 文件读取所需的可注入操作。 */
export interface ReplReadFileSystem {
  readFile(path: string): Promise<Buffer>;
}

/** 默认的 Node/Bun 字节读取。 */
export const nativeReplReadFileSystem: ReplReadFileSystem = { readFile };

/** 文件打开与保存的诊断编号。 */
export const REPL_FILE_ERROR = {
  path: "X11-CLI-SAVE-001",
  permission: "X11-CLI-SAVE-002",
  encoding: "X11-CLI-SAVE-003",
  newline: "X11-CLI-SAVE-004",
  io: "X11-CLI-SAVE-005",
} as const;

/** 用户可操作的文件读写错误。 */
export class ReplFileError extends Error {
  readonly exitCode = 74;
  readonly details: Record<string, unknown>;

  /** 保存稳定编号、目标路径和底层原因。 */
  constructor(readonly code: string, readonly path: string, reason: string, cause?: unknown) {
    super(`${code}: ${path}：${reason}`);
    this.name = "ReplFileError";
    this.details = { path, cause: fileErrorCode(cause) };
  }
}

/** 当前只允许 `.xiao` 文件；其他扩展名是后续 I2 扩展点。 */
export function resolveXiaoPath(path: string, cwd: string): string {
  if (path.trim() === "" || path.includes("\0")) {
    throw new ReplFileError(REPL_FILE_ERROR.path, path, "路径为空或包含 NUL，请输入有效的 .xiao 文件路径");
  }
  const absolute = resolve(cwd, path);
  if (extname(absolute) !== ".xiao") {
    throw new ReplFileError(REPL_FILE_ERROR.path, absolute, "当前只支持 .xiao 文件；其他扩展名留待后续扩展");
  }
  return absolute;
}

/** 读取现有文件，缺失文件返回已绑定的空逻辑缓冲区。 */
export async function loadEditorFile(
  path: string,
  cwd: string,
  fileSystem: ReplReadFileSystem = nativeReplReadFileSystem,
): Promise<{ path: string; lines: string[] }> {
  const absolute = resolveXiaoPath(path, cwd);
  let bytes: Buffer;
  try {
    bytes = await fileSystem.readFile(absolute);
  } catch (error) {
    if (fileErrorCode(error) === "ENOENT") return { path: absolute, lines: [""] };
    throw fileOperationError(absolute, "读取", error);
  }
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch (error) {
    throw new ReplFileError(REPL_FILE_ERROR.encoding, absolute, "不是合法 UTF-8，请先转换编码后再打开", error);
  }
  if (text.includes("\0")) {
    throw new ReplFileError(REPL_FILE_ERROR.newline, absolute, "包含 NUL 字节，无法保持源码行结构，请先移除");
  }
  const lines = text.replaceAll("\r\n", "\n").replaceAll("\r", "\n").split("\n");
  if (lines.length > MAX_LOGICAL_LINES) {
    throw new ReplFileError(REPL_FILE_ERROR.path, absolute, `超过 ${MAX_LOGICAL_LINES} 行编辑上限，请拆分文件`);
  }
  return { path: absolute, lines };
}

/** 将底层路径或权限失败归为可操作的稳定诊断。 */
export function fileOperationError(path: string, action: "读取" | "保存", error: unknown): ReplFileError {
  const code = fileErrorCode(error);
  if (["EACCES", "EPERM", "EROFS"].includes(code ?? "")) {
    return new ReplFileError(REPL_FILE_ERROR.permission, path, `${action}权限不足，请检查文件与目录权限`, error);
  }
  if (["ENOENT", "ENOTDIR", "EISDIR", "ENAMETOOLONG", "EINVAL"].includes(code ?? "")) {
    return new ReplFileError(REPL_FILE_ERROR.path, path, `${action}路径无效，请检查目标及父目录`, error);
  }
  return new ReplFileError(REPL_FILE_ERROR.io, path, `${action}失败，请检查磁盘和文件系统状态：${String(error)}`, error);
}

/** 读取 Node 文件系统错误码。 */
function fileErrorCode(error: unknown): string | null {
  return typeof error === "object" && error !== null && "code" in error && typeof error.code === "string"
    ? error.code : null;
}
