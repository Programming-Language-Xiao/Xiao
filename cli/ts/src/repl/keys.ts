/** raw mode 字节流到编辑按键；分块、UTF-8 和括号粘贴均由显式状态承接。 */

/** 编辑器接受的按键；控制指令另由 commands.ts 分派。 */
export type KeyEvent =
  | { kind: "text" | "paste"; text: string }
  | { kind: "kitty-report"; flags: number }
  | { kind: "enter" | "shift-enter" | "backspace" | "delete" | "left" | "right" | "up" | "down"
      | "home" | "end" | "word-left" | "word-right" | "kill-word" | "kill-end" | "kill-start"
      | "yank" | "redraw" | "interrupt" | "eof" | "escape" | "save" | "panel" };

/** 前一块遗留的序列与尚未闭合的括号粘贴。 */
export interface KeyParserState {
  pending: Uint8Array;
  pasteChunks: readonly Uint8Array[];
  pasting: boolean;
  kittyKeys: boolean;
  skipNextLF: boolean;
}

/** 一次纯解析的结果。 */
export interface ParsedKeys {
  events: KeyEvent[];
  state: KeyParserState;
}

const PASTE_START = Buffer.from("\u001b[200~");
const PASTE_END = Buffer.from("\u001b[201~");
const NAVIGATION = { A: "up", B: "down", C: "right", D: "left", H: "home", F: "end" } as const;

/** 新建不猜测修饰键的传统终端状态。 */
export function initialKeyParserState(kittyKeys = false): KeyParserState {
  return { pending: new Uint8Array(), pasteChunks: [], pasting: false, kittyKeys, skipNextLF: false };
}

/** 只有完成 Kitty 能力协商后才允许解析组合键。 */
export function withKittyKeys(state: KeyParserState, enabled: boolean): KeyParserState {
  return { ...state, kittyKeys: enabled };
}

/** 解析一个任意大小的字节块，未闭合序列留给下一块。 */
export function parseKeys(bytes: Uint8Array, state: KeyParserState = initialKeyParserState()): ParsedKeys {
  const input = Buffer.concat([Buffer.from(state.pending), Buffer.from(bytes)]);
  const events: KeyEvent[] = [];
  const pasteChunks = [...state.pasteChunks];
  let pasting = state.pasting;
  let skipNextLF = state.skipNextLF;
  let cursor = 0;
  while (cursor < input.length) {
    if (pasting) {
      const end = input.indexOf(PASTE_END, cursor);
      if (end < 0) {
        const safeEnd = Math.max(cursor, input.length - PASTE_END.length + 1);
        if (safeEnd > cursor) pasteChunks.push(input.subarray(cursor, safeEnd));
        return { events, state: { ...state, pending: input.subarray(safeEnd), pasteChunks, pasting, skipNextLF } };
      }
      pasteChunks.push(input.subarray(cursor, end));
      events.push({ kind: "paste", text: Buffer.concat(pasteChunks).toString("utf8") });
      pasteChunks.length = 0;
      pasting = false;
      cursor = end + PASTE_END.length;
      continue;
    }
    const byte = input[cursor];
    if (skipNextLF) {
      skipNextLF = false;
      if (byte === 10) { cursor += 1; continue; }
    }
    if (byte === 27) {
      if (cursor + 1 >= input.length) break;
      if (input[cursor + 1] === 91) {
        let end = cursor + 2;
        while (end < input.length && !(input[end] >= 0x40 && input[end] <= 0x7e)) end += 1;
        if (end >= input.length && input.length - cursor < 64) break;
        if (end >= input.length) { cursor = input.length; continue; }
        const sequence = input.toString("ascii", cursor, end + 1);
        if (sequence === PASTE_START.toString("ascii")) {
          pasting = true;
        } else {
          const event = csiKey(sequence, state.kittyKeys);
          if (event !== null) events.push(event);
        }
        cursor = end + 1;
        continue;
      }
      if (input[cursor + 1] === 79) {
        if (cursor + 2 >= input.length) break;
        const event = NAVIGATION[String.fromCharCode(input[cursor + 2]) as keyof typeof NAVIGATION];
        if (event !== undefined) events.push({ kind: event });
        cursor += 3;
        continue;
      }
      const alt = input[cursor + 1];
      if (alt === 98 || alt === 66) events.push({ kind: "word-left" });
      else if (alt === 102 || alt === 70) events.push({ kind: "word-right" });
      else if (alt === 127 || alt === 8) events.push({ kind: "kill-word" });
      else events.push({ kind: "escape" });
      cursor += 2;
      continue;
    }
    const control = legacyKey(byte);
    if (control !== null) {
      events.push(control);
      if (byte === 13) skipNextLF = true;
      cursor += 1;
      continue;
    }
    if (byte < 32 || byte === 127) { cursor += 1; continue; }
    const length = byte < 128 ? 1 : byte >= 0xf0 ? 4 : byte >= 0xe0 ? 3 : byte >= 0xc0 ? 2 : 1;
    if (cursor + length > input.length) break;
    events.push({ kind: "text", text: input.toString("utf8", cursor, cursor + length) });
    cursor += length;
  }
  return { events, state: { ...state, pending: input.subarray(cursor), pasteChunks, pasting, skipNextLF } };
}

/** 将超时后独立的 Esc 作为无操作键消费，避免污染下一个输入。 */
export function flushPendingKeys(state: KeyParserState): ParsedKeys {
  if (state.pending.length === 1 && state.pending[0] === 27) {
    return { events: [{ kind: "escape" }], state: { ...state, pending: new Uint8Array() } };
  }
  return { events: [], state };
}

/** CSI u 仅在协商成功时启用组合键；终端能力报告始终可解析。 */
function csiKey(sequence: string, kittyKeys: boolean): KeyEvent | null {
  const report = /^\u001b\[\?(\d+)u$/u.exec(sequence);
  if (report !== null) return { kind: "kitty-report", flags: Number(report[1]) };
  const arrows = /^\u001b\[(?:1;([0-9]+))?([ABCDHF])$/u.exec(sequence);
  if (arrows !== null) {
    const key = NAVIGATION[arrows[2] as keyof typeof NAVIGATION];
    if (arrows[1] === "3" && (key === "left" || key === "right")) return { kind: key === "left" ? "word-left" : "word-right" };
    return { kind: key };
  }
  if (/^\u001b\[(?:3(?:;[0-9]+)?)~$/u.test(sequence)) return { kind: "delete" };
  if (/^\u001b\[(?:1|7)~$/u.test(sequence)) return { kind: "home" };
  if (/^\u001b\[(?:4|8)~$/u.test(sequence)) return { kind: "end" };
  if (!kittyKeys) return null;
  const match = /^\u001b\[(\d+)(?::(\d+)(?::\d+)?)?(?:;(\d+)(?::(\d+))?)?(?:;([\d:]+))?u$/u.exec(sequence);
  if (match === null || match[4] === "3") return null;
  const code = Number(match[1]);
  const modifier = Number(match[3] ?? "1");
  if (code === 13) return { kind: modifier === 2 ? "shift-enter" : "enter" };
  if (code === 127 && modifier === 3) return { kind: "kill-word" };
  if (code === 127) return { kind: "backspace" };
  if (code === 27) return { kind: "escape" };
  if (code === 9) return { kind: "text", text: "\t" };
  if (modifier === 3 && code === 98) return { kind: "word-left" };
  if (modifier === 3 && code === 102) return { kind: "word-right" };
  if (modifier === 6 && code === 115) return { kind: "save" };
  if (modifier === 6 && code === 112) return { kind: "panel" };
  if (modifier === 5) {
    const controls = ({ 97: "home", 99: "interrupt", 100: "eof", 101: "end", 107: "kill-end",
      108: "redraw", 117: "kill-start", 119: "kill-word", 121: "yank" } as const);
    const kind = controls[code as keyof typeof controls];
    if (kind !== undefined) return { kind };
  }
  if (modifier === 1 || modifier === 2) {
    const codepoints = match[5]?.split(":").map(Number)
      ?? [modifier === 2 && match[2] !== undefined ? Number(match[2]) : code];
    if (codepoints.every((point) => point >= 32 && point <= 0x10ffff)) {
      const text = String.fromCodePoint(...codepoints);
      return { kind: "text", text: modifier === 2 && match[5] === undefined && match[2] === undefined ? text.toUpperCase() : text };
    }
  }
  return null;
}

/** 传统终端的控制字符，Enter 永不推断为 Shift+Enter。 */
function legacyKey(byte: number): KeyEvent | null {
  const kind = ({ 1: "home", 3: "interrupt", 4: "eof", 5: "end", 8: "backspace", 10: "enter",
    11: "kill-end", 12: "redraw", 13: "enter", 21: "kill-start", 23: "kill-word", 25: "yank",
    127: "backspace" } as const)[byte as 1];
  if (kind !== undefined) return { kind };
  if (byte === 9) return { kind: "text", text: "\t" };
  return null;
}
