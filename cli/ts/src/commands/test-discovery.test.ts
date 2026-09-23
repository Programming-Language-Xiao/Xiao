/** 项目测试文件发现与确定性排序回归。 */

import { describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { discoverProjectTests, testModuleName } from "./test-discovery.ts";

describe("项目测试发现", () => {
  test("递归发现 xiao 文件并按项目相对路径排序", async () => {
    const project = await mkdtemp(join(tmpdir(), "xiao-test-discovery-"));
    try {
      await mkdir(join(project, "tests", "nested"), { recursive: true });
      await writeFile(join(project, "tests", "z-last.xiao"), "value = 1\n", "utf8");
      await writeFile(join(project, "tests", "nested", "a-first.xiao"), "value = 1\n", "utf8");
      await writeFile(join(project, "tests", "README.md"), "not a test\n", "utf8");

      const files = await discoverProjectTests(project);
      expect(files.map((file) => file.relativePath)).toEqual([
        "tests/nested/a-first.xiao",
        "tests/z-last.xiao",
      ]);
      expect(testModuleName(files[0].relativePath)).toBe("tests/nested/a-first");
    } finally {
      await rm(project, { recursive: true, force: true });
    }
  });

  test("没有 tests 目录或测试文件时返回稳定命令错误", async () => {
    const project = await mkdtemp(join(tmpdir(), "xiao-test-discovery-"));
    try {
      await expect(discoverProjectTests(project)).rejects.toMatchObject({ code: "X11-CLI-TEST-003", exitCode: 64 });
      await mkdir(join(project, "tests"));
      await expect(discoverProjectTests(project)).rejects.toMatchObject({ code: "X11-CLI-TEST-003", exitCode: 64 });
    } finally {
      await rm(project, { recursive: true, force: true });
    }
  });
});
