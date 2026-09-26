/** 共享分派保证三个控制词都不污染最终源码或逻辑行号。 */

import { expect, test } from "bun:test";

import { dispatchControl } from "./commands.ts";
import { editorSource, initialEditorState, type EditorState } from "./editor.ts";

for (const [word, command] of [["!outLF!", "run"], ["!save!", "save"], ["!panel!", "panel"]] as const) {
  test(`${word} 独占行触发后从源码与行号映射剔除`, () => {
    const state: EditorState = { ...initialEditorState(), lines: ["code", word], cursor: { line: 1, column: word.length } };
    const result = dispatchControl(state);
    expect(result?.command).toBe(command);
    expect(result?.state.lines).toEqual(["code"]);
    expect(result && editorSource(result.state)).toBe("code");
    expect(state.lines).toEqual(["code", word]);
  });
}

test("普通源码、带空白的控制词不被识别；唯一控制行移除后保留空缓冲", () => {
  const initial = initialEditorState();
  expect(dispatchControl(initial)).toBeNull();
  const withSpace = { ...initial, lines: [" !outLF!"] };
  expect(dispatchControl(withSpace)).toBeNull();
  const only = { ...initial, lines: ["!outLF!"] };
  expect(dispatchControl(only)?.state.lines).toEqual([""]);
});
