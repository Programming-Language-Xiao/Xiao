/** Cargo 和 Bun workspace 的交叉校验。 */

import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";

import {
  isDirectory,
  isFile,
  readJson,
  readText,
  repoRelative,
  resolveRepoPath,
} from "./paths.ts";
import type { Diagnostic, RepositoryManifest, WorkspacePolicy } from "./types.ts";

/**
 * workspace 检查返回的成员快照。
 */
export interface WorkspaceSnapshot {
  /** 构建工具实际报告的成员目录。 */
  members: string[];
  /** 构建工具报告的包名。 */
  names: string[];
}

/**
 * 使用 `cargo metadata` 校验 Rust workspace 成员和命名唯一性。
 *
 * @param root 仓库根目录。
 * @param policy Rust workspace 政策声明。
 * @returns 成员快照与稳定诊断。
 */
export function inspectRustWorkspace(
  root: string,
  policy: WorkspacePolicy,
): { snapshot?: WorkspaceSnapshot; diagnostics: Diagnostic[] } {
  const diagnostics: Diagnostic[] = [];
  let manifestPath: string;
  try {
    manifestPath = resolveRepoPath(root, policy.manifest);
  } catch (error) {
    diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-002", policy.manifest, String(error), "a0.workspace.invalid_manifest"));
    return { diagnostics };
  }
  if (!isFile(manifestPath)) {
    diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-002", policy.manifest, "Rust workspace manifest 不存在。", "a0.workspace.missing_manifest"));
    return { diagnostics };
  }
  if (/\bmembers\s*=\s*\[[\s\S]*["']?[^\]]*[\*\?][^\]]*\]/m.test(readText(manifestPath))) {
    diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-001", policy.manifest, "Rust workspace 不得使用宽泛 glob 成员。", "a0.workspace.glob_forbidden"));
  }
  const result = spawnSync("cargo", ["metadata", "--no-deps", "--format-version", "1", "--manifest-path", manifestPath], {
    cwd: root,
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.error || result.status !== 0) {
    diagnostics.push(workspaceDiagnostic(
      "A0-WORKSPACE-003",
      policy.manifest,
      `cargo metadata 失败：${result.error?.message ?? result.stderr?.trim() ?? `退出码 ${result.status}`}`,
      "a0.workspace.cargo_failed",
    ));
    return { diagnostics };
  }
  let metadata: any;
  try {
    metadata = JSON.parse(result.stdout);
  } catch (error) {
    diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-003", policy.manifest, `cargo metadata 输出无法解析：${String(error)}`, "a0.workspace.cargo_json"));
    return { diagnostics };
  }
  const packages = Array.isArray(metadata.packages) ? metadata.packages : [];
  const members = packages
    .map((item: any) => typeof item?.manifest_path === "string" ? repoRelative(root, dirname(item.manifest_path)) : undefined)
    .filter((item: string | undefined): item is string => Boolean(item));
  const names = packages
    .map((item: any) => typeof item?.name === "string" ? item.name : undefined)
    .filter((item: string | undefined): item is string => Boolean(item));
  compareMembers(root, policy, members, diagnostics, "Rust");
  reportDuplicateNames(names, policy.manifest, diagnostics);
  return { snapshot: { members, names }, diagnostics };
}

/**
 * 读取 Bun workspace 的显式成员并校验包 manifest。
 *
 * @param root 仓库根目录。
 * @param policy Bun workspace 政策声明。
 * @returns 成员快照与稳定诊断。
 */
export function inspectBunWorkspace(
  root: string,
  policy: WorkspacePolicy,
): { snapshot?: WorkspaceSnapshot; diagnostics: Diagnostic[] } {
  const diagnostics: Diagnostic[] = [];
  let manifestPath: string;
  try {
    manifestPath = resolveRepoPath(root, policy.manifest);
  } catch (error) {
    diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-002", policy.manifest, String(error), "a0.workspace.invalid_manifest"));
    return { diagnostics };
  }
  if (!isFile(manifestPath)) {
    diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-002", policy.manifest, "Bun workspace manifest 不存在。", "a0.workspace.missing_manifest"));
    return { diagnostics };
  }
  let packageJson: any;
  try {
    packageJson = readJson(manifestPath);
  } catch (error) {
    diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-003", policy.manifest, `Bun manifest 无法解析：${String(error)}`, "a0.workspace.bun_json"));
    return { diagnostics };
  }
  const workspaces = Array.isArray(packageJson?.workspaces) ? packageJson.workspaces : undefined;
  if (!workspaces || !workspaces.every((item: unknown) => typeof item === "string" && item.length > 0)) {
    diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-001", policy.manifest, "Bun workspace 必须使用显式字符串成员数组。", "a0.workspace.bun_members"));
    return { diagnostics };
  }
  if (workspaces.some((item: string) => item.includes("*") || item.includes("?"))) {
    diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-001", policy.manifest, "Bun workspace 不得使用宽泛 glob 成员。", "a0.workspace.glob_forbidden"));
  }
  const members = workspaces.map((item: string) => item.replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/$/, ""));
  compareMembers(root, policy, members, diagnostics, "Bun");
  const names: string[] = [];
  for (const member of members) {
    let memberManifest: string;
    try {
      memberManifest = resolveRepoPath(root, join(member, "package.json"));
    } catch (error) {
      diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-002", member, String(error), "a0.workspace.invalid_package_path"));
      continue;
    }
    if (!isFile(memberManifest)) {
      diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-002", member, "workspace 包缺少 package.json。", "a0.workspace.missing_package"));
      continue;
    }
    try {
      const item = readJson(memberManifest) as any;
      if (typeof item?.name !== "string" || item.name.length === 0) {
        diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-002", repoRelative(root, memberManifest), "workspace 包缺少非空 name。", "a0.workspace.package_name"));
      } else {
        names.push(item.name);
      }
    } catch (error) {
      diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-003", repoRelative(root, memberManifest), `package.json 无法解析：${String(error)}`, "a0.workspace.package_json"));
    }
  }
  reportDuplicateNames(names, policy.manifest, diagnostics);
  return { snapshot: { members, names }, diagnostics };
}

/**
 * 同时校验 Rust 和 Bun workspace。
 *
 * @param root 仓库根目录。
 * @param manifest A0 仓库政策清单。
 * @returns 两套 workspace 的诊断。
 */
export function inspectWorkspaces(root: string, manifest: RepositoryManifest): Diagnostic[] {
  return [
    ...inspectRustWorkspace(root, manifest.rust).diagnostics,
    ...inspectBunWorkspace(root, manifest.typescript).diagnostics,
  ];
}

/** 比较政策清单和构建工具实际成员集合。 */
function compareMembers(
  root: string,
  policy: WorkspacePolicy,
  actual: string[],
  diagnostics: Diagnostic[],
  label: string,
): void {
  const expectedSet = new Set(policy.members.map((item) => item.replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/$/, "")));
  const actualSet = new Set(actual.map((item) => item.replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/$/, "")));
  for (const member of expectedSet) {
    if (!actualSet.has(member)) {
      diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-001", member, `${label} workspace 缺少政策清单成员。`, "a0.workspace.member_missing"));
    }
    try {
      if (!isDirectory(resolveRepoPath(root, member))) {
        diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-002", member, "workspace 成员目录不存在。", "a0.workspace.member_directory"));
      }
    } catch (error) {
      diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-002", member, String(error), "a0.workspace.member_path"));
    }
  }
  for (const member of actualSet) {
    if (!expectedSet.has(member)) {
      diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-001", member, `${label} workspace 存在未登记成员。`, "a0.workspace.member_extra"));
    }
  }
}

/** 报告 workspace 内重复的包或 crate 名称。 */
function reportDuplicateNames(names: string[], pathValue: string, diagnostics: Diagnostic[]): void {
  const seen = new Set<string>();
  for (const name of names) {
    if (seen.has(name)) {
      diagnostics.push(workspaceDiagnostic("A0-WORKSPACE-004", pathValue, `workspace 成员名称重复：${name}`, "a0.workspace.duplicate_name"));
    }
    seen.add(name);
  }
}

/** 创建 workspace 检查统一诊断。 */
function workspaceDiagnostic(code: string, pathValue: string, message: string, messageId: string): Diagnostic {
  return {
    code,
    severity: "error",
    path: pathValue.replaceAll("\\", "/"),
    subject: pathValue || "workspace",
    message,
    hint: "保持构建 manifest、政策清单和实际目录逐项一致。",
    message_id: messageId,
  };
}
