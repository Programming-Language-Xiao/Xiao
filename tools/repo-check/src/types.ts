/**
 * 仓库检查器公共类型定义。
 */

/**
 * 目录与文档检查器使用的严重级别。
 */
export type Severity = "error" | "warning" | "info";

/**
 * 可被 CI 和其他工具稳定消费的检查诊断。
 */
export interface Diagnostic {
  /** 稳定的规则编号。 */
  code: string;
  /** 严重级别；只有 error 会使检查失败。 */
  severity: Severity;
  /** 仓库相对路径；无法定位时为空字符串。 */
  path: string;
  /** 一基行号；未知时为空。 */
  line?: number;
  /** 诊断所针对的对象。 */
  subject: string;
  /** 面向人的说明。 */
  message: string;
  /** 建议的修复动作。 */
  hint?: string;
  /** 稳定消息编号，供国际化层使用。 */
  message_id?: string;
}

/**
 * 一次检查的统一结果。
 */
export interface CheckResult {
  /** 检查是否通过。 */
  passed: boolean;
  /** 本次检查产生的诊断。 */
  diagnostics: Diagnostic[];
}

/**
 * Rust workspace 的政策声明。
 */
export interface WorkspacePolicy {
  /** 构建工具 manifest 的仓库相对路径。 */
  manifest: string;
  /** 必须逐项出现的成员目录。 */
  members: string[];
}

/**
 * A0 仓库政策清单。
 */
export interface RepositoryManifest {
  /** 清单格式版本。 */
  schemaVersion: number;
  /** Rust workspace 声明。 */
  rust: WorkspacePolicy;
  /** Bun workspace 声明。 */
  typescript: WorkspacePolicy;
  /** 需要扫描代码目录的根。 */
  codeRoots: string[];
  /** 计入源文件发现的扩展名。 */
  sourceExtensions: string[];
  /** 不递归扫描的目录名。 */
  excludedDirectories: string[];
  /** 每个代码目录必须拥有的 README 文件名。 */
  readmeFile: string;
  /** 模块登记表路径。 */
  moduleRegistry: string;
}

/**
 * UseDocs 模块登记项。
 */
export interface ModuleRecord {
  /** 全局唯一模块标识。 */
  id: string;
  /** 对应开发期，例如 `A0` 或 `01`。 */
  stage: string;
  /** 模块交付状态。 */
  status: "planned" | "draft" | "verified" | "deprecated";
  /** 代码目录或文件。 */
  code: string[];
  /** 测试目录或文件。 */
  tests: string[];
  /** UseDocs 页面或索引。 */
  usedocs: string[];
  /** 废弃模块的替代模块 ID。 */
  replacement?: string;
}

/**
 * 模块登记文件的根对象。
 */
export interface ModuleRegistry {
  /** 登记格式版本。 */
  schemaVersion: number;
  /** 模块记录。 */
  modules: ModuleRecord[];
}

/**
 * 仓库政策清单的加载结果。
 */
export interface LoadedRepository {
  /** 仓库根绝对路径。 */
  root: string;
  /** 已解析的政策清单。 */
  manifest: RepositoryManifest;
  /** 模块登记表；缺失时由调用方报告。 */
  registry?: ModuleRegistry;
}
