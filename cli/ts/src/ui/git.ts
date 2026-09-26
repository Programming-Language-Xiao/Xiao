/** 提示符专用 Git 状态探测；失败只影响摘要，不影响输入循环。 */

import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

/** 当前分支的上游差异；没有上游时计数均为 null。 */
export interface GitSummary {
  branch: string;
  ahead: number | null;
  behind: number | null;
}

/** 可查询的探测失败原因，不包含可能泄露路径的 Git 错误原文。 */
export interface GitProbeDiagnostic {
  code: "X11-REPL-GIT-001";
  reason: "unavailable" | "not-repository" | "timeout" | "failed" | "no-branch";
}

/** 有摘要或明确降级原因的探测结果。 */
export interface GitProbeResult {
  summary: GitSummary | null;
  diagnostic: GitProbeDiagnostic | null;
}

/** 可注入的 Git 命令边界，用于跨平台无 Git 与超时测试。 */
export interface GitProbeOptions {
  env?: NodeJS.ProcessEnv;
  timeoutMs?: number;
  runStatus?: (cwd: string, env: NodeJS.ProcessEnv, signal: AbortSignal) => Promise<string>;
}

/** 每次重绘前探测一次；超时即放弃本轮，不等待子进程退出。 */
export async function probeGitSummary(cwd: string, options: GitProbeOptions = {}): Promise<GitProbeResult> {
  const controller = new AbortController();
  const env = { ...(options.env ?? process.env), LC_ALL: "C" };
  const deadline = options.timeoutMs ?? 250;
  let timedOut = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const timeout = new Promise<never>((_resolve, reject) => {
      timer = setTimeout(() => {
        timedOut = true;
        controller.abort();
        reject(new Error("Git 探测超时"));
      }, deadline);
    });
    const output = await Promise.race([(options.runStatus ?? readGitStatus)(cwd, env, controller.signal), timeout]);
    const summary = parseGitStatus(output);
    return summary === null ? failure("no-branch") : { summary, diagnostic: null };
  } catch (error) {
    if (timedOut) return failure("timeout");
    if (isNodeError(error, "ENOENT")) return failure("unavailable");
    if (typeof error === "object" && error !== null && "stderr" in error
      && typeof error.stderr === "string" && error.stderr.includes("not a git repository")) {
      return failure("not-repository");
    }
    return failure("failed");
  } finally {
    if (timer !== undefined) clearTimeout(timer);
  }
}

/** 只解析 porcelain v2 的头部；没有 branch.ab 意味着没有跟踪上游。 */
export function parseGitStatus(output: string): GitSummary | null {
  const head = /^# branch\.head (.+)$/mu.exec(output)?.[1];
  if (head === undefined || head === "(detached)" || /[\x00-\x1f\x7f]/u.test(head)) return null;
  const counts = /^# branch\.ab \+(\d+) -(\d+)$/mu.exec(output);
  return { branch: head, ahead: counts === null ? null : Number(counts[1]), behind: counts === null ? null : Number(counts[2]) };
}

/** 通过受限缓冲区读取 Git 状态，不启动 Shell，也不要求包管理器安装 Git。 */
async function readGitStatus(cwd: string, env: NodeJS.ProcessEnv, signal: AbortSignal): Promise<string> {
  const { stdout } = await execFileAsync("git", ["status", "--porcelain=v2", "--branch"], {
    cwd, env, signal, windowsHide: true, maxBuffer: 64 * 1024,
  });
  return stdout;
}

/** 合并稳定编号与降级原因。 */
function failure(reason: GitProbeDiagnostic["reason"]): GitProbeResult {
  return { summary: null, diagnostic: { code: "X11-REPL-GIT-001", reason } };
}

/** 识别进程创建失败。 */
function isNodeError(error: unknown, code: string): boolean {
  return typeof error === "object" && error !== null && "code" in error && error.code === code;
}
