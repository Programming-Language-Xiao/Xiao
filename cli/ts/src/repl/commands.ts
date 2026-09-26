/** 多行控制指令只在独占逻辑行时分派，并从源码视图中剔除。 */

import { graphemes, type EditorState } from "./editor.ts";

/** I1b/I2/I3 分别消费的后续动作。 */
export type ControlCommand = "run" | "save" | "panel";

/** 精确匹配当前逻辑行的指令；普通源码不受影响。 */
export function dispatchControl(state: EditorState): { state: EditorState; command: ControlCommand } | null {
  const line = state.cursor.line;
  const command = ({ "!outLF!": "run", "!save!": "save", "!panel!": "panel" } as const)[state.lines[line] as "!outLF!"];
  if (command === undefined) return null;
  const lines = [...state.lines];
  lines.splice(line, 1);
  if (lines.length === 0) lines.push("");
  const cursorLine = Math.min(line, lines.length - 1);
  return {
    state: { ...state, lines, cursor: { line: cursorLine, column: graphemes(lines[cursorLine]).length }, preferredColumn: null },
    command,
  };
}
