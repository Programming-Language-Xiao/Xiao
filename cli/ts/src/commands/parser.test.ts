/** CLI 参数解析的稳定命令面回归。 */

import { describe, expect, test } from "bun:test";

import { CliArgumentError, parseArguments } from "./parser.ts";

describe("xiao 命令解析", () => {
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

  test("非法参数不依赖本地化文本判断", () => {
    expect(() => parseArguments(["run"])).toThrow(CliArgumentError);
    expect(() => parseArguments(["--color=rainbow"])).toThrow("X11-CLI-ARG-001");
    expect(() => parseArguments(["config", "unknown.path", "true"])).not.toThrow();
  });
});
