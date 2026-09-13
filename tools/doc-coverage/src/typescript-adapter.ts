/** TypeScript Compiler API 声明扫描适配器。 */

import { readFileSync } from "node:fs";
import { basename, relative, resolve } from "node:path";

import ts from "typescript";

import type { DeclarationKind, DeclarationRecord } from "./types.ts";

/**
 * 使用 TypeScript Compiler API 扫描一个 TypeScript/TSX 文件。
 *
 * @param root 仓库根目录。
 * @param file 文件绝对路径。
 * @returns 统一声明记录。
 */
export function scanTypeScriptFile(root: string, file: string): DeclarationRecord[] {
  const sourceText = readFileSync(file, "utf8");
  const scriptKind = file.toLowerCase().endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS;
  const sourceFile = ts.createSourceFile(file, sourceText, ts.ScriptTarget.Latest, true, scriptKind);
  const parseDiagnostics = (sourceFile as ts.SourceFile & { parseDiagnostics?: readonly ts.Diagnostic[] }).parseDiagnostics ?? [];
  if (parseDiagnostics.length > 0) {
    throw new Error(formatParseDiagnostic(sourceFile, parseDiagnostics[0]));
  }
  const records: DeclarationRecord[] = [{
    language: "typescript",
    file: relativePath(root, file),
    line: 1,
    kind: "module",
    name: basename(file),
    isPublic: isEntryFile(file),
    hasDoc: hasJsDoc(sourceFile, sourceFile, sourceText),
    parser: `typescript-compiler-api/${ts.versionMajorMinor}`,
  }];
  const visit = (node: ts.Node): void => {
    const declaration = declarationRecord(root, sourceFile, sourceText, node);
    if (declaration) records.push(declaration);
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return records;
}

/** 把一个 TypeScript AST 节点转换为声明记录。 */
function declarationRecord(root: string, sourceFile: ts.SourceFile, sourceText: string, node: ts.Node): DeclarationRecord | undefined {
  let kind: DeclarationKind | undefined;
  let name = "<anonymous>";
  let isPublic = false;
  if (ts.isFunctionDeclaration(node)) {
    kind = "function";
    name = node.name?.text ?? "<default>";
    isPublic = hasExportModifier(node);
  } else if (ts.isMethodDeclaration(node) || ts.isGetAccessorDeclaration(node) || ts.isSetAccessorDeclaration(node) || ts.isConstructorDeclaration(node)) {
    kind = "method";
    name = ts.isConstructorDeclaration(node) ? "constructor" : node.name.getText(sourceFile);
    isPublic = !hasPrivateModifier(node) && (hasExportedContainingClass(node) || hasPublicModifier(node));
  } else if (ts.isClassDeclaration(node)) {
    kind = "class";
    name = node.name?.text ?? "<default>";
    isPublic = hasExportModifier(node);
  } else if (ts.isInterfaceDeclaration(node)) {
    kind = "interface";
    name = node.name.text;
    isPublic = hasExportModifier(node);
  } else if (ts.isTypeAliasDeclaration(node)) {
    kind = "type";
    name = node.name.text;
    isPublic = hasExportModifier(node);
  } else if (ts.isEnumDeclaration(node)) {
    kind = "enum";
    name = node.name.text;
    isPublic = hasExportModifier(node);
  } else if (ts.isModuleDeclaration(node)) {
    kind = "module";
    name = node.name.getText(sourceFile);
    isPublic = hasExportModifier(node);
  } else if (ts.isVariableStatement(node)) {
    isPublic = hasExportModifier(node);
    if (isPublic) {
      const declaration = node.declarationList.declarations[0];
      kind = "variable";
      name = declaration?.name.getText(sourceFile) ?? "<anonymous>";
    }
  } else if (ts.isImportEqualsDeclaration(node) || ts.isExportDeclaration(node) || ts.isExportAssignment(node)) {
    kind = "reexport";
    name = ts.isExportAssignment(node) ? "default" : "export";
    isPublic = true;
  }
  if (!kind) return undefined;
  const position = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;
  return {
    language: "typescript",
    file: relativePath(root, sourceFile.fileName),
    line: position,
    kind,
    name,
    isPublic,
    hasDoc: hasJsDoc(sourceFile, node, sourceText),
    parser: `typescript-compiler-api/${ts.versionMajorMinor}`,
  };
}

/** 判断声明前是否存在非空 JSDoc。 */
function hasJsDoc(sourceFile: ts.SourceFile, node: ts.Node, sourceText: string): boolean {
  const ranges = ts.getLeadingCommentRanges(sourceText, node.getFullStart()) ?? [];
  return ranges.some((range) => {
    if (range.kind !== ts.SyntaxKind.MultiLineCommentTrivia) return false;
    const comment = sourceText.slice(range.pos, range.end);
    return comment.startsWith("/**") && comment.slice(3, -2).replaceAll("*", "").trim().length > 0;
  });
}

/** 判断节点是否带 export/default 修饰符。 */
function hasExportModifier(node: ts.Node): boolean {
  return getModifiers(node).some((modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword || modifier.kind === ts.SyntaxKind.DefaultKeyword);
}

/** 判断节点是否显式声明为 private。 */
function hasPrivateModifier(node: ts.Node): boolean {
  return getModifiers(node).some((modifier) => modifier.kind === ts.SyntaxKind.PrivateKeyword);
}

/** 判断节点是否显式声明为 public。 */
function hasPublicModifier(node: ts.Node): boolean {
  return getModifiers(node).some((modifier) => modifier.kind === ts.SyntaxKind.PublicKeyword);
}

/** 读取 TypeScript 版本兼容的修饰符列表。 */
function getModifiers(node: ts.Node): readonly ts.Modifier[] {
  return ts.canHaveModifiers(node) ? ts.getModifiers(node) ?? [] : [];
}

/** 判断方法所在的类是否导出。 */
function hasExportedContainingClass(node: ts.Node): boolean {
  let current: ts.Node | undefined = node.parent;
  while (current) {
    if (ts.isClassDeclaration(current)) return hasExportModifier(current);
    if (ts.isSourceFile(current)) return false;
    current = current.parent;
  }
  return false;
}

/** 将 TypeScript 解析诊断转换为稳定、可定位的异常文本。 */
function formatParseDiagnostic(sourceFile: ts.SourceFile, diagnostic: ts.Diagnostic): string {
  const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, " ");
  const line = diagnostic.start === undefined ? undefined : sourceFile.getLineAndCharacterOfPosition(diagnostic.start).line + 1;
  return line === undefined ? `TS${diagnostic.code}: ${message}` : `TS${diagnostic.code}（第 ${line} 行）：${message}`;
}

/** 判断文件名是否属于包入口模块。 */
function isEntryFile(file: string): boolean {
  const name = basename(file).toLowerCase();
  return name === "index.ts" || name === "index.tsx" || name === "cli.ts" || name === "cli.tsx";
}

/** 将文件绝对路径转换为仓库相对路径。 */
function relativePath(root: string, file: string): string {
  return relative(resolve(root), resolve(file)).replaceAll("\\", "/");
}
