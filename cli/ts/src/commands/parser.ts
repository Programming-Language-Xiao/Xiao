/** `xiao` 命令行参数解析；不承载编译器或 Runtime 语义。 */

/** 全局 CLI 选项。 */
export interface GlobalCliOptions {
  /** 颜色模式。 */
  color: "auto" | "always" | "never";
  /** 输出机器 JSON。 */
  json: boolean;
  /** 强制运行时诊断窗口。 */
  debug: boolean;
}

/** E0 支持的 Shell 名称。 */
export type ShellName = "bash" | "powershell" | "cmd";

/** 解析成功的命令联合。 */
export type ParsedCommand =
  | { kind: "help"; options: GlobalCliOptions }
  | { kind: "version"; options: GlobalCliOptions }
  | { kind: "run"; file: string; options: GlobalCliOptions }
  | { kind: "config"; key: string; value: string; global: boolean; options: GlobalCliOptions }
  | { kind: "test"; project?: string; timeoutMs?: number; options: GlobalCliOptions }
  | { kind: "build"; file: string; output: string; llvmIrOutput: string | null; optimizationLevel: 0; args: readonly string[]; options: GlobalCliOptions }
  | { kind: "venv"; name?: string; options: GlobalCliOptions }
  | { kind: "sync"; keepExtra: boolean; locked: boolean; frozen: boolean; options: GlobalCliOptions }
  | { kind: "install"; options: GlobalCliOptions }
  | { kind: "shell-init"; shell: ShellName; options: GlobalCliOptions }
  | { kind: "deactivate"; options: GlobalCliOptions }
  | { kind: "repl"; options: GlobalCliOptions };

/** 参数解析异常。 */
export class CliArgumentError extends Error {
  /** 稳定诊断编号。 */
  readonly code = "X11-CLI-ARG-001";
  /** 帮助文本应显示的原因。 */
  readonly usage = true;

  /** 创建参数错误。 */
  constructor(message: string) {
    super(`${"X11-CLI-ARG-001"}: ${message}`);
    this.name = "CliArgumentError";
  }
}

/** 解析命令行参数；支持全局选项出现在子命令前后。 */
export function parseArguments(argv: readonly string[]): ParsedCommand {
  const { options, positional } = parseGlobalOptions(argv);
  if (argv.some((argument) => argument === "--help" || argument === "-h")) return { kind: "help", options };
  if (argv.some((argument) => argument === "--version" || argument === "-v")) return { kind: "version", options };
  if (positional.length === 0) return { kind: "repl", options };
  const [command, ...rest] = positional;
  if (command === "--help" || command === "-h") {
    if (rest.length > 0) throw new CliArgumentError("--help 不接受额外参数");
    return { kind: "help", options };
  }
  if (command === "--version" || command === "-v") {
    if (rest.length > 0) throw new CliArgumentError("--version 不接受额外参数");
    return { kind: "version", options };
  }
  if (command === "--inLF" || command === "-debug") {
    return { kind: "repl", options };
  }
  if (command === "run") return parseRun(rest, options);
  if (command === "config") return parseConfig(rest, options);
  if (command === "test") {
    return parseTest(rest, options);
  }
  if (command === "build") return parseBuild(rest, options);
  if (command === "venv") return parseVenv(rest, options);
  if (command === "sync") return parseSync(rest, options);
  if (command === "install" || command === "i") {
    if (rest.length !== 0) throw new CliArgumentError("install/i 不接受选项或位置参数");
    return { kind: "install", options };
  }
  if (command === "shell-init") return parseShellInit(rest, options);
  if (command === "deactivate") {
    if (rest.length > 0) throw new CliArgumentError("deactivate 不接受额外参数");
    return { kind: "deactivate", options };
  }
  if (command.endsWith(".xiao")) {
    if (rest.length > 0) throw new CliArgumentError("源码快捷运行只接受一个 .xiao 文件");
    return { kind: "run", file: command, options };
  }
  if (command.startsWith("-")) throw new CliArgumentError(`未知选项：${command}`);
  throw new CliArgumentError(`未知命令：${command}`);
}

/** 返回稳定帮助文本。 */
export function helpText(): string {
  return [
    "xiao 0.1.0",
    "用法：",
    "  xiao run <file.xiao> [-debug] [--json] [--color=auto|always|never]",
    "  xiao <file.xiao> [-debug]                运行源码快捷方式",
    "  xiao config [--global] <key.path> <value>",
    "  xiao test [project] [--timeout <ms>]     运行项目 tests/**/*.xiao",
    "  xiao build -o <output> <file.xiao> [-debug] [--emit-llvm <path>] [--json]",
    "  xiao venv [name]                         创建项目环境并输出激活提示",
    "  xiao sync [--keep-extra] [--locked|--frozen] 同步依赖并激活环境",
    "  xiao install | xiao i                     安装已有锁文件到激活或全局环境",
    "  xiao shell-init <bash|powershell|cmd>    输出一次性 Shell 钩子",
    "  xiao deactivate                          取消当前 Shell 环境激活",
    "  xiao --help | --version",
    "",
    "当前阶段不启动 REPL；无参数或 --inLF 会给出稳定的未实现诊断。",
  ].join("\n") + "\n";
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

/** 解析一次性 Shell 钩子输出命令。 */
function parseShellInit(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  if (args.length !== 1) throw new CliArgumentError("shell-init 需要且只需要一个 Shell 名称");
  const shell = args[0].toLowerCase();
  if (shell === "bash") return { kind: "shell-init", shell: "bash", options };
  if (shell === "powershell" || shell === "pwsh") return { kind: "shell-init", shell: "powershell", options };
  if (shell === "cmd" || shell === "cmd.exe") return { kind: "shell-init", shell: "cmd", options };
  throw new CliArgumentError("shell-init 仅支持 bash、powershell/pwsh 或 cmd/cmd.exe");
}

/** 解析项目测试命令；选项必须留在命令分支内，不能静默成为项目路径。 */
function parseTest(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  let project: string | undefined;
  let timeoutMs: number | undefined;
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
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
  return { kind: "test", project, timeoutMs, options };
}

/** 校验 `run` 的单一源码参数。 */
function parseRun(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  if (args.length !== 1) throw new CliArgumentError("run 需要且只需要一个 .xiao 文件");
  if (!args[0].toLowerCase().endsWith(".xiao")) throw new CliArgumentError("run 的输入必须是 .xiao 文件");
  return { kind: "run", file: args[0], options };
}

/** 解析已经冻结的原生构建参数。 */
function parseBuild(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  let output: string | undefined;
  let llvmIrOutput: string | null = null;
  let optimizationLevel: 0 = 0;
  const positional: string[] = [];
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
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
    if (argument === "-O0") {
      optimizationLevel = 0;
      continue;
    }
    if (/^-O\d+$/u.test(argument) || argument.startsWith("-O")) {
      throw new CliArgumentError(`当前只支持 -O0，不支持优化级别：${argument}`);
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
    optimizationLevel,
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

/** 提取颜色、JSON 等全局选项并保留其余位置参数。 */
function parseGlobalOptions(argv: readonly string[]): { options: GlobalCliOptions; positional: string[] } {
  let color: GlobalCliOptions["color"] = "auto";
  let json = false;
  let debug = false;
  const positional: string[] = [];
  for (const argument of argv) {
    if (argument === "--json") { json = true; continue; }
    if (argument === "-debug") { debug = true; continue; }
    if (argument === "--color") throw new CliArgumentError("--color 必须写成 --color=auto|always|never");
    if (argument.startsWith("--color=")) {
      const candidate = argument.slice("--color=".length);
      if (candidate !== "auto" && candidate !== "always" && candidate !== "never") throw new CliArgumentError(`不支持的颜色模式：${candidate}`);
      color = candidate;
      continue;
    }
    positional.push(argument);
  }
  return { options: { color, json, debug }, positional };
}
