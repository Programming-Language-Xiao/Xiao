/** 逻辑行到物理终端行的纯映射；软换行从不进入源码缓冲。 */

import { createColorizer } from "../ui/color.ts";
import type { TerminalView } from "../ui/terminal.ts";
import { displayWidth } from "../ui/width.ts";
import { graphemes, type EditorState } from "./editor.ts";

/** 一帧可绘制的编辑区和屏幕光标位置（1-based）。 */
export interface RenderedMultiline {
  text: string;
  cursorRow: number;
  cursorColumn: number;
  physicalRows: number;
}

/** 给逻辑行画行号，续行用空白栏；双宽字符按实际显示列宽换行。 */
export function renderMultiline(state: EditorState, terminal: TerminalView): RenderedMultiline {
  const width = Math.max(8, Math.floor(terminal.width));
  const height = Math.max(1, Math.floor(terminal.height));
  const contentWidth = width - 6;
  const rows: string[] = [];
  let cursorRow = 0;
  let cursorColumn = 7;
  const separator = coloredSeparator(terminal);
  for (let line = 0; line < state.lines.length; line += 1) {
    const chars = graphemes(state.lines[line]);
    const segments = wrap(chars, contentWidth);
    for (let part = 0; part < segments.length; part += 1) {
      const segment = segments[part];
      const gutter = part === 0 ? String(line + 1).padStart(5, " ") : "     ";
      rows.push(`${gutter}${separator}${segment.text}`);
      if (line === state.cursor.line && state.cursor.column >= segment.start
        && (state.cursor.column < segment.end || part === segments.length - 1)) {
        cursorRow = rows.length - 1;
        const beforeCursor = chars.slice(segment.start, state.cursor.column).join("");
        cursorColumn = Math.min(width, 7 + displayWidth(beforeCursor));
      }
    }
  }
  const start = Math.max(0, cursorRow - height + 1);
  return {
    text: rows.slice(start, start + height).join("\n"),
    cursorRow: cursorRow - start + 1,
    cursorColumn,
    physicalRows: rows.length,
  };
}

/** 物理分段记住字素区间，光标在边界时归到下一续行。 */
function wrap(chars: readonly string[], contentWidth: number): Array<{ text: string; start: number; end: number }> {
  const parts: Array<{ text: string; start: number; end: number }> = [];
  let start = 0;
  let width = 0;
  for (let index = 0; index < chars.length; index += 1) {
    const cellWidth = displayWidth(chars[index]);
    if (width > 0 && width + cellWidth > contentWidth) {
      parts.push({ text: chars.slice(start, index).join(""), start, end: index });
      start = index;
      width = 0;
    }
    width += cellWidth;
  }
  parts.push({ text: chars.slice(start).join(""), start, end: chars.length });
  return parts;
}

/** 行号栏分隔符遵守与 I0 相同的无色和颜色回退规则。 */
function coloredSeparator(terminal: TerminalView): string {
  const options = terminal.color;
  const term = options.term ?? "";
  const colorizer = createColorizer({
    mode: options.mode === "never" ? "never" : "auto",
    isTTY: terminal.isTTY,
    noColor: options.noColor ?? false,
    term,
    colorTerm: options.colorTerm ?? "",
  });
  if (!colorizer.enabled) return "|";
  const color = colorizer.trueColor ? "38;2;127;127;127" : term.includes("256color") ? "38;5;244" : "90";
  return `\u001b[${color}m|\u001b[39m`;
}
