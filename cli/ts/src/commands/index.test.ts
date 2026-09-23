/** 命令执行器的取消信号接线回归。 */

import { describe, expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
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
});
