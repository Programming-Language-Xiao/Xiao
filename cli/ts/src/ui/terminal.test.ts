/** Kitty 能力必须证实上报所有按键，传统终端保持组合键禁用。 */

import { expect, test } from "bun:test";

import { initialKeyboardProbe, keyboardReport, keyboardTimeout, KITTY_POP, KITTY_PUSH, KITTY_QUERY } from "./terminal.ts";

test("两步查询仅在确认 bit8 后开启 Shift+Enter", () => {
  expect(KITTY_PUSH).toBe("\u001b[>28u");
  expect(KITTY_POP).toBe("\u001b[<u");
  const queried = keyboardReport(initialKeyboardProbe(), 0);
  expect(queried.request).toBe(KITTY_PUSH + KITTY_QUERY);
  expect(queried.probe.kittyKeys).toBe(false);
  expect(keyboardReport(queried.probe, 28).probe).toMatchObject({ phase: "ready", kittyKeys: true });
  expect(keyboardReport(queried.probe, 4)).toMatchObject({ probe: { phase: "unsupported", kittyKeys: false }, request: KITTY_POP });
});

test("查询或确认超时均禁用组合键并恢复推入的终端状态", () => {
  expect(keyboardTimeout(initialKeyboardProbe())).toMatchObject({ probe: { kittyKeys: false, phase: "unsupported" }, request: "" });
  expect(keyboardTimeout(keyboardReport(initialKeyboardProbe(), 0).probe).request).toBe(KITTY_POP);
});
