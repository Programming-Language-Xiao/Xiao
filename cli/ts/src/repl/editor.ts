/** 多行源码缓冲与光标的纯状态转换；列号以 Unicode 字素为单位。 */

import type { KeyEvent } from "./keys.ts";

/** 一个逻辑源码位置，不计入终端软换行。 */
export interface EditorCursor {
  line: number;
  column: number;
}

/** 多行编辑状态；只有最近一次删除写入唯一 kill 缓冲。 */
export interface EditorState {
  lines: readonly string[];
  cursor: EditorCursor;
  preferredColumn: number | null;
  killBuffer: string;
}

/** 按键产生的缓冲区变化与控制动作。 */
export interface EditResult {
  state: EditorState;
  effect: "none" | "eof" | "interrupt" | "line-limit" | "redraw" | "run" | "save" | "panel";
}

/** 行号协议允许的最大逻辑行数。 */
export const MAX_LOGICAL_LINES = 99999;

const segmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });

/** 创建只有一个空逻辑行的编辑器。 */
export function initialEditorState(): EditorState {
  return { lines: [""], cursor: { line: 0, column: 0 }, preferredColumn: null, killBuffer: "" };
}

/** 源码视图只包含真实换行；软换行和终端控制指令不在此处生成。 */
export function editorSource(state: EditorState): string {
  return state.lines.join("\n");
}

/** Unicode 字素是编辑光标的最小单位，不能拆开代理对或组合字符。 */
export function graphemes(text: string): string[] {
  return Array.from(segmenter.segment(text), (part) => part.segment);
}

/** 应用一个按键，不读取终端、不写屏幕，也不执行源码。 */
export function applyKey(state: EditorState, key: KeyEvent): EditResult {
  switch (key.kind) {
    case "text":
    case "paste": return insertText(state, key.text);
    case "enter": return insertText(state, "\n");
    case "backspace": return backspace(state);
    case "delete": return deleteForward(state);
    case "eof": return editorSource(state) === "" ? { state, effect: "eof" } : deleteForward(state);
    case "left": return moveHorizontal(state, -1);
    case "right": return moveHorizontal(state, 1);
    case "up": return moveVertical(state, -1);
    case "down": return moveVertical(state, 1);
    case "home": return positioned(state, { line: state.cursor.line, column: 0 });
    case "end": return positioned(state, { line: state.cursor.line, column: graphemes(state.lines[state.cursor.line]).length });
    case "word-left": return positioned(state, { line: state.cursor.line, column: wordLeft(graphemes(state.lines[state.cursor.line]), state.cursor.column) });
    case "word-right": return positioned(state, { line: state.cursor.line, column: wordRight(graphemes(state.lines[state.cursor.line]), state.cursor.column) });
    case "kill-word": return killPreviousWord(state);
    case "kill-end": return killRange(state, state.cursor.column, graphemes(state.lines[state.cursor.line]).length);
    case "kill-start": return killRange(state, 0, state.cursor.column);
    case "yank": return state.killBuffer === "" ? unchanged(state) : insertText(state, state.killBuffer);
    case "interrupt": return { state: { ...initialEditorState(), killBuffer: state.killBuffer }, effect: "interrupt" };
    case "redraw": return { state, effect: "redraw" };
    case "shift-enter": return { state, effect: "run" };
    case "save": return { state, effect: "save" };
    case "panel": return { state, effect: "panel" };
    case "escape":
    case "kitty-report": return unchanged(state);
  }
}

/** 粘贴保留真实换行与缩进，超出 99999 行时整次插入原子拒绝。 */
function insertText(state: EditorState, text: string): EditResult {
  const pieces = text.replaceAll("\r\n", "\n").replaceAll("\r", "\n").split("\n");
  if (state.lines.length + pieces.length - 1 > MAX_LOGICAL_LINES) return { state, effect: "line-limit" };
  const { line, column } = state.cursor;
  const current = graphemes(state.lines[line]);
  const before = current.slice(0, column).join("");
  const after = current.slice(column).join("");
  const replacement = pieces.length === 1 ? [before + pieces[0] + after]
    : [before + pieces[0], ...pieces.slice(1, -1), pieces[pieces.length - 1] + after];
  const nextLine = line + replacement.length - 1;
  const nextColumn = graphemes(pieces.length === 1 ? before + pieces[0] : pieces[pieces.length - 1]).length;
  return {
    state: {
      ...state,
      lines: [...state.lines.slice(0, line), ...replacement, ...state.lines.slice(line + 1)],
      cursor: { line: nextLine, column: nextColumn }, preferredColumn: null,
    },
    effect: "none",
  };
}

/** 退格在行首删除真实换行并合并上一逻辑行。 */
function backspace(state: EditorState): EditResult {
  const { line, column } = state.cursor;
  if (column > 0) {
    const chars = graphemes(state.lines[line]);
    chars.splice(column - 1, 1);
    return replaceLine(state, line, chars.join(""), column - 1);
  }
  if (line === 0) return unchanged(state);
  const previous = state.lines[line - 1];
  return {
    state: { ...state, lines: [...state.lines.slice(0, line - 1), previous + state.lines[line], ...state.lines.slice(line + 1)],
      cursor: { line: line - 1, column: graphemes(previous).length }, preferredColumn: null },
    effect: "none",
  };
}

/** 删除当前字素；在行尾则删除真实换行。 */
function deleteForward(state: EditorState): EditResult {
  const { line, column } = state.cursor;
  const chars = graphemes(state.lines[line]);
  if (column < chars.length) {
    chars.splice(column, 1);
    return replaceLine(state, line, chars.join(""), column);
  }
  if (line + 1 >= state.lines.length) return unchanged(state);
  return {
    state: { ...state, lines: [...state.lines.slice(0, line), state.lines[line] + state.lines[line + 1], ...state.lines.slice(line + 2)], preferredColumn: null },
    effect: "none",
  };
}

/** 左右移动可跨过真实换行，但不改变源码。 */
function moveHorizontal(state: EditorState, direction: -1 | 1): EditResult {
  const { line, column } = state.cursor;
  const length = graphemes(state.lines[line]).length;
  if (direction < 0 && column > 0) return positioned(state, { line, column: column - 1 });
  if (direction < 0 && line > 0) return positioned(state, { line: line - 1, column: graphemes(state.lines[line - 1]).length });
  if (direction > 0 && column < length) return positioned(state, { line, column: column + 1 });
  if (direction > 0 && line + 1 < state.lines.length) return positioned(state, { line: line + 1, column: 0 });
  return unchanged(state);
}

/** 连续上下移动记住目标列，经过短行后仍可回到原列。 */
function moveVertical(state: EditorState, direction: -1 | 1): EditResult {
  const line = state.cursor.line + direction;
  if (line < 0 || line >= state.lines.length) return unchanged(state);
  const preferred = state.preferredColumn ?? state.cursor.column;
  return { state: { ...state, cursor: { line, column: Math.min(preferred, graphemes(state.lines[line]).length) }, preferredColumn: preferred }, effect: "none" };
}

/** 按词操作将空白、字母数字与标点分为三类。 */
function wordClass(char: string): "space" | "word" | "punctuation" {
  if (/^\s+$/u.test(char)) return "space";
  return /^[\p{L}\p{N}\p{M}_]/u.test(char) ? "word" : "punctuation";
}

/** 寻找当前列之前的词首。 */
function wordLeft(chars: readonly string[], column: number): number {
  let target = column;
  while (target > 0 && wordClass(chars[target - 1]) === "space") target -= 1;
  if (target === 0) return 0;
  const category = wordClass(chars[target - 1]);
  while (target > 0 && wordClass(chars[target - 1]) === category) target -= 1;
  return target;
}

/** 越过当前词及后续空白，停在下一词首。 */
function wordRight(chars: readonly string[], column: number): number {
  let target = column;
  if (target < chars.length && wordClass(chars[target]) !== "space") {
    const category = wordClass(chars[target]);
    while (target < chars.length && wordClass(chars[target]) === category) target += 1;
  }
  while (target < chars.length && wordClass(chars[target]) !== "word") target += 1;
  return target;
}

/** 删除上一词并记录最近一次 kill 内容。 */
function killPreviousWord(state: EditorState): EditResult {
  const chars = graphemes(state.lines[state.cursor.line]);
  const start = wordLeft(chars, state.cursor.column);
  return killRange(state, start, state.cursor.column);
}

/** 从当前逻辑行删除字素区间。 */
function killRange(state: EditorState, start: number, end: number): EditResult {
  if (start === end) return unchanged(state);
  const chars = graphemes(state.lines[state.cursor.line]);
  const killed = chars.slice(start, end).join("");
  const next = replaceLine(state, state.cursor.line, [...chars.slice(0, start), ...chars.slice(end)].join(""), start);
  return { state: { ...next.state, killBuffer: killed }, effect: "none" };
}

/** 更新一条逻辑行与光标。 */
function replaceLine(state: EditorState, line: number, text: string, column: number): EditResult {
  const lines = [...state.lines];
  lines[line] = text;
  return { state: { ...state, lines, cursor: { line, column }, preferredColumn: null }, effect: "none" };
}

/** 水平或行内定位时清除竖直目标列。 */
function positioned(state: EditorState, cursor: EditorCursor): EditResult {
  return { state: { ...state, cursor, preferredColumn: null }, effect: "none" };
}

/** 无效或不可上报的键不改变状态。 */
function unchanged(state: EditorState): EditResult {
  return { state, effect: "none" };
}
