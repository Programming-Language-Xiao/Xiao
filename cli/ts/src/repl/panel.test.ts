/** 共用空面板的输入、关闭、颜色与终端宽度回归。 */

import { expect, test } from "bun:test";

import { stripAnsi } from "../ui/color.ts";
import type { TerminalView } from "../ui/terminal.ts";
import { displayWidth } from "../ui/width.ts";
import { applyPanelKey, initialPanelState, renderPanel } from "./panel.ts";
import vectors from "../../../../tests/spec/11b-repl/panel.json";

/** 明确注入颜色与尺寸，避免宿主终端影响测试。 */
function view(width: number, height = 24, color = false): TerminalView {
  return {
    width, height, isTTY: color, kittyKeys: false,
    color: { isTTY: color, mode: "auto", noColor: false, term: "xterm", colorTerm: "truecolor" },
  };
}

test("面板键入仅改变临时状态，Enter 无动作，Esc 与快捷键关闭", () => {
  const empty = initialPanelState();
  let state = applyPanelKey(empty, { kind: "paste", text: "命令" }).state;
  expect(state.text).toBe("命令");
  state = applyPanelKey(state, { kind: "left" }).state;
  state = applyPanelKey(state, { kind: "backspace" }).state;
  expect(state.text).toBe("令");
  expect(applyPanelKey(state, { kind: "enter" })).toEqual({ state, close: false });
  expect(applyPanelKey(state, { kind: "escape" }).close).toBe(true);
  expect(applyPanelKey(state, { kind: "panel" }).close).toBe(true);
  expect(applyPanelKey(state, { kind: "interrupt" }).close).toBe(true);
  expect(applyPanelKey(state, { kind: "paste", text: "\u001b[2J" }).state).toBe(state);
  expect(initialPanelState()).toEqual(empty);
});

test("面板标题和两条分隔线自适应，并复用确认态配色", () => {
  for (const width of [8, 20, 40]) {
    const lines = renderPanel(initialPanelState(), view(width)).text.split("\r\n");
    expect(lines).toHaveLength(3);
    expect(lines[0]).toStartWith("─ ");
    expect(displayWidth(lines[0])).toBe(width);
    expect(lines[1]).toBe("> ");
    expect(lines[2]).toBe("─".repeat(width));
  }
  const colored = renderPanel(initialPanelState(), view(40, 24, true)).text;
  expect(colored).toContain("\u001b[38;2;127;127;127m");
  expect(colored).toContain("\u001b[38;2;220;220;173m>");
  expect(stripAnsi(colored)).toBe(renderPanel(initialPanelState(), view(40)).text);
});

for (const vector of vectors.cases) {
  test(`${vector.width} 列空面板与共享规格一致`, () => {
    expect(renderPanel(initialPanelState(), view(vector.width)).text).toBe(vector.expected);
  });
}

test("长输入只在面板内水平滚动，极矮终端不写越界行", () => {
  const state = { text: "very/long/panel/search", cursor: 22 };
  for (const height of [1, 2, 3]) {
    const frame = renderPanel(state, view(8, height));
    const lines = frame.text.split("\r\n");
    expect(lines).toHaveLength(height);
    expect(lines.some((line) => displayWidth(line) > 8)).toBe(false);
    expect(frame.cursorRow).toBe(Math.min(height, 2));
    expect(frame.cursorColumn).toBeLessThanOrEqual(8);
  }
});

test("大段粘贴不使用参数展开且只渲染光标附近的输入", () => {
  const state = applyPanelKey(initialPanelState(), { kind: "paste", text: "x".repeat(750_000) }).state;
  expect(state.text.length).toBe(750_000);
  expect(state.cursor).toBe(750_000);
  const frame = renderPanel(state, view(20));
  expect(frame.text.split("\r\n")[1]).toBe("> " + "x".repeat(17));
  expect(frame.cursorColumn).toBe(20);
});

test("中文输入的中间光标保持字素与显示列对齐", () => {
  const frame = renderPanel({ text: "甲乙丙abcd", cursor: 3 }, view(8));
  expect(frame.text.split("\r\n")[1]).toBe("> 乙丙ab");
  expect(frame.cursorColumn).toBe(7);
});

test("组合字符插入后光标仍在字素边界，退格可删除整体", () => {
  let state = applyPanelKey(initialPanelState(), { kind: "text", text: "a" }).state;
  state = applyPanelKey(state, { kind: "text", text: "\u0301" }).state;
  expect(state).toEqual({ text: "a\u0301", cursor: 1 });
  expect(applyPanelKey(state, { kind: "backspace" }).state).toEqual(initialPanelState());
});
