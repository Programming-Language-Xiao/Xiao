/** 追加式输出保留长文本、CRLF、峰值计量和旧协议缺字段降级。 */

import { expect, test } from "bun:test";
import { PassThrough } from "node:stream";

import type { ResultResponse } from "../protocol/messages.ts";
import type { TerminalView } from "../ui/terminal.ts";
import { beginOutput, finishOutput, runSummary, terminalLines } from "./output.ts";

const view: TerminalView = {
  width: 20, height: 3, isTTY: false, kittyKeys: false,
  color: { isTTY: false, noColor: true, term: "dumb", colorTerm: "" },
};

/** 聚合写入内容并保留真实分块边界。 */
function capture(): { stream: PassThrough; chunks: string[] } {
  const stream = new PassThrough();
  const chunks: string[] = [];
  stream.on("data", (chunk: Buffer) => chunks.push(chunk.toString()));
  return { stream, chunks };
}

/** 与实际驱动响应相同的最小运行结果。 */
function response(value: unknown, metrics: unknown): ResultResponse {
  return {
    type: "result", request_id: "run", operation: "run", exit_code: 0, exit_name: "success",
    diagnostics: [], report: null, events: [], metrics, value, artifact: null,
  };
}

test("输出段只追加分隔线、长输出和摘要，完全不清屏", async () => {
  const output = capture();
  const error = capture();
  const write = async (stream: NodeJS.WritableStream, text: string) => { stream.write(text); };
  await beginOutput(view, output.stream, write);
  const lines = Array.from({ length: 30 }, (_, index) => `line ${index}`).join("\n");
  await finishOutput(view, output.stream, error.stream, write, {
    response: response({ kind: "str", value: lines }, { peak_live_bytes: 12_345_678 }),
    rendered: { stdout: "", stderr: "", exitCode: 0 }, coreStderr: "", elapsedMs: 1250,
  });
  const printed = output.chunks.join("");
  expect(printed).not.toContain("\u001b[2J");
  expect(printed).toContain("line 0\r\n");
  expect(printed).toContain("line 29\r\n");
  expect(printed).toContain("time:1.2500 memory:12.35MB\r\n");
  expect(printed.endsWith("\r\n".repeat(view.height))).toBe(true);
  expect(error.chunks).toEqual([]);
});

test("raw mode 输出统一 CRLF，旧核心缺失内存字段不伪报为零", () => {
  expect(terminalLines("a\nb\r\nc")).toBe("a\r\nb\r\nc\r\n");
  expect(runSummary(0, response(null, {}))).toBe("time:0.0000 memory:?MB");
  expect(runSummary(0, response(null, {}), "zh-CN")).toBe("时间:0.0000 内存:?MB");
  expect(runSummary(0, response(null, { peak_live_bytes: Number.MAX_SAFE_INTEGER + 1 }))).toBe("time:0.0000 memory:?MB");
});
