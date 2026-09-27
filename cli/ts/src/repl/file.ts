/** 多行编辑文件的路径、字节读取和稳定诊断边界。 */

import { readFile } from "node:fs/promises";
import { extname, resolve } from "node:path";

import { atomicWriteFile, nativeAtomicWriteFileSystem, type AtomicWriteFileSystem } from "../platform/atomic-write.ts";
import { MAX_LOGICAL_LINES } from "./editor.ts";

/** 文件读取所需的可注入操作。 */
export interface ReplReadFileSystem {
  readFile(path: string): Promise<Buffer>;
}

/** 文件读写边界复用共享原子写所需操作。 */
export interface ReplFileSystem extends ReplReadFileSystem, AtomicWriteFileSystem {}

/** 默认的 Node/Bun 文件系统实现。 */
export const nativeReplFileSystem: ReplFileSystem = { ...nativeAtomicWriteFileSystem, readFile };

/** 文件打开与保存的诊断编号。 */
export const REPL_FILE_ERROR = {
  path: "X11-CLI-SAVE-001",
  permission: "X11-CLI-SAVE-002",
  encoding: "X11-CLI-SAVE-003",
  newline: "X11-CLI-SAVE-004",
  io: "X11-CLI-SAVE-005",
} as const;

const INVALID_SOURCE_LINE = /[\x00-\x08\x0b\x0c\x0e-\x1f\x7f\u0085\u2028\u2029]/u;

/** 用户可操作的文件读写错误。 */
export class ReplFileError extends Error {
  readonly exitCode = 74;
  readonly details: Record<string, unknown>;

  /** 保存稳定编号、目标路径和底层原因。 */
  constructor(readonly code: string, readonly path: string, reason: string, cause?: unknown) {
    super(`${code}: ${safePath(path)}：${reason}`);
    this.name = "ReplFileError";
    this.details = { path, cause: fileErrorCode(cause) };
  }
}

/** 当前只允许 `.xiao` 文件；其他扩展名是后续 I2 扩展点。 */
export function resolveXiaoPath(path: string, cwd: string): string {
  if (path.trim() === "" || /[\x00-\x1f\x7f]/u.test(path)) {
    throw new ReplFileError(REPL_FILE_ERROR.path, path, "路径为空或包含控制字符，请输入有效的 .xiao 文件路径");
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
  fileSystem: ReplReadFileSystem = nativeReplFileSystem,
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
  if (INVALID_SOURCE_LINE.test(text)) {
    throw new ReplFileError(REPL_FILE_ERROR.newline, absolute, "包含不支持的控制或分隔字符，无法保持源码行结构，请先移除");
  }
  const lines = text.replaceAll("\r\n", "\n").replaceAll("\r", "\n").split("\n");
  if (lines.length > MAX_LOGICAL_LINES) {
    throw new ReplFileError(REPL_FILE_ERROR.path, absolute, `超过 ${MAX_LOGICAL_LINES} 行编辑上限，请拆分文件`);
  }
  return { path: absolute, lines };
}

/** 以 UTF-8 无 BOM 和 LF 原子保存源码，并返回规范绑定路径。 */
export async function saveEditorFile(
  path: string,
  cwd: string,
  source: string,
  fileSystem: AtomicWriteFileSystem = nativeReplFileSystem,
): Promise<string> {
  const absolute = resolveXiaoPath(path, cwd);
  if (INVALID_SOURCE_LINE.test(source)) {
    throw new ReplFileError(REPL_FILE_ERROR.newline, absolute, "源码包含不支持的控制或分隔字符，无法保持行结构，请先移除");
  }
  const normalized = source.replaceAll("\r\n", "\n").replaceAll("\r", "\n");
  const encoded = new TextEncoder().encode(normalized);
  if (new TextDecoder("utf-8", { fatal: true }).decode(encoded) !== normalized) {
    throw new ReplFileError(REPL_FILE_ERROR.encoding, absolute, "源码包含无效 Unicode 字符，请先修正编码");
  }
  try {
    await atomicWriteFile(absolute, normalized, fileSystem);
  } catch (error) {
    throw fileOperationError(absolute, "保存", error);
  }
  return absolute;
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

/** 只转义终端控制字符，保留 Windows 路径分隔符的可读性。 */
function safePath(path: string): string {
  return path.replace(/[\x00-\x1f\x7f]/gu, (character) => JSON.stringify(character).slice(1, -1));
}
