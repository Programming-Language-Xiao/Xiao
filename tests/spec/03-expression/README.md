# 03-expression：P1 表达式与选择器规格快照

## 目录职责

保存 P1 Pratt 表达式解析、调用/转换、索引路径、范围选择器、步长和随机选择
的 UTF-8 正反规格输入。快照只验证语法 AST 能否建立以及错误编号，不启动类型
检查、容器运行时或随机源。

## 工程期

01/P1。集成入口为
`core/rust/crates/xiao-syntax/tests/p1_expression.rs`；P0 的兼容快照仍位于
`tests/spec/02-parser`。

## 快照约定

- `source` 保存原始 Xiao 源码，位置均按 UTF-8 字节偏移计数。
- `expect` 为 `success` 或 `error`；成功用例必须没有诊断，错误用例只比较
  稳定诊断编号集合。
- 选择器中的重复、混合项目和负路径只检查解析结构，边界/随机/可写性语义
  留给后续 S0/C0/C1 阶段。
