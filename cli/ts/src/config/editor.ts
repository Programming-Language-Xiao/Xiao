/** `config.xiao` 的 CLI 结构化写入边界。
 *
 * 这里保留原文而不是把配置重新格式化：配置文件同时承担人类注释和项目
 * 文档的职责，CLI 只修改目标键的字面量。配置语义的完整校验仍由 Rust
 * `xiao-config` 负责，本模块只处理已经冻结的两个 CLI 写入路径。
 */

import { chmod, mkdir, readFile, readdir, rename, rm, stat, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join, parse, resolve } from "node:path";

/** 支持写入的配置路径。 */
export type SupportedConfigKey = "CLI.git.summary" | "language.locale";

/** 配置目标范围。 */
export type ConfigScope = "project" | "global";

/** 配置读取/写入的选项。 */
export interface ConfigEditorOptions {
  /** 项目查找的起点目录。 */
  cwd?: string;
  /** 覆盖全局配置路径，便于测试和嵌入式宿主。 */
  globalPath?: string;
  /** 进程环境；默认使用当前环境。 */
  env?: NodeJS.ProcessEnv;
  /** 文件系统主机；默认使用 Node/Bun 文件系统。 */
  fs?: ConfigFileSystem;
}

/** 配置写回所需的最小文件系统接口。 */
export interface ConfigFileSystem {
  readFile(path: string, encoding: "utf8"): Promise<string>;
  writeFile(path: string, data: string, encoding: "utf8"): Promise<void>;
  rename(oldPath: string, newPath: string): Promise<void>;
  rm(path: string, options?: { force?: boolean }): Promise<void>;
  mkdir(path: string, options: { recursive: true }): Promise<void>;
  stat(path: string): Promise<{ mode: number }>;
  readdir(path: string): Promise<string[]>;
  chmod(path: string, mode: number): Promise<void>;
}

/** 成功写入的结果。 */
export interface ConfigWriteResult {
  /** 实际写入的配置文件。 */
  path: string;
  /** 写回后的完整文档。 */
  text: string;
  /** 规范化后的值。 */
  value: boolean | "zh-CN" | "en-US";
}

/** 结构化配置错误；调用方应读取 `code`，不要解析错误文本。 */
export class CliConfigError extends Error {
  /** 稳定机器错误码。 */
  readonly code: string;
  /** 相关配置路径。 */
  readonly path: string | null;
  /** 结构化附加字段。 */
  readonly details: Record<string, unknown>;

  /** 创建配置错误。 */
  constructor(code: string, message: string, path: string | null = null, details: Record<string, unknown> = {}) {
    super(`${code}: ${message}`);
    this.name = "CliConfigError";
    this.code = code;
    this.path = path;
    this.details = details;
  }
}

const DEFAULT_ENV = process.env;
const CANONICAL_FILE = "config.xiao";
const CONFIG_ERROR = {
  invalidKey: "X11-CONFIG-001",
  invalidValue: "X11-CONFIG-002",
  missing: "X11-CONFIG-003",
  malformed: "X11-CONFIG-004",
  nonCanonical: "X11-CONFIG-005",
  write: "X11-CONFIG-006",
} as const;

const nativeFs: ConfigFileSystem = {
  readFile: (path, encoding) => readFile(path, encoding),
  writeFile: (path, data, encoding) => writeFile(path, data, encoding),
  rename,
  rm,
  mkdir: (path, options) => mkdir(path, options).then(() => undefined),
  stat,
  readdir,
  chmod,
};

/** 返回受支持的配置键清单。 */
export function supportedConfigKeys(): readonly SupportedConfigKey[] {
  return ["CLI.git.summary", "language.locale"];
}

/** 校验并规范化配置值。 */
export function normalizeConfigValue(key: SupportedConfigKey, raw: string): boolean | "zh-CN" | "en-US" {
  if (key === "CLI.git.summary") {
    if (raw === "true") return true;
    if (raw === "false") return false;
    throw new CliConfigError(CONFIG_ERROR.invalidValue, "CLI.git.summary 只接受 true 或 false", null, { key, value: raw });
  }
  if (raw === "zh" || raw === "zh-CN") return "zh-CN";
  if (raw === "en" || raw === "en-US") return "en-US";
  throw new CliConfigError(CONFIG_ERROR.invalidValue, "language.locale 只接受 zh、zh-CN、en 或 en-US", null, { key, value: raw });
}

/** 查找项目级规范配置文件；只认小写 `config.xiao`。 */
export async function findProjectConfig(cwd = process.cwd(), fileSystem: ConfigFileSystem = nativeFs): Promise<string | null> {
  let current = resolve(cwd);
  while (true) {
    const entries = await readDirectory(current, fileSystem);
    const exact = entries.find((name) => name === CANONICAL_FILE);
    if (exact !== undefined) return join(current, exact);
    const nonCanonical = entries.find((name) => name.toLowerCase() === CANONICAL_FILE);
    if (nonCanonical !== undefined) throw new CliConfigError(CONFIG_ERROR.nonCanonical, `只识别小写 ${CANONICAL_FILE}，发现 ${nonCanonical}`, join(current, nonCanonical));
    const parent = parse(current).root === current ? null : dirname(current);
    if (parent === null || parent === current) return null;
    current = parent;
  }
}

/** 计算全局配置路径；`XIAO_GLOBAL_CONFIG` 优先用于测试和受控宿主。 */
export function globalConfigPath(options: ConfigEditorOptions = {}): string {
  const env = options.env ?? DEFAULT_ENV;
  const base = options.cwd ?? process.cwd();
  if (options.globalPath) return resolve(base, options.globalPath);
  if (env.XIAO_GLOBAL_CONFIG?.trim()) return resolve(base, env.XIAO_GLOBAL_CONFIG);
  if (process.platform === "win32") return join(env.APPDATA ?? join(homedir(), "AppData", "Roaming"), "Xiao", CANONICAL_FILE);
  if (process.platform === "darwin") return join(env.HOME ?? homedir(), "Library", "Application Support", "Xiao", CANONICAL_FILE);
  return join(env.XDG_CONFIG_HOME ?? join(env.HOME ?? homedir(), ".config"), "xiao", CANONICAL_FILE);
}

/** 解析项目或全局配置目标；项目不存在时返回待创建路径。 */
export async function resolveConfigPath(scope: ConfigScope, options: ConfigEditorOptions = {}): Promise<string> {
  const fileSystem = options.fs ?? nativeFs;
  if (scope === "global") return globalConfigPath(options);
  const existing = await findProjectConfig(options.cwd ?? process.cwd(), fileSystem);
  return existing ?? join(resolve(options.cwd ?? process.cwd()), CANONICAL_FILE);
}

/** 读取配置文件；不存在时返回空文档，写入命令会建立规范表。 */
export async function readConfig(scope: ConfigScope, options: ConfigEditorOptions = {}): Promise<{ path: string; text: string }> {
  const fileSystem = options.fs ?? nativeFs;
  const path = await resolveConfigPath(scope, options);
  try {
    return { path, text: await fileSystem.readFile(path, "utf8") };
  } catch (error) {
    if (isMissing(error)) return { path, text: "" };
    throw new CliConfigError(CONFIG_ERROR.missing, `无法读取配置文件：${String(error)}`, path);
  }
}

/** 修改一个已冻结的配置路径，并采用同目录临时文件原子替换。 */
export async function writeConfigValue(
  scope: ConfigScope,
  key: string,
  rawValue: string,
  options: ConfigEditorOptions = {},
): Promise<ConfigWriteResult> {
  if (key !== "CLI.git.summary" && key !== "language.locale") {
    throw new CliConfigError(CONFIG_ERROR.invalidKey, `不支持的配置路径：${key}`, null, { key });
  }
  const typedKey = key as SupportedConfigKey;
  const value = normalizeConfigValue(typedKey, rawValue);
  const fileSystem = options.fs ?? nativeFs;
  const { path, text } = await readConfig(scope, options);
  validateDocumentShape(text, path);
  const updated = updateDocument(text, typedKey, value, path);
  await atomicWrite(path, updated, fileSystem);
  return { path, text: updated, value };
}

/** 用同一个结构化入口读取当前配置值（未设置时返回空）。 */
export async function readConfigValue(
  scope: ConfigScope,
  key: SupportedConfigKey,
  options: ConfigEditorOptions = {},
): Promise<boolean | "zh-CN" | "en-US" | undefined> {
  const { text } = await readConfig(scope, options);
  const found = findValue(text, key);
  if (found === null) return undefined;
  return normalizeConfigValue(key, found);
}

/** 通过同目录临时文件和回滚备份完成原子替换。 */
async function atomicWrite(path: string, text: string, fileSystem: ConfigFileSystem): Promise<void> {
  const directory = dirname(path);
  try {
    await fileSystem.mkdir(directory, { recursive: true });
    let mode: number | undefined;
    try {
      mode = (await fileSystem.stat(path)).mode;
    } catch (error) {
      if (!isMissing(error)) throw error;
    }
    const temp = join(directory, `.${CANONICAL_FILE}.tmp-${process.pid}-${Date.now()}-${Math.random().toString(36).slice(2)}`);
    try {
      await fileSystem.writeFile(temp, text, "utf8");
      if (mode !== undefined) await fileSystem.chmod(temp, mode);
      try {
        await fileSystem.rename(temp, path);
      } catch (error) {
        // Windows 不允许直接覆盖现有文件；备份/替换失败时尽量回滚原文件。
        if (mode === undefined || !isAlreadyExists(error)) throw error;
        const backup = `${path}.xiao-backup-${process.pid}-${Date.now()}`;
        await fileSystem.rename(path, backup);
        try {
          await fileSystem.rename(temp, path);
          await fileSystem.rm(backup, { force: true });
        } catch (replaceError) {
          try { await fileSystem.rename(backup, path); } catch { /* 保留原始错误 */ }
          throw replaceError;
        }
      }
    } finally {
      await fileSystem.rm(temp, { force: true }).catch(() => undefined);
    }
  } catch (error) {
    if (error instanceof CliConfigError) throw error;
    throw new CliConfigError(CONFIG_ERROR.write, `原子写入配置失败：${String(error)}`, path);
  }
}

/** 在保留原始布局的前提下更新目标表和键。 */
function updateDocument(text: string, key: SupportedConfigKey, value: boolean | "zh-CN" | "en-US", path: string): string {
  const newline = text.includes("\r\n") ? "\r\n" : "\n";
  const hadTrailingNewline = text.endsWith("\n") || text.endsWith("\r");
  const lines = splitLines(text);
  const section = key === "CLI.git.summary" ? "cli" : "language";
  const leaf = key === "CLI.git.summary" ? "summary" : "locale";
  const parent = key === "CLI.git.summary" ? "git" : null;
  const sectionRange = locateSection(lines, section);
  if (sectionRange === null) {
    const existingText = lines.map((line) => line.raw).join("");
    const prefix = existingText.length > 0 && !existingText.endsWith(newline) ? newline : "";
    const block = parent === null
      ? `[language]${newline}locale = ${serializeValue(value)}${newline}`
      : `[CLI]${newline}git = { summary = ${serializeValue(value)} }${newline}`;
    return `${existingText}${prefix}${block}`;
  }

  const [start, end] = sectionRange;
  if (parent === null) {
    const direct = findAssignment(lines, start + 1, end, leaf);
    if (direct !== null) {
      lines[direct.line].content = replaceScalar(lines[direct.line].content, direct.valueStart, direct.valueEnd, serializeValue(value));
      return joinLines(lines, hadTrailingNewline, newline);
    }
    lines.splice(end, 0, makeLine(`locale = ${serializeValue(value)}`, newline));
    return joinLines(lines, hadTrailingNewline, newline);
  }

  const git = findAssignment(lines, start + 1, end, parent);
  if (git !== null) {
    const replacement = replaceDictionaryEntry(lines, git.line, leaf, serializeValue(value));
    if (replacement !== null) {
      lines.splice(git.line, replacement.deleteCount, ...replacement.lines);
      return joinLines(lines, hadTrailingNewline, newline);
    }
    throw new CliConfigError(CONFIG_ERROR.malformed, `配置表 ${parent} 的字典值无法识别`, path);
  }
  lines.splice(end, 0, makeLine(`git = { summary = ${serializeValue(value)} }`, newline));
  return joinLines(lines, hadTrailingNewline, newline);
}

/** 在写回前检查引号和静态容器是否闭合，避免把损坏文档继续传播。 */
function validateDocumentShape(text: string, path: string): void {
  let quote: string | null = null;
  let escaped = false;
  let braces = 0;
  let brackets = 0;
  for (const line of splitLines(text)) {
    const source = stripComment(line.content);
    for (const character of source) {
      if (quote !== null) {
        if (escaped) escaped = false;
        else if (character === "\\") escaped = true;
        else if (character === quote) quote = null;
        continue;
      }
      if (character === "\"" || character === "'") { quote = character; continue; }
      if (character === "{") braces += 1;
      else if (character === "}") braces -= 1;
      else if (character === "[") brackets += 1;
      else if (character === "]") brackets -= 1;
      if (braces < 0 || brackets < 0) throw new CliConfigError(CONFIG_ERROR.malformed, "配置容器闭合顺序无效", path);
    }
  }
  if (quote !== null || braces !== 0 || brackets !== 0 || escaped) throw new CliConfigError(CONFIG_ERROR.malformed, "配置包含未闭合的字符串或容器", path);
}

/** 一行文本及其原始换行信息。 */
interface Line { content: string; raw: string; ending: string }

/** 按物理换行拆分文档，并保留 CRLF/LF 形状。 */
function splitLines(text: string): Line[] {
  if (text.length === 0) return [];
  const lines: Line[] = [];
  let start = 0;
  for (let index = 0; index < text.length; index += 1) {
    if (text[index] !== "\n" && text[index] !== "\r") continue;
    const ending = text[index] === "\r" && text[index + 1] === "\n" ? "\r\n" : text[index];
    const end = index + ending.length;
    lines.push({ content: text.slice(start, index), raw: text.slice(start, end), ending });
    start = end;
    if (ending.length === 2) index += 1;
  }
  if (start < text.length) lines.push({ content: text.slice(start), raw: text.slice(start), ending: "" });
  return lines;
}

/** 创建带指定换行符的新文档行。 */
function makeLine(content: string, newline: string): Line { return { content, raw: `${content}${newline}`, ending: newline }; }

/** 重新组合行并恢复无尾换行的原始状态。 */
function joinLines(lines: Line[], hadTrailingNewline: boolean, newline: string): string {
  let result = lines.map((line) => `${line.content}${line.ending || newline}`).join("");
  if (!hadTrailingNewline && result.endsWith(newline)) result = result.slice(0, -newline.length);
  return result;
}

/** 找到一个顶层表的起止行。 */
function locateSection(lines: Line[], name: string): [number, number] | null {
  let start = -1;
  for (let index = 0; index < lines.length; index += 1) {
    const header = parseHeader(lines[index].content);
    if (header === name) {
      start = index;
      break;
    }
  }
  if (start < 0) return null;
  let end = lines.length;
  for (let index = start + 1; index < lines.length; index += 1) {
    if (parseHeader(lines[index].content) !== null) { end = index; break; }
  }
  return [start, end];
}

/** 读取顶层表头并按配置规则规范化大小写。 */
function parseHeader(content: string): string | null {
  const clean = stripComment(content).trim();
  if (!clean.startsWith("[") || !clean.endsWith("]")) return null;
  const name = clean.slice(1, -1).trim();
  if (name.length === 0 || name.includes(".")) return null;
  return name.toLowerCase();
}

/** 一条赋值在原始行中的值区间。 */
interface Assignment { line: number; valueStart: number; valueEnd: number }

/** 扫描指定表中的顶层键值赋值。 */
function findAssignment(lines: Line[], start: number, end: number, key: string): Assignment | null {
  for (let line = start; line < end; line += 1) {
    const source = lines[line].content;
    const clean = stripComment(source);
    const match = /^\s*([A-Za-z_][A-Za-z0-9_]*|`[^`]*`)\s*=/.exec(clean);
    if (!match || match[1].toLowerCase() !== key.toLowerCase()) continue;
    const equals = clean.indexOf("=", match.index + match[0].indexOf("="));
    const valueStart = equals + 1 + countLeadingSpace(clean.slice(equals + 1));
    const valueEnd = findValueEnd(clean, valueStart);
    return { line, valueStart, valueEnd };
  }
  return null;
}

/** 在内联或多行字典中替换一个成员值。 */
function replaceDictionaryEntry(lines: Line[], line: number, key: string, value: string): { deleteCount: number; lines: Line[] } | null {
  let depth = 0;
  let quote: string | null = null;
  let escaped = false;
  let endLine = line;
  for (let index = line; index < lines.length; index += 1) {
    const content = stripComment(lines[index].content);
    for (let cursor = 0; cursor < content.length; cursor += 1) {
      const character = content[cursor];
      if (quote !== null) {
        if (escaped) escaped = false;
        else if (character === "\\") escaped = true;
        else if (character === quote) quote = null;
        continue;
      }
      if (character === "\"" || character === "'") { quote = character; continue; }
      if (character === "{") depth += 1;
      if (character === "}") { depth -= 1; if (depth === 0) { endLine = index; break; } }
    }
    if (depth === 0 && index >= line) break;
    endLine = index;
  }
  if (depth !== 0) return null;
  for (let index = line; index <= endLine; index += 1) {
    const clean = stripComment(lines[index].content);
    const expression = new RegExp(`(^|[,\\{])\\s*${escapeRegExp(key)}\\s*=`, "i").exec(clean);
    if (!expression) continue;
    const equals = clean.indexOf("=", expression.index + expression[0].lastIndexOf("="));
    const valueStart = equals + 1 + countLeadingSpace(clean.slice(equals + 1));
    const valueEnd = findValueEnd(clean, valueStart);
    lines[index].content = replaceScalar(lines[index].content, valueStart, valueEnd, value);
    return { deleteCount: 0, lines: [] };
  }
  const closing = lines[endLine].content.lastIndexOf("}");
  if (closing < 0) return null;
  const before = lines[endLine].content.slice(0, closing).replace(/\s*$/, "");
  const after = lines[endLine].content.slice(closing);
  lines[endLine].content = `${before}${before.endsWith("{") ? "" : ", "}${key} = ${value} ${after}`;
  return { deleteCount: 0, lines: [] };
}

/** 从原文读取一个已支持配置键的字面量文本。 */
function findValue(text: string, key: SupportedConfigKey): string | null {
  const lines = splitLines(text);
  const section = key === "CLI.git.summary" ? "cli" : "language";
  const leaf = key === "CLI.git.summary" ? "summary" : "locale";
  const parent = key === "CLI.git.summary" ? "git" : null;
  const range = locateSection(lines, section);
  if (range === null) return null;
  const assignment = parent === null ? findAssignment(lines, range[0] + 1, range[1], leaf) : findAssignment(lines, range[0] + 1, range[1], parent);
  if (assignment === null) return null;
  if (parent === null) return stripComment(lines[assignment.line].content).slice(assignment.valueStart, assignment.valueEnd).trim().replace(/^(["'])(.*)\1$/s, "$2");
  for (let line = assignment.line; line < range[1]; line += 1) {
    const dictionaryText = stripComment(lines[line].content);
    const match = new RegExp(`(?:^|[,\\{])\\s*${escapeRegExp(leaf)}\\s*=\\s*([^,}]+)`, "i").exec(dictionaryText);
    if (match) return match[1].trim().replace(/^(["'])(.*)\1$/s, "$2");
    if (dictionaryText.includes("}")) break;
  }
  return null;
}

/** 删除字符串外的注释，保留引号中的井号。 */
function stripComment(text: string): string {
  let quote: string | null = null;
  let escaped = false;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (quote !== null) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === quote) quote = null;
    } else if (character === "\"" || character === "'") quote = character;
    else if (character === "#") return text.slice(0, index);
  }
  return text;
}

/** 扫描静态值边界，跳过字符串和嵌套容器。 */
function findValueEnd(text: string, start: number): number {
  let quote: string | null = null;
  let escaped = false;
  let depth = 0;
  for (let index = start; index < text.length; index += 1) {
    const character = text[index];
    if (quote !== null) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === quote) quote = null;
    } else if (character === "\"" || character === "'") quote = character;
    else if (character === "{" || character === "[") depth += 1;
    else if (character === "}" || character === "]") depth -= 1;
    else if (depth === 0 && (/\s/.test(character) || character === "," || character === "}" || character === "]")) return index;
  }
  return text.length;
}

/** 以字符区间替换标量，避免正则重排无关文本。 */
function replaceScalar(source: string, start: number, end: number, replacement: string): string {
  return `${source.slice(0, start)}${replacement}${source.slice(end)}`;
}

/** 返回值起点前的空白数量。 */
function countLeadingSpace(text: string): number {
  let index = 0;
  while (index < text.length && /\s/.test(text[index])) index += 1;
  return index;
}

/** 将已校验值序列化为配置字面量。 */
function serializeValue(value: boolean | "zh-CN" | "en-US"): string {
  return typeof value === "boolean" ? String(value) : JSON.stringify(value);
}

/** 转义只用于扫描键名的正则元字符。 */
function escapeRegExp(value: string): string { return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"); }

/** 读取目录名；不可读目录按“未找到”处理，继续向祖先查找。 */
async function readDirectory(directory: string, fileSystem: ConfigFileSystem): Promise<string[]> {
  try { return await fileSystem.readdir(directory); } catch { return []; }
}

/** 判断底层错误是否表示路径不存在。 */
function isMissing(error: unknown): boolean { return typeof error === "object" && error !== null && "code" in error && (error as { code?: string }).code === "ENOENT"; }
/** 判断重命名是否因目标文件存在而失败。 */
function isAlreadyExists(error: unknown): boolean { return typeof error === "object" && error !== null && "code" in error && ["EEXIST", "EPERM", "ENOTEMPTY"].includes((error as { code?: string }).code ?? ""); }
