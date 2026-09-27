/** 逻辑行到物理终端行的纯映射；软换行从不进入源码缓冲。 */

import { createColorizer } from "../ui/color.ts";
import type { TerminalView } from "../ui/terminal.ts";
import { displayWidth } from "../ui/width.ts";
import { graphemes, selectionRange, type EditorCursor, type EditorSelection, type EditorState } from "./editor.ts";

/** 当前可见物理行对应的逻辑行与字素区间。 */
export interface RenderedRowSource {
  line: number;
  start: number;
  end: number;
}

/** 一帧可绘制的编辑区和屏幕光标位置（1-based）。 */
export interface RenderedMultiline {
  text: string;
  cursorRow: number;
  cursorColumn: number;
  physicalRows: number;
  rows: readonly RenderedRowSource[];
  scrollTop: number;
}

/** 给逻辑行画行号，续行用空白栏；双宽字符按实际显示列宽换行。 */
export function renderMultiline(state: EditorState, terminal: TerminalView): RenderedMultiline {
  const width = Math.max(8, Math.floor(terminal.width));
  const height = Math.max(1, Math.floor(terminal.height));
  const contentWidth = width - 6;
  const renderedRows: string[] = [];
  const sources: RenderedRowSource[] = [];
  let cursorRow = 0;
  let cursorColumn = 7;
  const separator = coloredSeparator(terminal);
  const selection = selectionRange(state);
  for (let line = 0; line < state.lines.length; line += 1) {
    const chars = graphemes(state.lines[line]);
    const segments = wrap(chars, contentWidth);
    for (let part = 0; part < segments.length; part += 1) {
      const segment = segments[part];
      const gutter = part === 0 ? String(line + 1).padStart(5, " ") : "     ";
      renderedRows.push(`${gutter}${separator}${renderSegment(segment, chars, line, selection, terminal)}`);
      sources.push({ line, start: segment.start, end: segment.end });
      if (line === state.cursor.line && state.cursor.column >= segment.start
        && (state.cursor.column < segment.end || part === segments.length - 1)) {
        cursorRow = renderedRows.length - 1;
        const beforeCursor = chars.slice(segment.start, state.cursor.column).join("");
        cursorColumn = Math.min(width, 7 + displayWidth(beforeCursor));
      }
    }
  }
  const start = Math.max(0, cursorRow - height + 1);
  return {
    text: renderedRows.slice(start, start + height).join("\n"),
    cursorRow: cursorRow - start + 1,
    cursorColumn,
    physicalRows: renderedRows.length,
    rows: sources.slice(start, start + height),
    scrollTop: start,
  };
}

/** 把当前帧的 1-based 终端坐标反向映射到字素边界。 */
export function cursorFromRenderedPosition(
  frame: RenderedMultiline,
  state: EditorState,
  row: number,
  column: number,
): EditorCursor | null {
  if (row < 1 || row > frame.rows.length || column <= 6) return null;
  const source = frame.rows[row - 1];
  const chars = graphemes(state.lines[source.line]).slice(source.start, source.end);
  let offset = Math.max(0, column - 7);
  for (let index = 0; index < chars.length; index += 1) {
    const width = displayWidth(chars[index]);
    if (offset < width) return { line: source.line, column: source.start + index };
    offset -= width;
  }
  return { line: source.line, column: source.end };
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

/** 按锚点与光标对字素绘制选中背景，不改变源码显示宽度。 */
function renderSegment(
  segment: { start: number; end: number },
  chars: readonly string[],
  line: number,
  selection: EditorSelection | null,
  terminal: TerminalView,
): string {
  if (selection === null) return chars.slice(segment.start, segment.end).join("");
  const color = selectionColor(terminal);
  if (color === null) return chars.slice(segment.start, segment.end).join("");
  let result = "";
  let active = false;
  for (let index = segment.start; index < segment.end; index += 1) {
    const selected = inSelection({ line, column: index }, selection);
    if (selected && !active) { result += color.start; active = true; }
    if (!selected && active) { result += color.end; active = false; }
    result += chars[index];
  }
  if (active) result += color.end;
  return result;
}

/** 判断一个字素起点是否位于半开选区中。 */
function inSelection(cursor: EditorCursor, selection: EditorSelection): boolean {
  return compareCursors(cursor, selection.start) >= 0 && compareCursors(cursor, selection.end) < 0;
}

/** 比较两个逻辑源码位置。 */
function compareCursors(left: EditorCursor, right: EditorCursor): number {
  return left.line - right.line || left.column - right.column;
}

/** 选区背景按终端颜色能力降级；无色模式保持纯文本。 */
function selectionColor(terminal: TerminalView): { start: string; end: string } | null {
  const options = terminal.color;
  const term = options.term ?? "";
  const colorizer = createColorizer({
    mode: options.mode ?? "auto",
    isTTY: terminal.isTTY,
    noColor: options.noColor ?? false,
    term,
    colorTerm: options.colorTerm ?? "",
  });
  if (!colorizer.enabled) return null;
  const background = colorizer.trueColor ? "48;2;55;80;110" : term.includes("256color") ? "48;5;24" : "44";
  return { start: `\u001b[${background}m`, end: "\u001b[49m" };
}

/** 行号栏分隔符遵守与 I0 相同的无色和颜色回退规则。 */
function coloredSeparator(terminal: TerminalView): string {
  const options = terminal.color;
  const term = options.term ?? "";
  const colorizer = createColorizer({
    mode: options.mode ?? "auto",
    isTTY: terminal.isTTY,
    noColor: options.noColor ?? false,
    term,
    colorTerm: options.colorTerm ?? "",
  });
  if (!colorizer.enabled) return "|";
  const color = colorizer.trueColor ? "38;2;127;127;127" : term.includes("256color") ? "38;5;244" : "90";
  return `\u001b[${color}m|\u001b[39m`;
}
