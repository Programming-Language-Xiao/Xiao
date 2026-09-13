/** A0 文档覆盖率统计和 TypeScript AST 适配器测试。 */

import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { checkCoverage, calculateSummary } from "../src/checker.ts";
import { parseCoverageArguments } from "../src/cli.ts";
import { scanTypeScriptFile } from "../src/typescript-adapter.ts";
import { RUST_ADAPTER_PROTOCOL_VERSION, scanRustFiles, validateRustAdapterResponse } from "../src/rust-adapter.ts";

describe("覆盖率摘要", () => {
  test("空成员和公共门槛按百分比计算", () => {
    const summary = calculateSummary([
      { language: "typescript", file: "a.ts", line: 1, kind: "function", name: "a", isPublic: true, hasDoc: true, parser: "test" },
      { language: "typescript", file: "a.ts", line: 2, kind: "function", name: "b", isPublic: false, hasDoc: false, parser: "test" },
    ], ["a"]);
    expect(summary.total).toBe(2);
    expect(summary.documented).toBe(1);
    expect(summary.publicPercentage).toBe(100);
    expect(summary.percentage).toBe(50);
  });
});

describe("TypeScript AST", () => {
  test("识别导出项、私有项和 JSDoc", () => {
    const directory = mkdtempSync(join(tmpdir(), "xiao-doc-coverage-"));
    const file = join(directory, "index.ts");
    writeFileSync(file, "/** 模块说明 */\n/** 公共函数 */\nexport function visible() {}\nfunction hidden() {}\n", "utf8");
    const records = scanTypeScriptFile(directory, file);
    expect(records.some((item) => item.name === "visible" && item.isPublic && item.hasDoc)).toBe(true);
    expect(records.some((item) => item.name === "hidden" && !item.isPublic && !item.hasDoc)).toBe(true);
    rmSync(directory, { recursive: true, force: true });
  });

  test("语法解析失败时抛出诊断，而不是返回空声明", () => {
    const directory = mkdtempSync(join(tmpdir(), "xiao-doc-coverage-invalid-ts-"));
    const file = join(directory, "broken.ts");
    writeFileSync(file, "export function broken( {\n", "utf8");
    try {
      expect(() => scanTypeScriptFile(directory, file)).toThrow(/TS\d+/u);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});

describe("覆盖率负例", () => {
  test("未注释的导出函数触发公共 API 门禁", async () => {
    const directory = createCoverageFixture("export function missingDoc() {}\n");
    try {
      const result = await checkCoverage({ root: directory, totalThreshold: 0, publicThreshold: 100 });
      expect(result.passed).toBe(false);
      expect(result.diagnostics.some((item) => item.code === "A0-COVERAGE-002" && item.subject.includes("missingDoc"))).toBe(true);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});

describe("Rust AST 协议", () => {
  test("拒绝不兼容的响应版本", () => {
    const diagnostic = validateRustAdapterResponse({ protocol_version: 2, declarations: [], errors: [] });
    expect(diagnostic?.code).toBe("A0-PROTOCOL-001");
  });

  test("调用真实适配器时返回并校验协议版本", () => {
    const directory = mkdtempSync(join(tmpdir(), "xiao-doc-coverage-rust-ts-"));
    const file = join(directory, "fixture.rs");
    writeFileSync(file, "//! fixture\n/// visible\npub fn visible() {}\n", "utf8");
    try {
      const result = scanRustFiles({ root: process.cwd(), files: [file] });
      expect(result.diagnostics).toEqual([]);
      expect(result.declarations.some((item) => item.name === "visible" && item.hasDoc)).toBe(true);
      expect(RUST_ADAPTER_PROTOCOL_VERSION).toBe(1);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  }, 60_000);
});

describe("覆盖率参数", () => {
  test("解析阈值和输出格式", () => {
    expect(parseCoverageArguments(["--format", "json", "--total-threshold", "95", "--public-threshold", "100"])).toMatchObject({ format: "json", totalThreshold: 95, publicThreshold: 100 });
  });
});

/** 创建只包含 TypeScript 源文件的覆盖率检查 fixture。 */
function createCoverageFixture(source: string): string {
  const directory = mkdtempSync(join(tmpdir(), "xiao-doc-coverage-fixture-"));
  mkdirSync(join(directory, ".git"), { recursive: true });
  mkdirSync(join(directory, "tools", "repo-check"), { recursive: true });
  mkdirSync(join(directory, "src"), { recursive: true });
  writeFileSync(join(directory, "src", "module.ts"), source, "utf8");
  const manifest = {
    schemaVersion: 1,
    rust: { manifest: "core/rust/Cargo.toml", members: ["core/rust/crates/fixture"] },
    typescript: { manifest: "package.json", members: ["src"] },
    codeRoots: ["src"],
    sourceExtensions: [".ts"],
    excludedDirectories: ["node_modules", "target"],
    readmeFile: "README.md",
    moduleRegistry: "docs/module-registry.json",
  };
  writeFileSync(join(directory, "tools", "repo-check", "repository.manifest.json"), JSON.stringify(manifest), "utf8");
  mkdirSync(join(directory, "docs"), { recursive: true });
  writeFileSync(join(directory, "docs", "module-registry.json"), JSON.stringify({ schemaVersion: 1, modules: [] }), "utf8");
  writeFileSync(join(directory, "package.json"), JSON.stringify({ workspaces: ["src"] }), "utf8");
  return directory;
}
