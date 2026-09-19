# `tools/doc-coverage/src`

## 目录职责

实现文档注释覆盖率检查器的 TypeScript 编排层、统一声明记录、阈值计算和 JSON/SARIF 报告。语言解析器适配器的最终载体遵循 [A0 实现方案](../../docs/DevDocs/00a-a0-workspace-and-checkers.md) 的冻结决策。

## 工程期

A0.3 建立统计契约和解析适配器；A0.4 接入本地命令与 CI。后续阶段只扩展已版本化的报告 Schema，不改变 90%/100% 门槛的含义。

## 规则

解析失败必须报告并失败，不得退回正则猜测；Rust 适配器通过版本为 2 的 JSON 协议接入，
响应版本和字段必须先校验。请求的 `outline` 开关缺省为 `false`，不请求时响应仍带有空的
`outlines` 数组；覆盖率判定只读取 `declarations`。UseDocs 页面不计入代码 docstring 覆盖率。
实现、测试和用户说明必须同步提交。

`A0-SIZE-001` 是 `outlines` 的第一个消费者：只有 Rust 文件确实超过 2500 物理行时才请求
大纲。TypeScript 侧由 `outlineTypeScriptFile` 提供字段同形、类型独立的
`TypeScriptOutlineNode`，不得把它并入版本化的 `RustOutlineNode` 校验边界。Rust 适配器调用
有 120 秒上限，超时或解析失败只使大纲不可用，不能取消独立计算出的尺寸错误。
