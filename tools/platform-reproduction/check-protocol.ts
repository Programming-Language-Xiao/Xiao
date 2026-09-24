/**
 * 发送协议版本失配探针，并只验证核心返回帧的稳定结构化字段。
 */
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";

const maximumFrameBytes = 16 * 1024 * 1024;
const corePath = process.argv[2];

if (!corePath) {
  throw new Error("用法：check-protocol.ts <xiao-core>");
}

/**
 * 将 JSON 值编码为 Xiao 长度前缀协议帧。
 *
 * @param value 待编码的协议值。
 * @returns 包含八字节长度前缀和 UTF-8 JSON 载荷的帧。
 */
function encodeFrame(value: unknown): Buffer {
  const payload = Buffer.from(JSON.stringify(value), "utf8");
  if (payload.length > maximumFrameBytes) throw new Error("协议帧超过上限");
  const frame = Buffer.allocUnsafe(8 + payload.length);
  frame.writeBigUInt64BE(BigInt(payload.length), 0);
  payload.copy(frame, 8);
  return frame;
}

/**
 * 读取核心输出的第一帧，并在超时、退出或帧格式非法时失败。
 *
 * @param child 已启动且拥有标准输出管道的核心进程。
 * @returns 第一帧解析后的 JSON 对象。
 */
function readFirstFrame(child: ChildProcessWithoutNullStreams): Promise<Record<string, unknown>> {
  return new Promise((resolve, reject) => {
    let buffer = Buffer.alloc(0);
    let settled = false;
    const timeout = setTimeout(() => finish(new Error("读取协议响应超时")), 10_000);

    const cleanup = () => {
      clearTimeout(timeout);
      child.stdout.off("data", onData);
      child.off("error", onError);
      child.off("exit", onExit);
    };
    const finish = (error: Error | null, value?: Record<string, unknown>) => {
      if (settled) return;
      settled = true;
      cleanup();
      if (error) reject(error);
      else resolve(value as Record<string, unknown>);
    };
    const onData = (chunk: Buffer | string) => {
      buffer = Buffer.concat([buffer, Buffer.from(chunk)]);
      if (buffer.length < 8) return;
      const length = Number(buffer.readBigUInt64BE(0));
      if (length > maximumFrameBytes) {
        finish(new Error("协议响应帧超过上限"));
        return;
      }
      if (buffer.length < 8 + length) return;
      try {
        const value = JSON.parse(buffer.subarray(8, 8 + length).toString("utf8")) as Record<string, unknown>;
        finish(null, value);
      } catch (error) {
        finish(new Error(`协议响应不是有效 JSON：${String(error)}`));
      }
    };
    const onError = (error: Error) => finish(error);
    const onExit = (code: number | null, signal: string | null) => {
      finish(new Error(`核心在响应前退出：code=${code ?? "null"}, signal=${signal ?? "null"}`));
    };

    child.stdout.on("data", onData);
    child.on("error", onError);
    child.on("exit", onExit);
  });
}

/**
 * 启动核心、发送一个不支持的协议版本并检查稳定错误码。
 */
async function main(): Promise<void> {
  const child = spawn(corePath, [], {
    stdio: ["pipe", "pipe", "pipe"],
    windowsHide: true,
  });
  try {
    child.stdin.write(encodeFrame({
      type: "hello",
      request_id: "platform-reproduction-version-mismatch",
      protocol_version: 65_535,
      core_version: 1,
    }));
    const response = await readFirstFrame(child);
    const error = response.error as Record<string, unknown> | undefined;
    const result = {
      type: response.type ?? null,
      accepted: response.accepted ?? null,
      error_code: error?.code ?? null,
    };
    process.stdout.write(`${JSON.stringify(result)}\n`);
    if (result.type !== "hello" || result.accepted !== false || result.error_code !== "X11-PROTOCOL-004") {
      throw new Error("版本失配未按结构化协议错误拒绝");
    }
  } finally {
    child.stdin.destroy();
    if (child.exitCode === null && !child.killed) child.kill();
  }
}

await main();
