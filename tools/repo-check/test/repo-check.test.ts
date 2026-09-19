/** A0 仓库完整性检查器的单元与集成测试。 */

import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { tmpdir } from "node:os";

import { checkLayout } from "../src/layout.ts";
import { parseArguments, runCommand } from "../src/cli.ts";
import { checkMarkdownLinks, scanMarkdownDirectory } from "../src/docs.ts";
import { findRepositoryRoot } from "../src/manifest.ts";
import { renderJson, renderSarif, renderText } from "../src/report.ts";
import { checkFileSizes, countPhysicalLines, exemptionMissingSections, MAX_SOURCE_LINES, renderOutlineDetails } from "../src/size.ts";
import { inspectBunWorkspace } from "../src/workspace.ts";
import { readmeMissingSections } from "../src/paths.ts";
import type { MarkdownPage } from "../src/docs.ts";
import type { CheckResult } from "../src/types.ts";

describe("repo-check 参数", () => {
  test("默认执行 all 并支持 JSON 输出", () => {
    expect(parseArguments(["--format", "json"])).toEqual({ command: "all", root: undefined, format: "json", output: undefined });
  });
});

describe("README 规则", () => {
  test("完整 README 通过，缺字段会被指出", () => {
    expect(readmeMissingSections("# 目录\n\n## 工程期\n\nA0。\n\n## 职责\n\n负责检查。")) .toEqual([]);
    expect(readmeMissingSections("# 只有标题")).toEqual(["工程期", "职责或边界"]);
  });
});

describe("当前仓库布局", () => {
  test("A0 布局其它规则保持通过，尺寸债务名单精确可见", async () => {
    const result = await checkLayout(process.cwd());
    const knownOversized = [
      "core/rust/crates/xiao-bytecode/src/research/encode.rs",
      "core/rust/crates/xiao-syntax/src/parser.rs",
    ];
    const sizeErrors = result.diagnostics.filter((item) => item.code === "A0-SIZE-001");
    const others = result.diagnostics.filter((item) => item.code !== "A0-SIZE-001");
    expect(others).toEqual([]);
    // 这份名单只能随着拆分缩短，不能用“包含”断言掩盖新增超长文件。
    expect(sizeErrors.map((item) => item.path)).toEqual(knownOversized);
    expect(result.passed).toBe(false);
  }, 120_000);

  test("从子目录执行时报告根目录而不是子目录", async () => {
    const expectedRoot = findRepositoryRoot(process.cwd()) ?? process.cwd();
    const result = await runCommand({ command: "layout", root: join(expectedRoot, "tools"), format: "text" });
    expect(result.root).toBe(expectedRoot);
    // 当前两个已知超长文件尚未拆分，子目录入口也必须报告尺寸债务。
    expect(result.result.passed).toBe(false);
  }, 120_000);
});

describe("workspace 漂移诊断", () => {
  test("Bun 实际成员多于政策清单时失败", () => {
    const directory = mkdtempSync(join(tmpdir(), "xiao-repo-check-workspace-"));
    try {
      mkdirSync(join(directory, "declared"), { recursive: true });
      mkdirSync(join(directory, "extra"), { recursive: true });
      writeFileSync(join(directory, "package.json"), JSON.stringify({ workspaces: ["declared", "extra"] }), "utf8");
      writeFileSync(join(directory, "declared", "package.json"), JSON.stringify({ name: "@fixture/declared" }), "utf8");
      writeFileSync(join(directory, "extra", "package.json"), JSON.stringify({ name: "@fixture/extra" }), "utf8");
      const result = inspectBunWorkspace(directory, { manifest: "package.json", members: ["declared"] });
      expect(result.diagnostics.some((item) => item.code === "A0-WORKSPACE-001" && item.message.includes("未登记成员"))).toBe(true);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});

describe("目录完整性负例", () => {
  test("源目录缺少 README 时报告 A0-LAYOUT-001", async () => {
    const directory = createLayoutFixture(false);
    try {
      const result = await checkLayout(directory);
      expect(result.passed).toBe(false);
      expect(result.diagnostics.some((item) => item.code === "A0-LAYOUT-001" && item.message.includes("README"))).toBe(true);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  }, 60_000);
});

describe("单文件行数门禁", () => {
  test("统一计数器处理换行边界且恰好上限不报", async () => {
    expect(countPhysicalLines("a\r\nb\r\nc\n")).toBe(3);
    expect(countPhysicalLines("a\rb\u2028c\u2029")).toBe(3);
    expect(countPhysicalLines("")).toBe(0);
    const directory = mkdtempSync(join(tmpdir(), "xiao-repo-check-size-boundary-"));
    try {
      const file = "fixture.ts";
      writeFileSync(join(directory, file), `${"x\n".repeat(MAX_SOURCE_LINES)}`, "utf8");
      const result = await checkFileSizes(directory, {} as never, [file], { outlineProvider: () => { throw new Error("must not be called"); } });
      expect(result).toEqual({ passed: true, diagnostics: [] });
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  test("2501 行报告 error 且只在超标后请求大纲", async () => {
    const directory = mkdtempSync(join(tmpdir(), "xiao-repo-check-size-error-"));
    try {
      const file = "fixture.ts";
      writeFileSync(join(directory, file), `${"x\n".repeat(MAX_SOURCE_LINES + 1)}`, "utf8");
      let calls = 0;
      const result = await checkFileSizes(directory, {} as never, [file], {
        outlineProvider: () => {
          calls += 1;
          return { nodes: new Map(), failures: new Map() };
        },
      });
      expect(calls).toBe(1);
      expect(result.passed).toBe(false);
      expect(result.diagnostics).toHaveLength(1);
      expect(result.diagnostics[0]).toMatchObject({ code: "A0-SIZE-001", severity: "error", subject: "2501 行 / 上限 2500" });
      expect(result.diagnostics[0]?.line).toBeUndefined();
      expect(result.diagnostics[0]?.subject).not.toBe(result.diagnostics[0]?.path);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  test("完整旁置说明降级为 warning，仍保留债务诊断", async () => {
    const directory = mkdtempSync(join(tmpdir(), "xiao-repo-check-size-exempt-"));
    try {
      const file = "fixture.ts";
      writeFileSync(join(directory, file), `${"x\n".repeat(MAX_SOURCE_LINES + 1)}`, "utf8");
      writeFileSync(join(directory, `${file}的硬耦合需要的说明.md`), "# 边界\n不能拆分。\n\n# 理由\n会破坏协议。\n\n# 替代方案\n考虑过拆分。\n\n# 移除计划\n下一阶段移除。\n", "utf8");
      const result = await checkFileSizes(directory, {} as never, [file]);
      expect(result).toMatchObject({ passed: true, diagnostics: [{ code: "A0-SIZE-001", severity: "warning" }] });
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  test("缺段和空白豁免不会降低 error", async () => {
    const directory = mkdtempSync(join(tmpdir(), "xiao-repo-check-size-bad-exempt-"));
    try {
      const file = "fixture.ts";
      writeFileSync(join(directory, file), `${"x\n".repeat(MAX_SOURCE_LINES + 1)}`, "utf8");
      writeFileSync(join(directory, `${file}的硬耦合需要的说明.md`), "# 边界\n\n# 理由\n只有理由。\n", "utf8");
      const result = await checkFileSizes(directory, {} as never, [file]);
      expect(result.passed).toBe(false);
      expect(result.diagnostics.some((item) => item.message.includes("替代方案") && item.message.includes("移除计划"))).toBe(true);
      writeFileSync(join(directory, `${file}的硬耦合需要的说明.md`), "", "utf8");
      const empty = await checkFileSizes(directory, {} as never, [file]);
      expect(empty.passed).toBe(false);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  test("大纲详情按稳定大小截断并保留顶层节点", () => {
    const nodes = Array.from({ length: 3 }, (_, index) => ({
      kind: "function",
      name: `top${index}`,
      line: index + 1,
      end_line: index + 100,
      lines: 100 - index,
      signature: "fn()",
      source_line: "fn top() {}",
      children: Array.from({ length: 4 }, (_, childIndex) => ({
        kind: "method",
        name: `child${index}-${childIndex}`,
        line: childIndex + 2,
        end_line: childIndex + 3,
        lines: childIndex + 1,
        signature: "fn()",
        source_line: "fn child() {}",
        children: [],
      })),
    }));
    const details = renderOutlineDetails(nodes, 5);
    expect(details.filter((line) => line.startsWith("function top")).length).toBe(3);
    expect(details.filter((line) => !line.startsWith("……")).length).toBe(5);
    expect(details.at(-1)).toContain("省略");
  });

  test("大纲不可用时仍保留尺寸 error 与可复现诊断", async () => {
    const directory = mkdtempSync(join(tmpdir(), "xiao-repo-check-size-outline-error-"));
    try {
      const file = "fixture.ts";
      writeFileSync(join(directory, file), `${"x\n".repeat(MAX_SOURCE_LINES + 1)}`, "utf8");
      const result = await checkFileSizes(directory, {} as never, [file], { outlineProvider: () => { throw new Error("adapter unavailable"); } });
      expect(result.passed).toBe(false);
      expect(result.diagnostics).toEqual(expect.arrayContaining([
        expect.objectContaining({ code: "A0-PARSER-001", hint: expect.stringContaining("\"outline\":true") }),
        expect.objectContaining({ code: "A0-SIZE-001", severity: "error" }),
      ]));
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  test("超标诊断稳定排序，且多个超标文件共用一次大纲请求", async () => {
    const directory = mkdtempSync(join(tmpdir(), "xiao-repo-check-size-order-"));
    try {
      for (const file of ["a.ts", "b.ts"]) writeFileSync(join(directory, file), `${"x\n".repeat(MAX_SOURCE_LINES + 1)}`, "utf8");
      const batches: string[][] = [];
      const result = await checkFileSizes(directory, {} as never, ["b.ts", "a.ts"], {
        outlineProvider: (files) => {
          batches.push(files);
          return { nodes: new Map(), failures: new Map() };
        },
      });
      expect(result.diagnostics.map((item) => item.path)).toEqual(["a.ts", "b.ts"]);
      // Rust 适配器每次调用都要启动 cargo；逐文件请求会把启动开销乘以文件数。
      expect(batches).toHaveLength(1);
      expect(batches[0]?.map((item) => basename(item)).sort()).toEqual(["a.ts", "b.ts"]);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  test("豁免章节缺失识别稳定", () => {
    expect(exemptionMissingSections("# 边界\n内容\n# 理由\n内容\n# 替代方案\n内容\n# 移除计划\n内容\n")).toEqual([]);
    expect(exemptionMissingSections("# 边界\n内容\n# 理由\n")).toEqual(["理由", "替代方案", "移除计划"]);
  });
});

describe("报告详情渲染", () => {
  test("文本、JSON 与 SARIF 分别保留结构详情", () => {
    const result: CheckResult = {
      passed: false,
      diagnostics: [{
        code: "A0-SIZE-001",
        severity: "error",
        path: "src/long.ts",
        subject: "2501 行 / 上限 2500",
        message: "单文件共 2501 行，超过上限 2500 行。",
        details: ["class Reader [第 1-2501 行，共 2501 行] export class"],
      }],
    };
    expect(renderText(result)).toContain("  class Reader");
    const json = renderJson(result) as { diagnostics: Array<{ details?: string[] }> };
    expect(json.diagnostics[0]?.details).toEqual(result.diagnostics[0]?.details);
    const sarif = renderSarif(result) as { runs: Array<{ results: Array<{ message: { text: string }; properties?: { details?: string[] } }> }> };
    const entry = sarif.runs[0]?.results[0];
    expect(entry?.message.text).not.toContain("\n");
    expect(entry?.properties?.details).toEqual(result.diagnostics[0]?.details);
  });
});

describe("Markdown 链接负例", () => {
  test("不存在的相对链接被拒绝", () => {
    const directory = mkdtempSync(join(tmpdir(), "xiao-repo-check-links-"));
    try {
      const pages: MarkdownPage[] = [{
        path: "docs/DevDocs/index.md",
        frontMatter: {},
        headings: new Set(["index"]),
        links: ["missing.md"],
        related: [],
      }];
      const diagnostics = checkMarkdownLinks(directory, pages);
      expect(diagnostics.some((item) => item.code === "A0-DOCS-001" && item.message.includes("链接目标不存在"))).toBe(true);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});

describe("front matter related 校验", () => {
  test("related 指向不存在的路径时报告断链", () => {
    const directory = createMarkdownFixture({
      "a.md": "---\nid: a\nrelated:\n  - missing.md\n---\n\n# A\n",
    });
    try {
      const diagnostics = checkMarkdownLinks(directory, scanMarkdownDirectory(directory, "docs/DevDocs").pages);
      expect(diagnostics.some((item) => item.code === "A0-DOCS-001" && item.message.includes("链接目标不存在"))).toBe(true);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  test("related 指向存在的路径时不产生诊断", () => {
    const directory = createMarkdownFixture({
      "README.md": "---\nid: a\nrelated:\n  - b.md\n---\n\n# A\n\n[跳转](b.md)\n",
      "b.md": "# B\n",
    });
    try {
      const diagnostics = checkMarkdownLinks(directory, scanMarkdownDirectory(directory, "docs/DevDocs").pages);
      expect(diagnostics).toEqual([]);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  test("related 不计入入链，被它引用的页面仍是孤立页", () => {
    const directory = createMarkdownFixture({
      "README.md": "---\nid: a\nrelated:\n  - b.md\n---\n\n# A\n",
      "b.md": "# B\n",
    });
    try {
      const diagnostics = checkMarkdownLinks(directory, scanMarkdownDirectory(directory, "docs/DevDocs").pages);
      expect(diagnostics.some((item) => item.code === "A0-DOCS-001" && item.message_id === "a0.docs.orphan_page")).toBe(true);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});

/** 创建只含指定 Markdown 页面的最小文档目录。 */
function createMarkdownFixture(files: Record<string, string>): string {
  const directory = mkdtempSync(join(tmpdir(), "xiao-repo-check-related-"));
  const docs = join(directory, "docs", "DevDocs");
  mkdirSync(docs, { recursive: true });
  for (const [name, content] of Object.entries(files)) writeFileSync(join(docs, name), content, "utf8");
  return directory;
}

/** 创建仅用于目录检查负例的最小 Cargo/Bun 仓库。 */
function createLayoutFixture(withSourceReadme: boolean): string {
  const directory = mkdtempSync(join(tmpdir(), "xiao-repo-check-layout-"));
  mkdirSync(join(directory, ".git"), { recursive: true });
  mkdirSync(join(directory, "tools", "repo-check"), { recursive: true });
  mkdirSync(join(directory, "core", "rust", "crates", "fixture", "src"), { recursive: true });
  mkdirSync(join(directory, "pkg", "src"), { recursive: true });
  writeFileSync(join(directory, "core", "rust", "Cargo.toml"), [
    "[workspace]",
    "members = [\"crates/fixture\"]",
    "resolver = \"2\"",
    "",
  ].join("\n"), "utf8");
  writeFileSync(join(directory, "core", "rust", "crates", "fixture", "Cargo.toml"), [
    "[package]",
    "name = \"fixture\"",
    "version = \"0.1.0\"",
    "edition = \"2021\"",
    "",
  ].join("\n"), "utf8");
  writeFileSync(join(directory, "core", "rust", "crates", "fixture", "src", "lib.rs"), "//! fixture\n", "utf8");
  writeFileSync(join(directory, "pkg", "package.json"), JSON.stringify({ name: "@fixture/pkg" }), "utf8");
  writeFileSync(join(directory, "pkg", "src", "index.ts"), "/** fixture */\n", "utf8");
  writeFileSync(join(directory, "package.json"), JSON.stringify({ workspaces: ["pkg"] }), "utf8");
  const manifest = {
    schemaVersion: 1,
    rust: { manifest: "core/rust/Cargo.toml", members: ["core/rust/crates/fixture"] },
    typescript: { manifest: "package.json", members: ["pkg"] },
    codeRoots: ["core/rust", "pkg"],
    sourceExtensions: [".rs", ".ts"],
    excludedDirectories: ["target", "node_modules"],
    readmeFile: "README.md",
    moduleRegistry: "docs/module-registry.json",
  };
  writeFileSync(join(directory, "tools", "repo-check", "repository.manifest.json"), JSON.stringify(manifest), "utf8");
  mkdirSync(join(directory, "docs"), { recursive: true });
  writeFileSync(join(directory, "docs", "module-registry.json"), JSON.stringify({ schemaVersion: 1, modules: [] }), "utf8");
  writeFileSync(join(directory, "core", "rust", "README.md"), "# Rust\n\n工程期 A0。\n\n职责：fixture。\n", "utf8");
  writeFileSync(join(directory, "core", "rust", "crates", "README.md"), "# Crates\n\n工程期 A0。\n\n职责：fixture。\n", "utf8");
  writeFileSync(join(directory, "core", "rust", "crates", "fixture", "README.md"), "# Fixture\n\n工程期 A0。\n\n职责：fixture。\n", "utf8");
  if (withSourceReadme) writeFileSync(join(directory, "core", "rust", "crates", "fixture", "src", "README.md"), "# Source\n\n工程期 A0。\n\n职责：fixture。\n", "utf8");
  writeFileSync(join(directory, "pkg", "README.md"), "# Package\n\n工程期 A0。\n\n职责：fixture。\n", "utf8");
  writeFileSync(join(directory, "pkg", "src", "README.md"), "# Source\n\n工程期 A0。\n\n职责：fixture。\n", "utf8");
  return directory;
}
