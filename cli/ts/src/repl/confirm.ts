/** I1b 确认态和输出收尾共用的终端分隔线。 */

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
  const colorizer = createColorizer({
    mode: view.color.mode ?? "auto",
    isTTY: view.isTTY,
    noColor: view.color.noColor ?? false,
    term: view.color.term ?? "",
    colorTerm: view.color.colorTerm ?? "",
  });
  const term = view.color.term ?? "";
  const marker = !colorizer.enabled ? ">" : `\u001b[${colorizer.trueColor ? "38;2;220;220;173" : term.includes("256color") ? "38;5;187" : "33"}m>\u001b[39m`;
  return `${separator(view, CONFIRM_TITLE)}\r\n${marker}\r\n${separator(view)}`;
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
