/** 提交信息门禁：在提交前锁定标题规范和非空正文。 */

import { readFileSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";

import { isFile, isInside, repoRelative } from "./paths.ts";
import type { CheckResult, Diagnostic } from "./types.ts";

/** 允许的提交标题类型及可选 scope。 */
const COMMIT_SUBJECT_PATTERN = /^(?:feat|fix|test|docs|chore|refactor)(?:\([^()\r\n]+\))?:\s+\S/;

/**
 * 检查 Git 提交信息文件。
 *
 * 标题必须使用仓库约定的类型前缀，正文必须至少包含一行非空、非注释内容，
 * 用来说明本次提交为什么存在，而不是只重复标题已经做了什么。
 *
 * @param root 仓库根目录。
 * @param messagePath 提交信息文件的绝对路径或仓库相对路径。
 * @returns 可直接交给统一报告器的检查结果。
 */
export function checkCommitMessage(root: string, messagePath: string): CheckResult {
  const absolutePath = isAbsolute(messagePath) ? resolve(messagePath) : resolve(root, messagePath);
  const relativePath = repoRelative(root, absolutePath);
  const diagnostics: Diagnostic[] = [];
  if (!isInside(root, absolutePath)) {
    diagnostics.push({
      code: "A0-COMMIT-003",
      severity: "error",
      path: relativePath,
      subject: messagePath,
      message: "提交信息文件必须位于仓库根目录内。",
      hint: "把 Git 传入的提交信息文件路径原样交给检查器，不要指向仓库外部文件。",
      message_id: "a0.commit.message_path_outside_repository",
    });
    return { passed: false, diagnostics };
  }
  if (!isFile(absolutePath)) {
    diagnostics.push({
      code: "A0-COMMIT-003",
      severity: "error",
      path: relativePath,
      subject: messagePath,
      message: "提交信息文件不存在或不是普通文件。",
      hint: "通过 `commit-msg` 钩子传入 Git 提供的提交信息文件。",
      message_id: "a0.commit.message_file_missing",
    });
    return { passed: false, diagnostics };
  }

  let content: string;
  try {
    content = readFileSync(absolutePath, "utf8");
  } catch (error) {
    diagnostics.push({
      code: "A0-COMMIT-003",
      severity: "error",
      path: relativePath,
      subject: messagePath,
      message: `提交信息文件无法读取：${String(error)}`,
      hint: "确认提交信息文件仍由 Git 保持可读。",
      message_id: "a0.commit.message_file_unreadable",
    });
    return { passed: false, diagnostics };
  }

  const lines = content.replaceAll("\r\n", "\n").replaceAll("\r", "\n").split("\n");
  const subjectIndex = lines.findIndex((line) => {
    const trimmed = line.trim();
    return trimmed.length > 0 && !trimmed.startsWith("#");
  });
  if (subjectIndex < 0) {
    diagnostics.push({
      code: "A0-COMMIT-001",
      severity: "error",
      path: relativePath,
      line: 1,
      subject: "commit subject",
      message: "提交标题为空，或只包含 Git 注释。",
      hint: "使用 feat/fix/test/docs/chore/refactor 前缀，并写出具体标题。",
      message_id: "a0.commit.subject_missing",
    });
    return { passed: false, diagnostics };
  }

  const subject = lines[subjectIndex].trim();
  if (!COMMIT_SUBJECT_PATTERN.test(subject)) {
    diagnostics.push({
      code: "A0-COMMIT-001",
      severity: "error",
      path: relativePath,
      line: subjectIndex + 1,
      subject,
      message: "提交标题不符合仓库约定的类型前缀格式。",
      hint: "使用 `feat:`、`fix:`、`test:`、`docs:`、`chore:` 或 `refactor:`；可附带 `(scope)`。",
      message_id: "a0.commit.subject_invalid",
    });
  }

  const body = lines
    .slice(subjectIndex + 1)
    .filter((line) => !line.trimStart().startsWith("#"))
    .join("\n")
    .trim();
  if (body.length === 0) {
    diagnostics.push({
      code: "A0-COMMIT-002",
      severity: "error",
      path: relativePath,
      line: subjectIndex + 2,
      subject,
      message: "提交正文为空。",
      hint: "至少写一段说明为什么需要这次改动，以及关键验证依据。",
      message_id: "a0.commit.body_missing",
    });
  }

  return { passed: diagnostics.length === 0, diagnostics };
}
