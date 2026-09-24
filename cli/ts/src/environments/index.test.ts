/** 11A-E0 环境目录、提示符状态和 Shell 钩子协议回归。 */

import { describe, expect, test } from "bun:test";

import {
  activatePromptState,
  deactivatePromptState,
  environmentLayout,
  environmentPromptPrefix,
  removeEnvironmentPrefix,
  shellInitScript,
} from "./index.ts";
import { stripAnsi } from "../ui/color.ts";

describe("项目环境边界", () => {
  test("默认和显式目录遵循冻结规则", () => {
    expect(environmentLayout("C:/project")).toMatchObject({ logicalName: "venv", directoryName: ".venv", path: "C:\\project\\.venv" });
    expect(environmentLayout("C:/project", "dev")).toMatchObject({ logicalName: "dev", directoryName: "dev", path: "C:\\project\\dev" });
  });

  test("提示符状态重复激活、切换和取消不叠加", () => {
    const original = { prompt: "PS C:\\project> " };
    const dev = activatePromptState(original, "dev", { isTTY: true, noColor: false, term: "xterm", color: "always" });
    expect(stripAnsi(dev.prompt)).toBe("$dev$ PS C:\\project> ");
    expect(dev.prompt).toContain("\u001B[32m");
    const testEnvironment = activatePromptState(dev, "test", { isTTY: false });
    expect(testEnvironment.prompt).toBe("$test$ PS C:\\project> ");
    expect(stripAnsi(testEnvironment.prompt)).not.toContain("$dev$ $test$");
    expect(deactivatePromptState(testEnvironment)).toEqual({ prompt: "PS C:\\project> " });
  });

  test("前缀颜色遵守非 TTY、NO_COLOR、dumb 和 never 降级", () => {
    for (const options of [
      { isTTY: false },
      { isTTY: true, noColor: true },
      { isTTY: true, term: "dumb" },
      { isTTY: true, color: "never" as const },
    ]) {
      expect(environmentPromptPrefix("dev", options)).toBe("$dev$ ");
    }
  });

  test("Shell 钩子不改 profile，cmd 明确降级", () => {
    const bash = shellInitScript("bash");
    expect(bash).toContain("PS1");
    expect(bash).toContain("XIAO_ORIGINAL_PS1");
    expect(bash).toContain("deactivate");
    const powershell = shellInitScript("powershell");
    expect(powershell).toContain("[char]27");
    expect(powershell).toContain("Windows PowerShell 5.1");
    const cmd = shellInitScript("cmd");
    expect(cmd).toContain("不支持由子进程修改父会话");
    expect(cmd).not.toContain("AutoRun");
  });

  test("去除带 ANSI 的旧前缀不破坏原提示符", () => {
    const prompt = "\u001B[32m$dev$ \u001B[0mPS> ";
    expect(removeEnvironmentPrefix(prompt)).toBe("PS> ");
  });
});
