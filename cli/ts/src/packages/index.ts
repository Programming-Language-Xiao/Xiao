/** E2B 命令交互层；目标选择与锁文件判定均由 Rust 包模块完成。 */

import { readFile, stat } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";

import { CliArgumentError, type ParsedCommand } from "../commands/parser.ts";
import { findEnvironmentProjectRoot } from "../environments/index.ts";
import { requestActivation } from "../environments/activation.ts";
import { discoverToolchainWithMetadata } from "../platform/toolchain.ts";
import { hostTarget } from "../platform/core.ts";
import { ProtocolClient } from "../protocol/client.ts";
import { CORE_VERSION, PROTOCOL_VERSION, type PackageRequest, type ToolchainSpec } from "../protocol/messages.ts";
import { renderProtocolResponse, type RenderedDiagnostic, type DiagnosticRenderOptions } from "../diagnostics/render.ts";
import type { CommandContext } from "../commands/index.ts";

/** 安装只读显式目标或当前目录，不向上推断另一个项目。 */
async function installProjectRoot(cwd: string, project?: string): Promise<string> {
  const selected = resolve(cwd, project ?? ".");
  if (project === undefined) return selected;
  const entry = await stat(selected);
  if (entry.isDirectory()) return selected;
  if (entry.isFile() && basename(selected) === "config.xiao") return dirname(selected);
  throw new CliArgumentError("install/i 只接受项目目录或小写 config.xiao 路径");
}

/** 纯锁定或依赖编辑不调用原生编译器；保留协议必填字段的空形状。 */
const noBuildToolchain: ToolchainSpec = {
  clang: "", llvm_as: null, llc: null, runtime_library: null,
  native_static_libraries: [],
  versions: { clang: "", llvm_as: null, llc: null },
};

/** 执行包操作；别名 i 在解析后共享 install 分支。 */
export async function executePackageCommand(
  command: Extract<ParsedCommand, { kind: "sync" | "install" | "lock" | "update" | "add" | "remove" }>,
  context: CommandContext,
  renderOptions: DiagnosticRenderOptions,
): Promise<RenderedDiagnostic> {
  const cwd = context.cwd ?? process.cwd();
  const projectRoot = command.kind === "install"
    ? await installProjectRoot(cwd, command.project)
    : await findEnvironmentProjectRoot(cwd);
  const configText = await readFile(join(projectRoot, "config.xiao"), "utf8");
  const toolchain = command.kind === "sync" || command.kind === "install"
    ? context.environmentToolchain ?? (await discoverToolchainWithMetadata({
      cwd: projectRoot, env: context.env, executablePath: context.executablePath, probeLink: false,
    })).toolchain
    : noBuildToolchain;
  const request: PackageRequest = {
    type: "package", request_id: `package-${crypto.randomUUID()}`,
    protocol_version: PROTOCOL_VERSION, core_version: CORE_VERSION,
    operation: command.kind, project_root: projectRoot,
    active_environment: (context.env ?? process.env).XIAO_ACTIVE_ENV ?? null,
    config_text: configText, keep_extra: command.kind === "sync" && command.keepExtra,
    locked: command.kind === "sync" && command.locked,
    frozen: command.kind === "sync" && command.frozen,
    package_name: command.kind === "add" || command.kind === "remove" ? command.packageName : null,
    package_path: command.kind === "add" ? command.path : null,
    package_version: command.kind === "add" ? command.version ?? null : null,
    development: command.kind === "add" || command.kind === "remove" ? command.dev : false,
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
    stdout: `${({ sync: "已同步", install: "已安装", lock: "已锁定", update: "已更新锁文件", add: "已添加依赖", remove: "已移除依赖" })[command.kind]}：${response.result.environment_path}${instruction}\n`,
    stderr: "", exitCode: 0,
  };
}
