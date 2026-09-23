/** 命令执行器的取消信号接线回归。 */

import { describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { executeCommand } from "./index.ts";
import { parseArguments } from "./parser.ts";

describe("命令取消接线", () => {
  test("预取消信号沿 runSource 传递并在启动核心前返回稳定错误", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-cancel-"));
    const sourcePath = join(directory, "main.xiao");
    await writeFile(sourcePath, "value = 1\n", "utf8");
    try {
      const result = await executeCommand(parseArguments(["run", sourcePath]), {
        cwd: directory,
        signal: AbortSignal.abort(),
      });
      expect(result.exitCode).toBe(2);
      expect(result.stderr).toContain("X11-PROTOCOL-005");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("项目没有测试文件时返回 CLI 自身的 usage 诊断", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-test-empty-"));
    await mkdir(join(directory, "tests"));
    try {
      const result = await executeCommand(parseArguments(["test", directory]), { cwd: directory });
      expect(result.exitCode).toBe(64);
      expect(result.stderr).toContain("X11-CLI-TEST-003");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("项目测试源码读取失败返回 CLI 文件诊断，不伪装成核心失败", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-test-file-"));
    const sourcePath = join(directory, "tests", "bad.xiao");
    await mkdir(join(directory, "tests"));
    await writeFile(sourcePath, Buffer.from([0xff, 0xfe]));
    try {
      const result = await executeCommand(parseArguments(["test", directory]), { cwd: directory });
      expect(result.exitCode).toBe(64);
      expect(result.stderr).toContain("X11-CLI-FILE-001");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
});
