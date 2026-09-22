/** 配置结构化写回和原子目标发现回归。 */

import { describe, expect, test } from "bun:test";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { chmod, mkdir, readdir, rename, stat, writeFile as fsWriteFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { CliConfigError, findProjectConfig, readConfigValue, writeConfigValue } from "./editor.ts";

describe("config.xiao CLI 写回", () => {
  test("只修改目标字面量并保留注释、无关字段和换行", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-config-"));
    try {
      const path = join(directory, "config.xiao");
      await Bun.write(path, "# keep\r\n[CLI]\r\ngit = { summary = false, other = 1 } # note\r\n[language]\r\nlocale = \"zh\"\r\n");
      await writeConfigValue("project", "CLI.git.summary", "true", { cwd: directory });
      const text = await readFile(path, "utf8");
      expect(text).toContain("# keep\r\n");
      expect(text).toContain("summary = true, other = 1 } # note");
      await writeConfigValue("project", "language.locale", "en", { cwd: directory });
      expect(await readConfigValue("project", "language.locale", { cwd: directory })).toBe("en-US");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("非法路径和值在写文件前拒绝", async () => {
    await expect(writeConfigValue("project", "unknown.path", "true", { cwd: tmpdir() })).rejects.toMatchObject({ code: "X11-CONFIG-001" });
    await expect(writeConfigValue("project", "CLI.git.summary", "yes", { cwd: tmpdir() })).rejects.toMatchObject({ code: "X11-CONFIG-002" });
  });

  test("发现非规范大小写配置文件时给出稳定诊断", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-config-case-"));
    try {
      await Bun.write(join(directory, "Config.xiao"), "[CLI]\ngit = { summary = false }\n");
      await expect(findProjectConfig(directory)).rejects.toBeInstanceOf(CliConfigError);
      await expect(findProjectConfig(directory)).rejects.toMatchObject({ code: "X11-CONFIG-005" });
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("临时文件写入中断返回稳定错误且不覆盖原文", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-config-write-"));
    try {
      const path = join(directory, "config.xiao");
      const original = "[CLI]\ngit = { summary = false }\n";
      await Bun.write(path, original);
      const failingFs = {
        readFile: (file: string, encoding: "utf8") => readFile(file, encoding),
        writeFile: async () => { throw new Error("simulated interruption"); },
        rename,
        rm,
        mkdir: (file: string, options: { recursive: true }) => mkdir(file, options).then(() => undefined),
        stat,
        readdir,
        chmod,
      };
      await expect(writeConfigValue("project", "CLI.git.summary", "true", { cwd: directory, fs: failingFs })).rejects.toMatchObject({ code: "X11-CONFIG-006" });
      expect(await readFile(path, "utf8")).toBe(original);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("损坏的静态容器不会被 CLI 悄悄改写", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-config-malformed-"));
    try {
      await Bun.write(join(directory, "config.xiao"), "[CLI]\ngit = { summary = false\n");
      await expect(writeConfigValue("project", "CLI.git.summary", "true", { cwd: directory })).rejects.toMatchObject({ code: "X11-CONFIG-004" });
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
});
