/** E2B 命令交互层；目标选择与锁文件判定均由 Rust 包模块完成。 */

import { readFile } from "node:fs/promises";
import { join } from "node:path";

import { findEnvironmentProjectRoot } from "../environments/index.ts";
import { requestActivation } from "../environments/activation.ts";
import { discoverToolchainWithMetadata } from "../platform/toolchain.ts";
import { hostTarget } from "../platform/core.ts";
import { ProtocolClient } from "../protocol/client.ts";
import { CORE_VERSION, PROTOCOL_VERSION, type PackageRequest } from "../protocol/messages.ts";
import { renderProtocolResponse, type RenderedDiagnostic, type DiagnosticRenderOptions } from "../diagnostics/render.ts";
import type { CommandContext } from "../commands/index.ts";
import type { ParsedCommand } from "../commands/parser.ts";

/** 执行一次 sync/install；别名 i 在解析后共享同一个分支。 */
export async function executePackageCommand(
  command: Extract<ParsedCommand, { kind: "sync" | "install" }>,
  context: CommandContext,
  renderOptions: DiagnosticRenderOptions,
): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  const projectRoot = await findEnvironmentProjectRoot(cwd);
  const configText = await readFile(join(projectRoot, "config.xiao"), "utf8");
  const toolchain = context.environmentToolchain ?? (await discoverToolchainWithMetadata({
    cwd: projectRoot, env: context.env, executablePath: context.executablePath, probeLink: false,
  })).toolchain;
  const request: PackageRequest = {
    type: "package", request_id: `package-${crypto.randomUUID()}`,
    protocol_version: PROTOCOL_VERSION, core_version: CORE_VERSION,
    operation: command.kind, project_root: projectRoot,
    active_environment: (context.env ?? process.env).XIAO_ACTIVE_ENV ?? null,
    config_text: configText, keep_extra: command.kind === "sync" && command.keepExtra,
    locked: command.kind === "sync" && command.locked,
    frozen: command.kind === "sync" && command.frozen,
    target: hostTarget(), toolchain,
  };
  const client = new ProtocolClient({
    cwd, env: context.env, overridePath: context.corePath,
    executablePath: context.executablePath, spawnProcess: context.spawnProcess,
  });
  const { response } = await client.call(request, context.signal);
  if (response.type !== "package_result") return renderProtocolResponse(response, renderOptions);
  if (response.result.activate) await requestActivation(response.result.environment_path, context.env ?? process.env);
  if (command.options.json) return {
    stdout: `${JSON.stringify({ type: "package_result", ...response.result })}\n`, stderr: "", exitCode: 0,
  };
  const instruction = response.result.activate && !(context.env ?? process.env).XIAO_ACTIVATION_FILE
    ? "（未检测到激活钩子；可手工设置 XIAO_ACTIVE_ENV 为以上绝对路径，或在 Bash/PowerShell 初始化 shell-init 钩子）" : "";
  return {
    stdout: `${command.kind === "sync" ? "已同步" : "已安装"}：${response.result.environment_path}${instruction}\n`,
    stderr: "", exitCode: 0,
  };
}
