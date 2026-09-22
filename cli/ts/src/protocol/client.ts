/** X0-B 的 Rust 核心进程客户端；只传输结构化协议消息。 */

import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";

import { decodePayload, encodeFrame, FRAME_LENGTH_BYTES, MAX_FRAME_BYTES, ProtocolFrameError } from "./codec.ts";
import {
  CORE_VERSION,
  PROTOCOL_VERSION,
  type HelloResponse,
  type ProtocolResponse,
  type ProtocolTarget,
  type RunRequest,
} from "./messages.ts";
import { discoverCoreWithMetadata, hostTarget, type CoreDiscoveryOptions, type CoreDiscoverySource } from "../platform/core.ts";

/** 核心客户端运行选项。 */
export interface CoreClientOptions extends CoreDiscoveryOptions {
  /** 当前进程工作目录。 */
  cwd?: string;
  /** 传给核心的环境变量。 */
  env?: NodeJS.ProcessEnv;
  /** 注入进程启动器，供契约测试使用。 */
  spawnProcess?: SpawnCoreProcess;
}

/** 源码运行请求的便捷参数。 */
export interface SourceRunOptions {
  /** 逻辑模块名，默认使用文件基名。 */
  module?: string;
  /** 源文件路径；用于诊断位置和协议 source.path。 */
  path?: string | null;
  /** 协议目标，默认当前宿主。 */
  target?: ProtocolTarget;
  /** 语言版本。 */
  languageVersion?: string;
  /** Runtime 版本。 */
  runtimeVersion?: string;
  /** VM 调用深度。 */
  maxCallDepth?: number;
  /** 事件容量。 */
  eventCapacity?: number;
  /** 驱动器超时（毫秒）。 */
  timeoutMs?: number | null;
  /** 调试位；X0-B 保留字段，不打开 X0-D 窗口。 */
  debug?: boolean;
  /** 取消信号。 */
  signal?: AbortSignal;
}

/** 核心进程启动器签名。 */
export type SpawnCoreProcess = (path: string, options: { cwd: string; env: NodeJS.ProcessEnv }) => ChildProcessWithoutNullStreams;

/** 一次核心调用返回的附加信息。 */
export interface CoreCallResult {
  /** 核心结构化响应。 */
  response: ProtocolResponse;
  /** 核心 stderr，供诊断详情显示，不参与结果判断。 */
  stderr: string;
  /** 实际使用的核心路径；用于调试和安装诊断。 */
  corePath: string;
  /** 核心路径来源；`development` 明确表示开发布局。 */
  coreSource: CoreDiscoverySource;
}

/** 核心协议或进程边界错误。 */
export class CoreClientError extends Error {
  /** 稳定机器错误码。 */
  readonly code: string;
  /** CLI 建议的进程码；后端响应的退出码仍以协议字段为准。 */
  readonly exitCode: number;
  /** 结构化附加字段。 */
  readonly details: Record<string, unknown>;

  /** 创建核心客户端错误。 */
  constructor(code: string, message: string, exitCode = 4, details: Record<string, unknown> = {}) {
    super(`${code}: ${message}`);
    this.name = "CoreClientError";
    this.code = code;
    this.exitCode = exitCode;
    this.details = details;
  }
}

/** 与 Rust 核心建立一次协议会话。 */
export class ProtocolClient {
  private readonly options: CoreClientOptions;
  private readonly spawnProcess: SpawnCoreProcess;

  /** 创建客户端。 */
  constructor(options: CoreClientOptions = {}) {
    this.options = options;
    this.spawnProcess = options.spawnProcess ?? ((path, spawnOptions) => spawn(path, [], {
      cwd: spawnOptions.cwd,
      env: spawnOptions.env,
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    }));
  }

  /** 使用真实 Xiao 源码发送一次 `run` 请求。 */
  async runSource(sourceText: string, options: SourceRunOptions = {}): Promise<CoreCallResult> {
    const target = options.target ?? hostTarget();
    const sourcePath = options.path ?? null;
    const module = options.module ?? moduleName(sourcePath);
    const request: RunRequest = {
      type: "run",
      request_id: requestId("run"),
      protocol_version: PROTOCOL_VERSION,
      core_version: CORE_VERSION,
      language_version: options.languageVersion ?? "0.1.0",
      runtime_version: options.runtimeVersion ?? "0.1.0",
      target,
      optimization: { level: 0, debug: options.debug ?? false },
      source: { module, path: sourcePath, text: sourceText },
      options: {
        max_call_depth: options.maxCallDepth ?? 1024,
        event_capacity: options.eventCapacity ?? 256,
        timeout_ms: options.timeoutMs ?? null,
      },
    };
    return this.call(request, options.signal);
  }

  /** 使用已经规范化的协议运行请求发送一次调用。 */
  async call(request: RunRequest, signal?: AbortSignal): Promise<CoreCallResult> {
    if (signal?.aborted) throw new CoreClientError("X11-PROTOCOL-005", "请求已取消", 2);
    const discovery = await discoverCoreWithMetadata(this.options);
    const corePath = discovery.path;
    const cwd = this.options.cwd ?? process.cwd();
    const env = { ...process.env, ...(this.options.env ?? {}) };
    let child: ChildProcessWithoutNullStreams;
    try {
      child = this.spawnProcess(corePath, { cwd, env });
    } catch (error) {
      throw new CoreClientError("X11-CLI-CORE-002", `无法启动 xiao-core：${String(error)}`, 4, { path: corePath });
    }

    const frames = new FrameQueue(child);
    child.on("error", (error) => frames.fail(error));
    let stderr = "";
    child.stderr.on("data", (chunk: Buffer | string) => {
      stderr += chunk.toString();
      if (stderr.length > 64 * 1024) stderr = stderr.slice(-64 * 1024);
    });
    const abortHandler = () => {
      // 取消帧发送失败时，主请求仍会由核心退出/EOF 转换为稳定错误。
      void this.sendCancel(child, request.request_id).catch(() => undefined);
    };
    signal?.addEventListener("abort", abortHandler, { once: true });
    try {
      await writeChildFrame(child, {
        type: "hello",
        request_id: requestId("hello"),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
      });
      const hello = await frames.next();
      if (hello === null) throw await processEndedError(child, stderr);
      const helloResponse = asHello(hello);
      if (!helloResponse.accepted) {
        const error = helloResponse.error;
        throw new CoreClientError(error?.code ?? "X11-PROTOCOL-004", error?.message ?? "核心版本不兼容", 2, {
          response: helloResponse,
        });
      }

      await writeChildFrame(child, request);
      const response = await frames.nextMatching(request.request_id);
      await writeChildFrame(child, {
        type: "shutdown",
        request_id: requestId("shutdown"),
        protocol_version: PROTOCOL_VERSION,
        core_version: CORE_VERSION,
      });
      await frames.nextMatching((responseValue) => responseValue.type === "shutdown");
      child.stdin.end();
      await waitForExit(child);
      return { response: asProtocolResponse(response), stderr, corePath, coreSource: discovery.source };
    } catch (error) {
      if (error instanceof CoreClientError) throw error;
      if (error instanceof ProtocolFrameError) {
        throw new CoreClientError(error.code, error.message, 2, { stderr });
      }
      throw new CoreClientError("X11-CLI-CORE-003", `核心进程通信失败：${String(error)}`, 4, {
        stderr,
        process_exit_code: child.exitCode,
      });
    } finally {
      signal?.removeEventListener("abort", abortHandler);
      if (child.exitCode === null && !child.killed) child.kill();
    }
  }

  /** 发送取消帧，把 AbortSignal 映射到核心请求 ID。 */
  private async sendCancel(child: ChildProcessWithoutNullStreams, targetRequestId: string): Promise<void> {
    await writeChildFrame(child, {
      type: "cancel",
      request_id: requestId("cancel"),
      protocol_version: PROTOCOL_VERSION,
      core_version: CORE_VERSION,
      target_request_id: targetRequestId,
    });
  }
}

/** 便捷函数：发现核心并运行一段源码。 */
export async function runCoreSource(sourceText: string, options: SourceRunOptions & CoreClientOptions = {}): Promise<CoreCallResult> {
  const client = new ProtocolClient(options);
  return client.runSource(sourceText, options);
}

/** 将核心 stdout 的分块字节流还原为完整协议 JSON。 */
class FrameQueue {
  private buffer = new Uint8Array(0);
  private readonly values: unknown[] = [];
  private readonly waiters: Array<{ resolve: (value: unknown | null) => void; reject: (error: unknown) => void }> = [];
  private ended = false;
  private failure: unknown = null;

  /** 启动异步 stdout 消费。 */
  constructor(child: ChildProcessWithoutNullStreams) {
    void this.consume(child);
  }

  /** 取得下一条已解码消息，EOF 返回 null。 */
  async next(): Promise<unknown | null> {
    if (this.values.length > 0) return this.values.shift();
    if (this.failure !== null) throw this.failure;
    if (this.ended) return null;
    return new Promise((resolve, reject) => this.waiters.push({ resolve, reject }));
  }

  /** 丢弃无关 request_id 的消息，直到找到目标响应。 */
  async nextMatching(predicate: string | ((value: ProtocolResponse) => boolean)): Promise<unknown> {
    while (true) {
      const value = await this.next();
      if (value === null) throw new CoreClientError("X11-CLI-CORE-003", "核心在返回完整响应前退出", 4);
      const response = asProtocolResponse(value);
      const matches = typeof predicate === "string" ? responseRequestId(response) === predicate : predicate(response);
      if (matches) return response;
    }
  }

  /** 将子进程启动/IO 错误广播给当前等待者。 */
  fail(error: unknown): void {
    this.finish(error);
  }

  /** 持续读取 stdout 并在 EOF 时检查截断帧。 */
  private async consume(child: ChildProcessWithoutNullStreams): Promise<void> {
    try {
      for await (const chunk of child.stdout) this.append(new Uint8Array(chunk as Buffer));
      if (this.buffer.byteLength !== 0) throw new ProtocolFrameError("X11-PROTOCOL-001", "核心输出帧被截断");
      this.finish(null);
    } catch (error) {
      this.finish(error);
    }
  }

  /** 追加一个分块并尽可能解析多条帧。 */
  private append(chunk: Uint8Array): void {
    const merged = new Uint8Array(this.buffer.byteLength + chunk.byteLength);
    merged.set(this.buffer);
    merged.set(chunk, this.buffer.byteLength);
    this.buffer = merged;
    while (true) {
      if (this.buffer.byteLength < FRAME_LENGTH_BYTES) return;
      const length = new DataView(this.buffer.buffer, this.buffer.byteOffset, FRAME_LENGTH_BYTES).getBigUint64(0, false);
      if (length > BigInt(MAX_FRAME_BYTES)) throw new ProtocolFrameError("X11-PROTOCOL-001", "核心输出帧长度超过上限");
      const total = FRAME_LENGTH_BYTES + Number(length);
      if (this.buffer.byteLength < total) return;
      const payload = this.buffer.slice(FRAME_LENGTH_BYTES, total);
      this.buffer = this.buffer.slice(total);
      this.push(decodePayload(payload));
    }
  }

  /** 把消息交给等待者或排入队列。 */
  private push(value: unknown): void {
    const waiter = this.waiters.shift();
    if (waiter) waiter.resolve(value);
    else this.values.push(value);
  }

  /** 结束队列并唤醒所有等待者。 */
  private finish(error: unknown): void {
    if (error !== null) this.failure = error;
    this.ended = true;
    while (this.waiters.length > 0) {
      const waiter = this.waiters.shift();
      if (!waiter) continue;
      if (error !== null) waiter.reject(error);
      else waiter.resolve(null);
    }
  }
}

/** 编码并写入一帧，等待 stdin 接受回调。 */
async function writeChildFrame(child: ChildProcessWithoutNullStreams, value: unknown): Promise<void> {
  const frame = encodeFrame(value);
  await new Promise<void>((resolve, reject) => {
    child.stdin.write(Buffer.from(frame), (error?: Error | null) => error ? reject(error) : resolve());
  });
}

/** 等待核心进程关闭；已退出的进程立即返回。 */
async function waitForExit(child: ChildProcessWithoutNullStreams): Promise<void> {
  if (child.exitCode !== null) return;
  await new Promise<void>((resolve) => child.once("close", () => resolve()));
}

/** 把版本协商前的核心 EOF 转换为稳定错误。 */
async function processEndedError(child: ChildProcessWithoutNullStreams, stderr: string): Promise<CoreClientError> {
  await waitForExit(child);
  return new CoreClientError("X11-CLI-CORE-003", "核心在版本协商前退出", 4, {
    process_exit_code: child.exitCode,
    signal: child.signalCode,
    stderr,
  });
}

/** 验证首条响应确实是 hello。 */
function asHello(value: unknown): HelloResponse {
  if (!isRecord(value) || value.type !== "hello" || typeof value.accepted !== "boolean") {
    throw new CoreClientError("X11-PROTOCOL-002", "核心返回的 hello 响应形状无效", 2);
  }
  return value as unknown as HelloResponse;
}

/** 验证未知 JSON 至少具备协议响应的 type 字段。 */
function asProtocolResponse(value: unknown): ProtocolResponse {
  if (!isRecord(value) || typeof value.type !== "string") throw new CoreClientError("X11-PROTOCOL-002", "核心返回消息缺少 type", 2);
  return value as unknown as ProtocolResponse;
}

/** 读取可选响应 request_id。 */
function responseRequestId(response: ProtocolResponse): string | null {
  return "request_id" in response && typeof response.request_id === "string" ? response.request_id : null;
}

/** 判断未知值是否可作为 JSON 对象读取。 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** 生成一次会话内唯一的请求编号。 */
function requestId(prefix: string): string {
  return `${prefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

/** 从路径末段推导逻辑模块名。 */
function moduleName(path: string | null): string {
  if (!path) return "main";
  const normalized = path.replaceAll("\\", "/");
  const file = normalized.slice(normalized.lastIndexOf("/") + 1);
  return file.toLowerCase().endsWith(".xiao") ? file.slice(0, -5) || "main" : file || "main";
}
