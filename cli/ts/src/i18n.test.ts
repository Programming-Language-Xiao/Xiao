/** CLI 双语目录和参数插值回归。 */

import { expect, test } from "bun:test";

import { cliMessage } from "./i18n.ts";

test("CLI 状态消息保留双语文本并且不二次解析参数", () => {
  expect(cliMessage("xiao.cli.env.created", "zh-CN", { name: "venv", path: "C:/项目/{name}" }))
    .toBe("已创建环境 venv：C:/项目/{name}");
  expect(cliMessage("xiao.cli.env.created", "en-US", { name: "venv", path: "C:/project" }))
    .toBe("created environment venv: C:/project");
});
