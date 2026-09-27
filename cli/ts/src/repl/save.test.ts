/** 保存输入不触碰源码状态，渲染遵守既有终端配色与宽度。 */

import { expect, test } from "bun:test";

import type { TerminalView } from "../ui/terminal.ts";
import { stripAnsi } from "../ui/color.ts";
import { displayWidth } from "../ui/width.ts";
import { applySaveKey, initialSaveInput, renderSaveInput } from "./save.ts";
import vectors from "../../../../tests/spec/11b-repl/save.json";

const view: TerminalView = {
  width: 20, height: 10, isTTY: true, kittyKeys: false,
  color: { isTTY: true, mode: "never", noColor: true, term: "dumb", colorTerm: "" },
};

test("保存路径支持字素编辑、粘贴与 q/ESC 取消", () => {
  let state = initialSaveInput();
  state = applySaveKey(state, { kind: "paste", text: "目录/源码.xiao" }).state;
  expect(state.text).toBe("目录/源码.xiao");
  state = applySaveKey(state, { kind: "left" }).state;
  state = applySaveKey(state, { kind: "backspace" }).state;
  expect(state.text).toBe("目录/源码.xio");
  expect(applySaveKey(state, { kind: "escape" }).action).toBe("cancel");
  const cancel = applySaveKey(initialSaveInput(), { kind: "text", text: "q" }).state;
  expect(applySaveKey(cancel, { kind: "enter" }).action).toBe("cancel");
  expect(applySaveKey(state, { kind: "enter" }).action).toBe("submit");
  expect(applySaveKey(state, { kind: "paste", text: "\n" }).state.error).toContain("X11-CLI-SAVE-001");
  expect(applySaveKey(state, { kind: "paste", text: "\u2028" }).state.text).toBe(state.text);
});

test("保存标题、独立提示符和水平滚动路径适配窄终端", () => {
  const state = { text: "very/long/path/main.xiao", cursor: 24, error: null };
  const frame = renderSaveInput(state, view);
  const lines = frame.text.split("\r\n");
  expect(lines[0]).toContain("Enter the save");
  expect(lines[1].startsWith("> ")).toBe(true);
  expect(lines[1]).toContain("xiao");
  expect(displayWidth(lines[0])).toBe(20);
  expect(displayWidth(lines[1])).toBeLessThanOrEqual(20);
  expect(displayWidth(lines[2])).toBe(20);
  expect(frame.cursorColumn).toBeLessThanOrEqual(20);
});

for (const vector of vectors.cases) {
  test(`${vector.width} 列保存态共享规格与实际渲染一致`, () => {
    expect(renderSaveInput(initialSaveInput(), { ...view, width: vector.width }).text).toBe(vector.expected);
  });
}

test("保存提示符复用确认态配色，错误不超过终端高度", () => {
  const colored: TerminalView = {
    ...view, width: 40,
    color: { isTTY: true, mode: "always", noColor: false, term: "xterm", colorTerm: "truecolor" },
  };
  const frame = renderSaveInput(initialSaveInput(), colored).text;
  expect(frame).toContain("\u001b[38;2;127;127;127m");
  expect(frame).toContain("\u001b[38;2;220;220;173m>");
  expect(stripAnsi(frame)).toBe(renderSaveInput(initialSaveInput(), { ...view, width: 40 }).text);
  const failed = renderSaveInput({ text: "main.xiao", cursor: 9, error: "path: very long failure message" }, { ...view, height: 4 });
  expect(failed.text.split("\r\n")).toHaveLength(4);
  for (const height of [1, 2, 3]) {
    const compact = renderSaveInput({ text: "main.xiao", cursor: 9, error: "path: very long failure message" }, { ...view, height });
    expect(compact.text.split("\r\n")).toHaveLength(height);
    expect(compact.cursorRow).toBe(height === 2 ? 1 : Math.min(height, 2));
  }
});

test("大段路径粘贴不会因参数展开溢出，输入窗口保持有界", () => {
  const state = applySaveKey(initialSaveInput(), { kind: "paste", text: "x".repeat(750_000) }).state;
  expect(state.text.length).toBe(750_000);
  const frame = renderSaveInput(state, { ...view, width: 20 });
  expect(frame.text.split("\r\n")[1]).toBe("> " + "x".repeat(17));
  expect(frame.cursorColumn).toBe(20);
});

test("保存路径的组合字符插入后可按一个字素退格", () => {
  let state = applySaveKey(initialSaveInput(), { kind: "text", text: "a" }).state;
  state = applySaveKey(state, { kind: "text", text: "\u0301" }).state;
  expect(state).toEqual({ text: "a\u0301", cursor: 1, error: null });
  expect(applySaveKey(state, { kind: "backspace" }).state).toEqual(initialSaveInput());
});
