/** raw mode 循环使用可注入流验证终端恢复，不依赖伪终端库。 */

import { expect, test } from "bun:test";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { PassThrough } from "node:stream";

import { runCli } from "../main.ts";
import { nativeReplFileSystem } from "./file.ts";
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

/** 等待多行会话完成异步界面状态转换，超时即让测试失败。 */
async function waitFor(condition: () => boolean | Promise<boolean>): Promise<void> {
  const deadline = Date.now() + 2_000;
  while (!await condition()) {
    if (Date.now() >= deadline) throw new Error("多行会话未在期限内完成界面转换");
    await new Promise((resolve) => setTimeout(resolve, 2));
  }
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
  input.write(Buffer.from("code\r!outLF!\r"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("\u0003\u0003"));
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

test("文件入口载入逻辑行并立即绑定，源码不含原始 CRLF", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-bound-"));
  try {
    const path = join(directory, "main.xiao");
    await writeFile(path, "first\r\n  second", "utf8");
    const input = rawInput();
    const session = runMultilineSession({
      input, output: new PassThrough(), error: new PassThrough(), write: writeSafely,
      env: {}, isTTY: true, color: "never", cwd: directory, file: "main.xiao",
    });
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.end();
    const result = await session;
    expect(result.state.filePath).toBe(path);
    expect(result.state.lines).toEqual(["first", "  second"]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("未绑定保存剔除控制行，建立绑定后再次保存直接写回", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-first-save-"));
  try {
    const input = rawInput();
    const output = new PassThrough();
    let printed = "";
    output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
    const session = runMultilineSession({
      input, output, error: new PassThrough(), write: writeSafely,
      env: { NO_COLOR: "1" }, isTTY: true, color: "never", cwd: directory,
    });
    await waitFor(() => input.isRaw && input.listenerCount("data") > 0);
    input.write(Buffer.from("code\r!save!\r"));
    await waitFor(() => printed.includes("输入保存位置"));
    input.write(Buffer.from("main.xiao\r"));
    await waitFor(async () => await readFile(join(directory, "main.xiao"), "utf8").catch(() => null) === "code");
    await waitFor(() => printed.lastIndexOf("    1|code") > printed.lastIndexOf("输入保存位置"));
    input.write(Buffer.from("\r!save!\r"));
    input.end();
    const result = await session;
    expect(result.state.lines).toEqual(["code"]);
    expect(result.state.filePath).toBe(join(directory, "main.xiao"));
    expect(result.commands).toEqual(["save", "save"]);
    expect(await readFile(join(directory, "main.xiao"), "utf8")).toBe("code");
    expect(printed).toContain("输入保存位置");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("首次保存到已有 .xiao 文件直接覆盖且不进入运行确认", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-overwrite-"));
  try {
    const path = join(directory, "main.xiao");
    await writeFile(path, "old", "utf8");
    const input = rawInput();
    const output = new PassThrough();
    let printed = "";
    output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
    const session = runMultilineSession({
      input, output, error: new PassThrough(), write: writeSafely,
      env: {}, isTTY: true, color: "never", cwd: directory,
    });
    await waitFor(() => input.isRaw && input.listenerCount("data") > 0);
    input.write(Buffer.from("new\r!save!\r"));
    await waitFor(() => printed.includes("输入保存位置"));
    input.write(Buffer.from("main.xiao\r"));
    await waitFor(async () => await readFile(path, "utf8").catch(() => null) === "new");
    input.end();
    const result = await session;
    expect(result.state.filePath).toBe(path);
    expect(await readFile(path, "utf8")).toBe("new");
    expect(printed).toContain("输入保存位置");
    expect(printed).not.toContain("按 Enter 确认并运行");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("未绑定缓冲区的 Kitty Ctrl+Shift+S 进入路径保存", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-kitty-save-"));
  try {
    const input = rawInput();
    const output = new PassThrough();
    let printed = "";
    output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
    const session = runMultilineSession({
      input, output, error: new PassThrough(), write: writeSafely,
      env: {}, isTTY: true, color: "never", cwd: directory,
    });
    await waitFor(() => input.isRaw && input.listenerCount("data") > 0);
    input.write(Buffer.from("code\u001b[?0u\u001b[?28u\u001b[115;6u"));
    await waitFor(() => printed.includes("输入保存位置"));
    input.write(Buffer.from("main.xiao\r"));
    await waitFor(async () => await readFile(join(directory, "main.xiao"), "utf8").catch(() => null) === "code");
    input.end();
    const result = await session;
    expect(result.state.filePath).toBe(join(directory, "main.xiao"));
    expect(result.state.lines).toEqual(["code"]);
    expect(await readFile(join(directory, "main.xiao"), "utf8")).toBe("code");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("同一输入块不能绕过保存界面，q 和 Esc 无损取消", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-cancel-save-"));
  try {
    const input = rawInput();
    const session = runMultilineSession({
      input, output: new PassThrough(), error: new PassThrough(), write: writeSafely,
      env: {}, isTTY: true, color: "never", cwd: directory,
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    input.write(Buffer.from("code\r!save!\rmain.xiao\r"));
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("q\r"));
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("\r!save!\r"));
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("ignored.xiao\u001b"));
    await new Promise((resolve) => setTimeout(resolve, 70));
    input.end();
    const result = await session;
    expect(result.state.lines).toEqual(["code"]);
    expect(result.state.filePath).toBeNull();
    expect(await readdir(directory)).toEqual([]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("!panel! 剔除控制行，面板输入与回车不进入源码", async () => {
  const input = rawInput();
  const output = new PassThrough();
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely,
    env: {}, isTTY: true, color: "never",
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("code\r!panel!\r\r"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("ignored\r"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("\u001b"));
  await new Promise((resolve) => setTimeout(resolve, 70));
  input.end();
  const result = await session;
  expect(result.state.lines).toEqual(["code"]);
  expect(result.state.cursor).toEqual({ line: 0, column: 4 });
  expect(result.commands).toEqual(["panel"]);
  expect(printed).toContain("命令面板");
  expect(printed).not.toContain("按 Enter 确认并运行");
});

test("同一输入块里的面板键不能在界面出现前立刻关闭面板", async () => {
  const input = rawInput();
  const session = runMultilineSession({
    input, output: new PassThrough(), error: new PassThrough(), write: writeSafely,
    env: {}, isTTY: true, color: "never",
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("\u001b[?0u\u001b[?28u"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("code\r!panel!\r\u001b[112;6u"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("panel-only\r\u001b[112;6u"));
  await new Promise((resolve) => setTimeout(resolve, 15));
  input.end();
  const result = await session;
  expect(result.state.lines).toEqual(["code"]);
  expect(result.commands).toEqual(["panel"]);
});

test("Kitty 面板键可开关，关闭后保留文件绑定和编辑位置", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-panel-bound-"));
  try {
    const path = join(directory, "main.xiao");
    await writeFile(path, "code", "utf8");
    const input = rawInput();
    const session = runMultilineSession({
      input, output: new PassThrough(), error: new PassThrough(), write: writeSafely,
      env: {}, isTTY: true, color: "never", cwd: directory, file: "main.xiao",
    });
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("X\u001b[?0u\u001b[?28u\u001b[112;6u"));
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("temporary\r\u001b[112;6u"));
    await new Promise((resolve) => setTimeout(resolve, 15));
    input.end();
    const result = await session;
    expect(result.state.filePath).toBe(path);
    expect(result.state.lines).toEqual(["Xcode"]);
    expect(result.state.cursor).toEqual({ line: 0, column: 1 });
    expect(await readFile(path, "utf8")).toBe("code");
    expect(result.commands).toEqual(["panel"]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("临时 raw 面板从独立入口打开，Esc 结束后恢复终端", async () => {
  const input = rawInput();
  const output = new PassThrough();
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely,
    env: {}, isTTY: true, color: "never", initialPanel: true,
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("query\r\u001b"));
  await new Promise((resolve) => setTimeout(resolve, 70));
  const result = await session;
  expect(result.exitCode).toBe(0);
  expect(result.state.lines).toEqual([""]);
  expect(input.isRaw).toBe(false);
  expect(printed).toContain("命令面板");
  input.end();
});

test("已载入文件用 Kitty 保存键直接写回，不进入路径界面", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-direct-save-"));
  try {
    const path = join(directory, "main.xiao");
    await writeFile(path, "old\r\nsecond", "utf8");
    const input = rawInput();
    const output = new PassThrough();
    let printed = "";
    output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
    const session = runMultilineSession({
      input, output, error: new PassThrough(), write: writeSafely,
      env: {}, isTTY: true, color: "never", cwd: directory, file: "main.xiao",
    });
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("X\u001b[?0u\u001b[?28u\u001b[115;6u"));
    await new Promise((resolve) => setTimeout(resolve, 20));
    input.end();
    const result = await session;
    expect(result.state.filePath).toBe(path);
    expect(result.state.lines).toEqual(["Xold", "second"]);
    expect(await readFile(path, "utf8")).toBe("Xold\nsecond");
    expect(printed).not.toContain("输入保存位置");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("运行完整缓冲区后仍保留文件绑定并可直接保存", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-run-save-"));
  try {
    const path = join(directory, "main.xiao");
    await writeFile(path, "code", "utf8");
    const input = rawInput();
    const sources: string[] = [];
    const session = runMultilineSession({
      input, output: new PassThrough(), error: new PassThrough(), write: writeSafely,
      env: {}, isTTY: true, color: "never", cwd: directory, file: "main.xiao",
      executeSource: async (source) => {
        sources.push(source);
        return {
          response: {
            type: "result", request_id: "run", operation: "run", exit_code: 0, exit_name: "success",
            diagnostics: [], report: null, events: [], metrics: { peak_live_bytes: 0 },
            value: null, artifact: null,
          },
          stderr: "", corePath: "test", coreSource: "override",
        };
      },
    });
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("\u001b[?0u\u001b[?28u\u001b[13;2u"));
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("\r"));
    await new Promise((resolve) => setTimeout(resolve, 20));
    input.write(Buffer.from("X\u001b[115;6u"));
    await new Promise((resolve) => setTimeout(resolve, 20));
    input.end();
    const result = await session;
    expect(sources).toEqual(["code"]);
    expect(result.state.filePath).toBe(path);
    expect(result.state.lines).toEqual(["Xcode"]);
    expect(await readFile(path, "utf8")).toBe("Xcode");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("已绑定保存失败保留原文件、绑定与缓冲区并显示路径和原因", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-failed-save-"));
  try {
    const path = join(directory, "main.xiao");
    await writeFile(path, "old", "utf8");
    const input = rawInput();
    const output = new PassThrough();
    const error = new PassThrough();
    let printed = "";
    let errors = "";
    output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
    error.on("data", (chunk: Buffer) => { errors += chunk.toString(); });
    const session = runMultilineSession({
      input, output, error, write: writeSafely,
      env: {}, isTTY: true, color: "never", cwd: directory, file: "main.xiao",
      fileSystem: {
        ...nativeReplFileSystem,
        writeFile: async () => { throw Object.assign(new Error("denied"), { code: "EACCES" }); },
      },
    });
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("X\u001b[?0u\u001b[?28u\u001b[115;6u"));
    await new Promise((resolve) => setTimeout(resolve, 20));
    input.end();
    const result = await session;
    expect(result.state.filePath).toBe(path);
    expect(result.state.lines).toEqual(["Xold"]);
    expect(await readFile(path, "utf8")).toBe("old");
    expect(errors).toContain("X11-CLI-SAVE-002");
    expect(errors).toContain(path);
    expect(printed).toContain("权限不足");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("一行高终端的保存失败不生成越界光标坐标", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-short-save-"));
  try {
    const path = join(directory, "main.xiao");
    await writeFile(path, "old", "utf8");
    const input = rawInput();
    const output = new PassThrough();
    const error = new PassThrough();
    let printed = "";
    let errors = "";
    output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
    error.on("data", (chunk: Buffer) => { errors += chunk.toString(); });
    const session = runMultilineSession({
      input, output, error, write: writeSafely, env: {}, isTTY: true, color: "never",
      cwd: directory, file: "main.xiao", terminalSize: { width: 20, height: 1 },
      fileSystem: {
        ...nativeReplFileSystem,
        writeFile: async () => { throw Object.assign(new Error("denied"), { code: "EACCES" }); },
      },
    });
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("\u001b[?0u\u001b[?28u\u001b[115;6u"));
    await new Promise((resolve) => setTimeout(resolve, 20));
    input.end();
    const result = await session;
    expect(result.state.filePath).toBe(path);
    expect(errors).toContain("X11-CLI-SAVE-002");
    expect(printed).not.toMatch(/\u001b\[(?:0|-\d+);/u);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("首次保存失败仍未绑定，保留路径输入供修复后重试", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-repl-retry-save-"));
  try {
    let fail = true;
    const input = rawInput();
    const output = new PassThrough();
    let printed = "";
    output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
    const error = new PassThrough();
    let errors = "";
    error.on("data", (chunk: Buffer) => { errors += chunk.toString(); });
    const session = runMultilineSession({
      input, output, error, write: writeSafely,
      env: {}, isTTY: true, color: "never", cwd: directory,
      fileSystem: {
        ...nativeReplFileSystem,
        writeFile: async (path, data, encoding) => {
          if (fail) {
            fail = false;
            throw Object.assign(new Error("denied"), { code: "EACCES" });
          }
          await nativeReplFileSystem.writeFile(path, data, encoding);
        },
      },
    });
    await waitFor(() => input.isRaw && input.listenerCount("data") > 0);
    input.write(Buffer.from("code\r!save!\r"));
    await waitFor(() => printed.includes("输入保存位置"));
    input.write(Buffer.from("main.xiao\r"));
    await waitFor(() => printed.includes("X11-CLI-SAVE-002"));
    input.write(Buffer.from("\r"));
    await waitFor(async () => await readFile(join(directory, "main.xiao"), "utf8").catch(() => null) === "code");
    input.end();
    const result = await session;
    expect(errors).toContain("X11-CLI-SAVE-002");
    expect(result.state.filePath).toBe(join(directory, "main.xiao"));
    expect(result.state.lines).toEqual(["code"]);
    expect(await readFile(join(directory, "main.xiao"), "utf8")).toBe("code");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("raw mode 启动失败时仍尝试恢复终端", async () => {
  const input = rawInput();
  const transitions: boolean[] = [];
  input.setRawMode = (mode) => {
    transitions.push(mode);
    if (mode) throw new Error("raw mode 不可用");
  };
  await expect(runMultilineSession({
    input, output: new PassThrough(), error: new PassThrough(), write: writeSafely,
    env: {}, isTTY: true, color: "never",
  })).rejects.toThrow("raw mode 不可用");
  expect(transitions).toEqual([true, false]);
  input.end();
});

test("Ctrl+D 完成后排队的数据不会污染已结束的编辑状态", async () => {
  const input = rawInput();
  const session = runMultilineSession({
    input, output: new PassThrough(), error: new PassThrough(), write: writeSafely,
    env: {}, isTTY: true, color: "never",
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("\u0004"));
  input.write(Buffer.from("late"));
  const result = await session;
  expect(result.exitCode).toBe(0);
  expect(result.state.lines).toEqual([""]);
  input.end();
});

test("括号粘贴中以换行结束的控制行按 Enter 语义剔除", async () => {
  const input = rawInput();
  const output = new PassThrough();
  const commands: string[] = [];
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely, env: {}, isTTY: true, color: "never",
    onCommand: (command) => commands.push(command),
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("\u001b[200~!outLF!\n\u001b[201~"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("\u0003\u0004"));
  const result = await session;
  expect(result.exitCode).toBe(0);
  expect(commands).toEqual(["run"]);
  expect(result.state.lines).toEqual([""]);
  input.end();
});

test("确认态忽略源码字符，Esc 无损取消且不执行", async () => {
  const input = rawInput();
  const output = new PassThrough();
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  let called = false;
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely, env: {}, isTTY: true, color: "never",
    executeSource: async () => { called = true; throw new Error("不得执行"); },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("code\r!outLF!\r"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("ignored\u001b"));
  await new Promise((resolve) => setTimeout(resolve, 70));
  input.end();
  const result = await session;
  expect(called).toBe(false);
  expect(result.state.lines).toEqual(["code"]);
  expect(printed).toContain("按 Enter 确认并运行");
});

test("同一输入块中的第二个 Enter 不得绕过可见确认态", async () => {
  const input = rawInput();
  const output = new PassThrough();
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  let called = false;
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely, env: {}, isTTY: true, color: "never",
    executeSource: async () => { called = true; throw new Error("不得自动执行"); },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("code\r!outLF!\r\r"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.end();
  const result = await session;
  expect(called).toBe(false);
  expect(result.state.lines).toEqual(["code"]);
  expect(printed).toContain("按 Enter 确认并运行");
});

test("确认后调用完整源码并恢复覆盖模式、剪贴板和光标", async () => {
  const input = rawInput();
  const output = new PassThrough();
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  const requests: string[] = [];
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely, env: {}, isTTY: true, color: "never",
    executeSource: async (source) => {
      expect(input.isRaw).toBe(false);
      requests.push(source);
      return {
        response: {
          type: "result", request_id: "test", operation: "run", exit_code: 0, exit_name: "success",
          diagnostics: [], report: null, events: [], metrics: { peak_live_bytes: 2_000_000 },
          value: { kind: "int", value: "42" }, artifact: null,
        },
        stderr: "", corePath: "test", coreSource: "override",
      };
    },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("!ovr!\rfirst\rsecond\u0017\u0019\r!outLF!\r"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("\r"));
  await new Promise((resolve) => setTimeout(resolve, 20));
  input.end();
  const result = await session;
  expect(requests).toEqual(["first\nsecond"]);
  expect(result.state.lines).toEqual(["first", "second"]);
  expect(result.state.cursor).toEqual({ line: 1, column: 6 });
  expect(result.state.overwrite).toBe(true);
  expect(result.state.killBuffer).toBe("second");
  expect(printed).toContain("42\r\n");
  expect(printed).toContain("内存:2.00MB\r\n");
  expect(input.isRaw).toBe(false);
});

test("执行期 SIGINT 只取消本次运行，缓冲区和光标可继续编辑", async () => {
  const input = rawInput();
  const output = new PassThrough();
  let startRun!: () => void;
  const running = new Promise<void>((resolve) => { startRun = resolve; });
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely, env: {}, isTTY: true, color: "never",
    executeSource: async (_source, signal) => {
      startRun();
      await new Promise<void>((resolve) => signal.addEventListener("abort", () => resolve(), { once: true }));
      return {
        response: { type: "cancelled", request_id: "run", target_request_id: "run", accepted: true, exit_code: 2 },
        stderr: "", corePath: "test", coreSource: "override",
      };
    },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("code\r!outLF!\r"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("\r"));
  await running;
  expect(input.isRaw).toBe(false);
  process.emit("SIGINT");
  await new Promise((resolve) => setTimeout(resolve, 5));
  expect(input.isRaw).toBe(true);
  input.end();
  const result = await session;
  expect(result.exitCode).toBe(0);
  expect(result.state.lines).toEqual(["code"]);
  expect(result.state.cursor).toEqual({ line: 0, column: 4 });
});

test("确认态尺寸变化重画标题，输出期间不重排历史内容", async () => {
  const input = rawInput();
  const output = new PassThrough() as PassThrough & { columns: number; rows: number };
  output.columns = 20;
  output.rows = 10;
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely, env: {}, isTTY: true, color: "never",
    executeSource: async () => {
      output.columns = 40;
      output.emit("resize");
      return {
        response: {
          type: "result", request_id: "run", operation: "run", exit_code: 0, exit_name: "success",
          diagnostics: [], report: null, events: [], metrics: { peak_live_bytes: 0 },
          value: null, artifact: null,
        },
        stderr: "", corePath: "test", coreSource: "override",
      };
    },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("code\r!outLF!\r"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  output.columns = 30;
  output.emit("resize");
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("\r"));
  await new Promise((resolve) => setTimeout(resolve, 15));
  input.end();
  await session;
  expect(printed).toContain("─ 按 Enter 确认并运行");
  expect(printed).toContain("─".repeat(40) + "\r\n");
});

test("核心执行失败仍结束输出段并保留编辑状态", async () => {
  const input = rawInput();
  const output = new PassThrough();
  const error = new PassThrough();
  let printed = "";
  let errors = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  error.on("data", (chunk: Buffer) => { errors += chunk.toString(); });
  const session = runMultilineSession({
    input, output, error, write: writeSafely, env: {}, isTTY: true, color: "never",
    executeSource: async () => { throw new Error("核心不可用"); },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("code\r!outLF!\r"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("\r"));
  await new Promise((resolve) => setTimeout(resolve, 10));
  input.end();
  const result = await session;
  expect(result.state.lines).toEqual(["code"]);
  expect(result.state.cursor).toEqual({ line: 0, column: 4 });
  expect(errors).toContain("X11-CLI-001: 核心不可用\r\n");
  expect(printed).toContain("内存:?MB\r\n");
  expect(input.isRaw).toBe(false);
});

test("SGR 鼠标点击和拖动建立选区，滚轮忽略且退出关闭鼠标上报", async () => {
  const input = rawInput();
  const output = new PassThrough();
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely, env: {}, isTTY: true, color: "never",
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("abc\u001b[<0;7;1M\u001b[<32;10;1M\u001b[<64;10;1M\u001b[<0;10;1m"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.end();
  const result = await session;
  expect(result.state.anchor).toEqual({ line: 0, column: 0 });
  expect(result.state.cursor).toEqual({ line: 0, column: 3 });
  expect(printed).toContain("\u001b[?1000h\u001b[?1002h\u001b[?1006h");
  expect(printed).toContain("\u001b[?1006l\u001b[?1002l\u001b[?1000l");
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

test("Kitty 所有按键确认后 Shift+Enter 进入确认态", async () => {
  const input = rawInput();
  const output = new PassThrough();
  let printed = "";
  output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
  const commands: string[] = [];
  const session = runMultilineSession({
    input, output, error: new PassThrough(), write: writeSafely, env: {}, isTTY: true, color: "never",
    onCommand: (command) => commands.push(command),
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  input.write(Buffer.from("code\u001b[?0u\u001b[?28u\u001b[13;2u"));
  await new Promise((resolve) => setTimeout(resolve, 5));
  input.write(Buffer.from("\u0003"));
  input.end();
  const result = await session;
  expect(commands).toEqual(["run"]);
  expect(result.state.lines).toEqual(["code"]);
  expect(printed).toContain("按 Enter 确认并运行");
  expect(printed).toContain("\u001b[<u");
});

test("--inLF 通过 CLI 入口进入 raw mode，而不是走旧的占位诊断", async () => {
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
