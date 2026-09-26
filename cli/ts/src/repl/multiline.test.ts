/** raw mode 循环使用可注入流验证终端恢复，不依赖伪终端库。 */

import { expect, test } from "bun:test";
import { PassThrough } from "node:stream";

import { runCli } from "../main.ts";
import { runMultilineSession } from "./multiline.ts";

/** 为 raw mode 集成测试补充的输入流能力。 */
interface RawInput extends PassThrough {
  setRawMode: (mode: boolean) => void;
  isRaw: boolean;
}

/** 创建带 raw mode 形状的可注入输入流。 */
function rawInput(): RawInput {
  const input = new PassThrough() as RawInput;
  input.isRaw = false;
  input.setRawMode = (mode) => { input.isRaw = mode; };
  return input;
}

/** 测试用异步写流适配器。 */
function writeSafely(stream: NodeJS.WritableStream, text: string): Promise<void> {
  return new Promise((resolve, reject) => stream.write(text, (error?: Error | null) => error ? reject(error) : resolve()));
}

test("raw mode 保留真实多行、剔除 !outLF!，Ctrl+C 恢复状态并返回新提示符码", async () => {
  const input = rawInput();
  const output = new PassThrough();
  const error = new PassThrough();
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  const commands: string[] = [];
  const session = runMultilineSession({
    input, output, error, write: writeSafely, env: { NO_COLOR: "1" }, isTTY: true, color: "never",
    onCommand: (command) => commands.push(command), terminalSize: { width: 80, height: 24 },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("code\r!outLF!\r\u0003"));
  const result = await session;
  expect(result.exitCode).toBe(130);
  expect(result.state.lines).toEqual([""]);
  expect(commands).toEqual(["run"]);
  expect(input.isRaw).toBe(false);
  expect(printed).toContain("\u001b[?2004l");
  input.end();
});

test("空缓冲 Ctrl+D 结束多行会话并恢复 raw mode", async () => {
  const input = rawInput();
  const output = new PassThrough();
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely, env: {}, isTTY: true, color: "never",
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("\u0004"));
  const result = await session;
  expect(result.exitCode).toBe(0);
  expect(result.state.lines).toEqual([""]);
  expect(input.isRaw).toBe(false);
  input.end();
});

test("未确认 Kitty 能力时 Shift+Enter 不进入运行分派", async () => {
  const input = rawInput();
  const output = new PassThrough();
  const commands: string[] = [];
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely, env: {}, isTTY: true, color: "never",
    onCommand: (command) => commands.push(command),
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("\u001b[13;2u\u0004"));
  const result = await session;
  expect(result.exitCode).toBe(0);
  expect(commands).toEqual([]);
  expect(result.state.lines).toEqual([""]);
  input.end();
});

test("--inLF 通过 CLI 入口进入 raw mode，而不是走 I1 占位诊断", async () => {
  const input = rawInput();
  const output = new PassThrough();
  const started = runCli(["--inLF"], {
    stdin: input, stdout: output, stderr: new PassThrough(), isTTY: true, env: { NO_COLOR: "1" },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("\u0004"));
  expect(await started).toBe(0);
  expect(input.isRaw).toBe(false);
  input.end();
});

test("--inLF 的 Ctrl+C 回到单行会话，EOF 再结束", async () => {
  const input = rawInput();
  const output = new PassThrough();
  const started = runCli(["--inLF"], {
    stdin: input, stdout: output, stderr: new PassThrough(), isTTY: true, env: { NO_COLOR: "1" },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("\u0003"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.end();
  expect(await started).toBe(0);
  expect(input.isRaw).toBe(false);
});
