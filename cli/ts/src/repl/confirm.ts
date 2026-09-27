/** 确认、保存与面板共用的终端模态视觉边界。 */

import { createColorizer } from "../ui/color.ts";
import type { TerminalView } from "../ui/terminal.ts";
import { displayWidth } from "../ui/width.ts";
import { graphemes } from "./editor.ts";

/** 冻结的默认确认标题，11C 可只替换可见文案。 */
export const CONFIRM_TITLE = "Press Enter to confirm and run ↩︎";

/** 按显示列宽生成自适应分隔线，可在前段嵌入标题。 */
export function separator(view: TerminalView, title = ""): string {
  const width = Math.max(1, Math.floor(view.width));
  const line = title === "" ? "─".repeat(width) : titledLine(width, title);
  const colorizer = createColorizer({
    mode: view.color.mode ?? "auto",
    isTTY: view.isTTY,
    noColor: view.color.noColor ?? false,
    term: view.color.term ?? "",
    colorTerm: view.color.colorTerm ?? "",
  });
  if (!colorizer.enabled) return line;
  const term = view.color.term ?? "";
  const color = colorizer.trueColor ? "38;2;127;127;127" : term.includes("256color") ? "38;5;244" : "90";
  return `\u001b[${color}m${line}\u001b[39m`;
}

/** 绘制确认态三行：嵌入标题的分隔线、独立提示符、下方分隔线。 */
export function renderConfirmation(view: TerminalView): string {
  return `${separator(view, CONFIRM_TITLE)}\r\n${modalPrompt(view)}\r\n${separator(view)}`;
}

/** 确认、保存和后续面板共用的独立提示符配色。 */
export function modalPrompt(view: TerminalView): string {
  const colorizer = createColorizer({
    mode: view.color.mode ?? "auto",
    isTTY: view.isTTY,
    noColor: view.color.noColor ?? false,
    term: view.color.term ?? "",
    colorTerm: view.color.colorTerm ?? "",
  });
  const term = view.color.term ?? "";
  const marker = !colorizer.enabled ? ">" : `\u001b[${colorizer.trueColor ? "38;2;220;220;173" : term.includes("256color") ? "38;5;187" : "33"}m>\u001b[39m`;
  return marker;
}

/** 以字素光标裁剪单行模态输入，确保光标所在列始终可见。 */
export function modalInputWindow(text: string, cursor: number, width: number): { text: string; cursorColumn: number } {
  const columns = Math.max(1, Math.floor(width));
  const chars = graphemes(text);
  const available = Math.max(0, columns - 2);
  const position = Math.min(chars.length, Math.max(0, cursor));
  let start = position;
  let beforeCursorWidth = 0;
  while (start > 0 && beforeCursorWidth + displayWidth(chars[start - 1]) < available) {
    start -= 1;
    beforeCursorWidth += displayWidth(chars[start]);
  }
  let end = start;
  let used = 0;
  while (end < chars.length && used + displayWidth(chars[end]) <= available) {
    used += displayWidth(chars[end]);
    end += 1;
  }
  return {
    text: chars.slice(start, end).join(""),
    cursorColumn: Math.min(columns, Math.max(1, 3 + beforeCursorWidth)),
  };
}

/** 保持标题和分隔线在窄终端也精确占满目标列数。 */
function titledLine(width: number, title: string): string {
  if (width <= 2) return "─".repeat(width);
  const available = width - 2;
  let visible = "";
  for (const char of graphemes(title)) {
    if (displayWidth(visible + char) > available) break;
    visible += char;
  }
  const prefix = `─ ${visible}`;
  const used = displayWidth(prefix);
  return `${prefix}${used < width ? " " : ""}${"─".repeat(Math.max(0, width - used - 1))}`;
}
