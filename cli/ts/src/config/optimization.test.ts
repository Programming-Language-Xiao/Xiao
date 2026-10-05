/** 18A 优化参数、配置优先级和纯归一化回归。 */

import { describe, expect, test } from "bun:test";
import { mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { normalizeOptimization, parseOptimizationLayer, parseOptimizationLevel, resolveOptimization } from "./optimization.ts";

describe("18A 优化归一化", () => {
  test("只接受 O0 到 O3", () => {
    expect(parseOptimizationLevel("-O0")).toBe(0);
    expect(parseOptimizationLevel("-O3")).toBe(3);
    expect(() => parseOptimizationLevel("-Ox")).toThrow("X11-CLI-OPT-001");
    expect(() => parseOptimizationLevel("-O4")).toThrow("X11-CLI-OPT-001");
  });

  test("命令行 > 项目 > 全局 > O0，且列表稳定排序去重", () => {
    const result = normalizeOptimization(
      { level: 1, passSet: ["z", "a"] },
      { level: 2, passSet: ["b", "a"] },
      { level: 3 },
    );
    expect(result.config.level).toBe(3);
    expect(result.passSet).toEqual(["a", "b"]);
    expect(normalizeOptimization().config.level).toBe(0);
  });

  test("配置解析是静态的，并保留 13A 字段名", () => {
    const layer = parseOptimizationLayer(
      "[Optimization]\nlevel = 2\npass_set = [\"fold\", \"fold\"]\ndebug_info = true\nsource_map = true\n",
    );
    expect(layer).toMatchObject({ level: 2, debugInfo: true, sourceMap: true });
    expect(layer.passSet).toEqual(["fold", "fold"]);
  });

  test("归一化 O2 不写回项目配置或锁文件", async () => {
    const directory = await mkdtemp(join(tmpdir(), "xiao-optimization-side-effect-"));
    const configPath = join(directory, "config.xiao");
    const lockPath = join(directory, "xiao.lock");
    const config = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n[Optimization]\nlevel = 1\n";
    const lock = "lock-content\n";
    await writeFile(configPath, config, "utf8");
    await writeFile(lockPath, lock, "utf8");
    try {
      const beforeConfig = await stat(configPath);
      const beforeLock = await stat(lockPath);
      const normalized = await resolveOptimization({ cwd: directory, cli: { level: 2 } });
      expect(normalized.level).toBe(2);
      expect(await readFile(configPath, "utf8")).toBe(config);
      expect(await readFile(lockPath, "utf8")).toBe(lock);
      expect((await stat(configPath)).mtimeMs).toBe(beforeConfig.mtimeMs);
      expect((await stat(lockPath)).mtimeMs).toBe(beforeLock.mtimeMs);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
});
