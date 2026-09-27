/** 保存位置输入独立于源码缓冲区，只在保存态生效。 */

import type { TerminalView } from "../ui/terminal.ts";
import { displayWidth } from "../ui/width.ts";
import { modalPrompt, separator } from "./confirm.ts";
import { graphemes } from "./editor.ts";
import type { KeyEvent } from "./keys.ts";

/** 保存态标题，与运行确认标题明确区分。 */
export const SAVE_TITLE = "Enter the save location";

/** 尚未提交的路径和字素光标，不属于编辑器源码。 */
export interface SaveInputState {
  text: string;
  cursor: number;
  error: string | null;
}

/** 保存输入处理结果。 */
export interface SaveInputResult {
  state: SaveInputState;
  action: "none" | "cancel" | "submit";
}

/** 创建空路径输入状态。 */
export function initialSaveInput(): SaveInputState {
  return { text: "", cursor: 0, error: null };
}

/** 只编辑路径文本；`q` 不可能是合法 `.xiao` 目标，扩展名放开时须重审取消键。 */
export function applySaveKey(state: SaveInputState, key: KeyEvent): SaveInputResult {
  const chars = graphemes(state.text);
  if (key.kind === "escape" || key.kind === "interrupt") return { state, action: "cancel" };
  if (key.kind === "enter") return { state, action: state.text === "q" ? "cancel" : "submit" };
  if (key.kind === "left") return { state: { ...state, cursor: Math.max(0, state.cursor - 1) }, action: "none" };
  if (key.kind === "right") return { state: { ...state, cursor: Math.min(chars.length, state.cursor + 1) }, action: "none" };
  if (key.kind === "home") return { state: { ...state, cursor: 0 }, action: "none" };
  if (key.kind === "end") return { state: { ...state, cursor: chars.length }, action: "none" };
  if (key.kind === "backspace" || key.kind === "delete") {
    const index = key.kind === "backspace" ? state.cursor - 1 : state.cursor;
    if (index < 0 || index >= chars.length) return { state, action: "none" };
    chars.splice(index, 1);
    return { state: { text: chars.join(""), cursor: key.kind === "backspace" ? index : state.cursor, error: null }, action: "none" };
  }
  if (key.kind === "text" || key.kind === "paste") {
    if (/[\x00-\x1f\x7f]/u.test(key.text)) {
      return { state: { ...state, error: "X11-CLI-SAVE-001: 路径不能包含控制字符" }, action: "none" };
    }
    const inserted = graphemes(key.text);
    chars.splice(state.cursor, 0, ...inserted);
    return { state: { text: chars.join(""), cursor: state.cursor + inserted.length, error: null }, action: "none" };
  }
  return { state, action: "none" };
}

/** 绘制自适应保存态；路径太长时只水平滚动输入，不截断原始状态。 */
export function renderSaveInput(state: SaveInputState, view: TerminalView): { text: string; cursorColumn: number } {
  const width = Math.max(1, Math.floor(view.width));
  const chars = graphemes(state.text);
  const available = Math.max(0, width - 2);
  let start = 0;
  while (start < state.cursor && displayWidth(chars.slice(start, state.cursor).join("")) >= available && available > 0) start += 1;
  let end = start;
  let used = 0;
  while (end < chars.length && used + displayWidth(chars[end]) <= available) {
    used += displayWidth(chars[end]);
    end += 1;
  }
  const visible = chars.slice(start, end).join("");
  const marker = modalPrompt(view);
  const input = width === 1 ? marker : `${marker} ${visible}`;
  const cursorColumn = Math.min(width, Math.max(1, 3 + displayWidth(chars.slice(start, state.cursor).join(""))));
  const errorRows = state.error === null ? [] : wrapNotice(state.error, width)
    .slice(-Math.max(0, Math.floor(view.height) - 3));
  const error = errorRows.length === 0 ? "" : `\r\n${errorRows.join("\r\n")}`;
  return {
    text: `${separator(view, SAVE_TITLE)}\r\n${input}\r\n${separator(view)}${error}`,
    cursorColumn,
  };
}

/** 按终端显示列宽换行诊断，避免路径和原因覆盖相邻界面。 */
export function wrapNotice(message: string, width: number): string[] {
  const limit = Math.max(1, Math.floor(width));
  const rows: string[] = [];
  let current = "";
  let used = 0;
  for (const char of graphemes(message.replaceAll(/[\r\n]/gu, " "))) {
    const size = displayWidth(char);
    if (used + size > limit && current !== "") {
      rows.push(current);
      current = "";
      used = 0;
    }
    if (size > limit) continue;
    current += char;
    used += size;
  }
  rows.push(current);
  return rows;
}
