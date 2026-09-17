---
id: language.compiler.frontend
title: 前端流水线
status: verified
audience: contributor
module: rust.xiao-driver
stage: "08A"
version: "0.1.0"
related:
  - ../README.md
  - ../ir/README.md
  - ../../modules/imports.md
  - ../../../DevDocs/08a-u0-frontend-implementation.md
---

# 前端流水线

状态：`verified`，适用于 Xiao 0.1 的 Rust 前端接口。

统一前端按“解析 → 本地模块分析 → 类型检查 → 生命周期分析 → IR 降低 → IR 验证”
执行。任何阶段出现错误都会保留结构化诊断，并停止产生后端可消费的 IR。

前端只做静态分析，不执行配置值、模块初始化、`init`/`drop` 或用户代码。错误诊断
中的编号、消息键、参数和源码位置稳定；展示语言由上层国际化配置决定。

## 相关页面

- [IR 快照](../ir/README.md)
- [错误报告](../../../troubleshooting/errors-and-reports.md)
- [模块导入](../../modules/imports.md)
