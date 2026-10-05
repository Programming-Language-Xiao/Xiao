/** 18A 输入路由：只判定调用意图和输入类别，不执行源码或解码产物。 */

import { readFile, stat } from "node:fs/promises";
import { basename, resolve } from "node:path";

/** 18B 可消费的输入类别。 */
export type RoutedInputKind = "config" | "main-source" | "source" | "xiaoc" | "xar";

/** 输入路由的调用意图。 */
export type InputIntent = "run" | "build";

/** 结构化输入路由结果。 */
export interface RoutedInput {
  kind: RoutedInputKind;
  intent: InputIntent;
  path: string;
}

/** 输入路由错误；机器端读取 code/details，不解析文本。 */
export class InputRoutingError extends Error {
  readonly code: string;
  readonly details: Record<string, unknown>;

  /** 创建带稳定编号和结构化参数的路由错误。 */
  constructor(code: string, message: string, details: Record<string, unknown> = {}) {
    super(`${code}: ${message}`);
    this.name = "InputRoutingError";
    this.code = code;
    this.details = details;
  }
}

/** 对已经读取的输入做纯路由；`.xiao` 的 `[main]` 判定依赖内容而非扩展名。 */
export function routeInput(
  paths: readonly string[],
  intent: InputIntent,
  contents: ReadonlyMap<string, string> = new Map(),
): RoutedInput {
  if (paths.length === 0) throw new InputRoutingError("X11-CLI-INPUT-003", "缺少输入路径", { intent });
  if (paths.length > 1) throw new InputRoutingError("X11-CLI-INPUT-004", "一次调用只能提供一个输入路径", { intent, paths: [...paths] });
  const path = paths[0];
  const lower = basename(path).toLocaleLowerCase("en-US");
  let kind: RoutedInputKind;
  if (lower === "config.xiao") kind = "config";
  else if (lower.endsWith(".xar")) kind = "xar";
  else if (lower.endsWith(".xiaoc")) kind = "xiaoc";
  else if (lower.endsWith(".xiao")) {
    kind = /^\s*\[main\]/mu.test(contents.get(path) ?? "") ? "main-source" : "source";
  } else {
    throw new InputRoutingError("X11-CLI-INPUT-002", "无法识别输入类型", { intent, path });
  }
  if (intent === "build" && (kind === "xar" || kind === "xiaoc")) {
    throw new InputRoutingError("X11-CLI-INPUT-005", "build 需要源码或 config.xiao 输入", { intent, path, kind });
  }
  if (intent === "run" && kind === "config") {
    throw new InputRoutingError("X11-CLI-INPUT-006", "run 不能直接运行 config.xiao", { intent, path, kind });
  }
  return { kind, intent, path };
}

/** 读取一个输入文件后路由；只读取 UTF-8 源码用于 `[main]` 判定，不执行内容。 */
export async function resolveInputPath(
  paths: readonly string[],
  intent: InputIntent,
  cwd = process.cwd(),
): Promise<RoutedInput> {
  if (paths.length === 0) throw new InputRoutingError("X11-CLI-INPUT-003", "缺少输入路径", { intent });
  if (paths.length > 1) throw new InputRoutingError("X11-CLI-INPUT-004", "一次调用只能提供一个输入路径", { intent, paths: [...paths] });
  const path = resolve(cwd, paths[0]);
  try {
    const info = await stat(path);
    if (!info.isFile()) throw new Error("输入不是普通文件");
  } catch (error) {
    throw new InputRoutingError("X11-CLI-INPUT-001", `输入不存在或不可读取：${String(error)}`, { intent, path });
  }
  const lower = basename(path).toLocaleLowerCase("en-US");
  if (lower.endsWith(".xiao") || lower === "config.xiao") {
    const text = await readFile(path, "utf8");
    return routeInput([path], intent, new Map([[path, text]]));
  }
  return routeInput([path], intent);
}
