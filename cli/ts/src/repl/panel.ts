/** 单行和多行模式共用的空命令面板模态。 */

import type { TerminalView } from "../ui/terminal.ts";
import { modalInputWindow, modalPrompt, separator } from "./confirm.ts";
import { graphemes } from "./editor.ts";
import type { KeyEvent } from "./keys.ts";

/** 默认可见标题；实际语言目录由 11C 接入。 */
export const PANEL_TITLE = "Command Panel";

/** 尚未提交的面板输入，与源码缓冲区互不共享。 */
export interface PanelState {
  text: string;
  cursor: number;
}

/** 一个按键对面板的纯状态转换结果。 */
export interface PanelKeyResult {
  state: PanelState;
  close: boolean;
}

/** 创建没有命令和输入的空面板。 */
export function initialPanelState(): PanelState {
  return { text: "", cursor: 0 };
}

/** 处理输入与关闭；Enter 不执行任何命令，也不清空已键入文本。 */
export function applyPanelKey(state: PanelState, key: KeyEvent): PanelKeyResult {
  if (key.kind === "escape" || key.kind === "panel" || key.kind === "interrupt") {
    return { state, close: true };
  }
  if (key.kind === "enter") return { state, close: false };
  const chars = graphemes(state.text);
  if (key.kind === "left") return { state: { ...state, cursor: Math.max(0, state.cursor - 1) }, close: false };
  if (key.kind === "right") return { state: { ...state, cursor: Math.min(chars.length, state.cursor + 1) }, close: false };
  if (key.kind === "home") return { state: { ...state, cursor: 0 }, close: false };
  if (key.kind === "end") return { state: { ...state, cursor: chars.length }, close: false };
  if (key.kind === "backspace" || key.kind === "delete") {
    const index = key.kind === "backspace" ? state.cursor - 1 : state.cursor;
    if (index < 0 || index >= chars.length) return { state, close: false };
    chars.splice(index, 1);
    return { state: { text: chars.join(""), cursor: key.kind === "backspace" ? index : state.cursor }, close: false };
  }
  if (key.kind === "text" || key.kind === "paste") {
    if (/[\x00-\x1f\x7f\u0085\u2028\u2029]/u.test(key.text)) return { state, close: false };
    const prefix = chars.slice(0, state.cursor).join("") + key.text;
    const text = prefix + chars.slice(state.cursor).join("");
    return { state: { text, cursor: graphemes(prefix).length }, close: false };
  }
  return { state, close: false };
}

/** 绘制无命令项的自适应面板，窄终端只滚动输入文本。 */
export function renderPanel(state: PanelState, view: TerminalView): { text: string; cursorRow: number; cursorColumn: number } {
  const width = Math.max(1, Math.floor(view.width));
  const height = Math.max(1, Math.floor(view.height));
  const window = modalInputWindow(state.text, state.cursor, width);
  const marker = modalPrompt(view);
  const input = width === 1 ? marker : `${marker} ${window.text}`;
  if (height === 1) return { text: input, cursorRow: 1, cursorColumn: window.cursorColumn };
  const title = separator(view, PANEL_TITLE);
  if (height === 2) return { text: `${title}\r\n${input}`, cursorRow: 2, cursorColumn: window.cursorColumn };
  return { text: `${title}\r\n${input}\r\n${separator(view)}`, cursorRow: 2, cursorColumn: window.cursorColumn };
}
