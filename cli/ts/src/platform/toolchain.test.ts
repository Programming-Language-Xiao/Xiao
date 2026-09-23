/** 主机工具链发现的候选顺序、版本门槛和目标/链接探测回归。 */

import { describe, expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  ToolchainDiscoveryError,
  discoverToolchainWithMetadata,
  parseToolchainVersion,
} from "./toolchain.ts";

/** 在临时目录创建只模拟版本与退出状态的宿主工具。 */
async function fakeTool(directory: string, name: string, version: string, target?: string, exitCode = 0): Promise<string> {
  const windows = process.platform === "win32";
  const path = join(directory, windows ? `${name}.cmd` : name);
  const targetLine = target ? (windows ? `echo Target: ${target}` : `echo 'Target: ${target}'`) : null;
  const body = windows
    ? `@echo off\nif "%1"=="--version" (echo clang version ${version}${targetLine ? `&${targetLine}` : ""}&exit /b 0)\nexit /b ${exitCode}\n`
    : `#!/bin/sh\nif [ "$1" = "--version" ]; then echo "clang version ${version}"${targetLine ? `; ${targetLine}` : ""}; exit 0; fi\nexit ${exitCode}\n`;
  await Bun.write(path, body);
  if (!windows) await chmod(path, 0o755);
  return path;
}

/** 创建隔离临时目录，并在用例结束后递归回收。 */
async function withDirectory<T>(callback: (directory: string) => Promise<T>): Promise<T> {
  const directory = await mkdtemp(join(tmpdir(), "xiao-toolchain-test-"));
  try {
    return await callback(directory);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

/** 构造只暴露测试工具目录的隔离进程环境。 */
function environment(directory: string, extra: NodeJS.ProcessEnv = {}): NodeJS.ProcessEnv {
  return { ...process.env, PATH: directory, ...extra };
}

describe("主机工具链发现", () => {
  test("显式 clang 无效时不回退到 PATH 候选，并保留 override 来源", async () => {
    await withDirectory(async (directory) => {
      const valid = await fakeTool(directory, "clang", "22.0.0");
      await expect(discoverToolchainWithMetadata({
        cwd: directory,
        env: environment(directory, { XIAO_CLANG: join(directory, "missing-clang") }),
        probeLink: false,
      })).rejects.toMatchObject({
        code: "X11-CLI-TOOLCHAIN-OVERRIDE-001",
        candidates: [{ source: "override", status: "missing" }],
      });
      expect(valid).toContain("clang");
    });
  }, 15_000);

  test("显式 clang 始终执行所选路径而不是工作目录中的同名文件", async () => {
    await withDirectory(async (directory) => {
      await fakeTool(directory, "clang", "17.0.0");
      const selectedDirectory = join(directory, "selected");
      await mkdir(selectedDirectory);
      const selected = await fakeTool(selectedDirectory, "clang", "22.0.0");
      const result = await discoverToolchainWithMetadata({
        cwd: directory,
        env: environment(directory, { XIAO_CLANG: selected }),
        probeLink: false,
      });
      expect(result.toolchain.clang).toBe(selected);
      expect(result.toolchain.versions.clang).toContain("22.0.0");
    });
  }, 15_000);

  test("低于 llvm-toolchain.toml 门槛的 clang 稳定拒绝", async () => {
    await withDirectory(async (directory) => {
      const clang = await fakeTool(directory, "clang", "17.0.0");
      await expect(discoverToolchainWithMetadata({
        cwd: directory,
        env: environment(directory, { XIAO_CLANG: clang }),
        probeLink: false,
      })).rejects.toMatchObject({ code: "X11-CLI-TOOLCHAIN-VERSION-001" });
    });
  }, 15_000);

  test("版本报告的目标与请求目标冲突时在发现阶段拒绝", async () => {
    await withDirectory(async (directory) => {
      const clang = await fakeTool(directory, "clang", "22.0.0", "x86_64-unknown-linux-gnu");
      const target = { triple: "x86_64-apple-darwin", pointer_width: 64, endian: "little" as const, object_format: "macho" as const };
      await expect(discoverToolchainWithMetadata({
        cwd: directory,
        target,
        env: environment(directory, { XIAO_CLANG: clang }),
        probeLink: false,
      })).rejects.toMatchObject({ code: "X11-CLI-TOOLCHAIN-TARGET-001" });
    });
  }, 15_000);

  test("debug 的显式诊断组件路径无效时不回退到 PATH", async () => {
    await withDirectory(async (directory) => {
      const clang = await fakeTool(directory, "clang", "22.0.0");
      await expect(discoverToolchainWithMetadata({
        cwd: directory,
        env: environment(directory, {
          XIAO_CLANG: clang,
          XIAO_DIAGNOSTICS_PATH: join(directory, "missing-diagnostics"),
        }),
        requireDiagnostics: true,
        probeLink: false,
      })).rejects.toMatchObject({ code: "X11-CLI-TOOLCHAIN-OVERRIDE-001" });
    });
  }, 15_000);

  test("真实链接探测失败时给出 LINK 诊断", async () => {
    await withDirectory(async (directory) => {
      const clang = await fakeTool(directory, "clang", "22.0.0", undefined, 1);
      await expect(discoverToolchainWithMetadata({
        cwd: directory,
        env: environment(directory, { XIAO_CLANG: clang }),
      })).rejects.toMatchObject({ code: "X11-CLI-TOOLCHAIN-LINK-001" });
    });
  }, 15_000);

  test("通过编译/链接探测后返回工具链描述", async () => {
    await withDirectory(async (directory) => {
      const clang = await fakeTool(directory, "clang", "22.0.0");
      const result = await discoverToolchainWithMetadata({
        cwd: directory,
        env: environment(directory, { XIAO_CLANG: clang }),
      });
      expect(result.toolchain.clang).toBe(clang);
      expect(result.selected.clang?.source).toBe("override");
    });
  }, 15_000);

  test("版本主版本解析保持稳定", () => {
    expect(parseToolchainVersion("clang version 18.1.8")).toBe(18);
    expect(parseToolchainVersion("LLVM 19.0.0")).toBe(19);
    expect(parseToolchainVersion("unknown")).toBeNull();
  });

  test.skipIf(process.env.XIAO_TOOLCHAIN_SMOKE !== "1")("环境门控：使用真实 clang 完成目标探测", async () => {
    const result = await discoverToolchainWithMetadata({ requireDiagnostics: false });
    expect(result.toolchain.clang.length).toBeGreaterThan(0);
  });
});
