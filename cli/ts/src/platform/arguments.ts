/** CLI 启动参数的编码与预算接纳。 */

import type { RunArguments } from "../protocol/messages.ts";

const INVALID_ARGUMENT = "XIAO_LAUNCHER_INVALID_ARGUMENT";
const LAUNCHER_PATH = "XIAO_LAUNCHER_PATH";

/** 按运行入口的稳定诊断拒绝，避免把参数文本写到错误中。 */
export class RunArgumentsError extends Error {
  /** 稳定诊断不回显原始参数。 */
  readonly details = {};
  /** 驱动器在运行前拒绝请求的退出码。 */
  readonly exitCode = 2;

  /** 接纳失败的驱动器编号和说明。 */
  constructor(readonly code: "X09-DRIVER-004" | "X09-DRIVER-005", message: string) {
    super(message);
  }
}

/** 还原分发启动器记录的无效位置和文件关联应使用的入口路径。 */
export function launcherInvocation(): { argv: readonly string[]; executablePath: string | undefined } {
  const argv = process.argv.slice(2);
  const executablePath = process.env[LAUNCHER_PATH];
  const invalid = executablePath === undefined ? undefined : process.env[INVALID_ARGUMENT];
  delete process.env[INVALID_ARGUMENT];
  delete process.env[LAUNCHER_PATH];
  if (invalid !== undefined && /^(0|[1-9]\d*)$/u.test(invalid)) {
    const index = Number(invalid);
    if (index < argv.length) argv[index] = "\ud800";
  }
  return { argv, executablePath };
}

/** 对 UTF-16 代理项使用 Unicode 模式，匹配未配对的单元及 NUL。 */
export function requireArgumentEncoding(value: string): void {
  if (/[\u0000\ud800-\udfff]/u.test(value)) {
    throw new RunArgumentsError("X09-DRIVER-004", "运行参数含无效编码或 NUL");
  }
}

/** 保留调用拼写，按条目、编码及 UTF-8 字节上限接纳整份快照。 */
export function snapshotForRun(programName: string, argv: readonly string[]): RunArguments {
  if (argv.length > 1024) throw new RunArgumentsError("X09-DRIVER-005", "运行参数超过接纳上限");
  let total = 0;
  for (const value of [programName, ...argv]) {
    requireArgumentEncoding(value);
    const size = Buffer.byteLength(value, "utf8");
    total += size;
    if (size > 4096 || total > 65536) {
      throw new RunArgumentsError("X09-DRIVER-005", "运行参数超过接纳上限");
    }
  }
  return { program_name: programName, argv };
}
