/** I2 文件载入的绑定、编码与换行契约。 */

import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { loadEditorFile, resolveXiaoPath, saveEditorFile } from "./file.ts";

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

test("合法 UTF-8 BOM 可读，保存后按无 BOM 约定写出", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-bom-"));
  try {
    const path = join(directory, "main.xiao");
    await writeFile(path, Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from("name = 1\r\n")]));
    const loaded = await loadEditorFile("main.xiao", directory);
    expect(loaded.lines).toEqual(["name = 1", ""]);
    await saveEditorFile(path, directory, loaded.lines.join("\n"));
    expect(await readFile(path)).toEqual(Buffer.from("name = 1\n"));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("非法 UTF-8 与破坏行结构的控制字节分属编码和换行诊断", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-invalid-"));
  try {
    await writeFile(join(directory, "bad-utf8.xiao"), Buffer.from([0xff]));
    await writeFile(join(directory, "bad-lines.xiao"), Buffer.from("abc\0def"));
    await writeFile(join(directory, "bad-escape.xiao"), Buffer.from("abc\u001b[2J"));
    await writeFile(join(directory, "bad-separator.xiao"), "abc\u2028def", "utf8");
    await expect(loadEditorFile("bad-utf8.xiao", directory)).rejects.toMatchObject({ code: "X11-CLI-SAVE-003" });
    await expect(loadEditorFile("bad-lines.xiao", directory)).rejects.toMatchObject({ code: "X11-CLI-SAVE-004" });
    await expect(loadEditorFile("bad-escape.xiao", directory)).rejects.toMatchObject({ code: "X11-CLI-SAVE-004" });
    await expect(loadEditorFile("bad-separator.xiao", directory)).rejects.toMatchObject({ code: "X11-CLI-SAVE-004" });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("当前只接受 .xiao；权限与路径错误分开报告", async () => {
  expect(() => resolveXiaoPath("main.txt", process.cwd())).toThrow("X11-CLI-SAVE-001");
  for (const separator of ["\u001b", "\u0085", "\u2028", "\u2029"]) {
    const path = `bad${separator}.xiao`;
    expect(() => resolveXiaoPath(path, process.cwd())).toThrow("X11-CLI-SAVE-001");
    try {
      resolveXiaoPath(path, process.cwd());
    } catch (error) {
      expect((error as Error).message).not.toContain(separator);
    }
  }
  await expect(loadEditorFile("main.xiao", process.cwd(), {
    readFile: async () => { throw Object.assign(new Error("denied"), { code: "EACCES" }); },
  })).rejects.toMatchObject({ code: "X11-CLI-SAVE-002" });
  await expect(loadEditorFile("main.xiao", process.cwd(), {
    readFile: async () => { throw Object.assign(new Error("directory"), { code: "EISDIR" }); },
  })).rejects.toMatchObject({ code: "X11-CLI-SAVE-001" });
});

test("写出固定 UTF-8 无 BOM 和 LF，写前拒绝控制字符与非法 Unicode", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-save-"));
  try {
    const path = join(directory, "main.xiao");
    expect(await saveEditorFile("main.xiao", directory, "一\r\n二\r三")).toBe(path);
    expect(await readFile(path)).toEqual(Buffer.from("一\n二\n三", "utf8"));
    await expect(saveEditorFile("main.xiao", directory, "a\0b")).rejects.toMatchObject({ code: "X11-CLI-SAVE-004" });
    await expect(saveEditorFile("main.xiao", directory, "a\u001bb")).rejects.toMatchObject({ code: "X11-CLI-SAVE-004" });
    await expect(saveEditorFile("main.xiao", directory, "\ud800")).rejects.toMatchObject({ code: "X11-CLI-SAVE-003" });
    expect(await readFile(path)).toEqual(Buffer.from("一\n二\n三", "utf8"));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
