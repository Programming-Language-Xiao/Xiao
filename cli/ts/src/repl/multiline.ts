/** I1a raw mode 编辑循环；只编辑和分派，不执行多行缓冲区。 */

import type { ReplContext } from "./session.ts";
import { dispatchControl, type ControlCommand } from "./commands.ts";
import { applyKey, initialEditorState, type EditorState } from "./editor.ts";
import { flushPendingKeys, initialKeyParserState, parseKeys, type KeyParserState } from "./keys.ts";
import { renderMultiline } from "./render.ts";
import { initialKeyboardProbe, keyboardReport, keyboardTimeout, KITTY_POP, KITTY_QUERY, type KeyboardProbe, type TerminalView } from "../ui/terminal.ts";

/** 可注入的 I1a 终端边界与后续批次的指令消费点。 */
export interface MultilineContext extends Pick<ReplContext, "input" | "output" | "error" | "write" | "env" | "isTTY" | "color" | "signal"> {
  terminalSize?: { width: number; height: number };
  onCommand?: (command: ControlCommand, state: EditorState) => void;
}

/** 多行退出时交还的编辑状态；I1b 可以在此状态上接确认界面。 */
export interface MultilineResult {
  exitCode: number;
  state: EditorState;
  commands: readonly ControlCommand[];
}

/** 原生 raw mode 不可用时的稳定诊断，不伪造非 TTY 编辑路径。 */
export class MultilineTerminalError extends Error {
  readonly code = "X11-CLI-REPL-002";
  readonly exitCode = 64;
  readonly details: Record<string, unknown> = {};

  /** 创建需要 raw mode 的稳定终端诊断。 */
  constructor() {
    super("X11-CLI-REPL-002: 多行编辑需要支持 raw mode 的交互终端");
  }
}

/** 建立逐字节循环，关闭时恢复 raw mode、键盘模式、粘贴模式与光标。 */
export async function runMultilineSession(context: MultilineContext): Promise<MultilineResult> {
  const input = context.input as NodeJS.ReadableStream & { setRawMode?: (mode: boolean) => void; isRaw?: boolean };
  if (typeof input.setRawMode !== "function") throw new MultilineTerminalError();
  const output = context.output as NodeJS.WritableStream & { columns?: number; rows?: number };
  const wasRaw = Boolean(input.isRaw);
  const wasPaused = input.isPaused();
  const commands: ControlCommand[] = [];
  let state = initialEditorState();
  let parser: KeyParserState = initialKeyParserState();
  let keyboard: KeyboardProbe = initialKeyboardProbe();
  let probeTimer: ReturnType<typeof setTimeout> | undefined;
  let escapeTimer: ReturnType<typeof setTimeout> | undefined;
  const view = (): TerminalView => ({
    width: context.terminalSize?.width ?? output.columns ?? 80,
    height: context.terminalSize?.height ?? output.rows ?? 24,
    isTTY: context.isTTY,
    kittyKeys: keyboard.kittyKeys,
    color: {
      isTTY: context.isTTY, mode: context.color, noColor: context.env.NO_COLOR !== undefined,
      term: context.env.TERM ?? "", colorTerm: context.env.COLORTERM ?? "",
    },
  });
  const redraw = async () => {
    const frame = renderMultiline(state, view());
    await context.write(output, `\u001b[?25l\u001b[H\u001b[2J${frame.text}\u001b[${frame.cursorRow};${frame.cursorColumn}H\u001b[?25h`);
  };
  const armProbeTimer = () => {
    if (probeTimer !== undefined) clearTimeout(probeTimer);
    probeTimer = setTimeout(() => {
      const expired = keyboardTimeout(keyboard);
      keyboard = expired.probe;
      if (expired.request !== "") output.write(expired.request);
    }, 150);
  };
  const emitCommand = (command: ControlCommand) => {
    commands.push(command);
    context.onCommand?.(command, state);
  };
  const consume = async (chunk: Buffer): Promise<number | null> => {
    if (escapeTimer !== undefined) clearTimeout(escapeTimer);
    const parsed = parseKeys(chunk, parser);
    parser = parsed.state;
    for (const key of parsed.events) {
      if (key.kind === "kitty-report") {
        const report = keyboardReport(keyboard, key.flags);
        keyboard = report.probe;
        if (report.request !== "") output.write(report.request);
        if (keyboard.phase === "verify") armProbeTimer();
        else if (probeTimer !== undefined) clearTimeout(probeTimer);
        continue;
      }
      if ("kittyOnly" in key && key.kittyOnly && !keyboard.kittyKeys) continue;
      if (key.kind === "enter") {
        const dispatched = dispatchControl(state);
        if (dispatched !== null) {
          state = dispatched.state;
          emitCommand(dispatched.command);
          continue;
        }
      }
      const result = applyKey(state, key);
      state = result.state;
      if (result.effect === "eof") return 0;
      if (result.effect === "interrupt") return 130;
      if (result.effect === "line-limit") {
        await context.write(context.error, "X11-REPL-LINES-001: 多行缓冲区不能超过 99999 行\n");
      }
      if (result.effect === "run" || result.effect === "save" || result.effect === "panel") emitCommand(result.effect);
    }
    if (parser.pending.length === 1 && parser.pending[0] === 27) {
      escapeTimer = setTimeout(() => { parser = flushPendingKeys(parser).state; }, 40);
    }
    if (parsed.events.length > 0) await redraw();
    return null;
  };
  let exitCode = 0;
  input.setRawMode(true);
  try {
    await context.write(output, "\u001b[?2004h" + KITTY_QUERY);
    await redraw();
    exitCode = await new Promise<number>((resolve, reject) => {
      let finished = false;
      let processing = Promise.resolve();
      const cleanup = () => {
        input.off("data", onData);
        input.off("end", onEnd);
        input.off("error", onError);
        output.off("resize", onResize);
        context.signal?.removeEventListener("abort", onAbort);
      };
      const fail = (error: Error) => {
        if (finished) return;
        finished = true;
        cleanup();
        reject(error);
      };
      const finish = (code: number) => {
        if (finished) return;
        finished = true;
        void processing.then(() => { cleanup(); resolve(code); }, (error: Error) => { cleanup(); reject(error); });
      };
      const onData = (chunk: Buffer) => {
        if (finished) return;
        processing = processing.then(async () => {
          const result = await consume(chunk);
          if (result !== null) finish(result);
        });
        void processing.catch(fail);
      };
      const onEnd = () => finish(0);
      const onError = (error: Error) => fail(error);
      const onAbort = () => finish(130);
      const onResize = () => {
        if (finished) return;
        processing = processing.then(redraw);
        void processing.catch(fail);
      };
      input.on("data", onData);
      input.once("end", onEnd);
      input.once("error", onError);
      output.on("resize", onResize);
      context.signal?.addEventListener("abort", onAbort, { once: true });
      if (context.signal?.aborted) onAbort();
      if (!finished) { armProbeTimer(); input.resume(); }
    });
  } finally {
    if (probeTimer !== undefined) clearTimeout(probeTimer);
    if (escapeTimer !== undefined) clearTimeout(escapeTimer);
    if (wasPaused) input.pause();
    else input.resume();
    input.setRawMode(wasRaw);
    if (keyboard.pushed) output.write(KITTY_POP);
    await context.write(output, "\u001b[?2004l\u001b[?25h\n");
  }
  return { exitCode, state, commands };
}
