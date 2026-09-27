/** I1a 可注入终端状态与 Kitty 键盘能力协商。 */

import type { ColorOptions } from "./color.ts";

/** 多行编辑器绘制所需的终端能力。 */
export interface TerminalView {
  width: number;
  height: number;
  isTTY: boolean;
  kittyKeys: boolean;
  color: ColorOptions;
}

/** 一轮 Kitty 探测的显式状态；默认始终认为修饰键不可区分。 */
export interface KeyboardProbe {
  phase: "query" | "verify" | "ready" | "unsupported";
  kittyKeys: boolean;
  pushed: boolean;
}

/** Kitty 键盘协议查询序列。 */
export const KITTY_QUERY = "\u001b[?u";
/** 推入旧键盘标志并启用所有按键及关联文字上报，Shift+Enter 才有独立编码。 */
export const KITTY_PUSH = "\u001b[>28u";
/** 撤销本会话推入的键盘模式。 */
export const KITTY_POP = "\u001b[<u";
/** 开启按键、按住拖动和 SGR 坐标鼠标上报。 */
export const MOUSE_ENABLE = "\u001b[?1000h\u001b[?1002h\u001b[?1006h";
/** 按与开启相反的顺序关闭鼠标上报。 */
export const MOUSE_DISABLE = "\u001b[?1006l\u001b[?1002l\u001b[?1000l";

/** 探测从禁用组合键开始。 */
export function initialKeyboardProbe(): KeyboardProbe {
  return { phase: "query", kittyKeys: false, pushed: false };
}

/** Kitty 回报经过二次确认后才开放组合键；旧终端的迟到回复不会改变状态。 */
export function keyboardReport(probe: KeyboardProbe, flags: number): { probe: KeyboardProbe; request: string } {
  if (probe.phase === "query") {
    return { probe: { phase: "verify", kittyKeys: false, pushed: true }, request: KITTY_PUSH + KITTY_QUERY };
  }
  if (probe.phase === "verify") {
    if ((flags & 8) !== 0) return { probe: { phase: "ready", kittyKeys: true, pushed: true }, request: "" };
    return { probe: { phase: "unsupported", kittyKeys: false, pushed: false }, request: KITTY_POP };
  }
  return { probe, request: "" };
}

/** 探测超时回到旧终端语义，并撤销可能已推入的 Kitty 模式。 */
export function keyboardTimeout(probe: KeyboardProbe): { probe: KeyboardProbe; request: string } {
  if (probe.phase === "ready" || probe.phase === "unsupported") return { probe, request: "" };
  return { probe: { phase: "unsupported", kittyKeys: false, pushed: false }, request: probe.pushed ? KITTY_POP : "" };
}
