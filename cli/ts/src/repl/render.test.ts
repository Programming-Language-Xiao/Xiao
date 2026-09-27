/** I1a 三平台共享的五字符行号栏、软换行与颜色回退规格。 */

import { expect, test } from "bun:test";

import vectors from "../../../../tests/spec/11b-repl/multiline.json";
import { stripAnsi } from "../ui/color.ts";
import { initialEditorState, selectRange, type EditorState } from "./editor.ts";
import { cursorFromRenderedPosition, renderMultiline } from "./render.ts";
import type { TerminalView } from "../ui/terminal.ts";

/** 注入终端能力，不读取测试宿主的 COLORTERM。 */
function terminal(width: number, overrides: Partial<TerminalView> = {}): TerminalView {
  return { width, height: 24, isTTY: false, kittyKeys: false,
    color: { isTTY: false, noColor: false, term: "xterm", colorTerm: "" }, ...overrides };
}

for (const vector of vectors.cases) {
  test(`${vector.name} 行号与软换行只影响画面`, () => {
    const state: EditorState = { ...initialEditorState(), lines: vector.lines, cursor: { line: 0, column: 0 } };
    const before = state.lines.join("\n");
    const frame = renderMultiline(state, terminal(vector.width));
    expect(frame.text).toBe(vector.expected);
    expect(state.lines.join("\n")).toBe(before);
    expect(frame.physicalRows).toBe(vector.expected.split("\n").length);
  });
}

test("双宽字符恰满一行不生成空续行，光标与视口保持有效", () => {
  const state: EditorState = { ...initialEditorState(), lines: ["中文abc", "第二行"], cursor: { line: 1, column: 3 } };
  const frame = renderMultiline(state, terminal(10, { height: 2 }));
  expect(frame.text).toBe("    2|第二\n     |行");
  expect(frame.cursorRow).toBe(2);
  expect(frame.cursorColumn).toBe(9);
  expect(frame.physicalRows).toBe(4);
});

test("灰色分隔符真彩色、256/16 色回退和四种无色条件都不改变布局", () => {
  const state = initialEditorState();
  const base = { isTTY: true, noColor: false, term: "xterm", colorTerm: "truecolor" };
  const rgb = renderMultiline(state, terminal(12, { isTTY: true, color: base })).text;
  const fallback256 = renderMultiline(state, terminal(12, { isTTY: true, color: { ...base, term: "xterm-256color", colorTerm: "" } })).text;
  const fallback16 = renderMultiline(state, terminal(12, { isTTY: true, color: { ...base, colorTerm: "" } })).text;
  expect(rgb).toContain("\u001b[38;2;127;127;127m|");
  expect(fallback256).toContain("\u001b[38;5;244m|");
  expect(fallback16).toContain("\u001b[90m|");
  for (const color of [
    { ...base, isTTY: false }, { ...base, noColor: true }, { ...base, term: "dumb" }, { ...base, mode: "never" as const },
  ]) {
    expect(renderMultiline(state, terminal(12, { color, isTTY: color.isTTY })).text).toBe("    1|");
  }
  const forced = renderMultiline(state, terminal(12, {
    isTTY: false,
    color: { ...base, isTTY: false, mode: "always" },
  })).text;
  expect(forced).toContain("\u001b[38;2;127;127;127m|");
  expect(stripAnsi(rgb)).toBe("    1|");
});

test("物理行来源可把中文点击映射到字素边界，双宽字符后半格归前", () => {
  const state: EditorState = { ...initialEditorState(), lines: ["中文ab", "尾"], cursor: { line: 0, column: 0 } };
  const frame = renderMultiline(state, terminal(12));
  expect(frame.rows).toEqual([{ line: 0, start: 0, end: 4 }, { line: 1, start: 0, end: 1 }]);
  expect(cursorFromRenderedPosition(frame, state, 1, 6)).toBeNull();
  expect(cursorFromRenderedPosition(frame, state, 1, 7)).toEqual({ line: 0, column: 0 });
  expect(cursorFromRenderedPosition(frame, state, 1, 8)).toEqual({ line: 0, column: 0 });
  expect(cursorFromRenderedPosition(frame, state, 1, 9)).toEqual({ line: 0, column: 1 });
  expect(cursorFromRenderedPosition(frame, state, 1, 99)).toEqual({ line: 0, column: 4 });
});

test("选区绘制只增加背景色，不改变去色后的源码布局", () => {
  const base = { isTTY: true, noColor: false, term: "xterm", colorTerm: "truecolor" };
  const state = selectRange({ ...initialEditorState(), lines: ["abcdef"], cursor: { line: 0, column: 4 } },
    { line: 0, column: 1 }, { line: 0, column: 4 });
  const frame = renderMultiline(state, terminal(12, { isTTY: true, color: base }));
  expect(frame.text).toContain("\u001b[48;2;55;80;110m");
  expect(stripAnsi(frame.text)).toBe("    1|abcdef");
});
