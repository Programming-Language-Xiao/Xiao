/** 核心命令执行器；把解析结果接到配置编辑器和 Rust 协议客户端。 */

import { readFile } from "node:fs/promises";
import { basename, resolve } from "node:path";

import { writeConfigValue, type ConfigEditorOptions } from "../config/editor.ts";
import { ProtocolClient, type CoreClientOptions } from "../protocol/client.ts";
import { renderCliError, renderProtocolResponse, CLI_EXIT_CODES, type DiagnosticRenderOptions, type RenderedDiagnostic } from "../diagnostics/render.ts";
import { helpText as parserHelpText, type ParsedCommand } from "./parser.ts";

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
  if (command.kind === "unsupported-debug") {
    return renderCliError(new CliCommandError("X11-CLI-DEBUG-001", "-debug 诊断窗口属于 X0-D，当前尚未实现"), renderOptions(command.options, context));
  }
  if (command.kind === "test") {
    return renderCliError(new CliCommandError("X11-CLI-TEST-001", "xiao test 已登记但尚无项目测试语义；请使用 cargo test 或 bun test", CLI_EXIT_CODES.usage, { status: "registered_unimplemented", project: command.project ?? null }), renderOptions(command.options, context));
  }
  if (command.kind === "build") {
    return renderCliError(new CliCommandError("X11-CLI-BUILD-001", "xiao build 的主机工具链发现和原生构建属于 X0-E，当前尚未实现", CLI_EXIT_CODES.usage, { status: "x0e_unimplemented", arguments: command.args }), renderOptions(command.options, context));
  }
  if (command.kind === "config") return executeConfig(command, context);
  return executeRun(command, context);
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
    const result = await client.runSource(source, { path, module: moduleFromPath(path) });
    return renderProtocolResponse(result.response, renderOptions(command.options, context));
  } catch (error) {
    return renderCliError(error, renderOptions(command.options, context));
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
