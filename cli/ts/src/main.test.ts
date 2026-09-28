/** CLI 进程入口的 IO 注入和稳定退出码回归。 */

import { describe, expect, test } from "bun:test";
import { EventEmitter } from "node:events";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { PassThrough, Readable } from "node:stream";

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
  readonly requests: Record<string, unknown>[] = [];
  private buffer = new Uint8Array(0);

  /** 每次运行请求跨进程汇总到同一测试向量。 */
  constructor(private readonly runRequests: Record<string, unknown>[] = []) {
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
    this.requests.push(request);
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
    } else if (request.type === "run") {
      this.runRequests.push(request);
      const source = request.source as { text: string };
      const rejected = source.text === "bad";
      this.stdout.write(Buffer.from(encodeFrame({
        type: "result", request_id: request.request_id, operation: "run",
        exit_code: rejected ? 1 : 0, exit_name: rejected ? "source_rejected" : "success",
        diagnostics: rejected ? [{ code: "X11-REPL-TEST-001", message: "源码错误", severity: "error" }] : [],
        report: null, events: [], metrics: null,
        value: rejected ? null : { kind: "int", text: source.text }, artifact: null,
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
  test("运行入口仅以项目优先的规范语言发请求，配置无效时不启动核心", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-run-locale-"));
    const globalPath = join(directory, "global.xiao");
    const projectPath = join(directory, "config.xiao");
    const requests: Record<string, unknown>[] = [];
    const stdout = new PassThrough();
    const options = {
      cwd: directory, stdout, stderr: new PassThrough(),
      env: { XIAO_GLOBAL_CONFIG: globalPath, NO_COLOR: "1" },
      corePath: process.execPath,
      spawnProcess: () => new FakeCore(requests) as never,
    };
    try {
      await writeFile(join(directory, "main.xiao"), "value = 1\n");
      await writeFile(globalPath, '[language]\nlocale = "en"\n');
      const helpOutput = new PassThrough();
      let help = "";
      helpOutput.on("data", (chunk: Buffer) => { help += chunk.toString(); });
      expect(await runCli(["--help"], { ...options, stdout: helpOutput })).toBe(0);
      expect(help).toContain("Usage:");
      expect(help).toContain("xiao --inLF [file.xiao]");
      expect(help).not.toContain("用法：");
      expect(await runCli(["run", "main.xiao"], options)).toBe(0);
      await writeFile(projectPath, '[language]\nlocale = "zh"\n');
      expect(await runCli(["run", "main.xiao"], options)).toBe(0);
      expect(requests.filter((request) => request.type === "run").map((request) => request.locale)).toEqual(["en-US", "zh-CN"]);
      await writeFile(projectPath, "[language]\nlocale = 12\n");
      const invalidOutput = new PassThrough();
      let printed = "";
      invalidOutput.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
      expect(await runCli(["--json", "run", "main.xiao"], { ...options, stdout: invalidOutput })).toBe(78);
      expect(JSON.parse(printed)).toMatchObject({
        code: "X11-CONFIG-002", details: { path: projectPath, key: "language.locale", scope: "project" },
      });
      expect(requests.filter((request) => request.type === "run")).toHaveLength(2);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("无参数进入单行会话：执行、显示错误后继续，EOF 正常退出", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-repl-"));
    const stdout = new PassThrough();
    const stderr = new PassThrough();
    let output = "";
    let errors = "";
    stdout.on("data", (chunk: Buffer) => { output += chunk.toString(); });
    stderr.on("data", (chunk: Buffer) => { errors += chunk.toString(); });
    const requests: Record<string, unknown>[] = [];
    try {
      const code = await runCli([], {
        stdin: Readable.from(["1\nbad\n2\n"]), stdout, stderr, cwd: directory,
        corePath: process.execPath, spawnProcess: () => new FakeCore(requests) as never,
        env: { ...process.env, XIAO_GLOBAL_CONFIG: join(directory, "global.xiao"), XIAO_ACTIVE_ENV: join(directory, ".venv"), NO_COLOR: "1" },
        isTTY: false,
      });
      expect(code).toBe(0);
      expect(output).toContain("Xiao (c) XiaoCZX\nV0.1.0\n");
      expect(output).toContain(`$venv$ ${directory} [X> `);
      expect(output).toContain("1\n");
      expect(output).toContain("2\n");
      expect(errors).toContain("X11-REPL-TEST-001: 源码错误");
      expect(requests.map((request) => (request.source as { text: string }).text)).toEqual(["1", "bad", "2"]);
      expect(requests.map((request) => request.locale)).toEqual(["zh-CN", "zh-CN", "zh-CN"]);
      expect(requests.every((request) => (request.optimization as { level: number }).level === 0)).toBe(true);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("--inLF 执行完整缓冲区时沿用 CLI 核心注入", async () => {
    const input = new PassThrough() as PassThrough & { isRaw: boolean; setRawMode: (mode: boolean) => void };
    input.isRaw = false;
    input.setRawMode = (mode) => { input.isRaw = mode; };
    const output = new PassThrough();
    let printed = "";
    output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
    const requests: Record<string, unknown>[] = [];
    const session = runCli(["--inLF"], {
      stdin: input, stdout: output, stderr: new PassThrough(),
      cwd: process.cwd(), corePath: process.execPath,
      spawnProcess: () => new FakeCore(requests) as never,
      env: { NO_COLOR: "1" }, isTTY: true,
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    input.write(Buffer.from("first\rsecond\r!outLF!\r"));
    await new Promise((resolve) => setTimeout(resolve, 5));
    input.write(Buffer.from("\r"));
    await new Promise((resolve) => setTimeout(resolve, 20));
    input.end();
    expect(await session).toBe(0);
    expect(requests.map((request) => (request.source as { text: string }).text)).toEqual(["first\nsecond"]);
    expect(printed).toContain("first\r\nsecond\r\n");
  });

  test("--inLF 取消后回到单行时复用同一个核心会话", async () => {
    const input = new PassThrough() as PassThrough & { isRaw: boolean; setRawMode: (mode: boolean) => void };
    input.isRaw = false;
    input.setRawMode = (mode) => { input.isRaw = mode; };
    const output = new PassThrough();
    let printed = "";
    const singleLinePrompt = new Promise<void>((resolve) => {
      output.on("data", (chunk: Buffer) => {
        printed += chunk.toString();
        if (printed.includes("[X> ")) resolve();
      });
    });
    const runs: Record<string, unknown>[] = [];
    const cores: FakeCore[] = [];
    let spawnCount = 0;
    const session = runCli(["--inLF"], {
      stdin: input, stdout: output, stderr: new PassThrough(),
      cwd: process.cwd(), corePath: process.execPath,
      spawnProcess: () => {
        spawnCount += 1;
        const core = new FakeCore(runs);
        cores.push(core);
        return core as never;
      },
      env: { NO_COLOR: "1" }, isTTY: true,
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    input.write(Buffer.from("unfinished\u0003"));
    await singleLinePrompt;
    const runCompleted = new Promise<void>((resolve) => {
      output.on("data", (chunk: Buffer) => {
        if (/\b1\r?\n/.test(chunk.toString())) resolve();
      });
    });
    input.write(Buffer.from("1\n"));
    await runCompleted;
    input.end();
    expect(await session).toBe(0);
    expect(spawnCount).toBe(1);
    expect(cores).toHaveLength(1);
    expect(cores[0].requests.filter((request) => request.type === "hello")).toHaveLength(1);
    expect(cores[0].requests.filter((request) => request.type === "shutdown")).toHaveLength(1);
    expect(runs.map((request) => (request.source as { text: string }).text)).toEqual(["1"]);
  });

  test("--inLF 缺失文件立即绑定，首次保存直接创建文件", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-new-file-"));
    try {
      const input = new PassThrough() as PassThrough & { isRaw: boolean; setRawMode: (mode: boolean) => void };
      input.isRaw = false;
      input.setRawMode = (mode) => { input.isRaw = mode; };
      const output = new PassThrough();
      let printed = "";
      output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
      const session = runCli(["--inLF", "new.xiao"], {
        stdin: input, stdout: output, stderr: new PassThrough(), cwd: directory,
        env: { NO_COLOR: "1" }, isTTY: true,
      });
      await new Promise((resolve) => setTimeout(resolve, 5));
      input.write(Buffer.from("code\r!save!\r"));
      await new Promise((resolve) => setTimeout(resolve, 15));
      input.end();
      expect(await session).toBe(0);
      expect(await readFile(join(directory, "new.xiao"), "utf8")).toBe("code");
      expect(printed).not.toContain("输入保存位置");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("--inLF 拒绝非法 UTF-8 文件，不进入 raw mode", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-invalid-file-"));
    try {
      await writeFile(join(directory, "bad.xiao"), Buffer.from([0xff]));
      const input = new PassThrough() as PassThrough & { isRaw: boolean; setRawMode: (mode: boolean) => void };
      input.isRaw = false;
      input.setRawMode = (mode) => { input.isRaw = mode; };
      const error = new PassThrough();
      let errors = "";
      error.on("data", (chunk: Buffer) => { errors += chunk.toString(); });
      expect(await runCli(["--inLF", "bad.xiao"], {
        stdin: input, stdout: new PassThrough(), stderr: error, cwd: directory,
        env: { NO_COLOR: "1" }, isTTY: true,
      })).toBe(74);
      expect(errors).toContain("X11-CLI-SAVE-003");
      expect(input.isRaw).toBe(false);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("非 raw mode 多行入口给稳定诊断；机器 JSON 模式不启动交互会话", async () => {
    const stdout = new PassThrough();
    const stderr = new PassThrough();
    expect(await runCli(["--inLF"], { stdout, stderr })).toBe(64);
    expect(stderr.read()?.toString()).toContain("X11-CLI-REPL-002");
    expect(await runCli(["--inLF", "file.xiao"], { stdout: new PassThrough(), stderr: new PassThrough() })).toBe(64);
    const json = new PassThrough();
    expect(await runCli(["--json"], { stdout: json, stderr: new PassThrough() })).toBe(64);
    expect(JSON.parse(json.read()?.toString() ?? "{}")).toMatchObject({ type: "error", code: "X11-CLI-ARG-001" });
  });

  test("配置启用但未装 Git 时不阻断提示符，可从调试输出查询原因", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-repl-no-git-"));
    const stdout = new PassThrough();
    const stderr = new PassThrough();
    let output = "";
    let errors = "";
    stdout.on("data", (chunk: Buffer) => { output += chunk.toString(); });
    stderr.on("data", (chunk: Buffer) => { errors += chunk.toString(); });
    try {
      await writeFile(join(directory, "config.xiao"), "[CLI]\ngit = { summary = true }\n");
      const code = await runCli(["-debug"], {
        stdin: Readable.from([]), stdout, stderr, cwd: directory,
        env: { ...process.env, PATH: "", NO_COLOR: "1", XIAO_GLOBAL_CONFIG: join(directory, "global.xiao") }, isTTY: false,
      });
      expect(code).toBe(0);
      expect(output).toContain(`${directory} [X>`);
      expect(errors).toContain("X11-REPL-GIT-001: Git 摘要已降级（unavailable）");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("全局配置启用 Git 摘要，分支计数随每次提示符刷新；项目配置可覆盖", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-repl-git-"));
    const project = join(directory, "project");
    const globalConfig = join(directory, "global.xiao");
    try {
      await mkdir(project);
      await writeFile(globalConfig, "[CLI]\ngit = { summary = true }\n");
      const output = new PassThrough();
      let printed = "";
      output.on("data", (chunk: Buffer) => { printed += chunk.toString(); });
      let statusQueries = 0;
      const options = {
        stdin: Readable.from(["1\n"]), stdout: output, stderr: new PassThrough(), cwd: project,
        env: { ...process.env, XIAO_GLOBAL_CONFIG: globalConfig, NO_COLOR: "1" },
        corePath: process.execPath, isTTY: false,
        spawnProcess: () => new FakeCore() as never,
        gitRunStatus: async () => `# branch.oid 123\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +${statusQueries++} -0\n`,
      };
      expect(await runCli([], options)).toBe(0);
      expect(printed).toContain("main-0↑-0↓ [X>");
      expect(printed).toContain("main-1↑-0↓ [X>");
      expect(statusQueries).toBe(2);
      await writeFile(join(project, "config.xiao"), "[CLI]\ngit = { summary = false }\n");
      const disabled = new PassThrough();
      let disabledOutput = "";
      disabled.on("data", (chunk: Buffer) => { disabledOutput += chunk.toString(); });
      expect(await runCli([], { ...options, stdin: Readable.from([]), stdout: disabled })).toBe(0);
      expect(disabledOutput).not.toContain("main-");
      expect(statusQueries).toBe(2);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

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
