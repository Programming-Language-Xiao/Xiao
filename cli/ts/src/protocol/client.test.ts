/** 核心客户端的协议会话回归；使用内存子进程替身验证握手和 request_id 路由。 */

import { describe, expect, test } from "bun:test";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";

import { ProtocolClient } from "./client.ts";
import { decodePayload, encodeFrame } from "./codec.ts";
import type { PackageRequest } from "./messages.ts";

/** 在内存中模拟 xiao-core 的最小协议进程。 */
class FakeCore extends EventEmitter {
  readonly stdin = new PassThrough();
  readonly stdout = new PassThrough();
  readonly stderr = new PassThrough();
  exitCode: number | null = null;
  signalCode: NodeJS.Signals | null = null;
  killed = false;
  private buffer = new Uint8Array(0);
  readonly requests: Record<string, unknown>[] = [];
  private pendingRunId: string | null = null;
  private failedPackageView = false;

  /** 监听 stdin 并按帧返回固定结果。 */
  constructor(
    private readonly onRun?: () => void,
    private readonly capabilities: string[] = ["run", "environment"],
    private readonly packageErrorOnce = false,
  ) {
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
    this.requests.push(request);
    if (request.type === "hello") {
      this.stdout.write(Buffer.from(encodeFrame({
        type: "hello", request_id: request.request_id, accepted: true, protocol_version: 1,
        core_version: 1, versions: {}, capabilities: this.capabilities, error: null,
      })));
    } else if (request.type === "run") {
      if (this.onRun !== undefined) {
        this.pendingRunId = String(request.request_id);
        this.onRun();
        return;
      }
      this.stdout.write(Buffer.from(encodeFrame({
        type: "result", request_id: request.request_id, operation: "run", exit_code: 0,
        exit_name: "success", diagnostics: [], report: null, events: [], metrics: null, value: null, artifact: null,
      })));
    } else if (request.type === "cancel" && this.pendingRunId !== null) {
      this.stdout.write(Buffer.from(encodeFrame({
        type: "error", request_id: this.pendingRunId, exit_code: 2,
        error: { code: "X11-PROTOCOL-005", message: "请求已取消" }, report: null,
      })));
    } else if (request.type === "build") {
      this.stdout.write(Buffer.from(encodeFrame({
        type: "result", request_id: request.request_id, operation: "build", exit_code: 0,
        exit_name: "success", diagnostics: [], report: null, events: [], metrics: null, value: null,
        artifact: {
          executable: request.output, llvm_ir_output: request.llvm_ir_output,
          toolchain_fingerprint: "test", uses_runtime: false, runtime_components: [],
        },
      })));
    } else if (request.type === "test") {
      const cases = Array.isArray(request.cases) ? request.cases : [];
      this.stdout.write(Buffer.from(encodeFrame({
        type: "test_result", request_id: request.request_id, operation: "test", exit_code: 0,
        exit_name: "success", total: cases.length, passed: cases.length, failed: 0,
        tests: cases.map((value) => {
          const source = value as { path?: string | null; module?: string };
          return {
            path: source.path ?? source.module ?? "main",
            module: source.module ?? "main",
            exit_code: 0,
            exit_name: "success",
            diagnostics: [], report: null, events: [], metrics: null, value: null, error: null,
          };
        }),
      })));
    } else if (request.type === "environment") {
      const logicalName = typeof request.logical_name === "string" ? request.logical_name : "venv";
      this.stdout.write(Buffer.from(encodeFrame({
        type: "environment_result",
        request_id: request.request_id,
        metadata: {
          metadata_version: 1,
          logical_name: logicalName,
          directory_name: logicalName === "venv" ? ".venv" : logicalName,
          config_fingerprint: "config-test",
          toolchain_fingerprint: "toolchain-test",
          target_fingerprint: "target-test",
          environment_fingerprint: "environment-test",
          lockfile_summary: null,
        },
      })));
    } else if (request.type === "repl_packages") {
      if (this.packageErrorOnce && !this.failedPackageView) {
        this.failedPackageView = true;
        this.stdout.write(Buffer.from(encodeFrame({
          type: "error", request_id: request.request_id, exit_code: 2,
          error: { code: "X11-REPL-PACKAGE-002", message: "环境暂时不可读" }, report: null,
        })));
        return;
      }
      this.stdout.write(Buffer.from(encodeFrame({
        type: "repl_packages_result", request_id: request.request_id,
        environment_path: request.active_environment ?? "", packages: [], interface: null,
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
  test("核心发现期间取消不会启动进程", async () => {
    const controller = new AbortController();
    let spawned = false;
    const client = new ProtocolClient({
      overridePath: process.execPath,
      spawnProcess: () => { spawned = true; return new FakeCore() as never; },
    });
    const pending = client.runSource("value = 1\n", { signal: controller.signal });
    controller.abort();
    await expect(pending).rejects.toMatchObject({ code: "X11-PROTOCOL-005" });
    expect(spawned).toBe(false);
  });

  test("握手前取消不会发送尚未存在的运行请求", async () => {
    const controller = new AbortController();
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore();
        controller.abort();
        return fake as never;
      },
    });
    await expect(client.runSource("value = 1\n", { signal: controller.signal }))
      .rejects.toMatchObject({ code: "X11-PROTOCOL-005" });
    expect(fake?.requests.map((value) => value.type)).toEqual(["hello"]);
    expect(fake?.killed).toBe(true);
  });

  test("运行帧写入期间取消仍会发送取消帧", async () => {
    const controller = new AbortController();
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore(() => controller.abort());
        return fake as never;
      },
    });
    const result = await client.runSource("value = 1\n", { signal: controller.signal });
    expect(result.response.type).toBe("error");
    expect(fake?.requests.map((value) => value.type)).toEqual(["hello", "run", "cancel", "shutdown"]);
  });

  test("旧核心未声明 package 能力时不发送新请求", async () => {
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore();
        return fake as never;
      },
    });
    const fixturePath = new URL("../../../../tests/spec/11x0-protocol/package-request.json", import.meta.url);
    const request = await Bun.file(fixturePath).json() as PackageRequest;
    await expect(client.call(request)).rejects.toMatchObject({ code: "X11-CLI-CORE-004" });
    expect(fake?.requests.map((value) => value.type)).toEqual(["hello"]);
  });

  test("旧核心只声明 package 而无 repl_packages 时关闭进程并降级为空视图", async () => {
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      overridePath: process.execPath,
      spawnProcess: () => { fake = new FakeCore(undefined, ["run", "environment", "package"]); return fake as never; },
    });
    const result = await client.replPackages("C:/project/dev");
    expect(result.response).toMatchObject({ type: "repl_packages_result", packages: [], interface: null });
    expect(fake?.requests.map((value) => value.type)).toEqual(["hello", "shutdown"]);
    expect(fake?.exitCode).toBe(0);
  });

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

  test("keepAlive 会复用核心和单次握手，显式 shutdown 才关闭", async () => {
    let fake: FakeCore | undefined;
    let spawnCount = 0;
    const client = new ProtocolClient({
      keepAlive: true, overridePath: process.execPath,
      spawnProcess: () => {
        spawnCount += 1;
        fake = new FakeCore();
        return fake as never;
      },
    });
    await client.runSource("first = 1\n");
    await client.runSource("second = 2\n");
    expect(spawnCount).toBe(1);
    expect(fake?.requests.map((value) => value.type)).toEqual(["hello", "run", "run"]);
    expect(fake?.exitCode).toBeNull();
    await client.shutdown();
    expect(fake?.requests.map((value) => value.type)).toEqual(["hello", "run", "run", "shutdown"]);
    expect(fake?.exitCode).toBe(0);
  });

  test("repl_packages 同一环境键只发送一次并允许并发调用共享结果", async () => {
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      keepAlive: true, overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore(undefined, ["run", "repl_packages"]);
        return fake as never;
      },
    });
    await Promise.all([client.replPackages("C:/project/dev", null, "en-US"), client.replPackages("C:/project/dev", null, "en-US")]);
    expect(fake?.requests.map((value) => value.type)).toEqual(["hello", "repl_packages"]);
    expect(fake?.requests.find((value) => value.type === "repl_packages")).toMatchObject({ locale: "en-US" });
    await client.shutdown();
  });

  test("repl_packages 返回错误后下次会重试而不复用旧诊断", async () => {
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      keepAlive: true, overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore(undefined, ["run", "repl_packages"], true);
        return fake as never;
      },
    });
    expect((await client.replPackages("C:/project/dev")).response.type).toBe("error");
    expect((await client.replPackages("C:/project/dev")).response.type).toBe("repl_packages_result");
    expect(fake?.requests.filter((request) => request.type === "repl_packages")).toHaveLength(2);
    await client.shutdown();
  });

  test("同一客户端的多个调用按提交顺序串行发送", async () => {
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      keepAlive: true, overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore();
        return fake as never;
      },
    });
    await Promise.all([client.runSource("first\n"), client.runSource("second\n")]);
    expect(fake?.requests.filter((value) => value.type === "run").map((value) => (value.source as { text: string }).text))
      .toEqual(["first\n", "second\n"]);
    await client.shutdown();
  });

  test("通信失败后可重新握手，旧核心的包视图缓存不会复用", async () => {
    const cores: FakeCore[] = [];
    let current: FakeCore | undefined;
    const client = new ProtocolClient({
      keepAlive: true, overridePath: process.execPath,
      spawnProcess: () => {
        current = cores.length === 0
          ? new FakeCore(() => current?.kill(), ["run", "repl_packages"])
          : new FakeCore(undefined, ["run", "repl_packages"]);
        cores.push(current);
        return current as never;
      },
    });
    await client.replPackages("C:/project/dev");
    await expect(client.runSource("first\n")).rejects.toMatchObject({ code: "X11-CLI-CORE-003" });
    await client.replPackages("C:/project/dev");
    const result = await client.runSource("second\n");
    expect(result.response.type).toBe("result");
    expect(cores.map((core) => core.requests.map((request) => request.type)))
      .toEqual([["hello", "repl_packages", "run"], ["hello", "repl_packages", "run"]]);
    await client.shutdown();
  });

  test("关闭后不能悄悄重启核心", async () => {
    const client = new ProtocolClient({ keepAlive: true, overridePath: process.execPath });
    await client.shutdown();
    await expect(client.runSource("value = 1\n")).rejects.toMatchObject({ code: "X11-CLI-CORE-003" });
  });

  test("debug 位和诊断配置按结构化字段传给 Rust 核心", async () => {
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore();
        return fake as never;
      },
    });
    await client.runSource("value = 1\n", {
      debug: true,
      diagnostics: { terminal_level: "trace", file_level: "debug", log_dir: "logs" },
    });
    const request = fake?.requests.find((value) => value.type === "run");
    expect(request).toMatchObject({
      type: "run",
      optimization: {
        level: 0,
        debug: true,
        diagnostics: { terminal_level: "trace", file_level: "debug", log_dir: "logs" },
      },
    });
  });

  test("buildSource 使用真实源码并传递工具链、配置和输出路径", async () => {
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore();
        return fake as never;
      },
    });
    const toolchain = {
      clang: "clang", llvm_as: null, llc: null, runtime_library: null,
      native_static_libraries: [], versions: { clang: "clang version 22", llvm_as: null, llc: null, rustc: null },
    };
    const result = await client.buildSource("value = 1\n", {
      path: "main.xiao", output: "build/main.exe", llvmIrOutput: "build/main.ll",
      toolchain, debug: true, configText: "[Runtime]\ncall_stack_depth = 64\n", locale: "en-US",
    });
    expect(result.response.type).toBe("result");
    expect(fake?.requests.find((value) => value.type === "build")).toMatchObject({
      type: "build", source: { path: "main.xiao", text: "value = 1\n" },
      output: "build/main.exe", llvm_ir_output: "build/main.ll", config_text: "[Runtime]\ncall_stack_depth = 64\n",
      locale: "en-US",
      optimization: { level: 0, debug: true },
    });
  });

  test("testSources 按输入顺序批量传递源码和每用例期限", async () => {
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore();
        return fake as never;
      },
    });
    const result = await client.testSources([
      { module: "tests/z-last", path: "tests/z-last.xiao", text: "value = 1\n" },
      { module: "tests/a-first", path: "tests/a-first.xiao", text: "value = 1\n" },
    ], { timeoutMs: 25, locale: "en-US" });
    expect(result.response.type).toBe("test_result");
    const request = fake?.requests.find((value) => value.type === "test");
    expect(request).toMatchObject({
      type: "test",
      locale: "en-US",
      options: { timeout_ms: 25 },
      cases: [
        { module: "tests/z-last", path: "tests/z-last.xiao" },
        { module: "tests/a-first", path: "tests/a-first.xiao" },
      ],
    });
  });

  test("environmentMetadata 传递项目布局、配置和工具链并返回元数据", async () => {
    let fake: FakeCore | undefined;
    const client = new ProtocolClient({
      overridePath: process.execPath,
      spawnProcess: () => {
        fake = new FakeCore();
        return fake as never;
      },
    });
    const result = await client.environmentMetadata({
      projectRoot: "C:/project",
      logicalName: "dev",
      configText: "[project]\nname = \"demo\"\n",
      locale: "en-US",
      target: { triple: "x86_64-pc-windows-msvc", pointer_width: 64, endian: "little", object_format: "coff" },
      toolchain: {
        clang: "clang", llvm_as: null, llc: null, runtime_library: null,
        native_static_libraries: [], rustc: null,
        versions: { clang: "clang 18", llvm_as: null, llc: null, rustc: null },
      },
    });
    expect(result.response.type).toBe("environment_result");
    expect(fake?.requests.find((value) => value.type === "environment")).toMatchObject({
      type: "environment",
      project_root: "C:/project",
      logical_name: "dev",
      config_text: "[project]\nname = \"demo\"\n",
      locale: "en-US",
    });
    expect((result.response as { metadata: { environment_fingerprint: string } }).metadata.environment_fingerprint).toBe("environment-test");
  });
});
