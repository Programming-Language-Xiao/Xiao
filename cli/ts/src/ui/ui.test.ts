/** 终端呈现层的中文宽度、颜色降级和表格回归。 */

import { describe, expect, test } from "bun:test";

import { createColorizer, stripAnsi } from "./color.ts";
import { displayWidth, renderTable } from "./width.ts";

describe("CLI 呈现层", () => {
  test("中文和 emoji 使用显示列宽而不是 UTF-16 长度", () => {
    expect(displayWidth("中文")).toBe(4);
    expect(displayWidth("✅")).toBe(2);
    const table = renderTable([
      { header: "状态", values: ["成功"] },
      { header: "代码", values: ["X11-CLI-001"] },
    ]);
    const lines = table.split("\n");
    expect(displayWidth(lines[0])).toBe(displayWidth(lines[2]));
  });

  test("默认非 TTY 和 NO_COLOR 不输出 ANSI，always 可显式覆盖", () => {
    const plain = createColorizer({ isTTY: false, colorTerm: "truecolor" });
    expect(plain.enabled).toBe(false);
    expect(plain.color("error", "错误")).toBe("错误");

    const forced = createColorizer({ mode: "always", isTTY: false, colorTerm: "truecolor" });
    const colored = forced.color("error", "错误");
    expect(forced.enabled).toBe(true);
    expect(colored).toContain("\u001B[38;2;");
    expect(stripAnsi(colored)).toBe("错误");

    const noColor = createColorizer({ isTTY: true, noColor: true });
    expect(noColor.enabled).toBe(false);

    const dumb = createColorizer({ isTTY: true, term: "dumb" });
    expect(dumb.enabled).toBe(false);
  });
});
