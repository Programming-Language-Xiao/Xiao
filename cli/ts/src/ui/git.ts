/** 提示符专用 Git 状态探测；失败只影响摘要，不影响输入循环。 */

import { spawn } from "node:child_process";

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

/** 异步启动单条 Git 命令，限制双向输出并在取消时立即结束子进程。 */
async function readGitStatus(cwd: string, env: NodeJS.ProcessEnv, signal: AbortSignal): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn("git", ["status", "--porcelain=v2", "--branch"], {
      cwd, env, shell: false, windowsHide: true, stdio: ["ignore", "pipe", "pipe"],
    });
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    let stdoutBytes = 0;
    let stderrBytes = 0;
    let settled = false;
    const finish = (error: Error | null, output = "") => {
      if (settled) return;
      settled = true;
      signal.removeEventListener("abort", onAbort);
      if (error === null) resolve(output);
      else reject(error);
    };
    const onAbort = () => {
      child.kill();
      finish(Object.assign(new Error("Git 探测已取消"), { code: "ABORT_ERR" }));
    };
    const collect = (chunk: Buffer, chunks: Buffer[], bytes: number): number => {
      const total = bytes + chunk.byteLength;
      if (total > 64 * 1024) {
        child.kill();
        finish(new Error("Git 状态输出超过上限"));
      } else chunks.push(chunk);
      return total;
    };
    child.stdout.on("data", (chunk: Buffer) => { stdoutBytes = collect(chunk, stdout, stdoutBytes); });
    child.stderr.on("data", (chunk: Buffer) => { stderrBytes = collect(chunk, stderr, stderrBytes); });
    child.on("error", (error) => finish(error));
    child.on("close", (code) => {
      if (code === 0) finish(null, Buffer.concat(stdout).toString("utf8"));
      else finish(Object.assign(new Error("Git 状态查询失败"), { stderr: Buffer.concat(stderr).toString("utf8") }));
    });
    signal.addEventListener("abort", onAbort, { once: true });
    if (signal.aborted) onAbort();
  });
}

/** 合并稳定编号与降级原因。 */
function failure(reason: GitProbeDiagnostic["reason"]): GitProbeResult {
  return { summary: null, diagnostic: { code: "X11-REPL-GIT-001", reason } };
}

/** 识别进程创建失败。 */
function isNodeError(error: unknown, code: string): boolean {
  return typeof error === "object" && error !== null && "code" in error && error.code === code;
}
