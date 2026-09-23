/** `xiao` TypeScript CLI 入口。 */

import { parseArguments, CliArgumentError } from "./commands/parser.ts";
import { executeCommand } from "./commands/index.ts";
import { renderCliError } from "./diagnostics/render.ts";

/** CLI 入口依赖的可注入 IO。 */
export interface CliIo {
  /** 标准输出。 */
  stdout?: CliOutput;
  /** 标准错误。 */
  stderr?: CliOutput;
  /** 工作目录覆盖，供宿主和测试注入。 */
  cwd?: string;
  /** 环境变量覆盖，供宿主和测试注入。 */
  env?: NodeJS.ProcessEnv;
  /** 输出 TTY 能力覆盖。 */
  isTTY?: boolean;
  /** CLI 可执行文件路径覆盖，供相邻工具链发现和测试注入。 */
  executablePath?: string;
}

/** CLI 可写输出流的最小能力。 */
export interface CliOutput extends NodeJS.WritableStream {
  /** 是否连接终端；非 TTY 可省略。 */
  isTTY?: boolean;
}

/** 执行一次 CLI 调用并返回应交给宿主的进程码。 */
export async function runCli(argv: readonly string[] = process.argv.slice(2), io: CliIo = {}): Promise<number> {
  const stdout = io.stdout ?? process.stdout;
  const stderr = io.stderr ?? process.stderr;
  const env = io.env ?? process.env;
  const context = {
    cwd: io.cwd ?? process.cwd(),
    env,
    isTTY: io.isTTY ?? Boolean(stdout.isTTY),
    executablePath: io.executablePath ?? process.execPath,
  };
  try {
    const command = parseArguments(argv);
    const result = await executeCommand(command, context);
    await writeSafely(stdout, result.stdout);
    await writeSafely(stderr, result.stderr);
    return result.exitCode;
  } catch (error) {
    const result = renderCliError(error instanceof CliArgumentError ? error : error, {
      json: argv.includes("--json"),
      color: colorFromArguments(argv),
      isTTY: Boolean(stderr.isTTY),
      noColor: env.NO_COLOR !== undefined,
      colorTerm: env.COLORTERM,
      term: env.TERM,
    });
    await writeSafely(stdout, result.stdout);
    await writeSafely(stderr, result.stderr);
    return result.exitCode;
  }
}

/** 在管道提前关闭时吞掉 EPIPE，保持 CLI 可组合。 */
export async function writeSafely(stream: NodeJS.WritableStream, text: string): Promise<void> {
  if (text.length === 0) return;
  try {
    await new Promise<void>((resolve, reject) => {
      stream.write(text, (error?: Error | null) => error ? reject(error) : resolve());
    });
  } catch (error) {
    if (isEpipe(error)) return;
    throw error;
  }
}

/** 从原始参数提取颜色模式，供参数解析失败时仍能正确呈现。 */
function colorFromArguments(argv: readonly string[]): "auto" | "always" | "never" {
  const value = argv.find((argument) => argument.startsWith("--color="))?.slice("--color=".length);
  return value === "always" || value === "never" ? value : "auto";
}

/** 判断写端是否因管道接收端提前关闭而返回 EPIPE。 */
function isEpipe(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error && (error as { code?: string }).code === "EPIPE";
}

if (import.meta.main) {
  process.exitCode = await runCli();
}
