/**
 * 覆盖率工具公共类型定义。
 */

/**
 * 文档覆盖率支持的源语言。
 */
export type SourceLanguage = "rust" | "typescript";

/**
 * 可计入覆盖率的声明类别。
 */
export type DeclarationKind = "module" | "function" | "method" | "class" | "interface" | "type" | "enum" | "struct" | "trait" | "field" | "constant" | "static" | "union" | "reexport" | "variable";

/**
 * 统一语言声明记录。
 */
export interface DeclarationRecord {
  /** 源语言。 */
  language: SourceLanguage;
  /** 声明所在仓库相对路径。 */
  file: string;
  /** 一基行号。 */
  line: number;
  /** 声明类别。 */
  kind: DeclarationKind;
  /** 声明名称。 */
  name: string;
  /** 是否属于公共 API。 */
  isPublic: boolean;
  /** 是否存在实质代码文档。 */
  hasDoc: boolean;
  /** 解析器标识和版本。 */
  parser: string;
}

/**
 * 覆盖率检查的可选参数。
 */
export interface CoverageOptions {
  /** 仓库根目录。 */
  root?: string;
  /** 总体覆盖率最低百分比，默认 90。 */
  totalThreshold?: number;
  /** 公共 API 覆盖率最低百分比，默认 100。 */
  publicThreshold?: number;
  /** 已编译 Rust 适配器路径；未提供时使用 cargo run。 */
  rustAdapter?: string;
}

/**
 * 某个 workspace 成员的覆盖率摘要。
 */
export interface MemberSummary {
  /** 成员目录。 */
  member: string;
  /** 声明总数。 */
  total: number;
  /** 有文档的声明数。 */
  documented: number;
  /** 总体百分比。 */
  percentage: number;
  /** 公共声明总数。 */
  publicTotal: number;
  /** 有文档的公共声明数。 */
  publicDocumented: number;
  /** 公共百分比。 */
  publicPercentage: number;
}

/**
 * 覆盖率数值摘要。
 */
export interface CoverageSummary {
  /** 声明总数。 */
  total: number;
  /** 有文档的声明数。 */
  documented: number;
  /** 总体百分比。 */
  percentage: number;
  /** 公共声明总数。 */
  publicTotal: number;
  /** 有文档的公共声明数。 */
  publicDocumented: number;
  /** 公共百分比。 */
  publicPercentage: number;
  /** 每 workspace 成员摘要。 */
  members: MemberSummary[];
}

/**
 * 覆盖率检查结果，兼容仓库检查器的诊断接口。
 */
export interface CoverageResult {
  /** 是否满足两个阈值且没有解析错误。 */
  passed: boolean;
  /** 稳定诊断列表。 */
  diagnostics: CoverageDiagnostic[];
  /** 数值摘要。 */
  summary: CoverageSummary;
  /** 全部声明记录。 */
  declarations: DeclarationRecord[];
  /** 扫描器版本。 */
  scannerVersion: string;
}

/**
 * 覆盖率工具的结构化诊断。
 */
export interface CoverageDiagnostic {
  /** 稳定错误码。 */
  code: string;
  /** 严重级别。 */
  severity: "error" | "warning" | "info";
  /** 仓库相对路径。 */
  path: string;
  /** 一基行号。 */
  line?: number;
  /** 针对的声明或文件。 */
  subject: string;
  /** 面向人的说明。 */
  message: string;
  /** 修复提示。 */
  hint?: string;
  /** 稳定消息编号。 */
  message_id?: string;
}

/**
 * Rust 适配器返回的原始声明记录。
 */
export interface RustDeclaration {
  /** 文件路径。 */
  file: string;
  /** 一基行号。 */
  line: number;
  /** 声明类别。 */
  kind: string;
  /** 名称。 */
  name: string;
  /** 公共可见性。 */
  is_public: boolean;
  /** 是否有 Rustdoc。 */
  has_doc: boolean;
}

/**
 * Rust 适配器 JSON 响应。
 */
export interface RustAdapterResponse {
  /** 协议版本。 */
  protocol_version: number;
  /** 声明记录。 */
  declarations: RustDeclaration[];
  /** 解析错误。 */
  errors: Array<{ file: string; code: string; message: string }>;
}
