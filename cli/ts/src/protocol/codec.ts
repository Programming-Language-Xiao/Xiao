/** X0-A 长度前缀 JSON 编解码；支持管道、非 TTY 和机器可读输出。 */

import { FRAME_ERROR_CODE, MAX_FRAME_BYTES } from "./constants.ts";

/** 固定长度字段的字节数。 */
export const FRAME_LENGTH_BYTES = 8;

/** 单帧 JSON 负载的最大字节数。 */
export { MAX_FRAME_BYTES, FRAME_ERROR_CODE };

/** 帧解析失败；调用方应使用 code 而不是解析 message。 */
export class ProtocolFrameError extends Error {
  /** 稳定机器错误码。 */
  readonly code: string;

  /** 创建帧错误。 */
  constructor(code: string, message: string) {
    super(`${code}: ${message}`);
    this.name = "ProtocolFrameError";
    this.code = code;
  }
}

/** 一次读取结果，包含下一帧起点。 */
export interface FrameReadResult {
  /** 不含长度字段的 JSON 字节。 */
  payload: Uint8Array;
  /** 下一帧在输入缓冲区中的偏移。 */
  nextOffset: number;
}

/** 把 JSON 值编码为 8 字节大端长度前缀帧。 */
export function encodeFrame(value: unknown): Uint8Array {
  let jsonText: string;
  try {
    const serialized = JSON.stringify(value);
    if (serialized === undefined) throw new TypeError("值不可序列化");
    jsonText = serialized;
  } catch (error) {
    throw new ProtocolFrameError(FRAME_ERROR_CODE, `JSON 编码失败：${String(error)}`);
  }
  const payload = new TextEncoder().encode(jsonText);
  if (payload.byteLength > MAX_FRAME_BYTES) {
    throw new ProtocolFrameError(FRAME_ERROR_CODE, "帧长度超过上限");
  }
  const frame = new Uint8Array(FRAME_LENGTH_BYTES + payload.byteLength);
  new DataView(frame.buffer).setBigUint64(0, BigInt(payload.byteLength), false);
  frame.set(payload, FRAME_LENGTH_BYTES);
  return frame;
}

/** 读取一个完整帧；输入不足时返回稳定错误。 */
export function readFrame(input: Uint8Array, offset = 0): FrameReadResult | null {
  if (offset === input.byteLength) return null;
  if (input.byteLength - offset < FRAME_LENGTH_BYTES) {
    throw new ProtocolFrameError(FRAME_ERROR_CODE, "长度字段被截断");
  }
  const view = new DataView(input.buffer, input.byteOffset + offset, FRAME_LENGTH_BYTES);
  const length = view.getBigUint64(0, false);
  if (length > BigInt(MAX_FRAME_BYTES)) {
    throw new ProtocolFrameError(FRAME_ERROR_CODE, "帧长度超过上限");
  }
  const payloadLength = Number(length);
  const payloadStart = offset + FRAME_LENGTH_BYTES;
  const nextOffset = payloadStart + payloadLength;
  if (nextOffset > input.byteLength) {
    throw new ProtocolFrameError(FRAME_ERROR_CODE, "JSON 负载被截断");
  }
  return { payload: input.slice(payloadStart, nextOffset), nextOffset };
}

/** 解码一个完整帧，并拒绝尾随字节。 */
export function decodeFrame<T = unknown>(input: Uint8Array): T {
  const result = readFrame(input);
  if (result === null || result.nextOffset !== input.byteLength) {
    throw new ProtocolFrameError(FRAME_ERROR_CODE, "输入不是恰好一帧");
  }
  return decodePayload<T>(result.payload);
}

/** 从 UTF-8 JSON 负载解码消息。 */
export function decodePayload<T = unknown>(payload: Uint8Array): T {
  if (payload.byteLength > MAX_FRAME_BYTES) {
    throw new ProtocolFrameError(FRAME_ERROR_CODE, "负载长度超过上限");
  }
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(payload);
  } catch {
    throw new ProtocolFrameError(FRAME_ERROR_CODE, "JSON 负载不是 UTF-8");
  }
  try {
    return JSON.parse(text) as T;
  } catch {
    throw new ProtocolFrameError(FRAME_ERROR_CODE, "JSON 负载无效");
  }
}
