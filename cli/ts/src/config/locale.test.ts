/** 生效语言的配置优先级、别名及错误边界。 */

import { describe, expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { readConfigValue } from "./editor.ts";
import { resolveEffectiveLocale } from "./locale.ts";

describe("11C 生效语言", () => {
  test("项目覆盖全局、别名规范化、缺省为中文", async () => {
    const cwd = await mkdtemp(join(tmpdir(), "xiao-locale-"));
    const globalPath = join(cwd, "global.xiao");
    try {
      const options = { cwd, globalPath };
      expect(await resolveEffectiveLocale(options)).toEqual({ tag: "zh-CN", source: "default" });
      await writeFile(globalPath, '[language]\nlocale = "EN-us"\n');
      expect(await resolveEffectiveLocale(options)).toEqual({ tag: "en-US", source: "global" });
      await writeFile(join(cwd, "config.xiao"), '[language]\nlocale = "ZH-cn"\n');
      const context = await resolveEffectiveLocale(options);
      expect(context).toEqual({ tag: "zh-CN", source: "project" });
      expect(Object.isFrozen(context)).toBe(true);
    } finally {
      await rm(cwd, { recursive: true, force: true });
    }
  });

  test("拒绝非字符串、非法标签及重复键", async () => {
    const cwd = await mkdtemp(join(tmpdir(), "xiao-locale-"));
    const path = join(cwd, "config.xiao");
    try {
      for (const [text, code] of [
        ["[language]\nlocale = true\n", "X11-CONFIG-002"],
        ['[language]\nlocale = "fr"\n', "X11-CONFIG-002"],
        ['[language]\nlocale = "en"\nlocale = "zh"\n', "X11-CONFIG-004"],
        ['[language]\nlocale = "en"\n[language]\nlocale = "zh"\n', "X11-CONFIG-004"],
        ['[Language]\nlocale = "en"\n', "X11-CONFIG-005"],
        ['[language]\nLocale = "en"\n', "X11-CONFIG-005"],
        ['[language]\nlocale = "en"\nlocale = "en"\n', "X11-CONFIG-004"],
        ['[language]\nlocale = "en\n', "X11-CONFIG-004"],
      ]) {
        await writeFile(path, text);
        await expect(readConfigValue("project", "language.locale", { cwd })).rejects.toMatchObject({
          code, path, details: { key: "language.locale", scope: "project", supported_locales: ["zh-CN", "en-US"] },
        });
      }
    } finally {
      await rm(cwd, { recursive: true, force: true });
    }
  });

  test("全局文件无效时诊断指向全局来源，项目覆盖时不解析它", async () => {
    const cwd = await mkdtemp(join(tmpdir(), "xiao-locale-"));
    const globalPath = join(cwd, "global.xiao");
    try {
      await writeFile(globalPath, "[language]\nlocale = 12\n");
      await expect(resolveEffectiveLocale({ cwd, globalPath })).rejects.toMatchObject({
        code: "X11-CONFIG-002", path: globalPath, details: { key: "language.locale", scope: "global" },
      });
      await writeFile(join(cwd, "config.xiao"), '[language]\nlocale = "en"\n');
      expect(await resolveEffectiveLocale({ cwd, globalPath })).toEqual({ tag: "en-US", source: "project" });
    } finally {
      await rm(cwd, { recursive: true, force: true });
    }
  });
});
