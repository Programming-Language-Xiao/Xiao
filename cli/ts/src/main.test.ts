/** CLI 进程入口的 IO 注入和稳定退出码回归。 */

import { describe, expect, test } from "bun:test";
import { EventEmitter } from "node:events";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { PassThrough } from "node:stream";

import { runCli, writeSafely } from "./main.ts";
import { decodePayload, encodeFrame } from "./protocol/codec.ts";

/** 在 CLI 入口测试中模拟返回失败用例的核心进程。 */
class FakeCore extends EventEmitter {
  readonly stdin = new PassThrough();
  readonly stdout = new PassThrough();
  readonly stderr = new PassThrough();
  exitCode: number | null = null;
  signalCode: NodeJS.Signals | null = null;
  killed = false;
  private buffer = new Uint8Array(0);

  /** 监听 stdin 并按帧处理 CLI 请求。 */
  constructor() {
    super();
    this.stdin.on("data", (chunk: Buffer) => this.consume(chunk));
  }

  /** 模拟宿主结束核心。 */
  kill(): boolean {
    this.killed = true;
    this.exitCode = 4;
    this.stdin.end();
    this.stdout.end();
    queueMicrotask(() => this.emit("close", this.exitCode, this.signalCode));
    return true;
  }

  /** 聚合输入分块并解码协议帧。 */
  private consume(chunk: Uint8Array): void {
    const merged = new Uint8Array(this.buffer.byteLength + chunk.byteLength);
    merged.set(this.buffer);
    merged.set(chunk, this.buffer.byteLength);
    this.buffer = merged;
    while (this.buffer.byteLength >= 8) {
      const length = Number(new DataView(this.buffer.buffer, this.buffer.byteOffset, 8).getBigUint64(0, false));
      if (this.buffer.byteLength < 8 + length) return;
      const payload = this.buffer.slice(8, 8 + length);
      this.buffer = this.buffer.slice(8 + length);
      this.respond(decodePayload<Record<string, unknown>>(payload));
    }
  }

  /** 返回握手、测试聚合结果或关闭响应。 */
  private respond(request: Record<string, unknown>): void {
    if (request.type === "hello") {
      this.stdout.write(Buffer.from(encodeFrame({
        type: "hello", request_id: request.request_id, accepted: true, protocol_version: 1,
        core_version: 1, versions: {}, capabilities: ["test"], error: null,
      })));
    } else if (request.type === "test") {
      const cases = Array.isArray(request.cases) ? request.cases : [];
      const tests = cases.map((value) => {
        const source = value as { path?: string | null; module?: string };
        return {
          path: source.path ?? source.module ?? "main",
          module: source.module ?? "main",
          exit_code: 1,
          exit_name: "source_rejected",
          diagnostics: [{
            code: "X11-TEST-001", message_id: "x11.test.failure", severity: "error",
            span: null, params: {}, message: "测试用例失败",
          }],
          report: null, events: [], metrics: null, value: null, error: null,
        };
      });
      this.stdout.write(Buffer.from(encodeFrame({
        type: "test_result", request_id: request.request_id, operation: "test", exit_code: 1,
        exit_name: "source_rejected", total: tests.length, passed: 0, failed: tests.length, tests,
      })));
    } else if (request.type === "shutdown") {
      this.stdout.write(Buffer.from(encodeFrame({ type: "shutdown", request_id: request.request_id })));
      this.exitCode = 0;
      this.stdin.end();
      this.stdout.end();
      queueMicrotask(() => this.emit("close", 0, null));
    }
  }
}

describe("CLI 入口", () => {
  test("机器模式使用项目测试协议返回的失败进程码", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-main-test-"));
    await mkdir(join(directory, "tests"));
    await writeFile(join(directory, "tests", "case.xiao"), "value = 1\n", "utf8");
    const stdout = new PassThrough();
    const stderr = new PassThrough();
    (stdout as PassThrough & { isTTY?: boolean }).isTTY = false;
    (stderr as PassThrough & { isTTY?: boolean }).isTTY = false;
    try {
      const code = await runCli(["--json", "test", "--timeout", "17"], {
        stdout,
        stderr,
        cwd: directory,
        corePath: process.execPath,
        spawnProcess: () => new FakeCore() as never,
        isTTY: false,
      });
      const value = JSON.parse(stdout.read()?.toString() ?? "{}") as {
        type?: string;
        exit_code?: number;
        tests?: Array<{ path?: string; exit_code?: number }>;
      };
      expect(code).toBe(1);
      expect(value.type).toBe("test_result");
      expect(value.exit_code).toBe(1);
      expect(value.tests?.[0]).toMatchObject({ path: "tests/case.xiao", exit_code: 1 });
      expect(stderr.read()).toBeNull();
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
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
