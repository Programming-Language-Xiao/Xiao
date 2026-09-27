/** 编辑器纯状态回归：字素、真实换行、边界与 kill 缓冲。 */

import { expect, test } from "bun:test";

import { applyKey, editorSource, initialEditorState, MAX_LOGICAL_LINES, selectedText, selectRange } from "./editor.ts";
import type { KeyEvent } from "./keys.ts";

/** 连续作用按键并返回最后状态。 */
function edit(...keys: KeyEvent[]) {
  return keys.reduce((state, key) => applyKey(state, key).state, initialEditorState());
}

test("多行粘贴保留真实换行、缩进和字素位置", () => {
  const state = edit({ kind: "paste", text: "首行\r\n    二行\n🙂e\u0301" });
  expect(state.lines).toEqual(["首行", "    二行", "🙂e\u0301"]);
  expect(state.cursor).toEqual({ line: 2, column: 2 });
  expect(editorSource(state)).toBe("首行\n    二行\n🙂e\u0301");
  const moved = applyKey(state, { kind: "left" }).state;
  expect(moved.cursor.column).toBe(1);
  expect(applyKey(moved, { kind: "backspace" }).state.lines[2]).toBe("e\u0301");
});

test("Enter、行首退格与行尾 Delete 只改变真实换行", () => {
  const state = edit({ kind: "text", text: "ab" }, { kind: "enter" }, { kind: "text", text: "cd" });
  expect(state.lines).toEqual(["ab", "cd"]);
  const home = applyKey(state, { kind: "home" }).state;
  expect(applyKey(home, { kind: "backspace" }).state.lines).toEqual(["abcd"]);
  const end = applyKey(applyKey(state, { kind: "up" }).state, { kind: "end" }).state;
  expect(applyKey(end, { kind: "delete" }).state.lines).toEqual(["abcd"]);
});

test("上下移动经过短行后记住目标列，按词操作基础分类明确", () => {
  const state = edit({ kind: "paste", text: "abcdef\nx\nword, next" }, { kind: "up" }, { kind: "up" });
  expect(state.cursor).toEqual({ line: 0, column: 6 });
  const long = edit({ kind: "paste", text: "abcdef\nx\nuvwxyz" });
  const up = applyKey(long, { kind: "up" }).state;
  expect(up.cursor).toEqual({ line: 1, column: 1 });
  expect(applyKey(up, { kind: "up" }).state.cursor).toEqual({ line: 0, column: 6 });
  const words = edit({ kind: "text", text: "word, next" });
  expect(applyKey(words, { kind: "word-left" }).state.cursor.column).toBe(6);
  expect(applyKey(applyKey(words, { kind: "home" }).state, { kind: "word-right" }).state.cursor.column).toBe(6);
});

test("Ctrl+W/K/U 更新唯一 kill 缓冲，Ctrl+Y 贴回最近内容", () => {
  const state = edit({ kind: "text", text: "alpha beta" }, { kind: "kill-word" });
  expect(state.lines).toEqual(["alpha "]);
  expect(state.killBuffer).toBe("beta");
  expect(applyKey(state, { kind: "yank" }).state.lines).toEqual(["alpha beta"]);
  const middle = applyKey(applyKey(state, { kind: "left" }).state, { kind: "kill-start" }).state;
  expect(middle.killBuffer).toBe("alpha");
  expect(applyKey(middle, { kind: "kill-end" }).state.killBuffer).toBe(" ");
});

test("空缓冲 Ctrl+D 是 EOF，非空 Delete；Ctrl+C 清空而 Esc 无操作", () => {
  const empty = initialEditorState();
  expect(applyKey(empty, { kind: "eof" }).effect).toBe("eof");
  const state = edit({ kind: "text", text: "ab" }, { kind: "home" });
  expect(applyKey(state, { kind: "eof" }).state.lines).toEqual(["b"]);
  expect(applyKey(state, { kind: "escape" }).state).toBe(state);
  expect(applyKey(state, { kind: "interrupt" }).state.lines).toEqual([""]);
});

test("99999 行上限对 Enter 和粘贴原子拒绝", () => {
  const state = { ...initialEditorState(), lines: Array(MAX_LOGICAL_LINES).fill("") as string[] };
  expect(applyKey(state, { kind: "enter" })).toEqual({ state, effect: "line-limit" });
  expect(applyKey(state, { kind: "paste", text: "一\n二" })).toEqual({ state, effect: "line-limit" });
});

test("覆盖模式替换当前字素，行尾追加且不吞掉真实换行", () => {
  const state = edit({ kind: "paste", text: "ab\ncd" });
  const lineStart = applyKey(applyKey(state, { kind: "up" }).state, { kind: "home" }).state;
  const overwrite = applyKey(lineStart, { kind: "insert" }).state;
  const replaced = applyKey(overwrite, { kind: "text", text: "中" }).state;
  expect(replaced.lines).toEqual(["中b", "cd"]);
  const lineEnd = applyKey(replaced, { kind: "end" }).state;
  expect(applyKey(lineEnd, { kind: "text", text: "!" }).state.lines).toEqual(["中b!", "cd"]);
});

test("Shift 方向键扩展选区，普通方向键折叠到对应端点", () => {
  const state = edit({ kind: "text", text: "abcdef" }, { kind: "home" },
    { kind: "right", shift: true }, { kind: "right", shift: true });
  expect(selectedText(state)).toBe("ab");
  expect(applyKey(state, { kind: "left" }).state.cursor).toEqual({ line: 0, column: 0 });
  expect(applyKey(state, { kind: "right" }).state.cursor).toEqual({ line: 0, column: 2 });
  const extended = applyKey(state, { kind: "end", shift: true }).state;
  expect(selectedText(extended)).toBe("ab cdef".replace(" ", ""));
});

test("跨逻辑行选区可复制、剪切并用粘贴替换，kill 缓冲共用一份剪贴板", () => {
  const state = edit({ kind: "paste", text: "ab\ncd\nef" });
  const selected = selectRange(state, { line: 0, column: 1 }, { line: 2, column: 1 });
  expect(selectedText(selected)).toBe("b\ncd\ne");
  const copied = applyKey(selected, { kind: "copy" }).state;
  expect(copied.lines).toEqual(state.lines);
  expect(copied.killBuffer).toBe("b\ncd\ne");
  const cut = applyKey(copied, { kind: "cut" }).state;
  expect(cut.lines).toEqual(["af"]);
  expect(cut.cursor).toEqual({ line: 0, column: 1 });
  const replaced = applyKey(selectRange(state, { line: 0, column: 1 }, { line: 2, column: 1 }), { kind: "paste", text: "X\nY" }).state;
  expect(replaced.lines).toEqual(["aX", "Yf"]);
  expect(editorSource(replaced)).toBe("aX\nYf");
});

test("Ctrl+Shift+A 全选，选区删除不改变剪贴板，editorSource 不带编辑状态", () => {
  const state = edit({ kind: "paste", text: "首行\n末行" });
  const all = applyKey(state, { kind: "select-all" }).state;
  expect(selectedText(all)).toBe("首行\n末行");
  const copied = applyKey(all, { kind: "copy" }).state;
  const deleted = applyKey(copied, { kind: "backspace" }).state;
  expect(deleted.lines).toEqual([""]);
  expect(deleted.killBuffer).toBe("首行\n末行");
  expect(editorSource(deleted)).toBe("");
  expect(deleted.overwrite).toBe(false);
});
