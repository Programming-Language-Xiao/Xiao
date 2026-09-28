/** 确认态与收尾分隔线共享宽度、颜色和独立提示符规格。 */

import { expect, test } from "bun:test";

import { stripAnsi } from "../ui/color.ts";
import { displayWidth } from "../ui/width.ts";
import type { TerminalView } from "../ui/terminal.ts";
import { renderConfirmation, separator } from "./confirm.ts";
import vectors from "../../../../tests/spec/11b-repl/confirm.json";

/** 显式注入终端能力，避免宿主颜色环境影响测试。 */
function view(width: number, color = false): TerminalView {
  return {
    width, height: 24, isTTY: color, kittyKeys: false,
    color: { isTTY: color, noColor: false, term: "xterm", colorTerm: "truecolor", mode: "auto" },
  };
}

test("确认标题嵌入分隔线，独立提示符和三行随宽度调整", () => {
  for (const width of [8, 24, 80]) {
    const lines = renderConfirmation(view(width)).split("\r\n");
    expect(lines.length).toBe(3);
    expect(lines[0]).toStartWith("─ ");
    expect(lines[1]).toBe(">");
    expect(lines[2]).toBe("─".repeat(width));
    expect(displayWidth(lines[0])).toBe(width);
  }
});

for (const vector of vectors.cases) {
  test(`${vector.width} 列确认态共享规格与实际渲染一致`, () => {
    expect(renderConfirmation(view(vector.width))).toBe(vector.expected);
  });
}

test("分隔线使用灰色，独立提示符使用 #DCDCAD，去色后布局不变", () => {
  const frame = renderConfirmation(view(80, true));
  expect(frame).toContain("\u001b[38;2;127;127;127m");
  expect(frame).toContain("\u001b[38;2;220;220;173m>");
  expect(stripAnsi(frame)).toBe(renderConfirmation(view(80)));
  expect(stripAnsi(separator(view(80, true)))).toBe("─".repeat(80));
});

test("中文确认标题不改变三行宽度", () => {
  const frame = renderConfirmation(view(40), "zh-CN").split("\r\n");
  expect(frame[0]).toContain("按 Enter 确认并运行");
  expect(frame.map(displayWidth)).toEqual([40, 1, 40]);
});
