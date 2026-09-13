# 02-parser 规格快照

## 目录职责

本目录保存 01 阶段 P0 最小解析器的正反规格快照。快照只覆盖已经冻结的
程序根节点、字面量、名称、简单赋值、文档注释关联和错误恢复；它不提前
冻结运算、调用、索引、代码块、类型声明或容器语义。

## 工程期

01/P0。集成入口是
`core/rust/crates/xiao-syntax/tests/parser_snapshots.rs`，由 Rust workspace
测试直接读取这些 UTF-8 JSON 文件。

## 快照约定

- `source` 保存原始源码，所有区间都是 UTF-8 字节偏移的半开区间。
- `statements` 保存成功恢复出的顶层语句；表达式语句使用 `expression`，
  赋值语句使用 `target` 和 `value`。
- `leading_docs` 保存挂接到语句前的文档注释区间；没有后续语句的文档注释
  放在 `orphan_doc_comments`，不能静默丢弃。
- `diagnostics` 同时记录词法和解析诊断的机器编号、消息键和源码区间。

## 依赖边界

快照只验证 `xiao-source`、`xiao-diagnostics` 和 `xiao-syntax` 的公开前端
接口；不启动 Runtime、VM、LLVM 或 CLI。新增 P0 语法必须同时补充至少一份
正例和一份负例，并在 DevDocs 与 UseDocs 中说明边界。
