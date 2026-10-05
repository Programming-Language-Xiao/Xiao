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

  test("本地化 text 只影响终端文本，机器字段保持原样", () => {
    const response = {
      type: "result" as const, request_id: "locale", operation: "run" as const,
      exit_code: 1, exit_name: "source_rejected",
      diagnostics: [{ code: "X01-TEST", message_id: "xiao.status.cancelled", message: "请求已取消",
        text: "request cancelled", params: {}, severity: "error" }],
      report: null, events: [], metrics: null, value: null, artifact: null,
    };
    const human = renderProtocolResponse(response, { locale: "en-US" });
    expect(human.stderr).toContain("X01-TEST: request cancelled");
    expect(human.stderr).not.toContain("请求已取消");
    const machine = JSON.parse(renderProtocolResponse(response, { locale: "en-US", json: true }).stdout);
    expect(machine.diagnostics[0]).toMatchObject({ code: "X01-TEST", message_id: "xiao.status.cancelled",
      message: "请求已取消", text: "request cancelled", params: {} });
    const oldCore = { ...response, diagnostics: [{ ...response.diagnostics[0], text: undefined }] };
    expect(renderProtocolResponse(oldCore, { locale: "en-US" }).stderr).toContain("xiao.status.cancelled");
    expect(renderProtocolResponse(oldCore, { locale: "zh-CN" }).stderr).toContain("xiao.status.cancelled");
    expect(renderProtocolResponse(oldCore).stderr).toContain("请求已取消");
  });

  test("测试统计标签随 CLI 语言切换", () => {
    const rendered = renderProtocolResponse({
      type: "test_result",
      request_id: "test-locale",
      operation: "test",
      exit_code: 0,
      exit_name: "success",
      total: 0,
      passed: 0,
      failed: 0,
      tests: [],
    }, { locale: "en-US" });
    expect(rendered.stderr).toContain("test results  0/0 passed, failed 0");
  });

  test("构建成功展示链接后产物的 Runtime 事实", () => {
    const rendered = renderProtocolResponse({
      type: "result",
      request_id: "build-runtime",
      operation: "build",
      exit_code: 0,
      exit_name: "success",
      diagnostics: [],
      report: null,
      events: [],
      metrics: null,
      value: null,
      artifact: {
        executable: "build/main.exe",
        llvm_ir_output: null,
        toolchain_fingerprint: "xiao-fnv1a64-test",
        uses_runtime: true,
        runtime_components: ["value", "rc"],
        optimization_level: 0,
        artifact_runtime: {
          object_format: "coff",
          declared_components: ["value", "rc"],
          observed_components: ["value", "rc"],
          runtime_symbols: ["xiao_runtime_value_none", "xiao_runtime_value_release_strong"],
          dependencies: [],
          diagnostic_symbols: [],
          verification: "unverified-coff-exports",
        },
        diagnostic_activation: null,
        diagnostics_component: null,
        runtime_config: null,
      },
    }, { isTTY: false, color: "auto" });
    expect(rendered.exitCode).toBe(0);
    expect(rendered.stderr).toContain("产物 Runtime  coff  组件 value, rc");
    expect(rendered.stderr).toContain("COFF 导出表未验证裁剪");
    expect(rendered.stderr).not.toContain("undefined");
  });

  test("原生构建如实标注优化由 clang 完成、Xiao Pass 是否注册", () => {
    const build = (passesRegistered: boolean) => ({
      type: "result" as const,
      request_id: "build-backend",
      operation: "build",
      exit_code: 0,
      exit_name: "success",
      diagnostics: [],
      report: null,
      events: [],
      metrics: null,
      value: null,
      artifact: {
        executable: "build/main.exe",
        llvm_ir_output: null,
        toolchain_fingerprint: "xiao-fnv1a64-test",
        optimization_level: 2,
        optimization_backend: "clang",
        xiao_passes_registered: passesRegistered,
      },
    });
    const options = { isTTY: false, color: "auto" as const };

    const unregistered = renderProtocolResponse(build(false), options).stderr;
    expect(unregistered).toContain("原生优化后端  clang");
    expect(unregistered).toContain("Xiao LLVM Pass 尚未注册");
    expect(unregistered).not.toContain("Xiao Pass 已注册");

    const registered = renderProtocolResponse(build(true), options).stderr;
    expect(registered).toContain("原生优化后端  clang");
    expect(registered).toContain("Xiao Pass 已注册");
    expect(registered).not.toContain("尚未注册");

    const english = renderProtocolResponse(build(false), { ...options, locale: "en-US" }).stderr;
    expect(english).toContain("native optimization backend  clang");
    expect(english).toContain("Xiao LLVM passes are not registered");
  });

  test("verify 只展示 Rust 生成的发布报告字段", () => {
    const rendered = renderProtocolResponse({
      type: "result",
      request_id: "verify-report",
      operation: "verify",
      exit_code: 0,
      exit_name: "success",
      diagnostics: [],
      report: null,
      events: [],
      metrics: null,
      artifact: null,
      value: {
        kind: "verification",
        value: JSON.stringify({
          kind: "xiaoc",
          release_report: {
            artifact_sha256: "a".repeat(64),
            target_platform: "portable",
            runtime_abi: { min: 1, max: 1 },
            toolchain: "xiao-codegen-llvm/2",
            reproducibility: { status: "not-measured" },
            signature: { status: "unsigned" },
          },
        }),
      },
    }, { isTTY: false, color: "auto" });
    expect(rendered.stderr).toContain("发布摘要");
    expect(rendered.stderr).toContain("portable");
    expect(rendered.stderr).toContain("not-measured");
    expect(rendered.stderr).toContain("SHA-256 只保证完整性");
  });
});
