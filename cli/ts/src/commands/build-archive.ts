/** 18B 归档构建路由；TS 只处理参数、协议与展示。 */

import { dirname, resolve } from "node:path";

import { resolveEffectiveLocale } from "../config/locale.ts";
import { resolveOptimization } from "../config/optimization.ts";
import { renderCliError, renderProtocolResponse, type DiagnosticRenderOptions, type RenderedDiagnostic } from "../diagnostics/render.ts";
import { ProtocolClient } from "../protocol/client.ts";
import { resolveInputPath } from "./input-routing.ts";
import type { CommandContext } from "./index.ts";
import type { ParsedCommand } from "./parser.ts";

/** 输入分类沿用 18A；项目入口、闭包、缓存与归档全部交给 Rust。 */
export async function executeArchiveBuild(command: Extract<ParsedCommand, { kind: "build" }>, context: CommandContext, rendering: DiagnosticRenderOptions): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  let locale = context.locale?.tag ?? "zh-CN";
  try {
    const input = await resolveInputPath([command.file], "build", cwd);
    const projectDirectory = dirname(input.path);
    locale = context.locale?.tag ?? (await resolveEffectiveLocale({ cwd: projectDirectory, env: context.env })).tag;
    const normalized = await resolveOptimization({
      cwd: projectDirectory,
      env: context.env,
      cli: {
        ...(command.optimizationExplicit ? { level: command.optimizationLevel } : {}),
        ...(command.options.debug ? { debugInfo: true } : {}),
        locale,
      },
    });
    const client = new ProtocolClient({ cwd, env: context.env, overridePath: context.corePath, executablePath: context.executablePath, spawnProcess: context.spawnProcess });
    const result = await client.buildArchive(input.path, {
      output: resolve(cwd, command.output),
      optimizationLevel: normalized.level,
      debug: command.options.debug,
      locale,
      signal: context.signal,
    });
    return renderProtocolResponse(result.response, { ...rendering, locale });
  } catch (error) {
    return renderCliError(error, { ...rendering, locale });
  }
}
