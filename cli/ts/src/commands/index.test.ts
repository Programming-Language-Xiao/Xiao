/** 命令执行器的取消信号接线回归。 */

import { describe, expect, test } from "bun:test";
import { EventEmitter } from "node:events";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { PassThrough } from "node:stream";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { decodePayload, encodeFrame } from "../protocol/codec.ts";
import { executeCommand } from "./index.ts";
import { parseArguments } from "./parser.ts";

/** 为环境命令集成测试提供最小协议核心替身。 */
class EnvironmentFakeCore extends EventEmitter {
  readonly stdin = new PassThrough();
  readonly stdout = new PassThrough();
  readonly stderr = new PassThrough();
  exitCode: number | null = null;
  signalCode: NodeJS.Signals | null = null;
  killed = false;
  readonly requests: Record<string, unknown>[] = [];
  private buffer = new Uint8Array(0);

  /** 监听输入帧并返回环境协议响应。 */
  constructor() {
    super();
    this.stdin.on("data", (chunk: Buffer) => this.consume(chunk));
  }

  /** 模拟客户端强制终止核心进程。 */
  kill(): boolean {
    this.killed = true;
    this.exitCode = 4;
    this.stdin.end();
    this.stdout.end();
    queueMicrotask(() => this.emit("close", this.exitCode, this.signalCode));
    return true;
  }

  /** 聚合输入分块并逐帧分发。 */
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

  /** 根据请求类型返回握手、环境结果或关闭响应。 */
  private respond(request: Record<string, unknown>): void {
    this.requests.push(request);
    if (request.type === "hello") {
      this.stdout.write(Buffer.from(encodeFrame({
        type: "hello", request_id: request.request_id, accepted: true, protocol_version: 1,
        core_version: 1, versions: {}, capabilities: ["environment", "package"], error: null,
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
          config_fingerprint: "xiao-config-fingerprint-v1-test",
          toolchain_fingerprint: "xiao-fnv1a64-test",
          target_fingerprint: "xiao-target-fingerprint-v1-test",
          environment_fingerprint: "xiao-environment-fingerprint-v1-test",
          lockfile_summary: null,
        },
      })));
    } else if (request.type === "package") {
      this.stdout.write(Buffer.from(encodeFrame({
        type: "package_result", request_id: request.request_id,
        result: {
          environment_path: request.active_environment ?? "C:\\project\\.venv",
          created: request.operation === "sync", changed: true,
          activate: request.operation === "sync", lock_status: request.operation === "sync" ? "created" : null,
        },
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

const testToolchain = {
  clang: "clang",
  llvm_as: null,
  llc: null,
  runtime_library: null,
  native_static_libraries: [],
  rustc: null,
  versions: { clang: "clang 18", llvm_as: null, llc: null, rustc: null },
};

/** 构造带测试工具链和核心替身的命令上下文。 */
function environmentContext() {
  const fake = new EnvironmentFakeCore();
  return {
    corePath: process.execPath,
    environmentToolchain: testToolchain,
    fake,
    spawnProcess: () => fake as never,
  };
}

describe("命令取消接线", () => {
  test("sync 激活传 Rust 路径，install 与 i 完全共用协议，不写激活文件", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-activation."));
    const activationFile = join(directory, "activation.12345678");
    const config = "[project]\nname = \"app\"\nversion = \"0.1.0\"\n";
    await writeFile(join(directory, "config.xiao"), config);
    await writeFile(activationFile, "");
    try {
      const syncContext = environmentContext();
      const env = { ...process.env, XIAO_ACTIVATION_FILE: activationFile, XIAO_ACTIVE_ENV: "C:\\env\\dev" };
      const syncResult = await executeCommand(parseArguments(["sync", "--keep-extra", "--locked"]), { ...syncContext, cwd: directory, env });
      expect(syncResult.exitCode).toBe(0);
      expect(syncContext.fake.requests.find((value) => value.type === "package")).toMatchObject({
        operation: "sync", active_environment: "C:\\env\\dev", keep_extra: true, locked: true, frozen: false, config_text: config,
      });
      expect(await readFile(activationFile, "utf8")).toBe("XIAO_ACTIVE_ENV='C:\\env\\dev'\nexport XIAO_ACTIVE_ENV\n");
      const results = [];
      for (const alias of ["install", "i"]) {
        await writeFile(activationFile, "unchanged");
        const context = environmentContext();
        const result = await executeCommand(parseArguments([alias, "--json"]), { ...context, cwd: directory, env });
        expect(result.exitCode).toBe(0);
        const request = context.fake.requests.find((value) => value.type === "package");
        results.push({ response: JSON.parse(result.stdout), request: {
          operation: request?.operation, project_root: request?.project_root, config_text: request?.config_text,
          active_environment: request?.active_environment, keep_extra: request?.keep_extra,
          locked: request?.locked, frozen: request?.frozen,
        } });
        expect(await readFile(activationFile, "utf8")).toBe("unchanged");
      }
      expect(results[0]).toEqual(results[1]);
      expect(results[0]?.request).toMatchObject({ operation: "install", keep_extra: false, locked: false, frozen: false });
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("install/i 仅从当前目录或显式路径读取配置，不向上发现", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-install-target-"));
    const project = join(directory, "project");
    const nested = join(project, "nested");
    const config = "[project]\nname = \"chosen\"\nversion = \"0.1.0\"\n";
    await mkdir(nested, { recursive: true });
    await writeFile(join(project, "config.xiao"), config);
    try {
      for (const [alias, path] of [["install", ".."], ["i", "../config.xiao"]] as const) {
        const context = environmentContext();
        const result = await executeCommand(parseArguments([alias, path]), { ...context, cwd: nested });
        expect(result.exitCode).toBe(0);
        expect(context.fake.requests.find((request) => request.type === "package")).toMatchObject({
          operation: "install", project_root: project, config_text: config,
        });
      }
      const missing = await executeCommand(parseArguments(["install", "--json"]), { cwd: nested });
      expect(missing.exitCode).not.toBe(0);
      expect(JSON.parse(missing.stdout)).toMatchObject({ type: "error" });
      await writeFile(join(project, "other.xiao"), config);
      const unrelated = await executeCommand(parseArguments(["i", "../other.xiao", "--json"]), { cwd: nested });
      expect(unrelated.exitCode).toBe(64);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("锁和依赖编辑操作传递精确参数且不触发激活", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-package-edit-"));
    const project = join(directory, "project");
    const nested = join(project, "nested");
    const config = "[project]\nname = \"app\"\nversion = \"1\"\n";
    await mkdir(nested, { recursive: true });
    await writeFile(join(project, "config.xiao"), config);
    try {
      for (const [args, expected] of [
        [["lock"], { operation: "lock", package_name: null }],
        [["update"], { operation: "update", package_name: null }],
        [["add", "lib", "--path", "../lib", "--version", "1.2", "--dev"], {
          operation: "add", package_name: "lib", package_path: "../lib", package_version: "1.2", development: true,
        }],
        [["remove", "lib"], { operation: "remove", package_name: "lib", package_path: null, development: false }],
      ] as const) {
        const context = environmentContext();
        const result = await executeCommand(parseArguments(args), { ...context, cwd: nested });
        expect(result.exitCode).toBe(0);
        expect(context.fake.requests.find((request) => request.type === "package")).toMatchObject({
          project_root: project, config_text: config, ...expected,
        });
      }
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("预取消信号沿 runSource 传递并在启动核心前返回稳定错误", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-cancel-"));
    const sourcePath = join(directory, "main.xiao");
    await writeFile(sourcePath, "value = 1\n", "utf8");
    try {
      const result = await executeCommand(parseArguments(["run", sourcePath]), {
        cwd: directory,
        signal: AbortSignal.abort(),
      });
      expect(result.exitCode).toBe(2);
      expect(result.stderr).toContain("X11-PROTOCOL-005");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("项目没有测试文件时返回 CLI 自身的 usage 诊断", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-test-empty-"));
    await mkdir(join(directory, "tests"));
    try {
      const result = await executeCommand(parseArguments(["test", directory]), { cwd: directory });
      expect(result.exitCode).toBe(64);
      expect(result.stderr).toContain("X11-CLI-TEST-003");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("项目测试源码读取失败返回 CLI 文件诊断，不伪装成核心失败", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-test-file-"));
    const sourcePath = join(directory, "tests", "bad.xiao");
    await mkdir(join(directory, "tests"));
    await writeFile(sourcePath, Buffer.from([0xff, 0xfe]));
    try {
      const result = await executeCommand(parseArguments(["test", directory]), { cwd: directory });
      expect(result.exitCode).toBe(64);
      expect(result.stderr).toContain("X11-CLI-FILE-001");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("venv 从嵌套目录发现项目根、写入元数据并报告重复创建", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-venv-"));
    const project = join(directory, "project");
    const nested = join(project, "packages", "app");
    await mkdir(nested, { recursive: true });
    await writeFile(join(project, "config.xiao"), "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n", "utf8");
    try {
      const context = environmentContext();
      const created = await executeCommand(parseArguments(["venv", "dev", "--json"]), { cwd: nested, ...context });
      expect(created.exitCode).toBe(0);
      expect(context.fake.requests.find((value) => value.type === "environment")).toMatchObject({ logical_name: "dev" });
      const result = JSON.parse(created.stdout) as { path: string; logical_name: string };
      expect(result.logical_name).toBe("dev");
      expect(result.path).toBe(join(project, "dev"));
      const metadata = JSON.parse(await Bun.file(join(project, "dev", ".xiao-environment.json")).text()) as {
        metadata_version: number;
        logical_name: string;
        lockfile_summary: string | null;
      };
      expect(metadata).toMatchObject({ metadata_version: 1, logical_name: "dev", lockfile_summary: null });

      const duplicate = await executeCommand(parseArguments(["venv", "dev", "--json"]), { cwd: nested });
      expect(duplicate.exitCode).toBe(64);
      expect(JSON.parse(duplicate.stdout)).toMatchObject({ code: "X11-CLI-VENV-003" });
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("venv 默认目录回退 cwd；shell-init 与直接 deactivate 不改 Shell", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-cli-venv-default-"));
    try {
      const context = environmentContext();
      const created = await executeCommand(parseArguments(["venv"]), { cwd: directory, ...context });
      expect(created.exitCode).toBe(0);
      expect(created.stdout).toContain(join(directory, ".venv"));
      expect(context.fake.requests.find((value) => value.type === "environment")).toMatchObject({ logical_name: null });

      const shell = await executeCommand(parseArguments(["shell-init", "cmd"]), { cwd: directory });
      expect(shell.stdout).toContain("不支持由子进程修改父会话");
      expect(shell.stdout).not.toContain("AutoRun");
      const deactivate = await executeCommand(parseArguments(["deactivate"]), { cwd: directory });
      expect(deactivate.exitCode).toBe(0);
      expect(deactivate.stdout).toBe("");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
});
