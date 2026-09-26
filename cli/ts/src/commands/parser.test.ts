/** CLI 参数解析的稳定命令面回归。 */

import { describe, expect, test } from "bun:test";

import { CliArgumentError, parseArguments } from "./parser.ts";

describe("xiao 命令解析", () => {
  test("同步模式互斥，install/i 共享命令形状", () => {
    expect(parseArguments(["sync", "--keep-extra", "--frozen"])).toMatchObject({ kind: "sync", keepExtra: true, frozen: true, locked: false });
    expect(parseArguments(["i"])).toEqual(parseArguments(["install"]));
    expect(parseArguments(["install", "./project"])).toEqual(parseArguments(["i", "./project"]));
    expect(parseArguments(["--json", "i", "./project/config.xiao"])).toMatchObject({ kind: "install", project: "./project/config.xiao", options: { json: true } });
    for (const argumentsList of [["sync", "--locked", "--frozen"], ["sync", "--unknown"], ["i", "--locked"], ["i", "one", "two"], ["install", ""], ["sync", "--locked", "--locked"]]) {
      expect(() => parseArguments(argumentsList)).toThrow(CliArgumentError);
    }
  });

  test("锁定与依赖编辑命令的参数边界", () => {
    expect(parseArguments(["lock"])).toMatchObject({ kind: "lock" });
    expect(parseArguments(["update"])).toMatchObject({ kind: "update" });
    expect(parseArguments(["add", "lib", "--path", "../lib", "--version", "1.2", "--dev"])).toMatchObject({
      kind: "add", packageName: "lib", path: "../lib", version: "1.2", dev: true,
    });
    expect(parseArguments(["remove", "lib", "--dev"])).toMatchObject({ kind: "remove", packageName: "lib", dev: true });
    for (const args of [["lock", "lib"], ["update", "--frozen"], ["add", "lib"], ["add", "lib", "--path"], ["add", "lib", "--path", "one", "--path", "two"], ["remove", "lib", "--version", "1"]]) {
      expect(() => parseArguments(args)).toThrow(CliArgumentError);
    }
  });

  test("支持 run、源码快捷方式和全局输出选项", () => {
    expect(parseArguments(["run", "main.xiao", "--json"]).kind).toBe("run");
    expect(parseArguments(["main.xiao", "--color=always"]).kind).toBe("run");
    expect(parseArguments(["config", "--global", "CLI.git.summary", "true"]).kind).toBe("config");
    expect(parseArguments(["run", "--help"]).kind).toBe("help");
    const debugRun = parseArguments(["run", "main.xiao", "-debug"]);
    expect(debugRun.kind).toBe("run");
    expect(debugRun.kind === "run" && debugRun.options.debug).toBe(true);
  });

  test("未实现入口保持稳定命令身份", () => {
    expect(parseArguments([]).kind).toBe("repl");
    expect(parseArguments(["test"]).kind).toBe("test");
    expect(parseArguments(["build", "main.xiao"]).kind).toBe("build");
    expect(parseArguments(["-debug"]).kind).toBe("repl");
  });

  test("build 默认产物、LLVM 输出和优化级别保持冻结", () => {
    const command = parseArguments(["build", "src/main.xiao", "--emit-llvm", "out/main.ll", "-debug"]);
    expect(command).toMatchObject({ kind: "build", file: "src/main.xiao", llvmIrOutput: "out/main.ll" });
    if (command.kind === "build") {
      expect(command.output).toMatch(/(?:^|[\\/])build[\\/]main(?:\.exe)?$/u);
      expect(command.optimizationLevel).toBe(0);
      expect(command.options.debug).toBe(true);
    }
    expect(() => parseArguments(["build", "main.xiao", "-O1"])).toThrow("X11-CLI-ARG-001");
  });

  test("环境命令解析默认名、显式名、Shell 和取消激活", () => {
    expect(parseArguments(["venv"])).toMatchObject({ kind: "venv" });
    expect(parseArguments(["venv", "dev", "--color=never"])).toMatchObject({ kind: "venv", name: "dev" });
    expect(parseArguments(["shell-init", "bash"])).toMatchObject({ kind: "shell-init", shell: "bash" });
    expect(parseArguments(["shell-init", "zsh"])).toMatchObject({ kind: "shell-init", shell: "zsh", action: "print" });
    expect(parseArguments(["shell-init", "fish", "--install"])).toMatchObject({ kind: "shell-init", shell: "fish", action: "install" });
    expect(parseArguments(["shell-init", "bash", "--uninstall", "--profile", "/tmp/profile"])).toMatchObject({ kind: "shell-init", action: "uninstall", profile: "/tmp/profile" });
    expect(parseArguments(["shell-init", "pwsh"])).toMatchObject({ kind: "shell-init", shell: "powershell" });
    expect(parseArguments(["shell-init", "cmd.exe"])).toMatchObject({ kind: "shell-init", shell: "cmd" });
    expect(parseArguments(["deactivate"])).toMatchObject({ kind: "deactivate" });
    expect(() => parseArguments(["venv", "one", "two"])).toThrow(CliArgumentError);
    expect(() => parseArguments(["shell-init", "unknown"])).toThrow("X11-CLI-SHELL-001");
    expect(() => parseArguments(["shell-init", "cmd", "--install"])).toThrow("X11-CLI-SHELL-002");
    for (const args of [["shell-init", "bash", "--install", "--uninstall"], ["shell-init", "fish", "--profile", "/tmp/profile"], ["shell-init", "zsh", "--install", "--install"], ["shell-init", "bash", "--install", "--profile"], ["shell-init", "fish", "--wat"]]) {
      expect(() => parseArguments(args)).toThrow(CliArgumentError);
    }
    expect(() => parseArguments(["deactivate", "dev"])).toThrow(CliArgumentError);
  });

  test("非法参数不依赖本地化文本判断", () => {
    expect(() => parseArguments(["run"])).toThrow(CliArgumentError);
    expect(() => parseArguments(["--color=rainbow"])).toThrow("X11-CLI-ARG-001");
    expect(() => parseArguments(["config", "unknown.path", "true"])).not.toThrow();
  });
});
