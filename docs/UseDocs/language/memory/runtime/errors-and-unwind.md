---
id: language.memory.runtime.errors-and-unwind
title: Runtime 错误与展开
status: verified
audience: learner
module: rust.xiao-runtime
stage: "06B"
version: "0.1.0"
related:
  - README.md
  - objects-and-handles.md
  - table-lifecycle.md
  - ../../../../DevDocs/06b-runtime-objects-and-tables.md
  - ../../../../DevDocs/07-concurrency-and-errors.md
---

# 错误与展开

运行时错误统一使用 `xiao-diagnostics` 提供的 `XiaoError`，包含稳定错误码、消息标识、结构化参数、可选源码位置、上下文、调用栈和原因链，便于 CLI 或国际化层重新渲染消息。致命故障使用独立 `FatalError`，普通 `catch` 不得恢复；完整报告字段见[运行时错误报告](../../../troubleshooting/errors-and-reports.md)。

## 固定展开顺序

发生主错误时，运行时严格执行：`finally -> drop -> 匹配 catch/继续传播`。07-B 测试驱动器按错误类型名选择
第一个匹配处理器；没有匹配时保留原错误对象并继续传播。

## 与语言控制流的衔接

`catch` 绑定在独立作用域中，不能读取已经离开 `try` 主体并完成释放的局部资源。具体错误类型必须排在
`Error`/`XiaoError` 之前；`FatalError` 不属于普通可恢复捕获范围。错误码、条件和模式匹配，以及 `Result`
泛型传播尚未开放。

## 清理错误

`drop` 或清理阶段产生的错误会写入主错误的 `suppressed` 集合，不覆盖原始主错误。即使某个清理动作失败，剩余绑定仍继续释放。

## 原生 ABI 边界

LLVM 原生入口复用同一套错误身份和展开顺序：机器字段保持 `code`、`message_id`、参数、
源码位置和退出码不变，不在原生侧复制消息目录或插值逻辑。Runtime ABI 的次生可恢复错误
追加到主错误的 `suppressed`；已有 Fatal 保持 Fatal。当前 N0-C 对清理阶段 Fatal 直接终止，
因此不要把该路径当作普通 `catch` 可恢复错误处理。

## 调试边界

本页面不定义调试窗口、日志输出或完整错误本地化；这些能力属于后续 CLI、诊断和国际化阶段。
