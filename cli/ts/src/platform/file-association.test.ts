/** 17D 三平台关联设计与双击行为契约测试。 */

import { describe, expect, test } from "bun:test";
import { chmod, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  archiveLaunchContract,
  checkFileAssociation,
  fileAssociationPlan,
  installFileAssociation,
  noAssociationPrompt,
  uninstallFileAssociation,
} from "./file-association.ts";

describe(".xar 文件关联契约", () => {
  test("三平台只生成设计，不执行安装副作用", () => {
    for (const platform of ["win32", "linux", "darwin"] as const) {
      const plan = fileAssociationPlan(platform);
      expect(plan.extension).toBe(".xar");
      expect(plan.command).toContain("-xar");
      expect(plan.registrationTarget.length).toBeGreaterThan(0);
      expect(plan.uninstall.length).toBeGreaterThan(0);
    }
  });

  test("双击契约不依赖 cwd，参数、IO 和退出码与命令行一致", () => {
    const contract = archiveLaunchContract("C:/packages/demo.xar", ["--arg"]);
    expect(contract.cwdPolicy).toBe("launcher-directory-independent");
    expect(contract.argv).toEqual(["-xar", "C:/packages/demo.xar", "--arg"]);
    expect(contract.stdio).toBe("inherit");
    expect(contract.exitCode).toBe("forwarded");
  });

  test("未安装关联时的提示保持双语且不承诺自包含 Runtime", () => {
    expect(noAssociationPrompt("zh-CN")).toContain("xiao -xar");
    expect(noAssociationPrompt("en-US")).toContain("Xiao Runtime");
    expect(noAssociationPrompt("zh-CN")).not.toContain("自包含");
  });

  test("Linux 安装、检查、重复执行和移除都走用户目录且检查无副作用", async () => {
    const home = await mkdtemp(join(tmpdir(), "xiao-association-linux-"));
    const executable = join(home, "xiao");
    await Bun.write(executable, "#!/bin/sh\n");
    await chmod(executable, 0o755);
    const runner = async () => ({ status: 0, stdout: "", stderr: "" });
    try {
      const installed = await installFileAssociation({ platform: "linux", homeDir: home, executablePath: executable, runCommand: runner });
      expect(installed.changed).toBe(true);
      const desktopPath = join(home, ".local", "share", "applications", "xiao-xar.desktop");
      const before = await readFile(desktopPath, "utf8");
      const checked = await checkFileAssociation({ platform: "linux", homeDir: home, executablePath: executable, runCommand: runner });
      expect(checked.installed).toBe(true);
      expect(checked.matches).toBe(true);
      expect(checked.changed).toBe(false);
      expect(await readFile(desktopPath, "utf8")).toBe(before);
      const repeated = await installFileAssociation({ platform: "linux", homeDir: home, executablePath: executable, runCommand: runner });
      expect(repeated.changed).toBe(false);
      const removed = await uninstallFileAssociation({ platform: "linux", homeDir: home, executablePath: executable, runCommand: runner });
      expect(removed.changed).toBe(true);
      expect((await checkFileAssociation({ platform: "linux", homeDir: home, executablePath: executable, runCommand: runner })).installed).toBe(false);
    } finally {
      await rm(home, { recursive: true, force: true });
    }
  });

  test("Windows 和 macOS 执行体使用可注入平台命令并保留卸载路径", async () => {
    const home = await mkdtemp(join(tmpdir(), "xiao-association-platforms-"));
    const executable = join(home, "xiao.exe");
    await Bun.write(executable, "xiao");
    // macOS 路径按 X_OK 校验；测试夹具在 Unix 宿主也必须具备执行位。
    await chmod(executable, 0o755);
    const calls: string[] = [];
    const runner = async (command: string, args: readonly string[]) => {
      calls.push(`${command} ${args.join(" ")}`);
      return { status: 0, stdout: "", stderr: "" };
    };
    try {
      const windows = await installFileAssociation({ platform: "win32", executablePath: executable, runCommand: runner });
      expect(windows.registration).toContain("HKCU");
      expect(calls.some((call) => call.startsWith("reg.exe ADD"))).toBe(true);
      const mac = await installFileAssociation({ platform: "darwin", homeDir: home, executablePath: executable, env: { XIAO_ALLOW_MACOS_ASSOCIATION: "1" }, runCommand: runner });
      expect(mac.gated).toBe(true);
      expect(mac.registration).toContain("Info.plist");
      const removed = await uninstallFileAssociation({ platform: "darwin", homeDir: home, executablePath: executable, env: { XIAO_ALLOW_MACOS_ASSOCIATION: "1" }, runCommand: runner });
      expect(removed.changed).toBe(true);
    } finally {
      await rm(home, { recursive: true, force: true });
    }
  });
});
