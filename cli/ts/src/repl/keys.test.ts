/** 多平台原始键流采用同一套可注入向量，不依赖真实 TTY。 */

import { expect, test } from "bun:test";

import { flushPendingKeys, parseKeys } from "./keys.ts";

test("传统 Enter 只能新建逻辑行，Kitty 回复本身不是输入", () => {
  const legacy = parseKeys(Buffer.from("\r"));
  expect(legacy.events).toEqual([{ kind: "enter" }]);
  expect(parseKeys(Buffer.from("\u001b[13;2u")).events).toEqual([{ kind: "shift-enter", kittyOnly: true }]);
  expect(parseKeys(Buffer.from("\u001b[?8u")).events).toEqual([{ kind: "kitty-report", flags: 8 }]);
  const kitty = parseKeys(Buffer.from("\u001b[13;2u\u001b[115;6u\u001b[112;6u"));
  expect(kitty.events).toEqual([{ kind: "shift-enter", kittyOnly: true }, { kind: "save", kittyOnly: true }, { kind: "panel", kittyOnly: true }]);
  const text = parseKeys(Buffer.from("\u001b[97u\u001b[97;2;65u\u001b[49:33;2u"));
  expect(text.events).toEqual([{ kind: "text", text: "a" }, { kind: "text", text: "A" }, { kind: "text", text: "!" }]);
});

test("跨块 CSI、UTF-8、Esc 和退格不丢失也不拆坏字符", () => {
  const first = parseKeys(Buffer.from([0x1b, 0x5b, 0x31, 0x3b]));
  const second = parseKeys(Buffer.from("3D中"), first.state);
  expect(second.events).toEqual([{ kind: "word-left" }, { kind: "text", text: "中" }]);
  const utf8 = Buffer.from("🙂");
  const partial = parseKeys(utf8.subarray(0, 2));
  expect(parseKeys(utf8.subarray(2), partial.state).events).toEqual([{ kind: "text", text: "🙂" }]);
  const escape = parseKeys(Buffer.from([27]));
  expect(flushPendingKeys(escape.state).events).toEqual([{ kind: "escape" }]);
  expect(parseKeys(Buffer.from([8, 127, 4, 3])).events.map((key) => key.kind)).toEqual(["backspace", "backspace", "eof", "interrupt"]);
});

test("括号粘贴保留真实换行缩进，不把粘贴内容中的控制字节当编辑键", () => {
  const first = parseKeys(Buffer.from("\u001b[200~一\n  二\u001b[20"));
  expect(first.events).toEqual([]);
  const second = parseKeys(Buffer.from("1~\r"), first.state);
  expect(second.events).toEqual([{ kind: "paste", text: "一\n  二" }, { kind: "enter" }]);
  expect(parseKeys(Buffer.from("\r\n")).events).toEqual([{ kind: "enter" }]);
});

test("传统方向、Home/End、删除和 Alt 词操作稳定识别", () => {
  const events = parseKeys(Buffer.from("\u001b[A\u001b[B\u001b[C\u001b[D\u001b[H\u001b[F\u001b[3~\u001bb\u001bf\u001b\u007f")).events;
  expect(events.map((key) => key.kind)).toEqual(["up", "down", "right", "left", "home", "end", "delete", "word-left", "word-right", "kill-word"]);
  expect(events.at(-1)).toMatchObject({ kind: "kill-word", kittyOnly: true });
});

test("I1c 按键包含 Ins、Shift 选择、Ctrl 导航、Kitty 剪贴板和 SGR 鼠标", () => {
  const events = parseKeys(Buffer.from("\u001b[2~\u001b[1;2D\u001b[1;5D\u001b[1;5A\u001b[97;6u\u001b[99;6u\u001b[120;6u\u001b[<0;9;2M\u001b[<32;10;2M\u001b[<0;10;2m\u001b[<64;10;2M")).events;
  expect(events).toEqual([
    { kind: "insert" },
    { kind: "left", shift: true },
    { kind: "home" },
    { kind: "buffer-home" },
    { kind: "select-all", kittyOnly: true },
    { kind: "copy", kittyOnly: true },
    { kind: "cut", kittyOnly: true },
    { kind: "mouse", action: "press", button: 0, row: 2, column: 9 },
    { kind: "mouse", action: "drag", button: 0, row: 2, column: 10 },
    { kind: "mouse", action: "release", button: 0, row: 2, column: 10 },
    { kind: "mouse", action: "wheel", button: 0, row: 2, column: 10 },
  ]);
});
