/** 核心发现和主机目标回归。 */

import { describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { CoreDiscoveryError, developmentCandidates, discoverCore, discoverCoreWithMetadata, hostTarget } from "./core.ts";

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

  test("生产同目录候选优先于 PATH 和开发回环", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-core-adjacent-"));
    try {
      const executable = join(directory, "xiao.exe");
      const adjacent = join(directory, "xiao-core.exe");
      await Bun.write(executable, "cli");
      await Bun.write(adjacent, "core");
      const result = await discoverCoreWithMetadata({
        cwd: directory,
        executablePath: executable,
        platform: "win32",
        architecture: "x64",
        env: { PATH: "" },
      });
      expect(result.path).toBe(adjacent);
      expect(result.source).toBe("adjacent");
      expect(result.candidates.map((candidate) => candidate.source)).toEqual(["adjacent"]);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("没有仓库根时不把开发候选混入生产诊断", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-core-production-"));
    try {
      await expect(discoverCoreWithMetadata({
        cwd: directory,
        executablePath: join(directory, "xiao.exe"),
        platform: "win32",
        architecture: "x64",
        env: { PATH: "" },
      })).rejects.toMatchObject({
        candidateDetails: [{ source: "adjacent" }],
      });
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("开发回环候选带有 development 来源标记", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-core-development-"));
    try {
      await Bun.write(join(directory, "package.json"), "{}\n");
      await mkdir(join(directory, "core", "rust", "target", "debug"), { recursive: true });
      await Bun.write(join(directory, "core", "rust", "Cargo.toml"), "[workspace]\n");
      const core = join(directory, "core", "rust", "target", "debug", "xiao-core.exe");
      await Bun.write(core, "core");
      const result = await discoverCoreWithMetadata({
        cwd: join(directory, "nested"),
        executablePath: join(directory, "nested", "xiao.exe"),
        platform: "win32",
        architecture: "x64",
        env: { PATH: "" },
      });
      expect(result.path).toBe(core);
      expect(result.source).toBe("development");
      expect(result.candidates.at(-1)?.source).toBe("development");
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
});
