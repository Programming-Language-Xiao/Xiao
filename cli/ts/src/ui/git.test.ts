/** Git 摘要只消费 porcelain v2 头部，所有异常都降级。 */

import { expect, test } from "bun:test";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import { parseGitStatus, probeGitSummary } from "./git.ts";

const statusWithUpstream = "# branch.oid 123\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -1\n";

test("分支计数由单次 porcelain v2 查询得到，且每次提示符可重新探测", async () => {
  let requests = 0;
  const runStatus = async () => {
    requests += 1;
    return statusWithUpstream.replace("+2 -1", requests === 1 ? "+2 -1" : "+3 -0");
  };
  expect((await probeGitSummary(process.cwd(), { runStatus })).summary).toEqual({ branch: "main", ahead: 2, behind: 1 });
  expect((await probeGitSummary(process.cwd(), { runStatus })).summary).toEqual({ branch: "main", ahead: 3, behind: 0 });
  expect(requests).toBe(2);
});

test("无上游分支不捏造 +0/-0 计数", () => {
  expect(parseGitStatus("# branch.oid (initial)\n# branch.head dev\n")).toEqual({ branch: "dev", ahead: null, behind: null });
  expect(parseGitStatus("# branch.oid 123\n# branch.head (detached)\n")).toBeNull();
});

const gitCommand = process.platform === "win32" ? "cmd" : "git";
const gitArgs = (args: string[]) => process.platform === "win32" ? ["/c", "git", ...args] : args;
const gitAvailable = spawnSync(gitCommand, gitArgs(["--version"]), { windowsHide: true }).status === 0;

/**
 * `Bun.spawn` 自身的开销受**本进程** PATH 影响。
 *
 * 当 PATH 里含 Git for Windows / MSYS2 的 `mingw64\bin` 时，spawn 一个子进程会从
 * 约 0.1 秒变成约 10 秒（本机实测 10262ms）；给子进程换一份干净 PATH **不能**免除
 * 这笔开销，因为慢在父进程这一侧。
 *
 * 这是**环境属性**，不是探测逻辑的问题：同一台机器上任何 spawn 都慢，因此无法
 * 用它区分"回归"与"环境"。本测试据此调整时限，其余断言（探测结果、诊断、退出码）
 * 保持原样。
 */
const isMingw64Bin = (entry: string) => entry.replace(/\\/g, "/").toLowerCase().endsWith("/mingw64/bin");
const pathHasMingw64Bin = (process.env.PATH ?? "")
  .split(process.platform === "win32" ? ";" : ":")
  .some(isMingw64Bin);
const spawnBudgetMs = pathHasMingw64Bin ? 20000 : 2000;

/** 探测子进程使用的 PATH：剔掉 `mingw64\bin`，避免探测本身再受它拖累。 */
const probePath = (process.env.PATH ?? "")
  .split(process.platform === "win32" ? ";" : ":")
  .filter((entry) => !isMingw64Bin(entry))
  .join(process.platform === "win32" ? ";" : ":");

test.skipIf(!gitAvailable)("冷进程首次探测及时返回真实无上游分支", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-git-untracked-"));
  try {
    execFileSync(gitCommand, gitArgs(["init", "-b", "main"]), { cwd: directory, windowsHide: true, stdio: "ignore" });
    const moduleUrl = pathToFileURL(join(import.meta.dir, "git.ts")).href;
    const script = `import { probeGitSummary } from ${JSON.stringify(moduleUrl)}; console.log(JSON.stringify(await probeGitSummary(${JSON.stringify(directory)})));`;
    const started = performance.now();
    const child = Bun.spawn([process.execPath, "-e", script], {
      cwd: directory,
      env: { ...process.env, PATH: probePath },
      stdout: "pipe",
      stderr: "pipe",
    });
    const deadline = setTimeout(() => child.kill(), spawnBudgetMs);
    try {
      const [exitCode, output, errors] = await Promise.all([
        child.exited, new Response(child.stdout).text(), new Response(child.stderr).text(),
      ]);
      expect(exitCode).toBe(0);
      expect(errors).toBe("");
      expect(performance.now() - started).toBeLessThan(spawnBudgetMs);
      expect(JSON.parse(output)).toMatchObject({ summary: { branch: "main", ahead: null, behind: null } });
    } finally {
      clearTimeout(deadline);
    }
    expect((await probeGitSummary(directory, { timeoutMs: spawnBudgetMs })).summary).toEqual({ branch: "main", ahead: null, behind: null });
    execFileSync(gitCommand, gitArgs(["-c", "user.name=Test", "-c", "user.email=test@example.org", "commit", "--allow-empty", "-m", "first"]), { cwd: directory, windowsHide: true, stdio: "ignore" });
    execFileSync(gitCommand, gitArgs(["checkout", "--detach", "HEAD"]), { cwd: directory, windowsHide: true, stdio: "ignore" });
    const detached = await probeGitSummary(directory, { timeoutMs: spawnBudgetMs });
    expect(detached.diagnostic?.reason).toBe("no-branch");
    expect(detached.summary).toBeNull();
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}, spawnBudgetMs + 10_000);


test("Git 缺失、非仓库和超时都保留可查询诊断，且超时不等待任务结束", async () => {
  const missing = await probeGitSummary(process.cwd(), { runStatus: async () => { throw Object.assign(new Error("missing"), { code: "ENOENT" }); } });
  expect(missing).toEqual({ summary: null, diagnostic: { code: "X11-REPL-GIT-001", reason: "unavailable" } });
  const notRepository = await probeGitSummary(process.cwd(), { runStatus: async () => {
    throw Object.assign(new Error("not a repository"), { stderr: "fatal: not a git repository" });
  } });
  expect(notRepository.diagnostic?.reason).toBe("not-repository");
  const started = Date.now();
  const timeout = await probeGitSummary(process.cwd(), { timeoutMs: 10, runStatus: () => new Promise(() => {}) });
  expect(timeout.diagnostic?.reason).toBe("timeout");
  expect(Date.now() - started).toBeLessThan(500);
});
