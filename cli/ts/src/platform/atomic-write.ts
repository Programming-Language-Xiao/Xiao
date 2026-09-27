/** CLI 文件写回共用的同目录原子替换边界。 */

import { chmod, mkdir, rename, rm, stat, writeFile } from "node:fs/promises";
import { basename, dirname, join } from "node:path";

/** 原子写所需的可注入文件系统操作。 */
export interface AtomicWriteFileSystem {
  writeFile(path: string, data: string, encoding: "utf8"): Promise<void>;
  rename(oldPath: string, newPath: string): Promise<void>;
  rm(path: string, options?: { force?: boolean }): Promise<void>;
  mkdir(path: string, options: { recursive: true }): Promise<void>;
  stat(path: string): Promise<{ mode: number }>;
  chmod(path: string, mode: number): Promise<void>;
}

/** 默认 Node/Bun 文件系统实现。 */
export const nativeAtomicWriteFileSystem: AtomicWriteFileSystem = {
  writeFile: (path, data, encoding) => writeFile(path, data, encoding),
  rename,
  rm,
  mkdir: (path, options) => mkdir(path, options).then(() => undefined),
  stat,
  chmod,
};

/** 在目标同目录写临时文件后替换，Windows 覆盖失败时备份并回滚。 */
export async function atomicWriteFile(
  path: string,
  text: string,
  fileSystem: AtomicWriteFileSystem = nativeAtomicWriteFileSystem,
  tempPrefix = basename(path),
): Promise<void> {
  const directory = dirname(path);
  await fileSystem.mkdir(directory, { recursive: true });
  let mode: number | undefined;
  try {
    mode = (await fileSystem.stat(path)).mode;
  } catch (error) {
    if (!isMissing(error)) throw error;
  }
  const temp = join(directory, `.${tempPrefix}.tmp-${process.pid}-${Date.now()}-${Math.random().toString(36).slice(2)}`);
  try {
    await fileSystem.writeFile(temp, text, "utf8");
    if (mode !== undefined) await fileSystem.chmod(temp, mode);
    try {
      await fileSystem.rename(temp, path);
    } catch (error) {
      if (mode === undefined || !isAlreadyExists(error)) throw error;
      const backup = `${path}.xiao-backup-${process.pid}-${Date.now()}`;
      await fileSystem.rename(path, backup);
      try {
        await fileSystem.rename(temp, path);
      } catch (replaceError) {
        try {
          await fileSystem.rename(backup, path);
        } catch (rollbackError) {
          throw new Error(`原子替换与回滚均失败；原文件保留在 ${backup}，请手动恢复（替换：${String(replaceError)}；回滚：${String(rollbackError)}）`);
        }
        throw replaceError;
      }
      await fileSystem.rm(backup, { force: true }).catch(() => undefined);
    }
  } finally {
    await fileSystem.rm(temp, { force: true }).catch(() => undefined);
  }
}

/** 判断文件尚不存在。 */
function isMissing(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error && (error as { code?: string }).code === "ENOENT";
}

/** 判断重命名是否被现有目标阻挡。 */
function isAlreadyExists(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error && ["EEXIST", "EPERM", "ENOTEMPTY"].includes((error as { code?: string }).code ?? "");
}
