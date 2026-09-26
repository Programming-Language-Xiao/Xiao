/** E4 四种 Shell 共享同一份激活白名单向量；可注入模型不依赖安装任何 Shell。 */

import { expect, test } from "bun:test";

import vectors from "../../../../tests/spec/11a-shell/activation.json";
import { shellInitScript } from "./index.ts";

const validPath = process.platform === "win32" ? "C:\\project\\.venv" : "/home/project/.venv";

/** 与 E2B 发送端及四份钩子同构的无终端数据白名单模型。 */
function acceptedActivation(contents: string): boolean {
  const match = /^XIAO_ACTIVE_ENV='([^'\r\n]+)'\nexport XIAO_ACTIVE_ENV\n?$/u.exec(contents);
  return match !== null && match[0] === contents && !/[\x00-\x1f\x7f]/u.test(match[1])
    && /^(?:\/|[A-Za-z]:[\\/])/u.test(match[1]);
}

for (const shell of ["bash", "zsh", "fish", "powershell"] as const) {
  test(`${shell} 消费同一份两行白名单向量`, () => {
    const script = shellInitScript(shell);
    expect(script).toContain(`shell-init ${shell}`);
    expect(script).toContain("XIAO_ACTIVATION_FILE");
    expect(script).toContain("export XIAO_ACTIVE_ENV");
    if (shell === "bash" || shell === "zsh") {
      expect(script).toContain("_xiao_lines[@]} -eq 2");
      expect(script).toContain(`_xiao_lines[${shell === "bash" ? 0 : 1}]`);
      expect(script).toContain("[[:cntrl:]]");
      if (shell === "zsh") expect(script).toContain("%F{green}");
    } else if (shell === "fish") {
      expect(script).toContain("test (count $_xiao_lines) -eq 2");
      expect(script).toContain("functions -c fish_prompt __xiao_original_prompt");
      expect(script).toContain("functions -c __xiao_original_prompt fish_prompt");
      expect(script).toContain("[[:cntrl:]]");
    } else {
      expect(script).toContain("[regex]::Match($text");
      expect(script).toContain("[char]27");
    }
    for (const vector of vectors.cases) {
      expect(acceptedActivation(vector.content.replaceAll("{{ABS}}", validPath))).toBe(vector.accepted);
    }
  });
}
