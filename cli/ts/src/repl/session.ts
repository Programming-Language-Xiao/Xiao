/** I0 单行会话；每行通过现有 Rust run 协议执行，不维护第二套解释器。 */

import { createInterface } from "node:readline";

import { readConfigValue } from "../config/editor.ts";
import { renderCliError, renderProtocolResponse } from "../diagnostics/render.ts";
import { ProtocolClient, type CoreClientOptions } from "../protocol/client.ts";
import { probeGitSummary } from "../ui/git.ts";
import { renderReplBanner, renderReplPrompt } from "../ui/prompt.ts";

/** 标准流和配置/执行依赖由 CLI 入口传入，便于无终端集成验证。 */
export interface ReplContext {
  input: NodeJS.ReadableStream;
  output: NodeJS.WritableStream;
  error: NodeJS.WritableStream;
  write: (stream: NodeJS.WritableStream, text: string) => Promise<void>;
  cwd: string;
  env: NodeJS.ProcessEnv;
  isTTY: boolean;
  color: "auto" | "always" | "never";
  debug: boolean;
  version: string;
  signal?: AbortSignal;
  corePath?: CoreClientOptions["overridePath"];
  spawnProcess?: CoreClientOptions["spawnProcess"];
  executablePath?: string;
}

/** 启动、提交、执行、显示结果/错误并继续到 EOF；失败的一行不终止会话。 */
export async function runSingleLineRepl(context: ReplContext): Promise<number> {
  const { input, output, error, write, env } = context;
  await write(output, renderReplBanner(context.version));
  const firstPrompt = await currentPrompt(context);
  const terminal = context.isTTY && Boolean((input as NodeJS.ReadStream).isTTY);
  if (!terminal) await write(output, firstPrompt);
  const reader = createInterface({
    input,
    output: terminal ? output : undefined,
    terminal,
    crlfDelay: Infinity,
  });
  const lines = reader[Symbol.asyncIterator]();
  let closed = false;
  reader.on("close", () => { closed = true; });
  const onAbort = () => reader.close();
  reader.on("SIGINT", onAbort);
  context.signal?.addEventListener("abort", onAbort, { once: true });
  try {
    if (terminal) {
      reader.setPrompt(firstPrompt);
      reader.prompt();
    }
    while (!context.signal?.aborted) {
      const next = await lines.next();
      if (next.done) break;
      if (next.value.trim().length > 0) {
        try {
          const client = new ProtocolClient({
            cwd: context.cwd, env, overridePath: context.corePath,
            spawnProcess: context.spawnProcess, executablePath: context.executablePath,
          });
          const result = await client.runSource(next.value, { signal: context.signal });
          const rendered = renderProtocolResponse(result.response, {
            color: context.color, isTTY: context.isTTY,
            noColor: env.NO_COLOR !== undefined, term: env.TERM, colorTerm: env.COLORTERM,
          });
          if (result.response.type === "result" && result.response.exit_code === 0
            && typeof result.response.value === "object" && result.response.value !== null
            && "text" in result.response.value && typeof result.response.value.text === "string"
            && (!('kind' in result.response.value) || result.response.value.kind !== "none")) {
            await write(output, `${result.response.value.text}\n`);
          }
          await write(output, rendered.stdout);
          await write(error, rendered.stderr);
        } catch (failure) {
          const rendered = renderCliError(failure, {
            color: context.color, isTTY: context.isTTY,
            noColor: env.NO_COLOR !== undefined, term: env.TERM, colorTerm: env.COLORTERM,
          });
          await write(error, rendered.stderr);
        }
      }
      if (context.signal?.aborted) break;
      if (!terminal) await write(output, await currentPrompt(context));
      else if (!closed) {
        reader.setPrompt(await currentPrompt(context));
        if (!closed) reader.prompt();
      }
    }
  } finally {
    context.signal?.removeEventListener("abort", onAbort);
    reader.close();
  }
  return context.signal?.aborted ? 130 : 0;
}

/** 读取生效配置后重绘；Git 不可用时只在调试输出留下稳定原因。 */
async function currentPrompt(context: ReplContext): Promise<string> {
  let enabled = false;
  try {
    const config = { cwd: context.cwd, env: context.env };
    enabled = (await readConfigValue("project", "CLI.git.summary", config)
      ?? await readConfigValue("global", "CLI.git.summary", config)
      ?? false) === true;
  } catch (failure) {
    if (context.debug) await context.write(context.error, renderCliError(failure).stderr);
  }
  const result = enabled ? await probeGitSummary(context.cwd, { env: context.env }) : null;
  if (context.debug && result?.diagnostic != null && result.diagnostic.reason !== "not-repository") {
    await context.write(context.error, `${result.diagnostic.code}: Git 摘要已降级（${result.diagnostic.reason}）\n`);
  }
  return renderReplPrompt({
    cwd: context.cwd,
    activeEnvironment: context.env.XIAO_ACTIVE_ENV,
    git: result?.summary,
    color: {
      isTTY: context.isTTY,
      mode: context.color,
      noColor: context.env.NO_COLOR !== undefined,
      term: context.env.TERM ?? "",
      colorTerm: context.env.COLORTERM ?? "",
    },
  });
}
