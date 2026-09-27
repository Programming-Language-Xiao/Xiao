/** 多行源码缓冲与光标的纯状态转换；列号以 Unicode 字素为单位。 */

import type { KeyEvent } from "./keys.ts";

/** 一个逻辑源码位置，不计入终端软换行。 */
export interface EditorCursor {
  line: number;
  column: number;
}

/** 规范化后的半开选区；`end` 不包含在选区内。 */
export interface EditorSelection {
  start: EditorCursor;
  end: EditorCursor;
}

/** 多行编辑状态；kill 缓冲同时作为程序内编辑器剪贴板。 */
export interface EditorState {
  lines: readonly string[];
  cursor: EditorCursor;
  anchor: EditorCursor | null;
  overwrite: boolean;
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
  return {
    lines: [""], cursor: { line: 0, column: 0 }, anchor: null, overwrite: false,
    preferredColumn: null, killBuffer: "",
  };
}

/** 源码视图只包含真实换行；软换行和终端控制指令不在此处生成。 */
export function editorSource(state: EditorState): string {
  return state.lines.join("\n");
}

/** Unicode 字素是编辑光标的最小单位，不能拆开代理对或组合字符。 */
export function graphemes(text: string): string[] {
  return Array.from(segmenter.segment(text), (part) => part.segment);
}

/** 返回锚点与光标组成的规范化半开选区。 */
export function selectionRange(state: EditorState): EditorSelection | null {
  if (state.anchor === null || compareCursors(state.anchor, state.cursor) === 0) return null;
  return compareCursors(state.anchor, state.cursor) < 0
    ? { start: state.anchor, end: state.cursor }
    : { start: state.cursor, end: state.anchor };
}

/** 返回选区的真实源码文本；无选区时返回空字符串。 */
export function selectedText(state: EditorState): string {
  const selection = selectionRange(state);
  if (selection === null) return "";
  const { start, end } = selection;
  if (start.line === end.line) return graphemes(state.lines[start.line]).slice(start.column, end.column).join("");
  return [
    graphemes(state.lines[start.line]).slice(start.column).join(""),
    ...state.lines.slice(start.line + 1, end.line),
    graphemes(state.lines[end.line]).slice(0, end.column).join(""),
  ].join("\n");
}

/** 将鼠标点击定位到单一光标并取消选区。 */
export function positionCursor(state: EditorState, cursor: EditorCursor): EditorState {
  return { ...state, cursor, anchor: null, preferredColumn: null };
}

/** 以指定锚点和活动端建立选区。 */
export function selectRange(state: EditorState, anchor: EditorCursor, cursor: EditorCursor): EditorState {
  return { ...state, anchor, cursor, preferredColumn: null };
}

/** 应用一个按键，不读取终端、不写屏幕，也不执行源码。 */
export function applyKey(state: EditorState, key: KeyEvent): EditResult {
  switch (key.kind) {
    case "text": return insertText(state, key.text, state.overwrite);
    case "paste": return insertText(state, key.text, false);
    case "enter": return insertText(state, "\n", false);
    case "backspace": return backspace(state);
    case "delete": return deleteForward(state);
    case "left": return moveHorizontal(state, -1, key.shift === true);
    case "right": return moveHorizontal(state, 1, key.shift === true);
    case "up": return moveVertical(state, -1, key.shift === true);
    case "down": return moveVertical(state, 1, key.shift === true);
    case "home": return positioned(state, { line: state.cursor.line, column: 0 }, key.shift === true);
    case "end": return positioned(state, { line: state.cursor.line, column: graphemes(state.lines[state.cursor.line]).length }, key.shift === true);
    case "buffer-home": return positioned(state, { line: 0, column: 0 });
    case "buffer-end": {
      const line = state.lines.length - 1;
      return positioned(state, { line, column: graphemes(state.lines[line]).length });
    }
    case "word-left": return positioned(state, { line: state.cursor.line, column: wordLeft(graphemes(state.lines[state.cursor.line]), state.cursor.column) });
    case "word-right": return positioned(state, { line: state.cursor.line, column: wordRight(graphemes(state.lines[state.cursor.line]), state.cursor.column) });
    case "kill-word": return killPreviousWord(state);
    case "kill-end": return killRange(state, state.cursor.column, graphemes(state.lines[state.cursor.line]).length);
    case "kill-start": return killRange(state, 0, state.cursor.column);
    case "yank": return state.killBuffer === "" ? unchanged(state) : insertText(state, state.killBuffer, false);
    case "insert": return { state: { ...state, overwrite: !state.overwrite }, effect: "none" };
    case "select-all": return { state: selectAll(state), effect: "none" };
    case "copy": return copySelection(state);
    case "cut": return cutSelection(state);
    case "eof": {
      const selection = selectionRange(state);
      if (selection !== null) return replaceRange(state, selection.start, selection.end, "");
      return editorSource(state) === "" ? { state, effect: "eof" } : deleteForward(state);
    }
    case "interrupt": return { state: { ...initialEditorState(), killBuffer: state.killBuffer }, effect: "interrupt" };
    case "redraw": return { state, effect: "redraw" };
    case "shift-enter": return { state, effect: "run" };
    case "save": return { state, effect: "save" };
    case "panel": return { state, effect: "panel" };
    case "escape":
    case "kitty-report":
    case "mouse": return unchanged(state);
  }
}

/** 输入文本；覆盖模式只作用于没有换行的可打印文本。 */
function insertText(state: EditorState, text: string, overwrite: boolean): EditResult {
  const selection = selectionRange(state);
  if (selection !== null) return replaceRange(state, selection.start, selection.end, text);
  if (overwrite && !text.includes("\n") && !text.includes("\r")) return overwriteText(state, text);
  return replaceRange(state, state.cursor, state.cursor, text);
}

/** 覆盖当前逻辑行的字素；行尾只追加，不跨过真实换行。 */
function overwriteText(state: EditorState, text: string): EditResult {
  const incoming = graphemes(text);
  if (incoming.length === 0) return unchanged(state);
  const chars = graphemes(state.lines[state.cursor.line]);
  const next = [
    ...chars.slice(0, state.cursor.column),
    ...incoming,
    ...chars.slice(state.cursor.column + incoming.length),
  ].join("");
  return replaceLine(state, state.cursor.line, next, state.cursor.column + incoming.length);
}

/** 用文本替换任意跨行半开范围。 */
function replaceRange(state: EditorState, start: EditorCursor, end: EditorCursor, text: string): EditResult {
  const pieces = text.replaceAll("\r\n", "\n").replaceAll("\r", "\n").split("\n");
  const removedLines = end.line - start.line;
  const nextLineCount = state.lines.length - removedLines + pieces.length - 1;
  if (nextLineCount > MAX_LOGICAL_LINES) return { state, effect: "line-limit" };
  const startChars = graphemes(state.lines[start.line]);
  const endChars = graphemes(state.lines[end.line]);
  const before = startChars.slice(0, start.column).join("");
  const after = endChars.slice(end.column).join("");
  const replacement = pieces.length === 1 ? [before + pieces[0] + after]
    : [before + pieces[0], ...pieces.slice(1, -1), pieces[pieces.length - 1] + after];
  const nextLine = start.line + replacement.length - 1;
  const nextColumn = pieces.length === 1
    ? graphemes(before + pieces[0]).length
    : graphemes(pieces[pieces.length - 1]).length;
  return {
    state: {
      ...state,
      lines: [...state.lines.slice(0, start.line), ...replacement, ...state.lines.slice(end.line + 1)],
      cursor: { line: nextLine, column: nextColumn }, anchor: null, preferredColumn: null,
    },
    effect: "none",
  };
}

/** 退格在行首删除真实换行并合并上一逻辑行。 */
function backspace(state: EditorState): EditResult {
  const selection = selectionRange(state);
  if (selection !== null) return replaceRange(state, selection.start, selection.end, "");
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
      cursor: { line: line - 1, column: graphemes(previous).length }, anchor: null, preferredColumn: null },
    effect: "none",
  };
}

/** 删除当前字素；在行尾则删除真实换行。 */
function deleteForward(state: EditorState): EditResult {
  const selection = selectionRange(state);
  if (selection !== null) return replaceRange(state, selection.start, selection.end, "");
  const { line, column } = state.cursor;
  const chars = graphemes(state.lines[line]);
  if (column < chars.length) {
    chars.splice(column, 1);
    return replaceLine(state, line, chars.join(""), column);
  }
  if (line + 1 >= state.lines.length) return unchanged(state);
  return {
    state: { ...state, lines: [...state.lines.slice(0, line), state.lines[line] + state.lines[line + 1], ...state.lines.slice(line + 2)], anchor: null, preferredColumn: null },
    effect: "none",
  };
}

/** 左右移动可跨过真实换行；无 Shift 的移动先折叠选区。 */
function moveHorizontal(state: EditorState, direction: -1 | 1, extend: boolean): EditResult {
  const selection = selectionRange(state);
  if (selection !== null && !extend) return positioned(state, direction < 0 ? selection.start : selection.end);
  const { line, column } = state.cursor;
  const length = graphemes(state.lines[line]).length;
  if (direction < 0 && column > 0) return positioned(state, { line, column: column - 1 }, extend);
  if (direction < 0 && line > 0) return positioned(state, { line: line - 1, column: graphemes(state.lines[line - 1]).length }, extend);
  if (direction > 0 && column < length) return positioned(state, { line, column: column + 1 }, extend);
  if (direction > 0 && line + 1 < state.lines.length) return positioned(state, { line: line + 1, column: 0 }, extend);
  return unchanged(state);
}

/** 上下移动尽量保持目标列；无 Shift 的移动先折叠选区。 */
function moveVertical(state: EditorState, direction: -1 | 1, extend: boolean): EditResult {
  const selection = selectionRange(state);
  if (selection !== null && !extend) return positioned(state, direction < 0 ? selection.start : selection.end);
  const line = state.cursor.line + direction;
  if (line < 0 || line >= state.lines.length) return unchanged(state);
  const preferred = state.preferredColumn ?? state.cursor.column;
  const anchor = extend ? state.anchor ?? state.cursor : null;
  return { state: { ...state, cursor: { line, column: Math.min(preferred, graphemes(state.lines[line]).length) }, anchor, preferredColumn: preferred }, effect: "none" };
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
  const base = state.anchor === null ? state : positionCursor(state, state.cursor);
  const chars = graphemes(base.lines[base.cursor.line]);
  const start = wordLeft(chars, base.cursor.column);
  return killRange(base, start, base.cursor.column);
}

/** 从当前逻辑行删除字素区间。 */
function killRange(state: EditorState, start: number, end: number): EditResult {
  const base = state.anchor === null ? state : positionCursor(state, state.cursor);
  if (start === end) return unchanged(base);
  const chars = graphemes(base.lines[base.cursor.line]);
  const killed = chars.slice(start, end).join("");
  const next = replaceLine(base, base.cursor.line, [...chars.slice(0, start), ...chars.slice(end)].join(""), start);
  return { state: { ...next.state, killBuffer: killed }, effect: "none" };
}

/** 更新一条逻辑行与光标。 */
function replaceLine(state: EditorState, line: number, text: string, column: number): EditResult {
  const lines = [...state.lines];
  lines[line] = text;
  return { state: { ...state, lines, cursor: { line, column }, anchor: null, preferredColumn: null }, effect: "none" };
}

/** 水平或行内定位时清除竖直目标列。 */
function positioned(state: EditorState, cursor: EditorCursor, extend = false): EditResult {
  const anchor = extend ? state.anchor ?? state.cursor : null;
  return { state: { ...state, cursor, anchor, preferredColumn: null }, effect: "none" };
}

/** 比较两个逻辑源码位置。 */
function compareCursors(left: EditorCursor, right: EditorCursor): number {
  return left.line - right.line || left.column - right.column;
}

/** 选择整个源码缓冲区。 */
function selectAll(state: EditorState): EditorState {
  const line = state.lines.length - 1;
  return selectRange(state, { line: 0, column: 0 }, { line, column: graphemes(state.lines[line]).length });
}

/** 复制选区到与 kill 操作共享的编辑器剪贴板。 */
function copySelection(state: EditorState): EditResult {
  const text = selectedText(state);
  return text === "" ? unchanged(state) : { state: { ...state, killBuffer: text }, effect: "none" };
}

/** 剪切选区到与 kill 操作共享的编辑器剪贴板。 */
function cutSelection(state: EditorState): EditResult {
  const selection = selectionRange(state);
  if (selection === null) return unchanged(state);
  const text = selectedText(state);
  const result = replaceRange(state, selection.start, selection.end, "");
  return { state: { ...result.state, killBuffer: text }, effect: result.effect };
}

/** 无效或不可上报的键不改变状态。 */
function unchanged(state: EditorState): EditResult {
  return { state, effect: "none" };
}
