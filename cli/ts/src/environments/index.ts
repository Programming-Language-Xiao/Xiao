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
import { fishShellInitScript } from "./fish.ts";
import { zshShellInitScript } from "./zsh.ts";

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
  if (shell === "zsh") return zshShellInitScript(bashShellInitScript(commandName));
  if (shell === "fish") return fishShellInitScript(commandName);
  if (shell === "powershell") return powershellShellInitScript(commandName);
  if (shell === "cmd") return [
    "# xiao shell-init cmd",
    "提示：cmd.exe 不支持由子进程修改父会话的 PROMPT。",
    "环境创建仍可使用 `xiao venv [name]`，但不会自动修改当前提示符。",
    "手工激活请将 XIAO_ACTIVE_ENV 设为已创建/同步环境的绝对路径（cmd 示例：set \"XIAO_ACTIVE_ENV=C:\\project\\.venv\"）。",
    "请改用 `xiao shell-init powershell`、bash、zsh 或 fish。",
    "",
  ].join("\n");
  throw new EnvironmentCommandError("X11-CLI-SHELL-001", "不支持的 Shell；可选 bash、zsh、fish、powershell 或 cmd");
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
  const originalPrompt = state.originalPrompt ?? (state.environmentName === undefined ? state.prompt : removeEnvironmentPrefix(state.prompt));
  return {
    prompt: `${environmentPromptPrefix(name, options)}${originalPrompt}`,
    originalPrompt,
    environmentName: name,
  };
}

/** 取消激活并恢复进入环境前的提示符。 */
export function deactivatePromptState(state: ShellPromptState): ShellPromptState {
  return {
    prompt: state.originalPrompt ?? (state.environmentName === undefined ? state.prompt : removeEnvironmentPrefix(state.prompt)),
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
    "  export XIAO_ACTIVE_ENV=\"$3\"",
    "}",
    "_xiao_deactivate_environment() {",
    "  if [[ -n \"${XIAO_ORIGINAL_PS1+x}\" ]]; then PS1=\"$XIAO_ORIGINAL_PS1\"; unset XIAO_ORIGINAL_PS1; fi",
    "  unset XIAO_ACTIVE_ENV",
    "}",
    "xiao() {",
    "  local _xiao_command='' _xiao_arg _xiao_override=0",
    "  for _xiao_arg in \"$@\"; do",
    "    case \"$_xiao_arg\" in",
    "      --help|-h|--version|-v) _xiao_override=1 ;;",
    "      --json|-debug|--color=*) ;;",
    "      *) if [[ -z \"$_xiao_command\" ]]; then _xiao_command=\"$_xiao_arg\"; fi ;;",
    "    esac",
    "  done",
    "  [[ $_xiao_override -eq 1 ]] && _xiao_command=''",
    "  local _xiao_dir _xiao_file _xiao_status _xiao_line _xiao_path _xiao_name _xiao_base",
    "  _xiao_base=${TMPDIR:-/tmp}",
    "  if command -v cygpath >/dev/null 2>&1 && [[ -n \"${TEMP:-}\" ]]; then _xiao_base=$(cygpath -u \"$TEMP\") || return 70; fi",
    "  _xiao_dir=$(mktemp -d \"$_xiao_base/xiao-activation.XXXXXXXX\") || return 70",
    "  _xiao_file=$(mktemp \"$_xiao_dir/activation.XXXXXXXX\") || { rmdir \"$_xiao_dir\"; return 70; }",
    "  if command -v cygpath >/dev/null 2>&1; then",
    "    export XIAO_ACTIVATION_FILE=$(cygpath -w \"$_xiao_file\")",
    "  else",
    "    export XIAO_ACTIVATION_FILE=\"$_xiao_file\"",
    "  fi",
    `  command ${commandName} \"$@\"; _xiao_status=$?`,
    "  unset XIAO_ACTIVATION_FILE",
    "  if [[ $_xiao_status -eq 0 && ( \"$_xiao_command\" == venv || \"$_xiao_command\" == sync ) ]]; then",
    "    local -a _xiao_lines=()",
    "    while IFS= read -r _xiao_line || [[ -n \"$_xiao_line\" ]]; do _xiao_lines+=(\"$_xiao_line\"); done < \"$_xiao_file\"",
    "    if [[ ${#_xiao_lines[@]} -eq 2 && \"${_xiao_lines[0]}\" == \"XIAO_ACTIVE_ENV='\"*\"'\" && \"${_xiao_lines[1]}\" == 'export XIAO_ACTIVE_ENV' ]]; then",
    "      _xiao_line=${_xiao_lines[0]}",
    "      _xiao_path=${_xiao_line#XIAO_ACTIVE_ENV=\\'}",
    "      _xiao_path=${_xiao_path%\\'}",
    "      if [[ \"$_xiao_path\" != *\"'\"* && ! \"$_xiao_path\" =~ [[:cntrl:]] && ( \"$_xiao_path\" == /* || ( \"${_xiao_path:0:1}\" =~ ^[A-Za-z]$ && \"${_xiao_path:1:1}\" == : && ( \"${_xiao_path:2:1}\" == / || \"${_xiao_path:2:1}\" == \"\\\\\" ) ) ) ]]; then",
    "        _xiao_name=${_xiao_path##*/}",
    "        _xiao_name=${_xiao_name##*\\\\}",
    "        [[ \"$_xiao_name\" == .venv ]] && _xiao_name=venv",
    "        local _xiao_color=0 _xiao_can_color=0",
    "        [[ -t 1 && -z \"${NO_COLOR+x}\" && \"${TERM:-}\" != dumb ]] && _xiao_can_color=1",
    "        [[ $_xiao_can_color -eq 1 ]] && _xiao_color=1",
    "        for _xiao_arg in \"$@\"; do",
    "          [[ \"$_xiao_arg\" == \"--color=never\" ]] && _xiao_color=0",
    "          [[ \"$_xiao_arg\" == \"--color=always\" && $_xiao_can_color -eq 1 ]] && _xiao_color=1",
    "        done",
    "        _xiao_activate_environment \"$_xiao_name\" \"$_xiao_color\" \"$_xiao_path\"",
    "      fi",
    "    fi",
    "  elif [[ $_xiao_status -eq 0 && \"$_xiao_command\" == deactivate ]]; then",
    "    _xiao_deactivate_environment",
    "  fi",
    "  rm -f -- \"$_xiao_file\"",
    "  rmdir -- \"$_xiao_dir\"",
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
    "if (-not (Get-Variable -Name XiaoOriginalPrompt -Scope Script -ErrorAction SilentlyContinue)) { $script:XiaoOriginalPrompt = $null }",
    "if (-not (Get-Variable -Name XiaoActiveEnvironment -Scope Script -ErrorAction SilentlyContinue)) { $script:XiaoActiveEnvironment = $null }",
    "if (-not (Get-Variable -Name XiaoColorEnvironment -Scope Script -ErrorAction SilentlyContinue)) { $script:XiaoColorEnvironment = $true }",
    "function global:__xiao_activate_environment {",
    "  param([string]$Name, [bool]$UseColor = $true, [string]$Path)",
    "  if ($null -eq $script:XiaoOriginalPrompt) { $script:XiaoOriginalPrompt = (Get-Command prompt -CommandType Function).ScriptBlock }",
    "  $script:XiaoActiveEnvironment = $Name",
    "  $env:XIAO_ACTIVE_ENV = $Path",
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
    "  Remove-Item Env:XIAO_ACTIVE_ENV -ErrorAction SilentlyContinue",
    "}",
    `function global:${commandName} {`,
    "  $first = ''",
    "  $hasOverride = $false",
    "  foreach ($argument in $args) {",
    "    if ($argument -ceq '--help' -or $argument -ceq '-h' -or $argument -ceq '--version' -or $argument -ceq '-v') { $hasOverride = $true }",
    "    if (-not $first -and $argument -cne '--json' -and $argument -cne '-debug' -and $argument -cnotlike '--color=*') { $first = [string]$argument }",
    "  }",
    "  if ($hasOverride) { $first = '' }",
    "  $temporary = Join-Path ([System.IO.Path]::GetTempPath()) ('xiao-activation.' + [guid]::NewGuid().ToString('N'))",
    "  New-Item -ItemType Directory -Path $temporary -ErrorAction Stop | Out-Null",
    "  if ([Environment]::OSVersion.Platform -ne 'Win32NT') { & chmod 700 $temporary }",
    "  $file = Join-Path $temporary ('activation.' + [guid]::NewGuid().ToString('N'))",
    "  New-Item -ItemType File -Path $file -ErrorAction Stop | Out-Null",
    "  if ([Environment]::OSVersion.Platform -ne 'Win32NT') { & chmod 600 $file }",
    "  $env:XIAO_ACTIVATION_FILE = $file",
    `  $executable = Get-Command ${commandName}.exe -CommandType Application -ErrorAction SilentlyContinue`,
    `  if ($null -eq $executable) { $executable = Get-Command ${commandName} -CommandType Application -ErrorAction SilentlyContinue }`,
    "  if ($null -eq $executable) { Remove-Item Env:XIAO_ACTIVATION_FILE; Remove-Item -LiteralPath $temporary -Recurse -Force; throw '找不到 xiao 可执行文件' }",
    "  try {",
    "  & $executable.Source @args",
    "  $status = $LASTEXITCODE",
    "  Remove-Item Env:XIAO_ACTIVATION_FILE -ErrorAction SilentlyContinue",
    "  if ($status -eq 0 -and ($first -eq 'venv' -or $first -eq 'sync')) {",
    "    $text = [System.IO.File]::ReadAllText($file)",
    "    $match = [regex]::Match($text, \"\\AXIAO_ACTIVE_ENV='([^'\\r\\n]+)'\\nexport XIAO_ACTIVE_ENV\\n?\\z\")",
    "    if ($match.Success -and $match.Groups[1].Value -notmatch \"['\\x00-\\x1f\\x7f]\" -and $match.Groups[1].Value -match '^(?:/|[A-Za-z]:[\\\\/])') {",
    "      $path = $match.Groups[1].Value",
    "      $name = [System.IO.Path]::GetFileName($path.TrimEnd([char[]]@('/', '\\')))",
    "      if ($name -eq '.venv') { $name = 'venv' }",
    "      $canColor = (-not [Console]::IsOutputRedirected -and $env:NO_COLOR -eq $null -and $env:TERM -ne 'dumb')",
    "      $useColor = ($canColor -and $args -notcontains '--color=never')",
    "      if ($canColor -and $args -contains '--color=always') { $useColor = $true }",
    "      __xiao_activate_environment $name $useColor $path",
    "    }",
    "  } elseif ($status -eq 0 -and $first -eq 'deactivate') {",
    "    __xiao_deactivate_environment",
    "  }",
    "  } finally {",
    "    Remove-Item Env:XIAO_ACTIVATION_FILE -ErrorAction SilentlyContinue",
    "    Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue",
    "  }",
    "  if ($status -ne 0) { throw ('xiao 退出码 ' + $status) }",
    "}",
    "",
  ].join("\n");
}
