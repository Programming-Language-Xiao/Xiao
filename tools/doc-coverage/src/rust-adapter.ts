/** Rust `syn` AST 适配器的 JSON 协议调用层。 */

import { spawnSync } from "node:child_process";
import { join, relative, resolve } from "node:path";

import type { CoverageDiagnostic, DeclarationRecord, RustAdapterResponse } from "./types.ts";

/** 当前 TypeScript 编排器支持的 Rust AST 适配器协议版本。 */
export const RUST_ADAPTER_PROTOCOL_VERSION = 1;

/**
 * Rust AST 适配器调用选项。
 */
export interface RustAdapterOptions {
  /** 仓库根目录。 */
  root: string;
  /** 待扫描的 Rust 文件绝对路径。 */
  files: string[];
  /** 可选的已编译适配器路径。 */
  adapterPath?: string;
}

/**
 * 通过稳定 JSON 协议调用 Rust `syn` AST 适配器。
 *
 * @param options 适配器调用参数。
 * @returns 统一声明记录和诊断。
 */
export function scanRustFiles(options: RustAdapterOptions): { declarations: DeclarationRecord[]; diagnostics: CoverageDiagnostic[] } {
  if (options.files.length === 0) return { declarations: [], diagnostics: [] };
  const adapter = options.adapterPath ?? process.env.XIAO_RUST_DOC_ADAPTER;
  const command = adapter ? adapter : "cargo";
  const args = adapter
    ? []
    : ["run", "--quiet", "--manifest-path", join(options.root, "core/rust/Cargo.toml"), "-p", "xiao-doc-coverage-rust", "--bin", "xiao-doc-coverage-rust", "--"];
  const response = spawnSync(command, args, {
    cwd: options.root,
    input: JSON.stringify({ protocol_version: RUST_ADAPTER_PROTOCOL_VERSION, files: options.files }),
    encoding: "utf8",
    windowsHide: true,
  });
  if (response.error || response.status !== 0 && !response.stdout?.trim()) {
    return {
      declarations: [],
      diagnostics: [{
        code: "A0-PARSER-001",
        severity: "error",
        path: "",
        subject: "rust-adapter",
        message: `Rust AST 适配器启动失败：${response.error?.message ?? response.stderr?.trim() ?? `退出码 ${response.status}`}`,
        hint: "确认 Rust 工具链可用，并先运行 cargo test -p xiao-doc-coverage-rust。",
        message_id: "a0.parser.rust_adapter_failed",
      }],
    };
  }
  let parsed: RustAdapterResponse;
  try {
    parsed = JSON.parse(response.stdout) as RustAdapterResponse;
  } catch (error) {
    return {
      declarations: [],
      diagnostics: [{
        code: "A0-PARSER-001",
        severity: "error",
        path: "",
        subject: "rust-adapter",
        message: `Rust AST 适配器输出无法解析：${String(error)}`,
        hint: "检查适配器协议版本和标准输出是否被其他日志污染。",
        message_id: "a0.parser.rust_adapter_json",
      }],
    };
  }
  const protocolError = validateRustAdapterResponse(parsed);
  if (protocolError) {
    return { declarations: [], diagnostics: [protocolError] };
  }
  const declarations = (parsed.declarations ?? []).map((item) => ({
    language: "rust" as const,
    file: relativePath(options.root, item.file),
    line: item.line,
    kind: normalizeKind(item.kind),
    name: item.name,
    isPublic: item.is_public,
    hasDoc: item.has_doc,
    parser: "syn/2",
  }));
  const diagnostics = (parsed.errors ?? []).map((item) => ({
    code: item.code || "A0-PARSER-001",
    severity: "error" as const,
    path: relativePath(options.root, item.file),
    subject: item.file,
    message: item.message,
    hint: "修复 Rust 源文件语法或 AST 适配器协议后重试。",
    message_id: "a0.parser.rust_source",
  }));
  return { declarations, diagnostics };
}

/**
 * 校验适配器响应的版本和顶层字段，避免静默接受不兼容输出。
 *
 * @param value 未信任的 JSON 解码结果。
 * @returns 协议错误；响应合法时返回 `undefined`。
 */
export function validateRustAdapterResponse(value: unknown): CoverageDiagnostic | undefined {
  if (!isRecord(value)) return protocolDiagnostic("Rust AST 适配器响应不是 JSON 对象。", "检查适配器标准输出是否只包含协议 JSON。");
  if (value.protocol_version !== RUST_ADAPTER_PROTOCOL_VERSION) {
    return protocolDiagnostic(
      `Rust AST 适配器协议版本不匹配：收到 ${String(value.protocol_version)}，需要 ${RUST_ADAPTER_PROTOCOL_VERSION}。`,
      "升级或重新编译 Rust 适配器，并确保 TypeScript 编排器与适配器使用同一协议版本。",
    );
  }
  if (!Array.isArray(value.declarations) || !Array.isArray(value.errors)) {
    return protocolDiagnostic("Rust AST 适配器响应缺少 declarations 或 errors 数组。", "检查适配器协议实现，确保响应字段完整且类型正确。");
  }
  for (const declaration of value.declarations) {
    if (!isRecord(declaration)
      || typeof declaration.file !== "string"
      || typeof declaration.line !== "number"
      || typeof declaration.kind !== "string"
      || typeof declaration.name !== "string"
      || typeof declaration.is_public !== "boolean"
      || typeof declaration.has_doc !== "boolean") {
      return protocolDiagnostic("Rust AST 适配器返回了格式错误的声明记录。", "检查 declarations 中每条记录的字段类型。");
    }
  }
  for (const error of value.errors) {
    if (!isRecord(error) || typeof error.file !== "string" || typeof error.code !== "string" || typeof error.message !== "string") {
      return protocolDiagnostic("Rust AST 适配器返回了格式错误的错误记录。", "检查 errors 中每条记录的字段类型。");
    }
  }
  return undefined;
}

/** 适配器协议诊断的统一构造器。 */
function protocolDiagnostic(message: string, hint: string): CoverageDiagnostic {
  return {
    code: "A0-PROTOCOL-001",
    severity: "error",
    path: "",
    subject: "rust-adapter",
    message,
    hint,
    message_id: "a0.protocol.rust_adapter_response",
  };
}

/** 判断未知值是否为普通对象。 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** 把 Rust 适配器类别限制到统一报告枚举。 */
function normalizeKind(kind: string): DeclarationRecord["kind"] {
  const allowed: DeclarationRecord["kind"][] = ["module", "function", "method", "class", "interface", "type", "enum", "struct", "trait", "field", "constant", "static", "union", "reexport", "variable"];
  return allowed.includes(kind as DeclarationRecord["kind"]) ? kind as DeclarationRecord["kind"] : "variable";
}

/** 将适配器返回的路径转换为仓库相对路径。 */
function relativePath(root: string, file: string): string {
  return relative(resolve(root), resolve(file)).replaceAll("\\", "/");
}
