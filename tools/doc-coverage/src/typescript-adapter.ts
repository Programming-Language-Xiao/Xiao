/** TypeScript Compiler API 声明扫描适配器。 */

import { readFileSync } from "node:fs";
import { basename, relative, resolve } from "node:path";

import ts from "typescript";

import type { DeclarationKind, DeclarationRecord } from "./types.ts";

/**
 * TypeScript 文件结构大纲中的一个节点。
 *
 * 此类型刻意独立于 Rust 协议的 `RustOutlineNode`：两侧输出形状一致，
 * 但 Rust 响应仍需要保持其版本化 JSON 校验边界。
 */
export interface TypeScriptOutlineNode {
  /** 节点类别，例如 `class`、`field` 或 `variant`。 */
  kind: string;
  /** 节点名称；匿名节点使用稳定占位名。 */
  name: string;
  /** 声明自身起始行（一基）。 */
  line: number;
  /** 整个节点结束行（一基）。 */
  end_line: number;
  /** 从起始行到结束行的覆盖行数。 */
  lines: number;
  /** 去掉节点名称的定义句。 */
  signature: string;
  /** 节点起始行的源码文本。 */
  source_line: string;
  /** 递归子节点。 */
  children: TypeScriptOutlineNode[];
}

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

/**
 * 使用 TypeScript Compiler API 尽力生成文件结构大纲。
 *
 * 大纲只服务超长文件的拆分建议，因此不能沿用覆盖率扫描的「解析诊断即失败」
 * 策略；单个节点无法读取时跳过该节点，其他可恢复结构仍应保留。
 *
 * @param file 文件绝对路径。
 * @returns 与 Rust 大纲同形的 TypeScript 结构节点。
 */
export function outlineTypeScriptFile(file: string): TypeScriptOutlineNode[] {
  const sourceText = readFileSync(file, "utf8");
  const scriptKind = file.toLowerCase().endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS;
  const sourceFile = ts.createSourceFile(file, sourceText, ts.ScriptTarget.Latest, true, scriptKind);
  return collectOutlineNodes(sourceFile.statements, sourceFile, sourceText);
}

/** 收集同一结构层级中的可展示节点，局部失败不影响相邻节点。 */
function collectOutlineNodes(nodes: readonly ts.Node[], sourceFile: ts.SourceFile, sourceText: string): TypeScriptOutlineNode[] {
  const result: TypeScriptOutlineNode[] = [];
  for (const node of nodes) {
    try {
      const outlined = outlineNode(node, sourceFile, sourceText);
      if (outlined) {
        result.push(outlined);
      } else {
        result.push(...collectOutlineNodes(transparentOutlineChildren(node), sourceFile, sourceText));
      }
    } catch {
      // 大纲是尽力而为的辅助信息，坏节点不能掩盖文件行数错误。
    }
  }
  return result;
}

/** 将一个可展示的 TypeScript AST 节点转换为大纲节点。 */
function outlineNode(node: ts.Node, sourceFile: ts.SourceFile, sourceText: string): TypeScriptOutlineNode | undefined {
  const kind = outlineKind(node);
  if (!kind) return undefined;
  const named = outlineNameNode(node);
  const anchor = named ?? node;
  const name = named ? named.getText(sourceFile) : "<default>";
  const start = anchor.getStart(sourceFile);
  const line = sourceFile.getLineAndCharacterOfPosition(start).line + 1;
  const endPosition = Math.max(start, node.getEnd() - 1);
  const endLine = Math.max(line, sourceFile.getLineAndCharacterOfPosition(endPosition).line + 1);
  const sourceLine = sourceLineAt(sourceFile, sourceText, line);
  return {
    kind,
    name,
    line,
    end_line: endLine,
    lines: endLine - line + 1,
    signature: outlineSignature(sourceFile, sourceText, line, named),
    source_line: sourceLine,
    children: collectOutlineNodes(outlineChildren(node), sourceFile, sourceText),
  };
}

/** 映射为稳定的公共类别，不能依赖带别名的 SyntaxKind 反查字符串。 */
function outlineKind(node: ts.Node): string | undefined {
  if (ts.isFunctionDeclaration(node) || ts.isFunctionExpression(node) || ts.isArrowFunction(node)) return "function";
  if (ts.isClassDeclaration(node) || ts.isClassExpression(node)) return "class";
  if (ts.isInterfaceDeclaration(node)) return "interface";
  if (ts.isTypeAliasDeclaration(node)) return "type";
  if (ts.isEnumDeclaration(node)) return "enum";
  if (ts.isModuleDeclaration(node)) return "module";
  if (ts.isVariableDeclaration(node)) return "variable";
  if (ts.isMethodDeclaration(node)
    || ts.isMethodSignature(node)
    || ts.isGetAccessorDeclaration(node)
    || ts.isSetAccessorDeclaration(node)
    || ts.isConstructorDeclaration(node)
    || ts.isCallSignatureDeclaration(node)
    || ts.isConstructSignatureDeclaration(node)) return "method";
  if (ts.isPropertyDeclaration(node)
    || ts.isPropertySignature(node)
    || ts.isPropertyAssignment(node)
    || ts.isShorthandPropertyAssignment(node)
    || ts.isIndexSignatureDeclaration(node)) return "field";
  if (ts.isEnumMember(node)) return "variant";
  return undefined;
}

/** 返回名称 AST 节点；构造器和匿名默认导出会回退到整个节点。 */
function outlineNameNode(node: ts.Node): ts.Node | undefined {
  const candidate = (node as ts.Node & { name?: ts.Node }).name;
  return candidate;
}

/** 返回一个可展示节点的直接结构子项，函数体局部变量不进入大纲。 */
function outlineChildren(node: ts.Node): readonly ts.Node[] {
  if (ts.isClassDeclaration(node) || ts.isClassExpression(node) || ts.isInterfaceDeclaration(node)) return node.members;
  if (ts.isEnumDeclaration(node)) return node.members;
  if (ts.isModuleDeclaration(node)) return moduleBodyChildren(node.body);
  if (ts.isTypeAliasDeclaration(node)) return typeMembers(node.type);
  if (ts.isVariableDeclaration(node)) return initializerChildren(node.initializer);
  if (ts.isPropertyDeclaration(node) || ts.isPropertyAssignment(node)) return initializerChildren(node.initializer);
  return [];
}

/** 透明包装节点的子项；自身不应作为报告里的结构行。 */
function transparentOutlineChildren(node: ts.Node): readonly ts.Node[] {
  if (ts.isVariableStatement(node)) return node.declarationList.declarations;
  if (ts.isObjectLiteralExpression(node)) return node.properties;
  if (ts.isModuleBlock(node)) return node.statements;
  if (ts.isTypeLiteralNode(node)) return node.members;
  if (ts.isParenthesizedExpression(node)
    || ts.isAsExpression(node)
    || ts.isTypeAssertionExpression(node)
    || ts.isSatisfiesExpression(node)
    || ts.isExpressionStatement(node)
    || ts.isExportAssignment(node)) return [node.expression];
  return [];
}

/** 展开可承载对象、类或函数定义的初始化表达式。 */
function initializerChildren(initializer: ts.Expression | undefined): readonly ts.Node[] {
  return initializer ? [initializer] : [];
}

/** 取命名空间的直接语句或嵌套命名空间。 */
function moduleBodyChildren(body: ts.ModuleBody | undefined): readonly ts.Node[] {
  if (!body) return [];
  if (ts.isModuleBlock(body)) return body.statements;
  return [body];
}

/** 提取类型别名内的对象字面量成员，联合与交叉类型可继续下钻。 */
function typeMembers(type: ts.TypeNode): readonly ts.Node[] {
  if (ts.isTypeLiteralNode(type)) return type.members;
  if (ts.isParenthesizedTypeNode(type)) return typeMembers(type.type);
  if (ts.isUnionTypeNode(type) || ts.isIntersectionTypeNode(type)) return type.types.flatMap((item) => typeMembers(item));
  return [];
}

/** 使用 Compiler API 的行起点切出源码行，覆盖所有 TypeScript 行终止符。 */
function sourceLineAt(sourceFile: ts.SourceFile, sourceText: string, line: number): string {
  const starts = sourceFile.getLineStarts();
  const start = starts[line - 1] ?? 0;
  const end = starts[line] ?? sourceText.length;
  return sourceText.slice(start, end).replace(/[\r\n\u2028\u2029]+$/u, "").trim();
}

/** 按 AST 名称位置去掉定义句中的名称，避免短名误删到前面的无关文本。 */
function outlineSignature(sourceFile: ts.SourceFile, sourceText: string, line: number, named: ts.Node | undefined): string {
  const starts = sourceFile.getLineStarts();
  const lineStart = starts[line - 1] ?? 0;
  const lineEnd = starts[line] ?? sourceText.length;
  const rawLine = sourceText.slice(lineStart, lineEnd);
  const terminator = rawLine.search(/[;{]/u);
  const headEnd = terminator < 0 ? lineEnd : lineStart + terminator;
  let head = sourceText.slice(lineStart, headEnd);
  if (named) {
    const nameStart = named.getStart(sourceFile);
    const nameEnd = named.getEnd();
    if (nameStart >= lineStart && nameEnd <= headEnd) {
      const start = nameStart - lineStart;
      const end = nameEnd - lineStart;
      head = `${head.slice(0, start)}${head.slice(end)}`;
    }
  }
  return head.replace(/\s+/gu, " ").replace("fn (", "fn(").trim().replace(/[,:]$/u, "").trim();
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
