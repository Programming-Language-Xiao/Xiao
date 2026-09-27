/** I2 文件载入的绑定、编码与换行契约。 */

import { expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { loadEditorFile, resolveXiaoPath } from "./file.ts";

test("载入 CRLF、CR 和 LF 并保留缩进与尾换行", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-load-"));
  try {
    const path = join(directory, "main.xiao");
    await writeFile(path, "first\r\n  second\rthird\n", "utf8");
    expect(await loadEditorFile("main.xiao", directory)).toEqual({
      path, lines: ["first", "  second", "third", ""],
    });
    expect(await loadEditorFile("missing.xiao", directory)).toEqual({
      path: join(directory, "missing.xiao"), lines: [""],
    });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("非法 UTF-8 与 NUL 分属编码和换行诊断", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-invalid-"));
  try {
    await writeFile(join(directory, "bad-utf8.xiao"), Buffer.from([0xff]));
    await writeFile(join(directory, "bad-lines.xiao"), Buffer.from("abc\0def"));
    await expect(loadEditorFile("bad-utf8.xiao", directory)).rejects.toMatchObject({ code: "X11-CLI-SAVE-003" });
    await expect(loadEditorFile("bad-lines.xiao", directory)).rejects.toMatchObject({ code: "X11-CLI-SAVE-004" });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("当前只接受 .xiao；权限与路径错误分开报告", async () => {
  expect(() => resolveXiaoPath("main.txt", process.cwd())).toThrow("X11-CLI-SAVE-001");
  await expect(loadEditorFile("main.xiao", process.cwd(), {
    readFile: async () => { throw Object.assign(new Error("denied"), { code: "EACCES" }); },
  })).rejects.toMatchObject({ code: "X11-CLI-SAVE-002" });
  await expect(loadEditorFile("main.xiao", process.cwd(), {
    readFile: async () => { throw Object.assign(new Error("directory"), { code: "EISDIR" }); },
  })).rejects.toMatchObject({ code: "X11-CLI-SAVE-001" });
});
