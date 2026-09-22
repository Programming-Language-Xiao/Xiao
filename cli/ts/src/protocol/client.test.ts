/** 核心客户端的协议会话回归；使用内存子进程替身验证握手和 request_id 路由。 */

import { describe, expect, test } from "bun:test";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";

import { ProtocolClient } from "./client.ts";
import { decodePayload, encodeFrame } from "./codec.ts";

/** 在内存中模拟 xiao-core 的最小协议进程。 */
class FakeCore extends EventEmitter {
  readonly stdin = new PassThrough();
  readonly stdout = new PassThrough();
  readonly stderr = new PassThrough();
  exitCode: number | null = null;
  signalCode: NodeJS.Signals | null = null;
  killed = false;
  private buffer = new Uint8Array(0);

  /** 监听 stdin 并按帧返回固定结果。 */
  constructor() {
    super();
    this.stdin.on("data", (chunk: Buffer) => this.consume(chunk));
  }

  /** 模拟宿主终止核心。 */
  kill(): boolean {
    this.killed = true;
    this.exitCode = 4;
    this.stdin.end();
    this.stdout.end();
    queueMicrotask(() => this.emit("close", this.exitCode, this.signalCode));
    return true;
  }

  /** 聚合输入分块并解码请求。 */
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

  /** 返回 hello、result 或 shutdown 响应。 */
  private respond(request: Record<string, unknown>): void {
    if (request.type === "hello") {
      this.stdout.write(Buffer.from(encodeFrame({
        type: "hello", request_id: request.request_id, accepted: true, protocol_version: 1,
        core_version: 1, versions: {}, capabilities: ["run"], error: null,
      })));
    } else if (request.type === "run") {
      this.stdout.write(Buffer.from(encodeFrame({
        type: "result", request_id: request.request_id, operation: "run", exit_code: 0,
        exit_name: "success", diagnostics: [], report: null, events: [], metrics: null, value: null, artifact: null,
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

describe("协议客户端", () => {
  test("先握手，再按 request_id 取得运行结果并关闭核心", async () => {
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore();
        return fake as never;
      },
    });
    const result = await client.runSource("value = 1\n");
    expect(result.response.type).toBe("result");
    expect((result.response as { exit_code: number }).exit_code).toBe(0);
    expect(fake?.exitCode).toBe(0);
  });
});
