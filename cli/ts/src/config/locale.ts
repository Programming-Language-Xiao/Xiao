/** 单次 CLI/REPL 生命周期内不可变的有效语言上下文。 */

import { readConfigValue, type ConfigEditorOptions } from "./editor.ts";

/** 11C 首版支持的内置语言。 */
export type SupportedLocale = "zh-CN" | "en-US";

/** 调用链统一传递的有效语言及来源。 */
export interface LocaleContext {
  readonly tag: SupportedLocale;
  readonly source: "project" | "global" | "default";
}

/** 项目覆盖全局；未声明时只创建一份默认语言上下文。 */
export async function resolveEffectiveLocale(options: ConfigEditorOptions = {}): Promise<LocaleContext> {
  const project = await readConfigValue("project", "language.locale", options);
  if (project === "zh-CN" || project === "en-US") return Object.freeze({ tag: project, source: "project" });
  const global = await readConfigValue("global", "language.locale", options);
  if (global === "zh-CN" || global === "en-US") return Object.freeze({ tag: global, source: "global" });
  return Object.freeze({ tag: "zh-CN", source: "default" });
}
