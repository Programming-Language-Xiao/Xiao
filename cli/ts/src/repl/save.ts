/** 保存位置输入独立于源码缓冲区，只在保存态生效。 */

import type { TerminalView } from "../ui/terminal.ts";
import { displayWidth } from "../ui/width.ts";
import { modalInputWindow, modalPrompt, separator } from "./confirm.ts";
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
    if (/[\x00-\x1f\x7f\u0085\u2028\u2029]/u.test(key.text)) {
      return { state: { ...state, error: "X11-CLI-SAVE-001: 路径不能包含控制或分隔字符" }, action: "none" };
    }
    const prefix = chars.slice(0, state.cursor).join("") + key.text;
    const text = prefix + chars.slice(state.cursor).join("");
    return { state: { text, cursor: graphemes(prefix).length, error: null }, action: "none" };
  }
  return { state, action: "none" };
}

/** 绘制自适应保存态；路径太长时只水平滚动输入，不截断原始状态。 */
export function renderSaveInput(state: SaveInputState, view: TerminalView): { text: string; cursorRow: number; cursorColumn: number } {
  const width = Math.max(1, Math.floor(view.width));
  const height = Math.max(1, Math.floor(view.height));
  const window = modalInputWindow(state.text, state.cursor, width);
  const marker = modalPrompt(view);
  const input = width === 1 ? marker : `${marker} ${window.text}`;
  if (height === 1) return { text: input, cursorRow: 1, cursorColumn: window.cursorColumn };
  const title = separator(view, SAVE_TITLE);
  const errorCapacity = state.error === null ? 0 : height >= 4 ? height - 3 : 1;
  const errorRows = errorCapacity === 0 ? [] : wrapNotice(state.error!, width).slice(-errorCapacity);
  if (height === 2) {
    return errorRows.length === 0
      ? { text: `${title}\r\n${input}`, cursorRow: 2, cursorColumn: window.cursorColumn }
      : { text: `${input}\r\n${errorRows[0]}`, cursorRow: 1, cursorColumn: window.cursorColumn };
  }
  const tail = errorRows.length === 0 ? separator(view) : height === 3 ? errorRows[0] : `${separator(view)}\r\n${errorRows.join("\r\n")}`;
  return { text: `${title}\r\n${input}\r\n${tail}`, cursorRow: 2, cursorColumn: window.cursorColumn };
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
