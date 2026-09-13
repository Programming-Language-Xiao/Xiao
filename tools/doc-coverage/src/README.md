# `tools/doc-coverage/src`

## 目录职责

实现文档注释覆盖率检查器的 TypeScript 编排层、统一声明记录、阈值计算和 JSON/SARIF 报告。语言解析器适配器的最终载体遵循 [A0 实现方案](../../docs/DevDocs/00a-a0-workspace-and-checkers.md) 的冻结决策。

## 工程期

A0.3 建立统计契约和解析适配器；A0.4 接入本地命令与 CI。后续阶段只扩展已版本化的报告 Schema，不改变 90%/100% 门槛的含义。

## 规则

解析失败必须报告并失败，不得退回正则猜测；Rust 适配器通过版本为 1 的 JSON 协议接入，响应版本和字段必须先校验。UseDocs 页面不计入代码 docstring 覆盖率。实现、测试和用户说明必须同步提交。
