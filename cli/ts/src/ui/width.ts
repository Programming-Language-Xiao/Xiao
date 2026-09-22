/** 终端列宽和简单表格布局；所有宽度都使用 Unicode 感知实现。 */

import stringWidth from "string-width";

/** 表格对齐方向。 */
export type TableAlignment = "left" | "right";

/** 一个待渲染的表格列。 */
export interface TableColumn {
  /** 列标题。 */
  header: string;
  /** 列内容。 */
  values: readonly string[];
  /** 内容对齐方式。 */
  align?: TableAlignment;
}

/** 返回字符串在终端中占用的显示列数。 */
export function displayWidth(text: string): number {
  return stringWidth(text);
}

/** 按显示列数补齐文本，不使用 UTF-16 长度。 */
export function padDisplay(text: string, width: number, align: TableAlignment = "left"): string {
  const padding = Math.max(0, width - displayWidth(text));
  const spaces = " ".repeat(padding);
  return align === "right" ? `${spaces}${text}` : `${text}${spaces}`;
}

/**
 * 渲染一个无边框表格。
 *
 * 表格使用两个空格作为列间隔，适合窄终端和管道输出；ANSI 由
 * `string-width` 自动忽略，因此可在颜色化后再布局。
 */
export function renderTable(columns: readonly TableColumn[], separator = "  "): string {
  if (columns.length === 0) return "";
  const rowCount = Math.max(0, ...columns.map((column) => column.values.length));
  const widths = columns.map((column) => Math.max(
    displayWidth(column.header),
    ...column.values.map(displayWidth),
  ));
  const lines: string[] = [];
  lines.push(columns.map((column, index) => padDisplay(column.header, widths[index], column.align)).join(separator));
  lines.push(widths.map((width) => "-".repeat(width)).join(separator));
  for (let row = 0; row < rowCount; row += 1) {
    lines.push(columns.map((column, index) => padDisplay(column.values[row] ?? "", widths[index], column.align)).join(separator));
  }
  return lines.join("\n");
}
