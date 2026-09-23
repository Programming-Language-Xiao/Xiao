/** X0-A Rust/TypeScript 共享 fixture 契约测试。 */

import { describe, expect, test } from "bun:test";

import { decodeFrame, encodeFrame, FRAME_LENGTH_BYTES, readFrame } from "./codec.ts";
import { validateMessage } from "./messages.ts";

const fixtureDirectory = new URL("../../../../tests/spec/11x0-protocol/", import.meta.url);

/** 读取 tests/spec 中 Rust 与 TypeScript 共用的 JSON 样本。 */
async function fixture(name: string): Promise<unknown> {
  const path = new URL(name, fixtureDirectory);
  return JSON.parse(await Bun.file(path).text()) as unknown;
}

describe("X0-A 长度前缀协议", () => {
  test("共享 hello fixture 可编码、解码并验证消息形状", async () => {
    const value = validateMessage(await fixture("hello-request.json"));
    const frame = encodeFrame(value);
    expect(frame.byteLength).toBeGreaterThan(FRAME_LENGTH_BYTES);
    expect(decodeFrame<unknown>(frame)).toEqual(value);
  });

  test("共享 response fixture 可被解析，且前缀只计算 JSON 负载", async () => {
    const value = validateMessage(await fixture("run-response.json"));
    const frame = encodeFrame(value);
    const parsed = readFrame(frame);
    expect(parsed?.nextOffset).toBe(frame.byteLength);
    expect(parsed && new TextDecoder().decode(parsed.payload)).toBe(JSON.stringify(value));
  });

  test("截断长度字段与过大长度都返回稳定错误码", () => {
    expect(() => readFrame(new Uint8Array([0, 1]))).toThrow("X11-PROTOCOL-001");
    const oversized = new Uint8Array(FRAME_LENGTH_BYTES);
    new DataView(oversized.buffer).setBigUint64(0, BigInt(16 * 1024 * 1024 + 1), false);
    expect(() => readFrame(oversized)).toThrow("X11-PROTOCOL-001");
  });

  test("debug 请求保留诊断等级和聚焦规则", async () => {
    const value = validateMessage(await fixture("debug-run-request.json"));
    expect(value).toMatchObject({
      type: "run",
      optimization: {
        debug: true,
        diagnostics: {
          terminal_level: "info",
          file_level: "debug",
          focus: [{ module: "app.net", mirror: false }],
        },
      },
    });
    expect(decodeFrame<unknown>(encodeFrame(value))).toEqual(value);
  });

  test("项目测试请求保留发现顺序、每用例源码和超时", async () => {
    const value = validateMessage(await fixture("test-request.json"));
    expect(value).toMatchObject({
      type: "test",
      cases: [
        { path: "tests/nested/a-first.xiao", module: "tests/nested/a-first" },
        { path: "tests/z-last.xiao", module: "tests/z-last" },
      ],
      options: { timeout_ms: 5000, checkpoints_enabled: true },
    });
    expect(decodeFrame<unknown>(encodeFrame(value))).toEqual(value);
  });

  test("项目测试响应保留聚合退出码和逐用例结构化结果", async () => {
    const value = validateMessage(await fixture("test-response.json"));
    expect(value).toMatchObject({
      type: "test_result",
      operation: "test",
      exit_code: 1,
      total: 2,
      passed: 1,
      failed: 1,
      tests: [
        { path: "tests/nested/a-first.xiao", exit_code: 0, error: null },
        { path: "tests/z-last.xiao", exit_code: 1, error: { code: "X11-PROTOCOL-002" } },
      ],
    });
    expect(decodeFrame<unknown>(encodeFrame(value))).toEqual(value);
  });
});
