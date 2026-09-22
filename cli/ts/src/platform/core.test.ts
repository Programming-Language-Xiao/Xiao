/** 核心发现和主机目标回归。 */

import { describe, expect, test } from "bun:test";

import { CoreDiscoveryError, developmentCandidates, discoverCore, hostTarget } from "./core.ts";

describe("xiao-core 平台接线", () => {
  test("主机目标字段与平台文件格式一致", () => {
    expect(hostTarget("win32", "x64")).toEqual({ triple: "x86_64-pc-windows-msvc", pointer_width: 64, endian: "little", object_format: "coff" });
    expect(hostTarget("linux", "x64").object_format).toBe("elf");
    expect(hostTarget("darwin", "arm64").triple).toBe("aarch64-apple-darwin");
    expect(developmentCandidates("win32")[0]).toEndWith("xiao-core.exe");
  });

  test("显式覆盖路径不存在时使用稳定诊断", async () => {
    await expect(discoverCore({ overridePath: "C:/does-not-exist/xiao-core.exe" })).rejects.toBeInstanceOf(CoreDiscoveryError);
    await expect(discoverCore({ overridePath: "C:/does-not-exist/xiao-core.exe" })).rejects.toMatchObject({ code: "X11-CLI-CORE-001" });
  });
});
