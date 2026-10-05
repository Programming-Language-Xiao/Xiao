/** 核心命令执行器；把解析结果接到配置编辑器和 Rust 协议客户端。 */

import { readFile } from "node:fs/promises";
import { basename, resolve } from "node:path";

import { findProjectConfig, writeConfigValue, type ConfigEditorOptions } from "../config/editor.ts";
import { resolveEffectiveLocale, type LocaleContext } from "../config/locale.ts";
import { resolveOptimization } from "../config/optimization.ts";
import { ProtocolClient, type CoreClientOptions } from "../protocol/client.ts";
import type { ToolchainSpec } from "../protocol/messages.ts";
import { renderCliError, renderProtocolResponse, CLI_EXIT_CODES, type DiagnosticRenderOptions, type RenderedDiagnostic } from "../diagnostics/render.ts";
import { helpText as parserHelpText, type ParsedCommand } from "./parser.ts";
import { discoverProjectTests, testModuleName } from "./test-discovery.ts";
import { discoverToolchainWithMetadata, ToolchainDiscoveryError } from "../platform/toolchain.ts";
import {
  createEnvironment,
  shellInitScript,
  EnvironmentCommandError,
} from "../environments/index.ts";
import { requestActivation } from "../environments/activation.ts";
import { editShellProfile } from "../environments/profile.ts";
import { executePackageCommand } from "../packages/index.ts";
import { cliMessage } from "../i18n.ts";
import { FileAssociationError, manageFileAssociation, noAssociationPrompt, type AssociationPlatform } from "../platform/file-association.ts";

/** 命令执行上下文；IO 由入口注入，便于管道和测试。 */
export interface CommandContext {
  /** 单次 CLI 调用创建、向下透传的有效语言。 */
  locale?: LocaleContext;
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
  /** 环境创建时可注入的工具链描述；未提供时由 CLI 发现。 */
  environmentToolchain?: ToolchainSpec;
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
  if (command.kind === "help") return { stdout: helpText(context.locale?.tag), stderr: "", exitCode: 0 };
  if (command.kind === "version") return { stdout: "xiao 0.1.0\n", stderr: "", exitCode: 0 };
  if (command.kind === "repl") {
    const message = command.multiline ? "多行会话须由 CLI 入口提供标准输入与输出"
      : "交互会话须由 CLI 入口提供标准输入与输出";
    return renderCliError(new CliCommandError("X11-CLI-REPL-001", message), renderOptions(command.options, context));
  }
  if (command.kind === "test") return executeTest(command, context);
  if (command.kind === "venv") return executeVenv(command, context);
  if (command.kind === "sync" || command.kind === "install" || command.kind === "lock" || command.kind === "update" || command.kind === "add" || command.kind === "remove") {
    try {
      return await executePackageCommand(command, context, renderOptions(command.options, context));
    } catch (error) {
      return renderCliError(error, renderOptions(command.options, context));
    }
  }
  if (command.kind === "shell-init") return executeShellInit(command, context);
  if (command.kind === "deactivate") return executeDeactivate(command);
  if (command.kind === "build") {
    return executeBuild(command, context);
  }
  if (command.kind === "xar") return executeArchive(command, context);
  if (command.kind === "verify") return executeVerify(command, context);
  if (command.kind === "cache") return executeCache(command, context);
  if (command.kind === "association") return executeAssociation(command, context);
  if (command.kind === "config") return executeConfig(command, context);
  if (command.kind === "run" && command.file.toLocaleLowerCase("en-US").endsWith(".xiaoc")) {
    return executeXiaoc(command, context);
  }
  return executeRun(command, context);
}

/** 创建项目环境并写入 E0 最小元数据。Shell 前缀由已求值的钩子负责。 */
async function executeVenv(
  command: Extract<ParsedCommand, { kind: "venv" }>,
  context: CommandContext,
): Promise<RenderedDiagnostic> {
  try {
    const created = await createEnvironment(context.cwd ?? process.cwd(), command.name, {
      corePath: context.corePath,
      env: context.env,
      executablePath: context.executablePath,
      spawnProcess: context.spawnProcess,
      toolchain: context.environmentToolchain,
      locale: context.locale?.tag,
      signal: context.signal,
    });
    await requestActivation(created.path, context.env ?? process.env);
    if (command.options.json) {
      return {
        stdout: `${JSON.stringify({
          type: "environment_created",
          logical_name: created.logicalName,
          directory_name: created.directoryName,
          path: created.path,
          project_root: created.projectRoot,
          metadata_path: created.metadataPath,
        })}\n`,
        stderr: "",
        exitCode: 0,
      };
    }
    const locale = context.locale?.tag ?? "zh-CN";
    const activationMissing = !(context.env ?? process.env).XIAO_ACTIVATION_FILE;
    const lines = [
      cliMessage("xiao.cli.env.created", locale, { name: created.logicalName, path: created.path }),
      ...(activationMissing ? [cliMessage("xiao.cli.env.activation_missing", locale)] : []),
    ];
    return {
      stdout: `${lines.join("\n")}\n`,
      stderr: "",
      exitCode: 0,
    };
  } catch (error) {
    return renderCliError(error, renderOptions(command.options, context));
  }
}

/** 默认输出脚本；只有显式安装或移除才会改写指定 profile。 */
async function executeShellInit(
  command: Extract<ParsedCommand, { kind: "shell-init" }>,
  context: CommandContext,
): Promise<RenderedDiagnostic> {
  try {
    if (command.action !== "print") {
      const result = await editShellProfile(command.shell, command.action, command.profile, context.env ?? process.env);
      if (command.options.json) return { stdout: `${JSON.stringify({ type: "shell_profile", shell: command.shell, ...result })}\n`, stderr: "", exitCode: 0 };
      const locale = context.locale?.tag ?? "zh-CN";
      const status = result.changed
        ? result.action === "install"
          ? cliMessage("xiao.cli.shell.hook.installed", locale, { profile: result.profile })
          : cliMessage("xiao.cli.shell.hook.removed", locale, { profile: result.profile })
        : cliMessage("xiao.cli.shell.hook.unchanged", locale, { profile: result.profile });
      const backup = result.backup === null
        ? cliMessage("xiao.cli.shell.no_backup", locale)
        : cliMessage("xiao.cli.shell.backup", locale, { path: result.backup });
      return { stdout: `${status}\n${backup}\n`, stderr: "", exitCode: 0 };
    }
    const script = shellInitScript(command.shell);
    if (command.options.json) return { stdout: `${JSON.stringify({ type: "shell_init", shell: command.shell, script })}\n`, stderr: "", exitCode: 0 };
    return { stdout: script, stderr: "", exitCode: 0 };
  } catch (error) {
    return renderCliError(error, renderOptions(command.options, context));
  }
}

/** 取消激活由 Shell 钩子消费；直接运行时不伪造父 Shell 状态。 */
function executeDeactivate(command: Extract<ParsedCommand, { kind: "deactivate" }>): RenderedDiagnostic {
  if (command.options.json) {
    return { stdout: `${JSON.stringify({ type: "deactivate", applied: false })}\n`, stderr: "", exitCode: 0 };
  }
  return { stdout: "", stderr: "", exitCode: 0 };
}

/** 发现、读取并通过项目测试协议执行所有测试源码。 */
async function executeTest(command: Extract<ParsedCommand, { kind: "test" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  let locale: LocaleContext;
  try {
    locale = context.locale ?? await resolveEffectiveLocale({ cwd, env: context.env });
  } catch (error) {
    return renderCliError(error, renderOptions(command.options, context));
  }
  const options = { ...renderOptions(command.options, context), locale: locale.tag };
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
    const normalizedOptimization = await resolveOptimization({
      cwd,
      env: context.env,
      cli: {
        ...(command.optimizationExplicit ? { level: command.optimizationLevel } : {}),
        locale: locale.tag,
      },
    });
    const result = await client.testSources(sources, {
      timeoutMs: command.timeoutMs ?? null,
      optimizationLevel: normalizedOptimization.level,
      signal: context.signal,
      locale: locale.tag,
    });
    return renderProtocolResponse(result.response, options);
  } catch (error) {
    return renderCliError(error, options);
  }
}

/** 返回入口使用的帮助文本，避免命令解析器和执行器各维护一份。 */
export function helpText(locale?: LocaleContext["tag"]): string {
  return parserHelpText(locale);
}

/** 读取源码并通过协议客户端执行 `run`。 */
async function executeRun(command: Extract<ParsedCommand, { kind: "run" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  const options = renderOptions(command.options, context);
  let locale: LocaleContext;
  try {
    locale = context.locale ?? await resolveEffectiveLocale({ cwd, env: context.env });
  } catch (error) {
    return renderCliError(error, options);
  }
  const path = resolve(cwd, command.file);
  let source: string;
  try {
    const bytes = await readFile(path);
    source = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch (error) {
    return renderCliError(new CliCommandError("X11-CLI-FILE-001", `无法读取源码 ${path}：${String(error)}`, CLI_EXIT_CODES.usage, { path }), renderOptions(command.options, context));
  }
  try {
    const normalizedOptimization = await resolveOptimization({
      cwd,
      env: context.env,
      cli: {
        ...(command.optimizationExplicit ? { level: command.optimizationLevel } : {}),
        ...(command.options.debug ? { debugInfo: true } : {}),
        locale: locale.tag,
      },
    });
    const client = new ProtocolClient({
      cwd: context.cwd,
      env: context.env,
      overridePath: context.corePath,
      spawnProcess: context.spawnProcess,
    });
    const result = await client.runSource(source, {
      path,
      module: moduleFromPath(path),
      optimizationLevel: normalizedOptimization.level,
      debug: command.options.debug,
      diagnostics: normalizedOptimization.config.diagnostics,
      signal: context.signal,
      locale: locale.tag,
    });
    return renderProtocolResponse(result.response, { ...options, locale: locale.tag });
  } catch (error) {
    return renderCliError(error, renderOptions(command.options, context));
  }
}

/** 通过 Rust 核心运行已验证的 `.xiaoc`，不在 CLI 解码或执行产物。 */
async function executeXiaoc(command: Extract<ParsedCommand, { kind: "run" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  const options = renderOptions(command.options, context);
  if (command.optimizationExplicit && command.optimizationLevel !== 0) {
    return renderCliError(
      new CliCommandError("X11-CLI-OPT-002", "已编译 `.xiaoc` 不能在运行时重新选择优化级别", CLI_EXIT_CODES.usage, {
        path: command.file,
        optimization_level: command.optimizationLevel,
      }),
      options,
    );
  }
  let locale: LocaleContext;
  try {
    locale = context.locale ?? await resolveEffectiveLocale({ cwd, env: context.env });
  } catch (error) {
    return renderCliError(error, options);
  }
  const path = resolve(cwd, command.file);
  try {
    const normalizedOptimization = await resolveOptimization({
      cwd,
      env: context.env,
      cli: {
        ...(command.options.debug ? { debugInfo: true } : {}),
        locale: locale.tag,
      },
    });
    const client = new ProtocolClient({
      cwd,
      env: context.env,
      overridePath: context.corePath,
      executablePath: context.executablePath,
      spawnProcess: context.spawnProcess,
    });
    const result = await client.runXiaoc(path, {
      debug: command.options.debug,
      diagnostics: normalizedOptimization.config.diagnostics,
      locale: locale.tag,
      signal: context.signal,
    });
    return renderProtocolResponse(result.response, { ...options, locale: locale.tag });
  } catch (error) {
    return renderCliError(error, options);
  }
}

/** 通过唯一的 `run_archive` 协议操作运行 `.xar`；不在 CLI 猜测入口或依赖。 */
async function executeArchive(command: Extract<ParsedCommand, { kind: "xar" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  const options = renderOptions(command.options, context);
  let locale: LocaleContext;
  try {
    locale = context.locale ?? await resolveEffectiveLocale({ cwd, env: context.env });
  } catch (error) {
    return renderCliError(error, options);
  }
  const path = resolve(cwd, command.file);
  try {
    const normalizedOptimization = await resolveOptimization({
      cwd,
      env: context.env,
      cli: {
        ...(command.optimizationExplicit ? { level: command.optimizationLevel } : {}),
        ...(command.options.debug ? { debugInfo: true } : {}),
        locale: locale.tag,
      },
    });
    const client = new ProtocolClient({
      cwd,
      env: context.env,
      overridePath: context.corePath,
      executablePath: context.executablePath,
      spawnProcess: context.spawnProcess,
    });
    const result = await client.runArchive(path, {
      optimizationLevel: normalizedOptimization.level,
      debug: command.options.debug,
      diagnostics: normalizedOptimization.config.diagnostics,
      locale: locale.tag,
      signal: context.signal,
    });
    return renderProtocolResponse(result.response, { ...options, locale: locale.tag });
  } catch (error) {
    if (typeof error === "object" && error !== null && "code" in error
      && typeof (error as { code?: unknown }).code === "string"
      && (error as { code: string }).code.startsWith("X11-CLI-CORE")) {
      return renderCliError(
        new CliCommandError("X11-CLI-XAR-ASSOCIATION-001", noAssociationPrompt(locale.tag), CLI_EXIT_CODES.usage, { archive: path }),
        options,
      );
    }
    return renderCliError(error, options);
  }
}

/** 读取源码、发现工具链并通过协议执行 `build`。 */
async function executeBuild(command: Extract<ParsedCommand, { kind: "build" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  const path = resolve(cwd, command.file);
  let locale: LocaleContext;
  try {
    locale = context.locale ?? await resolveEffectiveLocale({ cwd, env: context.env });
  } catch (error) {
    return renderCliError(error, renderOptions(command.options, context));
  }
  const options = { ...renderOptions(command.options, context), locale: locale.tag };
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
    const normalizedOptimization = await resolveOptimization({
      cwd,
      env: context.env,
      cli: {
        ...(command.optimizationExplicit ? { level: command.optimizationLevel } : {}),
        ...(command.options.debug ? { debugInfo: true } : {}),
        locale: locale.tag,
      },
    });
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
      optimizationLevel: normalizedOptimization.level,
      output: resolve(cwd, command.output),
      llvmIrOutput: command.llvmIrOutput === null ? null : resolve(cwd, command.llvmIrOutput),
      toolchain: toolchain.toolchain,
      debug: command.options.debug,
      diagnostics: normalizedOptimization.config.diagnostics,
      configText,
      locale: locale.tag,
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
    return {
      stdout: `${cliMessage("xiao.cli.config.updated", context.locale?.tag ?? "zh-CN", {
        path: result.path,
        key: command.key,
        value: String(result.value),
      })}\n`,
      stderr: "",
      exitCode: 0,
    };
  } catch (error) {
    return renderCliError(error, renderOptions(command.options, context));
  }
}

/** 通过 Rust 校验器验证 `.xiaoc` 或 `.xar`，不在 CLI 重写格式逻辑。 */
async function executeVerify(command: Extract<ParsedCommand, { kind: "verify" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  const options = renderOptions(command.options, context);
  try {
    const client = new ProtocolClient({
      cwd,
      env: context.env,
      overridePath: context.corePath,
      executablePath: context.executablePath,
      spawnProcess: context.spawnProcess,
    });
    const result = await client.verify(resolve(cwd, command.file), {
      detail: command.detail,
      signal: context.signal,
    });
    return renderProtocolResponse(result.response, options);
  } catch (error) {
    return renderCliError(error, options);
  }
}

/** 通过 Rust 16B API 查询或执行缓存维护；clean 默认只返回计划。 */
async function executeCache(command: Extract<ParsedCommand, { kind: "cache" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  const options = renderOptions(command.options, context);
  try {
    const client = new ProtocolClient({
      cwd: context.cwd,
      env: context.env,
      overridePath: context.corePath,
      executablePath: context.executablePath,
      spawnProcess: context.spawnProcess,
    });
    const result = await client.cache({ action: command.action, apply: command.apply, signal: context.signal });
    return renderProtocolResponse(result.response, options);
  } catch (error) {
    return renderCliError(error, options);
  }
}

/** 执行三平台 `.xar` 文件关联；详细日志永远写 stderr。 */
async function executeAssociation(command: Extract<ParsedCommand, { kind: "association" }>, context: CommandContext): Promise<RenderedDiagnostic> {
  try {
    const result = await manageFileAssociation(command.action, {
      platform: command.platform as AssociationPlatform | undefined,
      executablePath: command.executable ?? context.executablePath,
      env: context.env,
    });
    const payload = {
      type: "result",
      request_id: "association",
      operation: "association",
      exit_code: 0,
      exit_name: "success",
      diagnostics: [],
      report: null,
      events: [],
      metrics: null,
      value: { kind: "association", value: JSON.stringify(result) },
      artifact: null,
      audit: null,
      cache: null,
    };
    const verbose = command.options.verbose
      ? `${JSON.stringify({ type: "association_log", action: command.action, platform: result.platform, registration: result.registration, changed: result.changed })}\n`
      : "";
    if (command.options.json) return { stdout: `${JSON.stringify(payload)}\n`, stderr: verbose, exitCode: 0 };
    const locale = context.locale?.tag ?? "zh-CN";
    const status = result.installed
      ? (result.matches ? "installed" : "mismatch")
      : "absent";
    const stdout = `${cliMessage("xiao.cli.association.result", locale, { action: command.action, platform: result.platform, status })}\n`;
    const gate = result.gated ? `${cliMessage("xiao.cli.association.gated", locale)}\n` : "";
    return { stdout: `${stdout}${gate}`, stderr: verbose, exitCode: 0 };
  } catch (error) {
    if (command.options.json && error instanceof FileAssociationError) {
      return {
        stdout: `${JSON.stringify({
          type: "error",
          request_id: null,
          error: {
            code: error.code,
            message_id: "x11.association.failed",
            message: error.message.replace(`${error.code}: `, ""),
            text: null,
            phase: "association",
            next_step: "检查已安装的 xiao 路径和当前用户权限",
            details: error.details,
          },
          report: null,
          exit_code: error.exitCode,
        })}\n`,
        stderr: "",
        exitCode: error.exitCode,
      };
    }
    return renderCliError(error, renderOptions(command.options, context));
  }
}

/** 把命令选项和宿主能力转换为呈现参数。 */
function renderOptions(options: { json: boolean; color: "auto" | "always" | "never" }, context: CommandContext): DiagnosticRenderOptions {
  return {
    json: options.json,
    locale: context.locale?.tag,
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
