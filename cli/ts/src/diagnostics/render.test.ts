/** 结构化诊断渲染回归。 */

import { describe, expect, test } from "bun:test";

import { renderCliError, renderProtocolResponse } from "./render.ts";
import { CoreDiscoveryError } from "../platform/core.ts";

describe("CLI 诊断呈现", () => {
  test("退出码直接来自协议字段，JSON 不混入 ANSI", () => {
    const response = {
      type: "result" as const,
      request_id: "r1",
      operation: "run" as const,
      exit_code: 3,
      exit_name: "runtime_error",
      diagnostics: [],
      report: { code: "X06-RUNTIME-001", message: "失败" },
      events: [],
      metrics: null,
      value: null,
      artifact: null,
    };
    const rendered = renderProtocolResponse(response, { json: true, color: "always" });
    expect(rendered.exitCode).toBe(3);
    expect(rendered.stdout).not.toContain("\u001B[");
    expect(JSON.parse(rendered.stdout).exit_code).toBe(3);
  });

  test("非 TTY 人类错误为纯文本", () => {
    const rendered = renderCliError(new Error("bad"), { isTTY: false, color: "auto" });
    expect(rendered.stderr).not.toContain("\u001B[");
    expect(rendered.stderr).toContain("X11-CLI-001");
  });

  test("核心发现 JSON 诊断保留候选来源", () => {
    const rendered = renderCliError(new CoreDiscoveryError(
      "找不到 xiao-core",
      ["C:/bundle/xiao-core.exe"],
      [{ path: "C:/bundle/xiao-core.exe", source: "adjacent" }],
    ), { json: true });
    const value = JSON.parse(rendered.stdout) as { code: string; details: { candidate_sources: Array<{ source: string }> } };
    expect(value.code).toBe("X11-CLI-CORE-001");
    expect(value.details.candidate_sources[0]?.source).toBe("adjacent");
  });

  test("人类模式展示项目测试统计、路径和结构化诊断", () => {
    const rendered = renderProtocolResponse({
      type: "test_result",
      request_id: "test-1",
      operation: "test",
      exit_code: 1,
      exit_name: "source_rejected",
      total: 1,
      passed: 0,
      failed: 1,
      tests: [{
        path: "tests/case.xiao",
        module: "tests/case",
        exit_code: 1,
        exit_name: "source_rejected",
        diagnostics: [{ code: "X11-TEST-001", message: "失败", severity: "error" }],
        report: null,
        events: [],
        metrics: null,
        value: null,
        error: null,
      }],
    }, { isTTY: false, color: "auto" });
    expect(rendered.exitCode).toBe(1);
    expect(rendered.stderr).toContain("0/1");
    expect(rendered.stderr).toContain("tests/case.xiao");
    expect(rendered.stderr).toContain("X11-TEST-001");
  });
});
