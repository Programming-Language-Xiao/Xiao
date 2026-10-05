/** `xiao` 命令行参数解析；不承载编译器或 Runtime 语义。 */

import { cliMessage } from "../i18n.ts";
import type { SupportedLocale } from "../config/locale.ts";
import { parseOptimizationLevel, type OptimizationLevel } from "../config/optimization.ts";

/** 全局 CLI 选项。 */
export interface GlobalCliOptions {
  /** 颜色模式。 */
  color: "auto" | "always" | "never";
  /** 输出机器 JSON。 */
  json: boolean;
  /** 强制运行时诊断窗口。 */
  debug: boolean;
  /** 不等待输入、不显示交互确认。 */
  nonInteractive: boolean;
  /** 输出详细过程日志到 stderr。 */
  verbose: boolean;
}

/** 11A-E4 支持的 Shell 名称。 */
export type ShellName = "bash" | "zsh" | "fish" | "powershell" | "cmd";

/** 解析成功的命令联合。 */
export type ParsedCommand =
  | { kind: "help"; options: GlobalCliOptions }
  | { kind: "version"; options: GlobalCliOptions }
  | { kind: "run"; file: string; optimizationLevel: OptimizationLevel; optimizationExplicit: boolean; options: GlobalCliOptions }
  | { kind: "xar"; file: string; optimizationLevel: OptimizationLevel; optimizationExplicit: boolean; options: GlobalCliOptions }
  | { kind: "config"; key: string; value: string; global: boolean; options: GlobalCliOptions }
  | { kind: "verify"; file: string; detail: boolean; options: GlobalCliOptions }
  | { kind: "cache"; action: "list" | "verify" | "rebuild" | "clean"; apply: boolean; options: GlobalCliOptions }
  | { kind: "association"; action: "install" | "check" | "uninstall"; platform?: "win32" | "linux" | "darwin"; executable?: string; options: GlobalCliOptions }
  | { kind: "test"; project?: string; timeoutMs?: number; optimizationLevel: OptimizationLevel; optimizationExplicit: boolean; options: GlobalCliOptions }
  | { kind: "build"; file: string; output: string; llvmIrOutput: string | null; xar: boolean; optimizationLevel: OptimizationLevel; optimizationExplicit: boolean; args: readonly string[]; options: GlobalCliOptions }
  | { kind: "venv"; name?: string; options: GlobalCliOptions }
  | { kind: "sync"; keepExtra: boolean; locked: boolean; frozen: boolean; options: GlobalCliOptions }
  | { kind: "install"; project?: string; options: GlobalCliOptions }
  | { kind: "lock"; options: GlobalCliOptions }
  | { kind: "update"; options: GlobalCliOptions }
  | { kind: "add"; packageName: string; path: string; version?: string; dev: boolean; options: GlobalCliOptions }
  | { kind: "remove"; packageName: string; dev: boolean; options: GlobalCliOptions }
  | { kind: "shell-init"; shell: ShellName; action: "print" | "install" | "uninstall"; profile?: string; options: GlobalCliOptions }
  | { kind: "deactivate"; options: GlobalCliOptions }
  | { kind: "repl"; multiline?: boolean; file?: string; optimizationLevel: OptimizationLevel; optimizationExplicit: boolean; options: GlobalCliOptions };

/** 参数解析异常。 */
export class CliArgumentError extends Error {
  /** 稳定诊断编号。 */
  readonly code: string;
  /** 帮助文本应显示的原因。 */
  readonly usage = true;

  /** 创建参数错误。 */
  constructor(message: string, code = "X11-CLI-ARG-001") {
    super(`${code}: ${message}`);
    this.name = "CliArgumentError";
    this.code = code;
  }
}

/** 解析命令行参数；支持全局选项出现在子命令前后。 */
export function parseArguments(argv: readonly string[]): ParsedCommand {
  const { options, positional } = parseGlobalOptions(argv);
  if (argv.some((argument) => argument === "--help" || argument === "-h")) return { kind: "help", options };
  if (argv.some((argument, index) => (argument === "-v" || argument === "--version") &&
    ((positional[0] !== "add" && positional[0] !== "remove") || index < argv.indexOf(positional[0])))) return { kind: "version", options };
  if (positional.length === 0) return { kind: "repl", optimizationLevel: 0, optimizationExplicit: false, options };
  if (positional.every((argument) => argument.startsWith("-O"))) {
    let optimizationLevel: OptimizationLevel = 0;
    for (const argument of positional) optimizationLevel = parseOptimizationLevel(argument);
    return { kind: "repl", optimizationLevel, optimizationExplicit: true, options };
  }
  const [command, ...rest] = positional;
  if (command === "--help" || command === "-h") {
    if (rest.length > 0) throw new CliArgumentError("--help 不接受额外参数");
    return { kind: "help", options };
  }
  if (command === "--version" || command === "-v") {
    if (rest.length > 0) throw new CliArgumentError("--version 不接受额外参数");
    return { kind: "version", options };
  }
  if (positional.includes("-xar") && command !== "build") return parseArchive(positional, options);
  if (command === "--inLF") {
    if (rest.length > 1) throw new CliArgumentError("--inLF 最多接受一个 .xiao 文件路径");
    if (rest.length === 1 && !rest[0].endsWith(".xiao")) {
      throw new CliArgumentError("--inLF 当前只支持 .xiao 文件；其他扩展名留待后续扩展", "X11-CLI-SAVE-001");
    }
    return rest.length === 0 ? { kind: "repl", multiline: true, optimizationLevel: 0, optimizationExplicit: false, options }
      : { kind: "repl", multiline: true, file: rest[0], optimizationLevel: 0, optimizationExplicit: false, options };
  }
  if (command === "run") {
    if (rest.includes("-xar")) return parseArchive(rest, options);
    return parseRun(rest, options);
  }
  if (command === "-xar") return parseArchive(rest, options);
  if (command === "config") return parseConfig(rest, options);
  if (command === "verify") return parseVerify(rest, options);
  if (command === "cache") return parseCache(rest, options);
  if (command === "association" || command === "file-association") return parseAssociation(rest, options);
  if (command === "test") {
    return parseTest(rest, options);
  }
  if (command === "build") return parseBuild(rest, options);
  if (command === "venv") return parseVenv(rest, options);
  if (command === "sync") return parseSync(rest, options);
  if (command === "lock" || command === "update") {
    if (rest.length !== 0) throw new CliArgumentError(`${command} 不接受额外参数`);
    return { kind: command, options };
  }
  if (command === "add") return parseAdd(rest, options);
  if (command === "remove") return parseRemove(rest, options);
  if (command === "install" || command === "i") {
    if (rest.length > 1 || rest[0] === "" || rest[0]?.startsWith("-")) {
      throw new CliArgumentError("install/i 最多接受一个项目目录或 config.xiao 路径");
    }
    return rest.length === 0 ? { kind: "install", options } : { kind: "install", project: rest[0], options };
  }
  if (command === "shell-init") return parseShellInit(rest, options);
  if (command === "deactivate") {
    if (rest.length > 0) throw new CliArgumentError("deactivate 不接受额外参数");
    return { kind: "deactivate", options };
  }
  if (command.endsWith(".xiao")) {
    let optimizationLevel: OptimizationLevel = 0;
    let optimizationExplicit = false;
    let archive = false;
    for (const argument of rest) {
      if (argument === "-xar") {
        if (archive) throw new CliArgumentError("-xar 不可重复");
        archive = true;
      } else if (argument.startsWith("-O")) {
        optimizationLevel = parseOptimizationLevel(argument);
        optimizationExplicit = true;
      } else {
        throw new CliArgumentError("源码快捷运行只接受一个 .xiao 文件和可选优化级别");
      }
    }
    return archive
      ? { kind: "xar", file: command, optimizationLevel, optimizationExplicit, options }
      : { kind: "run", file: command, optimizationLevel, optimizationExplicit, options };
  }
  if (command.startsWith("-")) throw new CliArgumentError(`未知选项：${command}`);
  throw new CliArgumentError(`未知命令：${command}`);
}

/** 解析两种等价的归档运行形式，`-xar` 与路径先后均可。 */
function parseArchive(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  let optimizationLevel: OptimizationLevel = 0;
  let optimizationExplicit = false;
  const values = args.filter((argument) => {
    if (argument === "-xar" || argument === "run") return false;
    if (argument.startsWith("-O")) {
      optimizationLevel = parseOptimizationLevel(argument);
      optimizationExplicit = true;
      return false;
    }
    return true;
  });
  if (args.filter((argument) => argument === "-xar").length > 1 || values.length !== 1) {
    throw new CliArgumentError("-xar 需要且只需要一个归档路径");
  }
  if (!values[0].toLowerCase().endsWith(".xar")) {
    throw new CliArgumentError("-xar 的输入必须是 .xar 归档");
  }
  return { kind: "xar", file: values[0], optimizationLevel, optimizationExplicit, options };
}

/** 返回稳定帮助文本。 */
export function helpText(locale: SupportedLocale = "zh-CN"): string {
  return cliMessage("xiao.cli.help", locale);
}

/** 解析环境创建命令；名称为空时使用冻结的默认环境。 */
function parseVenv(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  if (args.length > 1) throw new CliArgumentError("venv 最多接受一个环境名称");
  const name = args[0];
  if (name !== undefined && name.startsWith("-")) throw new CliArgumentError("venv 环境名称不能以选项开头");
  return name === undefined ? { kind: "venv", options } : { kind: "venv", name, options };
}

/** 同步开关只决定 Rust 请求模式，不在 CLI 实现锁文件规则。 */
function parseSync(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  if (args.some((argument) => !["--keep-extra", "--locked", "--frozen"].includes(argument))) {
    throw new CliArgumentError("sync 仅支持 --keep-extra、--locked、--frozen");
  }
  if (new Set(args).size !== args.length) throw new CliArgumentError("sync 选项不可重复");
  const locked = args.includes("--locked");
  const frozen = args.includes("--frozen");
  if (locked && frozen) throw new CliArgumentError("--locked 与 --frozen 不可同时使用");
  return { kind: "sync", keepExtra: args.includes("--keep-extra"), locked, frozen, options };
}

/** 依赖写回只接受明确的本地路径；源依赖在正文物化闭环完成后开放。 */
function parseAdd(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  const [packageName, ...rest] = args;
  if (!packageName || packageName.startsWith("-")) throw new CliArgumentError("add 需要一个包名");
  let path: string | undefined;
  let version: string | undefined;
  let dev = false;
  for (let index = 0; index < rest.length; index += 1) {
    const option = rest[index];
    if (option === "--dev" && !dev) { dev = true; continue; }
    const separator = option.indexOf("=");
    const flag = separator < 0 ? option : option.slice(0, separator);
    const inline = separator < 0 ? undefined : option.slice(separator + 1);
    if (flag !== "--path" && flag !== "--version") throw new CliArgumentError(`add 未知或重复选项：${option}`);
    const value = inline ?? rest[++index];
    if (!value || value.startsWith("--")) throw new CliArgumentError(`${flag} 需要一个值`);
    if (flag === "--path") {
      if (path !== undefined) throw new CliArgumentError("--path 不可重复");
      path = value;
    } else {
      if (version !== undefined) throw new CliArgumentError("--version 不可重复");
      version = value;
    }
  }
  if (path === undefined) throw new CliArgumentError("add 需要 --path <相对路径>");
  return { kind: "add", packageName, path, ...(version === undefined ? {} : { version }), dev, options };
}

/** 删除指定表中的已有声明，不隐式推断开发依赖。 */
function parseRemove(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  const [packageName, ...rest] = args;
  if (!packageName || packageName.startsWith("-") || rest.length > 1 || (rest.length === 1 && rest[0] !== "--dev")) {
    throw new CliArgumentError("remove 需要一个包名，可附加 --dev");
  }
  return { kind: "remove", packageName, dev: rest.length === 1, options };
}

/** 解析 Shell 钩子输出、显式安装和移除命令。 */
function parseShellInit(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  if (args.length === 0) throw new CliArgumentError("shell-init 需要一个 Shell 名称");
  const shell = args[0].toLowerCase();
  let canonical: ShellName;
  if (shell === "bash" || shell === "zsh" || shell === "fish") canonical = shell;
  else if (shell === "powershell" || shell === "pwsh") canonical = "powershell";
  else if (shell === "cmd" || shell === "cmd.exe") canonical = "cmd";
  else throw new CliArgumentError("shell-init 仅支持 bash、zsh、fish、powershell/pwsh 或 cmd/cmd.exe", "X11-CLI-SHELL-001");
  let action: "print" | "install" | "uninstall" = "print";
  let profile: string | undefined;
  for (let index = 1; index < args.length; index += 1) {
    const argument = args[index];
    if (argument === "--install" || argument === "--uninstall") {
      if (action !== "print") throw new CliArgumentError("shell-init 的安装与移除选项不可重复或并用");
      action = argument === "--install" ? "install" : "uninstall";
    } else if (argument === "--profile") {
      if (profile !== undefined || !args[index + 1] || args[index + 1].startsWith("--")) throw new CliArgumentError("--profile 需要一个绝对路径且不可重复");
      profile = args[++index];
    } else throw new CliArgumentError(`shell-init 不支持选项：${argument}`);
  }
  if (profile !== undefined && action === "print") throw new CliArgumentError("--profile 只能与 --install 或 --uninstall 同用");
  if (canonical === "cmd" && action !== "print") throw new CliArgumentError("cmd 只提供手工激活说明，不支持安装钩子", "X11-CLI-SHELL-002");
  return { kind: "shell-init", shell: canonical, action, ...(profile === undefined ? {} : { profile }), options };
}

/** 解析项目测试命令；选项必须留在命令分支内，不能静默成为项目路径。 */
function parseTest(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  let project: string | undefined;
  let timeoutMs: number | undefined;
  let optimizationLevel: OptimizationLevel = 0;
  let optimizationExplicit = false;
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (argument.startsWith("-O")) {
      optimizationLevel = parseOptimizationLevel(argument);
      optimizationExplicit = true;
      continue;
    }
    if (argument === "--timeout") {
      const value = args[++index];
      if (value === undefined || !/^\d+$/u.test(value)) throw new CliArgumentError("test --timeout 需要非负整数毫秒");
      timeoutMs = Number(value);
      if (!Number.isSafeInteger(timeoutMs)) throw new CliArgumentError("test --timeout 超出安全整数范围");
      continue;
    }
    if (argument.startsWith("-")) throw new CliArgumentError(`test 不支持选项：${argument}`);
    if (project !== undefined) throw new CliArgumentError("test 最多接受一个项目路径");
    project = argument;
  }
  return { kind: "test", project, timeoutMs, optimizationLevel, optimizationExplicit, options };
}

/** 校验 `run` 的单一源码参数。 */
function parseRun(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  let optimizationLevel: OptimizationLevel = 0;
  let optimizationExplicit = false;
  const positional = args.filter((argument) => {
    if (argument.startsWith("-O")) {
      optimizationLevel = parseOptimizationLevel(argument);
      optimizationExplicit = true;
      return false;
    }
    return true;
  });
  if (positional.length !== 1) throw new CliArgumentError("run 需要且只需要一个 .xiao 或 .xiaoc 文件");
  const lower = positional[0].toLowerCase();
  if (!lower.endsWith(".xiao") && !lower.endsWith(".xiaoc")) {
    throw new CliArgumentError("run 的输入必须是 .xiao 或 .xiaoc 文件");
  }
  return { kind: "run", file: positional[0], optimizationLevel, optimizationExplicit, options };
}

/** 解析已经冻结的原生构建参数。 */
function parseBuild(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  let output: string | undefined;
  let llvmIrOutput: string | null = null;
  let xar = false;
  let optimizationLevel: OptimizationLevel = 0;
  let optimizationExplicit = false;
  const positional: string[] = [];
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (argument === "-xar") {
      xar = true;
      continue;
    }
    if (argument === "-o" || argument === "--output") {
      const value = args[index + 1];
      if (!value || value.startsWith("-")) throw new CliArgumentError(`${argument} 需要一个输出路径`);
      output = value;
      index += 1;
      continue;
    }
    if (argument.startsWith("-o=") || argument.startsWith("--output=")) {
      const value = argument.slice(argument.indexOf("=") + 1);
      if (!value) throw new CliArgumentError(`${argument.slice(0, argument.indexOf("="))} 需要一个输出路径`);
      output = value;
      continue;
    }
    if (argument === "--emit-llvm") {
      const value = args[index + 1];
      if (!value || value.startsWith("-")) throw new CliArgumentError("--emit-llvm 需要一个输出路径");
      llvmIrOutput = value;
      index += 1;
      continue;
    }
    if (argument.startsWith("--emit-llvm=")) {
      const value = argument.slice("--emit-llvm=".length);
      if (!value) throw new CliArgumentError("--emit-llvm 需要一个输出路径");
      llvmIrOutput = value;
      continue;
    }
    if (argument.startsWith("-O")) {
      optimizationLevel = parseOptimizationLevel(argument);
      optimizationExplicit = true;
      continue;
    }
    if (argument.startsWith("-")) throw new CliArgumentError(`build 不支持选项：${argument}`);
    positional.push(argument);
  }
  if (positional.length !== 1) throw new CliArgumentError("build 需要且只需要一个 .xiao 文件");
  const file = positional[0];
  if (!file.toLowerCase().endsWith(".xiao")) throw new CliArgumentError("build 的输入必须是 .xiao 文件");
  const stem = file.replaceAll("\\", "/").slice(file.replaceAll("\\", "/").lastIndexOf("/") + 1).replace(/\.xiao$/iu, "") || "main";
  const defaultName = process.platform === "win32" ? `${stem}.exe` : stem;
  return {
    kind: "build",
    file,
    output: output ?? `build/${defaultName}`,
    llvmIrOutput,
    xar,
    optimizationLevel,
    optimizationExplicit,
    args,
    options,
  };
}

/** 解析配置范围、点分路径和值。 */
function parseConfig(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  let isGlobal = false;
  const values: string[] = [];
  for (const argument of args) {
    if (argument === "--global") {
      if (isGlobal) throw new CliArgumentError("--global 只能出现一次");
      isGlobal = true;
    } else values.push(argument);
  }
  if (values.length !== 2) throw new CliArgumentError("config 需要 <key.path> 和 <value>");
  return { kind: "config", key: values[0], value: values[1], global: isGlobal, options };
}

/** 解析只验证产物的命令。 */
function parseVerify(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  let detail = false;
  const values = args.filter((argument) => {
    if (argument === "--detail") {
      if (detail) throw new CliArgumentError("verify --detail 不可重复");
      detail = true;
      return false;
    }
    return true;
  });
  if (values.length !== 1) throw new CliArgumentError("verify 需要且只需要一个 .xiaoc 或 .xar 路径");
  const lower = values[0].toLocaleLowerCase("en-US");
  if (!lower.endsWith(".xiaoc") && !lower.endsWith(".xar")) throw new CliArgumentError("verify 只接受 .xiaoc 或 .xar");
  return { kind: "verify", file: values[0], detail, options };
}

/** 解析 16B 两阶段缓存命令。 */
function parseCache(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  const values = args.filter((argument) => argument !== "--apply");
  const apply = args.includes("--apply");
  if (values.length !== 1 || !["list", "verify", "rebuild", "clean"].includes(values[0])) {
    throw new CliArgumentError("cache 需要 list、verify、rebuild 或 clean");
  }
  if (apply && values[0] !== "clean") throw new CliArgumentError("--apply 只能用于 cache clean");
  return { kind: "cache", action: values[0] as "list" | "verify" | "rebuild" | "clean", apply, options };
}

/** 解析 `.xar` 文件关联的安装、检查和移除动作。 */
function parseAssociation(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  let action: "install" | "check" | "uninstall" | undefined;
  let platform: "win32" | "linux" | "darwin" | undefined;
  let executable: string | undefined;
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (argument === "install" || argument === "check" || argument === "uninstall") {
      if (action !== undefined) throw new CliArgumentError("association 动作不可重复");
      action = argument;
      continue;
    }
    if (argument === "--platform" || argument === "--xiao") {
      const value = args[++index];
      if (!value || value.startsWith("-")) throw new CliArgumentError(`${argument} 需要一个值`);
      if (argument === "--platform") {
        if (value !== "win32" && value !== "linux" && value !== "darwin") throw new CliArgumentError("association 平台必须是 win32、linux 或 darwin");
        platform = value;
      } else executable = value;
      continue;
    }
    if (argument.startsWith("--platform=") || argument.startsWith("--xiao=")) {
      const [key, value] = argument.split("=", 2);
      if (!value) throw new CliArgumentError(`${key} 需要一个值`);
      if (key === "--platform") {
        if (value !== "win32" && value !== "linux" && value !== "darwin") throw new CliArgumentError("association 平台必须是 win32、linux 或 darwin");
        platform = value;
      } else executable = value;
      continue;
    }
    throw new CliArgumentError(`association 不支持选项：${argument}`);
  }
  if (action === undefined) throw new CliArgumentError("association 需要 install、check 或 uninstall");
  return { kind: "association", action, ...(platform === undefined ? {} : { platform }), ...(executable === undefined ? {} : { executable }), options };
}

/** 提取颜色、JSON 等全局选项并保留其余位置参数。 */
function parseGlobalOptions(argv: readonly string[]): { options: GlobalCliOptions; positional: string[] } {
  let color: GlobalCliOptions["color"] = "auto";
  let json = false;
  let debug = false;
  let nonInteractive = false;
  let verbose = false;
  const positional: string[] = [];
  for (const argument of argv) {
    if (argument === "--json") { json = true; continue; }
    if (argument === "-debug") { debug = true; continue; }
    if (argument === "--non-interactive" || argument === "--noninteractive") { nonInteractive = true; continue; }
    if (argument === "--verbose") { verbose = true; continue; }
    if (argument === "--color") throw new CliArgumentError("--color 必须写成 --color=auto|always|never");
    if (argument.startsWith("--color=")) {
      const candidate = argument.slice("--color=".length);
      if (candidate !== "auto" && candidate !== "always" && candidate !== "never") throw new CliArgumentError(`不支持的颜色模式：${candidate}`);
      color = candidate;
      continue;
    }
    positional.push(argument);
  }
  return { options: { color, json, debug, nonInteractive, verbose }, positional };
}
