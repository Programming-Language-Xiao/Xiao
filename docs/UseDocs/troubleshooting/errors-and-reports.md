---
id: troubleshooting.errors-and-reports
title: 运行时错误报告
status: verified
audience: learner
module: rust.xiao-diagnostics
stage: "07A"
version: "0.1.0"
related:
  - README.md
  - diagnostics-structure.md
  - ../language/memory/runtime/errors-and-unwind.md
  - ../../DevDocs/07-concurrency-and-errors.md
---

# 运行时错误报告

[返回故障排查索引](README.md)

Xiao 的错误报告把“程序可判断的字段”和“给人阅读的文案”分开。程序、测试和自动化工具应读取稳定错误码、消息键、结构化参数和位置，不要匹配当前语言的整句文本。

## 可恢复错误

Runtime 可安全继续或交给调用者处理的问题使用 `XiaoError`。当前 06-B/07-A 已稳定的错误码仍以 `X06-RUNTIME-*` 开头，例如：

| 编号 | 含义 |
| --- | --- |
| `X06-RUNTIME-001` | 句柄无效或为空 |
| `X06-RUNTIME-002` | Runtime 类型不匹配 |
| `X06-RUNTIME-003` | 引用计数不变量错误 |
| `X06-RUNTIME-004` | 访问已经释放的对象 |
| `X06-RUNTIME-005` | 弱引用无法升级 |
| `X06-RUNTIME-006` | 表状态不允许当前操作 |
| `X06-RUNTIME-007` | 表初始化失败 |
| `X06-RUNTIME-008` | 表释放钩子失败 |
| `X06-RUNTIME-009` | 数值运算溢出或产生非有限结果 |
| `X06-RUNTIME-010` | 禁止跨线程传递 Runtime 对象 |
| `X06-RUNTIME-011` | Runtime 对象分配失败 |
| `X06-RUNTIME-012` | Runtime 值不满足操作要求 |
| `X06-RUNTIME-013` | 整数除法或取模的除数为零 |
| `X06-RUNTIME-014` | 容器索引超出长度 |
| `X06-RUNTIME-015` | 字典中不存在该键 |
| `X06-RUNTIME-016` | 该类型的值不能作为集合元素或字典键 |
| `X06-RUNTIME-017` | 选择器边界、路径或无序容器约束失败 |
| `X06-RUNTIME-018` | 选择器步长不是整数或为零 |
| `X06-RUNTIME-019` | 随机选择数量非法、超量或来源为空 |
| `X06-RUNTIME-020` | 随机种子不是合法非负整数 |

错误还可能包含 `message_id`、`params`、源码字节区间、操作上下文、调用栈、直接原因和 `suppressed` 清理错误。清理错误不会覆盖主错误；请先处理主错误，再检查 suppressed 列表。

## 致命故障

继续执行已经不安全的问题使用独立的 `FatalError`，普通 `catch` 不能把它恢复为成功。Fatal 错误码以 `X07-FATAL-*` 开头：

| 编号 | 含义 |
| --- | --- |
| `X07-FATAL-001` | Runtime 不变量损坏 |
| `X07-FATAL-002` | 字节码或其他产物损坏 |
| `X07-FATAL-003` | 无法安全建立错误对象的内存耗尽 |
| `X07-FATAL-004` | 调用栈耗尽 |
| `X07-FATAL-005` | 硬件异常 |
| `X07-FATAL-006` | 未分类的内部故障 |

Fatal 报告仍然保留原因、位置和调用栈，便于提交可复现报告；它不表示用户可以通过重试同一进程继续执行。

## 堆栈与后端位置

统一堆栈帧包含模块名、函数名、源文件、源码字节区间和用户/Runtime 帧类别，并预留字节码偏移、原生地址和内联深度。字节码与 LLVM 后端使用相同字段，因此切换执行模式时可以比较错误身份和回溯位置。

09R2 研究 VM 已执行有序容器选择、随机抽样和事务性广播；上述 `X06-RUNTIME-017..020`
均是可恢复 `XiaoError`，会沿当前帧 handler、`finally` 和释放计划传播。研究 VM 仍不等于
生产 `xiao run`，正式 `.xiaoc` 加载和用户可见命令由后续阶段开放。

## 报告与语言

07-A 提供结构化 `ReportRecord` 和默认文本报告，但不绑定 JSON 或其他具体序列化格式，也不直接依赖语言包。后续 `[language]` 配置接入后，语言只改变人类可读文本；错误码、消息键、参数、位置、原因链、堆栈和退出语义保持不变。

## 处理建议

1. 先记录完整错误码和 `message_id`，不要只复制翻译后的句子。
2. 若存在源码位置，检查对应字节区间；中文字符按 UTF-8 字节偏移保存。
3. 若存在原因链，先从最内层原因定位资源或类型问题，再查看外层上下文。
4. 看到 `X07-FATAL-*` 时保存完整报告并结束当前进程，勿尝试用普通 `catch` 忽略。
