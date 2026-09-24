/** 11A-E0 项目环境布局、Shell 钩子和提示符状态模型。 */

import { mkdir, readFile, rmdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

import { findProjectConfig } from "../config/editor.ts";
import { ProtocolClient, type CoreClientOptions } from "../protocol/client.ts";
import type { EnvironmentMetadata, ProtocolTarget, ToolchainSpec } from "../protocol/messages.ts";
import { hostTarget } from "../platform/core.ts";
import { discoverToolchainWithMetadata } from "../platform/toolchain.ts";
import { createColorizer, stripAnsi, type ColorMode } from "../ui/color.ts";
import type { ShellName } from "../commands/parser.ts";

/** 环境元数据文件名；字段与 Rust `xiao-package` 的 E0 元数据保持一致。 */
export const ENVIRONMENT_METADATA_FILE = ".xiao-environment.json";
/** 当前环境元数据版本。 */
export const ENVIRONMENT_METADATA_VERSION = 1;

/** 环境目录布局。 */
export interface EnvironmentLayout {
  /** 用户在 Shell 中看到的逻辑名称。 */
  logicalName: string;
  /** 相对于项目根的目录名称。 */
  directoryName: string;
  /** 环境目录绝对路径。 */
  path: string;
  /** 项目根绝对路径。 */
  projectRoot: string;
}

/** CLI 创建环境时写入的最小元数据。 */
export type EnvironmentBootstrapMetadata = EnvironmentMetadata;

/** 环境创建所需的核心和工具链注入项。 */
export interface EnvironmentCreationOptions {
  /** Rust 核心路径覆盖。 */
  corePath?: CoreClientOptions["overridePath"];
  /** 传给核心和工具链发现的环境变量。 */
  env?: NodeJS.ProcessEnv;
  /** CLI 可执行文件路径，用于开发/分发工具链发现。 */
  executablePath?: string;
  /** 测试用核心启动器。 */
  spawnProcess?: CoreClientOptions["spawnProcess"];
  /** 显式目标条件。 */
  target?: ProtocolTarget;
  /** 显式工具链描述；未提供时由 CLI 发现。 */
  toolchain?: ToolchainSpec;
  /** 取消信号。 */
  signal?: AbortSignal;
}

/** 环境创建结果。 */
export interface CreatedEnvironment extends EnvironmentLayout {
  /** 元数据文件绝对路径。 */
  metadataPath: string;
}

/** 提示符状态；用于测试和嵌入式宿主，不依赖真实父终端。 */
export interface ShellPromptState {
  /** 当前提示符文本。 */
  prompt: string;
  /** 激活前保存的原提示符。 */
  originalPrompt?: string;
  /** 当前逻辑环境名称。 */
  environmentName?: string;
}

/** 颜色和宿主能力。 */
export interface PromptColorOptions {
  color?: ColorMode;
  isTTY?: boolean;
  noColor?: boolean;
  colorTerm?: string;
  term?: string;
}

/** CLI 环境错误；上层只消费稳定编号。 */
export class EnvironmentCommandError extends Error {
  /** 稳定诊断编号。 */
  readonly code: string;
  /** CLI 退出码。 */
  readonly exitCode: number;
  /** 结构化附加字段。 */
  readonly details: Record<string, unknown>;

  /** 创建环境命令错误。 */
  constructor(code: string, message: string, exitCode = 64, details: Record<string, unknown> = {}) {
    super(`${code}: ${message}`);
    this.name = "EnvironmentCommandError";
    this.code = code;
    this.exitCode = exitCode;
    this.details = details;
  }
}

/** 向上发现 `config.xiao` 所在目录；找不到时回退当前目录。 */
export async function findEnvironmentProjectRoot(cwd = process.cwd()): Promise<string> {
  const configPath = await findProjectConfig(cwd);
  return configPath === null ? resolve(cwd) : dirname(configPath);
}

/** 根据冻结规则构造默认或显式环境目录。 */
export function environmentLayout(projectRoot: string, name?: string): EnvironmentLayout {
  const root = resolve(projectRoot);
  const logicalName = name ?? "venv";
  const directoryName = name ?? ".venv";
  validateEnvironmentName(logicalName);
  return {
    logicalName,
    directoryName,
    path: join(root, directoryName),
    projectRoot: root,
  };
}

/** 创建环境目录并通过 Rust 核心写入真实指纹元数据；不联网、不执行项目代码。 */
export async function createEnvironment(cwd: string, name?: string, options: EnvironmentCreationOptions = {}): Promise<CreatedEnvironment> {
  const projectRoot = await findEnvironmentProjectRoot(cwd);
  const layout = environmentLayout(projectRoot, name);
  try {
    await mkdir(layout.path);
  } catch (error) {
    if (isNodeError(error, "EEXIST")) {
      throw new EnvironmentCommandError(
        "X11-CLI-VENV-003",
        `环境已存在：${layout.path}`,
        64,
        { path: layout.path, logical_name: layout.logicalName },
      );
    }
    throw new EnvironmentCommandError(
      "X11-CLI-VENV-004",
      `无法创建环境目录 ${layout.path}：${String(error)}`,
      70,
      { path: layout.path },
    );
  }

  const metadataPath = join(layout.path, ENVIRONMENT_METADATA_FILE);
  let metadata: EnvironmentBootstrapMetadata;
  try {
    const configPath = await findProjectConfig(projectRoot);
    const configText = configPath === null ? null : await readFile(configPath, "utf8");
    const toolchain = options.toolchain ?? (await discoverToolchainWithMetadata({
      cwd: projectRoot,
      env: options.env,
      executablePath: options.executablePath,
      probeLink: false,
    })).toolchain;
    const client = new ProtocolClient({
      cwd: projectRoot,
      env: options.env,
      overridePath: options.corePath,
      executablePath: options.executablePath,
      spawnProcess: options.spawnProcess,
    });
    const result = await client.environmentMetadata({
      projectRoot,
      logicalName: name ?? null,
      configText,
      target: options.target ?? hostTarget(),
      toolchain,
      signal: options.signal,
    });
    if (result.response.type === "error") {
      throw new EnvironmentCommandError(
        result.response.error.code,
        result.response.error.message,
        result.response.exit_code,
        { response: result.response },
      );
    }
    if (result.response.type !== "environment_result") {
      throw new EnvironmentCommandError(
        "X11-CLI-VENV-005",
        `核心返回了非环境元数据响应：${result.response.type}`,
        70,
        { response_type: result.response.type },
      );
    }
    metadata = result.response.metadata;
  } catch (error) {
    await rmdir(layout.path).catch(() => undefined);
    throw error;
  }
  try {
    await writeFile(metadataPath, `${JSON.stringify(metadata, null, 2)}\n`, "utf8");
  } catch (error) {
    await rmdir(layout.path).catch(() => undefined);
    throw new EnvironmentCommandError(
      "X11-CLI-VENV-004",
      `无法写入环境元数据 ${metadataPath}：${String(error)}`,
      70,
      { path: metadataPath },
    );
  }
  return { ...layout, metadataPath };
}

/** 生成一次性、幂等的 Shell 钩子；不写 profile。 */
export function shellInitScript(shell: ShellName, commandName = "xiao"): string {
  if (shell === "bash") return bashShellInitScript(commandName);
  if (shell === "powershell") return powershellShellInitScript(commandName);
  return [
    "# xiao shell-init cmd",
    "提示：cmd.exe 不支持由子进程修改父会话的 PROMPT。",
    "环境创建仍可使用 `xiao venv [name]`，但不会自动修改当前提示符。",
    "请改用 `xiao shell-init powershell` 或 bash 兼容 Shell。",
    "",
  ].join("\n");
}

/** 把环境名称渲染为提示符前缀；非 TTY/NO_COLOR/never 时不输出 ANSI。 */
export function environmentPromptPrefix(name: string, options: PromptColorOptions = {}): string {
  validateEnvironmentName(name);
  const text = `$${name}$ `;
  const isTTY = options.isTTY ?? Boolean(process.stdout.isTTY);
  const noColor = options.noColor ?? process.env.NO_COLOR !== undefined;
  const dumbTerminal = (options.term ?? process.env.TERM ?? "").toLowerCase() === "dumb";
  if (!isTTY || noColor || dumbTerminal || options.color === "never") return text;
  return createColorizer({
    mode: "always",
    isTTY: true,
    noColor: options.noColor,
    colorTerm: options.colorTerm,
    term: options.term,
  }).color("success", text);
}

/** 激活或切换测试状态；重复激活不会叠加前缀。 */
export function activatePromptState(
  state: ShellPromptState,
  name: string,
  options: PromptColorOptions = {},
): ShellPromptState {
  validateEnvironmentName(name);
  const originalPrompt = state.originalPrompt ?? removeEnvironmentPrefix(state.prompt);
  return {
    prompt: `${environmentPromptPrefix(name, options)}${originalPrompt}`,
    originalPrompt,
    environmentName: name,
  };
}

/** 取消激活并恢复进入环境前的提示符。 */
export function deactivatePromptState(state: ShellPromptState): ShellPromptState {
  return {
    prompt: state.originalPrompt ?? removeEnvironmentPrefix(state.prompt),
  };
}

/** 去掉一个已有的 `$环境名$ ` 前缀，供状态模型和钩子测试使用。 */
export function removeEnvironmentPrefix(prompt: string): string {
  const plain = stripAnsi(prompt);
  const match = /^\$[^$\r\n]*\$ /.exec(plain);
  if (match === null) return prompt;
  const offset = match[0].length;
  let consumed = 0;
  let index = 0;
  while (index < prompt.length && consumed < offset) {
    if (prompt[index] === "\u001b" && prompt[index + 1] === "[") {
      const end = prompt.indexOf("m", index + 2);
      if (end >= 0) {
        index = end + 1;
        continue;
      }
    }
    index += 1;
    consumed += 1;
  }
  while (index < prompt.length) {
    const length = ansiSequenceLength(prompt, index);
    if (length === 0) break;
    index += length;
  }
  return prompt.slice(index);
}

/** 校验环境名称；路径分隔符和控制字符不能进入环境目录或提示符。 */
function validateEnvironmentName(name: string): void {
  if (name.length === 0 || name === "." || name === ".." || /[\u0000-\u001F\u007F\\/]/u.test(name)) {
    throw new EnvironmentCommandError("X11-CLI-VENV-002", `非法环境名称：${name}`, 64, { name });
  }
}

/** 判断异常是否带有指定 Node.js 错误码。 */
function isNodeError(error: unknown, code: string): boolean {
  return typeof error === "object" && error !== null && "code" in error && (error as { code?: string }).code === code;
}

/** 返回提示符中 ANSI 颜色序列的长度。 */
function ansiSequenceLength(text: string, index: number): number {
  if (text[index] !== "\u001b" || text[index + 1] !== "[") return 0;
  const end = text.indexOf("m", index + 2);
  return end < 0 ? 0 : end - index + 1;
}

/** 生成 Bash 一次性初始化钩子。 */
function bashShellInitScript(commandName: string): string {
  return [
    "# xiao shell-init bash",
    "# 只定义当前会话函数，不修改任何 profile 文件。",
    "_xiao_activate_environment() {",
    "  local _xiao_name=\"$1\"",
    "  local _xiao_color=\"$2\"",
    "  if [[ -z \"${XIAO_ORIGINAL_PS1+x}\" ]]; then XIAO_ORIGINAL_PS1=\"$PS1\"; fi",
    "  PS1=\"$XIAO_ORIGINAL_PS1\"",
    "  if [[ \"$_xiao_color\" == 1 ]]; then",
    "    PS1=\"\\[\\033[32m\\]\\$${_xiao_name}\\$ \\[\\033[0m\\]${XIAO_ORIGINAL_PS1}\"",
    "  else",
    "    PS1=\"\\$${_xiao_name}\\$ ${XIAO_ORIGINAL_PS1}\"",
    "  fi",
    "  XIAO_ACTIVE_ENV=\"$_xiao_name\"",
    "}",
    "_xiao_deactivate_environment() {",
    "  if [[ -n \"${XIAO_ORIGINAL_PS1+x}\" ]]; then PS1=\"$XIAO_ORIGINAL_PS1\"; unset XIAO_ORIGINAL_PS1; fi",
    "  unset XIAO_ACTIVE_ENV",
    "}",
    "xiao() {",
    "  local _xiao_command=\"${1:-}\"",
    `  command ${commandName} \"$@\"; local _xiao_status=$?`,
    "  if [[ $_xiao_status -eq 0 && \"$_xiao_command\" == venv ]]; then",
    "    local _xiao_name=venv _xiao_arg",
    "    for _xiao_arg in \"${@:2}\"; do",
    "      [[ \"$_xiao_arg\" == --* ]] && continue",
    "      _xiao_name=\"$_xiao_arg\"; break",
    "    done",
    "    local _xiao_color=0 _xiao_arg _xiao_can_color=0",
    "    [[ -t 1 && -z \"${NO_COLOR:-}\" && \"${TERM:-}\" != dumb ]] && _xiao_can_color=1",
    "    [[ $_xiao_can_color -eq 1 ]] && _xiao_color=1",
    "    for _xiao_arg in \"$@\"; do",
    "      [[ \"$_xiao_arg\" == \"--color=never\" ]] && _xiao_color=0",
    "      [[ \"$_xiao_arg\" == \"--color=always\" && $_xiao_can_color -eq 1 ]] && _xiao_color=1",
    "    done",
    "    _xiao_activate_environment \"$_xiao_name\" \"$_xiao_color\"",
    "  elif [[ $_xiao_status -eq 0 && \"$_xiao_command\" == deactivate ]]; then",
    "    _xiao_deactivate_environment",
    "  fi",
    "  return $_xiao_status",
    "}",
    "",
  ].join("\n");
}

/** 生成兼容 PowerShell 5.1 的一次性初始化钩子。 */
function powershellShellInitScript(commandName: string): string {
  return [
    "# xiao shell-init powershell",
    "# 兼容 Windows PowerShell 5.1；只定义当前会话函数，不修改 profile。",
    "$script:XiaoOriginalPrompt = $null",
    "$script:XiaoActiveEnvironment = $null",
    "$script:XiaoColorEnvironment = $true",
    "function global:__xiao_activate_environment {",
    "  param([string]$Name, [bool]$UseColor = $true)",
    "  if ($null -eq $script:XiaoOriginalPrompt) { $script:XiaoOriginalPrompt = (Get-Command prompt -CommandType Function).ScriptBlock }",
    "  $script:XiaoActiveEnvironment = $Name",
    "  $script:XiaoColorEnvironment = $UseColor",
    "  function global:prompt {",
    "    $base = & $script:XiaoOriginalPrompt",
    "    $prefix = '$' + $script:XiaoActiveEnvironment + '$ '",
    "    if ($script:XiaoColorEnvironment) { $prefix = ([char]27).ToString() + '[32m' + $prefix + ([char]27).ToString() + '[0m' }",
    "    return $prefix + $base",
    "  }",
    "}",
    "function global:__xiao_deactivate_environment {",
    "  if ($null -ne $script:XiaoOriginalPrompt) { Set-Item Function:\\global:prompt -Value $script:XiaoOriginalPrompt }",
    "  $script:XiaoOriginalPrompt = $null",
    "  $script:XiaoActiveEnvironment = $null",
    "}",
    `function global:${commandName} {`,
    "  $first = if ($args.Count -gt 0) { [string]$args[0] } else { '' }",
    `  $executable = Get-Command ${commandName}.exe -CommandType Application -ErrorAction SilentlyContinue`,
    `  if ($null -eq $executable) { $executable = Get-Command ${commandName} -CommandType Application -ErrorAction SilentlyContinue }`,
    "  if ($null -eq $executable) { Write-Error '找不到 xiao 可执行文件'; return 70 }",
    "  & $executable.Source @args",
    "  $status = $LASTEXITCODE",
    "  if ($status -eq 0 -and $first -eq 'venv') {",
    "    $name = 'venv'",
    "    for ($index = 1; $index -lt $args.Count; $index += 1) { if ([string]$args[$index] -notlike '-*') { $name = [string]$args[$index]; break } }",
    "    $canColor = (-not [Console]::IsOutputRedirected -and $env:NO_COLOR -eq $null -and $env:TERM -ne 'dumb')",
    "    $useColor = ($canColor -and $args -notcontains '--color=never')",
    "    if ($canColor -and $args -contains '--color=always') { $useColor = $true }",
    "    __xiao_activate_environment $name $useColor",
    "  } elseif ($status -eq 0 -and $first -eq 'deactivate') {",
    "    __xiao_deactivate_environment",
    "  }",
    "  return $status",
    "}",
    "",
  ].join("\n");
}
