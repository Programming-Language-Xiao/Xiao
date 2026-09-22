/** CLI 进程入口的 IO 注入和稳定退出码回归。 */

import { describe, expect, test } from "bun:test";
import { PassThrough } from "node:stream";

import { runCli, writeSafely } from "./main.ts";

describe("CLI 入口", () => {
  test("机器模式保留未实现命令的稳定进程码", async () => {
    const stdout = new PassThrough();
    const stderr = new PassThrough();
    (stdout as PassThrough & { isTTY?: boolean }).isTTY = false;
    (stderr as PassThrough & { isTTY?: boolean }).isTTY = false;
    const code = await runCli(["--json", "test"], { stdout, stderr, isTTY: false });
    expect(code).toBe(64);
    expect(JSON.parse(stdout.read()?.toString() ?? "{}").code).toBe("X11-CLI-TEST-001");
    expect(stderr.read()).toBeNull();
  });

  test("管道接收端关闭时吞掉 EPIPE", async () => {
    const stream = {
      write: (_text: string, callback: (error?: Error | null) => void) => {
        const error = Object.assign(new Error("closed"), { code: "EPIPE" });
        callback(error);
        return false;
      },
    } as unknown as NodeJS.WritableStream;
    await expect(writeSafely(stream, "output\n")).resolves.toBeUndefined();
  });
});
