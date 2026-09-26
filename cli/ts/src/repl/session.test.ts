/** `!inLF!` 从 readline 交接到 raw mode 后，Ctrl+C 必须回到单行会话。 */

import { expect, test } from "bun:test";
import { PassThrough } from "node:stream";

import { runSingleLineRepl } from "./session.ts";

/** 为 readline/raw mode 交接测试补充的输入流能力。 */
interface RawInput extends PassThrough {
  isTTY: boolean;
  isRaw: boolean;
  setRawMode: (mode: boolean) => void;
}

/** 创建带 raw mode 形状的可注入输入流。 */
function inputStream(): RawInput {
  const input = new PassThrough() as RawInput;
  input.isTTY = true;
  input.isRaw = false;
  input.setRawMode = (mode) => { input.isRaw = mode; };
  return input;
}

/** 测试用异步写流适配器。 */
function writeSafely(stream: NodeJS.WritableStream, text: string): Promise<void> {
  return new Promise((resolve, reject) => stream.write(text, (error?: Error | null) => error ? reject(error) : resolve()));
}

test("独占 !inLF! 不交给核心，raw mode 结束后不重复版权行并可继续单行", async () => {
  const input = inputStream();
  const output = new PassThrough();
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  const session = runSingleLineRepl({
    input, output, error: new PassThrough(), write: writeSafely, cwd: "/project", env: { NO_COLOR: "1" },
    isTTY: true, color: "never", debug: false, version: "0.1.0",
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("!inLF!\n"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("code\r\u0003"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.end();
  expect(await session).toBe(0);
  expect(input.isRaw).toBe(false);
  expect(printed.match(/Xiao \(c\) XiaoCZX/gu)?.length).toBe(1);
});

test("!inLF! 后空缓冲 Ctrl+D 结束整个会话，不重新进入单行", async () => {
  const input = inputStream();
  const output = new PassThrough();
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  const session = runSingleLineRepl({
    input, output, error: new PassThrough(), write: writeSafely, cwd: "/project", env: { NO_COLOR: "1" },
    isTTY: true, color: "never", debug: false, version: "0.1.0",
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("!inLF!\n"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("\u0004"));
  expect(await session).toBe(0);
  expect(printed.match(/Xiao \(c\) XiaoCZX/gu)?.length).toBe(1);
});
