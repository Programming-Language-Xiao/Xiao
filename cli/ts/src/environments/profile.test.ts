/** E4 profile 安装的无终端回归，所有操作均注入临时路径。 */

import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { executeCommand } from "../commands/index.ts";
import { parseArguments } from "../commands/parser.ts";

test("安装显式备份、重复无叠加、移除只删标记块并保留原换行", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-shell-profile-"));
  const profile = join(directory, "test.profile");
  const original = "用户配置\r\n下一行无终止换行";
  try {
    await writeFile(profile, original);
    const install = await executeCommand(parseArguments(["shell-init", "bash", "--install", "--profile", profile]));
    expect(install.exitCode).toBe(0);
    expect(install.stdout).toContain(`已安装 Shell 钩子：${profile}`);
    const backup = install.stdout.match(/备份：(.*)\n/u)?.[1];
    expect(backup).toBeDefined();
    expect(await readFile(backup!, "utf8")).toBe(original);
    const installed = await readFile(profile, "utf8");
    expect(installed.match(/# >>> xiao init >>>/gu)).toHaveLength(1);
    expect(installed).toContain("eval \"$(command xiao shell-init bash)\"");
    const again = await executeCommand(parseArguments(["shell-init", "bash", "--install", "--profile", profile]));
    expect(again.stdout).toContain("无需更改");
    expect(await readFile(profile, "utf8")).toBe(installed);
    const uninstall = await executeCommand(parseArguments(["shell-init", "bash", "--uninstall", "--profile", profile]));
    expect(uninstall.exitCode).toBe(0);
    expect(await readFile(profile, "utf8")).toBe(original);
    expect(uninstall.stdout).toContain("备份：");
    expect((await executeCommand(parseArguments(["shell-init", "bash", "--uninstall", "--profile", profile]))).stdout).toContain("无需更改");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("默认目标按 Shell 规则选择，fish 缺少父目录明确失败且不创建", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-shell-home-"));
  const env = { ...process.env, HOME: directory };
  try {
    const missing = await executeCommand(parseArguments(["shell-init", "fish", "--install"]), { env });
    expect(missing.exitCode).not.toBe(0);
    expect(missing.stderr).toContain(process.platform === "win32" ? "X11-CLI-SHELL-003" : "profile 上级目录不可用");
    if (process.platform !== "win32") {
      const bash = await executeCommand(parseArguments(["shell-init", "bash", "--install"]), { env });
      expect(bash.stdout).toContain(join(directory, ".bashrc"));
      const zsh = await executeCommand(parseArguments(["shell-init", "zsh", "--install"]), { env });
      expect(zsh.stdout).toContain(join(directory, ".zshrc"));
      expect(await readFile(join(directory, ".zshrc"), "utf8")).toContain("shell-init zsh");
    }
    const explicit = join(directory, "fish.profile");
    const created = await executeCommand(parseArguments(["shell-init", "fish", "--install", "--profile", explicit]));
    expect(created.exitCode).toBe(0);
    expect(created.stdout).toContain("未改写既有文件，无备份");
    expect(await readFile(explicit, "utf8")).toContain("shell-init fish | source -");
    const removed = await executeCommand(parseArguments(["shell-init", "fish", "--uninstall", "--profile", explicit]));
    expect(removed.exitCode).toBe(0);
    expect(await readFile(explicit, "utf8")).toBe("");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("移除时保留标记块之后的用户配置且不将两段配置粘连", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-shell-suffix-"));
  const profile = join(directory, "profile");
  try {
    await writeFile(profile, "BEFORE=1");
    expect((await executeCommand(parseArguments(["shell-init", "zsh", "--install", "--profile", profile]))).exitCode).toBe(0);
    await writeFile(profile, `${await readFile(profile, "utf8")}AFTER=1\n`);
    const removed = await executeCommand(parseArguments(["shell-init", "zsh", "--uninstall", "--profile", profile]));
    expect(removed.exitCode).toBe(0);
    expect(await readFile(profile, "utf8")).toBe("BEFORE=1\nAFTER=1\n");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("畸形标记、非绝对路径和无目标的 PowerShell 安装明确失败", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-shell-error-"));
  const profile = join(directory, "profile");
  try {
    await writeFile(profile, "用户内容\n# >>> xiao init >>>\n未闭合");
    const invalid = await executeCommand(parseArguments(["shell-init", "bash", "--uninstall", "--profile", profile]));
    expect(invalid.stderr).toContain("X11-CLI-SHELL-005");
    expect(await readFile(profile, "utf8")).toContain("未闭合");
    const relative = await executeCommand(parseArguments(["shell-init", "zsh", "--install", "--profile", "relative"]));
    expect(relative.stderr).toContain("X11-CLI-SHELL-003");
    const powershell = await executeCommand(parseArguments(["shell-init", "powershell", "--install"]));
    expect(powershell.stderr).toContain("X11-CLI-SHELL-003");
    const printed = await executeCommand(parseArguments(["shell-init", "fish", "--json"]));
    expect(JSON.parse(printed.stdout)).toMatchObject({ type: "shell_init", shell: "fish" });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
