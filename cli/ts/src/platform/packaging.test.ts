/** X0-C 目标解析和分发命名回归。 */

import { describe, expect, test } from "bun:test";

import { hostPackageTarget, parsePackageTarget, parsePackagingArguments } from "./packaging.ts";

describe("CLI 独立打包目标", () => {
  test("目标参数与 Rust 目标描述保持一致", () => {
    const target = parsePackageTarget("bun-windows-x64");
    expect(target).toMatchObject({
      bunTarget: "bun-windows-x64",
      platform: "win32",
      architecture: "x64",
      executableName: "xiao.exe",
      coreName: "xiao-core.exe",
    });
    expect(target.rustTarget.triple).toBe("x86_64-pc-windows-msvc");
    expect(parsePackageTarget("bun-darwin-arm64").rustTarget.triple).toBe("aarch64-apple-darwin");
  });

  test("默认目标、参数和非法目标保持稳定形状", () => {
    expect(hostPackageTarget("linux", "x64").bunTarget).toBe("bun-linux-x64");
    expect(parsePackagingArguments(["--target=bun-linux-x64", "--outdir", "release", "--core", "core-bin"])).toEqual({
      target: "bun-linux-x64",
      outDir: "release",
      corePath: "core-bin",
      help: false,
    });
    expect(() => parsePackageTarget("bun-freebsd-x64")).toThrow("X11-PACKAGE-TARGET-001");
  });
});
