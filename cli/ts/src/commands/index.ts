/** 核心命令执行器；把解析结果接到配置编辑器和 Rust 协议客户端。 */

import { readFile } from "node:fs/promises";
import { basename, resolve } from "node:path";

import { findProjectConfig, writeConfigValue, type ConfigEditorOptions } from "../config/editor.ts";
import { ProtocolClient, type CoreClientOptions } from "../protocol/client.ts";
import { renderCliError, renderProtocolResponse, CLI_EXIT_CODES, type DiagnosticRenderOptions, type RenderedDiagnostic } from "../diagnostics/render.ts";
import { helpText as parserHelpText, type ParsedCommand } from "./parser.ts";
import { discoverProjectTests, testModuleName } from "./test-discovery.ts";
import { discoverToolchainWithMetadata, ToolchainDiscoveryError } from "../platform/toolchain.ts";

/** 命令执行上下文；IO 由入口注入，便于管道和测试。 */
export interface CommandContext {
  /** 当前工作目录。 */
  cwd?: string;
  /** 环境变量。 */
  env?: NodeJS.ProcessEnv;
  /** Rust 核心路径覆盖。 */
  corePath?: string;
  /** 输出是否连接 TTY。 */
  isTTY?: boolean;
  /** 测试用核心启动器。 */
  spawnProcess?: CoreClientOptions["spawnProcess"];
  /** CLI 可执行文件路径，用于工具链/诊断组件相邻发现。 */
  executablePath?: string;
  /** 当前命令的取消信号。 */
  signal?: AbortSignal;
}

/** 命令本身尚未进入本批的稳定诊断。 */
export class CliCommandError extends Error {
  /** 稳定编号。 */
  readonly code: string;
  /** 建议的进程码。 */
  readonly exitCode: number;
  /** 结构化附加字段。 */
  readonly details: Record<string, unknown>;

  /** 创建命令诊断。 */
  constructor(code: string, message: string, exitCode = CLI_EXIT_CODES.usage, details: Record<string, unknown> = {}) {
    super(`${code}: ${message}`);
    this.name = "CliCommandError";
    this.code = code;
    this.exitCode = exitCode;
    this.details = details;
  }
}

/** 执行一条已解析命令并返回终端输出。 */
export async function executeCommand(command: ParsedCommand, context: CommandContext = {}): Promise<RenderedDiagnostic> {
  if (command.kind === "help") return { stdout: helpText(), stderr: "", exitCode: 0 };
  if (command.kind === "version") return { stdout: "xiao 0.1.0\n", stderr: "", exitCode: 0 };
  if (command.kind === "repl") {
    return renderCliError(new CliCommandError("X11-CLI-REPL-001", "交互式解释器尚未在本批实现；请使用 xiao run <file.xiao>"), renderOptions(command.options, context));
  }
  if (command.kind === "test") return executeTest(command, context);
  if (command.kind === "build") {
    return executeBuild(command, context);
  }
  if (command.kind === "config") return executeConfig(command, context);
  return executeRun(command, context);
}

/** 发现、读取并通过项目测试协议执行所有测试源码。 */
async function executeTest(command: Extract<ParsedCommand, { kind: "test" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  const options = renderOptions(command.options, context);
  try {
    const files = await discoverProjectTests(command.project ?? ".", cwd);
    const sources = [] as Array<{ module: string; path: string; text: string }>;
    for (const file of files) {
      try {
        const bytes = await readFile(file.absolutePath);
        const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
        sources.push({ module: testModuleName(file.relativePath), path: file.relativePath, text });
      } catch (error) {
        return renderCliError(new CliCommandError(
          "X11-CLI-FILE-001",
          `无法读取测试源码 ${file.absolutePath}：${String(error)}`,
          CLI_EXIT_CODES.usage,
          { path: file.absolutePath, relative_path: file.relativePath },
        ), options);
      }
    }
    const client = new ProtocolClient({
      cwd,
      env: context.env,
      overridePath: context.corePath,
      executablePath: context.executablePath,
      spawnProcess: context.spawnProcess,
    });
    const result = await client.testSources(sources, {
      timeoutMs: command.timeoutMs ?? null,
      signal: context.signal,
    });
    return renderProtocolResponse(result.response, options);
  } catch (error) {
    return renderCliError(error, options);
  }
}

/** 返回入口使用的帮助文本，避免命令解析器和执行器各维护一份。 */
export function helpText(): string {
  return parserHelpText();
}

/** 读取源码并通过协议客户端执行 `run`。 */
async function executeRun(command: Extract<ParsedCommand, { kind: "run" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  const path = resolve(context.cwd ?? process.cwd(), command.file);
  let source: string;
  try {
    const bytes = await readFile(path);
    source = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch (error) {
    return renderCliError(new CliCommandError("X11-CLI-FILE-001", `无法读取源码 ${path}：${String(error)}`, CLI_EXIT_CODES.usage, { path }), renderOptions(command.options, context));
  }
  try {
    const client = new ProtocolClient({
      cwd: context.cwd,
      env: context.env,
      overridePath: context.corePath,
      spawnProcess: context.spawnProcess,
    });
    const result = await client.runSource(source, {
      path,
      module: moduleFromPath(path),
      debug: command.options.debug,
      signal: context.signal,
    });
    return renderProtocolResponse(result.response, renderOptions(command.options, context));
  } catch (error) {
    return renderCliError(error, renderOptions(command.options, context));
  }
}

/** 读取源码、发现工具链并通过协议执行 `build`。 */
async function executeBuild(command: Extract<ParsedCommand, { kind: "build" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  const path = resolve(cwd, command.file);
  const options = renderOptions(command.options, context);
  let source: string;
  try {
    const bytes = await readFile(path);
    source = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch (error) {
    return renderCliError(new CliCommandError("X11-CLI-FILE-001", `无法读取源码 ${path}：${String(error)}`, CLI_EXIT_CODES.usage, { path }), options);
  }
  try {
    const configPath = await findProjectConfig(cwd);
    const configText = configPath === null ? null : await readFile(configPath, "utf8");
    const toolchain = await discoverToolchainWithMetadata({
      cwd,
      env: context.env,
      executablePath: context.executablePath,
      requireDiagnostics: command.options.debug,
    });
    const client = new ProtocolClient({
      cwd,
      env: context.env,
      overridePath: context.corePath,
      executablePath: context.executablePath,
      spawnProcess: context.spawnProcess,
    });
    const result = await client.buildSource(source, {
      path,
      module: moduleFromPath(path),
      output: resolve(cwd, command.output),
      llvmIrOutput: command.llvmIrOutput === null ? null : resolve(cwd, command.llvmIrOutput),
      toolchain: toolchain.toolchain,
      debug: command.options.debug,
      configText,
      signal: context.signal,
    });
    return renderProtocolResponse(result.response, options);
  } catch (error) {
    if (error instanceof ToolchainDiscoveryError) {
      return renderCliError(error, { ...options, json: command.options.json });
    }
    return renderCliError(error, options);
  }
}

/** 执行结构化配置写回并渲染结果。 */
async function executeConfig(command: Extract<ParsedCommand, { kind: "config" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  try {
    const options: ConfigEditorOptions = { cwd: context.cwd, env: context.env, globalPath: context.env?.XIAO_GLOBAL_CONFIG };
    const result = await writeConfigValue(command.global ? "global" : "project", command.key, command.value, options);
    if (command.options.json) {
      return { stdout: `${JSON.stringify({ type: "config", path: result.path, key: command.key, value: result.value })}\n`, stderr: "", exitCode: 0 };
    }
    return { stdout: `已更新 ${result.path}：${command.key} = ${String(result.value)}\n`, stderr: "", exitCode: 0 };
  } catch (error) {
    return renderCliError(error, renderOptions(command.options, context));
  }
}

/** 把命令选项和宿主能力转换为呈现参数。 */
function renderOptions(options: { json: boolean; color: "auto" | "always" | "never" }, context: CommandContext): DiagnosticRenderOptions {
  return {
    json: options.json,
    color: options.color,
    isTTY: context.isTTY ?? Boolean(process.stderr.isTTY),
    noColor: (context.env ?? process.env).NO_COLOR !== undefined,
    colorTerm: (context.env ?? process.env).COLORTERM,
    term: (context.env ?? process.env).TERM,
  };
}

/** 从 `.xiao` 文件名派生协议逻辑模块名。 */
function moduleFromPath(path: string): string {
  const name = basename(path);
  return name.toLowerCase().endsWith(".xiao") ? name.slice(0, -5) || "main" : name;
}
