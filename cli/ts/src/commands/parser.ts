/** `xiao` 命令行参数解析；不承载编译器或 Runtime 语义。 */

/** 全局 CLI 选项。 */
export interface GlobalCliOptions {
  /** 颜色模式。 */
  color: "auto" | "always" | "never";
  /** 输出机器 JSON。 */
  json: boolean;
}

/** 解析成功的命令联合。 */
export type ParsedCommand =
  | { kind: "help"; options: GlobalCliOptions }
  | { kind: "version"; options: GlobalCliOptions }
  | { kind: "run"; file: string; options: GlobalCliOptions }
  | { kind: "config"; key: string; value: string; global: boolean; options: GlobalCliOptions }
  | { kind: "test"; project?: string; options: GlobalCliOptions }
  | { kind: "build"; args: readonly string[]; options: GlobalCliOptions }
  | { kind: "repl"; options: GlobalCliOptions }
  | { kind: "unsupported-debug"; options: GlobalCliOptions };

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
  if (argv.some((argument) => argument === "-debug")) return { kind: "unsupported-debug", options };
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
    return command === "-debug" ? { kind: "unsupported-debug", options } : { kind: "repl", options };
  }
  if (command === "run") return parseRun(rest, options);
  if (command === "config") return parseConfig(rest, options);
  if (command === "test") {
    if (rest.length > 1) throw new CliArgumentError("test 最多接受一个项目路径");
    return { kind: "test", project: rest[0], options };
  }
  if (command === "build") return { kind: "build", args: rest, options };
  if (command.endsWith(".xiao")) {
    if (rest.length > 0) throw new CliArgumentError("源码快捷运行只接受一个 .xiao 文件");
    if (command === "-debug") return { kind: "unsupported-debug", options };
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
    "  xiao run <file.xiao> [--json] [--color=auto|always|never]",
    "  xiao <file.xiao>                         运行源码快捷方式",
    "  xiao config [--global] <key.path> <value>",
    "  xiao test                                已登记，测试框架待后续批次",
    "  xiao build ...                           X0-E 尚未实现",
    "  xiao --help | --version",
    "",
    "当前阶段不启动 REPL；无参数或 --inLF 会给出稳定的未实现诊断。",
  ].join("\n") + "\n";
}

/** 校验 `run` 的单一源码参数。 */
function parseRun(args: readonly string[], options: GlobalCliOptions): ParsedCommand {
  if (args.length !== 1) throw new CliArgumentError("run 需要且只需要一个 .xiao 文件");
  if (!args[0].toLowerCase().endsWith(".xiao")) throw new CliArgumentError("run 的输入必须是 .xiao 文件");
  return { kind: "run", file: args[0], options };
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
  const positional: string[] = [];
  for (const argument of argv) {
    if (argument === "--json") { json = true; continue; }
    if (argument === "--color") throw new CliArgumentError("--color 必须写成 --color=auto|always|never");
    if (argument.startsWith("--color=")) {
      const candidate = argument.slice("--color=".length);
      if (candidate !== "auto" && candidate !== "always" && candidate !== "never") throw new CliArgumentError(`不支持的颜色模式：${candidate}`);
      color = candidate;
      continue;
    }
    positional.push(argument);
  }
  return { options: { color, json }, positional };
}
