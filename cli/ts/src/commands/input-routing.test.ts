/** 18A 五类输入路由与互斥/缺失诊断回归。 */

import { describe, expect, test } from "bun:test";

import { routeInput } from "./input-routing.ts";

describe("18A 输入路由", () => {
  test("按意图区分 config、main 源码、普通源码、xiaoc 和 xar", () => {
    expect(routeInput(["config.xiao"], "build").kind).toBe("config");
    expect(routeInput(["main.xiao"], "build", new Map([["main.xiao", "[main]\nprint(1)\n"]])).kind).toBe("main-source");
    expect(routeInput(["module.xiao"], "run", new Map([["module.xiao", "print(1)\n"]])).kind).toBe("source");
    expect(routeInput(["module.xiaoc"], "run").kind).toBe("xiaoc");
    expect(routeInput(["app.xar"], "run").kind).toBe("xar");
  });

  test("互斥、缺失意图和 build 产物输入都有稳定诊断", () => {
    expect(() => routeInput([], "run")).toThrow("X11-CLI-INPUT-003");
    expect(() => routeInput(["a.xiao", "b.xiao"], "build")).toThrow("X11-CLI-INPUT-004");
    expect(() => routeInput(["app.xar"], "build")).toThrow("X11-CLI-INPUT-005");
    expect(() => routeInput(["config.xiao"], "run")).toThrow("X11-CLI-INPUT-006");
    expect(() => routeInput(["unknown.bin"], "run")).toThrow("X11-CLI-INPUT-002");
  });
});
