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
test.skipIf(!gitAvailable)("冷进程首次探测及时返回真实无上游分支", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-git-untracked-"));
  try {
    execFileSync(gitCommand, gitArgs(["init", "-b", "main"]), { cwd: directory, windowsHide: true, stdio: "ignore" });
    const moduleUrl = pathToFileURL(join(import.meta.dir, "git.ts")).href;
    const script = `import { probeGitSummary } from ${JSON.stringify(moduleUrl)}; console.log(JSON.stringify(await probeGitSummary(${JSON.stringify(directory)})));`;
    const started = performance.now();
    const child = Bun.spawn([process.execPath, "-e", script], { cwd: directory, stdout: "pipe", stderr: "pipe" });
    const deadline = setTimeout(() => child.kill(), 2000);
    try {
      const [exitCode, output, errors] = await Promise.all([
        child.exited, new Response(child.stdout).text(), new Response(child.stderr).text(),
      ]);
      expect(exitCode).toBe(0);
      expect(errors).toBe("");
      expect(performance.now() - started).toBeLessThan(2000);
      expect(JSON.parse(output)).toMatchObject({ summary: { branch: "main", ahead: null, behind: null } });
    } finally {
      clearTimeout(deadline);
    }
    expect((await probeGitSummary(directory, { timeoutMs: 2000 })).summary).toEqual({ branch: "main", ahead: null, behind: null });
    execFileSync(gitCommand, gitArgs(["-c", "user.name=Test", "-c", "user.email=test@example.org", "commit", "--allow-empty", "-m", "first"]), { cwd: directory, windowsHide: true, stdio: "ignore" });
    execFileSync(gitCommand, gitArgs(["checkout", "--detach", "HEAD"]), { cwd: directory, windowsHide: true, stdio: "ignore" });
    expect((await probeGitSummary(directory, { timeoutMs: 2000 })).summary).toBeNull();
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});


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
