/** 18A 优化级别与语言上下文的纯归一化边界。
 *
 * 这里不执行编译、优化或文件写入；它只把四层输入整理成协议可消费的
 * `OptimizationConfig` 和有效语言。13A 的 Rust 优化器仍是配置与指纹的语义来源。
 */

import type { DiagnosticConfig, OptimizationConfig } from "../protocol/messages.ts";
import { readConfig, type ConfigEditorOptions, type ConfigScope } from "./editor.ts";
import type { SupportedLocale } from "./locale.ts";

/** 已冻结的优化级别集合。 */
export type OptimizationLevel = 0 | 1 | 2 | 3;

/** 一层静态优化配置；未提供的字段不参与覆盖。 */
export interface OptimizationLayer {
  level?: OptimizationLevel;
  debugInfo?: boolean;
  sourceMap?: boolean;
  diagnosticEvents?: boolean;
  allowCpuSpecialization?: boolean;
  allowLto?: boolean;
  passSet?: readonly string[];
  experimentalPasses?: readonly string[];
  /** `[debug]` 的诊断输出配置；它本身不激活窗口。 */
  diagnostics?: DiagnosticConfig | null;
  locale?: SupportedLocale;
}

/** 归一化后的 CLI/协议输入。 */
export interface NormalizedOptimization {
  /** 规范化后的 O0--O3 级别。 */
  level: OptimizationLevel;
  /** 与 X0-A/Rust 共享的优化配置字段。 */
  config: OptimizationConfig;
  /** 四层优先级解析后的有效语言。 */
  locale: SupportedLocale;
  /** 与 13A 同名的静态字段，交给 18B 映射到完整后端配置。 */
  passSet: readonly string[];
  sourceMap: boolean;
  diagnosticEvents: boolean;
  allowCpuSpecialization: boolean;
  allowLto: boolean;
  experimentalPasses: readonly string[];
}

/** 优化配置错误；调用方读取稳定 code 和 details。 */
export class OptimizationConfigError extends Error {
  readonly code: string;
  readonly details: Record<string, unknown>;

  /** 创建带稳定编号和结构化参数的优化配置错误。 */
  constructor(code: string, message: string, details: Record<string, unknown> = {}) {
    super(`${code}: ${message}`);
    this.name = "OptimizationConfigError";
    this.code = code;
    this.details = details;
  }
}

/** 解析 `-O0` 至 `-O3`；其他写法统一拒绝。 */
export function parseOptimizationLevel(argument: string): OptimizationLevel {
  const match = /^-O([0-3])$/u.exec(argument);
  if (match === null) {
    throw new OptimizationConfigError("X11-CLI-OPT-001", `不支持的优化级别：${argument}`, { argument });
  }
  return Number(match[1]) as OptimizationLevel;
}

/** 将任意数值校验为冻结的优化级别。 */
export function normalizeOptimizationLevel(value: unknown, source = "optimization.level"): OptimizationLevel {
  if (value === 0 || value === 1 || value === 2 || value === 3) return value;
  throw new OptimizationConfigError("X11-CONFIG-007", `${source} 必须是 0、1、2 或 3`, { field: source, value });
}

/** 对列表执行与 13A 相同的去空白、去重和字典序规范化。 */
function normalizeList(values: readonly string[] | undefined): string[] {
  return [...new Set((values ?? []).map((value) => value.trim()).filter((value) => value.length > 0))].sort();
}

/** 按命令行 > 项目 > 全局 > 内建缺省的顺序生成稳定协议对象。 */
export function normalizeOptimization(
  global: OptimizationLayer = {},
  project: OptimizationLayer = {},
  cli: OptimizationLayer = {},
): NormalizedOptimization {
  const pick = <T>(key: keyof OptimizationLayer, fallback: T): T => {
    const values = [cli, project, global];
    for (const layer of values) {
      const value = layer[key] as T | undefined;
      if (value !== undefined) return value;
    }
    return fallback;
  };
  const level = normalizeOptimizationLevel(pick("level", 0));
  const locale = pick<SupportedLocale>("locale", "zh-CN");
  const passSet = normalizeList(pick("passSet", []));
  const experimentalPasses = normalizeList(pick("experimentalPasses", []));
  const sourceMap = Boolean(pick("sourceMap", true));
  const diagnosticEvents = Boolean(pick("diagnosticEvents", false));
  const allowCpuSpecialization = Boolean(pick("allowCpuSpecialization", false));
  const allowLto = Boolean(pick("allowLto", false));
  const diagnostics = mergeDiagnostics(global.diagnostics, project.diagnostics, cli.diagnostics);
  const config: OptimizationConfig = {
    level,
    debug: Boolean(pick("debugInfo", false)),
    diagnostics,
  };
  return Object.freeze({
    level,
    config: Object.freeze(config),
    locale,
    passSet,
    sourceMap,
    diagnosticEvents,
    allowCpuSpecialization,
    allowLto,
    experimentalPasses,
  });
}

/** 按命令行 > 项目 > 全局逐字段合并 `[debug]`，避免项目只覆盖一个输出项时丢失其余设置。 */
function mergeDiagnostics(...layers: readonly (DiagnosticConfig | null | undefined)[]): DiagnosticConfig | null {
  const result: DiagnosticConfig = {};
  let present = false;
  for (const layer of layers) {
    if (layer === null || layer === undefined) continue;
    present = true;
    if (layer.terminal_level !== undefined) result.terminal_level = layer.terminal_level;
    if (layer.file_level !== undefined) result.file_level = layer.file_level;
    if (layer.log_dir !== undefined) result.log_dir = layer.log_dir;
    if (layer.log_file !== undefined) result.log_file = layer.log_file;
    if (layer.stacktrace !== undefined) result.stacktrace = layer.stacktrace;
    if (layer.focus !== undefined) result.focus = layer.focus;
  }
  return present ? result : null;
}

/** 从声明式配置文本读取本批登记的 `[optimization]` 字段；不执行文本。 */
export function parseOptimizationLayer(text: string, path = "config.xiao"): OptimizationLayer {
  let table = "";
  const layer: OptimizationLayer = {};
  const lines = text.replaceAll("\r\n", "\n").replaceAll("\r", "\n").split("\n");
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const rawLine = lines[lineIndex];
    const line = rawLine.replace(/#.*/u, "").trim();
    if (line.length === 0) continue;
    const header = /^\[([^\]]+)\]$/u.exec(line);
    if (header !== null) {
      table = header[1].trim().toLocaleLowerCase("en-US");
      continue;
    }
    if (table === "debug") {
      const entry = /^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$/u.exec(line);
      if (entry === null) throw new OptimizationConfigError("X11-CONFIG-004", `配置行无法解析：${rawLine}`, { path });
      const [, key] = entry;
      let rawValue = entry[2];
      const allowed = ["terminal_level", "file_level", "log_dir", "log_file", "stacktrace", "focus"];
      if (!allowed.includes(key)) throw new OptimizationConfigError("X11-CONFIG-001", `debug 中不支持字段：${key}`, { path, field: key });
      if (key === "focus") {
        while (!rawValue.endsWith("]") && lineIndex + 1 < lines.length) {
          lineIndex += 1;
          rawValue += lines[lineIndex].trim();
        }
        if (!rawValue.startsWith("[") || !rawValue.endsWith("]")) throw new OptimizationConfigError("X11-CONFIG-002", "focus 必须是字典数组", { path, field: key });
        const focus = [...rawValue.matchAll(/\{([^{}]*)\}/gu)].map((match) => {
          const fields = match[1];
          const readString = (name: string): string | null => {
            const value = new RegExp(String.raw`${name}\s*=\s*"((?:[^"\\]|\\.)*)"`, "u").exec(fields)?.[1];
            return value === undefined ? null : value.replaceAll('\\"', '"').replaceAll("\\\\", "\\");
          };
          const output = readString("output");
          if (output === null || output.length === 0) throw new OptimizationConfigError("X11-CONFIG-002", "focus.output 必须是非空字符串", { path, field: key });
          const mirrorText = /mirror\s*=\s*(true|false)/u.exec(fields)?.[1];
          return { module: readString("module"), source: readString("source"), output, level: readString("level"), mirror: mirrorText === "true" };
        });
        if (focus.length === 0 && rawValue !== "[]") throw new OptimizationConfigError("X11-CONFIG-002", "focus 必须包含字典项", { path, field: key });
        const diagnostics = layer.diagnostics ?? {};
        diagnostics.focus = focus;
        layer.diagnostics = diagnostics;
        continue;
      }
      const match = /^(?:"((?:[^"\\]|\\.)*)"|'([^']*)')$/u.exec(rawValue);
      if (match === null) throw new OptimizationConfigError("X11-CONFIG-002", `${key} 必须是字符串`, { path, field: key });
      const value = (match[1] ?? match[2] ?? "").replaceAll('\\"', '"').replaceAll("\\\\", "\\");
      const diagnostics = layer.diagnostics ?? {};
      if (key === "terminal_level") diagnostics.terminal_level = value;
      if (key === "file_level") diagnostics.file_level = value;
      if (key === "log_dir") diagnostics.log_dir = value;
      if (key === "log_file") diagnostics.log_file = value;
      if (key === "stacktrace") diagnostics.stacktrace = value;
      layer.diagnostics = diagnostics;
      continue;
    }
    if (table !== "optimization") continue;
    const entry = /^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$/u.exec(line);
    if (entry === null) {
      throw new OptimizationConfigError("X11-CONFIG-004", `配置行无法解析：${rawLine}`, { path });
    }
    const [, key, rawValue] = entry;
    if (key === "level") {
      if (!/^\d+$/u.test(rawValue)) throw new OptimizationConfigError("X11-CONFIG-007", "optimization.level 必须是整数", { path, field: key });
      layer.level = normalizeOptimizationLevel(Number(rawValue), "optimization.level");
    } else if (["debug_info", "source_map", "diagnostic_events", "allow_cpu_specialization", "allow_lto"].includes(key)) {
      if (rawValue !== "true" && rawValue !== "false") throw new OptimizationConfigError("X11-CONFIG-002", `${key} 必须是布尔值`, { path, field: key });
      const value = rawValue === "true";
      if (key === "debug_info") layer.debugInfo = value;
      if (key === "source_map") layer.sourceMap = value;
      if (key === "diagnostic_events") layer.diagnosticEvents = value;
      if (key === "allow_cpu_specialization") layer.allowCpuSpecialization = value;
      if (key === "allow_lto") layer.allowLto = value;
    } else if (key === "pass_set" || key === "experimental_passes") {
      const values = [...rawValue.matchAll(/"([^"\\]*(?:\\.[^"\\]*)*)"/gu)].map((match) => match[1]);
      if (!rawValue.startsWith("[") || !rawValue.endsWith("]")) throw new OptimizationConfigError("X11-CONFIG-002", `${key} 必须是字符串数组`, { path, field: key });
      if (key === "pass_set") layer.passSet = values;
      else layer.experimentalPasses = values;
    } else {
      throw new OptimizationConfigError("X11-CONFIG-001", `optimization 中不支持字段：${key}`, { path, field: key });
    }
  }
  return layer;
}

/** 读取一层配置；缺少文件按空层处理，其他错误原样抛出。 */
export async function readOptimizationLayer(scope: ConfigScope, options: ConfigEditorOptions = {}): Promise<OptimizationLayer> {
  const { path, text } = await readConfig(scope, options);
  return parseOptimizationLayer(text, path);
}

/** 读取全局/项目配置并叠加一次性命令行覆盖。 */
export async function resolveOptimization(
  options: ConfigEditorOptions & { cli?: OptimizationLayer } = {},
): Promise<NormalizedOptimization> {
  const global = await readOptimizationLayer("global", options);
  const project = await readOptimizationLayer("project", options);
  return normalizeOptimization(global, project, options.cli);
}
