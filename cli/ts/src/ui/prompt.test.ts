/** I0 提示符跨平台规格与显式注入的颜色能力测试。 */

import { expect, test } from "bun:test";

import vectors from "../../../../tests/spec/11b-repl/prompt.json";
import { stripAnsi } from "./color.ts";
import { renderReplBanner, renderReplPrompt } from "./prompt.ts";

test("版权与版本固定为两行", () => {
  expect(renderReplBanner("0.1.0")).toBe("Xiao (c) XiaoCZX\nV0.1.0\n");
});

for (const vector of vectors.cases) {
  test(`${vector.platform} 路径、环境名与 [X> 排列一致`, () => {
    expect(renderReplPrompt({ cwd: vector.cwd, activeEnvironment: vector.activeEnvironment, color: { isTTY: false, colorTerm: "truecolor", noColor: false, term: "xterm" } })).toBe(vector.expected);
  });
}

test("无上游只显示分支，有上游才显示领先落后", () => {
  const base = { cwd: "/project", color: { isTTY: false, noColor: false, colorTerm: "", term: "xterm" } };
  expect(renderReplPrompt({ ...base, git: { branch: "dev", ahead: null, behind: null } })).toBe("/project dev [X> ");
  expect(renderReplPrompt({ ...base, git: { branch: "main", ahead: 0, behind: 2 } })).toBe("/project main-0↑-2↓ [X> ");
});

test("非 TTY、NO_COLOR、dumb 与 never 必须完全不输出 ANSI", () => {
  const basic = { cwd: "/project", activeEnvironment: "/project/.venv", color: { isTTY: true, colorTerm: "truecolor", noColor: false, term: "xterm", mode: "auto" as const } };
  for (const color of [
    { ...basic.color, isTTY: false },
    { ...basic.color, noColor: true },
    { ...basic.color, term: "dumb" },
    { ...basic.color, mode: "never" as const },
  ]) {
    const result = renderReplPrompt({ ...basic, color });
    expect(result).toBe("$venv$ /project [X> ");
    expect(result).not.toContain("\u001b");
  }
});

test("显式 always 在非 TTY 下仍保留提示符颜色", () => {
  const result = renderReplPrompt({
    cwd: "/project",
    activeEnvironment: "/project/.venv",
    color: { isTTY: false, mode: "always", colorTerm: "truecolor", term: "xterm" },
  });
  expect(result).toContain("\u001b[38;2;");
  expect(stripAnsi(result)).toBe("$venv$ /project [X> ");
});

test("真彩色与 256/16 色回退保持文本相同，分支颜色按规范区分", () => {
  const base = { cwd: "/project", activeEnvironment: "/project/.venv", git: { branch: "master", ahead: 2, behind: 1 } };
  const color = { isTTY: true, noColor: false };
  const trueColor = renderReplPrompt({ ...base, color: { ...color, colorTerm: "truecolor", term: "xterm" } });
  const color256 = renderReplPrompt({ ...base, color: { ...color, colorTerm: "", term: "xterm-256color" } });
  const color16 = renderReplPrompt({ ...base, color: { ...color, colorTerm: "", term: "xterm" } });
  expect(trueColor).toContain("\u001b[38;2;135;230;140m$venv$");
  expect(trueColor).toContain("\u001b[38;2;251;167;122mmaster");
  expect(trueColor).toContain("\u001b[38;2;246;226;183m[X>");
  expect(color256).toContain("\u001b[38;5;216mmaster");
  expect(color16).toContain("\u001b[33mmaster");
  for (const output of [trueColor, color256, color16]) {
    expect(stripAnsi(output)).toBe("$venv$ /project master-2↑-1↓ [X> ");
  }
});
