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
});
