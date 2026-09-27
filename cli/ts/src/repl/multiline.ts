/** 多行 raw mode 编辑、运行确认和追加式执行输出状态机。 */

import type { ReplContext } from "./session.ts";
import { ProtocolClient, type CoreCallResult, type SpawnCoreProcess } from "../protocol/client.ts";
import { renderCliError, renderProtocolResponse } from "../diagnostics/render.ts";
import { dispatchControl, type ControlCommand } from "./commands.ts";
import { applyKey, editorSource, initialEditorState, MAX_LOGICAL_LINES, positionCursor, selectRange, type EditorCursor, type EditorState } from "./editor.ts";
import { flushPendingKeys, initialKeyParserState, parseKeys, type KeyEvent, type KeyParserState } from "./keys.ts";
import { cursorFromRenderedPosition, renderMultiline } from "./render.ts";
import { renderConfirmation } from "./confirm.ts";
import { loadEditorFile, saveEditorFile, type ReplFileSystem } from "./file.ts";
import { beginOutput, finishOutput, terminalLines, type RunDisplay } from "./output.ts";
import { applySaveKey, initialSaveInput, renderSaveInput, wrapNotice } from "./save.ts";
import { initialKeyboardProbe, keyboardReport, keyboardTimeout, KITTY_POP, KITTY_PUSH, KITTY_QUERY, MOUSE_DISABLE, MOUSE_ENABLE, type KeyboardProbe, type TerminalView } from "../ui/terminal.ts";

/** 可注入的终端边界、核心进程和控制指令消费点。 */
export interface MultilineContext extends Pick<ReplContext, "input" | "output" | "error" | "write" | "env" | "isTTY" | "color" | "signal"> {
  terminalSize?: { width: number; height: number };
  onCommand?: (command: ControlCommand, state: EditorState) => void;
  cwd?: string;
  corePath?: string;
  spawnProcess?: SpawnCoreProcess;
  executablePath?: string;
  debug?: boolean;
  file?: string;
  fileSystem?: ReplFileSystem;
  executeSource?: (source: string, signal: AbortSignal) => Promise<CoreCallResult>;
}

/** 多行退出时交还的编辑状态与已触发控制指令。 */
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
  const setRawMode = input.setRawMode.bind(input);
  const output = context.output as NodeJS.WritableStream & { columns?: number; rows?: number };
  const wasRaw = Boolean(input.isRaw);
  const wasPaused = input.isPaused();
  const commands: ControlCommand[] = [];
  let state = initialEditorState();
  if (context.file !== undefined) {
    const loaded = await loadEditorFile(context.file, context.cwd ?? process.cwd(), context.fileSystem);
    state = { ...state, lines: loaded.lines, filePath: loaded.path };
  }
  let parser: KeyParserState = initialKeyParserState();
  let keyboard: KeyboardProbe = initialKeyboardProbe();
  let mode: "edit" | "confirm" | "save" | "output" = "edit";
  let confirmationReady = false;
  let saveReady = false;
  let saveInput = initialSaveInput();
  let editorNotice: string | null = null;
  let inputGeneration = 0;
  let mouseAnchor: EditorCursor | null = null;
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
    if (mode === "output") return;
    if (mode === "confirm") {
      await context.write(output, `\u001b[?25l\u001b[H\u001b[2J${renderConfirmation(view())}\u001b[2;2H\u001b[?25h`);
      confirmationReady = true;
      return;
    }
    if (mode === "save") {
      const frame = renderSaveInput(saveInput, view());
      await context.write(output, `\u001b[?25l\u001b[H\u001b[2J${frame.text}\u001b[${frame.cursorRow};${frame.cursorColumn}H\u001b[?25h`);
      saveReady = true;
      return;
    }
    const terminal = view();
    const maxNoticeRows = Math.max(0, Math.floor(terminal.height) - 1);
    const notices = editorNotice === null || maxNoticeRows === 0 ? [] : wrapNotice(editorNotice, terminal.width).slice(-maxNoticeRows);
    const frame = renderMultiline(state, { ...terminal, height: Math.max(1, terminal.height - notices.length) });
    const notice = notices.length === 0 ? "" : `\u001b[${terminal.height - notices.length + 1};1H${notices.join("\r\n")}`;
    await context.write(output, `\u001b[?25l\u001b[H\u001b[2J${frame.text}${notice}\u001b[${frame.cursorRow};${frame.cursorColumn}H\u001b[?25h`);
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
    if (command === "run") {
      mode = "confirm";
      confirmationReady = false;
      inputGeneration += 1;
    } else if (command === "save" && state.filePath === null) {
      mode = "save";
      saveInput = initialSaveInput();
      saveReady = false;
      editorNotice = null;
      inputGeneration += 1;
    }
  };
  const saveCurrent = async (path: string): Promise<boolean> => {
    try {
      const savedPath = await saveEditorFile(path, context.cwd ?? process.cwd(), editorSource(state), context.fileSystem);
      state = { ...state, filePath: savedPath };
      editorNotice = null;
      return true;
    } catch (failure) {
      const rendered = renderCliError(failure, {
        color: context.color, isTTY: context.isTTY,
        noColor: context.env.NO_COLOR !== undefined,
        term: context.env.TERM, colorTerm: context.env.COLORTERM,
      });
      await context.write(context.error, terminalLines(rendered.stderr));
      const message = failure instanceof Error ? failure.message : String(failure);
      if (mode === "save") saveInput = { ...saveInput, error: message };
      else editorNotice = message;
      return false;
    }
  };
  const handleSaveCommand = async () => {
    if (state.filePath !== null) {
      inputGeneration += 1;
      await saveCurrent(state.filePath);
      inputGeneration += 1;
    }
    await redraw();
  };
  const execute = async () => {
    inputGeneration += 1;
    await context.write(output, "\u001b[H\u001b[2J");
    mode = "output";
    if (keyboard.pushed) {
      output.write(KITTY_POP);
      keyboard = { ...keyboard, pushed: false };
    }
    await context.write(output, MOUSE_DISABLE + "\u001b[?2004l");
    setRawMode(false);
    const controller = new AbortController();
    const onInterrupt = () => controller.abort();
    const signal = context.signal === undefined
      ? controller.signal : AbortSignal.any([controller.signal, context.signal]);
    process.on("SIGINT", onInterrupt);
    const started = performance.now();
    let display: RunDisplay | null = null;
    try {
      await beginOutput(view(), output, context.write);
      const result = context.executeSource !== undefined
        ? await context.executeSource(editorSource(state), signal)
        : await new ProtocolClient({
          cwd: context.cwd, env: context.env, overridePath: context.corePath,
          spawnProcess: context.spawnProcess, executablePath: context.executablePath,
        }).runSource(editorSource(state), { debug: context.debug, signal });
      display = {
        response: result.response,
        rendered: renderProtocolResponse(result.response, {
          color: context.color, isTTY: context.isTTY,
          noColor: context.env.NO_COLOR !== undefined,
          term: context.env.TERM, colorTerm: context.env.COLORTERM,
        }),
        coreStderr: result.stderr,
        elapsedMs: performance.now() - started,
      };
    } catch (failure) {
      display = {
        response: null,
        rendered: renderCliError(failure, {
          color: context.color, isTTY: context.isTTY,
          noColor: context.env.NO_COLOR !== undefined,
          term: context.env.TERM, colorTerm: context.env.COLORTERM,
        }),
        coreStderr: "", elapsedMs: performance.now() - started,
      };
    } finally {
      process.off("SIGINT", onInterrupt);
      try {
        if (display !== null) await finishOutput(view(), output, context.error, context.write, display);
      } finally {
        setRawMode(true);
        await context.write(output, "\u001b[?2004h" + MOUSE_ENABLE + (keyboard.kittyKeys ? KITTY_PUSH : ""));
        if (keyboard.kittyKeys) keyboard = { ...keyboard, pushed: true };
        mode = "edit";
      }
    }
  };
  const consume = async (chunk: Buffer): Promise<number | null> => {
    if (escapeTimer !== undefined) clearTimeout(escapeTimer);
    const parsed = parseKeys(chunk, parser);
    parser = parsed.state;
    for (const parsedKey of parsed.events) {
      if (parsedKey.kind === "kitty-report") {
        const key = parsedKey;
        const report = keyboardReport(keyboard, key.flags);
        keyboard = report.probe;
        if (report.request !== "") output.write(report.request);
        if (keyboard.phase === "verify") armProbeTimer();
        else if (probeTimer !== undefined) clearTimeout(probeTimer);
        continue;
      }
      if (mode === "confirm") {
        if ("kittyOnly" in parsedKey && parsedKey.kittyOnly && !keyboard.kittyKeys) continue;
        if (parsedKey.kind === "enter") {
          confirmationReady = false;
          await execute();
          await redraw();
          return null;
        }
        else if (parsedKey.kind === "escape" || parsedKey.kind === "interrupt") {
          mode = "edit";
          confirmationReady = false;
        }
        continue;
      }
      if (mode === "save") {
        if ("kittyOnly" in parsedKey && parsedKey.kittyOnly && !keyboard.kittyKeys) continue;
        const result = applySaveKey(saveInput, parsedKey);
        saveInput = result.state;
        if (result.action === "cancel") {
          mode = "edit";
          saveReady = false;
          saveInput = initialSaveInput();
          inputGeneration += 1;
          await redraw();
          return null;
        }
        if (result.action === "submit") {
          saveReady = false;
          const saved = await saveCurrent(saveInput.text);
          inputGeneration += 1;
          if (saved) {
            mode = "edit";
            saveInput = initialSaveInput();
          }
          await redraw();
          return null;
        }
        continue;
      }
      if (parsedKey.kind === "mouse") {
        if (parsedKey.action === "release") {
          mouseAnchor = null;
          continue;
        }
        if (parsedKey.action === "wheel" || parsedKey.button !== 0) continue;
        const terminal = view();
        const noticeRows = editorNotice === null ? 0 : Math.min(Math.max(0, terminal.height - 1), wrapNotice(editorNotice, terminal.width).length);
        const frame = renderMultiline(state, { ...terminal, height: Math.max(1, terminal.height - noticeRows) });
        const position = cursorFromRenderedPosition(frame, state, parsedKey.row, parsedKey.column);
        if (position === null) continue;
        if (parsedKey.action === "press") {
          state = positionCursor(state, position);
          mouseAnchor = position;
        } else if (parsedKey.action === "drag") {
          mouseAnchor ??= state.cursor;
          state = selectRange(state, mouseAnchor, position);
        }
        continue;
      }
      const keys = parsedKey.kind === "paste" ? expandPaste(parsedKey.text, state) : [parsedKey];
      for (const key of keys) {
        if ("kittyOnly" in key && key.kittyOnly && !keyboard.kittyKeys) continue;
        if (key.kind === "enter") {
          const dispatched = dispatchControl(state);
          if (dispatched !== null) {
            state = dispatched.state;
            emitCommand(dispatched.command);
            if (dispatched.command === "run" || dispatched.command === "save") {
              if (dispatched.command === "save") await handleSaveCommand();
              else await redraw();
              return null;
            }
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
        if (result.effect === "run" || result.effect === "save") {
          if (result.effect === "save") await handleSaveCommand();
          else await redraw();
          return null;
        }
      }
    }
    if (parser.pending.length === 1 && parser.pending[0] === 27) {
      escapeTimer = setTimeout(() => {
        parser = flushPendingKeys(parser).state;
        if (mode === "confirm" || mode === "save") {
          mode = "edit";
          confirmationReady = false;
          saveReady = false;
          saveInput = initialSaveInput();
          inputGeneration += 1;
          void redraw().catch(failSession);
        }
      }, 40);
    }
    if (parsed.events.length > 0) await redraw();
    return null;
  };
  let exitCode = 0;
  let failSession: (error: Error) => void = () => undefined;
  try {
    setRawMode(true);
    await context.write(output, "\u001b[?2004h" + MOUSE_ENABLE + KITTY_QUERY);
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
      failSession = fail;
      const finish = (code: number) => {
        if (finished) return;
        finished = true;
        void processing.then(() => { cleanup(); resolve(code); }, (error: Error) => { cleanup(); reject(error); });
      };
      const onData = (chunk: Buffer) => {
        if (finished || mode === "output" || (mode === "confirm" && !confirmationReady) || (mode === "save" && !saveReady)) return;
        const generation = inputGeneration;
        processing = processing.then(async () => {
          if (finished || generation !== inputGeneration) return;
          const result = await consume(chunk);
          if (result !== null) finish(result);
        });
        void processing.catch(fail);
      };
      const onEnd = () => finish(0);
      const onError = (error: Error) => fail(error);
      const onAbort = () => finish(130);
      const onResize = () => {
        if (finished || mode === "output") return;
        processing = processing.then(async () => {
          if (!finished) await redraw();
        });
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
    setRawMode(wasRaw);
    if (keyboard.pushed) output.write(KITTY_POP);
    await context.write(output, MOUSE_DISABLE + "\u001b[?2004l\u001b[?25h\n");
  }
  return { exitCode, state, commands };
}

/** 把粘贴中的真实换行按 Enter 语义交给控制指令分派，同时保留整次插入的行数原子性。 */
function expandPaste(text: string, state: EditorState): KeyEvent[] {
  const normalized = text.replaceAll("\r\n", "\n").replaceAll("\r", "\n");
  const pieces = normalized.split("\n");
  if (state.lines.length + pieces.length - 1 > MAX_LOGICAL_LINES) return [{ kind: "paste", text }];
  const events: KeyEvent[] = [];
  for (let index = 0; index < pieces.length; index += 1) {
    if (pieces[index] !== "") events.push({ kind: "paste", text: pieces[index] });
    if (index + 1 < pieces.length) events.push({ kind: "enter" });
  }
  return events;
}
