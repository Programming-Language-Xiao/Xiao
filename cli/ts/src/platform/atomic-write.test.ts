/** 配置与源码共享的原子替换契约。 */

import { expect, test } from "bun:test";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { atomicWriteFile, nativeAtomicWriteFileSystem } from "./atomic-write.ts";

test("临时文件使用目标前缀，成功替换后不遗留文件", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-atomic-"));
  try {
    const path = join(directory, "example.xiao");
    await writeFile(path, "old", "utf8");
    const written: string[] = [];
    await atomicWriteFile(path, "新内容\n", {
      ...nativeAtomicWriteFileSystem,
      writeFile: async (file, data, encoding) => {
        written.push(file);
        await nativeAtomicWriteFileSystem.writeFile(file, data, encoding);
      },
    });
    expect(written).toHaveLength(1);
    expect(written[0]).toContain(".example.xiao.tmp-");
    expect(await readFile(path, "utf8")).toBe("新内容\n");
    expect(await readdir(directory)).toEqual(["example.xiao"]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("覆盖替换中途失败回滚旧文件并清理临时文件", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-atomic-rollback-"));
  try {
    const path = join(directory, "example.xiao");
    await writeFile(path, "old", "utf8");
    let replacements = 0;
    await expect(atomicWriteFile(path, "new", {
      ...nativeAtomicWriteFileSystem,
      rename: async (source, target) => {
        if (source.includes(".tmp-") && target === path) {
          replacements += 1;
          throw Object.assign(new Error("替换失败"), { code: replacements === 1 ? "EEXIST" : "EIO" });
        }
        await nativeAtomicWriteFileSystem.rename(source, target);
      },
    })).rejects.toMatchObject({ code: "EIO" });
    expect(replacements).toBe(2);
    expect(await readFile(path, "utf8")).toBe("old");
    expect(await readdir(directory)).toEqual(["example.xiao"]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("新文件已到位后备份清理失败不再误报保存失败", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-atomic-cleanup-"));
  try {
    const path = join(directory, "example.xiao");
    await writeFile(path, "old", "utf8");
    let replacements = 0;
    await atomicWriteFile(path, "new", {
      ...nativeAtomicWriteFileSystem,
      rename: async (source, target) => {
        if (source.includes(".tmp-") && target === path && replacements++ === 0) {
          throw Object.assign(new Error("已存在"), { code: "EEXIST" });
        }
        await nativeAtomicWriteFileSystem.rename(source, target);
      },
      rm: async (file, options) => {
        if (file.includes(".xiao-backup-")) throw Object.assign(new Error("清理失败"), { code: "EACCES" });
        await nativeAtomicWriteFileSystem.rm(file, options);
      },
    });
    expect(await readFile(path, "utf8")).toBe("new");
    const backup = (await readdir(directory)).find((entry) => entry.includes(".xiao-backup-"));
    expect(backup).toBeDefined();
    expect(await readFile(join(directory, backup!), "utf8")).toBe("old");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("替换和回滚都失败时指出原文件的备份位置", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-atomic-recovery-"));
  try {
    const path = join(directory, "example.xiao");
    await writeFile(path, "old", "utf8");
    let replacements = 0;
    let failure: unknown;
    try {
      await atomicWriteFile(path, "new", {
        ...nativeAtomicWriteFileSystem,
        rename: async (source, target) => {
          if (source.includes(".tmp-") && target === path) {
            replacements += 1;
            throw Object.assign(new Error("替换失败"), { code: replacements === 1 ? "EEXIST" : "EIO" });
          }
          if (source.includes(".xiao-backup-") && target === path) {
            throw Object.assign(new Error("回滚失败"), { code: "EACCES" });
          }
          await nativeAtomicWriteFileSystem.rename(source, target);
        },
      });
    } catch (error) {
      failure = error;
    }
    const backup = (await readdir(directory)).find((entry) => entry.includes(".xiao-backup-"));
    expect(backup).toBeDefined();
    expect(await readFile(join(directory, backup!), "utf8")).toBe("old");
    expect(failure).toBeInstanceOf(Error);
    expect((failure as Error).message).toContain(join(directory, backup!));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
