/** 单行会话提示符；只读取传入的路径与环境名，不读取或执行环境包。 */

import { basename } from "node:path";

import { createColorizer, type ColorOptions } from "./color.ts";
import type { GitSummary } from "./git.ts";

/** 可注入的工作目录、环境与终端颜色能力。 */
export interface PromptOptions {
  cwd: string;
  activeEnvironment?: string | null;
  git?: GitSummary | null;
  color?: ColorOptions;
}

/** 与 11B 颜色表逐项对应的提示符片段。 */
type PromptColor = "environment" | "marker" | "main" | "master" | "dev" | "other" | "counts";

const COLORS: Record<PromptColor, { rgb: string; ansi256: number; ansi16: number }> = {
  environment: { rgb: "135;230;140", ansi256: 114, ansi16: 32 },
  marker: { rgb: "246;226;183", ansi256: 223, ansi16: 33 },
  main: { rgb: "208;170;252", ansi256: 183, ansi16: 35 },
  master: { rgb: "251;167;122", ansi256: 216, ansi16: 33 },
  dev: { rgb: "250;125;116", ansi256: 209, ansi16: 31 },
  other: { rgb: "126;192;252", ansi256: 117, ansi16: 36 },
  counts: { rgb: "183;205;170", ansi256: 151, ansi16: 32 },
};

/** 启动时只显示一次的版权与实际 CLI 版本。 */
export function renderReplBanner(version: string): string {
  return `Xiao (c) XiaoCZX\nV${version}\n`;
}

/** 在每次读取一行前生成提示符；不强制更改 Windows/Unix 的路径分隔符。 */
export function renderReplPrompt(options: PromptOptions): string {
  const parts: string[] = [];
  const environment = environmentName(options.activeEnvironment);
  if (environment !== null) parts.push(paint("environment", `$${environment}$`, options.color));
  parts.push(options.cwd);
  if (options.git !== null && options.git !== undefined) {
    const { branch, ahead, behind } = options.git;
    const role = branch === "main" || branch === "master" || branch === "dev" ? branch : "other";
    const counts = ahead === null || behind === null ? "" : paint("counts", `-${ahead}↑-${behind}↓`, options.color);
    parts.push(paint(role, branch, options.color) + counts);
  }
  parts.push(paint("marker", "[X>", options.color));
  return `${parts.join(" ")} `;
}

/** shell 激活协议只传路径，提示符只从路径的末段派生逻辑环境名。 */
function environmentName(path: string | null | undefined): string | null {
  if (!path) return null;
  const name = basename(path.replaceAll("\\", "/"));
  return name === "" ? null : name === ".venv" ? "venv" : name;
}

/** 复用 CLI 的能力判断，并按 24 位、256 色、16 色顺序降级。 */
function paint(role: PromptColor, text: string, options: ColorOptions = {}): string {
  const isTTY = options.isTTY ?? false;
  const term = options.term ?? "";
  const colorizer = createColorizer({
    mode: options.mode ?? "auto",
    isTTY,
    noColor: options.noColor ?? false,
    term,
    colorTerm: options.colorTerm ?? "",
  });
  if (!colorizer.enabled) return text;
  const color = COLORS[role];
  const sequence = colorizer.trueColor ? `38;2;${color.rgb}`
    : term.includes("256color") ? `38;5;${color.ansi256}` : String(color.ansi16);
  return `\u001b[${sequence}m${text}\u001b[39m`;
}
