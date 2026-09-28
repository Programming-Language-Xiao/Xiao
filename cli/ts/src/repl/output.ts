/** I1b 输出段只追加终端文本，不清屏或修改编辑缓冲区。 */

import { peakLiveBytes, type RenderedDiagnostic } from "../diagnostics/render.ts";
import type { ProtocolResponse } from "../protocol/messages.ts";
import type { TerminalView } from "../ui/terminal.ts";
import { separator } from "./confirm.ts";
import { cliMessage } from "../i18n.ts";
import type { SupportedLocale } from "../config/locale.ts";

/** 一次完整执行交给输出段的显示数据。 */
export interface RunDisplay {
  response: ProtocolResponse | null;
  rendered: RenderedDiagnostic;
  coreStderr: string;
  elapsedMs: number;
}

/** 将 LF 和 CRLF 统一为 raw mode 终端可正确回车的 CRLF。 */
export function terminalLines(text: string): string {
  if (text === "") return "";
  return `${text.replaceAll("\r\n", "\n").replaceAll("\r", "\n").replaceAll("\n", "\r\n").replace(/\r\n$/u, "")}\r\n`;
}

/** 写出上方分隔线；执行结果随后逐段追加，不在输出段清屏。 */
export async function beginOutput(
  view: TerminalView,
  output: NodeJS.WritableStream,
  write: (stream: NodeJS.WritableStream, text: string) => Promise<void>,
): Promise<void> {
  await write(output, `${separator(view)}\r\n`);
}

/** 追加核心结果、错误和收尾摘要；所有可见行都以 CRLF 结束。 */
export async function finishOutput(
  view: TerminalView,
  output: NodeJS.WritableStream,
  error: NodeJS.WritableStream,
  write: (stream: NodeJS.WritableStream, text: string) => Promise<void>,
  display: RunDisplay,
  locale?: SupportedLocale,
): Promise<void> {
  const value = display.response?.type === "result" && display.response.exit_code === 0
    ? display.response.value : null;
  if (isRecord(value) && value.kind !== "none") {
    const text = typeof value.text === "string" ? value.text : typeof value.value === "string" ? value.value : "";
    if (text !== "") await write(output, terminalLines(text));
  }
  await write(output, terminalLines(display.rendered.stdout));
  await write(error, terminalLines(display.rendered.stderr + display.coreStderr));
  await write(output, `${separator(view)}\r\n${runSummary(display.elapsedMs, display.response, locale)}\r\n`);
  await write(output, "\r\n".repeat(Math.max(1, Math.floor(view.height))));
}

/** 格式化本次耗时和可选 Runtime 峰值对象字节。 */
export function runSummary(elapsedMs: number, response: ProtocolResponse | null, locale?: SupportedLocale): string {
  const seconds = Math.max(0, elapsedMs) / 1000;
  const bytes = peakLiveBytes(response);
  const memory = bytes === null ? "?MB" : `${(bytes / 1_000_000).toFixed(2)}MB`;
  return `${cliMessage("xiao.cli.repl.summary.time", locale)}:${seconds.toFixed(4)} ${cliMessage("xiao.cli.repl.summary.memory", locale)}:${memory}`;
}

/** 只读取结构化对象字段，不根据本地化文本判断执行状态。 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
