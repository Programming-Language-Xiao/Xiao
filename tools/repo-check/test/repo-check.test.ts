/** A0 仓库完整性检查器的单元与集成测试。 */

import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { checkLayout } from "../src/layout.ts";
import { parseArguments, runCommand } from "../src/cli.ts";
import { checkMarkdownLinks, scanMarkdownDirectory } from "../src/docs.ts";
import { findRepositoryRoot } from "../src/manifest.ts";
import { inspectBunWorkspace } from "../src/workspace.ts";
import { readmeMissingSections } from "../src/paths.ts";
import type { MarkdownPage } from "../src/docs.ts";

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
  test("A0 骨架通过 workspace 和 README 检查", () => {
    const result = checkLayout(process.cwd());
    expect(result.passed).toBe(true);
    expect(result.diagnostics).toEqual([]);
  }, 60_000);

  test("从子目录执行时报告根目录而不是子目录", async () => {
    const expectedRoot = findRepositoryRoot(process.cwd()) ?? process.cwd();
    const result = await runCommand({ command: "layout", root: join(expectedRoot, "tools"), format: "text" });
    expect(result.root).toBe(expectedRoot);
    expect(result.result.passed).toBe(true);
  }, 60_000);
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
  test("源目录缺少 README 时报告 A0-LAYOUT-001", () => {
    const directory = createLayoutFixture(false);
    try {
      const result = checkLayout(directory);
      expect(result.passed).toBe(false);
      expect(result.diagnostics.some((item) => item.code === "A0-LAYOUT-001" && item.message.includes("README"))).toBe(true);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  }, 60_000);
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
